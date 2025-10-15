
//# Main Function
function ComplexHeaderTest(x: int) -> int {
    x + 1234
}

test Foo {
    functions [ComplexHeaderTest]
    args {
        x 5
    }
}