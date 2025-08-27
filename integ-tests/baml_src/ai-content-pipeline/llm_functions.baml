// LLM functions for AI Content Pipeline

function GenerateVideoMetadata(transcript: string, meeting_info: ZoomMeeting) -> VideoMetadata {
  client GPT4o
  prompt #"
    Based on the following meeting information and transcript, generate metadata for a YouTube video.
    
    Meeting Info:
    - Topic: {{ meeting_info.topic }}
    - Duration: {{ meeting_info.duration }} minutes
    - Host ID: {{ meeting_info.host_id }}
    
    Transcript:
    {{ transcript }}
    
    Please generate appropriate metadata including:
    - A compelling title (max 100 characters)
    - A detailed description (max 5000 characters) 
    - Relevant tags (5-10 tags)
    - An appropriate category
    
    {{ ctx.output_format }}
  "#
}

function ExtractKeyTopics(transcript: string) -> string[] {
  client GPT4o
  prompt #"
    Analyze the following transcript and extract the main topics and key points discussed.
    Return an array of topic strings.
    
    Transcript:
    {{ transcript }}
    
    {{ ctx.output_format }}
  "#
}

function GenerateVideoTitle(topics: string[], meeting_topic: string?) -> string {
  client GPT4o
  prompt #"
    Create a compelling YouTube video title based on these key topics and meeting information.
    
    Meeting Topic: {{ meeting_topic }}
    Key Topics Discussed:
    {% for topic in topics %}
    - {{ topic }}
    {% endfor %}
    
    Requirements:
    - Maximum 100 characters
    - Clear and engaging
    - Professional tone
    - Include main subject matter
    
    Return just the title string.
    
    {{ ctx.output_format }}
  "#
}

function GenerateVideoDescription(transcript: string, topics: string[], title: string) -> string {
  client GPT4o
  prompt #"
    Generate a detailed YouTube video description based on the transcript and key topics.
    
    Video Title: {{ title }}
    
    Key Topics:
    {% for topic in topics %}
    - {{ topic }}
    {% endfor %}
    
    Transcript:
    {{ transcript }}
    
    Requirements:
    - Maximum 5000 characters
    - Include timestamp markers for key sections
    - Add relevant hashtags
    - Professional tone
    - Include call-to-action
    
    {{ ctx.output_format }}
  "#
}

function GenerateVideoTags(topics: string[], meeting_topic: string?) -> string[] {
  client GPT4o
  prompt #"
    Generate relevant YouTube tags based on the key topics and meeting information.
    
    Meeting Topic: {{ meeting_topic }}
    Key Topics:
    {% for topic in topics %}
    - {{ topic }}
    {% endfor %}
    
    Requirements:
    - 5-10 tags
    - Mix of broad and specific tags
    - Relevant to content
    - Help with discoverability
    
    {{ ctx.output_format }}
  "#
}

function ClassifyMeetingType(transcript: string, meeting_topic: string?) -> string {
  client GPT35
  prompt #"
    Classify the type of meeting based on the transcript and topic.
    
    Meeting Topic: {{ meeting_topic }}
    Transcript: {{ transcript }}
    
    Choose from these categories:
    - "presentation"
    - "discussion"  
    - "training"
    - "interview"
    - "brainstorming"
    - "review"
    - "other"
    
    Return just the category name.
    
    {{ ctx.output_format }}
  "#
}

function DetermineVideoCategory(meeting_type: string, topics: string[]) -> string {
  client GPT35
  prompt #"
    Determine the best YouTube category for this video.
    
    Meeting Type: {{ meeting_type }}
    Key Topics:
    {% for topic in topics %}
    - {{ topic }}
    {% endfor %}
    
    Choose from these YouTube categories:
    - "Education"
    - "Science & Technology"
    - "People & Blogs"
    - "Entertainment"
    - "News & Politics"
    - "Howto & Style"
    - "Travel & Events"
    - "Gaming"
    - "Sports"
    - "Music"
    
    Return just the category name.
    
    {{ ctx.output_format }}
  "#
}

function SummarizeTranscript(transcript: string) -> string {
  client GPT4o
  prompt #"
    Create a concise summary of this transcript, highlighting the main points and conclusions.
    
    Transcript:
    {{ transcript }}
    
    Requirements:
    - 2-3 paragraphs maximum
    - Include key decisions or outcomes
    - Maintain professional tone
    - Focus on actionable insights
    
    {{ ctx.output_format }}
  "#
}