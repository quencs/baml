use crate::package::CurrentRenderPackage;

use crate::r#type::TypeRust;

#[derive(Debug, Clone)]
pub struct FunctionRust {
    pub documentation: Option<String>,
    pub name: String,
    pub args: Vec<(String, TypeRust)>, // (name, type)
    pub return_type: TypeRust,
    pub stream_return_type: TypeRust,
}

use crate::r#type::SerializeType;

pub fn render_functions(
    functions: &[FunctionRust],
    pkg: &CurrentRenderPackage,
) -> Result<String, anyhow::Error> {
    let mut output = String::new();
    output.push_str("use crate::runtime::BamlClient;\n");
    output.push_str("use crate::types::*;\n");
    output.push_str("use baml_client_rust::{BamlResult, StreamState};\n");
    output.push_str("use std::collections::HashMap;\n\n");

    for func in functions {
        // Generate synchronous function
        let args_str = func
            .args
            .iter()
            .map(|(name, ty)| format!("{}: {}", name, ty.serialize_type(pkg)))
            .collect::<Vec<_>>()
            .join(", ");

        let return_type = func.return_type.serialize_type(pkg);
        let stream_return_type = func.stream_return_type.serialize_type(pkg);

        output.push_str(&format!(
            r#"impl BamlClient {{
    pub async fn {}(&self, {}) -> BamlResult<{}> {{
        // TODO: Implement actual function call
        todo!("Function {} not yet implemented")
    }}

    pub async fn {}_stream(&self, {}) -> BamlResult<impl futures::Stream<Item = {}>> {{
        // TODO: Implement streaming function call
        todo!("Streaming function {} not yet implemented")
    }}
}}

"#,
            func.name.to_lowercase(),
            args_str,
            return_type,
            func.name,
            func.name.to_lowercase(),
            args_str,
            stream_return_type,
            func.name
        ));
    }

    Ok(output)
}

pub fn render_runtime_code(_pkg: &CurrentRenderPackage) -> Result<String, anyhow::Error> {
    Ok(
        r#"use baml_client_rust::{BamlRuntime as CoreRuntime, BamlClient as CoreClient};

pub type BamlRuntime = CoreRuntime;
pub type BamlClient = CoreClient;

impl BamlClient {
    pub fn new() -> Self {
        // TODO: Initialize with proper configuration
        todo!("BamlClient::new not yet implemented")
    }
}
"#
        .to_string(),
    )
}

pub fn render_source_files(_file_map: Vec<(String, String)>) -> Result<String, anyhow::Error> {
    Ok(r#"// Source file mapping
// TODO: Implement source map functionality
"#
    .to_string())
}
