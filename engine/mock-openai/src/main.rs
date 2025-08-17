use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use chrono::Utc;
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory containing test pair JSONL files
    #[arg(short, long, default_value = "test-pairs")]
    test_pairs_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct TestPair {
    input: Value,
    output: Value,
}

#[derive(Debug, Clone)]
struct TestPairInfo {
    output: Value,
    file_path: PathBuf,
    line_number: usize,
}

#[derive(Debug, Clone)]
struct AppState {
    model_name: String,
    test_pairs: HashMap<String, TestPairInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CompletionRequest {
    model: String,
    prompt: Option<String>,
    messages: Option<Vec<Message>>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    n: Option<u32>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    user: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CompletionResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    text: Option<String>,
    message: Option<Message>,
    index: u32,
    finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    code: Option<String>,
}

fn remove_line_from_file(file_path: &Path, line_number: usize) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let lines: Vec<&str> = content.lines().collect();
    
    if line_number == 0 || line_number > lines.len() {
        return Err(format!("Invalid line number: {}", line_number).into());
    }
    
    let new_content: Vec<&str> = lines
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i != line_number - 1) // Convert to 0-based index
        .map(|(_, line)| line)
        .collect();
    
    fs::write(file_path, new_content.join("\n") + "\n")?;
    Ok(())
}

