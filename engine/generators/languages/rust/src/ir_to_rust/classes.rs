use crate::{
    generated_types::{RustClass, RustField},
    package::CurrentRenderPackage,
    r#type::SerializeType,
    utils::to_snake_case,
};
use internal_baml_core::ir::Class;

pub fn ir_class_to_rust(class: &Class, pkg: &CurrentRenderPackage) -> RustClass {
    let fields = class
        .elem
        .static_fields
        .iter()
        .map(|field| {
            let field_type = &field.elem.r#type.elem;
            let rust_type = crate::ir_to_rust::type_to_rust(
                &field_type.to_non_streaming_type(pkg.lookup()),
                pkg.lookup(),
            );
            RustField {
                name: to_snake_case(&field.elem.name),
                original_name: field.elem.name.clone(),
                rust_type: rust_type.serialize_type(pkg),
                optional: rust_type.meta().is_optional(),
            }
        })
        .collect();

    RustClass {
        name: class.elem.name.clone(),
        fields,
    }
}
