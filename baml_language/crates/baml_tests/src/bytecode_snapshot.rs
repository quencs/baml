//! Suite-level bytecode + engine snapshot runner.
//!
//! Snapshot files are top-level YAML maps:
//!
//! ```text
//! test_name:
//!   baml: |-
//!     function main() -> int { 42 }
//!   entry: main      # optional, defaults to "main"
//!   opt: one         # optional, one|zero, defaults to "one"
//!   args: { ... }    # optional, but required when entry takes params
//!                    # named map keyed by parameter name
//!                    # class args are plain objects, e.g. { c: { value: 0 } }
//!   bytecode: ...    # auto-generated
//!   result: ...      # auto-generated
//! ```

use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Write,
    sync::Arc,
};

use anyhow::Context;
use bex_engine::{BexEngine, BexExternalValue, FunctionCallContextBuilder};
use bex_vm::{BexVm, VmExecState};
use indexmap::IndexMap;
use insta::{assert_snapshot, with_settings};
use serde::Deserialize;
use serde_yaml::Value as YamlValue;

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SnapshotOptLevel {
    Zero,
    #[default]
    One,
}

impl SnapshotOptLevel {
    fn as_emit_opt_level(self) -> baml_compiler_emit::OptLevel {
        match self {
            SnapshotOptLevel::Zero => baml_compiler_emit::OptLevel::Zero,
            SnapshotOptLevel::One => baml_compiler_emit::OptLevel::One,
        }
    }

    fn as_yaml_str(self) -> &'static str {
        match self {
            SnapshotOptLevel::Zero => "zero",
            SnapshotOptLevel::One => "one",
        }
    }
}

fn default_entry() -> String {
    "main".to_string()
}

fn is_default_entry(entry: &str) -> bool {
    entry == "main"
}

fn is_default_opt(opt: &SnapshotOptLevel) -> bool {
    matches!(opt, SnapshotOptLevel::One)
}

#[derive(Debug, Deserialize)]
struct InputCase {
    baml: String,
    #[serde(default = "default_entry")]
    entry: String,
    #[serde(default)]
    opt: SnapshotOptLevel,
    #[serde(default)]
    args: Option<YamlValue>,
    #[allow(dead_code)]
    #[serde(default)]
    bytecode: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    result: Option<String>,
}

#[derive(Debug)]
struct OutputCase {
    baml: String,
    entry: String,
    opt: SnapshotOptLevel,
    args: Option<YamlValue>,
    bytecode: String,
    notifications: Option<Vec<String>>,
    result: String,
}

fn strip_insta_frontmatter(contents: &str) -> &str {
    if let Some(rest) = contents.strip_prefix("---\n") {
        if let Some(idx) = rest.find("\n---\n") {
            // Skip `<metadata>\n---\n`.
            return &rest[idx + "\n---\n".len()..];
        }
    } else if let Some(rest) = contents.strip_prefix("---\r\n")
        && let Some(idx) = rest.find("\r\n---\r\n")
    {
        // Skip `<metadata>\r\n---\r\n`.
        return &rest[idx + "\r\n---\r\n".len()..];
    }
    contents
}

fn parse_cases(snapshot_contents: &str) -> anyhow::Result<IndexMap<String, InputCase>> {
    let body = strip_insta_frontmatter(snapshot_contents).trim();
    serde_yaml::from_str(body).context("failed to parse suite YAML from snapshot body")
}

fn compile_with_opt(source: &str, opt: SnapshotOptLevel) -> anyhow::Result<bex_vm_types::Program> {
    let db = crate::bytecode::setup_test_db(source);
    {
        use baml_compiler_diagnostics::Severity;

        let project = db.get_project().context("project should be set")?;
        let all_files = db.get_source_files();
        let diagnostics = baml_project::collect_diagnostics(&db, project, &all_files);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error))
            .collect();
        if !errors.is_empty() {
            let mut msg = String::from("Compilation produced diagnostic errors:\n");
            for (i, err) in errors.iter().enumerate() {
                msg.push_str(&format!("  {}. [{}] {}\n", i + 1, err.code(), err.message));
            }
            anyhow::bail!(msg);
        }
    }

    let project = db.get_project().context("project should be set")?;
    let all_files = project.files(&db).clone();
    baml_compiler_emit::compile_files(&db, &all_files, opt.as_emit_opt_level())
        .context("compile_files failed")
}

