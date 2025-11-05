//# This file intentionally contains a syntax error to verify error handling.
function Invalid() -> string {
    //# Missing closing parenthesis on purpose
    let x = if (true {
        "ok"
    } else {
        "bad"
    };

    x
}
