"""
BAML-powered Video Processing Pipeline Driver

This module demonstrates how to use BAML functions to replace the original Python video_processor.py
logic with a declarative, observable, and AI-powered approach.
"""

import asyncio
import logging
from typing import Optional, List, Dict, Any
from datetime import datetime

# Import BAML client (this would be auto-generated)
try:
    from baml_client import b
    from baml_client.types import VideoProcessingJob, ProcessingProgress, ZoomMeeting
except ImportError:
    print("BAML client not found. Please run 'baml generate' first.")
    raise


class VideoProcessingPipeline:
    """
    BAML-powered video processing pipeline that orchestrates the workflow
    using BAML expression functions and LLM functions.
    """
    
    def __init__(self):
        self.logger = logging.getLogger(__name__)

    
    async def process_single_video(self, meeting_id: str, recording_id: str) -> VideoProcessingJob:
        """
        Process a single video using BAML workflow functions.
        
        This replaces the original video_processor.py process_video method
        with a BAML-powered approach that provides observability.
        """
        self.logger.info(f"Starting video processing: meeting={meeting_id}, recording={recording_id}")
        
        # Create event listener for BAML observability
        events = b.events.ProcessSingleVideo(meeting_id, recording_id)
        
        # Set up event listeners for observability
        events.on('block:*:entered', lambda event: self.logger.info(f"Entered: {event.name}"))
        events.on('block:*:exited', lambda event: self.logger.info(f"Exited: {event.name}"))
        events.on('var:progress_percent', lambda event: self.logger.info(f"Progress: {event.value}%"))
        events.on('var:start_time', lambda event: self.logger.info(f"Started at: {event.value}"))
        
        
        # Execute the workflow
        try:
            result = await b.ProcessSingleVideo(meeting_id, recording_id, events=events)
            self.logger.info(f"Video processing completed successfully: {result.youtube_video_id}")
            return result
        except Exception as e:
            self.logger.error(f"Video processing failed: {e}")
            # Create failed job record
            return VideoProcessingJob(
                id=f"{meeting_id}_{recording_id}",
                zoom_meeting_id=meeting_id,
                zoom_recording_id=recording_id,
                status="failed",
                error_message=str(e),
                youtube_video_id=None,
                created_at=datetime.now().isoformat(),
                updated_at=datetime.now().isoformat()
            )
    
    async def process_meeting_recordings(self, meeting_id: str) -> List[VideoProcessingJob]:
        """
        Process all recordings for a meeting.
        
        This demonstrates the BAML approach to handling multiple recordings
        and selecting the best one for processing.
        """
        self.logger.info(f"Processing recordings for meeting: {meeting_id}")
        
        # Set up observability for meeting processing
        events = b.events.ProcessMeetingRecordings(meeting_id)
        
        events.on('block:*:entered', lambda event: self.logger.info(f"Entered: {event.name}"))
        events.on('block:*:exited', lambda event: self.logger.info(f"Exited: {event.name}"))
        events.on('var:total_recordings', lambda event: self.logger.info(f"Found {event.value} recordings"))
        
        try:
            results = await b.ProcessMeetingRecordings(meeting_id, events=events)
            self.logger.info(f"Processed {len(results)} recordings for meeting {meeting_id}")
            return results
        except Exception as e:
            self.logger.error(f"Meeting processing failed: {e}")
            return []
    
    async def process_video_queue(self) -> List[VideoProcessingJob]:
        """
        Process the entire video processing queue.
        
        This replaces batch processing logic with BAML's queue processing.
        """
        self.logger.info("Starting video queue processing")
        
        # Set up queue observability
        events = b.events.ProcessVideoQueue()
        
        events.on('block:*:entered', lambda event: self.logger.info(f"Entered: {event.name}"))
        events.on('block:*:exited', lambda event: self.logger.info(f"Exited: {event.name}"))
        events.on('var:processed_count', lambda event: self.logger.info(f"Processed {event.value} videos"))
        events.on('var:total_pending', lambda event: self.logger.info(f"Total pending: {event.value}"))
        
        try:
            results = await b.ProcessVideoQueue(events=events)
            self.logger.info(f"Queue processing completed: {len(results)} jobs processed")
            return results
        except Exception as e:
            self.logger.error(f"Queue processing failed: {e}")
            return []
    
    async def generate_video_metadata(self, transcript: str, meeting: ZoomMeeting) -> Dict[str, Any]:
        """
        Generate video metadata using BAML LLM functions.
        
        This demonstrates how AI logic is handled declaratively in BAML
        rather than imperatively in Python.
        """
        self.logger.info("Generating video metadata with AI")
        
        # Set up metadata generation observability
        events = b.events.GenerateCompleteVideoMetadata(transcript, meeting)
        
        events.on('block:*:entered', lambda event: self.logger.info(f"Entered: {event.name}"))
        events.on('block:*:exited', lambda event: self.logger.info(f"Exited: {event.name}"))
        
        try:
            metadata = await b.GenerateCompleteVideoMetadata(transcript, meeting, events=events)
            self.logger.info(f"Generated metadata: title='{metadata.title}'")
            return {
                "title": metadata.title,
                "description": metadata.description,
                "tags": metadata.tags,
                "category": metadata.category
            }
        except Exception as e:
            self.logger.error(f"Metadata generation failed: {e}")
            return {
                "title": f"Meeting: {meeting.topic or 'Untitled'}",
                "description": "Auto-generated from meeting transcript",
                "tags": ["meeting", "recording"],
                "category": "Education"
            }
    
    async def check_pipeline_health(self) -> Dict[str, str]:
        """Check the health of all pipeline components."""
        self.logger.info("Checking pipeline health")
        
        events = b.events.CheckPipelineHealth()
        
        events.on('block:*:entered', lambda event: self.logger.info(f"Entered: {event.name}"))
        events.on('block:*:exited', lambda event: self.logger.info(f"Exited: {event.name}"))
        
        try:
            health_status = await b.CheckPipelineHealth(events=events)
            self.logger.info(f"Pipeline health: {health_status}")
            return health_status
        except Exception as e:
            self.logger.error(f"Health check failed: {e}")
            return {
                "zoom": "unknown",
                "youtube": "unknown", 
                "database": "unknown",
                "overall": "unhealthy"
            }


async def main():
    """
    Demo script showing how to use the BAML video processing pipeline.
    
    This replaces the original video_processor.py global instance approach
    with a more structured, observable workflow.
    """
    # Set up logging
    logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
    
    # Create pipeline
    pipeline = VideoProcessingPipeline()
    
    print("🎬 AI Content Pipeline - BAML Edition")
    print("=====================================")
    
    # Check pipeline health first
    print("\n📊 Checking pipeline health...")
    health = await pipeline.check_pipeline_health()
    print(f"Health status: {health['overall']}")
    
    # Process a single video
    print("\n🎥 Processing single video...")
    meeting_id = "test_meeting_12345"
    recording_id = "test_recording_67890"
    
    result = await pipeline.process_single_video(meeting_id, recording_id)
    print(f"✅ Processing completed: {result.status}")
    
    if result.youtube_video_id:
        print(f"🎬 YouTube video ID: {result.youtube_video_id}")
    
    # Process entire queue
    print("\n📋 Processing video queue...")
    queue_results = await pipeline.process_video_queue()
    print(f"✅ Queue processed: {len(queue_results)} jobs completed")
    
    print("\n🎉 Pipeline demo completed!")


if __name__ == "__main__":
    asyncio.run(main())