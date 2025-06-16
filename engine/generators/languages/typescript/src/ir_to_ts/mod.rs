use crate::package::Package;
use crate::r#type::{MediaTypeTS, TypeMetaTS, TypeTS, TypeWrapper};
use baml_types::{
    baml_value::TypeLookups,
    ir_type::{Type, TypeStreaming},
    type_meta::base::TypeMeta,
    type_meta::stream::TypeMetaStreaming,
    BamlMediaType, ConstraintLevel, TypeValue,
};

pub mod classes;
// pub mod enums;
pub mod functions;
// pub mod type_aliases;
// pub mod unions;

pub(crate) fn stream_type_to_ts(field: &TypeStreaming, _lookup: &impl TypeLookups) -> TypeTS {
    use TypeStreaming as T;
    let recursive_fn = |field| stream_type_to_ts(field, _lookup);
    let meta = stream_meta_to_ts(field.meta());

    let types_pkg: Package = Package::types();
    let stream_pkg: Package = Package::stream_types();

    let type_ts: TypeTS = match field {
        T::Primitive(type_value, _) => {
            let t: TypeTS = type_value.into();
            t.with_meta(meta)
        }
        T::Enum { name, dynamic, .. } => TypeTS::Enum {
            package: types_pkg.clone(),
            name: name.clone(),
            dynamic: *dynamic,
            meta,
        },
        T::Literal(literal_value, _) => match literal_value {
            baml_types::LiteralValue::String(_) => TypeTS::String(meta),
            baml_types::LiteralValue::Int(_) => TypeTS::Int(meta),
            baml_types::LiteralValue::Bool(_) => TypeTS::Bool(meta),
        },
        T::Class {
            name,
            dynamic,
            meta: cls_meta,
            ..
        } => TypeTS::Class {
            package: match cls_meta.streaming_behavior.done {
                true => types_pkg.clone(),
                false => stream_pkg.clone(),
            },
            name: name.clone(),
            dynamic: *dynamic,
            meta,
        },
        T::List(type_generic, _) => TypeTS::List(Box::new(recursive_fn(type_generic)), meta),
        T::Map(type_generic, type_generic1, _) => TypeTS::Map(
            Box::new(recursive_fn(type_generic)),
            Box::new(recursive_fn(type_generic1)),
            meta,
        ),
        T::RecursiveTypeAlias {
            name,
            meta: alias_meta,
            ..
        } => TypeTS::TypeAlias {
            package: match alias_meta.streaming_behavior.done {
                true => types_pkg.clone(),
                false => stream_pkg.clone(),
            },
            name: name.clone(),
            meta,
        },
        T::Tuple(..) => TypeTS::Any {
            reason: "tuples are not supported in Go".to_string(),
            meta,
        },
        T::Arrow(..) => TypeTS::Any {
            reason: "arrow types are not supported in Go".to_string(),
            meta,
        },
        T::Union(union_type_generic, union_meta) => match union_type_generic.view() {
            baml_types::ir_type::UnionTypeViewGeneric::Null => TypeTS::Any {
                reason: "Null types are not supported in Go".to_string(),
                meta,
            },
            baml_types::ir_type::UnionTypeViewGeneric::Optional(type_generic) => {
                let mut type_go = recursive_fn(type_generic);
                if union_meta
                    .constraints
                    .iter()
                    .any(|c| matches!(c.level, ConstraintLevel::Check))
                {
                    type_go.meta_mut().make_checked();
                }
                type_go.meta_mut().make_optional();
                if union_meta.streaming_behavior.state {
                    type_go.meta_mut().set_stream_state();
                }
                type_go
            }
            baml_types::ir_type::UnionTypeViewGeneric::OneOf(type_generics) => {
                let options: Vec<_> = type_generics.into_iter().map(&recursive_fn).collect();
                let num_options = options.len();
                let mut name = options
                    .iter()
                    .map(|t| t.default_name_within_union())
                    .collect::<Vec<_>>();
                name.sort();
                let name = name.join("Or");
                TypeTS::Union {
                    package: stream_pkg.clone(),
                    name: format!("Union{}{}", num_options, name),
                    meta,
                }
            }
            baml_types::ir_type::UnionTypeViewGeneric::OneOfOptional(type_generics) => {
                let options: Vec<_> = type_generics.into_iter().map(recursive_fn).collect();
                let num_options = options.len();
                let mut name = options
                    .iter()
                    .map(|t| t.default_name_within_union())
                    .collect::<Vec<_>>();
                name.sort();
                let name = name.join("Or");
                let mut meta = meta;
                meta.make_optional();
                TypeTS::Union {
                    package: match union_meta.streaming_behavior.done {
                        true => types_pkg.clone(),
                        false => stream_pkg.clone(),
                    },
                    name: format!("Union{}{}", num_options, name),
                    meta,
                }
            }
        },
    };

    type_ts
}

