use crate::generated_types::{ClassGo, FieldGo};
use internal_baml_core::ir::{Class, Field};

use crate::package::CurrentRenderPackage;


pub fn ir_class_to_go<'a>(class: &Class, pkg: &'a CurrentRenderPackage) -> ClassGo<'a> {
    ClassGo {
        name: class.elem.name.clone(),
        docstring: class.elem.docstring.clone().map(|docstring| docstring.0.clone()),
        dynamic: class.attributes.dynamic(),
        pkg,
        fields: class.elem.static_fields.iter().map(|field| ir_field_to_go(field, pkg)).collect(),
    }
}

pub fn ir_class_to_go_stream<'a>(class: &Class, pkg: &'a CurrentRenderPackage) -> ClassGo<'a> {
    ClassGo {
        name: class.elem.name.clone(),
        docstring: class.elem.docstring.clone().map(|docstring| docstring.0.clone()),
        dynamic: class.attributes.dynamic(),
        pkg,
        fields: class.elem.static_fields.iter().map(|field| ir_field_to_go_stream(field, pkg)).collect(),
    }
}


fn ir_field_to_go<'a>(field: &Field, pkg: &'a CurrentRenderPackage) -> FieldGo<'a> {
    FieldGo {
        name: field.elem.name.clone(),
        r#type: super::type_to_go(&field.elem.r#type.elem),
        docstring: field.elem.docstring.clone().map(|docstring| docstring.0.clone()),
        pkg,
    }
}

fn ir_field_to_go_stream<'a>(field: &Field, pkg: &'a CurrentRenderPackage) -> FieldGo<'a> {
    FieldGo {
        name: field.elem.name.clone(),
        r#type: super::stream_type_to_go(&field.elem.r#type.elem.partialize()),
        docstring: field.elem.docstring.clone().map(|docstring| docstring.0.clone()),
        pkg,
    }
}