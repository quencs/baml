class SimpleClass {
    digits int
    words string  @stream.with_state
}

function MakeSimpleClass() -> SimpleClass {
    client "openai/gpt-4o"
    prompt #"
        {{ ctx.output_format }}
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
