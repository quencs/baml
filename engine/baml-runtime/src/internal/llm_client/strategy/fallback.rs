use std::collections::HashMap;

use anyhow::{Context, Result};
use internal_baml_core::ir::ClientWalker;
use internal_llm_client::{
    ClientProvider, ClientSpec, ResolvedClientProperty, UnresolvedClientProperty,
};

use crate::{
    client_registry::ClientProperty,
    internal::llm_client::orchestrator::{
        ExecutionScope, IterOrchestrator, OrchestrationScope, OrchestrationState,
    },
    runtime_interface::InternalClientLookup,
    RuntimeContext,
};

#[derive(Debug)]
pub struct FallbackStrategy {
    pub name: String,
    pub(super) retry_policy: Option<String>,
    // TODO: We can add conditions to each client
    client_specs: Vec<ClientSpec>,
    pub http_config: internal_llm_client::HttpConfig,
}

fn resolve_strategy(
    provider: &ClientProvider,
    properties: &UnresolvedClientProperty<()>,
    ctx: &RuntimeContext,
) -> Result<(Vec<ClientSpec>, internal_llm_client::HttpConfig)> {
    let properties = properties.resolve(provider, &ctx.eval_ctx(false))?;
    let ResolvedClientProperty::Fallback(props) = properties else {
        anyhow::bail!(
            "Invalid client property. Should have been a fallback property but got: {}",
            properties.name()
        );
    };
    Ok((props.strategy, props.http_config))
}

impl TryFrom<(&ClientProperty, &RuntimeContext)> for FallbackStrategy {
    type Error = anyhow::Error;

    fn try_from(
        (client, ctx): (&ClientProperty, &RuntimeContext),
    ) -> std::result::Result<Self, Self::Error> {
        let (strategy, http_config) =
            resolve_strategy(&client.provider, &client.unresolved_options()?, ctx)?;
        Ok(Self {
            name: client.name.clone(),
            retry_policy: client.retry_policy.clone(),
            client_specs: strategy,
            http_config,
        })
    }
}

impl TryFrom<(&ClientWalker<'_>, &RuntimeContext)> for FallbackStrategy {
    type Error = anyhow::Error;

    fn try_from((client, ctx): (&ClientWalker, &RuntimeContext)) -> Result<Self> {
        let (strategy, http_config) =
            resolve_strategy(&client.elem().provider, client.options(), ctx)?;
        Ok(Self {
            name: client.item.elem.name.clone(),
            retry_policy: client.retry_policy().as_ref().map(String::from),
            client_specs: strategy,
            http_config,
        })
    }
}

impl IterOrchestrator for std::sync::Arc<FallbackStrategy> {
    fn iter_orchestrator<'a>(
        &self,
        state: &mut OrchestrationState,
        _previous: OrchestrationScope,
        ctx: &RuntimeContext,
        client_lookup: &'a dyn InternalClientLookup<'a>,
    ) -> Result<crate::internal::llm_client::orchestrator::OrchestratorNodeIterator> {
        let items = self
            .client_specs
            .iter()
            .enumerate()
            .map(
                |(idx, client)| match client_lookup.get_llm_provider(client, ctx) {
                    Ok(client) => {
                        let client = client.clone();
                        Ok(client.iter_orchestrator(
                            state,
                            ExecutionScope::Fallback(self.clone(), idx).into(),
                            ctx,
                            client_lookup,
                        ))
                    }
                    Err(e) => Err(e),
                },
            )
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .flatten()
            .collect();

        Ok(items)
    }
}
