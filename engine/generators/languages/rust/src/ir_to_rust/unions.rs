use baml_types::ir_type::TypeNonStreaming;
use crate::{generated_types::RustUnion, package::CurrentRenderPackage};

pub fn ir_union_to_rust(
    union_type: &TypeNonStreaming,
    _pkg: &CurrentRenderPackage,
) -> Option<RustUnion> {
    // For now, create a simple union representation
    // TODO: Implement proper union type analysis
    Some(RustUnion {
        name: "UnionType".to_string(), // TODO: Generate proper names
        variants: vec!["serde_json::Value".to_string()], // Placeholder
    })
}