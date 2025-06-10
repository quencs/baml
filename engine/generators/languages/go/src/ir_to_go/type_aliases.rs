use crate::generated_types::TypeAliasGo;
use crate::ir_to_go;
use crate::package::CurrentRenderPackage;
use baml_types::baml_value::TypeLookups;
use internal_baml_core::ir::TypeAlias;


pub fn ir_type_alias_to_go<'a>(alias: &TypeAlias, pkg: &'a CurrentRenderPackage, lookup: &impl TypeLookups) -> TypeAliasGo<'a> {
    TypeAliasGo {
        name: alias.elem.name.clone(),
        type_: ir_to_go::type_to_go(&alias.elem.r#type.elem, lookup),
        docstring: alias.elem.docstring.clone().map(|docstring| docstring.0.clone()),
        pkg,
    }
}

pub fn ir_type_alias_to_go_stream<'a>(alias: &TypeAlias, pkg: &'a CurrentRenderPackage, lookup: &impl TypeLookups) -> TypeAliasGo<'a> {
    TypeAliasGo {
        name: alias.elem.name.clone(),
        type_: ir_to_go::stream_type_to_go(&alias.elem.r#type.elem.partialize(), lookup),
        docstring: alias.elem.docstring.clone().map(|docstring| docstring.0.clone()),
        pkg,
    }
}