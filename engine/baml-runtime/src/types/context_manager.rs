use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use baml_ids::SpanId;
use baml_types::BamlValue;
use std::fmt;

use crate::{
    client_registry::ClientRegistry, tracing::BamlTracer, type_builder::TypeBuilder,
    RuntimeContext, SpanCtx,
};

use super::runtime_context::{AwsCredProvider, BamlSrcReader};
pub type BamlContext = (uuid::Uuid, String, HashMap<String, BamlValue>, SpanId);

#[derive(Clone)]
pub struct RuntimeContextManager {
    baml_src_reader: Arc<BamlSrcReader>,
    aws_cred_provider: AwsCredProvider,
    context: Arc<Mutex<Vec<BamlContext>>>,
    env_vars: HashMap<String, String>,
    global_tags: Arc<Mutex<HashMap<String, BamlValue>>>,
}

impl fmt::Debug for RuntimeContextManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeContextManager")
            .field("context", &self.context.lock())
            .field("global_tags", &self.global_tags)
            .finish()
    }
}

impl RuntimeContextManager {
    pub fn deep_clone(&self) -> Self {
        Self {
            baml_src_reader: self.baml_src_reader.clone(),
            aws_cred_provider: self.aws_cred_provider.clone(),
            context: Arc::new(Mutex::new(self.context.lock().unwrap().clone())),
            env_vars: self.env_vars.clone(),
            global_tags: Arc::new(Mutex::new(self.global_tags.lock().unwrap().clone())),
        }
    }

    // pub fn span_id(&self) -> Result<uuid::Uuid> {
    //     self.context
    //         .lock()
    //         .unwrap()
    //         .last()
    //         .map(|(id, ..)| *id)
    //         .ok_or_else(|| anyhow::anyhow!("No span id found. This indicates a bug in BAML. Please report this with a stack trace (RUST_BACKTRACE=1)"))
    // }

    pub fn span_id_chain(&self, allow_empty: bool) -> Result<Vec<SpanId>> {
        let res: Vec<SpanId> = self
            .context
            .lock()
            .unwrap()
            .iter()
            .map(|(.., span_id)| span_id.clone())
            .collect();
        if res.is_empty() && !allow_empty {
            Err(anyhow::anyhow!("No span_id found. This indicates a bug in BAML. Please report this with a stack trace (RUST_BACKTRACE=1)"))
        } else {
            Ok(res)
        }
    }

    pub fn new_from_env_vars(
        env_vars: HashMap<String, String>,
        baml_src_reader: BamlSrcReader,
        aws_cred_provider: AwsCredProvider,
    ) -> Self {
        Self {
            baml_src_reader: Arc::new(baml_src_reader),
            aws_cred_provider: aws_cred_provider,
            context: Default::default(),
            env_vars,
            global_tags: Default::default(),
        }
    }

    pub fn upsert_tags(&self, tags: HashMap<String, BamlValue>) {
        let mut ctx = self.context.lock().unwrap();
        if let Some((.., last_tags, _)) = ctx.last_mut() {
            last_tags.extend(tags);
        } else {
            self.global_tags.lock().unwrap().extend(tags);
        }
    }

    fn clone_last_tags(&self) -> HashMap<String, BamlValue> {
        self.context
            .lock()
            .unwrap()
            .last()
            .map(|(_, _, tags, _)| tags.clone())
            .unwrap_or_default()
    }

    // Note, after entering, calling ctx.span_id() will return the span id of the old context still.
    pub fn enter(&self, name: &str) -> (uuid::Uuid, Vec<SpanId>) {
        let last_tags = self.clone_last_tags();
        let span = uuid::Uuid::new_v4();
        let span_id = SpanId::new();
        let mut ctx = self.context.lock().unwrap();
        ctx.push((span, name.to_string(), last_tags, span_id));

        let span_chain = ctx.iter().map(|(.., span_id)| span_id.clone()).collect();
        log::trace!("Entering with: {:#?}", ctx);
        (span, span_chain)
    }

    pub fn exit(&self) -> Option<(uuid::Uuid, Vec<SpanCtx>, HashMap<String, BamlValue>)> {
        let mut ctx = self.context.lock().unwrap();
        log::trace!("Exiting: {:#?}", ctx);

        let prev = ctx
            .iter()
            .map(|(span, name, _, span_id)| SpanCtx {
                span_id: *span,
                name: name.clone(),
                new_span_id: span_id.clone(),
            })
            .collect();

        let (id, _, mut tags, new_id) = ctx.pop()?;

        for (k, v) in self.global_tags.lock().unwrap().iter() {
            tags.entry(k.clone()).or_insert_with(|| v.clone());
        }

        Some((id, prev, tags))
    }

    pub fn create_ctx(
        &self,
        type_builder: Option<&TypeBuilder>,
        client_registry: Option<&ClientRegistry>,
        // the tracer initializes the new span_id,
        // and then passes it back in here for a new context. It's kind of circular since tracer uses this class to _create_ the span_id....
        // Anyway RuntimeCtx is passed everywhere and we need to know what the last span_id that the tracer created was.
        // tl;dr
        // 1. Tracer creates a new span id using the current context that's passe dinto call_function()
        // 2. Tracer passes the span_id back in here for a new context
        // 3. profit
        span_id_chain: Vec<SpanId>,
    ) -> Result<RuntimeContext> {
        // let mut tags = self.global_tags.lock().unwrap().clone();
        // let ctx_tags = {
        //     self.context
        //         .lock()
        //         .unwrap()
        //         .last()
        //         .map(|(.., x, _)| x)
        //         .cloned()
        //         .unwrap_or_default()
        // };
        // tags.extend(ctx_tags);
        let tags = {
            let mut tags = self.global_tags.lock().unwrap().clone();
            let ctx = self.context.lock().unwrap();
            let ctx = ctx.last();
            if let Some((.., ctx_tags, _)) = ctx {
                tags.extend(ctx_tags.into_iter().map(|(k, v)| (k.clone(), v.clone())));
            }
            tags
        };

        let (cls, enm, als, rec_cls, rec_als) = type_builder
            .map(TypeBuilder::to_overrides)
            .unwrap_or_default();

        let mut ctx = RuntimeContext::new(
            self.baml_src_reader.clone(),
            self.aws_cred_provider.clone(),
            self.env_vars.clone(),
            tags,
            Default::default(),
            cls,
            enm,
            als,
            rec_cls,
            rec_als,
            span_id_chain,
        );

        ctx.client_overrides = match client_registry {
            Some(cr) => Some(
                cr.to_clients(&ctx)
                    .with_context(|| "Failed to create clients from client_registry")?,
            ),
            None => None,
        };

        Ok(ctx)
    }

    // for tests only
    pub fn create_ctx_with_default(&self) -> RuntimeContext {
        let ctx = self.context.lock().unwrap();

        RuntimeContext::new(
            self.baml_src_reader.clone(),
            self.aws_cred_provider.clone(),
            self.env_vars.clone(),
            ctx.last().map(|(.., x, _)| x).cloned().unwrap_or_default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            vec![SpanId::new()],
        )
    }

    pub fn context_depth(&self) -> usize {
        let ctx = self.context.lock().unwrap();
        ctx.len()
    }
}
