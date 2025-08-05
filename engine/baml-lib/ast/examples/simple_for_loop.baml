fn SimpleForLoop() -> string {
    # Processing stuff!
    let items = ["a", "b", "c"];
    let output = "";

    ## For loop with items
    for item in items {
        # Process items
        if item == "b" {
            output + item + "b"
        } else {
            output + item
        }
    }

    ## Result
    output
}

client<llm> GPT4o {
    provider openai
    options {
        model gpt-4o
        api_key env.OPENAI_API_KEY
    }
} 