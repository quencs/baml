use crate::r#type::{MediaTypeGo, Package, TypeGo, TypeMetaGo, TypeWrapper};
use baml_types::{ir_type::{Type, TypeStreaming}, BamlMediaType, ConstraintLevel, TypeMeta, TypeMetaStreaming, TypeValue};

pub mod functions;

fn stream_type_to_go(field: &TypeStreaming, type_pkg: &Package) -> TypeGo {
    use TypeStreaming as T;
    let recursive_fn = stream_type_to_go;
    let meta = stream_meta_to_go(field.meta());

    let type_go: TypeGo = match field {
        T::Primitive(type_value, _) => {
            type_value.into()
        },
        T::Enum { name, dynamic, .. } => {
            TypeGo::Enum {
                package: type_pkg.clone(),
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
                package: type_pkg.clone(),
                name: name.clone(),
                dynamic: *dynamic,
                meta
            }
        },
        T::List(type_generic, _) => {
            TypeGo::List(Box::new(recursive_fn(type_generic, type_pkg)), meta)
        },
        T::Map(type_generic, type_generic1, _) => {
            TypeGo::Map(Box::new(recursive_fn(type_generic, type_pkg)), Box::new(recursive_fn(type_generic1, type_pkg)), meta)
        },
        T::RecursiveTypeAlias(name, _) => {
            TypeGo::Class {
                package: type_pkg.clone(),
                name: name.clone(),
                dynamic: false,
                meta
            }
        },
        T::Tuple(..) => TypeGo::Any { reason: "tuples are not supported in Go".to_string(), meta },
        T::Arrow(..) => TypeGo::Any { reason: "arrow types are not supported in Go".to_string(), meta },
        T::Union(union_type_generic, _) => {
            let options: Vec<_> = union_type_generic.iter_skip_null().into_iter().map(|t| recursive_fn(t, type_pkg)).collect();
            let meta = if union_type_generic.is_optional() {
                let mut meta = meta;
                meta.type_wrapper = meta.type_wrapper.as_optional();
                meta
            } else {
                meta
            };
            let num_options = options.len();
            let name = options.iter().map(|t| t.default_name_within_union()).collect::<Vec<_>>().join("Or");
            TypeGo::Union {
                package: type_pkg.clone(),
                name: format!("Union{}{}", num_options, name),
                meta
            }
        },
    };

    type_go
}

fn type_to_go(field: &Type, type_pkg: &Package) -> TypeGo {
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
            TypeGo::List(Box::new(recursive_fn(type_generic, type_pkg)), meta)
        },
        T::Map(type_generic, type_generic1, _) => {
            TypeGo::Map(Box::new(recursive_fn(type_generic, type_pkg)), Box::new(recursive_fn(type_generic1, type_pkg)), meta)
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
        T::Union(union_type_generic, _) => {
            let options: Vec<_> = union_type_generic.iter_skip_null().into_iter().map(|t| recursive_fn(t, type_pkg)).collect();
            let meta = if union_type_generic.is_optional() {
                let mut meta = meta;
                meta.type_wrapper = meta.type_wrapper.as_optional();
                meta
            } else {
                meta
            };
            let num_options = options.len();
            let name = options.iter().map(|t| t.default_name_within_union()).collect::<Vec<_>>().join("Or");
            TypeGo::Union {
                package: TYPE_PKG.clone(),
                name: format!("Union{}{}", num_options, name),
                meta
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