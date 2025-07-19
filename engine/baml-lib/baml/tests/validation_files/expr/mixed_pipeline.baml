function LLM(x: int) -> int {
    client GPT4o
    prompt #"Return {{ x }} {{ ctx.output_format}}"#
}

function UseLLM(x: int) -> int {
    LLM(x)
}

client<llm> GPT4o {
  provider openai
  options {
    model gpt-4o
    api_key env.OPENAI_API_KEY
  }
}

// warning: Workflow functions are experimental, and will break in the future.
//   -->  expr/mixed_pipeline.baml:1
//    | 
//    | 
//  1 | function LLM(x: int) -> int {
//    | 
// warning: Workflow functions are experimental, and will break in the future.
//   -->  expr/mixed_pipeline.baml:6
//    | 
//  5 | 
//  6 | function UseLLM(x: int) -> int {
//    | 
// error: Error validating: Type mismatch in lambda: string vs int
//   -->  expr/mixed_pipeline.baml:2
//    | 
//  1 | function LLM(x: int) -> int {
//  2 |     client GPT4o
//    | 
