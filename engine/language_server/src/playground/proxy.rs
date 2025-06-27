use std::sync::Arc;
use anyhow::Result;
use warp::{Filter, Reply, Rejection};
use serde_json::Value;

// Custom response type for binary data
struct BinaryResponse {
    body: Vec<u8>,
    status: http::StatusCode,
}

impl warp::Reply for BinaryResponse {
    fn into_response(self) -> warp::http::Response<warp::hyper::Body> {
        warp::http::Response::builder()
            .status(self.status)
            .header("access-control-allow-origin", "*")
            .body(warp::hyper::Body::from(self.body))
            .unwrap()
    }
}

// Custom error type for proxy
#[derive(Debug)]
struct ProxyError;

impl warp::reject::Reject for ProxyError {}

// API keys for model providers - these should be injected into requests
const API_KEY_INJECTION_ALLOWED: &[(&str, &str, &str, &str)] = &[
    ("https://api.openai.com", "Authorization", "OPENAI_API_KEY", "baml-openai-api-key"),
    ("https://api.anthropic.com", "x-api-key", "ANTHROPIC_API_KEY", "baml-anthropic-api-key"),
    ("https://generativelanguage.googleapis.com", "x-goog-api-key", "GOOGLE_API_KEY", "baml-google-api-key"),
    ("https://openrouter.ai", "Authorization", "OPENROUTER_API_KEY", "baml-openrouter-api-key"),
    ("https://api.llmapi.com", "Authorization", "LLAMA_API_KEY", "baml-llama-api-key"),
];

// Temporary dummy API keys for testing (remove in production)
const DUMMY_API_KEYS: &[(&str, &str)] = &[
    ("OPENAI_API_KEY", "sk-dummy-openai-key-for-testing-only"),
    ("ANTHROPIC_API_KEY", "sk-ant-dummy-anthropic-key-for-testing-only"),
    ("GOOGLE_API_KEY", "dummy-google-api-key-for-testing-only"),
    ("OPENROUTER_API_KEY", "sk-dummy-openrouter-key-for-testing-only"),
    ("LLAMA_API_KEY", "sk-dummy-llama-key-for-testing-only"),
];

pub struct ProxyServer {
    port: u16,
}

impl ProxyServer {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn start(self) -> Result<()> {
        let addr = ([127, 0, 0, 1], self.port);
        
        // Handle OPTIONS requests (preflight CORS requests)
        let cors_route = warp::options()
            .and(warp::path::tail())
            .map(|_| {
                warp::http::Response::builder()
                    .status(http::StatusCode::OK)
                    .header("access-control-allow-origin", "*")
                    .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS")
                    .header("access-control-allow-headers", "Content-Type, Authorization, x-api-key, baml-original-url, baml-openai-api-key, baml-anthropic-api-key, baml-google-api-key, baml-openrouter-api-key, baml-llama-api-key")
                    .header("access-control-max-age", "86400")
                    .body(warp::hyper::Body::empty())
                    .unwrap()
            });

        // Proxy all requests - use a catch-all route that matches any path
        let proxy_route = warp::any()
            .and(warp::body::bytes())
            .and(warp::method())
            .and(warp::path::tail()) // Use tail() to capture any path
            .and(warp::header::headers_cloned())
            .and_then(handle_proxy_request);

        // Combine CORS and proxy routes
        let routes = cors_route.or(proxy_route);

        tracing::info!("Proxy server listening on port {}", self.port);
        warp::serve(routes)
            .run(addr)
            .await;

        Ok(())
    }
}

