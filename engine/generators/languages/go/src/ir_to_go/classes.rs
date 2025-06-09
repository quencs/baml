use crate::generated_types::{ClassGo, FieldGo};
use internal_baml_core::ir::{Class, Field};

use crate::package::CurrentRenderPackage;

pub fn ir_class_to_go<'a>(class: &Class, pkg: &'a CurrentRenderPackage) -> ClassGo<'a> {
    ClassGo {
        name: class.elem.name.clone(),
        docstring: class
            .elem
            .docstring
            .clone()
            .map(|docstring| docstring.0.clone()),
        dynamic: class.attributes.dynamic(),
        pkg,
        fields: class
            .elem
            .static_fields
            .iter()
            .map(|field| ir_field_to_go(field, pkg))
            .collect(),
    }
}

pub fn ir_class_to_go_stream<'a>(class: &Class, pkg: &'a CurrentRenderPackage) -> ClassGo<'a> {
    ClassGo {
        name: class.elem.name.clone(),
        docstring: class
            .elem
            .docstring
            .clone()
            .map(|docstring| docstring.0.clone()),
        dynamic: class.attributes.dynamic(),
        pkg,
        fields: class
            .elem
            .static_fields
            .iter()
            .map(|field| ir_field_to_go_stream(field, pkg))
            .collect(),
    }
}

fn ir_field_to_go<'a>(field: &Field, pkg: &'a CurrentRenderPackage) -> FieldGo<'a> {
    FieldGo {
        name: field.elem.name.clone(),
        r#type: super::type_to_go(&field.elem.r#type.elem),
        docstring: field
            .elem
            .docstring
            .clone()
            .map(|docstring| docstring.0.clone()),
        pkg,
    }
}

fn ir_field_to_go_stream<'a>(field: &Field, pkg: &'a CurrentRenderPackage) -> FieldGo<'a> {
    let partialized_type = field
        .elem
        .r#type
        .elem
        .partialize(field.attributes.streaming_behavior().needed);
    let type_go = super::stream_type_to_go(&partialized_type);
    FieldGo {
        name: field.elem.name.clone(),
        r#type: type_go,
        docstring: field
            .elem
            .docstring
            .clone()
            .map(|docstring| docstring.0.clone()),
        pkg,
    }
}

#[cfg(test)]
mod tests {
    use internal_baml_core::ir::{repr::make_test_ir, IRHelper};

    use crate::{
        package::Package,
        r#type::{TypeGo, TypeMetaGo},
    };

    use super::*;

    #[test]
    fn test_ir_class_to_go() {
        let ir = make_test_ir(
            r#"
            class SimpleClass {
                words string @stream.with_state
            }
        "#,
        )
        .unwrap();
        let class = ir.find_class("SimpleClass").unwrap().item;
        let pkg = CurrentRenderPackage::new("baml_client");
        let class_go = ir_class_to_go_stream(&class, &pkg);
        assert_eq!(class_go.name, "SimpleClass");
        assert_eq!(class_go.fields.len(), 1);
        assert_eq!(class_go.fields[0].r#type.meta().wrap_stream_state, true);
        println!("{}", class_go.fields[0]);
    }

    #[test]
    fn test_ir_class_to_go_needed_field() {
        let ir = make_test_ir(
            r#"
            class ChildClass {
                digits int @stream.with_state @stream.not_null
            }
        "#,
        )
        .unwrap();
        let class = ir.find_class("ChildClass").unwrap().item;
        let pkg = CurrentRenderPackage::new("baml_client");
        let class_go = ir_class_to_go_stream(&class, &pkg);
        let digits_field = class_go.fields.iter().find(|f| f.name == "digits").unwrap();
        eprintln!("{:?}", digits_field);
        assert!(digits_field.r#type.meta().wrap_stream_state);
        assert_eq!(class_go.name, "ChildClass");
        assert_eq!(class_go.fields.len(), 1);
        println!("{}", class_go.fields[0]);
    }
}
