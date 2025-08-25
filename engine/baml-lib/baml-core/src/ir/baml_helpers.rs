use std::{collections::HashMap, future::Future};
use anyhow::{anyhow, Result};
use baml_types::{BamlValue, BamlMap, BamlValueWithMeta, expr::{Expr, ExprMetadata}};
use internal_baml_diagnostics::Span;

/// Native expression evaluator for BAML constraint expressions
/// 
/// This function evaluates constraint expressions using the THIR interpreter and supports
/// LLM function calls through a callback mechanism.
/// 
/// # Arguments
/// * `this` - The BamlValue representing the 'this' context for constraint evaluation
/// * `context` - Additional context variables available during evaluation
/// * `predicate_expression` - The constraint expression to evaluate
/// * `llm_function_callback` - Async callback for handling LLM function calls
/// 
/// # Returns
/// * `Ok(true)` if constraint passes
/// * `Ok(false)` if constraint fails
/// * `Err(...)` if evaluation error occurs
/// 
/// # Example
/// ```rust
/// use baml_types::BamlValue;
/// use std::collections::HashMap;
/// 
/// let llm_callback = |name: String, args: Vec<BamlValue>| async move {
///     // Your LLM function implementation here
///     Ok(BamlValueWithMeta::Bool(true, (Span::fake(), None)))
/// };
/// 
/// let result = evaluate_native_predicate(
///     &BamlValue::String("test".to_string()),
///     &HashMap::new(),
///     &constraint_expr,
///     llm_callback
/// )?;
/// ```
pub fn evaluate_native_predicate<F, Fut>(
    this: &BamlValue,
    context: &HashMap<String, BamlValue>,
    predicate_expression: &Expr<ExprMetadata>,
    llm_function_callback: F,
) -> Result<bool> 
where
    F: FnMut(String, Vec<BamlValue>) -> Fut + Send + Sync,
    Fut: Future<Output = Result<BamlValueWithMeta<ExprMetadata>>> + Send,
{
    // Convert BamlValue to BamlValueWithMeta for the interpreter
    let this_with_meta = baml_value_to_baml_value_with_meta(this);
    
    // Build evaluation context from 'this' and additional context
    let mut eval_context = BamlMap::new();
    eval_context.insert("this".to_string(), this_with_meta);
    
    // Add additional context variables
    for (key, value) in context.iter() {
        eval_context.insert(key.clone(), baml_value_to_baml_value_with_meta(value));
    }
    
    // Convert Expr to THIR expression
    let thir_expr = convert_expr_to_thir(predicate_expression);
    
    // Create a simple THir environment for constraint evaluation
    // We don't need full THir with functions, just expression evaluation
    let thir = baml_compiler::thir::THir {
        expr_functions: vec![],
        llm_functions: vec![],
        global_assignments: BamlMap::new(),
        classes: BamlMap::new(),
        enums: BamlMap::new(),
    };
    
    // Use the THIR interpreter in a synchronous context
    // Since constraint evaluation must be synchronous, we use tokio runtime
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| anyhow!("Failed to create tokio runtime for constraint evaluation: {}", e))?;
    
    let result = runtime.block_on(async {
        baml_compiler::thir::interpret::interpret_thir(
            thir,
            thir_expr,
            llm_function_callback,
            eval_context,
        )
        .await
    })?;
    
    // Ensure the result is a boolean
    match result {
        BamlValueWithMeta::Bool(b, _) => Ok(b),
        other => Err(anyhow!(
            "Constraint expression must evaluate to boolean, got: {:?}",
            other
        )),
    }
}

/// Convenience function for evaluating native predicates without LLM function support
/// 
/// This is a simpler version that rejects any LLM function calls during constraint evaluation.
/// Use this when you only need basic constraint evaluation without LLM functions.
/// 
/// # Arguments
/// * `this` - The BamlValue representing the 'this' context for constraint evaluation
/// * `context` - Additional context variables available during evaluation  
/// * `predicate_expression` - The constraint expression to evaluate
/// 
/// # Returns
/// * `Ok(true)` if constraint passes
/// * `Ok(false)` if constraint fails
/// * `Err(...)` if evaluation error occurs or LLM function is called
pub fn evaluate_native_predicate_no_llm(
    this: &BamlValue,
    context: &HashMap<String, BamlValue>,
    predicate_expression: &Expr<ExprMetadata>,
) -> Result<bool> {
    let llm_function_callback = |name: String, _args: Vec<BamlValue>| async move {
        Err(anyhow!("LLM function '{}' not supported in constraint expressions", name))
    };
    
    evaluate_native_predicate(this, context, predicate_expression, llm_function_callback)
}

