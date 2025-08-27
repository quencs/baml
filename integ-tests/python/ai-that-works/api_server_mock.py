"""
Mock API server for database proxy operations.

Since BAML doesn't have direct database access, this demonstrates
how the API server would handle database operations that the
BAML functions would call.
"""

from datetime import datetime
from typing import Dict, List, Optional
import json


class MockDatabaseAPI:
    """
    Mock implementation of the API server that proxies database calls.
    
    In a real implementation, this would be a FastAPI or Flask server
    that handles HTTP requests and translates them to database operations.
    """
    
    def __init__(self):
        # In-memory storage for demo purposes
        self.jobs: Dict[str, Dict] = {}
        self.meetings: Dict[str, Dict] = {}
        self.recordings: Dict[str, Dict] = {}
        
        # Populate with some test data
        self._initialize_test_data()
    
    def _initialize_test_data(self):
        """Initialize with test data for demo purposes."""
        # Test meeting
        self.meetings["test_meeting_12345"] = {
            "id": "test_meeting_12345",
            "topic": "Q1 Planning Session - Engineering Team",
            "start_time": "2024-01-15T14:00:00Z",
            "duration": 45,
            "host_id": "john.doe@company.com"
        }
        
        # Test recordings
        self.recordings["test_recording_67890"] = {
            "id": "test_recording_67890",
            "meeting_id": "test_meeting_12345", 
            "download_url": "https://zoom.us/rec/download/test_recording_67890",
            "file_type": "MP4",
            "file_size": 1048576,
            "recording_type": "screen_share",
            "status": "completed"
        }
        
        # Test job
        self.jobs["test_meeting_12345_test_recording_67890"] = {
            "id": "test_meeting_12345_test_recording_67890",
            "zoom_meeting_id": "test_meeting_12345",
            "zoom_recording_id": "test_recording_67890",
            "status": "pending",
            "error_message": None,
            "youtube_video_id": None,
            "created_at": datetime.now().isoformat(),
            "updated_at": None
        }
    
    def get_meeting_info(self, meeting_id: str) -> Optional[Dict]:
        """GET /api/meetings/{meeting_id}"""
        return self.meetings.get(meeting_id)
    
    def get_recording_info(self, recording_id: str) -> Optional[Dict]:
        """GET /api/recordings/{recording_id}"""
        return self.recordings.get(recording_id)
    
    def get_recordings_for_meeting(self, meeting_id: str) -> List[Dict]:
        """GET /api/meetings/{meeting_id}/recordings"""
        return [
            recording for recording in self.recordings.values() 
            if recording["meeting_id"] == meeting_id
        ]
    
    def create_job(self, meeting_id: str, recording_id: str) -> Dict:
        """POST /api/jobs"""
        job_id = f"{meeting_id}_{recording_id}"
        job = {
            "id": job_id,
            "zoom_meeting_id": meeting_id,
            "zoom_recording_id": recording_id,
            "status": "pending",
            "error_message": None,
            "youtube_video_id": None,
            "created_at": datetime.now().isoformat(),
            "updated_at": None
        }
        self.jobs[job_id] = job
        return job
    
    def update_job_status(self, job_id: str, status: str, youtube_id: Optional[str] = None, error: Optional[str] = None) -> Dict:
        """PUT /api/jobs/{job_id}"""
        if job_id not in self.jobs:
            raise ValueError(f"Job {job_id} not found")
            
        job = self.jobs[job_id]
        job["status"] = status
        job["updated_at"] = datetime.now().isoformat()
        
        if youtube_id:
            job["youtube_video_id"] = youtube_id
        if error:
            job["error_message"] = error
            
        return job
    
    def get_job(self, job_id: str) -> Optional[Dict]:
        """GET /api/jobs/{job_id}"""
        return self.jobs.get(job_id)
    
    def get_pending_jobs(self) -> List[Dict]:
        """GET /api/jobs?status=pending"""
        return [
            job for job in self.jobs.values() 
            if job["status"] == "pending"
        ]
    
    def get_job_progress(self, job_id: str) -> List[Dict]:
        """GET /api/jobs/{job_id}/progress"""
        # Mock progress history
        return [
            {
                "stage": "DOWNLOADING_RECORDING",
                "progress_percent": 25,
                "message": "Downloading video file...",
                "timestamp": datetime.now().isoformat()
            },
            {
                "stage": "GENERATING_METADATA", 
                "progress_percent": 60,
                "message": "Generating video metadata with AI...",
                "timestamp": datetime.now().isoformat()
            }
        ]


# Global mock API instance
mock_api = MockDatabaseAPI()


def get_zoom_transcript(meeting_id: str, recording_id: str) -> str:
    """
    Mock function to simulate getting transcript from Zoom.
    
    In real implementation, this would call the Zoom API to get
    the transcript for a specific recording.
    """
    return """
    Welcome everyone to our Q1 planning session. Today we'll be discussing our engineering roadmap, 
    budget allocations, and team goals for the upcoming quarter.

    First, let's review our progress from last quarter. We successfully delivered three major features 
    and improved our system performance by 40%. However, we did face some challenges with the 
    new authentication system that we'll need to address.

    For Q1, our main priorities are:
    1. Implementing the new user dashboard
    2. Migrating to the new database architecture  
    3. Improving our testing infrastructure
    4. Expanding our API capabilities

    The budget for Q1 has been approved at $500K, with 60% allocated to engineering resources 
    and 40% to infrastructure and tooling.

    Let's dive into each of these areas in more detail...
    """


async def demo_observability_features():
    """
    Demonstrate the observability features of the BAML pipeline.
    
    This shows how the emit statements provide real-time visibility
    into the video processing workflow.
    """
    print("\n🔍 Observability Demo")
    print("====================")
    
    pipeline = VideoProcessingPipeline()
    
    # Set up detailed progress tracking
    def detailed_progress_handler(event):
        timestamp = datetime.fromisoformat(event["timestamp"]).strftime("%H:%M:%S")
        print(f"[{timestamp}] {event['stage']}: {event['progress']}% - {event['message']}")
    
    pipeline.add_progress_callback(detailed_progress_handler)
    
    # Process a video and watch the progress
    result = await pipeline.process_single_video("demo_meeting_123", "demo_recording_456")
    
    print(f"\n📊 Final result: {result.status}")
    if result.youtube_video_id:
        print(f"🎬 YouTube URL: https://youtube.com/watch?v={result.youtube_video_id}")


async def demo_batch_processing():
    """
    Demonstrate batch processing capabilities.
    """
    print("\n📦 Batch Processing Demo") 
    print("========================")
    
    pipeline = VideoProcessingPipeline()
    
    # Process multiple meetings
    meeting_ids = ["meeting_001", "meeting_002", "meeting_003"]
    
    for meeting_id in meeting_ids:
        print(f"\n🎥 Processing meeting: {meeting_id}")
        results = await pipeline.process_meeting_recordings(meeting_id)
        print(f"✅ Completed {len(results)} recordings")


if __name__ == "__main__":
    asyncio.run(demo_observability_features())
    asyncio.run(demo_batch_processing())