use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use baml_types::tracing::events::{HTTPBody, HTTPRequest, HTTPResponse};
use baml_ids::HttpRequestId;
use serde_json::{json, Value};

/// Collects HTTP request/response pairs and writes them as JSONL test pairs
/// for use with mock servers and testing frameworks.
#[derive(Debug)]
pub struct TestPairCollector {
    /// Directory where test pair files will be written
    output_dir: PathBuf,
    /// Whether the collector is enabled
    enabled: bool,
    /// Files currently open for writing, keyed by filename
    open_files: Arc<Mutex<HashMap<String, fs::File>>>,
    /// Temporary storage for HTTP requests waiting for their responses
    pending_requests: Arc<Mutex<HashMap<HttpRequestId, HTTPRequest>>>,
    /// Environment variables for sanitizing sensitive header values
    env_vars: Arc<Mutex<HashMap<String, String>>>,
}

impl TestPairCollector {
    /// Create a new TestPairCollector
    pub fn new(output_dir: PathBuf, env_vars: HashMap<String, String>) -> Result<Self> {
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir)
                .with_context(|| format!("Failed to create test pairs directory: {:?}", output_dir))?;
        }

        Ok(Self {
            output_dir,
            enabled: true,
            open_files: Arc::new(Mutex::new(HashMap::new())),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            env_vars: Arc::new(Mutex::new(env_vars)),
        })
    }

    /// Create a disabled collector (no-op)
    pub fn disabled() -> Self {
        Self {
            output_dir: PathBuf::new(),
            enabled: false,
            open_files: Arc::new(Mutex::new(HashMap::new())),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            env_vars: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if collector is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Update the environment variables used for sanitization
    pub fn update_env_vars(&self, env_vars: HashMap<String, String>) {
        if self.enabled {
            *self.env_vars.lock().unwrap() = env_vars;
        }
    }

    /// Store an HTTP request for later matching with its response
    pub fn store_request(&self, request: HTTPRequest) {
        if !self.enabled {
            return;
        }
        
        if let Ok(mut pending) = self.pending_requests.lock() {
            pending.insert(request.id().clone(), request);
        }
    }

    /// Retrieve and remove a stored request by its ID
    pub fn take_request(&self, request_id: &HttpRequestId) -> Option<HTTPRequest> {
        if !self.enabled {
            return None;
        }
        
        if let Ok(mut pending) = self.pending_requests.lock() {
            pending.remove(request_id)
        } else {
            None
        }
    }

    /// Get the output directory
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Capture a request/response pair and write it to appropriate JSONL file
    pub fn capture_request_response(
        &self,
        request: &HTTPRequest,
        response: &HTTPResponse,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let test_pair = self.create_test_pair(request, response)?;
        let filename = self.determine_filename(request);
        
        self.write_test_pair(&filename, &test_pair)
            .with_context(|| format!("Failed to write test pair to {}", filename))?;

        Ok(())
    }

    /// Create a test pair JSON object from request and response
    fn create_test_pair(
        &self,
        request: &HTTPRequest,
        response: &HTTPResponse,
    ) -> Result<Value> {
        // Convert HTTPRequest to a JSON object that matches OpenAI API format
        let input = json!({
            "url": request.url(),
            "method": request.method(),
            "headers": self.headers_to_json(request.headers())?,
            "body": self.body_to_json(request.body())?
        });

        // Convert HTTPResponse to a JSON object
        let output = if self.is_error_response(response) {
            // For error responses, structure as an error object
            json!({
                "error": {
                    "message": self.extract_error_message(response)?,
                    "type": self.determine_error_type(response),
                    "code": response.status
                }
            })
        } else {
            // For success responses, parse the body as JSON
            self.body_to_json(&response.body)?
        };

        Ok(json!({
            "input": input,
            "output": output
        }))
    }

    /// Determine the filename for a test pair based on the request
    fn determine_filename(&self, request: &HTTPRequest) -> String {
        // Extract endpoint from URL to determine file type
        if request.url.contains("/chat/completions") {
            "chat-completion.jsonl".to_string()
        } else if request.url.contains("/completions") {
            "completion.jsonl".to_string()
        } else if request.url.contains("/embeddings") {
            "embeddings.jsonl".to_string()
        } else {
            // Default filename for unknown endpoints
            "other-requests.jsonl".to_string()
        }
    }

    /// Write a test pair to the specified JSONL file
    fn write_test_pair(&self, filename: &str, test_pair: &Value) -> Result<()> {
        
        let mut files = self.open_files.lock().unwrap();
        
        let file = match files.get_mut(filename) {
            Some(file) => {
                file
            }
            None => {
                let file_path = self.output_dir.join(filename);
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&file_path)
                    .with_context(|| format!("Failed to open test pairs file: {:?}", file_path))?;
                files.insert(filename.to_string(), file);
                files.get_mut(filename).unwrap()
            }
        };

        writeln!(file, "{}", serde_json::to_string(test_pair)?)?;
        file.flush()?;

        Ok(())
    }

    /// Convert headers to JSON object, sanitizing environment variable values
    fn headers_to_json(&self, headers: &HashMap<String, String>) -> Result<Value> {
        let env_vars = self.env_vars.lock().unwrap();
        let mut json_headers = serde_json::Map::new();
        
        for (key, value) in headers {
            let sanitized_value = if key.to_lowercase() == "authorization" && value.starts_with("Bearer ") {
                // Special handling for Bearer tokens
                let token = &value[7..]; // Skip "Bearer " prefix
                env_vars.iter()
                    .find(|(_, env_value)| *env_value == token)
                    .map(|(env_name, _)| format!("Bearer {{{{ {} }}}}", env_name))
                    .unwrap_or_else(|| value.clone())
            } else {
                // Check if this header value matches any environment variable value
                env_vars.iter()
                    .find(|(_, env_value)| *env_value == value)
                    .map(|(env_name, _)| format!("{{{{ {} }}}}", env_name))
                    .unwrap_or_else(|| value.clone())
            };
            
            json_headers.insert(key.clone(), Value::String(sanitized_value));
        }
        Ok(Value::Object(json_headers))
    }

    /// Convert HTTPBody to JSON object
    fn body_to_json(&self, body: &HTTPBody) -> Result<Value> {
        if body.raw().is_empty() {
            return Ok(Value::Null);
        }

        let body_str = std::str::from_utf8(body.raw())
            .context("Request body is not valid UTF-8")?;

        // Try to parse as JSON first
        match serde_json::from_str::<Value>(body_str) {
            Ok(json_value) => Ok(json_value),
            Err(_) => {
                // If not JSON, store as string
                Ok(Value::String(body_str.to_string()))
            }
        }
    }

    /// Check if response indicates an error
    fn is_error_response(&self, response: &HTTPResponse) -> bool {
        response.status >= 400
    }

    /// Extract error message from response body
    fn extract_error_message(&self, response: &HTTPResponse) -> Result<String> {
        let body_str = std::str::from_utf8(response.body.raw())
            .unwrap_or("Invalid response body");

        // Try to parse as JSON and extract error message
        if let Ok(json_value) = serde_json::from_str::<Value>(body_str) {
            if let Some(error) = json_value.get("error") {
                if let Some(message) = error.get("message") {
                    if let Some(msg_str) = message.as_str() {
                        return Ok(msg_str.to_string());
                    }
                }
            }
        }

        // Fallback to status code description
        Ok(format!("HTTP {} error", response.status))
    }

    /// Determine error type from response
    fn determine_error_type(&self, response: &HTTPResponse) -> String {
        match response.status {
            400 => "invalid_request_error".to_string(),
            401 => "authentication_error".to_string(),
            403 => "permission_error".to_string(),
            404 => "not_found_error".to_string(),
            429 => "rate_limit_error".to_string(),
            500 => "internal_server_error".to_string(),
            502 => "api_error".to_string(),
            503 => "service_unavailable_error".to_string(),
            _ => "api_error".to_string(),
        }
    }

    /// Flush all open files and close them
    pub fn flush_and_close(&self) -> Result<()> {
        let mut files = self.open_files.lock().unwrap();
        for (_, mut file) in files.drain() {
            file.flush().context("Failed to flush test pairs file")?;
        }
        Ok(())
    }
}

/// Expand tilde (~) in path to home directory
fn expand_tilde_path(path_str: &str) -> PathBuf {
    if path_str.starts_with("~/") {
        if let Some(home_dir) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home_dir);
            path.push(&path_str[2..]);  // Skip the "~/"
            return path;
        }
    }
    PathBuf::from(path_str)
}

/// Parse the BAML_GENERATE_TEST_PAIRS environment variable and create a collector
pub fn create_from_env(env_vars: HashMap<String, String>) -> Result<Option<TestPairCollector>> {
    match std::env::var("BAML_GENERATE_TEST_PAIRS") {
        Ok(path_str) if !path_str.is_empty() => {
            let path = expand_tilde_path(&path_str);
            Ok(Some(TestPairCollector::new(path, env_vars)?))
        }
        _ => Ok(None),
    }
}
