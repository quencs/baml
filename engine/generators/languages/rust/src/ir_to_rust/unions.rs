use baml_types::ir_type::TypeNonStreaming;
use crate::{generated_types::RustUnion, package::CurrentRenderPackage, r#type::{SerializeType, TypeRust}};

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
                            .map(|t| {
                                let rust_type = crate::ir_to_rust::type_to_rust(t, pkg.lookup());
                                rust_type.serialize_type(pkg)
                            })
                            .collect();
                        
                        Some(RustUnion {
                            name,
                            variants,
                        })
                    }
                    baml_types::ir_type::UnionTypeViewGeneric::OneOfOptional(type_generics) => {
                        let variants = type_generics
                            .into_iter()
                            .map(|t| {
                                let rust_type = crate::ir_to_rust::type_to_rust(t, pkg.lookup());
                                rust_type.serialize_type(pkg)
                            })
                            .collect();
                        
                        Some(RustUnion {
                            name,
                            variants,
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