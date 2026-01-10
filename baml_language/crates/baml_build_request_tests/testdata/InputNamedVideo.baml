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

client<llm> Vertex {
  provider vertex-ai
  options {
    model gemini-2.5-flash
    location us-central1
    credentials env.INTEG_TESTS_GOOGLE_APPLICATION_CREDENTIALS_CONTENT
  }
}

function FnInputNamedVideo(vid: video) -> string{
  client Gemini
  prompt #"
    {{ _.role("user") }}

    Use 5 words to describe the video.

    {{ vid }}
  "#
}

function FnInputNamedVideoVertex(vid: video) -> string{
  client Vertex
  prompt #"
    {{ _.role("user") }}

    Use 5 words to describe the video.

    {{ vid }}
  "#
}

test TestInputNamedVideo{
  functions [FnInputNamedVideo]
  args {
    vid{
      file "../../../../../sample-5s.mp4"
    }
  }
}

test TestInputNamedVideo2{
  functions [FnInputNamedVideoVertex]
  args {
    vid{
      file "../../../../../sample-5s.mp4"
    }
  }
}


test TestInputNamedVideoWithMediaType{
  functions [FnInputNamedVideo]
  args {
    vid {
      file "../../../../../sample-5s.mp4"
      media_type "video/mp4"
    }
  }
}

test TestInputNamedVideoWithUrl{
  functions [FnInputNamedVideo]
  args {
    vid {
      url "https://youtu.be/dQw4w9WgXcQ?si=aQdfsK0DdcDtCCud"
    }
  }
}

test TestInputNamedVideoWithUrlVertex{
  functions [FnInputNamedVideoVertex]
  args {
    vid {
      url "https://youtu.be/dQw4w9WgXcQ?si=aQdfsK0DdcDtCCud"
      media_type "video/mp4"
    }
  }
}
