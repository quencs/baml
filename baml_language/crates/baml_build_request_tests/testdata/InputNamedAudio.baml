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

function FnInputNamedAudio(aud: audio) -> string{
  client Gemini
  prompt #"
    {{ _.role("user") }}

   Does this sound like a roar? Yes or no? One word no other characters.

    {{ aud }}
  "#
}
//

client GPT4Audio {
  provider openai
  options {
    model "gpt-4o-audio-preview"
  }
}

function AudioInputOpenai(aud: audio, prompt: string) -> string{
  client GPT4Audio
  prompt #"
    {{ _.role("user") }}

   {{ prompt }}

    {{ aud }}
  "#
}
//



test TestInputNamedAudio {
  functions [FnInputNamedAudio]
  args {
    aud{
      url https://actions.google.com/sounds/v1/emergency/beeper_emergency_call.ogg
    }
  }
}

test TestURLAudioInputOpenai{
  functions [AudioInputOpenai]
  args {
    aud{
      url https://actions.google.com/sounds/v1/emergency/beeper_emergency_call.ogg
    }
    prompt "Does this sound like a roar? Yes or no? One word no other characters."
  }
}


