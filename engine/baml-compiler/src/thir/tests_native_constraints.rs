use baml_types::{BamlValue, BamlValueWithMeta};
use internal_baml_diagnostics::Span;

use super::*;
use crate::thir::{interpret::interpret_thir, Expr, ExprMetadata, THir};

/// Test suite for native BAML constraint expression evaluation in THIR interpreter

fn meta() -> ExprMetadata {
    (Span::fake(), None)
}

fn empty_thir() -> THir<ExprMetadata> {
    THir {
        expr_functions: vec![],
        llm_functions: vec![],
        global_assignments: baml_types::BamlMap::new(),
        classes: baml_types::BamlMap::new(),
        enums: baml_types::BamlMap::new(),
    }
}

async fn mock_llm_function(
    _fn_name: String,
    _args: Vec<BamlValue>,
) -> anyhow::Result<BamlValueWithMeta<ExprMetadata>> {
    Ok(BamlValueWithMeta::Int(42, meta()))
}

/// Test boolean constraint expressions to verify boolean equality works
#[tokio::test]
async fn test_constraint_boolean_expressions() {
    let thir = empty_thir();
    let mut context = baml_types::BamlMap::new();
    context.insert("this".to_string(), BamlValueWithMeta::Bool(true, meta()));

    // Test: this == true
    let expr = Expr::BinaryOperation {
        left: Arc::new(Expr::Var("this".to_string(), meta())),
        operator: crate::hir::BinaryOperator::Eq,
        right: Arc::new(Expr::Value(BamlValueWithMeta::Bool(true, meta()))),
        meta: meta(),
    };

    let result = interpret_thir(thir, expr, mock_llm_function, context)
        .await
        .unwrap();

    match result {
        BamlValueWithMeta::Bool(true, _) => (),
        other => panic!("Expected true, got {:?}", other),
    }
}

/// Test complex equality operations with maps, lists, enums and classes
#[tokio::test]
async fn test_constraint_complex_equality_expressions() {
    let thir = empty_thir();
    let mut context = baml_types::BamlMap::new();

    // Test list equality: [1, 2, 3] == [1, 2, 3]
    context.insert(
        "this".to_string(),
        BamlValueWithMeta::List(
            vec![
                BamlValueWithMeta::Int(1, meta()),
                BamlValueWithMeta::Int(2, meta()),
                BamlValueWithMeta::Int(3, meta()),
            ],
            meta(),
        ),
    );

    let expected_list = vec![
        BamlValueWithMeta::Int(1, meta()),
        BamlValueWithMeta::Int(2, meta()),
        BamlValueWithMeta::Int(3, meta()),
    ];

    let expr = Expr::BinaryOperation {
        left: Arc::new(Expr::Var("this".to_string(), meta())),
        operator: crate::hir::BinaryOperator::Eq,
        right: Arc::new(Expr::Value(BamlValueWithMeta::List(expected_list, meta()))),
        meta: meta(),
    };

    let result = interpret_thir(thir.clone(), expr, mock_llm_function, context.clone())
        .await
        .unwrap();

    match result {
        BamlValueWithMeta::Bool(true, _) => (),
        other => panic!("Expected true for list equality, got {:?}", other),
    }

    // Test map equality: {"a": 1, "b": 2} == {"a": 1, "b": 2}
    let mut map1 = baml_types::BamlMap::new();
    map1.insert("a".to_string(), BamlValueWithMeta::Int(1, meta()));
    map1.insert("b".to_string(), BamlValueWithMeta::Int(2, meta()));

    let mut map2 = baml_types::BamlMap::new();
    map2.insert("a".to_string(), BamlValueWithMeta::Int(1, meta()));
    map2.insert("b".to_string(), BamlValueWithMeta::Int(2, meta()));

    context.clear();
    context.insert("this".to_string(), BamlValueWithMeta::Map(map1, meta()));

    let expr2 = Expr::BinaryOperation {
        left: Arc::new(Expr::Var("this".to_string(), meta())),
        operator: crate::hir::BinaryOperator::Eq,
        right: Arc::new(Expr::Value(BamlValueWithMeta::Map(map2, meta()))),
        meta: meta(),
    };

    let result2 = interpret_thir(thir.clone(), expr2, mock_llm_function, context.clone())
        .await
        .unwrap();

    match result2 {
        BamlValueWithMeta::Bool(true, _) => (),
        other => panic!("Expected true for map equality, got {:?}", other),
    }

    // Test enum equality: RED == RED
    context.clear();
    context.insert(
        "this".to_string(),
        BamlValueWithMeta::Enum("Color".to_string(), "RED".to_string(), meta()),
    );

    let expr3 = Expr::BinaryOperation {
        left: Arc::new(Expr::Var("this".to_string(), meta())),
        operator: crate::hir::BinaryOperator::Eq,
        right: Arc::new(Expr::Value(BamlValueWithMeta::Enum(
            "Color".to_string(),
            "RED".to_string(),
            meta(),
        ))),
        meta: meta(),
    };

    let result3 = interpret_thir(thir.clone(), expr3, mock_llm_function, context.clone())
        .await
        .unwrap();

    match result3 {
        BamlValueWithMeta::Bool(true, _) => (),
        other => panic!("Expected true for enum equality, got {:?}", other),
    }

    // Test class equality: Person{name: "John", age: 25} == Person{name: "John", age: 25}
    let mut class_fields1 = baml_types::BamlMap::new();
    class_fields1.insert(
        "name".to_string(),
        BamlValueWithMeta::String("John".to_string(), meta()),
    );
    class_fields1.insert("age".to_string(), BamlValueWithMeta::Int(25, meta()));

    let mut class_fields2 = baml_types::BamlMap::new();
    class_fields2.insert(
        "name".to_string(),
        BamlValueWithMeta::String("John".to_string(), meta()),
    );
    class_fields2.insert("age".to_string(), BamlValueWithMeta::Int(25, meta()));

    context.clear();
    context.insert(
        "this".to_string(),
        BamlValueWithMeta::Class("Person".to_string(), class_fields1, meta()),
    );

    let expr4 = Expr::BinaryOperation {
        left: Arc::new(Expr::Var("this".to_string(), meta())),
        operator: crate::hir::BinaryOperator::Eq,
        right: Arc::new(Expr::Value(BamlValueWithMeta::Class(
            "Person".to_string(),
            class_fields2,
            meta(),
        ))),
        meta: meta(),
    };

    let result4 = interpret_thir(thir, expr4, mock_llm_function, context)
        .await
        .unwrap();

    match result4 {
        BamlValueWithMeta::Bool(true, _) => (),
        other => panic!("Expected true for class equality, got {:?}", other),
    }
}
