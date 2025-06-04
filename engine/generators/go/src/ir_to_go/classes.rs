use crate::generated_types::{ClassGo, FieldGo};
use internal_baml_core::ir::{Class, Field};

use crate::r#type::Package;


pub fn ir_class_to_go<'a>(class: &Class, pkg: &'a Package) -> ClassGo<'a> {
    ClassGo {
        name: class.elem.name.clone(),
        docstring: class.elem.docstring.clone().map(|docstring| docstring.0.clone()),
        dynamic: class.attributes.dynamic(),
        pkg,
        fields: class.elem.static_fields.iter().map(|field| ir_field_to_go(field, pkg)).collect(),
    }
}


fn ir_field_to_go<'a>(field: &Field, pkg: &'a Package) -> FieldGo<'a> {
    FieldGo {
        name: field.elem.name.clone(),
        r#type: super::type_to_go(&field.elem.r#type.elem, pkg),
        docstring: field.elem.docstring.clone().map(|docstring| docstring.0.clone()),
        pkg,
    }
}