fn disassemble_program(program: &bex_vm_types::Program) -> anyhow::Result<String> {
    let mut names: Vec<_> = program
        .function_indices
        .keys()
        .filter(|name| !name.starts_with("baml."))
        .collect();
    names.sort();

    let functions: Vec<(String, &bex_vm_types::types::Function)> = names
        .iter()
        .map(|name| {
            let idx = *program.function_indices.get(*name).ok_or_else(|| {
                anyhow::anyhow!("function '{name}' missing from function_indices")
            })?;
            let object = program
                .objects
                .get(idx)
                .ok_or_else(|| anyhow::anyhow!("object index {idx} not found for '{name}'"))?;
            let bex_vm_types::Object::Function(function) = object else {
                anyhow::bail!("object index {idx} for '{name}' is not a Function");
            };
            Ok(((*name).clone(), function.as_ref()))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(bex_vm::debug::display_program(
        &functions,
        bex_vm::debug::BytecodeFormat::Textual,
    ))
}

fn entry_function<'a>(
    program: &'a bex_vm_types::Program,
    entry: &str,
) -> anyhow::Result<&'a bex_vm_types::types::Function> {
    let idx = *program
        .function_indices
        .get(entry)
        .ok_or_else(|| anyhow::anyhow!("function '{entry}' missing from function_indices"))?;
    let object = program
        .objects
        .get(idx)
        .ok_or_else(|| anyhow::anyhow!("object index {idx} not found for '{entry}'"))?;
    let bex_vm_types::Object::Function(function) = object else {
        anyhow::bail!("object index {idx} for '{entry}' is not a Function");
    };
    Ok(function.as_ref())
}

fn merge_types(types: impl IntoIterator<Item = baml_type::Ty>) -> baml_type::Ty {
    let mut unique: Vec<baml_type::Ty> = Vec::new();
    for ty in types {
        if !unique.contains(&ty) {
            unique.push(ty);
        }
    }

    match unique.len() {
        0 => baml_type::Ty::Null,
        1 => unique.pop().expect("non-empty by construction"),
        _ => baml_type::Ty::Union(unique),
    }
}

fn external_value_type(value: &BexExternalValue) -> baml_type::Ty {
    match value {
        BexExternalValue::Null => baml_type::Ty::Null,
        BexExternalValue::Int(_) => baml_type::Ty::Int,
        BexExternalValue::Float(_) => baml_type::Ty::Float,
        BexExternalValue::Bool(_) => baml_type::Ty::Bool,
        BexExternalValue::String(_) => baml_type::Ty::String,
        BexExternalValue::Array { element_type, .. } => {
            baml_type::Ty::List(Box::new(element_type.clone()))
        }
        BexExternalValue::Map {
            key_type,
            value_type,
            ..
        } => baml_type::Ty::Map {
            key: Box::new(key_type.clone()),
            value: Box::new(value_type.clone()),
        },
        _ => baml_type::Ty::Null,
    }
}

fn yaml_value_to_external_arg(value: &YamlValue) -> anyhow::Result<BexExternalValue> {
    match value {
        YamlValue::Null => Ok(BexExternalValue::Null),
        YamlValue::Bool(value) => Ok(BexExternalValue::Bool(*value)),
        YamlValue::Number(value) => {
            if let Some(int) = value.as_i64() {
                Ok(BexExternalValue::Int(int))
            } else if let Some(float) = value.as_f64() {
                Ok(BexExternalValue::Float(float))
            } else {
                anyhow::bail!("unsupported numeric arg value: {value}");
            }
        }
        YamlValue::String(value) => Ok(BexExternalValue::String(value.clone())),
        YamlValue::Sequence(values) => {
            let items: Vec<BexExternalValue> = values
                .iter()
                .map(yaml_value_to_external_arg)
                .collect::<anyhow::Result<Vec<_>>>()?;
            let element_type = merge_types(items.iter().map(external_value_type));
            Ok(BexExternalValue::Array {
                element_type,
                items,
            })
        }
        YamlValue::Mapping(entries) => {
            let enum_tag = YamlValue::String("__enum__".to_string());
            let variant_tag = YamlValue::String("__variant__".to_string());
            if let Some(YamlValue::String(enum_name)) = entries.get(&enum_tag) {
                let Some(YamlValue::String(variant_name)) = entries.get(&variant_tag) else {
                    anyhow::bail!("enum arg for '{enum_name}' must include string `__variant__`");
                };
                return Ok(BexExternalValue::Variant {
                    enum_name: enum_name.clone(),
                    variant_name: variant_name.clone(),
                });
            }

            let mut map = IndexMap::new();
            let mut value_types = Vec::new();
            for (key, value) in entries {
                let YamlValue::String(key) = key else {
                    anyhow::bail!("map args must use string keys, got key: {key:?}");
                };
                let converted = yaml_value_to_external_arg(value)?;
                value_types.push(external_value_type(&converted));
                map.insert(key.clone(), converted);
            }

            Ok(BexExternalValue::Map {
                key_type: baml_type::Ty::String,
                value_type: merge_types(value_types),
                entries: map,
            })
        }
        other => anyhow::bail!("unsupported arg YAML value: {other:?}"),
    }
}

fn type_name_candidates(type_name: &baml_type::TypeName) -> Vec<String> {
    let mut candidates = Vec::new();

    let display = type_name.display_name.to_string();
    if !display.is_empty() {
        candidates.push(display);
    }

    let short = type_name.name.to_string();
    if !short.is_empty() && !candidates.iter().any(|candidate| candidate == &short) {
        candidates.push(short);
    }

    candidates
}

