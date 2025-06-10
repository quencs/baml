use crate::r#type::{MediaTypePython, TypeMetaPython, TypePython, TypeWrapper};
use baml_types::{
    ir_type::{Type, TypeStreaming},
    type_meta::base::TypeMeta,
    type_meta::stream::TypeMetaStreaming,
    BamlMediaType, ConstraintLevel, TypeValue,
};

use crate::package::Module;

pub mod classes;
pub mod enums;
pub mod functions;
pub mod type_aliases;
pub mod unions;

fn stream_type_to_go(field: &TypeStreaming) -> TypePython {
    use TypeStreaming as T;
    let recursive_fn = stream_type_to_go;
    let meta = stream_meta_to_go(field.meta());

    let TYPES_PKG: Module = Module::new("baml_client.types");
    let STREAM_PKG: Module = Module::new("baml_client.stream_types");

    let type_go: TypePython = match field {
        T::Primitive(type_value, _) => {
            let mut primitive_type_go: TypePython = type_value.into();
            // println!("field metadata: {:?}, type_go: {:?}", field.meta(), primitive_type_go);
            *primitive_type_go.meta_mut() = meta;
            primitive_type_go
        }
        T::Enum { name, dynamic, .. } => TypePython::Enum {
            package: TYPES_PKG.clone(),
            name: name.clone(),
            dynamic: *dynamic,
            meta,
        },
        T::Literal(literal_value, _) => match literal_value {
            baml_types::LiteralValue::String(_) => TypePython::String(meta),
            baml_types::LiteralValue::Int(_) => TypePython::Int(meta),
            baml_types::LiteralValue::Bool(_) => TypePython::Bool(meta),
        },
        T::Class {
            name,
            dynamic,
            mode,
            meta: cls_meta,
        } => TypePython::Class {
            package: match cls_meta.streaming_behavior.done {
                true => TYPES_PKG.clone(),
                false => STREAM_PKG.clone(),
            },
            name: name.clone(),
            dynamic: *dynamic,
            meta,
        },
        T::List(type_generic, _) => TypePython::List(Box::new(recursive_fn(type_generic)), meta),
        T::Map(type_generic, type_generic1, _) => TypePython::Map(
            Box::new(recursive_fn(type_generic)),
            Box::new(recursive_fn(type_generic1)),
            meta,
        ),
        T::RecursiveTypeAlias(name, _) => {
            // TODO: hack to generate correct types for recuriviely nullable types
            let mut meta = meta;
            meta.make_optional();
            TypePython::Class {
                package: STREAM_PKG.clone(),
                name: name.clone(),
                dynamic: false,
                meta,
            }
        }
        T::Tuple(..) => TypePython::Any {
            reason: "tuples are not supported in Go".to_string(),
            meta,
        },
        T::Arrow(..) => TypePython::Any {
            reason: "arrow types are not supported in Go".to_string(),
            meta,
        },
        T::Union(union_type_generic, union_meta) => match union_type_generic.view() {
            baml_types::ir_type::UnionTypeViewGeneric::Null => TypePython::Any {
                reason: "Null types are not supported in Go".to_string(),
                meta,
            },
            baml_types::ir_type::UnionTypeViewGeneric::Optional(type_generic) => {
                let mut type_go = recursive_fn(type_generic);
                *type_go.meta_mut() = meta;
                if union_meta
                    .constraints
                    .iter()
                    .any(|c| matches!(c.level, ConstraintLevel::Check))
                {
                    type_go.meta_mut().make_checked();
                }
                type_go.meta_mut().make_optional();
                type_go
            }
            baml_types::ir_type::UnionTypeViewGeneric::OneOf(type_generics) => {
                let options: Vec<_> = type_generics.into_iter().map(|t| recursive_fn(t)).collect();
                let num_options = options.len();
                let mut name = options
                    .iter()
                    .map(|t| t.default_name_within_union())
                    .collect::<Vec<_>>();
                name.sort();
                let name = name.join("Or");
                TypePython::Union {
                    package: STREAM_PKG.clone(),
                    name: format!("Union{}{}", num_options, name),
                    meta,
                }
            }
            baml_types::ir_type::UnionTypeViewGeneric::OneOfOptional(type_generics) => {
                let options: Vec<_> = type_generics.into_iter().map(|t| recursive_fn(t)).collect();
                let num_options = options.len();
                let mut name = options
                    .iter()
                    .map(|t| t.default_name_within_union())
                    .collect::<Vec<_>>();
                name.sort();
                let name = name.join("Or");
                let mut meta = meta;
                meta.make_optional();
                TypePython::Union {
                    package: match union_meta.streaming_behavior.done {
                        true => TYPES_PKG.clone(),
                        false => STREAM_PKG.clone(),
                    },
                    name: format!("Union{}{}", num_options, name),
                    meta,
                }
            }
        },
    };

    type_go
}