pub(crate) fn type_to_ts(field: &Type, _lookup: &impl TypeLookups) -> TypeTS {
    use Type as T;
    let recursive_fn = |field| type_to_ts(field, _lookup);
    let meta = meta_to_ts(field.meta());

    let type_pkg = Package::types();

    let type_ts = match field {
        T::Primitive(type_value, _) => type_value.into(),
        T::Enum { name, dynamic, .. } => TypeTS::Enum {
            package: type_pkg.clone(),
            name: name.clone(),
            dynamic: *dynamic,
            meta,
        },
        T::Literal(literal_value, _) => match literal_value {
            baml_types::LiteralValue::String(_) => TypeTS::String(meta),
            baml_types::LiteralValue::Int(_) => TypeTS::Int(meta),
            baml_types::LiteralValue::Bool(_) => TypeTS::Bool(meta),
        },
        T::Class { name, dynamic, .. } => TypeTS::Class {
            package: type_pkg.clone(),
            name: name.clone(),
            dynamic: *dynamic,
            meta,
        },
        T::List(type_generic, _) => TypeTS::List(Box::new(recursive_fn(type_generic)), meta),
        T::Map(type_generic, type_generic1, _) => TypeTS::Map(
            Box::new(recursive_fn(type_generic)),
            Box::new(recursive_fn(type_generic1)),
            meta,
        ),
        T::Tuple(..) => TypeTS::Any {
            reason: "tuples are not supported in Typescript".to_string(),
            meta,
        },
        T::Arrow(..) => TypeTS::Any {
            reason: "arrow types are not supported in Typescript".to_string(),
            meta,
        },
        T::RecursiveTypeAlias { name, .. } => TypeTS::TypeAlias {
            package: type_pkg.clone(),
            name: name.clone(),
            meta,
        },
        T::Union(union_type_generic, union_meta) => match union_type_generic.view() {
            baml_types::ir_type::UnionTypeViewGeneric::Null => TypeTS::Any {
                reason: "Null types are not supported in Typescript".to_string(),
                meta,
            },
            baml_types::ir_type::UnionTypeViewGeneric::Optional(type_generic) => {
                let mut type_ts = recursive_fn(type_generic);
                type_ts.meta_mut().make_optional();
                if union_meta
                    .constraints
                    .iter()
                    .any(|c| matches!(c.level, ConstraintLevel::Check))
                {
                    type_ts.meta_mut().make_checked();
                }
                type_ts
            }
            baml_types::ir_type::UnionTypeViewGeneric::OneOf(type_generics) => {
                let options: Vec<_> = type_generics.into_iter().map(&recursive_fn).collect();
                let num_options = options.len();
                let mut name = options
                    .iter()
                    .map(|t| t.default_name_within_union())
                    .collect::<Vec<_>>();
                name.sort();
                let name = name.join("Or");
                TypeTS::Union {
                    package: type_pkg.clone(),
                    name: format!("Union{}{}", num_options, name),
                    meta,
                }
            }
            baml_types::ir_type::UnionTypeViewGeneric::OneOfOptional(type_generics) => {
                let options: Vec<_> = type_generics.into_iter().map(recursive_fn).collect();
                let num_options = options.len();

                let mut name = options
                    .iter()
                    .map(|t| t.default_name_within_union())
                    .collect::<Vec<_>>();
                name.sort();
                let name = name.join("Or");

                let mut meta = meta;
                meta.make_optional();
                TypeTS::Union {
                    package: type_pkg.clone(),
                    name: format!("Union{}{}", num_options, name),
                    meta,
                }
            }
        },
    };

    type_ts
}

// convert ir metadata to go metadata
fn meta_to_ts(meta: &TypeMeta) -> TypeMetaTS {
    let has_checks = meta
        .constraints
        .iter()
        .any(|c| matches!(c.level, ConstraintLevel::Check));

    let wrapper = TypeWrapper::default();
    let wrapper = if has_checks {
        wrapper.wrap_with_checked()
    } else {
        wrapper
    };

    // optionality is handled by unions
    TypeMetaTS {
        type_wrapper: wrapper,
        wrap_stream_state: false,
    }
}

fn stream_meta_to_ts(meta: &TypeMetaStreaming) -> TypeMetaTS {
    let has_checks = meta
        .constraints
        .iter()
        .any(|c| matches!(c.level, ConstraintLevel::Check));

    let wrapper = TypeWrapper::default();
    let wrapper = if has_checks {
        wrapper.wrap_with_checked()
    } else {
        wrapper
    };

    TypeMetaTS {
        type_wrapper: wrapper,
        wrap_stream_state: meta.streaming_behavior.state,
    }
}

impl From<&TypeValue> for TypeTS {
    fn from(type_value: &TypeValue) -> Self {
        let meta = TypeMetaTS::default();
        match type_value {
            TypeValue::String => TypeTS::String(meta),
            TypeValue::Int => TypeTS::Int(meta),
            TypeValue::Float => TypeTS::Float(meta),
            TypeValue::Bool => TypeTS::Bool(meta),
            TypeValue::Null => TypeTS::Any {
                reason: "Null types are not supported in Typescript".to_string(),
                meta,
            },
            TypeValue::Media(baml_media_type) => TypeTS::Media(baml_media_type.into(), meta),
        }
    }
}

impl From<&BamlMediaType> for MediaTypeTS {
    fn from(baml_media_type: &BamlMediaType) -> Self {
        match baml_media_type {
            BamlMediaType::Image => MediaTypeTS::Image,
            BamlMediaType::Audio => MediaTypeTS::Audio,
        }
    }
}

#[cfg(test)]
mod tests {}