/// Convert BamlValue to BamlValueWithMeta with fake metadata
fn baml_value_to_baml_value_with_meta(value: &BamlValue) -> BamlValueWithMeta<ExprMetadata> {
    let fake_meta = (Span::fake(), None);
    
    match value {
        BamlValue::String(s) => BamlValueWithMeta::String(s.clone(), fake_meta),
        BamlValue::Int(i) => BamlValueWithMeta::Int(*i, fake_meta),
        BamlValue::Float(f) => BamlValueWithMeta::Float(*f, fake_meta),
        BamlValue::Bool(b) => BamlValueWithMeta::Bool(*b, fake_meta),
        BamlValue::List(items) => {
            let converted_items: Vec<_> = items
                .iter()
                .map(baml_value_to_baml_value_with_meta)
                .collect();
            BamlValueWithMeta::List(converted_items, fake_meta)
        }
        BamlValue::Map(map) => {
            let converted_map: BamlMap<_, _> = map
                .iter()
                .map(|(k, v)| (k.clone(), baml_value_to_baml_value_with_meta(v)))
                .collect();
            BamlValueWithMeta::Map(converted_map, fake_meta)
        }
        BamlValue::Class(name, fields) => {
            let converted_fields: BamlMap<_, _> = fields
                .iter()
                .map(|(k, v)| (k.clone(), baml_value_to_baml_value_with_meta(v)))
                .collect();
            BamlValueWithMeta::Class(name.clone(), converted_fields, fake_meta)
        }
        BamlValue::Enum(name, value) => BamlValueWithMeta::Enum(name.clone(), value.clone(), fake_meta),
        BamlValue::Media(media) => BamlValueWithMeta::Media(media.clone(), fake_meta),
        BamlValue::Null => BamlValueWithMeta::Null(fake_meta),
    }
}

/// Convert constraint Expr to THIR Expr
/// This is a simplified conversion for constraint evaluation
fn convert_expr_to_thir(expr: &Expr<ExprMetadata>) -> baml_compiler::thir::Expr<ExprMetadata> {
    match expr {
        Expr::Atom(value_with_meta) => baml_compiler::thir::Expr::Value(value_with_meta.clone()),
        Expr::List(items, meta) => {
            let converted_items: Vec<_> = items
                .iter()
                .map(convert_expr_to_thir)
                .collect();
            baml_compiler::thir::Expr::List(converted_items, meta.clone())
        }
        Expr::Map(entries, meta) => {
            let converted_entries: BamlMap<_, _> = entries
                .iter()
                .map(|(k, v)| (k.clone(), convert_expr_to_thir(v)))
                .collect();
            baml_compiler::thir::Expr::Map(converted_entries, meta.clone())
        }
        // Add more conversions as needed for constraint expressions
        // For now, we handle the most common cases
        _ => {
            // Fallback: create a boolean true expression for unsupported cases
            log::warn!("Unsupported expression type in constraint: {:?}", expr);
            baml_compiler::thir::Expr::Value(BamlValueWithMeta::Bool(true, (Span::fake(), None)))
        }
    }
}

/// Helper function to convert Expr back to string representation
/// This is used for constraint evaluation during the transition period
pub fn expr_to_string(expr: &Expr<ExprMetadata>) -> String {
    // Simple string representation for now
    // This will be enhanced as the evaluator is completed
    format!("{:?}", expr)
}

