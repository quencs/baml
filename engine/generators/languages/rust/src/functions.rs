use crate::package::CurrentRenderPackage;
use crate::r#type::{SerializeType, TypeRust};
use askama::Template;

mod filters {
    use crate::utils::to_snake_case;
    
    pub fn snake_case(s: &str, _args: &dyn askama::Values) -> askama::Result<String> {
        Ok(to_snake_case(s))
    }
}

#[derive(Debug, Clone)]
pub struct FunctionRust {
    pub documentation: Option<String>,
    pub name: String,
    pub args: Vec<(String, TypeRust)>, // (name, type)
    pub return_type: TypeRust,
    pub stream_return_type: TypeRust,
}


/// Template for the complete functions module
#[derive(askama::Template)]
#[template(path = "client.rs.j2", escape = "none")]
pub struct RustFunctions<'a> {
    functions: &'a [SingleFunctionRust<'a>],
}

/// Individual function template
#[derive(askama::Template, Clone)]
#[template(path = "function.rs.j2", escape = "none")]
pub struct SingleFunctionRust<'a> {
    pub documentation: Option<String>,
    pub name: String,
    pub args: Vec<(String, TypeRust)>,
    pub return_type: TypeRust,
    pub stream_return_type: TypeRust,
    pub pkg: &'a CurrentRenderPackage,
}

pub fn render_functions(
    functions: &[FunctionRust],
    pkg: &CurrentRenderPackage,
) -> Result<String, askama::Error> {
    use askama::Template;
    
    let single_functions: Vec<SingleFunctionRust> = functions.iter().map(|f| {
        SingleFunctionRust {
            documentation: f.documentation.clone(),
            name: f.name.clone(),
            args: f.args.clone(),
            return_type: f.return_type.clone(),
            stream_return_type: f.stream_return_type.clone(),
            pkg,
        }
    }).collect();
    
    RustFunctions { functions: &single_functions }.render()
}


pub fn render_source_files(_file_map: Vec<(String, String)>) -> Result<String, askama::Error> {
    Ok(r#"// Source file mapping
// TODO: Implement source map functionality
"#
    .to_string())
}
