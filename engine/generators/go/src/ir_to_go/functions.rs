// TODO: DO NOT EXPOSE THIS
use internal_baml_core::ir::FunctionNode;

use crate::{functions::FunctionGo, package::CurrentRenderPackage};
use super::{stream_type_to_go, type_to_go};

pub fn ir_function_to_go(function: &FunctionNode, pkg: &CurrentRenderPackage) -> FunctionGo {
    FunctionGo {
        documentation: None,
        name: function.elem.name().to_string(),
        args: function
            .elem
            .inputs()
            .iter()
            .map(|(name, field_type)| (name.clone(), type_to_go(field_type)))
            .collect(),
        return_type: type_to_go(function.elem.output()),
        stream_return_type: stream_type_to_go(&function.elem.output().partialize()),
    }
}
