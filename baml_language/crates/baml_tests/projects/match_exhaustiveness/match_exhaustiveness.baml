// Test: Match expression exhaustiveness and unreachable arm detection
class Success {
    data string
}

class Failure {
    reason string
}

// ============================================================================
// UNREACHABLE ARM TESTS
// These test that arms after a catch-all are marked as unreachable
// ============================================================================

// Unreachable arm: catch-all before typed pattern
function UnreachableAfterCatchAll(x: int) -> string {
    match (x) {
        _ => "catch all",
        n: int => "never reached"
    }
}

// Unreachable arm: multiple arms after catch-all
function UnreachableMultiple(x: int) -> string {
    match (x) {
        _ => "catch all",
        n: int => "unreachable 1",
        m: int => "unreachable 2"
    }
}

// ============================================================================
// EXHAUSTIVE WITH CATCH-ALL
// Catch-all patterns should make the match exhaustive
// ============================================================================

// Exhaustive: underscore catch-all
function ExhaustiveWithUnderscore(x: int) -> int {
    match (x) {
        1 => 100,
        2 => 200,
        _ => 0
    }
}

// Exhaustive: named catch-all (acts same as _)
function ExhaustiveWithNamedCatchAll(x: int) -> int {
    match (x) {
        1 => 100,
        other => other + 1
    }
}

// ============================================================================
// GUARDS DON'T CONTRIBUTE TO EXHAUSTIVENESS
// ============================================================================

// Non-exhaustive: only guarded patterns (guards don't guarantee coverage)
function GuardedOnlyNonExhaustive(x: int) -> int {
    match (x) {
        n: int if n > 0 => n * 2,
        n: int if n < 0 => n * -1
    }
}

// Exhaustive: guarded patterns followed by unguarded catch-all
function GuardedWithFallback(x: int) -> int {
    match (x) {
        n: int if n > 0 => n * 2,
        _ => 0
    }
}
