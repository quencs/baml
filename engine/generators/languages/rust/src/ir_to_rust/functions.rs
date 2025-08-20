use internal_baml_core::ir::FunctionNode;
use crate::{functions::RustFunction, package::CurrentRenderPackage, r#type::to_rust_type};

pub fn ir_function_to_rust(
    func: &FunctionNode,
    pkg: &CurrentRenderPackage,
) -> RustFunction {
    let args = func
        .elem
        .inputs()
        .iter()
        .map(|(name, ty)| (name.clone(), to_rust_type(&ty.to_non_streaming_type(pkg.ir.as_ref()))))
        .collect();

    let return_type = to_rust_type(&func.elem.output().to_non_streaming_type(pkg.ir.as_ref()));

    RustFunction {
        name: func.elem.name().to_string(),
        args,
        return_type,
    }
}