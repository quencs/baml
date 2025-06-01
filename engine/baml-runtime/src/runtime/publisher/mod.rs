use super::InternalBamlRuntime;
use crate::{internal::ir_features::WithInternal, tracingv2::publisher::TypeLookup};
use baml_rpc::ast::{ast_node_id::AstNodeId, tops::BamlFunctionId};
use baml_rpc::BamlTypeId;
use baml_types::FieldType;
use cowstr::CowStr;
use internal_baml_core::ir::ir_hasher;
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

/// Type alias for a value with its dependencies
pub type WithDependency<T> = (Arc<T>, Arc<Vec<Arc<BamlTypeId>>>);

use super::super::tracingv2::publisher::rpc_converters::IntoRpcEvent;

#[derive(Serialize)]
pub struct TypeWithDependencies {
    pub type_id: WithDependency<BamlTypeId>,
    pub field_type: Arc<FieldType>,
    pub class_fields: Option<Arc<Vec<(String, Arc<FieldType>)>>>,
    pub enum_values: Option<Arc<Vec<String>>>,
}

#[derive(Serialize)]
pub struct FunctionSignatureWithDependencies {
    pub function_id: WithDependency<BamlFunctionId>,
    pub inputs: Arc<Vec<(String, FieldType)>>,
    pub output: Arc<FieldType>,
}

#[derive(Default, Serialize)]
pub struct AstSignatureWrapper {
    /// Path to source code
    pub source_code: HashMap<PathBuf, CowStr>,
    pub functions: HashMap<String, FunctionSignatureWithDependencies>,
    pub types: HashMap<String, TypeWithDependencies>,
    pub env_vars: HashMap<String, String>,
}

impl AstSignatureWrapper {
    pub fn env_var(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
    }
}

impl TypeLookup for AstSignatureWrapper {
    fn type_lookup(&self, name: &str) -> Option<Arc<BamlTypeId>> {
        self.types.get(name).map(|t| t.type_id.0.clone())
    }

    fn function_lookup(&self, name: &str) -> Option<Arc<BamlFunctionId>> {
        self.functions.get(name).map(|f| f.function_id.0.clone())
    }
}

impl TryFrom<(Arc<InternalBamlRuntime>, HashMap<String, String>)> for AstSignatureWrapper {
    type Error = anyhow::Error;

