//! Core type inference snapshot tests.

use super::support::{make_db, render_tir};

#[test]
fn literal_int() {
    let mut db = make_db();
    let file = db.add_file("test.baml", "function f() -> int { return 1; }");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    function user.f() -> int {
      { : never
        return 1 : 1
      }
    }
    ");
}

#[test]
fn let_binding_widens() {
    let mut db = make_db();
    let file = db.add_file("test.baml", "function f() -> int { let x = 1; return x; }");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    function user.f() -> int {
      { : never
        let x = 1 : 1 -> int
        return x : int
      }
    }
    ");
}

#[test]
fn class_field_access() {
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "class Foo { name string }\nfunction f(x: Foo) -> string { return x.name; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    class user.Foo {
      name: string
    }
    function user.f(x: user.Foo) -> string {
      { : never
        return x.name : string
      }
    }
    ");
}

#[test]
fn type_mismatch() {
    let mut db = make_db();
    let file = db.add_file("test.baml", "function f() -> string { return 1; }");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    function user.f() -> string {
      { : never
        return 1 : 1
      }
      !! 32..33: type mismatch: expected string, got 1
    }
    ");
}

#[test]
fn unresolved_field() {
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "class Foo { name string }\nfunction f(x: Foo) -> string { return x.missing; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    class user.Foo {
      name: string
    }
    function user.f(x: user.Foo) -> string {
      { : never
        return x.missing : unknown
      }
      !! 63..73: unresolved member: user.Foo.missing
    }
    ");
}

#[test]
fn binary_op_int_add() {
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "function f(a: int, b: int) -> int { return a + b; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    function user.f(a: int, b: int) -> int {
      { : never
        return a Add b : int
      }
    }
    ");
}

#[test]
fn if_else_joins_types() {
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "function f(x: bool) -> int { return if (x) { 1 } else { 2 }; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    function user.f(x: bool) -> int {
      { : never
        return : 1 | 2
          if (x : bool) : 1 | 2
            { : 1
              1 : 1
            }
          else
            { : 2
              2 : 2
            }
      }
    }
    ");
}

#[test]
fn enum_variant_resolution() {
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "enum Color { Red\nGreen\nBlue }\nfunction f() -> Color { return Color.Red; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    enum user.Color
    function user.f() -> user.Color {
      { : never
        return Color.Red : user.Color.Red
      }
    }
    ");
}

#[test]
fn resolve_class_fields_query() {
    let mut db = make_db();
    let file = db.add_file("test.baml", "class Point { x int\ny float\nlabel string }");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    class user.Point {
      x: int
      y: float
      label: string
    }
    ");
}

#[test]
fn resolve_type_alias_query() {
    let mut db = make_db();
    let file = db.add_file("test.baml", "type MyStr = string");
    insta::assert_snapshot!(render_tir(&db, file), @"type user.MyStr = string");
}

// ── Type alias struct literal regression tests ─────────────────────────────

#[test]
fn class_struct_literal() {
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "class Foo { x int }\nfunction f() -> Foo { return Foo { x: 1 }; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    class user.Foo {
      x: int
    }
    function user.f() -> user.Foo {
      { : never
        return Foo { x: 1 } : user.Foo
      }
    }
    ");
}

#[test]
fn type_alias_struct_literal() {
    // Regression test: type alias used in struct literal should resolve
    // the alias and type-check fields against the underlying class.
    //
    // BUG: Currently `Bar { x: 1 }` is typed as `user.Bar` by creating a
    // fake Ty::Class with the alias name. This "works" superficially but
    // field access will fail (see type_alias_struct_literal_field_access).
    // After fix: the struct literal should resolve through the alias to
    // produce Ty::Class(user.Foo).
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "class Foo { x int }\ntype Bar = Foo\nfunction f() -> Bar { return Bar { x: 1 }; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    class user.Foo {
      x: int
    }
    type user.Bar = user.Foo
    function user.f() -> user.Bar {
      { : never
        return Bar { x: 1 } : user.Foo
      }
    }
    ");
}

#[test]
fn type_alias_struct_literal_field_access() {
    // Field access through a type-alias-constructed struct literal.
    //
    // BUG: `v.x` fails with "unresolved member: user.Bar.x" because the
    // struct literal was typed as a fake class "Bar" which has no fields.
    // After fix: `v.x` should resolve to `int`.
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "class Foo { x int }\ntype Bar = Foo\nfunction f() -> int { let v = Bar { x: 1 }; return v.x; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    class user.Foo {
      x: int
    }
    type user.Bar = user.Foo
    function user.f() -> int {
      { : never
        let v = Bar { x: 1 } : user.Foo
        return v.x : int
      }
    }
    ");
}

#[test]
fn type_alias_check_expr_path() {
    // When the expected type is a TypeAlias (from return type annotation),
    // check_expr should resolve through the alias for field checking.
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "class Foo { x int }\ntype Bar = Foo\nfunction f() -> Bar { return Foo { x: 1 }; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    class user.Foo {
      x: int
    }
    type user.Bar = user.Foo
    function user.f() -> user.Bar {
      { : never
        return Foo { x: 1 } : user.Foo
      }
    }
    ");
}

#[test]
fn type_alias_member_access_on_param() {
    // Field access on a parameter typed with an alias.
    //
    // BUG: `v.x` fails with "unresolved member: user.Bar.x" because
    // resolve_member has no TypeAlias arm.
    // After fix: `v.x` should resolve to `int`.
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "class Foo { x int }\ntype Bar = Foo\nfunction f(v: Bar) -> int { return v.x; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    class user.Foo {
      x: int
    }
    type user.Bar = user.Foo
    function user.f(v: user.Bar) -> int {
      { : never
        return v.x : int
      }
    }
    ");
}

#[test]
fn two_functions_independent() {
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "function ok() -> int { return 1; }\nfunction bad() -> string { return 42; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    function user.ok() -> int {
      { : never
        return 1 : 1
      }
    }
    function user.bad() -> string {
      { : never
        return 42 : 42
      }
      !! 69..71: type mismatch: expected string, got 42
    }
    ");
}
