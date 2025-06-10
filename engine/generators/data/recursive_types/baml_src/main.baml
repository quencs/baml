type JSON = string | null | int | float | map<string, JSON> | JSON[]

function Foo(x: int) -> JSON {
    client "ollama/phi4:latest"
    prompt #"
        print out a json object with the following fields:
        - a: {{ x }}
        - b: "hello"
        - c: [1, 2, 3]
        - d: { "e": ["f"] }
        - f: null
        - g: 1.0
        - h: { a: 1, b: null, c: [1, 2, 3] }
    "#
}