fn find_class_for_type<'a>(
    program: &'a bex_vm_types::Program,
    type_name: &baml_type::TypeName,
) -> Option<&'a bex_vm_types::types::Class> {
    let candidates = type_name_candidates(type_name);
    program.objects.iter().find_map(|object| {
        let bex_vm_types::Object::Class(class) = object else {
            return None;
        };
        candidates
            .iter()
            .any(|candidate| candidate == &class.name)
            .then_some(class)
    })
}

fn find_enum_for_type<'a>(
    program: &'a bex_vm_types::Program,
    type_name: &baml_type::TypeName,
) -> Option<&'a bex_vm_types::types::Enum> {
    let candidates = type_name_candidates(type_name);
    program.objects.iter().find_map(|object| {
        let bex_vm_types::Object::Enum(enm) = object else {
            return None;
        };
        candidates
            .iter()
            .any(|candidate| candidate == &enm.name)
            .then_some(enm)
    })
}

fn yaml_value_to_external_arg_typed(
    program: &bex_vm_types::Program,
    value: &YamlValue,
    expected_ty: &baml_type::Ty,
) -> anyhow::Result<BexExternalValue> {
    match expected_ty {
        baml_type::Ty::Int => {
            let YamlValue::Number(number) = value else {
                anyhow::bail!("expected int arg, got {value:?}");
            };
            let Some(int) = number.as_i64() else {
                anyhow::bail!("expected int arg, got non-integer number {number}");
            };
            Ok(BexExternalValue::Int(int))
        }
        baml_type::Ty::Float => {
            let YamlValue::Number(number) = value else {
                anyhow::bail!("expected float arg, got {value:?}");
            };
            let Some(float) = number.as_f64() else {
                anyhow::bail!("expected float arg, got invalid number {number}");
            };
            Ok(BexExternalValue::Float(float))
        }
        baml_type::Ty::Bool => {
            let YamlValue::Bool(boolean) = value else {
                anyhow::bail!("expected bool arg, got {value:?}");
            };
            Ok(BexExternalValue::Bool(*boolean))
        }
        baml_type::Ty::String => {
            let YamlValue::String(string) = value else {
                anyhow::bail!("expected string arg, got {value:?}");
            };
            Ok(BexExternalValue::String(string.clone()))
        }
        baml_type::Ty::Null => {
            if !matches!(value, YamlValue::Null) {
                anyhow::bail!("expected null arg, got {value:?}");
            }
            Ok(BexExternalValue::Null)
        }
        baml_type::Ty::Literal(baml_type::Literal::Int(expected)) => {
            let YamlValue::Number(number) = value else {
                anyhow::bail!("expected int literal {expected}, got {value:?}");
            };
            let Some(actual) = number.as_i64() else {
                anyhow::bail!("expected int literal {expected}, got non-integer number {number}");
            };
            if &actual != expected {
                anyhow::bail!("expected int literal {expected}, got {actual}");
            }
            Ok(BexExternalValue::Int(actual))
        }
        baml_type::Ty::Literal(baml_type::Literal::Float(expected)) => {
            let YamlValue::Number(number) = value else {
                anyhow::bail!("expected float literal {expected}, got {value:?}");
            };
            let Some(actual) = number.as_f64() else {
                anyhow::bail!("expected float literal {expected}, got invalid number {number}");
            };
            let expected = expected.parse::<f64>().with_context(|| {
                format!("invalid float literal '{expected}' in expected parameter type")
            })?;
            if (actual - expected).abs() > f64::EPSILON {
                anyhow::bail!("expected float literal {expected}, got {actual}");
            }
            Ok(BexExternalValue::Float(actual))
        }
        baml_type::Ty::Literal(baml_type::Literal::String(expected)) => {
            let YamlValue::String(actual) = value else {
                anyhow::bail!("expected string literal {expected:?}, got {value:?}");
            };
            if actual != expected {
                anyhow::bail!("expected string literal {expected:?}, got {actual:?}");
            }
            Ok(BexExternalValue::String(actual.clone()))
        }
        baml_type::Ty::Literal(baml_type::Literal::Bool(expected)) => {
            let YamlValue::Bool(actual) = value else {
                anyhow::bail!("expected bool literal {expected}, got {value:?}");
            };
            if actual != expected {
                anyhow::bail!("expected bool literal {expected}, got {actual}");
            }
            Ok(BexExternalValue::Bool(*actual))
        }
        baml_type::Ty::Optional(inner) => {
            if matches!(value, YamlValue::Null) {
                Ok(BexExternalValue::Null)
            } else {
                yaml_value_to_external_arg_typed(program, value, inner)
            }
        }
        baml_type::Ty::List(element_type) => {
            let YamlValue::Sequence(values) = value else {
                anyhow::bail!("expected list arg, got {value:?}");
            };

            let items = values
                .iter()
                .map(|item| yaml_value_to_external_arg_typed(program, item, element_type))
                .collect::<anyhow::Result<Vec<_>>>()?;

            Ok(BexExternalValue::Array {
                element_type: (**element_type).clone(),
                items,
            })
        }
        baml_type::Ty::Map {
            key,
            value: value_ty,
        } => {
            let YamlValue::Mapping(entries) = value else {
                anyhow::bail!("expected map arg, got {value:?}");
            };

            if !matches!(**key, baml_type::Ty::String) {
                anyhow::bail!("map args currently support string keys only, got key type {key:?}");
            }

            let mut converted = IndexMap::new();
            for (entry_key, entry_value) in entries {
                let YamlValue::String(entry_key) = entry_key else {
                    anyhow::bail!("map arg keys must be strings, got key: {entry_key:?}");
                };
                converted.insert(
                    entry_key.clone(),
                    yaml_value_to_external_arg_typed(program, entry_value, value_ty)
                        .with_context(|| format!("invalid map value for key '{entry_key}'"))?,
                );
            }

            Ok(BexExternalValue::Map {
                key_type: (**key).clone(),
                value_type: (**value_ty).clone(),
                entries: converted,
            })
        }
        baml_type::Ty::Class(type_name) => {
            let YamlValue::Mapping(entries) = value else {
                anyhow::bail!("expected object arg for class '{type_name}', got {value:?}");
            };

            let class = find_class_for_type(program, type_name);
            let class_name = class
                .map(|class| class.name.clone())
                .or_else(|| type_name_candidates(type_name).into_iter().next())
                .unwrap_or_else(|| type_name.to_string());

            let mut provided_fields: IndexMap<&str, &YamlValue> = IndexMap::new();
            for (key, entry_value) in entries {
                let YamlValue::String(field_name) = key else {
                    anyhow::bail!(
                        "class arg fields for '{class_name}' must use string keys, got key: {key:?}"
                    );
                };
                provided_fields.insert(field_name.as_str(), entry_value);
            }

            if let Some(class) = class {
                let expected_fields: IndexMap<&str, &baml_type::Ty> = class
                    .fields
                    .iter()
                    .map(|field| (field.name.as_str(), &field.field_type))
                    .collect();

                let unknown: Vec<&str> = provided_fields
                    .keys()
                    .copied()
                    .filter(|field_name| !expected_fields.contains_key(field_name))
                    .collect();
                if !unknown.is_empty() {
                    anyhow::bail!(
                        "class arg '{class_name}' received unknown fields: {}",
                        unknown.join(", ")
                    );
                }

                let missing: Vec<&str> = class
                    .fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .filter(|field_name| !provided_fields.contains_key(field_name))
                    .collect();
                if !missing.is_empty() {
                    anyhow::bail!(
                        "class arg '{class_name}' is missing required fields: {}",
                        missing.join(", ")
                    );
                }

                let mut converted_fields = IndexMap::new();
                for field in &class.fields {
                    let raw_value = provided_fields
                        .get(field.name.as_str())
                        .expect("validated above");
                    converted_fields.insert(
                        field.name.clone(),
                        yaml_value_to_external_arg_typed(program, raw_value, &field.field_type)
                            .with_context(|| {
                                format!(
                                    "invalid value for class field '{}.{}'",
                                    class.name, field.name
                                )
                            })?,
                    );
                }

                return Ok(BexExternalValue::Instance {
                    class_name: class.name.clone(),
                    fields: converted_fields,
                });
            }

            let mut converted_fields = IndexMap::new();
            for (field_name, field_value) in provided_fields {
                converted_fields.insert(
                    field_name.to_string(),
                    yaml_value_to_external_arg(field_value)?,
                );
            }

            Ok(BexExternalValue::Instance {
                class_name,
                fields: converted_fields,
            })
        }
        baml_type::Ty::Enum(type_name) => {
            let enm = find_enum_for_type(program, type_name);
            let enum_name = enm
                .map(|enm| enm.name.clone())
                .or_else(|| type_name_candidates(type_name).into_iter().next())
                .unwrap_or_else(|| type_name.to_string());

            let variant_name = match value {
                YamlValue::String(variant_name) => variant_name.clone(),
                YamlValue::Mapping(entries) => {
                    let enum_tag = YamlValue::String("__enum__".to_string());
                    let variant_tag = YamlValue::String("__variant__".to_string());

                    if let Some(YamlValue::String(provided_enum_name)) = entries.get(&enum_tag)
                        && provided_enum_name != &enum_name
                    {
                        anyhow::bail!(
                            "enum arg tag mismatch: expected '{enum_name}', got '{provided_enum_name}'"
                        );
                    }

                    let Some(YamlValue::String(variant_name)) = entries.get(&variant_tag) else {
                        anyhow::bail!(
                            "enum arg for '{enum_name}' must be a string variant or include `__variant__`"
                        );
                    };

                    variant_name.clone()
                }
                other => anyhow::bail!(
                    "expected enum arg for '{enum_name}' as a string or mapping, got {other:?}"
                ),
            };

            if let Some(enm) = enm
                && !enm
                    .variants
                    .iter()
                    .any(|variant| variant.name == variant_name)
            {
                anyhow::bail!("enum arg '{enum_name}' received unknown variant '{variant_name}'");
            }

            Ok(BexExternalValue::Variant {
                enum_name,
                variant_name,
            })
        }
        baml_type::Ty::Union(options) => {
            if matches!(value, YamlValue::Null)
                && options
                    .iter()
                    .any(|option| matches!(option, baml_type::Ty::Null))
            {
                return Ok(BexExternalValue::Null);
            }

            let mut first_error: Option<anyhow::Error> = None;
            for option in options {
                if matches!(option, baml_type::Ty::Null) {
                    continue;
                }

                match yaml_value_to_external_arg_typed(program, value, option) {
                    Ok(converted) => return Ok(converted),
                    Err(err) => {
                        if first_error.is_none() {
                            first_error = Some(err);
                        }
                    }
                }
            }

            if let Some(err) = first_error {
                return Err(err.context(format!(
                    "value {value:?} does not match union parameter type {expected_ty:?}"
                )));
            }

            anyhow::bail!("value {value:?} does not match union parameter type {expected_ty:?}")
        }
        _ => yaml_value_to_external_arg(value),
    }
}

