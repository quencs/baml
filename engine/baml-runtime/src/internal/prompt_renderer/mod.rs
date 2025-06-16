mod render_output_format;
pub(crate) mod scoped_ir;
use internal_llm_client::ClientSpec;
use jsonish::{BamlValueWithFlags, ResponseBamlValue};
use render_output_format::render_output_format;

use anyhow::Result;
use baml_types::{BamlValue, FieldType, StreamingBehavior, TypeValue};
use internal_baml_core::{
    error_unsupported,
    ir::{
        repr::IntermediateRepr, FunctionWalker, IRHelper, IRHelperExtended,
        IRSemanticStreamingHelper,
    },
};
use internal_baml_jinja::{
    types::OutputFormatContent, RenderContext, RenderContext_Client, RenderedPrompt,
    TemplateStringMacro,
};
use scoped_ir::ScopedIr;

use crate::{runtime_context::RuntimeClassOverride, RuntimeContext};

use super::llm_client::parsed_value_to_response;

#[derive(Debug)]
pub struct PromptRenderer {
    pub function_name: String,
    pub client_spec: ClientSpec,
    pub output_defs: OutputFormatContent,
    pub output_type: FieldType,
    pub xml_format_used: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PromptRenderer {
    pub fn from_function(
        function: &FunctionWalker,
        ir: &IntermediateRepr,
        ctx: &RuntimeContext,
    ) -> Result<PromptRenderer> {
        let func_v2 = function.elem();
        let Some(config) = func_v2.configs.first() else {
            error_unsupported!("function", function.name(), "no valid prompt found")
        };

        Ok(PromptRenderer {
            function_name: function.name().into(),
            client_spec: match &ctx.client_overrides {
                Some((Some(client), _)) => ClientSpec::Named(client.clone()),
                _ => config.client.clone(),
            },
            output_defs: render_output_format(ir, ctx, &func_v2.output)?,
            output_type: func_v2.output.clone(),
            xml_format_used: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// A temporary function used to generate a fake prompt renderer, for cases
    /// when we call BamlRuntime's `call` API with Expression fns, which
    /// don't have a prompt.
    pub fn mk_fake() -> PromptRenderer {
        PromptRenderer {
            function_name: "fake".into(),
            client_spec: ClientSpec::Named("fake".into()),
            output_defs: OutputFormatContent::mk_fake(),
            output_type: FieldType::Primitive(TypeValue::String),
            xml_format_used: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn client_spec(&self) -> &ClientSpec {
        &self.client_spec
    }

    pub fn parse(
        &self,
        ir: &IntermediateRepr,
        ctx: &RuntimeContext,
        raw_string: &str,
        allow_partials: bool,
    ) -> Result<ResponseBamlValue> {
        let parsed = jsonish::from_str(
            &self.output_defs,
            &self.output_type,
            raw_string,
            allow_partials,
        )?;
        let scoped_ir = ScopedIr::new(ir, ctx);
        let res = parsed_value_to_response(&scoped_ir, parsed, allow_partials);
        res
    }

    /// Parse using XML parser instead of JSON parser
    pub fn parse_xml(
        &self,
        ir: &IntermediateRepr,
        ctx: &RuntimeContext,
        raw_string: &str,
        allow_partials: bool,
    ) -> Result<ResponseBamlValue> {
        let parsed = jsonish::from_str_xml(
            &self.output_defs,
            &self.output_type,
            raw_string,
            allow_partials,
        )?;
        let scoped_ir = ScopedIr::new(ir, ctx);
        let res = parsed_value_to_response(&scoped_ir, parsed, allow_partials);
        res
    }

    /// Smart parse that automatically chooses between XML and JSON parsing based on the format used during rendering
    pub fn smart_parse(
        &self,
        ir: &IntermediateRepr,
        ctx: &RuntimeContext,
        raw_string: &str,
        allow_partials: bool,
    ) -> Result<ResponseBamlValue> {
        let xml_used = self
            .xml_format_used
            .load(std::sync::atomic::Ordering::Relaxed);

        if xml_used {
            self.parse_xml(ir, ctx, raw_string, allow_partials)
        } else {
            self.parse(ir, ctx, raw_string, allow_partials)
        }
    }

    pub fn render_prompt(
        &self,
        ir: &IntermediateRepr,
        ctx: &RuntimeContext,
        params: &BamlValue,
        client_ctx: &RenderContext_Client,
    ) -> Result<RenderedPrompt> {
        let func = ir.find_function(&self.function_name)?;

        let func_v2 = func.elem();

        let Some(config) = func_v2.configs.first() else {
            error_unsupported!("function", self.function_name, "no valid prompt found")
        };

        let render_ctx = RenderContext {
            client: client_ctx.clone(),
            tags: ctx.tags.clone(),
            output_format: self.output_defs.clone(),
            xml_format_used: self.xml_format_used.clone(),
        };

        let (rendered_prompt, xml_used) = internal_baml_jinja::render_prompt(
            &config.prompt_template,
            params,
            render_ctx,
            &ir.walk_template_strings()
                .map(|t| TemplateStringMacro {
                    name: t.name().into(),
                    args: t
                        .inputs()
                        .iter()
                        .map(|i| (i.name.clone(), i.r#type.elem.to_string()))
                        .collect(),
                    template: t.template().into(),
                })
                .collect::<Vec<_>>(),
            ir,
            ctx.env_vars(),
        )?;

        // Store whether XML format was used for later use in parsing
        self.xml_format_used
            .store(xml_used, std::sync::atomic::Ordering::Relaxed);

        Ok(rendered_prompt)
    }
}
