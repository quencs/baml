client<llm> Claude {
  provider anthropic
  options {
    model claude-3-haiku-20240307
    api_key env.ANTHROPIC_API_KEY
    max_tokens 1000
  }
}

function FnInputNamedImage(img: image) -> string{
  client Claude
  prompt #"
    {{ _.role("user") }}

    Describe this in 4 words. One word must be the color {{img}}
  "#
}

test TestInputNamedImage {
  functions [FnInputNamedImage]
  args {
    img {
      media_type "image/jpeg"
      url "https://www.pbs.org/wnet/nature/files/2023/11/Odontodactylus_scyllarus_Re%CC%81union-e1699977649790.jpg"
    }
  }
}
