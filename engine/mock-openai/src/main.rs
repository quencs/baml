use axum::{
    extract::{Json, State, Query},
    http::{HeaderMap, StatusCode, Method, Uri},
    response::IntoResponse,
    Router,
};
use chrono::Utc;
use clap::Parser;
use serde::Deserialize;
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
    test_pairs: HashMap<String, TestPairInfo>,
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
    
    // Log some sample keys for debugging
    if !test_pairs.is_empty() {
        info!("Sample test pair keys (first 3):");
        for (i, key) in test_pairs.keys().take(3).enumerate() {
            info!("  {}: {} chars", i + 1, key.len());
            if key.len() < 200 {
                info!("    Key: {}", key);
            } else {
                info!("    Key (truncated): {}...", &key[..200]);
            }
        }
    }
    
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
        test_pairs,
    });

    let app = Router::new()
        .fallback(handle_request)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "127.0.0.1:3000";
    info!("Generic HTTP mock server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

async fn handle_request(
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    Query(query_params): Query<HashMap<String, String>>,
    body: Option<Json<Value>>,
) -> impl IntoResponse {
    info!("Received {} request to {}", method, uri);

    // Extract baml-original-url header if present
    let original_url = headers
        .get("baml-original-url")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("http://localhost:3000{}", uri));

    // Create headers map excluding baml-original-url
    let mut headers_map = HashMap::new();
    for (name, value) in headers.iter() {
        if name.as_str() != "baml-original-url" {
            if let Ok(v) = value.to_str() {
                headers_map.insert(name.as_str().to_string(), v.to_string());
            }
        }
    }

    // Create input object matching BAML test pair format
    let input_obj = serde_json::json!({
        "url": original_url,
        "method": method.to_string(),
        "headers": headers_map,
        "body": body.map(|Json(v)| v).unwrap_or(Value::Null)
    });

    // Check for test pair match first
    let input_key = serde_json::to_string(&input_obj).unwrap_or_default();
    info!("Looking for test pair with key length: {} chars", input_key.len());
    info!("Using original URL: {}", original_url);
    
    if let Some(test_info) = state.test_pairs.get(&input_key) {
        info!("✅ Found test pair match from {}, line {}", test_info.file_path.display(), test_info.line_number);
        
        // Check if the test output is an error response
        if let Some(error_obj) = test_info.output.get("error") {
            let status_code = error_obj.get("code")
                .and_then(|c| c.as_u64())
                .map(|c| StatusCode::from_u16(c as u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            
            return (status_code, Json(test_info.output.clone())).into_response();
        }
        
        // Return the test output as a successful response
        return (StatusCode::OK, Json(test_info.output.clone())).into_response();
    } else {
        info!("❌ No test pair match found");
    }

    // Fall back to generic mock behavior
    let mock_response = serde_json::json!({
        "message": format!("Mock response for {} {}", method, uri),
        "timestamp": Utc::now().timestamp(),
        "path": uri.to_string(),
        "method": method.to_string(),
        "query_params": query_params,
        "note": "This is a fallback mock response. No test pair was found for this request."
    });

    (StatusCode::OK, Json(mock_response)).into_response()
}
