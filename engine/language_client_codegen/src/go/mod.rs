mod generate_types;
mod go_language_features;

use std::{fmt::format, path::PathBuf};

use anyhow::Result;
use generate_types::{cast_value, to_go_literal, GoType, ToTypeReferenceInTypeDefinition};
use indexmap::{IndexMap, IndexSet};
use internal_baml_core::{
    configuration::{GeneratorDefaultClientMode, GeneratorOutputType},
    ir::{repr::IntermediateRepr, FieldType, IRHelper},
};

use self::go_language_features::{GoLanguageFeatures, ToGo};
use crate::{dir_writer::FileCollector, field_type_attributes};

#[derive(askama::Template)]
#[template(path = "client.go.j2", escape = "none")]
struct GoClient {
    package_name: String,
    funcs: Vec<GoFunction>,
}

struct GoFunction {
    name: String,
    go_name: String,
    partial_return_type: String,
    return_type: String,
    return_type_type: GoType,
    args: Vec<(String, String)>,
}

#[derive(askama::Template)]
#[template(path = "inlinedbaml.go.j2", escape = "none")]
struct InlinedBaml {
    file_map: Vec<(String, String)>,
}

pub(crate) fn generate(
    ir: &IntermediateRepr,
    generator: &crate::GeneratorArgs,
) -> Result<IndexMap<PathBuf, String>> {
    let mut collector = FileCollector::<GoLanguageFeatures>::new();

    collector.add_template::<GoClient>("client.go", (ir, generator))?;
    collector.add_template::<InlinedBaml>("inlinedbaml.go", (ir, generator))?;
    collector.add_template::<generate_types::GoEncode>("encode.go", (ir, generator))?;
    collector.add_template::<generate_types::GoTypes>("types/types.go", (ir, generator))?;
    collector.add_template::<generate_types::GoEnums>("types/enums.go", (ir, generator))?;
    collector.add_template::<generate_types::GoUnions>("types/unions.go", (ir, generator))?;
    collector.add_template::<generate_types::GoStreamTypes>(
        "stream_types/stream_types.go",
        (ir, generator),
    )?;

    collector.commit(&generator.output_dir())
}

impl TryFrom<(&'_ IntermediateRepr, &'_ crate::GeneratorArgs)> for InlinedBaml {
    type Error = anyhow::Error;

    fn try_from((_ir, args): (&IntermediateRepr, &crate::GeneratorArgs)) -> Result<Self> {
        Ok(InlinedBaml {
            file_map: args.file_map()?,
        })
    }
}

impl TryFrom<(&'_ IntermediateRepr, &'_ crate::GeneratorArgs)> for GoClient {
    type Error = anyhow::Error;

