# Mock OpenAI Server

A simple mock server that implements the OpenAI completions API using Axum.

## Building

```bash
cargo build
```

## Running

```bash
cargo run
```

The server will start on `http://127.0.0.1:3000`

## API Endpoints

- `POST /v1/completions` - Text completions endpoint
- `POST /v1/chat/completions` - Chat completions endpoint

## Example Usage

### Chat Completions
```bash
curl -X POST http://127.0.0.1:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {"role": "user", "content": "Hello"}
    ]
  }'
```

### Text Completions
```bash
curl -X POST http://127.0.0.1:3000/v1/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "text-davinci-003",
    "prompt": "Hello"
  }'
```

## Features

- Mock responses based on input patterns
- Proper OpenAI response format
- Token usage calculation
- CORS support
- Request logging