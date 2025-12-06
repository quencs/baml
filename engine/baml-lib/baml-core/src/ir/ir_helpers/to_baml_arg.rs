use core::result::Result;
use std::path::PathBuf;

use baml_types::{
    BamlMap, BamlMediaType, BamlValue, BamlValueWithMeta, Constraint, ConstraintLevel,
    LiteralValue, TypeIR, TypeValue,
};

use super::{scope_diagnostics::ScopeStack, IRHelper, IRHelperExtended};
use crate::ir::{ir_helpers::infer_type, jinja_helpers::evaluate_predicate, IntermediateRepr};

#[derive(Default)]
pub struct ParameterError {
    vec: Vec<String>,
}

#[allow(dead_code)]
impl ParameterError {
    pub(super) fn required_param_missing(&mut self, param_name: &str) {
        self.vec
            .push(format!("Missing required parameter: {param_name}"));
    }

    pub fn invalid_param_type(&mut self, param_name: &str, expected: &str, got: &str) {
        self.vec.push(format!(
            "Invalid parameter type for {param_name}: expected {expected}, got {got}"
        ));
    }
}

pub struct ArgCoercer {
    pub span_path: Option<PathBuf>,
    pub allow_implicit_cast_to_string: bool,
}

/// Linter doesn't like `Result<T, ()>` so we'll use this as a placeholder.
pub struct ArgCoerceError;

