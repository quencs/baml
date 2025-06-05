class Example {
    a int
    b string

    @@stream.with_state
}

class Example2 {
    item Example
    element string
    element2 string
}

function Foo(x: int) -> Example2 | Example {
    client "openai/gpt-4o"
    prompt #"
        Fill out this data model with some examples.

        {{ ctx.output_format }}

        use {{ x }} somewhere in the data model
    "#
}

function Bar(x: int) -> Example | Example2 {
    client "ollama/phi4:latest"
    prompt #"
        Fill out this data model with some examples.

        {{ ctx.output_format }}

        use {{ x }} somewhere in the data model
    "#
}

test FooTest {
    functions [Foo]
    args {
        x 1
    }
}