    fn try_from((ir, args): (&'_ IntermediateRepr, &'_ crate::GeneratorArgs)) -> Result<Self> {
        let functions = ir
            .walk_functions()
            .map(|f| {
                let configs = f.walk_impls();

                let funcs = configs
                    .into_iter()
                    .map(|c| {
                        let (_function, _impl_) = c.item;
                        Ok(GoFunction {
                            name: f.name().to_string(),
                            go_name: {
                                let mut name = f.name().to_string();
                                if let Some(first_char) = name.get_mut(0..1) {
                                    first_char.make_ascii_lowercase();
                                }
                                name
                            },
                            partial_return_type: f.elem().output().to_partial_type_ref(ir, true),
                            return_type: f.elem().output().to_type_ref(ir, true),
                            return_type_type: f.elem().output().to_type_ref_2(ir, true),
                            args: f
                                .inputs()
                                .iter()
                                .map(|(name, r#type)| {
                                    (name.to_string(), r#type.to_type_ref(ir, false))
                                })
                                .collect(),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(funcs)
            })
            .collect::<Result<Vec<Vec<GoFunction>>>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(GoClient {
            package_name: args.client_package_name.clone().unwrap_or_default(),
            funcs: functions,
        })
    }
}

trait ToUnionName {
    fn to_union_name(&self) -> String;
    fn find_union_types(&self) -> IndexSet<FieldType>;
}

impl ToUnionName for FieldType {
    fn find_union_types(&self) -> IndexSet<FieldType> {
        // TODO: its pretty hard to get type aliases here
        let value = self.simplify();
        match &value {
            FieldType::Union(_) => IndexSet::from_iter([value]),
            FieldType::List(inner) => inner.find_union_types(),
            FieldType::Map(field_type, field_type1) => {
                let mut set = field_type.find_union_types();
                set.extend(field_type1.find_union_types());
                set
            }
            FieldType::Primitive(_)
            | FieldType::Enum(_)
            | FieldType::Literal(_)
            | FieldType::Class(_)
            | FieldType::RecursiveTypeAlias(_) => IndexSet::new(),
            FieldType::Tuple(inner) => inner.iter().flat_map(|t| t.find_union_types()).collect(),
            FieldType::Optional(inner) => inner.find_union_types(),
            FieldType::WithMetadata { base, .. } => base.find_union_types(),
        }
    }

    fn to_union_name(&self) -> String {
        match self {
            baml_types::FieldType::Primitive(type_value) => type_value.to_go(),
            baml_types::FieldType::Enum(name) => name.to_string(),
            baml_types::FieldType::Literal(literal_value) => match literal_value {
                baml_types::LiteralValue::String(value) => format!(
                    "string_{}",
                    value
                        .chars()
                        .map(|c| if c.is_alphanumeric() { c } else { '_' })
                        .collect::<String>()
                ),
                baml_types::LiteralValue::Int(val) => format!("int_{}", val.to_string()),
                baml_types::LiteralValue::Bool(val) => format!("bool_{}", val.to_string()),
            },
            baml_types::FieldType::Class(name) => name.to_string(),
            baml_types::FieldType::List(field_type) => {
                format!("List__{}", field_type.to_union_name())
            }
            baml_types::FieldType::Map(field_type, field_type1) => {
                format!(
                    "Map__{}_{}",
                    field_type.to_union_name(),
                    field_type1.to_union_name()
                )
            }
            baml_types::FieldType::Union(field_types) => format!(
                "Union__{}",
                field_types
                    .iter()
                    .map(|v| v.to_union_name())
                    .collect::<Vec<_>>()
                    .join("__")
                    .to_string()
            ),
            baml_types::FieldType::Tuple(field_types) => format!(
                "Tuple__{}",
                field_types
                    .iter()
                    .map(|v| v.to_union_name())
                    .collect::<Vec<_>>()
                    .join("__")
                    .to_string()
            ),
            baml_types::FieldType::Optional(field_type) => {
                format!("Optional__{}", field_type.to_union_name())
            }
            baml_types::FieldType::RecursiveTypeAlias(name) => name.to_string(),
            baml_types::FieldType::WithMetadata {
                base,
                constraints,
                streaming_behavior,
            } => base.to_union_name(),
        }
    }
}

trait ToTypeReferenceInClientDefinition {
    fn to_type_ref(&self, ir: &IntermediateRepr, with_checked: bool) -> String;
    fn to_type_ref_impl(&self, ir: &IntermediateRepr, with_checked: bool) -> String;
    fn to_partial_type_ref(&self, ir: &IntermediateRepr, with_checked: bool) -> String;
    fn to_partial_type_ref_impl(&self, ir: &IntermediateRepr, with_checked: bool) -> String;
}

impl ToTypeReferenceInClientDefinition for FieldType {
    fn to_type_ref(&self, ir: &IntermediateRepr, with_checked: bool) -> String {
        self.simplify().to_type_ref_impl(ir, with_checked)
    }
    fn to_partial_type_ref(&self, ir: &IntermediateRepr, with_checked: bool) -> String {
        self.simplify().to_partial_type_ref_impl(ir, with_checked)
    }
    fn to_type_ref_impl(&self, ir: &IntermediateRepr, _with_checked: bool) -> String {
        match self {
            FieldType::Enum(name) => {
                // enums handle the dynamic types internally
                format!("types.{name}")
            }
            FieldType::Literal(value) => to_go_literal(value),
            FieldType::RecursiveTypeAlias(name) => format!("types.{name}"),
            FieldType::Class(name) => format!("types.{name}"),
            FieldType::List(inner) => format!("[]{}", inner.to_type_ref(ir, _with_checked)),
            FieldType::Map(key, value) => {
                format!(
                    "map[{}]{}",
                    key.to_type_ref(ir, _with_checked),
                    value.to_type_ref(ir, _with_checked)
                )
            }
            FieldType::Primitive(r#type) => r#type.to_go(),
            FieldType::Union(inner) => format!("types.{}", self.to_union_name()),
            FieldType::Tuple(inner) => format!(
                "Tuple[{}]",
                inner
                    .iter()
                    .map(|t| t.to_type_ref(ir, _with_checked))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            FieldType::Optional(inner) => {
                format!("*{}", inner.to_type_ref(ir, _with_checked))
            }
            FieldType::WithMetadata { base, .. } => match field_type_attributes(self) {
                Some(_) => {
                    let base_type_ref = base.to_type_ref(ir, _with_checked);
                    format!("types.Checked[{base_type_ref}]")
                }
                None => base.to_type_ref(ir, _with_checked),
            },
        }
    }

    fn to_partial_type_ref_impl(&self, ir: &IntermediateRepr, with_checked: bool) -> String {
        match self {
            FieldType::Enum(name) => {
                if ir
                    .find_enum(name)
                    .map(|e| e.item.attributes.get("dynamic_type").is_some())
                    .unwrap_or(false)
                {
                    format!("*Union[types.{name}, str]")
                } else {
                    format!("*types.{name}")
                }
            }
            FieldType::Class(name) => format!("partial_types.{name}"),
            FieldType::RecursiveTypeAlias(name) => format!("types.{name}"),
            FieldType::Literal(value) => format!("*{}", to_go_literal(value)),
            FieldType::List(inner) => {
                format!("[]{}", inner.to_partial_type_ref(ir, with_checked))
            }
            FieldType::Map(key, value) => {
                format!(
                    "map[{}]{}",
                    key.to_type_ref(ir, with_checked),
                    value.to_partial_type_ref(ir, with_checked)
                )
            }
            FieldType::Primitive(r#type) => format!("*{}", r#type.to_go()),
            FieldType::Union(inner) => format!(
                "*Union[{}]",
                inner
                    .iter()
                    .map(|t| t.to_partial_type_ref(ir, with_checked))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            FieldType::Tuple(inner) => format!(
                "*Tuple[{}]",
                inner
                    .iter()
                    .map(|t| t.to_partial_type_ref(ir, with_checked))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            FieldType::Optional(inner) => inner.to_partial_type_ref(ir, with_checked),
            FieldType::WithMetadata { base, .. } => match field_type_attributes(self) {
                Some(_) => {
                    let base_type_ref = base.to_partial_type_ref(ir, with_checked);
                    format!("Checked[{base_type_ref}]")
                }
                None => base.to_partial_type_ref(ir, with_checked),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use internal_baml_core::ir::repr::make_test_ir;

    use crate::GeneratorArgs;

    use super::*;

    fn mk_ir() -> IntermediateRepr {
        make_test_ir(
            r#"
class Greg {
  inner Foo? @stream.not_null @stream.with_state @check(foo, {{ true }})
}

class Foo {
  s string
}

// class Foo {
//   i int @stream.not_null @stream.with_state
//   b Bar @stream.done
// }

// class Foo {
//   str string @stream.with_state
// }
//
// class Inner {
//   inner_int int
//   inner_string string @stream.not_null
//   inner_string_2 string @stream.not_null @stream.done
// }
//
// class InnerDone {
//   inner_done_inner Inner @stream.done
//   inner_done_int int
//   inner_done_str string
//   @@stream.done
// }
        "#,
        )
        .unwrap()
    }

    fn mk_gen() -> GeneratorArgs {
        GeneratorArgs::new(
            "baml_client",
            "baml_src",
            vec![],
            "no_version".to_string(),
            true,
            GeneratorDefaultClientMode::Async,
            Vec::new(),
            Some(GeneratorOutputType::Go),
            Some("example.com/integ-tests".to_string()),
        )
        .unwrap()
    }

    #[test]
    fn generate_streaming_go() {
        let ir = mk_ir();
        let generator_args = mk_gen();
        let res = generate(&ir, &generator_args).unwrap();
        let partial_types = res.get(&PathBuf::from("partial_types.go")).unwrap();
        eprintln!("{}", partial_types);
    }
}
