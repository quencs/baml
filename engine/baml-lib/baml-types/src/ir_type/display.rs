use crate::ir_type::UnionTypeViewGeneric;

use super::{ConstraintLevel, StreamingBehavior, TypeGeneric, TypeMeta, TypeStreaming};

impl std::fmt::Display for TypeStreaming {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut metadata_display_fmt = String::new();

        for constraint in &self.meta().constraints {
            // " @check( the_name, {{..}} )"
            let constraint_level = match constraint.level {
                ConstraintLevel::Assert => "assert",
                ConstraintLevel::Check => "check",
            };
            let constraint_name = match &constraint.label {
                None => "".to_string(),
                Some(label) => format!("{}, ", label),
            };
            metadata_display_fmt.push_str(&format!(
                " @{constraint_level}({constraint_name}, {{{{..}}}} )"
            ));
        }
        let StreamingBehavior {
            done,
            state,
            ..
        } = self.meta().streaming_behavior;
        if done {
            metadata_display_fmt.push_str(" @stream.done")
        }

        // TODO: stream.not_null is irrelevant for type streaming since it's represented in the union with a null
        // if needed {
        //     metadata_display_fmt.push_str(" @stream.not_null")
        // }
        if state {
            metadata_display_fmt.push_str(" @stream.with_state")
        }

        let _res = match self {
            TypeStreaming::Enum { name, .. }
            | TypeStreaming::Class { name, .. }
            | TypeStreaming::RecursiveTypeAlias(name, _) => write!(f, "{name}"),
            TypeStreaming::Primitive(t, _) => write!(f, "{t}"),
            TypeStreaming::Literal(v, _) => write!(f, "{v}"),
            TypeStreaming::Union(choices, _) => {
                let view = choices.view();
                let res = match view {
                    UnionTypeViewGeneric::Null => "null".to_string(),
                    UnionTypeViewGeneric::Optional(field_type) => {
                        format!("{} | null", field_type.to_string())
                    }
                    UnionTypeViewGeneric::OneOf(field_types) => field_types
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" | "),
                    UnionTypeViewGeneric::OneOfOptional(field_types) => {
                        let not_null_choices_str = field_types
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(" | ");
                        format!("{} | null", not_null_choices_str)
                    }
                };
                write!(f, "({res})")
            }
            TypeStreaming::Tuple(choices, _) => {
                write!(
                    f,
                    "({})",
                    choices
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TypeStreaming::Map(k, v, _) => write!(f, "map<{k}, {v}>"),
            TypeStreaming::List(t, _) => write!(f, "{t}[]"),
            TypeStreaming::Arrow(arrow, _) => write!(
                f,
                "({}) -> {}",
                arrow
                    .param_types
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                arrow.return_type.to_string()
            ),
        }?;

        write!(f, "{}", metadata_display_fmt)
    }
}


impl std::fmt::Display for TypeGeneric<TypeMeta> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut metadata_display_fmt = String::new();

        for constraint in &self.meta().constraints {
            // " @check( the_name, {{..}} )"
            let constraint_level = match constraint.level {
                ConstraintLevel::Assert => "assert",
                ConstraintLevel::Check => "check",
            };
            let constraint_name = match &constraint.label {
                None => "".to_string(),
                Some(label) => format!("{}, ", label),
            };
            metadata_display_fmt.push_str(&format!(
                " @{constraint_level}({constraint_name}, {{{{..}}}} )"
            ));
        }
        let StreamingBehavior {
            done,
            needed,
            state,
            ..
        } = self.streaming_behavior();
        if *done {
            metadata_display_fmt.push_str(" @stream.done")
        }
        if *needed {
            metadata_display_fmt.push_str(" @stream.not_null")
        }
        if *state {
            metadata_display_fmt.push_str(" @stream.with_state")
        }

        let _res = match self {
            TypeGeneric::Enum { name, .. }
            | TypeGeneric::Class { name, .. }
            | TypeGeneric::RecursiveTypeAlias(name, _) => write!(f, "{name}"),
            TypeGeneric::Primitive(t, _) => write!(f, "{t}"),
            TypeGeneric::Literal(v, _) => write!(f, "{v}"),
            TypeGeneric::Union(choices, _) => {
                let view = choices.view();
                let res = match view {
                    UnionTypeViewGeneric::Null => "null".to_string(),
                    UnionTypeViewGeneric::Optional(field_type) => {
                        format!("{} | null", field_type.to_string())
                    }
                    UnionTypeViewGeneric::OneOf(field_types) => field_types
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" | "),
                    UnionTypeViewGeneric::OneOfOptional(field_types) => {
                        let not_null_choices_str = field_types
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(" | ");
                        format!("{} | null", not_null_choices_str)
                    }
                };
                write!(f, "({res})")
            }
            TypeGeneric::Tuple(choices, _) => {
                write!(
                    f,
                    "({})",
                    choices
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TypeGeneric::Map(k, v, _) => write!(f, "map<{k}, {v}>"),
            TypeGeneric::List(t, _) => write!(f, "{t}[]"),
            TypeGeneric::Arrow(arrow, _) => write!(
                f,
                "({}) -> {}",
                arrow
                    .param_types
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                arrow.return_type.to_string()
            ),
        }?;

        write!(f, "{}", metadata_display_fmt)
    }
}