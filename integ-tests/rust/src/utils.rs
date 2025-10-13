//! Test utilities for BAML Rust integration tests

use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tempfile;

/// Retry a function up to max_attempts times with exponential backoff
pub async fn retry_with_backoff<F, T, E>(mut f: F, max_attempts: usize) -> Result<T, E>
where
    F: FnMut() -> Pin<Box<dyn Future<Output = Result<T, E>>>>,
    E: std::fmt::Debug,
{
    let mut attempt = 1;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                if attempt >= max_attempts {
                    return Err(error);
                }

                let backoff_ms = 100 * (1 << (attempt - 1)); // Exponential backoff
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                attempt += 1;
            }
        }
    }
}

/// Assert that an eventual condition becomes true within a timeout
pub async fn assert_eventually<F>(
    mut condition: F,
    timeout: Duration,
    check_interval: Duration,
) -> Result<(), String>
where
    F: FnMut() -> bool,
{
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if condition() {
            return Ok(());
        }
        tokio::time::sleep(check_interval).await;
    }

    Err(format!("Condition not met within {:?}", timeout))
}

/// Helper to create temporary test files
pub fn create_temp_file_with_content(content: &[u8]) -> Result<tempfile::NamedTempFile> {
    use std::io::Write;

    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(content)?;
    temp_file.flush()?;
    Ok(temp_file)
}

/// Utility to compare JSON values ignoring order
pub fn json_values_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Object(a_obj), serde_json::Value::Object(b_obj)) => {
            if a_obj.len() != b_obj.len() {
                return false;
            }
            for (key, value) in a_obj {
                match b_obj.get(key) {
                    Some(b_value) => {
                        if !json_values_equal(value, b_value) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
        }
        (serde_json::Value::Array(a_arr), serde_json::Value::Array(b_arr)) => {
            if a_arr.len() != b_arr.len() {
                return false;
            }
            a_arr
                .iter()
                .zip(b_arr.iter())
                .all(|(a_item, b_item)| json_values_equal(a_item, b_item))
        }
        _ => a == b,
    }
}

/// Test data constants
pub mod test_data {
    /// Base64 encoded test image (1x1 pixel PNG)
    pub const TEST_IMAGE_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChAGAWCRYdwAAAABJRU5ErkJggg==";

    /// Test PDF content (minimal valid PDF)
    pub const TEST_PDF_BASE64: &str = "JVBERi0xLjEKJcKlwrHDqwoKMSAwIG9iago8PAovVHlwZSAvQ2F0YWxvZwovUGFnZXMgMiAwIFIKPj4KZW5kb2JqCgoyIDAgb2JqCjw8Ci9UeXBlIC9QYWdlcwovS2lkcyBbMyAwIFJdCi9Db3VudCAxCj4+CmVuZG9iagoKMyAwIG9iago8PAovVHlwZSAvUGFnZQovUGFyZW50IDIgMCBSCi9NZWRpYUJveCBbMCAwIDIxMiAyNzJdCj4+CmVuZG9iagoKeHJlZgowIDQKMDAwMDAwMDAwMCA2NTUzNSBmIAowMDAwMDAwMDA5IDAwMDAwIG4gCjAwMDAwMDAwNTggMDAwMDAgbiAKMDAwMDAwMDExNSAwMDAwMCBuIAp0cmFpbGVyCjw8Ci9TaXplIDQKL1Jvb3QgMSAwIFIKPj4Kc3RhcnR4cmVmCjE3NQolJUVPRg==";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_with_backoff_success() {
        let mut attempt_count = 0;
        let result = retry_with_backoff(
            || {
                attempt_count += 1;
                Box::pin(async move {
                    if attempt_count < 3 {
                        Err("Not ready")
                    } else {
                        Ok("Success")
                    }
                })
            },
            5,
        )
        .await;

        assert_eq!(result, Ok("Success"));
        assert_eq!(attempt_count, 3);
    }

    #[test]
    fn test_json_values_equal() {
        let a = serde_json::json!({"key": "value", "num": 42});
        let b = serde_json::json!({"num": 42, "key": "value"});

        assert!(json_values_equal(&a, &b));
    }
}