async fn handle_proxy_request(
    body: bytes::Bytes,
    method: http::Method,
    path: warp::path::Tail,
    mut headers: http::HeaderMap,
) -> Result<BinaryResponse, Rejection> {
    let path_str = path.as_str();
    tracing::info!("[PROXY] Received request: {} {}", method, path_str);
    
    // Debug: Log request headers and body
    tracing::info!("[PROXY] Request headers: {:?}", headers);
    tracing::info!("[PROXY] Request body length: {} bytes", body.len());
    if body.len() < 1000 { // Only log small bodies to avoid spam
        tracing::info!("[PROXY] Request body: {}", String::from_utf8_lossy(&body));
    }

    // Get the original URL from the header (same as VSCode extension)
    let original_url = match headers.get("baml-original-url") {
        Some(url) => match url.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                tracing::warn!("[PROXY] Invalid baml-original-url header");
                return Ok(BinaryResponse { body: Vec::new(), status: http::StatusCode::BAD_REQUEST });
            }
        },
        None => {
            tracing::warn!("[PROXY] Missing baml-original-url header");
            return Ok(BinaryResponse { body: Vec::new(), status: http::StatusCode::BAD_REQUEST });
        }
    };

    tracing::info!("[PROXY] Original URL: {}", original_url);

    // Clean up headers that upstream may reject (same as VSCode extension)
    headers.remove("baml-original-url");
    headers.remove("origin");
    headers.remove("authorization"); // Remove frontend's Authorization header to avoid conflicts
    headers.remove("host"); // Remove host header so reqwest sets it based on the target URL

    // Parse the original URL - strip trailing slash like VSCode extension
    let clean_original_url = if original_url.ends_with('/') {
        &original_url[..original_url.len() - 1]
    } else {
        &original_url
    };
    
    let url = match url::Url::parse(clean_original_url) {
        Ok(url) => url,
        Err(_) => {
            return Ok(BinaryResponse { body: Vec::new(), status: http::StatusCode::BAD_REQUEST });
        }
    };

    // Handle image requests by clearing the path (same as VSCode extension)
    if path_str.matches('.').count() == 1 && method == http::Method::GET {
        tracing::info!("[PROXY] Image request detected, clearing path: {}", path_str);
        return Ok(BinaryResponse { body: Vec::new(), status: http::StatusCode::OK });
    }

    // Build the target URL following VSCode extension logic
    let mut target_url = url.clone();
    
    // Get the base path from the original URL (like VSCode extension)
    let base_path = if url.path().ends_with('/') {
        url.path().trim_end_matches('/').to_string()
    } else {
        url.path().to_string()
    };
    
    // Construct the final path following VSCode logic
    let final_path = if base_path.is_empty() {
        // Remove trailing slash from path_str like VSCode extension
        if path_str.ends_with('/') {
            path_str[..path_str.len() - 1].to_string()
        } else {
            path_str.to_string()
        }
    } else {
        // Guard against double-prefixing like VSCode extension
        if !path_str.starts_with(&base_path) {
            // Ensure there's exactly one slash between basePath and existing path
            if path_str.starts_with('/') {
                format!("{}{}", base_path, path_str)
            } else {
                format!("{}/{}", base_path, path_str)
            }
        } else {
            path_str.to_string()
        }
    };
    
    // Remove trailing slash from final path like VSCode extension
    let clean_final_path = if final_path.ends_with('/') {
        &final_path[..final_path.len() - 1]
    } else {
        &final_path
    };
    
    target_url.set_path(clean_final_path);
    
    tracing::info!("[PROXY] {} {} → {:?}", method, path_str, url.origin());

    // Create the request to the target
    let mut request_builder = http::Request::builder()
        .method(method.clone())
        .uri(target_url.to_string());

    // Add headers
    for (name, value) in headers.iter() {
      request_builder = request_builder.header(name.as_str(), value);
    }

    // Inject API keys for allowed origins (same as VSCode extension)
    let origin_str = match url.origin() {
        url::Origin::Tuple(scheme, host, port) => {
            // Match VSCode extension behavior - don't include default ports
            match (scheme.as_str(), port) {
                ("http", 80) | ("https", 443) => format!("{}://{}", scheme, host),
                _ => format!("{}://{}:{}", scheme, host, port)
            }
        }
        url::Origin::Opaque(_) => {
            return Ok(BinaryResponse { body: Vec::new(), status: http::StatusCode::BAD_REQUEST });
        }
    };
    
    tracing::info!("[PROXY] Checking origin: {} against allowed origins", origin_str);
    
    for (allowed_origin, header_name, env_var, baml_header) in API_KEY_INJECTION_ALLOWED {
        if origin_str == *allowed_origin {
            tracing::info!("[PROXY] Origin {} matches allowed origin {}", origin_str, allowed_origin);
            
            // Try to get API key from environment variable first
            let api_key = if let Ok(key) = std::env::var(env_var) {
                tracing::info!("[PROXY] Using API key from environment variable {}", env_var);
                key
            } else if let Some(api_key_value) = headers.get(*baml_header) {
                if let Ok(key) = api_key_value.to_str() {
                    tracing::info!("[PROXY] Using API key from header {}", baml_header);
                    key.to_string()
                } else {
                    tracing::warn!("[PROXY] Invalid API key in header {}", baml_header);
                    continue;
                }
            } else {
                // Use dummy key as fallback for testing
                if let Some((_, dummy_key)) = DUMMY_API_KEYS.iter().find(|(key, _)| **key == **env_var) {
                    tracing::warn!("[PROXY] Using dummy API key for {} (env var: {}, header: {})", allowed_origin, env_var, baml_header);
                    dummy_key.to_string()
                } else {
                    tracing::warn!("[PROXY] No API key found for {} (tried env var {} and header {})", allowed_origin, env_var, baml_header);
                    continue;
                }
            };
            
            let header_value = if *header_name == "Authorization" {
                format!("Bearer {}", api_key)
            } else {
                api_key
            };
            request_builder = request_builder.header(*header_name, header_value);
            tracing::info!("[PROXY] Injected API key for {} (header: {})", allowed_origin, header_name);
        }
    }

    for (name, value) in request_builder.headers_ref().unwrap().iter() {
        tracing::info!("[PROXY] Outgoing header: {}: {:?}", name, value);
    }

    // Build the request
    let request = match request_builder.body(body.to_vec()) {
        Ok(req) => req,
        Err(_) => {
            return Ok(BinaryResponse { body: Vec::new(), status: http::StatusCode::INTERNAL_SERVER_ERROR });
        }
    };

    // Make the request using reqwest
    let client = reqwest::Client::new();
    let response = match client.execute(request.try_into().map_err(|_| warp::reject::custom(ProxyError))?).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("[PROXY ERROR] {} {}: {}", method, path_str, e);
            return Ok(BinaryResponse { body: Vec::new(), status: http::StatusCode::BAD_GATEWAY });
        }
    };

    // Build the response
    let status = response.status();
    let body_bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("[PROXY ERROR] Failed to read response body: {}", e);
            return Ok(BinaryResponse { body: Vec::new(), status: http::StatusCode::INTERNAL_SERVER_ERROR });
        }
    };

    tracing::info!("[PROXY] {} {} ← {}", method, path_str, status);
    tracing::info!("[PROXY] Upstream response status: {}", status);
    tracing::info!("[PROXY] Upstream response body: {}", String::from_utf8_lossy(&body_bytes));

    // Return response (CORS header will be added by warp middleware)
    Ok(BinaryResponse { body: body_bytes.to_vec(), status })
} 