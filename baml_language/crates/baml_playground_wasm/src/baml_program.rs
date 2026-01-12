use std::path::PathBuf;

use baml_db::{baml_compiler_hir, baml_compiler_tir, baml_workspace, SourceFile};
use baml_project::ProjectDatabase;
use baml_program::function_lookup;
use wasm_bindgen::prelude::*;

/// A basic runtime wrapper around BAML source content.
#[wasm_bindgen]
pub struct BamlProgram {
    baml_src: String,
    db: ProjectDatabase,
    project: baml_workspace::Project,
    source_file: Option<SourceFile>,
}

#[wasm_bindgen]
impl BamlProgram {
    #[wasm_bindgen(constructor)]
    pub fn new(baml_src: String) -> BamlProgram {
        use baml_db::Setter;

        let mut db = ProjectDatabase::new();

        // Create a project with a virtual file path
        let project = db.set_project_root(&PathBuf::from("/baml_src"));

        // Add the source file to the database
        let source_file = db.add_file("/baml_src/main.baml", &baml_src);

        // Wire up the project to include this file
        project.set_files(&mut db).to(vec![source_file]);

        BamlProgram {
            baml_src,
            db,
            project,
            source_file: Some(source_file),
        }
    }

    /// Allows updating the stored BAML source for subsequent renders.
    ///
    /// This uses Salsa's incremental computation - only queries affected
    /// by the text change will be recomputed on subsequent calls.
    #[wasm_bindgen]
    pub fn set_source(&mut self, baml_src: String) {
        use baml_db::Setter;

        self.baml_src = baml_src.clone();

        // Update the source file in the Salsa database
        // This marks dependent queries as potentially stale
        if let Some(source_file) = self.source_file {
            source_file.set_text(&mut self.db).to(baml_src);
        }
    }

    /// Returns the names of all functions defined in the BAML project.
    ///
    /// This uses Salsa's `list_function_names` tracked query, which:
    /// 1. Depends on `project_items` → `file_items` → `file_item_tree`
    /// 2. Is memoized - subsequent calls return cached results if source unchanged
    /// 3. Only recomputes when function signatures change (not body edits)
    #[wasm_bindgen]
    pub fn function_names(&self) -> Vec<String> {
        let mut names: Vec<String> = baml_compiler_hir::list_function_names(&self.db, self.project)
            .into_iter()
            .map(|(name, _span)| name)
            .collect();
        names.push("injected-hot-reload4".to_string());
        names
    }

    /// Convenience helper returning the raw BAML source currently stored.
    #[wasm_bindgen(getter)]
    pub fn baml_src(&self) -> String {
        self.baml_src.clone()
    }

