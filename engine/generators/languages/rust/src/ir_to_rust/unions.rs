use crate::{
    generated_types::{RustLiteralKind, RustUnion, RustVariant},
    package::CurrentRenderPackage,
    r#type::TypeRust,
};
use baml_types::ir_type::TypeNonStreaming;
use std::collections::HashSet;

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
                        let mut seen_names = HashSet::new();
                        let variants = type_generics
                            .into_iter()
                            .enumerate()
                            .map(|(i, t)| {
                                let rust_type = crate::ir_to_rust::type_to_rust(t, pkg.lookup());
                                build_variant(i, rust_type, &mut seen_names)
                            })
                            .collect();

                        Some(RustUnion {
                            name,
                            variants,
                            docstring: None,
                        })
                    }
                    baml_types::ir_type::UnionTypeViewGeneric::OneOfOptional(type_generics) => {
                        let mut seen_names = HashSet::new();
                        let variants = type_generics
                            .into_iter()
                            .enumerate()
                            .map(|(i, t)| {
                                let rust_type = crate::ir_to_rust::type_to_rust(t, pkg.lookup());
                                build_variant(i, rust_type, &mut seen_names)
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

fn build_variant(
    index: usize,
    rust_type: TypeRust,
    seen_names: &mut HashSet<String>,
) -> RustVariant {
    let mut variant_name = rust_type.default_name_within_union();

    if !seen_names.insert(variant_name.clone()) {
        let mut counter = index;
        loop {
            let candidate = format!("{}{}", variant_name, counter);
            counter += 1;
            if seen_names.insert(candidate.clone()) {
                variant_name = candidate;
                break;
            }
        }
    }

    let (literal_value, literal_kind) = match &rust_type {
        TypeRust::String(Some(value), _) => (
            Some(value.clone()),
            Some(RustLiteralKind::String),
        ),
        TypeRust::Int(Some(value), _) => (
            Some(value.to_string()),
            Some(RustLiteralKind::Int),
        ),
        TypeRust::Bool(Some(value), _) => (
            Some(value.to_string()),
            Some(RustLiteralKind::Bool),
        ),
        _ => (None, None),
    };

    RustVariant {
        name: variant_name,
        rust_type,
        docstring: None,
        literal_value,
        literal_kind,
    }
}
