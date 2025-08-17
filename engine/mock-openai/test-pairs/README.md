# Mock OpenAI Test Pairs

This directory contains JSONL files with test pairs for the mock OpenAI API. Each line in the JSONL files contains an object with:

- `input`: The OpenAI API request object
- `output`: The expected OpenAI API response object

## Files

### completion.jsonl
Test pairs for the `/v1/completions` endpoint (text completion).

Examples include:
- Basic completion requests
- Requests with various parameters (temperature, max_tokens, etc.)
- Empty prompts
- Different models

### chat-completion.jsonl
Test pairs for the `/v1/chat/completions` endpoint (chat completion).

Examples include:
- Single user messages
- Multi-turn conversations
- System messages
- Empty message arrays
- Various model types

### error-cases.jsonl
Test pairs for error scenarios that should return error responses.

Examples include:
- Streaming requests (not implemented)
- Invalid models
- Invalid parameters
- Missing required fields
- Invalid message roles

## Usage

These test files can be used for:
1. Unit testing the mock API implementation
2. Integration testing with client libraries
3. Validation of request/response formats
4. Documentation of expected behavior

Each JSONL file can be read line by line, with each line being a valid JSON object containing the test case.