    /// Get the typed body of a function, including type inference results.
    ///
    /// Returns a `FunctionTypedBodyResult` containing:
    /// - The pretty-printed typed IR tree
    /// - Any type errors found during inference
    /// - Function metadata (signature, body kind)
    #[wasm_bindgen]
    pub fn get_function_typed_body(&self, function_name: &str) -> FunctionTypedBodyResult {
        // Step 1: Find the function by name
        let func_loc = match self.find_function_by_name(function_name) {
            Some(loc) => loc,
            None => {
                return FunctionTypedBodyResult {
                    success: false,
                    error: Some(format!("Function '{}' not found", function_name)),
                    tree: None,
                    type_errors: vec![],
                    signature: None,
                    body_kind: None,
                };
            }
        };

        // Step 2: Get signature and body from HIR
        let signature = baml_compiler_hir::function_signature(&self.db, func_loc);
        let body = baml_compiler_hir::function_body(&self.db, func_loc);

        // Step 3: Determine body kind
        let body_kind = match body.as_ref() {
            baml_compiler_hir::FunctionBody::Llm(_) => "llm",
            baml_compiler_hir::FunctionBody::Expr(_) => "expr",
            baml_compiler_hir::FunctionBody::Missing => "missing",
        };

        // Step 4: Build typing context
        let globals_map = baml_compiler_tir::typing_context(&self.db, self.project);
        let class_fields_map = baml_compiler_tir::class_field_types(&self.db, self.project);
        let type_aliases_map = baml_compiler_tir::type_aliases(&self.db, self.project);
        let enum_variants_map = baml_compiler_tir::enum_variants(&self.db, self.project);

        // Step 5: Run type inference
        let inference_result = baml_compiler_tir::infer_function(
            &self.db,
            &signature,
            &body,
            Some(globals_map.functions(&self.db).clone()),
            Some(class_fields_map.classes(&self.db).clone()),
            Some(type_aliases_map.aliases(&self.db).clone()),
            Some(enum_variants_map.enums(&self.db).clone()),
            func_loc,
        );

        // Step 6: Create resolution context for rendering
        let resolution_ctx = baml_compiler_tir::TypeResolutionContext::new(&self.db, self.project);

        // Step 7: Render the tree
        let tree = baml_compiler_tir::render_function_tree(
            &self.db,
            &resolution_ctx,
            function_name,
            &signature,
            &body,
            &inference_result,
        );

        // Step 8: Format type errors
        let type_errors: Vec<String> = inference_result
            .errors
            .iter()
            .map(baml_compiler_tir::pretty::short_display)
            .collect();

        // Step 9: Format signature
        let signature_str = format_signature(&signature);

        FunctionTypedBodyResult {
            success: true,
            error: None,
            tree: Some(tree),
            type_errors,
            signature: Some(signature_str),
            body_kind: Some(body_kind.to_string()),
        }
    }
}

impl BamlProgram {
    /// Find a FunctionLoc by name, using the shared function_lookup module.
    fn find_function_by_name(&self, name: &str) -> Option<baml_compiler_hir::FunctionLoc<'_>> {
        function_lookup::find_function_by_name(&self.db, self.project, name)
    }
}

// ============================================================================
// Runtime Execution Bindings
// ============================================================================

/// Result from rendering a prompt.
#[wasm_bindgen]
pub struct RenderPromptResult {
    success: bool,
    error: Option<String>,
    prompt: Option<String>,
    messages_json: Option<String>,
}

