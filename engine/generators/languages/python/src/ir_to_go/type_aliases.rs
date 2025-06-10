use crate::generated_types::TypeAliasPython;
use crate::ir_to_go;
use crate::package::CurrentRenderModule;
use internal_baml_core::ir::TypeAlias;

pub fn ir_type_alias_to_go<'a>(
    alias: &TypeAlias,
    pkg: &'a CurrentRenderModule,
) -> TypeAliasPython<'a> {
    TypeAliasPython {
        name: alias.elem.name.clone(),
        type_: ir_to_go::type_to_go(&alias.elem.r#type.elem),
        docstring: alias
            .elem
            .docstring
            .clone()
            .map(|docstring| docstring.0.clone()),
        pkg,
    }
}

pub fn ir_type_alias_to_go_stream<'a>(
    alias: &TypeAlias,
    pkg: &'a CurrentRenderModule,
) -> TypeAliasPython<'a> {
    TypeAliasPython {
        name: alias.elem.name.clone(),
        type_: ir_to_go::stream_type_to_go(&alias.elem.r#type.elem.partialize(false)),
        docstring: alias
            .elem
            .docstring
            .clone()
            .map(|docstring| docstring.0.clone()),
        pkg,
    }
}
