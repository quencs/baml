//! Common types used throughout the executor.

use std::time::Duration;

// Re-export core types from baml_program
pub use baml_program::{
    BamlMap, BamlMedia, BamlValue, BamlValueWithMeta, MediaContent, MediaKind, Ty,
};

/// Result of a function execution.
#[derive(Debug, Clone)]
pub struct FunctionResult {
    /// The parsed output value.
    pub value: BamlValue,
    /// All orchestration attempts made.
    pub attempts: Vec<OrchestrationAttemptSummary>,
    /// Total execution duration.
    pub duration: Duration,
}

/// Summary of an orchestration attempt (for result reporting).
#[derive(Debug, Clone)]
pub struct OrchestrationAttemptSummary {
    /// Which client was used.
    pub client_name: String,
    /// Whether this attempt succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Duration of this attempt.
    pub duration: Duration,
}

/// Partial result during streaming.
#[derive(Debug, Clone)]
pub struct PartialResult {
    /// The partially parsed value (may be incomplete).
    pub value: BamlValue,
    /// Raw accumulated content from the stream.
    pub raw_content: String,
}

/// Result of running a test.
#[derive(Debug, Clone)]
pub struct TestResult {
    /// The function execution result.
    pub function_result: FunctionResult,
    /// Results of evaluating @assert/@check constraints.
    pub constraint_results: Vec<ConstraintResult>,
}

/// Result of evaluating a single constraint.
#[derive(Debug, Clone)]
pub struct ConstraintResult {
    /// Name of the constraint.
    pub name: String,
    /// Whether the constraint passed.
    pub passed: bool,
    /// Optional message explaining the result.
    pub message: Option<String>,
}
