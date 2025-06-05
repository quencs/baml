use crate::r#type::{MediaTypeGo, TypeGo, TypeMetaGo, TypeWrapper};
use baml_types::{ir_type::{Type, TypeStreaming}, BamlMediaType, ConstraintLevel, type_meta::base::TypeMeta, type_meta::stream::TypeMetaStreaming, TypeValue};

use crate::package::Package;

pub mod functions;
pub mod classes;
pub mod enums;
pub mod unions;

fn stream_type_to_go(field: &TypeStreaming) -> TypeGo {
    use TypeStreaming as T;
    let recursive_fn = stream_type_to_go;
    let meta = stream_meta_to_go(field.meta());

    let TYPES_PKG: Package = Package::new("baml_client.types");
    let STREAM_PKG: Package = Package::new("baml_client.stream_types");

    let type_go: TypeGo = match field {
        T::Primitive(type_value, _) => {
            type_value.into()
        },
        T::Enum { name, dynamic, .. } => {
            TypeGo::Enum {
                package: TYPES_PKG.clone(),
                name: name.clone(),
                dynamic: *dynamic,
                meta
            }
        },
        T::Literal(literal_value, _) => {
            match literal_value {
                baml_types::LiteralValue::String(_) => TypeGo::String(meta),
                baml_types::LiteralValue::Int(_) => TypeGo::Int(meta),
                baml_types::LiteralValue::Bool(_) => TypeGo::Bool(meta),
            }
        },
        T::Class { name, dynamic, mode, .. } => {
            TypeGo::Class {
                package: match mode {
                    baml_types::StreamingMode::NonStreaming => TYPES_PKG.clone(),
                    baml_types::StreamingMode::Streaming => STREAM_PKG.clone(),
                },
                name: name.clone(),
                dynamic: *dynamic,
                meta
            }
        },
        T::List(type_generic, _) => {
            TypeGo::List(Box::new(recursive_fn(type_generic)), meta)
        },
        T::Map(type_generic, type_generic1, _) => {
            TypeGo::Map(Box::new(recursive_fn(type_generic)), Box::new(recursive_fn(type_generic1)), meta)
        },
        T::RecursiveTypeAlias(name, _) => {
            // TODO: we need mode here as well.
            TypeGo::Class {
                package: STREAM_PKG.clone(),
                name: name.clone(),
                dynamic: false,
                meta
            }
        },
        T::Tuple(..) => TypeGo::Any { reason: "tuples are not supported in Go".to_string(), meta },
        T::Arrow(..) => TypeGo::Any { reason: "arrow types are not supported in Go".to_string(), meta },
        T::Union(union_type_generic, union_meta) => {
            match union_type_generic.view() {
                baml_types::ir_type::UnionTypeViewGeneric::Null => TypeGo::Any { reason: "Null types are not supported in Go".to_string(), meta },
                baml_types::ir_type::UnionTypeViewGeneric::Optional(type_generic) => {
                    let mut type_go = recursive_fn(type_generic);
                    if union_meta.constraints.iter().any(|c| {
                        matches!(c.level, ConstraintLevel::Check)
                    }) {
                        type_go.meta_mut().make_checked();
                    }
                    type_go.meta_mut().make_optional();
                    type_go
                },
                baml_types::ir_type::UnionTypeViewGeneric::OneOf(type_generics) => {
                    let options: Vec<_> = type_generics.into_iter().map(|t| recursive_fn(t)).collect();
                    let num_options = options.len();
                    let name = options.iter().map(|t| t.default_name_within_union()).collect::<Vec<_>>().join("Or");
                    TypeGo::Union {
                        package: STREAM_PKG.clone(),
                        name: format!("Union{}{}", num_options, name),
                        meta
                    }
                },
                baml_types::ir_type::UnionTypeViewGeneric::OneOfOptional(type_generics) => {
                    let options: Vec<_> = type_generics.into_iter().map(|t| recursive_fn(t)).collect();
                    let num_options = options.len();
                    let name = options.iter().map(|t| t.default_name_within_union()).collect::<Vec<_>>().join("Or");
                    let mut meta = meta;
                    meta.make_optional();
                    TypeGo::Union {
                        package: STREAM_PKG.clone(),
                        name: format!("Union{}{}", num_options, name),
                        meta,
                    }
                },
            }
        },
    };

    type_go
}

