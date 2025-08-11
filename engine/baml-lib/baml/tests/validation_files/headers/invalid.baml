// This file contains invalid MDX header usage patterns

// Headers outside function body (should fail)
# Top Level Header
// This should not be allowed

function InvalidHeaderPlacement() -> string {
    let x = "test"
    ## Header After Expression // This line should fail - header not on its own line
    x
}

function HeaderInPrompt() -> string {
    client GPT4
    prompt #"
        # This is inside a prompt
        It should be treated as literal text, not a header
    "#
}

// Headers in wrong context
class MyClass {
    # Class Header // Should fail - headers only allowed in expression functions
    name string
}

function MixedOnSameLine() -> string {
    let x = "hello" ## Inline Header // Should fail
    x
}

function HeaderWithoutSpace() -> string {
    #No Space After Hash // Should potentially fail depending on grammar rules
    let x = "test"
    x
}

function EmptyHeader() -> string {
    # 
    // Empty header title - this should fail
    let x = "test"
    ##
    // Another empty header
    x
}

// Test headers in places they shouldn't parse
function HeaderInExpression() -> string {
    let x = "start" + # Middle Header // Should fail
        "end"
    x
}

function HeaderInStringLiteral() -> string {
    let x = "This string contains # Header Text"
    let y = #"
        Raw string with # Header
        Should not be parsed
    "#
    x + y
}

// LLM function (not expression function) - headers should not be allowed
function LLMFunction(input: string) -> string {
    client GPT4
    # Section Header
    // Should fail - only allowed in expression functions
    prompt #"Process: {{ input }}"#
}

client<llm> GPT4 {
    provider openai
    options {
        model gpt-4
        api_key env.OPENAI_API_KEY
    }
}

// error: Error validating: This line is invalid. It does not start with any known Baml schema keyword.
//   -->  headers/mdx_headers_invalid.baml:4
//    | 
//  3 | // Headers outside function body (should fail)
//  4 | # Top Level Header
//  5 | // This should not be allowed
//    | 


