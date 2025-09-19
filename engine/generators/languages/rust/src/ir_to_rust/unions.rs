use crate::{
    generated_types::{RustUnion, RustVariant},
    package::CurrentRenderPackage,
    r#type::TypeRust,
};
use baml_types::ir_type::TypeNonStreaming;

pub fn ir_union_to_rust(
    union_type: &TypeNonStreaming,
    pkg: &CurrentRenderPackage,
) -> Option<RustUnion> {
    // Use the new type system to generate proper union types
    let rust_type = crate::ir_to_rust::type_to_rust(union_type, pkg.lookup());

    if let TypeRust::Union { name, .. } = rust_type {
        // Extract the union variants based on the union type
        match union_type {
            TypeNonStreaming::Union(union_type_generic, _) => {
                match union_type_generic.view() {
                    baml_types::ir_type::UnionTypeViewGeneric::Null => None,
                    baml_types::ir_type::UnionTypeViewGeneric::Optional(_) => None, // Handled as Option<T>
                    baml_types::ir_type::UnionTypeViewGeneric::OneOf(type_generics) => {
                        let variants = type_generics
                            .into_iter()
                            .enumerate()
                            .map(|(i, t)| {
                                let rust_type = crate::ir_to_rust::type_to_rust(t, pkg.lookup());
                                let variant_name = match &rust_type {
                                    TypeRust::String(_, _) => "String".to_string(),
                                    TypeRust::Int(_, _) => "Int".to_string(),
                                    TypeRust::Float(_) => "Float".to_string(),
                                    TypeRust::Bool(_, _) => "Bool".to_string(),
                                    TypeRust::Class { name, .. } => name.clone(),
                                    TypeRust::Enum { name, .. } => name.clone(),
                                    TypeRust::Union { name, .. } => name.clone(),
                                    TypeRust::List(_, _) => format!("List{}", i),
                                    TypeRust::Map(_, _, _) => format!("Map{}", i),
                                    TypeRust::Media(_, _) => format!("Media{}", i),
                                    TypeRust::TypeAlias { name, .. } => name.clone(),
                                    TypeRust::Any { .. } => format!("Any{}", i),
                                };

                                RustVariant {
                                    name: variant_name,
                                    rust_type,
                                    docstring: None,
                                    literal_value: None,
                                }
                            })
                            .collect();

                        Some(RustUnion {
                            name,
                            variants,
                            docstring: None,
                        })
                    }
                    baml_types::ir_type::UnionTypeViewGeneric::OneOfOptional(type_generics) => {
                        let variants = type_generics
                            .into_iter()
                            .enumerate()
                            .map(|(i, t)| {
                                let rust_type = crate::ir_to_rust::type_to_rust(t, pkg.lookup());
                                let variant_name = match &rust_type {
                                    TypeRust::String(_, _) => "String".to_string(),
                                    TypeRust::Int(_, _) => "Int".to_string(),
                                    TypeRust::Float(_) => "Float".to_string(),
                                    TypeRust::Bool(_, _) => "Bool".to_string(),
                                    TypeRust::Class { name, .. } => name.clone(),
                                    TypeRust::Enum { name, .. } => name.clone(),
                                    TypeRust::Union { name, .. } => name.clone(),
                                    TypeRust::List(_, _) => format!("List{}", i),
                                    TypeRust::Map(_, _, _) => format!("Map{}", i),
                                    TypeRust::Media(_, _) => format!("Media{}", i),
                                    TypeRust::TypeAlias { name, .. } => name.clone(),
                                    TypeRust::Any { .. } => format!("Any{}", i),
                                };

                                RustVariant {
                                    name: variant_name,
                                    rust_type,
                                    docstring: None,
                                    literal_value: None,
                                }
                            })
                            .collect();

                        Some(RustUnion {
                            name,
                            variants,
                            docstring: None,
                        })
                    }
                }
            }
            _ => None,
        }
    } else {
        None
    }
}
