use internal_baml_core::ir::Class;
use crate::{generated_types::{RustClass, RustField}, package::CurrentRenderPackage, r#type::to_rust_type, utils::safe_rust_identifier};

pub fn ir_class_to_rust(
    class: &Class,
    pkg: &CurrentRenderPackage,
) -> RustClass {
    let fields = class
        .elem
        .static_fields
        .iter()
        .map(|field| {
            let field_type = &field.elem.r#type.elem;
            RustField {
                name: safe_rust_identifier(&field.elem.name),
                rust_type: to_rust_type(&field_type.to_non_streaming_type(pkg.ir.as_ref())),
                optional: field_type.is_optional(),
            }
        })
        .collect();

    RustClass {
        name: class.elem.name.clone(),
        fields,
    }
}