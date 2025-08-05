fn SimpleForLoop() -> string {
    let items = ["a", "b", "c"];
    let output = "";
    for item in items {
        if item == "b" {
            output + item + "b"
        } else {
            output + item
        }
    }
    output
}

client<llm> GPT4o {
    provider openai
    options {
        model gpt-4o
        api_key env.OPENAI_API_KEY
    }
} 