/// Convert BamlValue to constraint Expr
/// This is used for converting runtime values to constraint expressions
pub fn baml_value_to_expr(value: &BamlValue) -> Result<Expr<ExprMetadata>> {
    let fake_meta = (Span::fake(), None);
    
    match value {
        BamlValue::String(s) => Ok(Expr::Atom(BamlValueWithMeta::String(s.clone(), fake_meta))),
        BamlValue::Int(i) => Ok(Expr::Atom(BamlValueWithMeta::Int(*i, fake_meta))),
        BamlValue::Float(f) => Ok(Expr::Atom(BamlValueWithMeta::Float(*f, fake_meta))),
        BamlValue::Bool(b) => Ok(Expr::Atom(BamlValueWithMeta::Bool(*b, fake_meta))),
        BamlValue::List(items) => {
            let converted_items: Result<Vec<_>> = items
                .iter()
                .map(baml_value_to_expr)
                .collect();
            Ok(Expr::List(converted_items?, fake_meta))
        }
        BamlValue::Map(map) => {
            let converted_map: Result<BamlMap<_, _>> = map
                .iter()
                .map(|(k, v)| baml_value_to_expr(v).map(|expr| (k.clone(), expr)))
                .collect();
            Ok(Expr::Map(converted_map?, fake_meta))
        }
        BamlValue::Class(name, fields) => {
            let converted_fields: Result<BamlMap<_, _>> = fields
                .iter()
                .map(|(k, v)| baml_value_to_expr(v).map(|expr| (k.clone(), expr)))
                .collect();
            Ok(Expr::Map(converted_fields?, fake_meta)) // Classes are treated as maps in constraints
        }
        BamlValue::Enum(name, value) => Ok(Expr::Atom(BamlValueWithMeta::Enum(name.clone(), value.clone(), fake_meta))),
        BamlValue::Media(media) => Ok(Expr::Atom(BamlValueWithMeta::Media(media.clone(), fake_meta))),
        BamlValue::Null => Ok(Expr::Atom(BamlValueWithMeta::Null(fake_meta))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baml_types::BamlValue;

    #[test]
    fn test_baml_value_to_expr() {
        // Test basic types
        let string_val = BamlValue::String("hello".to_string());
        let string_expr = baml_value_to_expr(&string_val).unwrap();
        assert!(matches!(string_expr, Expr::Atom(_)));
        
        let int_val = BamlValue::Int(42);
        let int_expr = baml_value_to_expr(&int_val).unwrap();
        assert!(matches!(int_expr, Expr::Atom(_)));
        
        let bool_val = BamlValue::Bool(true);
        let bool_expr = baml_value_to_expr(&bool_val).unwrap();
        assert!(matches!(bool_expr, Expr::Atom(_)));
    }
    
    #[test]
    fn test_list_conversion() {
        let list_val = BamlValue::List(vec![
            BamlValue::Int(1),
            BamlValue::Int(2),
            BamlValue::Int(3),
        ]);
        let list_expr = baml_value_to_expr(&list_val).unwrap();
        assert!(matches!(list_expr, Expr::List(_, _)));
    }
    
    #[test]
    fn test_evaluate_native_predicate_no_llm() {
        use internal_baml_diagnostics::Span;
        
        // Test with a simple boolean expression
        let context = HashMap::new();
        let this_val = BamlValue::Bool(true);
        let expr = Expr::Atom(BamlValueWithMeta::Bool(true, (Span::fake(), None)));
        
        let result = evaluate_native_predicate_no_llm(&this_val, &context, &expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        
        // Test with false
        let expr = Expr::Atom(BamlValueWithMeta::Bool(false, (Span::fake(), None)));
        let result = evaluate_native_predicate_no_llm(&this_val, &context, &expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }
    
    #[test]
    fn test_evaluate_native_predicate_with_callback() {
        use internal_baml_diagnostics::Span;
        
        // Test with callback-based version
        let context = HashMap::new();
        let this_val = BamlValue::String("test".to_string());
        let expr = Expr::Atom(BamlValueWithMeta::Bool(true, (Span::fake(), None)));
        
        // Mock LLM function that returns a boolean result
        let mock_llm_callback = |function_name: String, _args: Vec<BamlValue>| async move {
            match function_name.as_str() {
                "ValidateText" => Ok(BamlValueWithMeta::Bool(true, (Span::fake(), None))),
                "CheckLength" => Ok(BamlValueWithMeta::Bool(false, (Span::fake(), None))),
                _ => Err(anyhow!("Unknown function: {}", function_name)),
            }
        };
        
        let result = evaluate_native_predicate(&this_val, &context, &expr, mock_llm_callback);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }
}