//! Tests for type alias usage in struct literals, field access, and related contexts.

use baml_tests::baml_test;
use bex_engine::BexExternalValue;
use indexmap::indexmap;

// ============================================================================
// Struct literal construction through aliases
// ============================================================================

#[tokio::test]
async fn struct_literal_through_alias() {
    let output = baml_test!(
        "
        class Foo { x int }
        type Bar = Foo;

        function main() -> Bar {
            let v = Bar { x: 1 };
            v
        }
    "
    );

    insta::assert_snapshot!(output.bytecode);

    assert_eq!(
        output.result,
        Ok(BexExternalValue::instance(
            "Foo",
            indexmap! {
                "x" => BexExternalValue::Int(1),
            }
        ))
    );
}

#[tokio::test]
async fn chained_alias_struct_literal() {
    let output = baml_test!(
        "
        class Foo { x int }
        type A = B;
        type B = Foo;

        function main() -> Foo {
            let v = A { x: 42 };
            v
        }
    "
    );

    insta::assert_snapshot!(output.bytecode);

    assert_eq!(
        output.result,
        Ok(BexExternalValue::instance(
            "Foo",
            indexmap! {
                "x" => BexExternalValue::Int(42),
            }
        ))
    );
}

#[tokio::test]
async fn field_access_through_alias() {
    let output = baml_test!(
        "
        class Foo { x int }
        type Bar = Foo;

        function main() -> int {
            let v = Bar { x: 7 };
            v.x
        }
    "
    );

    insta::assert_snapshot!(output.bytecode);

    assert_eq!(output.result, Ok(BexExternalValue::Int(7)));
}

#[tokio::test]
async fn alias_in_return_type_annotation() {
    let output = baml_test!(
        "
        class Foo { x int }
        type Bar = Foo;

        function main() -> Bar {
            Foo { x: 5 }
        }
    "
    );

    insta::assert_snapshot!(output.bytecode);

    assert_eq!(
        output.result,
        Ok(BexExternalValue::instance(
            "Foo",
            indexmap! {
                "x" => BexExternalValue::Int(5),
            }
        ))
    );
}

// ============================================================================
// Field access through aliases (old TIR regression)
// ============================================================================

/// Regression: old TIR `infer_field_access` used the alias name to look up
/// class fields, which failed because fields are stored under the real class
/// name. Fixed by resolving through the alias chain before field lookup.
#[tokio::test]
async fn chained_alias_field_access() {
    let output = baml_test!(
        "
        class Point { x int  y int }
        type A = B;
        type B = Point;

        function main() -> int {
            let p = A { x: 10, y: 20 };
            p.x + p.y
        }
    "
    );

    insta::assert_snapshot!(output.bytecode);
    assert_eq!(output.result, Ok(BexExternalValue::Int(30)));
}

/// Regression: field access on a local variable whose type is inferred from
/// an alias-constructed struct literal.  The old TIR needed to resolve the
/// alias to find the underlying class for field lookup.
#[tokio::test]
async fn alias_multi_field_access() {
    let output = baml_test!(
        "
        class Pair { a int  b int }
        type MyPair = Pair;

        function main() -> int {
            let p = MyPair { a: 3, b: 4 };
            p.a + p.b
        }
    "
    );

    insta::assert_snapshot!(output.bytecode);
    assert_eq!(output.result, Ok(BexExternalValue::Int(7)));
}

/// Regression: alias-typed parameter field access.  The emit phase was not
/// passing type_aliases to infer_function, so alias-typed parameters were
/// not recognized and their fields could not be accessed.
#[tokio::test]
async fn alias_param_field_access() {
    let output = baml_test!(
        "
        class Pair { a int  b int }
        type MyPair = Pair;

        function main() -> int {
            sum(MyPair { a: 3, b: 4 })
        }

        function sum(p: MyPair) -> int {
            p.a + p.b
        }
    "
    );

    insta::assert_snapshot!(output.bytecode);
    assert_eq!(output.result, Ok(BexExternalValue::Int(7)));
}

// ============================================================================
// MIR lowering regression — alias name resolved to class name for emit
// ============================================================================

/// Regression: MIR lowering passed the alias name ("Bar") to
/// `AggregateKind::Class`, causing `emit.rs` to panic with
/// "undefined class: Bar".  Fixed by resolving through `type_aliases`
/// before creating the aggregate.
#[tokio::test]
async fn mir_alias_aggregate_resolution() {
    let output = baml_test!(
        "
        class Coord { x int  y int  z int }
        type Vec3 = Coord;

        function main() -> int {
            let c = Vec3 { x: 1, y: 2, z: 3 };
            c.x + c.y + c.z
        }
    "
    );

    insta::assert_snapshot!(output.bytecode);
    assert_eq!(output.result, Ok(BexExternalValue::Int(6)));
}

/// Regression: chained aliases through MIR — both alias hops must resolve
/// to the real class name for the emitter.
#[tokio::test]
async fn mir_chained_alias_aggregate_resolution() {
    let output = baml_test!(
        "
        class RGB { r int  g int  b int }
        type Color = RGB;
        type Shade = Color;

        function main() -> int {
            let c = Shade { r: 10, g: 20, b: 30 };
            c.r + c.g + c.b
        }
    "
    );

    insta::assert_snapshot!(output.bytecode);
    assert_eq!(output.result, Ok(BexExternalValue::Int(60)));
}