fn load_test_pairs(test_pairs_dir: &Path) -> HashMap<String, TestPairInfo> {
    let mut test_pairs = HashMap::new();
    
    if !test_pairs_dir.exists() {
        warn!("Test pairs directory does not exist: {:?}", test_pairs_dir);
        return test_pairs;
    }

    info!("Loading test pairs from: {:?}", test_pairs_dir);

    let entries = match fs::read_dir(test_pairs_dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Failed to read test pairs directory: {}", e);
            return test_pairs;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                warn!("Failed to read directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        if path.extension() != Some(std::ffi::OsStr::new("jsonl")) {
            continue;
        }

        info!("Loading test pairs from file: {:?}", path);

        let file = match fs::File::open(&path) {
            Ok(file) => file,
            Err(e) => {
                warn!("Failed to open file {:?}: {}", path, e);
                continue;
            }
        };

        let reader = BufReader::new(file);
        for (line_num, line) in reader.lines().enumerate() {
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    warn!("Failed to read line {} in {:?}: {}", line_num + 1, path, e);
                    continue;
                }
            };

            let test_pair: TestPair = match serde_json::from_str(&line) {
                Ok(pair) => pair,
                Err(e) => {
                    warn!("Failed to parse line {} in {:?}: {}", line_num + 1, path, e);
                    continue;
                }
            };

            // Create a consistent key from the input
            let input_key = serde_json::to_string(&test_pair.input).unwrap_or_else(|_| {
                format!("invalid_input_{}", line_num)
            });

            // Check for duplicates
            if let Some(existing_info) = test_pairs.get(&input_key) {
                warn!("Duplicate input found in {:?} at line {}", path, line_num + 1);
                
                let existing_output_str = serde_json::to_string_pretty(&existing_info.output).unwrap_or_default();
                let new_output_str = serde_json::to_string_pretty(&test_pair.output).unwrap_or_default();
                
                println!("\n🔄 DUPLICATE INPUT DETECTED");
                println!("Existing: {:?}, Line: {}", existing_info.file_path, existing_info.line_number);
                println!("New: {:?}, Line: {}", path, line_num + 1);
                println!("Input: {}", serde_json::to_string_pretty(&test_pair.input).unwrap_or_default());
                println!("\n📊 OUTPUT DIFFERENCES:");
                
                let diff = TextDiff::from_lines(&existing_output_str, &new_output_str);
                for change in diff.iter_all_changes() {
                    let sign = match change.tag() {
                        ChangeTag::Delete => "- ",
                        ChangeTag::Insert => "+ ",
                        ChangeTag::Equal => "  ",
                    };
                    print!("{}{}", sign, change);
                }
                
                println!("\n🤔 Which output would you like to keep?");
                println!("1. Keep existing output (delete new entry)");
                println!("2. Use new output (delete existing entry)");
                print!("Enter choice (1 or 2): ");
                
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                
                match input.trim() {
                    "2" => {
                        // Delete the existing entry from its file
                        match remove_line_from_file(&existing_info.file_path, existing_info.line_number) {
                            Ok(()) => {
                                println!("🗑️ Deleted existing entry from {:?}", existing_info.file_path);
                                test_pairs.insert(input_key, TestPairInfo {
                                    output: test_pair.output,
                                    file_path: path.clone(),
                                    line_number: line_num + 1,
                                });
                                println!("✅ Using new output");
                            }
                            Err(e) => {
                                warn!("Failed to delete existing entry: {}", e);
                                println!("⚠️ Could not delete existing entry, keeping both for now");
                            }
                        }
                    }
                    _ => {
                        // Delete the new entry from current file
                        match remove_line_from_file(&path, line_num + 1) {
                            Ok(()) => {
                                println!("🗑️ Deleted new entry from {:?}", path);
                                println!("✅ Keeping existing output");
                            }
                            Err(e) => {
                                warn!("Failed to delete new entry: {}", e);
                                println!("⚠️ Could not delete new entry, keeping both for now");
                            }
                        }
                    }
                }
            } else {
                test_pairs.insert(input_key, TestPairInfo {
                    output: test_pair.output,
                    file_path: path.clone(),
                    line_number: line_num + 1,
                });
            }
        }
    }

    info!("Loaded {} test pairs", test_pairs.len());
    test_pairs
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    
    // Load test pairs from the specified directory
    let test_pairs = load_test_pairs(&args.test_pairs_dir);

    let state = Arc::new(AppState {
        model_name: "gpt-3.5-turbo".to_string(),
        test_pairs,
    });

    let app = Router::new()
        .route("/v1/completions", post(handle_completions))
        .route("/v1/chat/completions", post(handle_chat_completions))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "127.0.0.1:3000";
    info!("Mock OpenAI server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

async fn handle_completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CompletionRequest>,
) -> impl IntoResponse {
    info!("Received completion request: {:?}", req);

    // Check for test pair match first
    let input_key = serde_json::to_string(&req).unwrap_or_default();
    if let Some(test_info) = state.test_pairs.get(&input_key) {
        info!("Using test pair response for completion request");
        
        // Check if the test output is an error response
        if let Some(_error_obj) = test_info.output.get("error") {
            let error_response: ErrorResponse = serde_json::from_value(test_info.output.clone())
                .unwrap_or_else(|_| ErrorResponse {
                    error: ErrorDetail {
                        message: "Invalid test pair error format".to_string(),
                        error_type: "test_error".to_string(),
                        code: None,
                    },
                });
            return (StatusCode::BAD_REQUEST, Json(error_response)).into_response();
        }
        
        // Parse the test output as a completion response
        if let Ok(response) = serde_json::from_value::<CompletionResponse>(test_info.output.clone()) {
            return (StatusCode::OK, Json(response)).into_response();
        }
    }

    // Fall back to original mock behavior
    if req.stream == Some(true) {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(ErrorResponse {
                error: ErrorDetail {
                    message: "Streaming is not yet implemented".to_string(),
                    error_type: "not_implemented".to_string(),
                    code: None,
                },
            }),
        )
            .into_response();
    }

    let prompt = req.prompt.unwrap_or_default();
    let mock_response = generate_mock_completion(&prompt, &req.model);

    let response = CompletionResponse {
        id: format!("cmpl-{}", Uuid::new_v4()),
        object: "text_completion".to_string(),
        created: Utc::now().timestamp(),
        model: req.model.clone(),
        choices: vec![Choice {
            text: Some(mock_response.clone()),
            message: None,
            index: 0,
            finish_reason: "stop".to_string(),
        }],
        usage: Usage {
            prompt_tokens: prompt.split_whitespace().count() as u32 * 2,
            completion_tokens: mock_response.split_whitespace().count() as u32 * 2,
            total_tokens: (prompt.split_whitespace().count() + mock_response.split_whitespace().count()) as u32 * 2,
        },
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn handle_chat_completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CompletionRequest>,
) -> impl IntoResponse {
    info!("Received chat completion request: {:?}", req);

    // Check for test pair match first
    let input_key = serde_json::to_string(&req).unwrap_or_default();
    if let Some(test_info) = state.test_pairs.get(&input_key) {
        info!("Using test pair response for chat completion request");
        
        // Check if the test output is an error response
        if let Some(_error_obj) = test_info.output.get("error") {
            let error_response: ErrorResponse = serde_json::from_value(test_info.output.clone())
                .unwrap_or_else(|_| ErrorResponse {
                    error: ErrorDetail {
                        message: "Invalid test pair error format".to_string(),
                        error_type: "test_error".to_string(),
                        code: None,
                    },
                });
            return (StatusCode::BAD_REQUEST, Json(error_response)).into_response();
        }
        
        // Parse the test output as a completion response
        if let Ok(response) = serde_json::from_value::<CompletionResponse>(test_info.output.clone()) {
            return (StatusCode::OK, Json(response)).into_response();
        }
    }

    // Fall back to original mock behavior
    if req.stream == Some(true) {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(ErrorResponse {
                error: ErrorDetail {
                    message: "Streaming is not yet implemented".to_string(),
                    error_type: "not_implemented".to_string(),
                    code: None,
                },
            }),
        )
            .into_response();
    }

    let messages = req.messages.unwrap_or_default();
    let default_content = String::new();
    let last_message = messages
        .last()
        .map(|m| &m.content)
        .unwrap_or(&default_content);

    let mock_response = generate_mock_chat_response(last_message, &req.model);

    let response = CompletionResponse {
        id: format!("chatcmpl-{}", Uuid::new_v4()),
        object: "chat.completion".to_string(),
        created: Utc::now().timestamp(),
        model: req.model.clone(),
        choices: vec![Choice {
            text: None,
            message: Some(Message {
                role: "assistant".to_string(),
                content: mock_response.clone(),
            }),
            index: 0,
            finish_reason: "stop".to_string(),
        }],
        usage: Usage {
            prompt_tokens: messages.iter().map(|m| m.content.split_whitespace().count()).sum::<usize>() as u32 * 2,
            completion_tokens: mock_response.split_whitespace().count() as u32 * 2,
            total_tokens: (messages.iter().map(|m| m.content.split_whitespace().count()).sum::<usize>() 
                + mock_response.split_whitespace().count()) as u32 * 2,
        },
    };

    (StatusCode::OK, Json(response)).into_response()
}

fn generate_mock_completion(prompt: &str, model: &str) -> String {
    match prompt.to_lowercase() {
        p if p.contains("hello") => "Hello! How can I assist you today?".to_string(),
        p if p.contains("weather") => "I'm a mock API and cannot provide real weather information.".to_string(),
        p if p.contains("test") => "This is a test response from the mock OpenAI API.".to_string(),
        _ => format!("Mock response for prompt: '{}' using model: {}", prompt, model),
    }
}

fn generate_mock_chat_response(message: &str, model: &str) -> String {
    match message.to_lowercase() {
        m if m.contains("hello") => "Hello! I'm a mock assistant. How can I help you today?".to_string(),
        m if m.contains("weather") => "I'm a mock API and cannot provide real weather information, but I can tell you it's always sunny in the mock world!".to_string(),
        m if m.contains("test") => "This is a test response from the mock OpenAI chat API.".to_string(),
        m if m.contains("who are you") => format!("I'm a mock version of {} running on a local server.", model),
        _ => format!("Mock chat response for: '{}' using model: {}", message, model),
    }
}