fn resolve_runtime_args(
    program: &bex_vm_types::Program,
    entry: &str,
    provided: Option<&YamlValue>,
) -> anyhow::Result<Vec<BexExternalValue>> {
    let function = entry_function(program, entry).ok();

    let Some(function) = function else {
        return match provided {
            Some(YamlValue::Mapping(_)) => Ok(Vec::new()),
            None => Ok(Vec::new()),
            Some(other) => {
                anyhow::bail!("entry point '{entry}' expects `args` as a mapping, got {other:?}")
            }
        };
    };

    match provided {
        Some(YamlValue::Mapping(args)) => {
            if function.param_names.len() != function.arity {
                anyhow::bail!(
                    "entry point '{entry}' does not expose parameter names for named args"
                );
            }

            if function.param_types.len() != function.arity {
                anyhow::bail!(
                    "entry point '{entry}' parameter metadata is inconsistent: arity={} but param_types={}",
                    function.arity,
                    function.param_types.len()
                );
            }

            let mut by_name: IndexMap<&str, &YamlValue> = IndexMap::new();
            for (key, value) in args {
                let YamlValue::String(name) = key else {
                    anyhow::bail!("named args must use string keys, got key: {key:?}");
                };
                by_name.insert(name.as_str(), value);
            }

            let unknown: Vec<&str> = by_name
                .keys()
                .copied()
                .filter(|name| !function.param_names.iter().any(|expected| expected == name))
                .collect();
            if !unknown.is_empty() {
                anyhow::bail!(
                    "entry point '{entry}' received unknown arg names: {}",
                    unknown.join(", ")
                );
            }

            let missing: Vec<&str> = function
                .param_names
                .iter()
                .map(String::as_str)
                .filter(|name| !by_name.contains_key(name))
                .collect();
            if !missing.is_empty() {
                anyhow::bail!(
                    "entry point '{entry}' is missing required args: {}",
                    missing.join(", ")
                );
            }

            function
                .param_names
                .iter()
                .zip(function.param_types.iter())
                .map(|(name, ty)| {
                    let value = by_name.get(name.as_str()).expect("validated above");
                    yaml_value_to_external_arg_typed(program, value, ty)
                })
                .collect::<anyhow::Result<Vec<_>>>()
        }
        Some(other) => {
            anyhow::bail!("entry point '{entry}' expects `args` as a mapping, got {other:?}")
        }
        None if function.arity == 0 => Ok(Vec::new()),
        None => anyhow::bail!(
            "entry point '{entry}' requires {} args but they were not provided; add `args: {{name: value, ...}}` to this case",
            function.arity
        ),
    }
}

