use askama::Template;
use baml_types::GeneratorDefaultClientMode;
use std::fmt;

use crate::{generated_types::{ClassTS}, package::CurrentRenderPackage, r#type::{SerializeType, TypeTS}};

impl fmt::Display for TypeTS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

pub struct FunctionTS {
    pub(crate) documentation: Option<String>,
    pub(crate) name: String,
    pub(crate) args: Vec<(String, TypeTS)>,
    pub(crate) return_type: TypeTS,
    pub(crate) stream_return_type: TypeTS,
}

#[derive(askama::Template)]
#[template(path = "base_client.ts.j2", escape = "none", ext = "txt")]
struct BaseClient<'a> {
    functions: &'a [FunctionTS],
    pkg: &'a CurrentRenderPackage,
    client_class: &'a str,
}

pub fn render_base_client(functions: &[FunctionTS], pkg: &CurrentRenderPackage) -> Result<String, askama::Error> {
    BaseClient {
        functions,
        pkg,
        client_class: "client_class",
    }.render()
}

#[derive(askama::Template)]
#[template(path = "async_client.ts.j2", escape = "none", ext = "txt")]
struct AsyncClient<'a> {
    functions: &'a [FunctionTS],
    pkg: &'a CurrentRenderPackage,
    client_class: &'a str,
}

pub fn render_async_client(functions: &[FunctionTS], pkg: &CurrentRenderPackage) -> Result<String, askama::Error> {
    AsyncClient {
        functions,
        pkg,
        client_class: "client_class",
    }.render()
}


#[derive(askama::Template)]
#[template(path = "sync_client.ts.j2", escape = "none")]
struct SyncClient<'a> {
    functions: &'a [FunctionTS],
    pkg: &'a CurrentRenderPackage,
    client_class: &'a str,
}

pub fn render_sync_client(functions: &[FunctionTS], pkg: &CurrentRenderPackage) -> Result<String, askama::Error> {
    SyncClient {
        functions,
        pkg,
        client_class: "client_class",
    }.render()
}



#[derive(askama::Template)]
#[template(path = "index.ts.j2", escape = "none", ext = "txt")]
struct Index<'a> {
    version: &'a str,
    default_client_mode: GeneratorDefaultClientMode,
}

pub fn render_index(default_client_mode: &GeneratorDefaultClientMode) -> Result<String, askama::Error> {
    Index {
        version: env!("CARGO_PKG_VERSION"),
        default_client_mode: default_client_mode.clone(),
    }.render()
}


pub fn render_globals(_pkg: &CurrentRenderPackage) -> Result<String, askama::Error> {
    Ok(include_str!("./_templates/globals.ts").to_string())
}


/// A map of file paths to their contents.
///
/// ```askama
/// package baml_client
///
/// var file_map = map[string]string{
/// {% for (path, contents) in file_map %}  
///   {{ path }}: {{ contents }},
/// {%- endfor %}  
/// }
///
/// func getBamlFiles() map[string]string {
///   return file_map
/// }
/// ```
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
struct SourceFiles<'a> {
    file_map: &'a [(String, String)],
}

pub fn render_source_files(file_map: Vec<(String, String)>) -> Result<String, askama::Error> {
    SourceFiles {
        file_map: &file_map,
    }.render()
}

