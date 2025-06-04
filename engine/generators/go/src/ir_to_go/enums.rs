use internal_baml_core::ir::Enum;

use crate::r#type::Package;

pub fn ir_enum_to_go(enum_: &Enum, _pkg: &Package) -> crate::generated_types::EnumGo {
    crate::generated_types::EnumGo {
        name: enum_.elem.name.clone(),
        values: enum_.elem.values.iter().map(|(val, doc_string)| (val.elem.0.clone(), doc_string.as_ref().map(|d| d.0.clone()))).collect(),
        docstring: enum_.elem.docstring.as_ref().map(|d| d.0.clone()),
        dynamic: enum_.attributes.dynamic(),
    }
}