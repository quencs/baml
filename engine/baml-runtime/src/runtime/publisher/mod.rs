use super::InternalBamlRuntime;
use crate::{internal::ir_features::WithInternal, tracingv2::publisher::TypeLookup};
use baml_rpc::ast::{ast_node_id::AstNodeId, tops::BamlFunctionId, types::type_definition::TypeId};
use cowstr::CowStr;
use internal_baml_core::ir::ir_hasher;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

#[derive(Default)]
pub struct AstSignatureWrapper {
    // path to source code
    source_code: HashMap<PathBuf, CowStr>,
    functions: HashMap<String, Arc<baml_rpc::ast::tops::BamlFunctionId>>,
    types: HashMap<String, Arc<TypeId>>,
}

impl TypeLookup for AstSignatureWrapper {
    fn type_lookup(&self, name: &str) -> Option<Arc<TypeId>> {
        if let Some(id) = self.types.get(name) {
            Some(id.clone())
        } else {
            // This happens for dynamic types, like LLM responses.
            None
        }
    }

    fn function_lookup(&self, name: &str) -> Option<Arc<baml_rpc::ast::tops::BamlFunctionId>> {
        self.functions.get(name).cloned()
    }
}

impl TryFrom<Arc<InternalBamlRuntime>> for AstSignatureWrapper {
    type Error = anyhow::Error;

    fn try_from(ir: Arc<InternalBamlRuntime>) -> Result<Self, Self::Error> {
        let ir_signature = ir_hasher::IRSignature::new_from_ir(&ir.ir)?;

        Ok(Self {
            functions: ir_signature
                .functions
                .into_iter()
                .map(|(name, signature)| {
                    (
                        name.clone(),
                        AstNodeId::new_function(
                            name,
                            signature.interface_hash(),
                            signature.implementation_hash(),
                        ),
                    )
                })
                .map(|(name, id)| (name, Arc::new(BamlFunctionId(id))))
                .collect(),
            types: {
                let mut types = HashMap::new();
                for (name, signature) in ir_signature.classes.into_iter() {
                    types.insert(
                        name.clone(),
                        AstNodeId::new_class(
                            name,
                            signature.interface_hash(),
                            signature.implementation_hash(),
                        ),
                    );
                }
                for (name, signature) in ir_signature.enums.into_iter() {
                    types.insert(
                        name.clone(),
                        AstNodeId::new_enum(
                            name,
                            signature.interface_hash(),
                            signature.implementation_hash(),
                        ),
                    );
                }
                for (name, signature) in ir_signature.type_aliases.into_iter() {
                    types.insert(
                        name.clone(),
                        AstNodeId::new_type_alias(
                            name,
                            signature.interface_hash(),
                            signature.implementation_hash(),
                        ),
                    );
                }
                types
                    .into_iter()
                    .map(|(name, id)| (name, Arc::new(TypeId(id))))
                    .collect()
            },
            source_code: ir
                .source_files
                .iter()
                .map(|file| (file.path_buf().clone(), CowStr::from(file.as_str())))
                .collect(),
        })
    }
}
