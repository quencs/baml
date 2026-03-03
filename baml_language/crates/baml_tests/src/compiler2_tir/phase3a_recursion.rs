//! Phase 3B-18/19 cycle validation tests.
//!
//! Tests that invalid (unguarded) type alias cycles produce diagnostics,
//! while valid recursive types (guarded by containers) are accepted.

use super::support::{make_db, render_tir};

// ── 3B-18/19. Cycle validation ───────────────────────────────────────────

#[test]
fn type_alias_direct_self_reference() {
    // type A = A — direct self-reference with no base case.
    let mut db = make_db();
    let file = db.add_file("test.baml", "type A = A");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    type user.A = user.A
      !! 0..10: recursive type alias cycle: A
    ");
}

#[test]
fn type_alias_mutual_recursion() {
    // type A = B, type B = A — mutual cycle with no base case.
    let mut db = make_db();
    let file = db.add_file("test.baml", "type A = B\ntype B = A");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    type user.A = user.B
      !! 0..10: recursive type alias cycle: A
    type user.B = user.A
      !! 10..21: recursive type alias cycle: B
    ");
}

#[test]
fn type_alias_indirect_cycle_three() {
    // type A = B, type B = C, type C = A — three-way cycle with no base case.
    let mut db = make_db();
    let file = db.add_file("test.baml", "type A = B\ntype B = C\ntype C = A");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    type user.A = user.B
      !! 0..10: recursive type alias cycle: A
    type user.B = user.C
      !! 10..21: recursive type alias cycle: B
    type user.C = user.A
      !! 21..32: recursive type alias cycle: C
    ");
}

#[test]
fn type_alias_valid_recursive_via_container() {
    // type JSON = string | int | JSON[] — valid recursive type (guarded by container).
    // No cycle diagnostic should be emitted.
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "type JSON = string | int | bool | null | JSON[] | map<string, JSON>",
    );
    insta::assert_snapshot!(render_tir(&db, file), @"type user.JSON = string | int | bool | null | user.JSON[] | map<string, user.JSON>");
}

#[test]
fn type_alias_cycle_used_in_function() {
    // Cycle in alias used as function param/return — cycle diagnostic on the alias.
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "type Loop = Loop\nfunction f(x: Loop) -> Loop { return x; }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    type user.Loop = user.Loop
      !! 0..16: recursive type alias cycle: Loop
    function user.f(x: user.Loop) -> user.Loop {
      { : never
        return x : user.Loop
      }
    }
    ");
}

#[test]
fn class_field_self_reference() {
    // class Node { next Node } — direct structural self-reference.
    // This is valid (class types are nominal, not expanded inline).
    let mut db = make_db();
    let file = db.add_file("test.baml", "class Node { next Node }");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    class user.Node {
      next: user.Node
    }
    ");
}

#[test]
fn class_field_mutual_reference() {
    // class A { b B }, class B { a A } — mutual class reference.
    // This is valid (class types are nominal).
    let mut db = make_db();
    let file = db.add_file(
        "test.baml",
        "class Husband { wife Wife }\nclass Wife { husband Husband }",
    );
    insta::assert_snapshot!(render_tir(&db, file), @r"
    class user.Husband {
      wife: user.Wife
    }
    class user.Wife {
      husband: user.Husband
    }
    ");
}

// ── Edge cases: Optional / Union guardedness ──────────────────────────────
//
// Key question: does Optional or Union provide a structural base case?
// V1 says NO — only List and Map are structural.
// These tests document the expected behavior.

#[test]
fn type_alias_optional_self_reference() {
    // type A = A? — is Optional a structural guard?
    // Optional does NOT provide structural termination — this is invalid.
    let mut db = make_db();
    let file = db.add_file("test.baml", "type A = A?");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    type user.A = user.A?
      !! 0..11: recursive type alias cycle: A
    ");
}

#[test]
fn type_alias_union_with_base_case() {
    // type A = A | string — Union with a non-recursive base case.
    // Union does NOT provide structural termination — this is invalid.
    let mut db = make_db();
    let file = db.add_file("test.baml", "type A = A | string");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    type user.A = user.A | string
      !! 0..19: recursive type alias cycle: A
    ");
}

#[test]
fn type_alias_list_in_union() {
    // type A = A[] | string — List-guarded recursive in union.
    // List provides structural guard — this is valid.
    let mut db = make_db();
    let file = db.add_file("test.baml", "type A = A[] | string");
    insta::assert_snapshot!(render_tir(&db, file), @"type user.A = user.A[] | string");
}

#[test]
fn type_alias_optional_list_self_reference() {
    // type A = A[]? — List then Optional.
    // List provides structural guard — this is valid.
    let mut db = make_db();
    let file = db.add_file("test.baml", "type A = A[]?");
    insta::assert_snapshot!(render_tir(&db, file), @"type user.A = user.A[]?");
}

#[test]
fn type_alias_map_in_union() {
    // type A = map<string, A> | string — Map-guarded recursive in union.
    // Map provides structural guard — this is valid.
    let mut db = make_db();
    let file = db.add_file("test.baml", "type A = map<string, A> | string");
    insta::assert_snapshot!(render_tir(&db, file), @"type user.A = map<string, user.A> | string");
}

#[test]
fn type_alias_mutual_cycle_through_optional() {
    // type A = B?, type B = A — mutual cycle through Optional.
    // Optional does NOT provide structural termination — both are invalid.
    let mut db = make_db();
    let file = db.add_file("test.baml", "type A = B?\ntype B = A");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    type user.A = user.B?
      !! 0..11: recursive type alias cycle: A
    type user.B = user.A
      !! 11..22: recursive type alias cycle: B
    ");
}

#[test]
fn type_alias_mutual_cycle_through_list() {
    // type A = B[], type B = A — mutual cycle through List.
    // List provides structural guard — both are valid.
    let mut db = make_db();
    let file = db.add_file("test.baml", "type A = B[]\ntype B = A");
    insta::assert_snapshot!(render_tir(&db, file), @r"
    type user.A = user.B[]
    type user.B = user.A
    ");
}