fn external_arg_to_vm_value(vm: &mut BexVm, value: &BexExternalValue) -> bex_vm_types::Value {
    match value {
        BexExternalValue::Null => bex_vm_types::Value::Null,
        BexExternalValue::Int(value) => bex_vm_types::Value::Int(*value),
        BexExternalValue::Float(value) => bex_vm_types::Value::Float(*value),
        BexExternalValue::Bool(value) => bex_vm_types::Value::Bool(*value),
        BexExternalValue::String(value) => vm.alloc_string(value.clone()),
        BexExternalValue::Array { items, .. } => {
            let vm_items = items
                .iter()
                .map(|item| external_arg_to_vm_value(vm, item))
                .collect();
            vm.alloc_array(vm_items)
        }
        BexExternalValue::Map { entries, .. } => {
            let vm_entries: IndexMap<String, bex_vm_types::Value> = entries
                .iter()
                .map(|(key, value)| (key.clone(), external_arg_to_vm_value(vm, value)))
                .collect();
            vm.alloc_map(vm_entries)
        }
        _ => bex_vm_types::Value::Null,
    }
}

fn run_with_engine(
    runtime: &tokio::runtime::Runtime,
    program: bex_vm_types::Program,
    entry: &str,
    args: &[BexExternalValue],
) -> anyhow::Result<String> {
    let engine = BexEngine::new(program, Arc::new(sys_types::SysOps::all_unsupported()))
        .context("failed to construct BexEngine")?;

    let context = FunctionCallContextBuilder::new(sys_types::CallId::next()).build();
    let result = runtime
        .block_on(engine.call_function(entry, args.to_vec(), context))
        .with_context(|| format!("engine call_function failed for '{entry}'"))?;

    Ok(format!("BexExternalValue::{result:?}"))
}

