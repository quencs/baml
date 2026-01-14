//! Executor API - Entry points for executing BAML functions.

use std::collections::HashMap;

use baml_db::baml_workspace::Project;
use baml_jinja_runtime::{LlmClientSpec, RenderContext};
use baml_llm_interface::RenderedPrompt;
use baml_output_format::{OutputFormatContent, OutputFormatOptions};
use baml_program::{BamlProgram, Ty};
use baml_program_compile::convert_tir_ty;
use baml_project::ProjectDatabase as RootDatabase;

use crate::{
    context::{DynamicBamlContext, PerCallContext, SharedCallContext},
    errors::RuntimeError,
    function_lookup::{FunctionBodyInfo, find_function_by_name, get_function_info},
    llm_request::openai::{OpenAiClientConfig, OpenAiRequest},
    orchestrator::{
        ClientConfig, FunctionResultStream, OrchestrationScope, OrchestratorConfig,
        OrchestratorNode, ProviderType, orchestrate_call, orchestrate_stream,
    },
    output_format::build_output_format,
    prepared_function::PreparedFunction,
    render_options::RenderOptions,
    types::{BamlMap, BamlValue, FunctionResult, TestResult},
};

/// The BAML executor - main entry point for executing BAML functions.
///
/// BamlExecutor holds a compiled BAML database and provides methods for:
/// - Preparing functions for execution
/// - Rendering prompts
/// - Building provider requests
/// - Executing functions (with or without streaming)
pub struct BamlExecutor {
    db: RootDatabase,
    project: Project,
    /// Compiled BamlProgram for output format building.
    program: BamlProgram,
}

impl BamlExecutor {
    pub fn new(db: RootDatabase) -> Result<Self, RuntimeError> {
        let project = db
            .project()
            .ok_or_else(|| RuntimeError::Validation("Database has no project set".to_string()))?;
        let program = baml_program_compile::compile_program(&db, project);
        Ok(Self {
            db,
            project,
            program,
        })
    }

    pub fn with_project(db: RootDatabase, project: Project) -> Self {
        let program = baml_program_compile::compile_program(&db, project);
        Self {
            db,
            project,
            program,
        }
    }

    pub fn with_program(db: RootDatabase, project: Project, program: BamlProgram) -> Self {
        Self {
            db,
            project,
            program,
        }
    }

    pub fn db(&self) -> &RootDatabase {
        &self.db
    }

    pub fn program(&self) -> &BamlProgram {
        &self.program
    }

    pub fn project(&self) -> Project {
        self.project
    }

    pub fn prepare_function(
        &self,
        function_name: &str,
        args: BamlMap,
    ) -> Result<PreparedFunction, RuntimeError> {
        let func_loc = find_function_by_name(&self.db, self.project, function_name)
            .ok_or_else(|| RuntimeError::FunctionNotFound(function_name.to_string()))?;

        let func_info = get_function_info(&self.db, self.project, func_loc);

        let prompt_template = match &func_info.body {
            FunctionBodyInfo::Llm(llm) => llm.prompt.as_ref().cloned().ok_or_else(|| {
                RuntimeError::Validation(format!(
                    "Function '{}' has no prompt template",
                    function_name
                ))
            })?,
            FunctionBodyInfo::Expr => {
                return Err(RuntimeError::Validation(format!(
                    "Function '{}' is an expression function, not an LLM function",
                    function_name
                )));
            }
            FunctionBodyInfo::Missing => {
                return Err(RuntimeError::Validation(format!(
                    "Function '{}' has no body",
                    function_name
                )));
            }
        };

        let client_name = match &func_info.body {
            FunctionBodyInfo::Llm(llm) => {
                llm.client.clone().unwrap_or_else(|| "default".to_string())
            }
            _ => "default".to_string(),
        };

        let args_with_types: indexmap::IndexMap<String, crate::prepared_function::TypedArg> = args
            .iter()
            .map(|(k, v)| {
                let arg_ty = func_info
                    .signature
                    .params
                    .iter()
                    .find(|p| p.name == *k)
                    .map(|p| convert_tir_ty(&p.ty))
                    .unwrap_or(Ty::String);
                (
                    k.clone(),
                    crate::prepared_function::TypedArg {
                        value: v.clone(),
                        arg_ty,
                    },
                )
            })
            .collect();

        Ok(PreparedFunction {
            function_name: function_name.to_string(),
            args,
            args_with_types,
            return_ty: convert_tir_ty(&func_info.signature.return_type),
            client_name,
            prompt_template,
        })
    }

    pub async fn call_function(
        &self,
        prepared: &PreparedFunction,
        _shared_ctx: &SharedCallContext,
        _dynamic_ctx: &DynamicBamlContext,
        per_call_ctx: &PerCallContext,
    ) -> Result<FunctionResult, RuntimeError> {
        let config = build_orchestrator_config(&prepared.client_name)?;
        let prompt = self.render_prompt(prepared, _dynamic_ctx)?;

        let result = orchestrate_call(
            &prompt,
            &config,
            &per_call_ctx.env_vars,
            &prepared.return_ty,
            || per_call_ctx.is_cancelled(),
        )
        .await?;

        Ok(FunctionResult {
            value: result.response.map(|r| r.value).unwrap_or(BamlValue::Null),
            attempts: result
                .attempts
                .iter()
                .map(|a| crate::types::OrchestrationAttemptSummary {
                    client_name: a.node.client.name.clone(),
                    success: a.error.is_none(),
                    error: a.error.as_ref().map(|e| e.to_string()),
                    duration: a.duration,
                })
                .collect(),
            duration: result.total_duration,
        })
    }

