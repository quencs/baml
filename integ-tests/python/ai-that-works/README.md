# AI Content Pipeline - BAML Implementation

This directory contains the BAML port of the AI Content Pipeline, demonstrating how Python workflow logic can be migrated to BAML for improved observability, type safety, and AI integration.

## Project Structure

```
ai-that-works/
├── video_pipeline_driver.py    # Main Python driver using BAML functions
├── api_server_mock.py          # Mock API server for database operations  
├── comparison_analysis.py      # Analysis comparing Python vs BAML approaches
└── README.md                   # This file

../baml_src/ai-content-pipeline/
├── types.baml                  # Data models and enums
├── llm_functions.baml          # AI-powered functions for metadata generation
├── expression_functions.baml   # Workflow orchestration functions
└── api_functions.baml          # External API integration functions
```

## Key Features Demonstrated

### 1. **Declarative AI Logic**
- LLM functions replace embedded Python AI calls
- Type-safe input/output with automatic validation
- Easy prompt modification without code changes

### 2. **Built-in Observability** 
- `emit` statements provide real-time workflow visibility
- Event-driven progress tracking
- No manual instrumentation required

### 3. **Composable Workflows**
- Expression functions orchestrate complex workflows
- Clear separation of concerns
- Reusable components for different use cases

### 4. **Type Safety**
- Compile-time type checking for all data flows
- Structured data models replace dynamic dictionaries
- Automatic serialization/deserialization

## Usage Example

```python
from video_pipeline_driver import VideoProcessingPipeline

# Create pipeline with observability
pipeline = VideoProcessingPipeline()

# Add progress tracking
pipeline.add_progress_callback(lambda event: 
    print(f"Progress: {event['stage']} - {event['progress']}%")
)

# Process a single video
result = await pipeline.process_single_video("meeting_123", "recording_456")
print(f"YouTube URL: {result.youtube_video_id}")
```

## Observability Features

The BAML implementation provides comprehensive observability through emit statements:

- **Workflow Progress**: Track overall pipeline progress
- **Stage Transitions**: Monitor movement between processing stages  
- **Error Tracking**: Count retries and capture error details
- **Resource Usage**: Monitor API calls and file operations
- **Batch Operations**: Track progress across multiple videos

## Comparison with Original Implementation

| Aspect | Original Python | BAML Implementation |
|--------|----------------|-------------------|
| **Code Organization** | Mixed concerns in single class | Clear separation: AI, workflow, API |
| **Observability** | Manual logging | Built-in emit statements |
| **AI Integration** | Embedded prompt strings | Declarative LLM functions |
| **Error Handling** | Scattered try/catch | Structured retry patterns |
| **Type Safety** | Runtime validation | Compile-time checking |
| **Testing** | Unit tests for methods | BAML test cases with assertions |
| **Maintainability** | Tightly coupled logic | Loosely coupled, composable functions |

## Migration Benefits

1. **Improved Developer Experience**: Clear workflow visualization and real-time feedback
2. **Better AI Integration**: Dedicated LLM functions with proper templating
3. **Enhanced Debugging**: Event-driven observability shows exactly where issues occur
4. **Easier Maintenance**: Modular functions can be updated independently
5. **Type Safety**: Catch integration errors at compile time
6. **Scalability**: Expression functions can be easily parallelized and optimized

## Running the Demo

```bash
# Generate BAML client code
baml generate

# Run the pipeline demo
python video_pipeline_driver.py

# Run comparison analysis  
python comparison_analysis.py
```

## API Server Integration

Since BAML doesn't have direct database access, the implementation assumes an API server that proxies database operations:

- `GET /api/meetings/{id}` - Fetch meeting information
- `GET /api/recordings/{id}` - Fetch recording details
- `POST /api/jobs` - Create processing jobs
- `PUT /api/jobs/{id}` - Update job status
- `GET /api/jobs?status=pending` - Fetch pending jobs

The `api_server_mock.py` file provides a mock implementation for testing purposes.

## Future Enhancements

- **Real API Integration**: Replace mock functions with actual HTTP clients
- **Database Support**: Once BAML adds database capabilities, eliminate API proxy layer
- **Advanced Error Recovery**: Implement more sophisticated retry and fallback strategies
- **Performance Monitoring**: Add metrics collection for optimization
- **Webhook Integration**: Support real-time notifications for job status changes