#[wasm_bindgen]
impl RenderPromptResult {
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn prompt(&self) -> Option<String> {
        self.prompt.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn messages_json(&self) -> Option<String> {
        self.messages_json.clone()
    }
}

/// Result from rendering a curl command.
#[wasm_bindgen]
pub struct RenderCurlResult {
    success: bool,
    error: Option<String>,
    curl: Option<String>,
}

#[wasm_bindgen]
impl RenderCurlResult {
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn curl(&self) -> Option<String> {
        self.curl.clone()
    }
}

/// Result from building a request.
#[wasm_bindgen]
pub struct BuildRequestResult {
    success: bool,
    error: Option<String>,
    url: Option<String>,
    method: Option<String>,
    headers_json: Option<String>,
    body_json: Option<String>,
}

#[wasm_bindgen]
impl BuildRequestResult {
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn url(&self) -> Option<String> {
        self.url.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn method(&self) -> Option<String> {
        self.method.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn headers_json(&self) -> Option<String> {
        self.headers_json.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn body_json(&self) -> Option<String> {
        self.body_json.clone()
    }
}

#[wasm_bindgen]
impl BamlProgram {
    /// Render a prompt for a function without executing.
    ///
    /// Takes a function name and JSON-encoded arguments.
    #[wasm_bindgen]
    pub fn render_prompt_for_function(
        &self,
        function_name: &str,
        args_json: &str,
    ) -> RenderPromptResult {
        // Parse arguments
        let args: baml_program::BamlMap =
            match serde_json::from_str(args_json) {
                Ok(args) => args,
                Err(e) => {
                    return RenderPromptResult {
                        success: false,
                        error: Some(format!("Failed to parse arguments: {}", e)),
                        prompt: None,
                        messages_json: None,
                    };
                }
            };

        // Create a stub prepared function
        // TODO: Get actual prompt template from function definition
        let prepared = baml_program::PreparedFunction::new_stub(
            function_name,
            args,
            baml_program::TypeRef::string(),
            baml_program::ClientSpec::new("openai/gpt-4"),
            baml_program::PromptTemplate::new("{{ input }} and this is more"),
        );

        // TODO: render_prompt is now a method on BamlProgram, not a free function
        // This needs to be updated when the runtime API is finalized
        let _ = prepared;
        RenderPromptResult {
            success: false,
            error: Some("render_prompt not yet implemented in playground".to_string()),
            prompt: None,
            messages_json: None,
        }
    }

    /// Generate a curl command for a function.
    ///
    /// Takes a function name, JSON-encoded arguments, and whether to expose secrets.
    #[wasm_bindgen]
    pub fn render_curl_for_function(
        &self,
        function_name: &str,
        args_json: &str,
        expose_secrets: bool,
    ) -> RenderCurlResult {
        // Parse arguments
        let args: baml_program::BamlMap =
            match serde_json::from_str(args_json) {
                Ok(args) => args,
                Err(e) => {
                    return RenderCurlResult {
                        success: false,
                        error: Some(format!("Failed to parse arguments: {}", e)),
                        curl: None,
                    };
                }
            };

        // Create a stub prepared function
        let prepared = baml_program::PreparedFunction::new_stub(
            function_name,
            args,
            baml_program::TypeRef::string(),
            baml_program::ClientSpec::new("openai/gpt-4"),
            baml_program::PromptTemplate::new("{{ input }}"),
        );

        // Create context
        let ctx = baml_program::context::PerCallContext::new();

        // TODO: render_raw_curl is now a method on BamlProgram, not a free function
        // This needs to be updated when the runtime API is finalized
        let _ = (prepared, ctx, expose_secrets);
        RenderCurlResult {
            success: false,
            error: Some("render_raw_curl not yet implemented in playground".to_string()),
            curl: None,
        }
    }

    /// Build a provider-specific request for a function.
    ///
    /// Returns the request details as structured data.
    #[wasm_bindgen]
    pub fn build_request_for_function(
        &self,
        function_name: &str,
        args_json: &str,
        stream: bool,
    ) -> BuildRequestResult {
        // Parse arguments
        let args: baml_program::BamlMap =
            match serde_json::from_str(args_json) {
                Ok(args) => args,
                Err(e) => {
                    return BuildRequestResult {
                        success: false,
                        error: Some(format!("Failed to parse arguments: {}", e)),
                        url: None,
                        method: None,
                        headers_json: None,
                        body_json: None,
                    };
                }
            };

        // Create a stub prepared function
        let prepared = baml_program::PreparedFunction::new_stub(
            function_name,
            args,
            baml_program::TypeRef::string(),
            baml_program::ClientSpec::new("openai/gpt-4"),
            baml_program::PromptTemplate::new("{{ input }}"),
        );

        // TODO: build_request is now a method on BamlProgram, not a free function
        // This needs to be updated when the runtime API is finalized
        let _ = (prepared, stream);
        BuildRequestResult {
            success: false,
            error: Some("build_request not yet implemented in playground".to_string()),
            url: None,
            method: None,
            headers_json: None,
            body_json: None,
        }
    }
}

/// Format a function signature as a string.
fn format_signature(sig: &baml_compiler_hir::FunctionSignature) -> String {
    let params: Vec<String> = sig
        .params
        .iter()
        .map(|p| format!("{}: {:?}", p.name, p.type_ref))
        .collect();
    format!(
        "{}({}) -> {:?}",
        sig.name,
        params.join(", "),
        sig.return_type
    )
}

/// Result of getting a function's typed body.
#[wasm_bindgen]
pub struct FunctionTypedBodyResult {
    success: bool,
    error: Option<String>,
    tree: Option<String>,
    type_errors: Vec<String>,
    signature: Option<String>,
    body_kind: Option<String>,
}

#[wasm_bindgen]
impl FunctionTypedBodyResult {
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn tree(&self) -> Option<String> {
        self.tree.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn type_errors(&self) -> Vec<String> {
        self.type_errors.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn signature(&self) -> Option<String> {
        self.signature.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn body_kind(&self) -> Option<String> {
        self.body_kind.clone()
    }
}
