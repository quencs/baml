class SimpleClass {
    digits int
    words string  @stream.with_state
}

function MakeSimpleClass() -> SimpleClass {
    client "ollama/phi4:latest"
    prompt #"
        {{ ctx.output_format }}
    "#
}

function ConsumeSimpleClass(item: SimpleClass) -> SimpleClass {
    client "ollama/phi4:latest"
    prompt #"
        Return back to me verbatim:

        {{ item }}
    "#
}

test MakeSimpleClassTest {
    functions [MakeSimpleClass]
    args {
        class_1:
            digits: 123
            words: "hello"
    }
}
