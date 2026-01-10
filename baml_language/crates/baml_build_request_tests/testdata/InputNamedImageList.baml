client<llm> GPT4o {
  provider openai
  options {
    model gpt-4o
    api_key env.OPENAI_API_KEY
  }
}

function FnInputNamedImageList(imgs: image[]) -> string{
  client GPT4o
  prompt #"
    {{ _.role("user") }}

    What colors do these have in common? {{imgs}}
  "#
}

test TestInputNamedImageList {
  functions [FnInputNamedImageList]
  args {
    imgs [
      {
        media_type "image/png"
        url "https://www.google.com/images/branding/googlelogo/2x/googlelogo_color_92x30dp.png"
      },
      {
        url "https://i.imgur.com/93fWs5R.png"
      }
    ]
  }
}