    pub fn stream_function(
        &self,
        prepared: &PreparedFunction,
        _shared_ctx: &SharedCallContext,
        _dynamic_ctx: &DynamicBamlContext,
        per_call_ctx: &PerCallContext,
    ) -> Result<FunctionResultStream, RuntimeError> {
        let config = build_orchestrator_config(&prepared.client_name)?;
        let prompt = self.render_prompt(prepared, _dynamic_ctx)?;

        orchestrate_stream(
            &prompt,
            config,
            &per_call_ctx.env_vars,
            prepared.return_ty.clone(),
            || per_call_ctx.is_cancelled(),
        )
    }

    pub async fn run_test(
        &self,
        prepared: &PreparedFunction,
        shared_ctx: &SharedCallContext,
        dynamic_ctx: &DynamicBamlContext,
        per_call_ctx: &PerCallContext,
    ) -> Result<TestResult, RuntimeError> {
        let function_result = self
            .call_function(prepared, shared_ctx, dynamic_ctx, per_call_ctx)
            .await?;

        Ok(TestResult {
            function_result,
            constraint_results: vec![],
        })
    }

    pub fn render_prompt(
        &self,
        prepared: &PreparedFunction,
        _dynamic_ctx: &DynamicBamlContext,
    ) -> Result<RenderedPrompt, RuntimeError> {
        let ctx = self.build_render_context(prepared)?;
        let args = BamlValue::Map(prepared.args.clone());

        baml_jinja_runtime::render_prompt(&prepared.prompt_template, &args, ctx)
            .map_err(|e| RuntimeError::Render(e.to_string()))
    }

    pub fn build_request(
        &self,
        prepared: &PreparedFunction,
        dynamic_ctx: &DynamicBamlContext,
        per_call_ctx: &PerCallContext,
        stream: bool,
    ) -> Result<OpenAiRequest, RuntimeError> {
        let prompt = self.render_prompt(prepared, dynamic_ctx)?;

        let client_config = OpenAiClientConfig {
            api_key: per_call_ctx
                .env_vars
                .get("OPENAI_API_KEY")
                .cloned()
                .unwrap_or_default(),
            model: "gpt-4".to_string(),
            ..Default::default()
        };

        OpenAiRequest::from_rendered(&prompt, &client_config, stream).map_err(RuntimeError::from)
    }

    pub fn render_raw_curl(
        &self,
        prepared: &PreparedFunction,
        dynamic_ctx: &DynamicBamlContext,
        per_call_ctx: &PerCallContext,
        options: &RenderOptions,
    ) -> Result<String, RuntimeError> {
        let request = self.build_request(prepared, dynamic_ctx, per_call_ctx, false)?;
        Ok(request.to_curl(options))
    }

    fn build_render_context(
        &self,
        prepared: &PreparedFunction,
    ) -> Result<RenderContext, RuntimeError> {
        let client_name = prepared.client_name.clone();
        let (provider, default_role, allowed_roles) = parse_client_config(&client_name);

        // Build output format from the return type
        let output_format = self.build_output_format_string(&prepared.return_ty);

        Ok(RenderContext {
            client: LlmClientSpec {
                name: client_name,
                provider,
                default_role,
                allowed_roles,
                ..Default::default()
            },
            tags: HashMap::new(),
            output_format,
        })
    }

    /// Build the output format string for a given return type.
    fn build_output_format_string(&self, return_ty: &Ty) -> Option<String> {
        let output_format_content = build_output_format(&self.program, return_ty);
        let options = OutputFormatOptions::default();

        match baml_output_format::render(&output_format_content, &options) {
            Ok(Some(rendered)) => Some(rendered),
            Ok(None) => None,
            Err(_) => None,
        }
    }

    /// Get the OutputFormatContent for a given return type.
    ///
    /// This is useful for response parsing and validation.
    pub fn get_output_format_content(&self, return_ty: &Ty) -> OutputFormatContent {
        build_output_format(&self.program, return_ty)
    }
}

fn parse_client_config(client_name: &str) -> (String, String, Vec<String>) {
    let provider = if client_name.to_lowercase().contains("anthropic") {
        "anthropic".to_string()
    } else {
        // Default to OpenAI for any other client
        "openai".to_string()
    };

    let (default_role, allowed_roles) = if provider == "anthropic" {
        (
            "user".to_string(),
            vec!["user".to_string(), "assistant".to_string()],
        )
    } else {
        (
            "system".to_string(),
            vec![
                "system".to_string(),
                "user".to_string(),
                "assistant".to_string(),
            ],
        )
    };

    (provider, default_role, allowed_roles)
}

fn build_orchestrator_config(client_name: &str) -> Result<OrchestratorConfig, RuntimeError> {
    let provider = if client_name.to_lowercase().contains("anthropic") {
        ProviderType::Anthropic
    } else {
        ProviderType::OpenAi
    };

    let node = OrchestratorNode {
        client: ClientConfig {
            name: client_name.to_string(),
            provider,
            options: serde_json::json!({}),
        },
        scope: OrchestrationScope::Direct,
        delay: None,
    };

    Ok(OrchestratorConfig::single(node))
}
