use baml_types::ir_type::{TypeNonStreaming, TypeValue};

pub fn to_rust_type(ty: &TypeNonStreaming) -> String {
    match ty {
        TypeNonStreaming::Primitive(prim, _) => match prim {
            TypeValue::String => "String".to_string(),
            TypeValue::Int => "i64".to_string(),
            TypeValue::Float => "f64".to_string(),
            TypeValue::Bool => "bool".to_string(),
            TypeValue::Null => "()".to_string(),
            TypeValue::Media(media_type) => match media_type {
                baml_types::BamlMediaType::Image => "BamlImage".to_string(),
                baml_types::BamlMediaType::Audio => "BamlAudio".to_string(),
                baml_types::BamlMediaType::Pdf => "BamlPdf".to_string(),
                baml_types::BamlMediaType::Video => "BamlVideo".to_string(),
            },
        },
        TypeNonStreaming::Class { name, .. } => name.clone(),
        TypeNonStreaming::Enum { name, .. } => name.clone(),
        TypeNonStreaming::List(inner, _) => format!("Vec<{}>", to_rust_type(inner)),
        TypeNonStreaming::Map(_, value, _) => format!("std::collections::HashMap<String, {}>", to_rust_type(value)),
        TypeNonStreaming::Union(inner, _) => {
            // For now, use a simple approach for unions
            // TODO: Implement proper enum-based unions
            "serde_json::Value".to_string()
        }
        TypeNonStreaming::Literal(lit, _) => match lit {
            baml_types::LiteralValue::String(_) => "String".to_string(),
            baml_types::LiteralValue::Int(_) => "i64".to_string(),
            baml_types::LiteralValue::Bool(_) => "bool".to_string(),
        },
        TypeNonStreaming::Tuple(_, _) => "serde_json::Value".to_string(), // Fallback for tuples
        TypeNonStreaming::RecursiveTypeAlias { .. } => "serde_json::Value".to_string(), // Fallback
        TypeNonStreaming::Arrow(_, _) => "serde_json::Value".to_string(), // Fallback for function types
    }
}

pub fn is_optional(ty: &TypeNonStreaming) -> bool {
    // Check if this is a union with null
    match ty {
        TypeNonStreaming::Union(inner, _) => {
            // TODO: Check if union contains null
            false
        }
        _ => false,
    }
}