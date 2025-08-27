// API integration functions for external services
// Using std::fetch_value for real HTTP calls to API server

function FetchMeetingInfo(meeting_id: string) -> ZoomMeeting {
  let api_url = env.API_BASE_URL + "/api/meetings/" + meeting_id;
  
  return std::fetch_value<ZoomMeeting>(std::Request {
    base_url: api_url,
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer " + env.API_TOKEN
    },
    query_params: {}
  });
}

function FetchRecordingInfo(recording_id: string) -> ZoomRecording {
  let api_url = env.API_BASE_URL + "/api/recordings/" + recording_id;
  
  return std::fetch_value<ZoomRecording>(std::Request {
    base_url: api_url,
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer " + env.API_TOKEN
    },
    query_params: {}
  });
}

function FetchRecordingsForMeeting(meeting_id: string) -> ZoomRecording[] {
  let api_url = env.API_BASE_URL + "/api/meetings/" + meeting_id + "/recordings";
  
  return std::fetch_value<ZoomRecording[]>(std::Request {
    base_url: api_url,
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer " + env.API_TOKEN
    },
    query_params: {}
  });
}

function FetchTranscriptFromZoom(meeting_id: string, recording_id: string) -> string {
  let api_url = env.API_BASE_URL + "/api/meetings/" + meeting_id + "/recordings/" + recording_id + "/transcript";
  
  let response_data = std::fetch_value<map<string, string>>(std::Request {
    base_url: api_url,
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer " + env.API_TOKEN
    },
    query_params: {}
  });
  
  return response_data["transcript"];
}

function DownloadZoomRecording(recording: ZoomRecording) -> map<string, string> {
  let api_url = env.API_BASE_URL + "/api/recordings/" + recording.id + "/download";
  
  return std::fetch_value<map<string, string>>(std::Request {
    base_url: api_url,
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer " + env.API_TOKEN
    },
    query_params: {
      "recording_id": recording.id,
      "download_url": recording.download_url
    }
  });
}

function UploadVideoToYoutube(file_path: string, metadata: VideoMetadata) -> YouTubeVideo {
  let api_url = env.API_BASE_URL + "/api/youtube/upload";
  
  return std::fetch_value<YouTubeVideo>(std::Request {
    base_url: api_url,
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer " + env.API_TOKEN
    },
    query_params: {
      "file_path": file_path,
      "title": metadata.title,
      "description": metadata.description,
      "tags": metadata.tags.join(","),
      "category": metadata.category,
      "privacy_status": "private"
    }
  });
}

function UpdateJobStatus(meeting_id: string, recording_id: string, youtube_id: string?, status: string) -> VideoProcessingJob {
  let job_id = meeting_id + "_" + recording_id;
  let api_url = env.API_BASE_URL + "/api/jobs/" + job_id;
  
  return std::fetch_value<VideoProcessingJob>(std::Request {
    base_url: api_url,
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer " + env.API_TOKEN
    },
    query_params: {
      "status": status,
      "youtube_video_id": youtube_id,
      "updated_at": GetCurrentTimestamp()
    }
  });
}

function FetchJobDetails(job_id: string) -> VideoProcessingJob {
  let api_url = env.API_BASE_URL + "/api/jobs/" + job_id;
  
  return std::fetch_value<VideoProcessingJob>(std::Request {
    base_url: api_url,
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer " + env.API_TOKEN
    },
    query_params: {}
  });
}

function CreateProcessingJob(meeting_id: string, recording_id: string) -> VideoProcessingJob {
  let api_url = env.API_BASE_URL + "/api/jobs";
  
  return std::fetch_value<VideoProcessingJob>(std::Request {
    base_url: api_url,
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer " + env.API_TOKEN
    },
    query_params: {
      "zoom_meeting_id": meeting_id,
      "zoom_recording_id": recording_id,
      "status": "pending",
      "created_at": GetCurrentTimestamp()
    }
  });
}