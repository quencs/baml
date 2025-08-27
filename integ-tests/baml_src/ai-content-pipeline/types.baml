// Data models for AI Content Pipeline

class ZoomMeeting {
  id string
  topic string?
  start_time string
  duration int?
  host_id string?
}

class ZoomRecording {
  id string
  meeting_id string
  download_url string?
  file_type string // "MP4", "M4A", "TIMELINE", "TRANSCRIPT", "CC"
  file_size int?
  recording_type string // "speaker_view", "screen_share", "gallery_view"
  status string // "processing", "completed"
}

class YouTubeVideo {
  id string?
  title string
  description string?
  privacy_status string // "private", "public", "unlisted"
  upload_status string? // "uploaded", "processed", "rejected"
  url string?
}

class VideoProcessingJob {
  id string
  zoom_meeting_id string
  zoom_recording_id string
  status string // "pending", "downloading", "processing", "uploading", "completed", "failed"
  error_message string?
  youtube_video_id string?
  created_at string
  updated_at string?
}

class ProcessingProgress {
  stage string
  progress_percent int
  message string?
  timestamp string
}

class VideoMetadata {
  title string
  description string
  tags string[]
  category string?
  thumbnail_url string?
  duration_seconds int?
}

enum RecordingPriority {
  SCREEN_SHARE_WITH_SPEAKER
  SCREEN_SHARE
  VIDEO_ONLY
  AUDIO_ONLY
}

enum ProcessingStage {
  INITIALIZING
  DOWNLOADING_RECORDING
  EXTRACTING_TRANSCRIPT
  GENERATING_METADATA
  UPLOADING_TO_YOUTUBE
  UPDATING_DATABASE
  COMPLETED
  FAILED
}