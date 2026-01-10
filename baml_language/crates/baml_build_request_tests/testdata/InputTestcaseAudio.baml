client<llm> Gemini {
  provider google-ai
  options {
    model gemini-2.5-flash
    api_key env.GOOGLE_API_KEY
    safetySettings {
      category HARM_CATEGORY_HATE_SPEECH
      threshold BLOCK_LOW_AND_ABOVE
    }
  }
}

function FnInputTestcaseAudio(aud: audio) -> string{
  client Gemini
  prompt #"
    {{ _.role("user") }}

   Does this sound like a roar? Yes or no? One word no other characters.

    {{ aud }}
  "#
}

test TestInputTestcaseAudio {
  functions [FnInputTestcaseAudio]
  args {
    aud {
      media_type "audio/mp3"
      base64 #"
        //uQZAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWGluZwAAAA8AAABRAACbvAACBAgKDQ0QExcZGRweIiQkJyouMTQ0ODs+QkJFSUxPT1NWWl1dYGRna25ucXV4e3t/goaJiYyPkpaWmJyfoqamqq2ws7O3ur3BwcTHy87O0tbZ3eHh5Ofq7e3v8vT29vr7//8AAAA8TEFNRTMuOThyBK8AAAAAAAAAADQgJATvTQABzAAAm7x1lCT3AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
      "#
    }
  }
}