// NOTE: We intentionally render VM values directly here instead of reusing
// `BexExternalValue` conversion for now:
// - watch-mode snapshots need the raw `BexVm` loop to capture `VmExecState::Notify`.
// - existing deep-copy conversion used by the engine is not cycle-safe for arbitrary graphs.
// - watch snapshots need deterministic object shape output (with cycle refs), not lossy fallback.
#[derive(Default)]
struct VmValueRenderState {
    object_ids: HashMap<bex_vm_types::HeapPtr, usize>,
    visiting: HashSet<bex_vm_types::HeapPtr>,
}

impl VmValueRenderState {
    fn object_id(&mut self, ptr: bex_vm_types::HeapPtr) -> usize {
        let next = self.object_ids.len();
        *self.object_ids.entry(ptr).or_insert(next)
    }
}

fn render_vm_value(
    vm: &BexVm,
    value: &bex_vm_types::Value,
    state: &mut VmValueRenderState,
) -> String {
    match value {
        bex_vm_types::Value::Null => "Null".to_string(),
        bex_vm_types::Value::Int(v) => format!("Int({v})"),
        bex_vm_types::Value::Float(v) => format!("Float({v})"),
        bex_vm_types::Value::Bool(v) => format!("Bool({v})"),
        bex_vm_types::Value::Object(ptr) => render_vm_object(vm, *ptr, state),
    }
}