impl ArgCoercer {
    pub fn coerce_arg(
        &self,
        ir: &IntermediateRepr,
        field_type: &TypeIR,
        value: &BamlValue, // original value passed in by user
        scope: &mut ScopeStack,
    ) -> Result<BamlValueWithMeta<TypeIR>, ArgCoerceError> {
        let metadata = field_type.meta();

        let value = match field_type {
            TypeIR::Top(_) => {
                // For Top type, we accept any value and convert it to the appropriate type
                match value {
                    BamlValue::String(s) => {
                        Ok(BamlValueWithMeta::String(s.clone(), TypeIR::string()))
                    }
                    BamlValue::Int(i) => Ok(BamlValueWithMeta::Int(*i, TypeIR::int())),
                    BamlValue::Float(f) => Ok(BamlValueWithMeta::Float(*f, TypeIR::float())),
                    BamlValue::Bool(b) => Ok(BamlValueWithMeta::Bool(*b, TypeIR::bool())),
                    BamlValue::Map(index_map) => {
                        let mut pairs = index_map.into_iter();

                        let (map, first_type) = 'build_map: {
                            let Some((first_key, first_val)) = pairs.next() else {
                                break 'build_map (BamlMap::new(), TypeIR::Top(Default::default()));
                            };

                            // coerce first value to top, inferring the map type.
                            let first_val = self.coerce_arg(ir, field_type, first_val, scope)?;

                            let first_type = first_val.meta().clone();

                            // coerce rest to first value's type.
                            let rest = pairs.map(|(k, v)| {
                                self.coerce_arg(ir, &first_type, v, scope)
                                    .map(|v| (k.clone(), v))
                            });

                            let chained =
                                [Ok((first_key.clone(), first_val))].into_iter().chain(rest);

                            (chained.collect::<Result<_, _>>()?, first_type)
                        };

                        Ok(BamlValueWithMeta::Map(
                            map,
                            TypeIR::Map(
                                Box::new(TypeIR::string()),
                                Box::new(first_type),
                                Default::default(),
                            ),
                        ))
                    }
                    BamlValue::List(baml_values) => {
                        // Convert the list values recursively using with_const_meta
                        let converted_list = BamlValueWithMeta::with_same_meta_at_all_nodes(
                            &BamlValue::List(baml_values.clone()),
                            TypeIR::string(),
                        );
                        if let BamlValueWithMeta::List(list_values, _) = converted_list {
                            Ok(BamlValueWithMeta::List(
                                list_values,
                                TypeIR::list(TypeIR::string()),
                            ))
                        } else {
                            unreachable!()
                        }
                    }
                    BamlValue::Media(baml_media) => Ok(BamlValueWithMeta::Media(
                        baml_media.clone(),
                        TypeIR::image(),
                    )),
                    BamlValue::Enum(name, value) => Ok(BamlValueWithMeta::Enum(
                        name.clone(),
                        value.clone(),
                        TypeIR::r#enum("Any"),
                    )),
                    BamlValue::Class(name, index_map) => {
                        // Convert the class fields recursively using with_const_meta
                        let converted_class = BamlValueWithMeta::with_same_meta_at_all_nodes(
                            &BamlValue::Class(name.clone(), index_map.clone()),
                            TypeIR::string(),
                        );
                        if let BamlValueWithMeta::Class(_, class_fields, _) = converted_class {
                            Ok(BamlValueWithMeta::Class(
                                name.clone(),
                                class_fields,
                                TypeIR::class("Any"),
                            ))
                        } else {
                            unreachable!()
                        }
                    }
                    BamlValue::Null => Ok(BamlValueWithMeta::Null(TypeIR::null())),
                }
            }
            TypeIR::Primitive(t, _) => match (t, value) {
                (TypeValue::String, BamlValue::String(v)) => {
                    Ok(BamlValueWithMeta::String(v.clone(), TypeIR::string()))
                }
                // Handle { file "path/to/file" } syntax for string types - read file contents as string
                (TypeValue::String, BamlValue::Map(kv)) => {
                    if let Some(BamlValue::String(file_path)) = kv.get("file") {
                        // Validate that only "file" key is present
                        for key in kv.keys() {
                            if key != "file" {
                                scope.push_error(format!(
                                    "Invalid property `{key}` on file string: only `file` is supported"
                                ));
                            }
                        }
                        match self.span_path.as_ref() {
                            Some(span_path) => {
                                // Resolve the file path relative to the BAML file location
                                let resolved_path = span_path
                                    .parent()
                                    .map(|p| p.join(file_path))
                                    .unwrap_or_else(|| PathBuf::from(file_path));

                                // Read the file contents as UTF-8 string
                                match std::fs::read_to_string(&resolved_path) {
                                    Ok(contents) => {
                                        Ok(BamlValueWithMeta::String(contents, TypeIR::string()))
                                    }
                                    Err(e) => {
                                        scope.push_error(format!(
                                            "Failed to read file '{}': {}",
                                            resolved_path.display(),
                                            e
                                        ));
                                        Err(ArgCoerceError)
                                    }
                                }
                            }
                            None => {
                                scope.push_error(
                                    "BAML internal error: span is missing, cannot resolve file ref"
                                        .to_string(),
                                );
                                Err(ArgCoerceError)
                            }
                        }
                    } else {
                        scope.push_error(format!("Expected type String, got `{value}`"));
                        Err(ArgCoerceError)
                    }
                }
                (TypeValue::String, v) if self.allow_implicit_cast_to_string => match v {
                    BamlValue::Int(i) => {
                        Ok(BamlValueWithMeta::String(i.to_string(), TypeIR::string()))
                    }
                    BamlValue::Float(f) => {
                        Ok(BamlValueWithMeta::String(f.to_string(), TypeIR::string()))
                    }
                    BamlValue::Bool(true) => Ok(BamlValueWithMeta::String(
                        "true".to_string(),
                        TypeIR::string(),
                    )),
                    BamlValue::Bool(false) => Ok(BamlValueWithMeta::String(
                        "false".to_string(),
                        TypeIR::string(),
                    )),
                    BamlValue::Null => Ok(BamlValueWithMeta::String(
                        "null".to_string(),
                        TypeIR::string(),
                    )),
                    _ => {
                        scope.push_error(format!("Expected type {t:?}, got `{value}`"));
                        Err(ArgCoerceError)
                    }
                },
                (TypeValue::Int, BamlValue::Int(v)) => {
                    Ok(BamlValueWithMeta::Int(*v, TypeIR::int()))
                }
                (TypeValue::Float, BamlValue::Int(val)) => {
                    Ok(BamlValueWithMeta::Float(*val as f64, TypeIR::float()))
                }
                (TypeValue::Float, BamlValue::Float(v)) => {
                    Ok(BamlValueWithMeta::Float(*v, TypeIR::float()))
                }
                (TypeValue::Bool, BamlValue::Bool(v)) => {
                    Ok(BamlValueWithMeta::Bool(*v, TypeIR::bool()))
                }
                (TypeValue::Null, BamlValue::Null) => Ok(BamlValueWithMeta::Null(TypeIR::null())),
                (TypeValue::Media(BamlMediaType::Image), BamlValue::Media(v)) => {
                    Ok(BamlValueWithMeta::Media(v.clone(), TypeIR::image()))
                }
                (TypeValue::Media(BamlMediaType::Audio), BamlValue::Media(v)) => {
                    Ok(BamlValueWithMeta::Media(v.clone(), TypeIR::audio()))
                }
                (TypeValue::Media(BamlMediaType::Pdf), BamlValue::Media(v)) => {
                    Ok(BamlValueWithMeta::Media(v.clone(), TypeIR::pdf()))
                }
                (TypeValue::Media(BamlMediaType::Video), BamlValue::Media(v)) => {
                    Ok(BamlValueWithMeta::Media(v.clone(), TypeIR::video()))
                }
                (TypeValue::Media(media_type), BamlValue::Map(kv)) => {
                    let mime_type = match kv.get("media_type") {
                        None => None,
                        Some(v) => match v.as_str() {
                            None => {
                                scope.push_error(format!("Invalid property `media_type` on media {}: expected string, got {:?}", media_type, v.r#type()));
                                return Err(ArgCoerceError);
                            }
                            Some(val) => Some(val.to_string()),
                        },
                    };
                    if let Some(BamlValue::String(s)) = kv.get("file") {
                        for key in kv.keys() {
                            if !["file", "media_type"].contains(&key.as_str()) {
                                scope.push_error(format!(
                                    "Invalid property `{key}` on file {media_type}: `media_type` is the only supported property"
                                ));
                            }
                        }
                        match self.span_path.as_ref() {
                            Some(span_path) => Ok(BamlValueWithMeta::Media(
                                baml_types::BamlMedia::file(
                                    *media_type,
                                    span_path.clone(),
                                    s.to_string(),
                                    mime_type,
                                ),
                                field_type.clone(),
                            )),
                            None => {
                                scope.push_error(
                                    "BAML internal error: span is missing, cannot resolve file ref"
                                        .to_string(),
                                );
                                Err(ArgCoerceError)
                            }
                        }
                    } else if let Some(BamlValue::String(s)) = kv.get("url") {
                        for key in kv.keys() {
                            if !["url", "media_type"].contains(&key.as_str()) {
                                scope.push_error(format!(
                                    "Invalid property `{key}` on url {media_type}: `media_type` is the only supported property"
                                ));
                            }
                        }
                        Ok(BamlValueWithMeta::Media(
                            baml_types::BamlMedia::url(*media_type, s.to_string(), mime_type),
                            field_type.clone(),
                        ))
                    } else if let Some(BamlValue::String(s)) = kv.get("base64") {
                        for key in kv.keys() {
                            if !["base64", "media_type"].contains(&key.as_str()) {
                                scope.push_error(format!(
                                    "Invalid property `{key}` on base64 {media_type}: `media_type` is the only supported property"
                                ));
                            }
                        }
                        Ok(BamlValueWithMeta::Media(
                            baml_types::BamlMedia::base64(*media_type, s.to_string(), mime_type),
                            field_type.clone(),
                        ))
                    } else {
                        scope.push_error(format!(
                            "Invalid media source: expected `file`, `url`, or `base64`, got `{value}`"
                        ));
                        Err(ArgCoerceError)
                    }
                }
                (_, _) => {
                    scope.push_error(format!("Expected type {t:?}, got `{value}`"));
                    Err(ArgCoerceError)
                }
            },
            TypeIR::Enum { name, .. } => match value {
                BamlValue::String(s) => {
                    if let Ok(e) = ir.find_enum(name) {
                        if e.walk_values().any(|v| v.item.elem.0 == *s)
                            || e.item.attributes.dynamic()
                        {
                            Ok(BamlValueWithMeta::Enum(
                                name.to_string(),
                                s.to_string(),
                                TypeIR::r#enum(name),
                            ))
                        } else {
                            scope.push_error(format!(
                                "Invalid enum {}: expected one of ({}), got `{}`",
                                name,
                                e.walk_values()
                                    .map(|v| v.item.elem.0.as_str())
                                    .collect::<Vec<&str>>()
                                    .join(" | "),
                                s
                            ));
                            Err(ArgCoerceError)
                        }
                    } else {
                        scope.push_error(format!("Enum {name} not found"));
                        Err(ArgCoerceError)
                    }
                }
                BamlValue::Enum(n, s) if n == name => Ok(BamlValueWithMeta::Enum(
                    name.to_string(),
                    s.to_string(),
                    TypeIR::r#enum(name),
                )),
                _ => {
                    scope.push_error(format!("Invalid enum {name}: Got `{value}`"));
                    Err(ArgCoerceError)
                }
            },
            TypeIR::Literal(literal, _) => match (literal, value) {
                (LiteralValue::Int(lit), BamlValue::Int(v)) if lit == v => {
                    Ok(BamlValueWithMeta::Int(*v, TypeIR::literal_int(*lit)))
                }
                (LiteralValue::String(lit), BamlValue::String(v)) if lit == v => Ok(
                    BamlValueWithMeta::String(v.clone(), TypeIR::literal_string(lit.clone())),
                ),
                (LiteralValue::Bool(lit), BamlValue::Bool(v)) if lit == v => {
                    Ok(BamlValueWithMeta::Bool(*v, TypeIR::literal_bool(*lit)))
                }
                _ => {
                    scope.push_error(format!("Expected literal {literal:?}, got `{value}`"));
                    Err(ArgCoerceError)
                }
            },
            TypeIR::Class { name, .. } => match value {
                BamlValue::Class(_, obj) | BamlValue::Map(obj) => match ir.find_class(name) {
                    Ok(c) => {
                        let mut fields = BamlMap::new();
                        let is_dynamic = c.item.attributes.dynamic();

                        // Process fields in the order they appear in the input object to preserve ordering
                        for (key, value) in obj {
                            // Check if this is a known class field first
                            if let Some(field) = c.walk_fields().find(|f| f.name() == key) {
                                if let Ok(v) = self.coerce_arg(ir, field.r#type(), value, scope) {
                                    fields.insert(key.clone(), v);
                                }
                            } else if is_dynamic {
                                // Handle dynamic field
                                let inferred_type = infer_type(value);
                                if let Some(inferred_type) = inferred_type {
                                    if let Ok(coerced_value) =
                                        self.coerce_arg(ir, &inferred_type, value, scope)
                                    {
                                        fields.insert(key.clone(), coerced_value);
                                    }
                                }
                            }
                        }

                        // Check for missing required fields
                        for f in c.walk_fields() {
                            if !fields.contains_key(f.name()) && !f.r#type().is_optional() {
                                scope.push_error(format!(
                                    "Missing required field `{}` for class {}",
                                    f.name(),
                                    name
                                ));
                            }
                        }

                        Ok(BamlValueWithMeta::Class(
                            name.to_string(),
                            fields,
                            TypeIR::class(name),
                        ))
                    }
                    Err(_) => {
                        scope.push_error(format!("Class {name} not found"));
                        Err(ArgCoerceError)
                    }
                },
                _ => {
                    scope.push_error(format!("Expected class {name}, got `{value}`"));
                    Err(ArgCoerceError)
                }
            },
            TypeIR::RecursiveTypeAlias { name, .. } => {
                let mut maybe_coerced = None;
                // TODO: Fix this O(n)
                for cycle in ir.structural_recursive_alias_cycles() {
                    if let Some(target) = cycle.get(name) {
                        maybe_coerced = Some(self.coerce_arg(ir, target, value, scope)?);
                        break;
                    }
                }

                match maybe_coerced {
                    Some(coerced) => Ok(coerced),
                    None => {
                        scope.push_error(format!("Recursive type alias {name} not found"));
                        Err(ArgCoerceError)
                    }
                }
            }
            TypeIR::List(item, _) => match value {
                BamlValue::List(arr) => {
                    let mut items = Vec::new();
                    for v in arr {
                        if let Ok(v) = self.coerce_arg(ir, item, v, scope) {
                            items.push(v);
                        }
                    }
                    Ok(BamlValueWithMeta::List(items, item.clone().as_list()))
                }
                _ => {
                    scope.push_error(format!("Expected array, got `{value}`"));
                    Err(ArgCoerceError)
                }
            },
            TypeIR::Tuple(_, _) => {
                scope.push_error("Tuples are not yet supported".to_string());
                Err(ArgCoerceError)
            }
            TypeIR::Map(k, v, _) => match value {
                BamlValue::Map(kv) => {
                    let mut map = BamlMap::new();
                    for (key, value) in kv {
                        scope.push("<key>".to_string());
                        let k = self.coerce_arg(ir, k, &BamlValue::String(key.clone()), scope);
                        scope.pop(false);

                        if k.is_ok() {
                            scope.push(key.to_string());
                            if let Ok(v) = self.coerce_arg(ir, v, value, scope) {
                                map.insert(key.clone(), v);
                            }
                            scope.pop(false);
                        }
                    }
                    Ok(BamlValueWithMeta::Map(map, (**v).clone()))
                }
                _ => {
                    scope.push_error(format!("Expected map, got `{value}`"));
                    Err(ArgCoerceError)
                }
            },
            TypeIR::Union(options, _) => {
                let mut first_good_result = Err(ArgCoerceError);
                for option in options.iter_include_null() {
                    let mut scope = ScopeStack::new();
                    if first_good_result.is_err() {
                        let result = self.coerce_arg(ir, option, value, &mut scope);
                        if !scope.has_errors() && first_good_result.is_err() {
                            first_good_result = result
                        }
                    }
                }
                if first_good_result.is_err() {
                    scope.push_error(format!("Expected one of {options:?}, got `{value}`"));
                    Err(ArgCoerceError)
                } else {
                    first_good_result
                }
            }
            TypeIR::Arrow(_, _) => {
                scope.push_error(String::from(
                    "A json value may not be coerced into a function type",
                ));
                Err(ArgCoerceError)
            }
        }?;

        let search_for_failures_result =
            first_failing_assert_nested(ir, &value.clone().value(), field_type).map_err(|e| {
                scope.push_error(format!("Failed to evaluate assert: {e:?}"));
                ArgCoerceError
            })?;

        match search_for_failures_result {
            Some(Constraint {
                label, expression, ..
            }) => {
                let msg = label.as_ref().unwrap_or(&expression.0);
                scope.push_error(format!("Failed assert: {msg}"));
                Err(ArgCoerceError)
            }
            None => Ok(value),
        }
    }
}

