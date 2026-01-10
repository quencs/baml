client<llm> GPT4o {
  provider openai
  options {
    model gpt-4o
    api_key env.OPENAI_API_KEY
  }
}

client<llm> Claude {
  provider anthropic
  options {
    model claude-3-haiku-20240307
    api_key env.ANTHROPIC_API_KEY
    max_tokens 1000
  }
}

client<llm> Vertex {
  provider vertex-ai
  options {
    model gemini-2.5-flash
    location us-central1
    credentials env.INTEG_TESTS_GOOGLE_APPLICATION_CREDENTIALS_CONTENT
  }
}

function FnInputNamedPdf(pdf: pdf) -> string{
  client GPT4o
  prompt #"
    {{ _.role("user") }}

   Describe the contents of this PDF document and what it is about in 15 words or less.

    {{ pdf }}
  "#
}

test TestInputNamedPdf {
  functions [FnInputNamedPdf]
  args {
    pdf{
      file "../../../../../dummy.pdf"
    }
  }
}
