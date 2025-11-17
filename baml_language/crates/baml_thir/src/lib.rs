//! Typed High-level Intermediate Representation.
//!
//! Provides type checking and inference for BAML.

use std::collections::HashMap;

use baml_base::Name;
use baml_hir::{ClassId, EnumId, FunctionId};

mod types;
pub use types::*;

/// Type inference result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceResult<'db> {
    pub return_type: Ty<'db>,
    pub param_types: HashMap<Name, Ty<'db>>,
    pub errors: Vec<TypeError<'db>>,
}

/// Type errors that can occur during type checking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeError<'db> {
    TypeMismatch {
        expected: Ty<'db>,
        found: Ty<'db>,
        span: baml_base::Span,
    },
    UnknownType {
        name: String,
        span: baml_base::Span,
    },
    // Add more variants as needed
}

impl baml_base::Diagnostic for TypeError<'_> {
    fn message(&self) -> String {
        match self {
            TypeError::TypeMismatch {
                expected, found, ..
            } => {
                format!("Type mismatch: expected {expected:?}, found {found:?}")
            }
            TypeError::UnknownType { name, .. } => {
                format!("Unknown type: {name}")
            }
        }
    }

    fn span(&self) -> Option<baml_base::Span> {
        match self {
            TypeError::TypeMismatch { span, .. } | TypeError::UnknownType { span, .. } => {
                Some(*span)
            }
        }
    }

    fn severity(&self) -> baml_base::Severity {
        baml_base::Severity::Error
    }
}

/// Helper function for type inference (non-tracked for now)
/// In a real implementation, this would use tracked functions with proper salsa types
pub fn infer_function<'db>(
    db: &'db dyn salsa::Database,
    func: FunctionId<'db>,
) -> InferenceResult<'db> {
    // TODO: Implement type inference by looking up function in ItemTree
    // We need to get the file from the FunctionLoc, get the ItemTree,
    // and look up the function data by its LocalItemId
    let _file = func.file(db);
    let _local_id = func.id(db);

    InferenceResult {
        return_type: Ty::Unknown,
        param_types: HashMap::new(),
        errors: vec![],
    }
}

/// Helper function for class type resolution (non-tracked for now)
pub fn class_type<'db>(db: &'db dyn salsa::Database, class: ClassId<'db>) -> Ty<'db> {
    // TODO: Resolve class type by looking up class in ItemTree
    // We need to get the file from the ClassLoc, get the ItemTree,
    // and look up the class data by its LocalItemId
    let _file = class.file(db);
    let _local_id = class.id(db);
    Ty::Unknown
}

/// Helper function for enum type resolution (non-tracked for now)
pub fn enum_type<'db>(_db: &'db dyn salsa::Database, _enum_id: EnumId<'db>) -> Ty<'db> {
    // TODO: Resolve enum type
    Ty::Unknown
}