/// Search a potentially deeply-nested `BamlValue` for any failing asserts,
/// returning the first one encountered.
fn first_failing_assert_nested<'a>(
    ir: &'a IntermediateRepr,
    baml_value: &BamlValue,
    field_type: &'a TypeIR,
) -> anyhow::Result<Option<Constraint>> {
    let value_with_types = ir.distribute_type(baml_value.clone(), field_type.clone())?;
    let first_failure = value_with_types
        .iter()
        .map(|value_node| {
            let constraints = value_node.meta().meta().constraints.clone();
            constraints
                .into_iter()
                .filter_map(|c| {
                    let constraint = c.clone();
                    let baml_value: BamlValue = value_node.into();
                    let result = evaluate_predicate(&baml_value, &c.expression).map_err(|e| {
                        anyhow::anyhow!(format!("Error evaluating constraint: {:?}", e))
                    });
                    match result {
                        Ok(false) => {
                            if c.level == ConstraintLevel::Assert {
                                Some(Ok(constraint))
                            } else {
                                None
                            }
                        }
                        Ok(true) => None,
                        Err(e) => Some(Err(e)),
                    }
                })
                .collect::<Vec<_>>()
        })
        .flat_map(|x| x.into_iter())
        .next();
    first_failure.transpose()
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use baml_types::{
        type_meta::base::{StreamingBehavior, TypeMeta},
        JinjaExpression,
    };

    use super::*;
    use crate::ir::repr::make_test_ir;

    #[test]
    fn test_malformed_check_in_argument() {
        let ir = make_test_ir(
            r##"
            client<llm> GPT4 {
              provider openai
              options {
                model gpt-4o
                api_key env.OPENAI_API_KEY
              }
            }
            function Foo(a: int @assert(malformed, {{ this.length() > 0 }})) -> int {
              client GPT4
              prompt #""#
            }
            "##,
        )
        .unwrap();
        let value = BamlValue::Int(1);
        let type_ = TypeIR::Primitive(
            TypeValue::Int,
            TypeMeta {
                constraints: vec![Constraint {
                    level: ConstraintLevel::Assert,
                    expression: JinjaExpression("this.length() > 0".to_string()),
                    label: Some("foo".to_string()),
                }],
                streaming_behavior: StreamingBehavior::default(),
            },
        );
        let arg_coercer = ArgCoercer {
            span_path: None,
            allow_implicit_cast_to_string: true,
        };
        let res = arg_coercer.coerce_arg(&ir, &type_, &value, &mut ScopeStack::new());
        assert!(res.is_err());
    }

    #[test]
    fn test_mutually_recursive_aliases() {
        let ir = make_test_ir(
            r##"
type JsonValue = int | bool | float | string | JsonArray | JsonObject
type JsonObject = map<string, JsonValue>
type JsonArray = JsonValue[]
            "##,
        )
        .unwrap();

        let arg_coercer = ArgCoercer {
            span_path: None,
            allow_implicit_cast_to_string: true,
        };

        // let json = BamlValueWithMeta::Map(
        //     BamlMap::from([
        //         ("number".to_string(), BamlValue::Int(1)),
        //         ("string".to_string(), BamlValue::String("test".to_string())),
        //         ("bool".to_string(), BamlValue::Bool(true)),
        //     ]),
        //     FieldType::RecursiveTypeAlias("JsonValue".to_string()),
        // );

        // let res = arg_coercer.coerce_arg(
        //     &ir,
        //     &FieldType::RecursiveTypeAlias("JsonValue".to_string()),
        //     &json,
        //     &mut ScopeStack::new(),
        // );

        // assert_eq!(res, Ok(json));
    }

    #[test]
    fn test_file_as_string() {
        // Create a temporary file with known content
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_file_as_string.txt");
        let content = "Hello, this is test content!\nWith multiple lines.";

        let mut file = std::fs::File::create(&temp_file).expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write to temp file");

        let ir = make_test_ir(
            r##"
            client<llm> GPT4 {
              provider openai
              options {
                model gpt-4o
                api_key env.OPENAI_API_KEY
              }
            }
            function Foo(text: string) -> string {
              client GPT4
              prompt #"{{ text }}"#
            }
            "##,
        )
        .unwrap();

        // Create a BamlValue::Map with the "file" key pointing to our temp file
        let mut kv = BamlMap::new();
        kv.insert(
            "file".to_string(),
            BamlValue::String(temp_file.to_string_lossy().to_string()),
        );
        let value = BamlValue::Map(kv);

        // Use temp_dir as the span_path to resolve relative paths
        let arg_coercer = ArgCoercer {
            span_path: Some(temp_dir.join("fake_baml_file.baml")),
            allow_implicit_cast_to_string: true,
        };

        let result = arg_coercer.coerce_arg(&ir, &TypeIR::string(), &value, &mut ScopeStack::new());

        // Clean up temp file
        std::fs::remove_file(&temp_file).ok();

        // Verify the result
        match result {
            Ok(BamlValueWithMeta::String(s, _)) => {
                assert_eq!(s, content);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(_) => panic!("Expected Ok, got error"),
        }
    }

    #[test]
    fn test_file_as_string_relative_path() {
        // Create a temporary file with known content
        let temp_dir = std::env::temp_dir();
        let test_subdir = temp_dir.join("baml_test_subdir");
        std::fs::create_dir_all(&test_subdir).expect("Failed to create test subdir");

        let temp_file = test_subdir.join("relative_test.txt");
        let content = "Relative path content!";

        let mut file = std::fs::File::create(&temp_file).expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write to temp file");

        let ir = make_test_ir(
            r##"
            client<llm> GPT4 {
              provider openai
              options {
                model gpt-4o
                api_key env.OPENAI_API_KEY
              }
            }
            function Foo(text: string) -> string {
              client GPT4
              prompt #"{{ text }}"#
            }
            "##,
        )
        .unwrap();

        // Create a BamlValue::Map with the "file" key using a relative path
        let mut kv = BamlMap::new();
        kv.insert(
            "file".to_string(),
            BamlValue::String("relative_test.txt".to_string()),
        );
        let value = BamlValue::Map(kv);

        // The span_path is in the same directory as the file
        let arg_coercer = ArgCoercer {
            span_path: Some(test_subdir.join("fake_baml_file.baml")),
            allow_implicit_cast_to_string: true,
        };

        let result = arg_coercer.coerce_arg(&ir, &TypeIR::string(), &value, &mut ScopeStack::new());

        // Clean up
        std::fs::remove_file(&temp_file).ok();
        std::fs::remove_dir(&test_subdir).ok();

        // Verify the result
        match result {
            Ok(BamlValueWithMeta::String(s, _)) => {
                assert_eq!(s, content);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(_) => panic!("Expected Ok, got error"),
        }
    }

    #[test]
    fn test_file_as_string_missing_file() {
        let temp_dir = std::env::temp_dir();

        let ir = make_test_ir(
            r##"
            client<llm> GPT4 {
              provider openai
              options {
                model gpt-4o
                api_key env.OPENAI_API_KEY
              }
            }
            function Foo(text: string) -> string {
              client GPT4
              prompt #"{{ text }}"#
            }
            "##,
        )
        .unwrap();

        // Create a BamlValue::Map with the "file" key pointing to a non-existent file
        let mut kv = BamlMap::new();
        kv.insert(
            "file".to_string(),
            BamlValue::String("nonexistent_file_12345.txt".to_string()),
        );
        let value = BamlValue::Map(kv);

        let arg_coercer = ArgCoercer {
            span_path: Some(temp_dir.join("fake_baml_file.baml")),
            allow_implicit_cast_to_string: true,
        };

        let result = arg_coercer.coerce_arg(&ir, &TypeIR::string(), &value, &mut ScopeStack::new());

        // Should fail because file doesn't exist
        assert!(result.is_err());
    }
}