fn type_to_go(field: &Type) -> TypeGo {
    use Type as T;
    let recursive_fn = type_to_go;
    let meta = meta_to_go(field.meta());

    let TYPE_PKG: Package = Package::new("baml_client.types");

    let type_go: TypeGo = match field {
        T::Primitive(type_value, _) => {
            type_value.into()
        },
        T::Enum { name, dynamic, .. } => {
            TypeGo::Enum {
                package: TYPE_PKG.clone(),
                name: name.clone(),
                dynamic: *dynamic,
                meta
            }
        },
        T::Literal(literal_value, _) => {
            match literal_value {
                baml_types::LiteralValue::String(_) => TypeGo::String(meta),
                baml_types::LiteralValue::Int(_) => TypeGo::Int(meta),
                baml_types::LiteralValue::Bool(_) => TypeGo::Bool(meta),
            }
        },
        T::Class { name, dynamic, .. } => {
            TypeGo::Class {
                package: TYPE_PKG.clone(),
                name: name.clone(),
                dynamic: *dynamic,
                meta
            }
        },
        T::List(type_generic, _) => {
            TypeGo::List(Box::new(recursive_fn(type_generic)), meta)
        },
        T::Map(type_generic, type_generic1, _) => {
            TypeGo::Map(Box::new(recursive_fn(type_generic)), Box::new(recursive_fn(type_generic1)), meta)
        },
        T::RecursiveTypeAlias(name, _) => {
            TypeGo::Class {
                package: TYPE_PKG.clone(),
                name: name.clone(),
                dynamic: false,
                meta
            }
        },
        T::Tuple(..) => TypeGo::Any { reason: "tuples are not supported in Go".to_string(), meta },
        T::Arrow(..) => TypeGo::Any { reason: "arrow types are not supported in Go".to_string(), meta },
        T::Union(union_type_generic, union_meta) => {         
            match union_type_generic.view() {
                baml_types::ir_type::UnionTypeViewGeneric::Null => TypeGo::Any { reason: "Null types are not supported in Go".to_string(), meta },
                baml_types::ir_type::UnionTypeViewGeneric::Optional(type_generic) => {
                    let mut type_go = recursive_fn(type_generic);
                    type_go.meta_mut().make_optional();
                    if union_meta.constraints.iter().any(|c| {
                        matches!(c.level, ConstraintLevel::Check)
                    }) {
                        type_go.meta_mut().make_checked();
                    }
                    type_go
                },
                baml_types::ir_type::UnionTypeViewGeneric::OneOf(type_generics) => {
                    let options: Vec<_> = type_generics.into_iter().map(|t| recursive_fn(t)).collect();
                    let num_options = options.len();
                    let mut name = options.iter().map(|t| t.default_name_within_union()).collect::<Vec<_>>();
                    name.sort();
                    let name = name.join("Or");
                    TypeGo::Union {
                        package: TYPE_PKG.clone(),
                        name: format!("Union{}{}", num_options, name),
                        meta
                    }
                },
                baml_types::ir_type::UnionTypeViewGeneric::OneOfOptional(type_generics) => {
                    let options: Vec<_> = type_generics.into_iter().map(|t| recursive_fn(t)).collect();
                    let num_options = options.len();
                    
                    let mut name = options.iter().map(|t| t.default_name_within_union()).collect::<Vec<_>>();
                    name.sort();
                    let name = name.join("Or");
                    
                    let mut meta = meta;
                    meta.make_optional();
                    TypeGo::Union {
                        package: TYPE_PKG.clone(),
                        name: format!("Union{}{}", num_options, name),
                        meta,
                    }
                },
            }
        },
    };

    type_go
}

// convert ir metadata to go metadata
fn meta_to_go(meta: &TypeMeta) -> TypeMetaGo {
    let has_checks = meta.constraints.iter().any(|c| {
        matches!(c.level, ConstraintLevel::Check)
    });

    let wrapper = TypeWrapper::default();
    let wrapper = if has_checks {
        wrapper.as_checked()
    } else {
        wrapper
    };

    // optionality is handled by unions
    TypeMetaGo {
        type_wrapper: wrapper,
        wrap_stream_state: false,
    }
}

fn stream_meta_to_go(meta: &TypeMetaStreaming) -> TypeMetaGo {
    let has_checks = meta.constraints.iter().any(|c| {
        matches!(c.level, ConstraintLevel::Check)
    });

    let wrapper = TypeWrapper::default();
    let wrapper = if has_checks {
        wrapper.as_checked()
    } else {
        wrapper
    };

    TypeMetaGo {
        type_wrapper: wrapper,
        wrap_stream_state: meta.streaming_behavior.state,
    }
}


impl From<&TypeValue> for TypeGo {
    fn from(type_value: &TypeValue) -> Self {
        let meta = TypeMetaGo::default();
        match type_value {
            TypeValue::String => TypeGo::String(meta),
            TypeValue::Int => TypeGo::Int(meta),
            TypeValue::Float => TypeGo::Float(meta),
            TypeValue::Bool => TypeGo::Bool(meta),
            TypeValue::Null => TypeGo::Any { reason: "Null types are not supported in Go".to_string(), meta },
            TypeValue::Media(baml_media_type) => TypeGo::Media(baml_media_type.into(), meta),
        }
    }
}

impl From<&BamlMediaType> for MediaTypeGo {
    fn from(baml_media_type: &BamlMediaType) -> Self {
        match baml_media_type {
            BamlMediaType::Image => MediaTypeGo::Image,
            BamlMediaType::Audio => MediaTypeGo::Audio,
        }
    }
}

#[cfg(test)]
mod tests{
    
}