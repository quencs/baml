use std::ops::Deref;

use baml_types::{ir_type::TypeStreaming, FieldType, ToUnionName};

use crate::{package::CurrentRenderPackage, r#type::TypeGo};

pub fn ir_union_to_go<'a>(union: &FieldType, pkg: &'a CurrentRenderPackage) -> Option<crate::generated_types::UnionGo<'a>> {
    let go_type = crate::ir_to_go::type_to_go(union);
    if let TypeGo::Union { name, .. } = go_type {
        let FieldType::Union(union_type_generic, _) = union else {
            panic!("ir_union_to_go expects a union. Got: {}", union);
        };
        let variants = union_type_generic.iter_skip_null().iter().map(|t| {
            let go_type = crate::ir_to_go::type_to_go(t);
            (go_type.default_name_within_union(), go_type)
        }).collect::<Vec<_>>();
        Some(crate::generated_types::UnionGo {
            name,
            cffi_name: union.to_union_name(),
            docstring: Some(format!("Generated from: {}", union)),
            variants,
            pkg,
        })
    } else {
        None
    }
}

pub fn ir_union_to_go_stream<'a>(union: &FieldType, pkg: &'a CurrentRenderPackage) -> Option<crate::generated_types::UnionGo<'a>> {

    let stream_union = union.partialize();
    let go_type = crate::ir_to_go::stream_type_to_go(&stream_union);
    if let TypeGo::Union { name, .. } = go_type {
        let TypeStreaming::Union(union_type_generic, _) = stream_union else {
            panic!("ir_union_to_go expects a union. Got: {}", stream_union);
        };
        let variants = union_type_generic.iter_skip_null().iter().map(|t| {
            let go_type = crate::ir_to_go::stream_type_to_go(t);
            (go_type.default_name_within_union(), go_type)
        }).collect::<Vec<_>>();
        Some(crate::generated_types::UnionGo {
            name,
            cffi_name: union.to_union_name(),
            docstring: Some(format!("Generated from: {}", union)),
            variants,
            pkg,
        })
    } else {
        None
    }
}