fn render_vm_object(
    vm: &BexVm,
    ptr: bex_vm_types::HeapPtr,
    state: &mut VmValueRenderState,
) -> String {
    let object_id = state.object_id(ptr);
    if !state.visiting.insert(ptr) {
        return format!("Ref(#{object_id})");
    }

    let rendered = match vm.get_object(ptr) {
        bex_vm_types::Object::Function(function) => format!("Function({})", function.name),
        bex_vm_types::Object::Class(class) => format!("Class({})", class.name),
        bex_vm_types::Object::Enum(enm) => format!("Enum({})", enm.name),
        bex_vm_types::Object::String(value) => format!("String({value:?})"),
        bex_vm_types::Object::Array(array) => {
            let items = array
                .iter()
                .map(|value| render_vm_value(vm, value, state))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Array([{items}])")
        }
        bex_vm_types::Object::Map(map) => {
            let entries = map
                .iter()
                .map(|(key, value)| format!("{key:?}: {}", render_vm_value(vm, value, state)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Map({{{entries}}})")
        }
        bex_vm_types::Object::Instance(instance) => {
            let (class_name, field_names) = match vm.get_object(instance.class) {
                bex_vm_types::Object::Class(class) => (
                    class.name.clone(),
                    class
                        .fields
                        .iter()
                        .map(|field| field.name.clone())
                        .collect::<Vec<_>>(),
                ),
                _ => ("<invalid-class>".to_string(), Vec::new()),
            };

            let fields = instance
                .fields
                .iter()
                .enumerate()
                .map(|(idx, value)| {
                    let name = field_names
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| format!("field_{idx}"));
                    format!("{name}: {}", render_vm_value(vm, value, state))
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("Instance({class_name} {{ {fields} }})")
        }
        bex_vm_types::Object::Variant(variant) => {
            let (enum_name, variant_name) = match vm.get_object(variant.enm) {
                bex_vm_types::Object::Enum(enm) => (
                    enm.name.clone(),
                    enm.variants
                        .get(variant.index)
                        .map(|value| value.name.clone())
                        .unwrap_or_else(|| format!("variant_{}", variant.index)),
                ),
                _ => (
                    "<invalid-enum>".to_string(),
                    format!("variant_{}", variant.index),
                ),
            };
            format!("Variant({enum_name}::{variant_name})")
        }
        bex_vm_types::Object::Future(future) => match future {
            bex_vm_types::types::Future::Pending(pending) => {
                let args = pending
                    .args
                    .iter()
                    .map(|value| render_vm_value(vm, value, state))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Future(Pending(op: {}, args: [{args}]))", pending.operation)
            }
            bex_vm_types::types::Future::Ready(value) => {
                format!("Future(Ready({}))", render_vm_value(vm, value, state))
            }
        },
        bex_vm_types::Object::Resource(resource) => format!("Resource({resource})"),
        bex_vm_types::Object::Media(media) => format!("Media({media:?})"),
        bex_vm_types::Object::PromptAst(prompt_ast) => format!("PromptAst({prompt_ast:?})"),
        bex_vm_types::Object::Collector(_) => "Collector".to_string(),
        bex_vm_types::Object::Type(ty) => format!("Type({ty})"),
        #[cfg(feature = "heap_debug")]
        bex_vm_types::Object::Sentinel(kind) => format!("Sentinel({kind:?})"),
    };

    state.visiting.remove(&ptr);
    rendered
}

struct WatchRunOutput {
    notifications: Vec<String>,
    result: String,
}

fn run_watch_mode(
    program: bex_vm_types::Program,
    entry: &str,
    args: &[BexExternalValue],
) -> anyhow::Result<WatchRunOutput> {
    let function_index = program
        .function_index(entry)
        .ok_or_else(|| anyhow::anyhow!("function '{entry}' not found"))?;

    let mut vm = BexVm::from_program(program).context("failed to create BexVm")?;
    let function_ptr = vm.heap.compile_time_ptr(function_index);
    let vm_args: Vec<_> = args
        .iter()
        .map(|value| external_arg_to_vm_value(&mut vm, value))
        .collect();
    vm.set_entry_point(function_ptr, &vm_args);

    let mut notifications: Vec<String> = Vec::new();
    let mut render_state = VmValueRenderState::default();

    let final_state = loop {
        let result = vm.exec().context("vm.exec failed")?;
        if matches!(result, VmExecState::SpanNotify(_)) {
            continue;
        }
        match result {
            VmExecState::Notify(notification) => {
                notifications.push(format!("{notification:?}"));
            }
            VmExecState::Complete(value) => {
                break format!(
                    "Complete({})",
                    render_vm_value(&vm, &value, &mut render_state)
                );
            }
            VmExecState::Await(handle) => {
                let _ = handle;
                break "Await".to_string();
            }
            VmExecState::ScheduleFuture(handle) => {
                let _ = handle;
                break "ScheduleFuture".to_string();
            }
            VmExecState::SpanNotify(_) => {
                // handled above
            }
        }
    };

    Ok(WatchRunOutput {
        notifications,
        result: final_state,
    })
}

fn format_error_with_debug(err: &anyhow::Error) -> String {
    format!("ERROR: {err:#}\nRUST_ERR_DEBUG: Err({err:?})")
}

fn panic_payload_to_string(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

fn format_panic_with_debug(payload: Box<dyn Any + Send>) -> String {
    let message = panic_payload_to_string(payload.as_ref());
    format!("ERROR: panic during execution: {message}\nRUST_ERR_DEBUG: Panic({message:?})")
}

struct CaseExecutionOutput {
    notifications: Option<Vec<String>>,
    result: String,
}

impl CaseExecutionOutput {
    fn plain(result: String) -> Self {
        Self {
            notifications: None,
            result,
        }
    }
}

fn run_plain_case_with_panic_capture(
    f: impl FnOnce() -> anyhow::Result<String>,
) -> CaseExecutionOutput {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(value)) => CaseExecutionOutput::plain(value),
        Ok(Err(err)) => CaseExecutionOutput::plain(format_error_with_debug(&err)),
        Err(payload) => CaseExecutionOutput::plain(format_panic_with_debug(payload)),
    }
}

fn run_watch_case_with_panic_capture(
    f: impl FnOnce() -> anyhow::Result<WatchRunOutput>,
) -> CaseExecutionOutput {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(value)) => CaseExecutionOutput {
            notifications: Some(value.notifications),
            result: value.result,
        },
        Ok(Err(err)) => CaseExecutionOutput::plain(format_error_with_debug(&err)),
        Err(payload) => CaseExecutionOutput::plain(format_panic_with_debug(payload)),
    }
}

fn render_inline_yaml_string(value: &str) -> anyhow::Result<String> {
    let mut rendered = serde_yaml::to_string(value).context("failed to serialize scalar")?;
    if let Some(stripped) = rendered.strip_prefix("---\n") {
        rendered = stripped.to_string();
    }
    Ok(rendered.trim_end().to_string())
}

fn render_block(out: &mut String, key: &str, value: &str) {
    let _ = writeln!(out, "    {key}: |");
    for line in value.lines() {
        if line.is_empty() {
            out.push('\n');
        } else {
            let _ = writeln!(out, "        {line}");
        }
    }
}

fn render_string_flow_list(out: &mut String, key: &str, values: &[String]) -> anyhow::Result<()> {
    if values.is_empty() {
        writeln!(out, "    {key}: []").expect("write to string should not fail");
        return Ok(());
    }

    writeln!(out, "    {key}: [").expect("write to string should not fail");
    for value in values {
        let rendered = format!("{value:?}");
        writeln!(out, "        {rendered},").expect("write to string should not fail");
    }
    writeln!(out, "    ]").expect("write to string should not fail");
    Ok(())
}