    fn try_from(
        (ir_runtime, env_vars): (Arc<InternalBamlRuntime>, HashMap<String, String>),
    ) -> Result<Self, Self::Error> {
        let ir_signature = ir_hasher::IRSignature::new_from_ir(&ir_runtime.ir)?;

        let name_to_baml_type_id_map: HashMap<String, Arc<BamlTypeId>> = ir_signature
            .classes
            .iter()
            .map(|(name, (type_node_sig, _class_details))| {
                (
                    name.clone(),
                    Arc::new(BamlTypeId(type_node_sig.signature.clone_into_ast_node_id())),
                )
            })
            .chain(
                ir_signature
                    .enums
                    .iter()
                    .map(|(name, (type_node_sig, _enum_details))| {
                        (
                            name.clone(),
                            Arc::new(BamlTypeId(type_node_sig.signature.clone_into_ast_node_id())),
                        )
                    }),
            )
            .chain(
                ir_signature
                    .type_aliases
                    .iter()
                    .map(|(name, type_node_sig)| {
                        (
                            name.clone(),
                            Arc::new(BamlTypeId(type_node_sig.signature.clone_into_ast_node_id())),
                        )
                    }),
            )
            .collect();

        let functions: HashMap<String, FunctionSignatureWithDependencies> = ir_signature
            .functions
            .into_iter()
            .map(|(name, func_sig)| {
                let dep_names_vec: Vec<String> = func_sig.signature.dependency_names().clone();
                let dependencies = dep_names_vec
                    .iter()
                    .filter_map(|dep_name| name_to_baml_type_id_map.get(dep_name).cloned())
                    .collect::<Vec<Arc<BamlTypeId>>>();
                (
                    name,
                    FunctionSignatureWithDependencies {
                        function_id: (
                            Arc::new(BamlFunctionId(func_sig.signature.clone_into_ast_node_id())),
                            Arc::new(dependencies),
                        ),
                        inputs: func_sig.inputs.clone(),
                        output: func_sig.output.clone(),
                    },
                )
            })
            .collect();

        let types: HashMap<String, TypeWithDependencies> = ir_signature
            .classes
            .into_iter()
            .map(|(name, (type_node_sig, class_details))| {
                let dep_names_vec: Vec<String> = type_node_sig.signature.dependency_names().clone();
                let dependencies = dep_names_vec
                    .iter()
                    .filter_map(|dep_name| name_to_baml_type_id_map.get(dep_name).cloned())
                    .collect::<Vec<Arc<BamlTypeId>>>();
                (
                    name.clone(),
                    TypeWithDependencies {
                        type_id: (
                            Arc::new(BamlTypeId(type_node_sig.signature.clone_into_ast_node_id())),
                            Arc::new(dependencies),
                        ),
                        field_type: type_node_sig.field_type.clone(),
                        class_fields: Some(class_details.fields.clone()),
                        enum_values: None,
                    },
                )
            })
            .chain(
                ir_signature
                    .enums
                    .into_iter()
                    .map(|(name, (type_node_sig, enum_details))| {
                        let dep_names_vec: Vec<String> =
                            type_node_sig.signature.dependency_names().clone();
                        let dependencies = dep_names_vec
                            .iter()
                            .filter_map(|dep_name| name_to_baml_type_id_map.get(dep_name).cloned())
                            .collect::<Vec<Arc<BamlTypeId>>>();
                        (
                            name.clone(),
                            TypeWithDependencies {
                                type_id: (
                                    Arc::new(BamlTypeId(
                                        type_node_sig.signature.clone_into_ast_node_id(),
                                    )),
                                    Arc::new(dependencies),
                                ),
                                field_type: type_node_sig.field_type.clone(),
                                class_fields: None,
                                enum_values: Some(enum_details.values.clone()),
                            },
                        )
                    }),
            )
            .chain(
                ir_signature
                    .type_aliases
                    .into_iter()
                    .map(|(name, type_node_sig)| {
                        let dep_names_vec: Vec<String> =
                            type_node_sig.signature.dependency_names().clone();
                        let dependencies = dep_names_vec
                            .iter()
                            .filter_map(|dep_name| name_to_baml_type_id_map.get(dep_name).cloned())
                            .collect::<Vec<Arc<BamlTypeId>>>();
                        (
                            name.clone(),
                            TypeWithDependencies {
                                type_id: (
                                    Arc::new(BamlTypeId(
                                        type_node_sig.signature.clone_into_ast_node_id(),
                                    )),
                                    Arc::new(dependencies),
                                ),
                                field_type: type_node_sig.field_type.clone(),
                                class_fields: None,
                                enum_values: None,
                            },
                        )
                    }),
            )
            .collect();

        let source_code = ir_runtime
            .source_files
            .iter()
            .map(|file| (file.path_buf().clone(), CowStr::from(file.as_str())))
            .collect();

        Ok(Self {
            env_vars,
            functions,
            types,
            source_code,
        })
    }
}

// Helper extension trait to convert ir_hasher::Signature to AstNodeId
trait SignatureExt {
    fn clone_into_ast_node_id(&self) -> AstNodeId;
}

impl SignatureExt for internal_baml_core::ir::ir_hasher::Signature {
    fn clone_into_ast_node_id(&self) -> AstNodeId {
        let interface_hash = self.interface_hash();
        let impl_hash = self.implementation_hash();
        let name = self.display_name().to_string();

        match self.r#type {
            internal_baml_core::ir::ir_hasher::SignatureType::Class => {
                AstNodeId::new_class(name, interface_hash, impl_hash)
            }
            internal_baml_core::ir::ir_hasher::SignatureType::Enum => {
                AstNodeId::new_enum(name, interface_hash, impl_hash)
            }
            internal_baml_core::ir::ir_hasher::SignatureType::TypeAlias => {
                AstNodeId::new_type_alias(name, interface_hash, impl_hash)
            }
            internal_baml_core::ir::ir_hasher::SignatureType::Function => {
                AstNodeId::new_function(name, interface_hash, impl_hash)
            }
            _ => panic!(
                "Unsupported signature type for AstNodeId conversion: {:?}",
                self.r#type
            ),
        }
    }
}
