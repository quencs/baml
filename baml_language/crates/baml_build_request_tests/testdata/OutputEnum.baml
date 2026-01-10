retry_policy GPT4oRetry {
  max_retries 2
  strategy {
    type exponential_backoff
  }
}

client<llm> GPT35 {
  provider openai
  retry_policy GPT4oRetry
  options {
    model "gpt-4o-mini"
    api_key env.OPENAI_API_KEY
  }
}

/// An enum with three values,
/// ONE, TWO and THREE.
enum EnumOutput {

  /// The first enum.
  ONE

  /// The second enum.
  TWO @alias("two") @description("two")
  THREE @alias("hi") @description("three")

  @@alias("VALUE_ENUM")
}

function FnOutputEnum(input: string) -> EnumOutput {
  client GPT35
  prompt #"
    Choose one of these values randomly. Before you give the answer, write out an unrelated haiku about the ocean.

    {{ctx.output_format(prefix=null)}}
  "#
}

test TestOutputEnum {
  functions [FnOutputEnum]
  args {
    input "example input"
  }
}
