use crate::tracingv2::publisher::TypeLookup;
use baml_rpc::ast::{ast_node_id::AstNodeId, types::type_definition::TypeId};
use std::{collections::HashMap, sync::Arc};

use super::InternalBamlRuntime;

#[derive(Default)]
pub struct AstSignatureWrapper {
    functions: HashMap<String, Arc<baml_rpc::ast::tops::BamlFunctionId>>,
    classes: HashMap<String, baml_rpc::ast::types::type_definition::TypeId>,
    type_aliases: HashMap<String, baml_rpc::ast::types::type_definition::TypeId>,
    enums: HashMap<String, baml_rpc::ast::types::type_definition::TypeId>,
}

impl TypeLookup for AstSignatureWrapper {
    fn type_lookup(&self, name: &str) -> TypeId {
        TypeId(AstNodeId::new_class(name.to_string(), 0, None))
    }

    fn function_lookup(&self, name: &str) -> Option<Arc<baml_rpc::ast::tops::BamlFunctionId>> {
        self.functions.get(name).cloned()
    }
}

impl From<Arc<InternalBamlRuntime>> for AstSignatureWrapper {
    fn from(ir: Arc<InternalBamlRuntime>) -> Self {
        Self {
            ..Default::default()
        }
    }
}