fn render_yaml_value(out: &mut String, key: &str, value: &YamlValue) -> anyhow::Result<()> {
    let mut rendered = serde_yaml::to_string(value).context("failed to serialize args value")?;
    if let Some(stripped) = rendered.strip_prefix("---\n") {
        rendered = stripped.to_string();
    }

    let rendered = rendered.trim_end();
    let is_collection = matches!(value, YamlValue::Mapping(_) | YamlValue::Sequence(_));
    if !is_collection && !rendered.contains('\n') {
        writeln!(out, "    {key}: {rendered}").expect("write to string should not fail");
        return Ok(());
    }

    writeln!(out, "    {key}:").expect("write to string should not fail");
    for line in rendered.lines() {
        writeln!(out, "      {line}").expect("write to string should not fail");
    }

    Ok(())
}

fn render_cases(cases: &IndexMap<String, OutputCase>) -> anyhow::Result<String> {
    let mut out = String::new();

    for (idx, (test_name, case)) in cases.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }

        writeln!(out, "{test_name}:").expect("write to string should not fail");
        render_block(&mut out, "baml", &case.baml);

        if !is_default_entry(&case.entry) {
            out.push('\n');
            let entry = render_inline_yaml_string(&case.entry)?;
            writeln!(out, "    entry: {entry}").expect("write to string should not fail");
        }

        if !is_default_opt(&case.opt) {
            out.push('\n');
            writeln!(out, "    opt: {}", case.opt.as_yaml_str())
                .expect("write to string should not fail");
        }

        if let Some(args) = &case.args {
            out.push('\n');
            render_yaml_value(&mut out, "args", args)?;
        }

        out.push('\n');
        render_block(&mut out, "bytecode", &case.bytecode);
        out.push('\n');
        if let Some(notifications) = &case.notifications {
            render_string_flow_list(&mut out, "notifications", notifications)?;
            out.push('\n');
        }
        if case.result.contains('\n') {
            render_block(&mut out, "result", &case.result);
        } else {
            let result = render_inline_yaml_string(&case.result)?;
            writeln!(out, "    result: {result}").expect("write to string should not fail");
        }
    }

    Ok(out)
}

/// Recompute all test cases from a suite snapshot, then snapshot the canonical YAML.
pub fn assert_suite_snapshot(
    suite_name: &str,
    snapshot_path: &str,
    snapshot_contents: &str,
) -> anyhow::Result<()> {
    let input_cases = parse_cases(snapshot_contents)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?;

    let mut output_cases: IndexMap<String, OutputCase> = IndexMap::new();

    for (test_name, case) in input_cases {
        let baml = case.baml;
        let entry = case.entry;
        let args = case.args;

        let (bytecode, execution) = match compile_with_opt(&baml, case.opt)
            .with_context(|| format!("compile failed for case '{test_name}'"))
        {
            Ok(program) => {
                let bytecode = match disassemble_program(&program)
                    .with_context(|| format!("disassembly failed for case '{test_name}'"))
                {
                    Ok(text) => text,
                    Err(err) => format_error_with_debug(&err),
                };

                let call_args = match resolve_runtime_args(&program, &entry, args.as_ref())
                    .with_context(|| format!("argument resolution failed for case '{test_name}'"))
                {
                    Ok(args) => args,
                    Err(err) => {
                        let execution = CaseExecutionOutput::plain(format_error_with_debug(&err));
                        output_cases.insert(
                            test_name,
                            OutputCase {
                                baml: baml.trim_end_matches('\n').to_string(),
                                entry,
                                opt: case.opt,
                                args,
                                bytecode,
                                notifications: execution.notifications,
                                result: execution.result,
                            },
                        );
                        continue;
                    }
                };

                let watch_execution = run_watch_case_with_panic_capture(|| {
                    run_watch_mode(program.clone(), &entry, &call_args)
                        .with_context(|| format!("watch execution failed for case '{test_name}'"))
                });

                let execution = if watch_execution
                    .notifications
                    .as_ref()
                    .is_some_and(|notifications| !notifications.is_empty())
                {
                    watch_execution
                } else {
                    run_plain_case_with_panic_capture(|| {
                        run_with_engine(&runtime, program, &entry, &call_args)
                            .with_context(|| format!("execution failed for case '{test_name}'"))
                    })
                };

                (bytecode, execution)
            }
            Err(err) => {
                let formatted = format_error_with_debug(&err);
                (
                    formatted.clone(),
                    CaseExecutionOutput::plain(format!(
                        "ERROR: compile step failed; no runtime result\n{formatted}"
                    )),
                )
            }
        };

        output_cases.insert(
            test_name,
            OutputCase {
                baml: baml.trim_end_matches('\n').to_string(),
                entry,
                opt: case.opt,
                args,
                bytecode,
                notifications: execution.notifications,
                result: execution.result,
            },
        );
    }

    let rendered = render_cases(&output_cases)?;

    with_settings!({snapshot_path => snapshot_path, omit_expression => true, prepend_module_to_snapshot => false}, {
        assert_snapshot!(suite_name, rendered);
    });

    Ok(())
}
