use baml_rpc::ast::{ast_node_id::AstNodeId, types::type_definition::TypeId};

use crate::tracingv2::publisher::TypeLookup;

use super::InternalBamlRuntime;

impl TypeLookup for InternalBamlRuntime {
    fn type_lookup(&self, name: &str) -> TypeId {
        TypeId(AstNodeId::new_class(name.to_string(), 0, None))
    }

    fn function_lookup(&self, name: &str) -> Option<baml_rpc::ast::tops::BamlFunctionId> {
        None
    }
}