fn type_to_go(field: &Type) -> TypePython {
    use Type as T;
    let recursive_fn = type_to_go;
    let meta = meta_to_go(field.meta());

    let TYPE_PKG: Module = Module::new("baml_client.types");

    let type_go: TypePython = match field {
        T::Primitive(type_value, _) => {
            let mut primitive_type_go: TypePython = type_value.into();
            *primitive_type_go.meta_mut() = meta;
            primitive_type_go
        }
        T::Enum { name, dynamic, .. } => TypePython::Enum {
            package: TYPE_PKG.clone(),
            name: name.clone(),
            dynamic: *dynamic,
            meta,
        },
        T::Literal(literal_value, _) => match literal_value {
            baml_types::LiteralValue::String(_) => TypePython::String(meta),
            baml_types::LiteralValue::Int(_) => TypePython::Int(meta),
            baml_types::LiteralValue::Bool(_) => TypePython::Bool(meta),
        },
        T::Class { name, dynamic, .. } => TypePython::Class {
            package: TYPE_PKG.clone(),
            name: name.clone(),
            dynamic: *dynamic,
            meta,
        },
        T::List(type_generic, _) => TypePython::List(Box::new(recursive_fn(type_generic)), meta),
        T::Map(type_generic, type_generic1, _) => TypePython::Map(
            Box::new(recursive_fn(type_generic)),
            Box::new(recursive_fn(type_generic1)),
            meta,
        ),
        T::RecursiveTypeAlias(name, _) => {
            // TODO: hack to generate correct types for recuriviely nullable types
            let mut meta = meta;
            meta.make_optional();
            TypePython::Class {
                package: TYPE_PKG.clone(),
                name: name.clone(),
                dynamic: false,
                meta,
            }
        }
        T::Tuple(..) => TypePython::Any {
            reason: "tuples are not supported in Go".to_string(),
            meta,
        },
        T::Arrow(..) => TypePython::Any {
            reason: "arrow types are not supported in Go".to_string(),
            meta,
        },
        T::Union(union_type_generic, union_meta) => match union_type_generic.view() {
            baml_types::ir_type::UnionTypeViewGeneric::Null => TypePython::Any {
                reason: "Null types are not supported in Go".to_string(),
                meta,
            },
            baml_types::ir_type::UnionTypeViewGeneric::Optional(type_generic) => {
                let mut type_go = recursive_fn(type_generic);
                type_go.meta_mut().make_optional();
                if union_meta
                    .constraints
                    .iter()
                    .any(|c| matches!(c.level, ConstraintLevel::Check))
                {
                    type_go.meta_mut().make_checked();
                }
                type_go
            }
            baml_types::ir_type::UnionTypeViewGeneric::OneOf(type_generics) => {
                let options: Vec<_> = type_generics.into_iter().map(|t| recursive_fn(t)).collect();
                let num_options = options.len();
                let mut name = options
                    .iter()
                    .map(|t| t.default_name_within_union())
                    .collect::<Vec<_>>();
                name.sort();
                let name = name.join("Or");
                TypePython::Union {
                    package: TYPE_PKG.clone(),
                    name: format!("Union{}{}", num_options, name),
                    meta,
                }
            }
            baml_types::ir_type::UnionTypeViewGeneric::OneOfOptional(type_generics) => {
                let options: Vec<_> = type_generics.into_iter().map(|t| recursive_fn(t)).collect();
                let num_options = options.len();

                let mut name = options
                    .iter()
                    .map(|t| t.default_name_within_union())
                    .collect::<Vec<_>>();
                name.sort();
                let name = name.join("Or");

                let mut meta = meta;
                meta.make_optional();
                TypePython::Union {
                    package: TYPE_PKG.clone(),
                    name: format!("Union{}{}", num_options, name),
                    meta,
                }
            }
        },
    };

    type_go
}

// convert ir metadata to go metadata
fn meta_to_go(meta: &TypeMeta) -> TypeMetaPython {
    let has_checks = meta
        .constraints
        .iter()
        .any(|c| matches!(c.level, ConstraintLevel::Check));

    let wrapper = TypeWrapper::default();
    let wrapper = if has_checks {
        wrapper.as_checked()
    } else {
        wrapper
    };

    // optionality is handled by unions
    TypeMetaPython {
        type_wrapper: wrapper,
        wrap_stream_state: false,
    }
}

fn stream_meta_to_go(meta: &TypeMetaStreaming) -> TypeMetaPython {
    let has_checks = meta
        .constraints
        .iter()
        .any(|c| matches!(c.level, ConstraintLevel::Check));

    let wrapper = TypeWrapper::default();
    let wrapper = if has_checks {
        wrapper.as_checked()
    } else {
        wrapper
    };

    TypeMetaPython {
        type_wrapper: wrapper,
        wrap_stream_state: meta.streaming_behavior.state,
    }
}

impl From<&TypeValue> for TypePython {
    fn from(type_value: &TypeValue) -> Self {
        let meta = TypeMetaPython::default();
        match type_value {
            TypeValue::String => TypePython::String(meta),
            TypeValue::Int => TypePython::Int(meta),
            TypeValue::Float => TypePython::Float(meta),
            TypeValue::Bool => TypePython::Bool(meta),
            TypeValue::Null => TypePython::Any {
                reason: "Null types are not supported in Go".to_string(),
                meta,
            },
            TypeValue::Media(baml_media_type) => TypePython::Media(baml_media_type.into(), meta),
        }
    }
}

impl From<&BamlMediaType> for MediaTypePython {
    fn from(baml_media_type: &BamlMediaType) -> Self {
        match baml_media_type {
            BamlMediaType::Image => MediaTypePython::Image,
            BamlMediaType::Audio => MediaTypePython::Audio,
        }
    }
}

#[cfg(test)]
mod tests {}
