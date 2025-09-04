use baml_cffi as ffi;

use crate::{
    // ffi,
    types::{BamlValue, FromBamlValue},
    BamlContext,
    BamlError,
    BamlResult,
    FunctionResult,
    StreamState,
};
use futures::{Stream, StreamExt};
use serde_json;
use std::collections::HashMap;
use std::os::raw::c_void;
use std::path::Path;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use std::task::{Context as TaskContext, Poll};
use tokio::sync::{mpsc as async_mpsc, oneshot};

/// High-level BAML client for executing functions
#[derive(Clone, Debug)]
pub struct BamlClient {
    runtime_ptr: *const c_void,
    callback_manager: Arc<CallbackManager>,
}

// Ensure BamlClient is Send + Sync
unsafe impl Send for BamlClient {}
unsafe impl Sync for BamlClient {}

/// Manages async callbacks from the BAML runtime
#[derive(Default, Debug)]
struct CallbackManager {
    next_id: AtomicU32,
    pending_calls: Arc<Mutex<HashMap<u32, oneshot::Sender<CallbackResult>>>>,
    pending_streams: Arc<Mutex<HashMap<u32, async_mpsc::UnboundedSender<StreamEvent>>>>,
}

#[derive(Debug, Clone)]
struct CallbackResult {
    success: bool,
    data: String,
}

#[derive(Debug, Clone)]
struct StreamEvent {
    is_final: bool,
    data: String,
    success: bool,
}

impl CallbackManager {
    fn new() -> Self {
        Self {
            next_id: AtomicU32::new(1),
            pending_calls: Arc::new(Mutex::new(HashMap::new())),
            pending_streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_next_id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    fn register_call(&self, id: u32) -> oneshot::Receiver<CallbackResult> {
        let (tx, rx) = oneshot::channel();
        self.pending_calls.lock().unwrap().insert(id, tx);
        rx
    }

    fn register_stream(&self, id: u32) -> async_mpsc::UnboundedReceiver<StreamEvent> {
        let (tx, rx) = async_mpsc::unbounded_channel();
        self.pending_streams.lock().unwrap().insert(id, tx);
        rx
    }

    fn handle_callback(&self, id: u32, success: bool, data: String) {
        if let Some(sender) = self.pending_calls.lock().unwrap().remove(&id) {
            let _ = sender.send(CallbackResult { success, data });
        }
    }

    fn handle_stream_event(&self, id: u32, is_final: bool, success: bool, data: String) {
        if let Some(sender) = self.pending_streams.lock().unwrap().get(&id) {
            let _ = sender.send(StreamEvent {
                is_final,
                data,
                success,
            });

            if is_final {
                // Remove the sender after final event
                self.pending_streams.lock().unwrap().remove(&id);
            }
        }
    }
}

impl BamlClient {
    /// Create a new BAML client from environment variables
    ///
    /// This will look for BAML configuration in environment variables
    /// and initialize the runtime accordingly.
    pub fn from_env() -> BamlResult<Self> {
        let env_vars: HashMap<String, String> = std::env::vars().collect();
        Self::from_file_content(".", &HashMap::new(), env_vars)
    }

    /// Create a new BAML client from a directory containing BAML source files
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_directory<P: AsRef<Path>>(
        path: P,
        env_vars: HashMap<String, String>,
    ) -> BamlResult<Self> {
        // Read all .baml files from the directory
        use std::fs;

        let mut files = HashMap::new();
        let dir_path = path.as_ref();

        fn read_baml_files(
            dir: &Path,
            files: &mut HashMap<String, String>,
            base_path: &Path,
        ) -> BamlResult<()> {
            let entries = fs::read_dir(dir).map_err(|e| {
                BamlError::Configuration(format!("Failed to read directory {:?}: {}", dir, e))
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    BamlError::Configuration(format!("Failed to read directory entry: {}", e))
                })?;
                let path = entry.path();

                if path.is_dir() {
                    read_baml_files(&path, files, base_path)?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("baml") {
                    let content = fs::read_to_string(&path).map_err(|e| {
                        BamlError::Configuration(format!("Failed to read file {:?}: {}", path, e))
                    })?;
                    let relative_path = path.strip_prefix(base_path).map_err(|e| {
                        BamlError::Configuration(format!(
                            "Failed to get relative path for {:?}: {}",
                            path, e
                        ))
                    })?;
                    files.insert(relative_path.to_string_lossy().to_string(), content);
                }
            }
            Ok(())
        }

        read_baml_files(dir_path, &mut files, dir_path)?;

        Self::from_file_content(dir_path.to_string_lossy().as_ref(), &files, env_vars)
    }

    /// Create a new BAML client from file contents
    pub fn from_file_content(
        root_path: &str,
        files: &HashMap<String, String>,
        env_vars: HashMap<String, String>,
    ) -> BamlResult<Self> {
        // Serialize files and env_vars to JSON for the C FFI
        let src_files_json = serde_json::to_string(files).map_err(|e| {
            BamlError::invalid_argument(format!("Failed to serialize files: {}", e))
        })?;
        let env_vars_json = serde_json::to_string(&env_vars).map_err(|e| {
            BamlError::invalid_argument(format!("Failed to serialize env_vars: {}", e))
        })?;

        // Create the BAML runtime via FFI
        // Convert strings to C strings
        let root_path_c = std::ffi::CString::new(root_path)
            .map_err(|e| BamlError::invalid_argument(format!("Invalid root_path: {}", e)))?;
        let src_files_json_c = std::ffi::CString::new(src_files_json)
            .map_err(|e| BamlError::invalid_argument(format!("Invalid src_files_json: {}", e)))?;
        let env_vars_json_c = std::ffi::CString::new(env_vars_json)
            .map_err(|e| BamlError::invalid_argument(format!("Invalid env_vars_json: {}", e)))?;
        
        let runtime_ptr = ffi::create_baml_runtime(
            root_path_c.as_ptr(),
            src_files_json_c.as_ptr(),
            env_vars_json_c.as_ptr(),
        );
        
        // Check if runtime creation failed (null pointer indicates error)
        if runtime_ptr.is_null() {
            return Err(BamlError::Runtime(anyhow::anyhow!("Failed to create BAML runtime")));
        }

        let callback_manager = Arc::new(CallbackManager::new());

        // TODO: Register global callbacks with the FFI interface
        // This would require exposing callback registration in the FFI

        Ok(Self {
            runtime_ptr,
            callback_manager,
        })
    }

    /// Create a new BAML client with a pre-configured runtime pointer
    ///
    /// This is primarily for internal use where you already have a runtime pointer
    /// from the FFI interface.
    pub fn with_runtime_ptr(runtime_ptr: *const c_void) -> BamlResult<Self> {
        let callback_manager = Arc::new(CallbackManager::new());

        Ok(Self {
            runtime_ptr,
            callback_manager,
        })
    }

    /// Call a BAML function asynchronously
    pub async fn call_function<T>(&self, function_name: &str, context: BamlContext) -> BamlResult<T>
    where
        T: FromBamlValue,
    {
        let result = self.call_function_raw(function_name, context).await?;
        T::from_baml_value(result.data)
    }

    /// Call a BAML function and return the raw result
    pub async fn call_function_raw(
        &self,
        function_name: &str,
        context: BamlContext,
    ) -> BamlResult<FunctionResult> {
        // Serialize the arguments to JSON for the C FFI
        let encoded_args = serde_json::to_string(&context.args).map_err(|e| {
            BamlError::invalid_argument(format!("Failed to serialize arguments: {}", e))
        })?;

        // Get a unique ID for this call
        let call_id = self.callback_manager.get_next_id();

        // Register for the callback
        let callback_receiver = self.callback_manager.register_call(call_id);

        // Make the FFI call
        // Convert strings to C strings
        let function_name_c = std::ffi::CString::new(function_name)
            .map_err(|e| BamlError::invalid_argument(format!("Invalid function_name: {}", e)))?;
        let encoded_args_c = std::ffi::CString::new(encoded_args.clone())
            .map_err(|e| BamlError::invalid_argument(format!("Invalid encoded_args: {}", e)))?;
        
        let result_ptr = ffi::call_function_from_c(
            self.runtime_ptr,
            function_name_c.as_ptr(),
            encoded_args_c.as_ptr(),
            encoded_args.len(),
            call_id as u32,
        );
        
        // Check if the call failed (non-null pointer indicates error)
        if !result_ptr.is_null() {
            return Err(BamlError::Runtime(anyhow::anyhow!("FFI function call failed")));
        }

        // Wait for the callback result
        let callback_result = callback_receiver
            .await
            .map_err(|_| BamlError::Runtime(anyhow::anyhow!("Callback channel closed")))?;

        if callback_result.success {
            // Parse the JSON response into a BamlValue
            let baml_value: BamlValue =
                serde_json::from_str(&callback_result.data).map_err(|e| {
                    BamlError::deserialization(format!("Failed to parse result: {}", e))
                })?;

            Ok(FunctionResult::new(baml_value, call_id.to_string()))
        } else {
            Err(BamlError::Runtime(anyhow::anyhow!(
                "Function call failed: {}",
                callback_result.data
            )))
        }
    }

    /// Call a BAML function with streaming support
    pub async fn call_function_stream<T>(
        &self,
        function_name: &str,
        context: BamlContext,
    ) -> BamlResult<impl futures::Stream<Item = BamlResult<StreamState<T>>>>
    where
        T: FromBamlValue + Send + Sync + 'static,
    {
        let stream = self
            .call_function_stream_raw(function_name, context)
            .await?;
        Ok(stream.map(|result| match result {
            Ok(stream_state) => match stream_state {
                StreamState::Partial(value) => T::from_baml_value(value).map(StreamState::Partial),
                StreamState::Final(value) => T::from_baml_value(value).map(StreamState::Final),
            },
            Err(e) => Err(e),
        }))
    }

    /// Call a BAML function with streaming support, returning raw results
    pub async fn call_function_stream_raw(
        &self,
        function_name: &str,
        context: BamlContext,
    ) -> BamlResult<BamlStream> {
        // Serialize the arguments to JSON for the C FFI
        let encoded_args = serde_json::to_string(&context.args).map_err(|e| {
            BamlError::invalid_argument(format!("Failed to serialize arguments: {}", e))
        })?;

        // Get a unique ID for this call
        let call_id = self.callback_manager.get_next_id();

        // Register for stream events
        let stream_receiver = self.callback_manager.register_stream(call_id);

        // Make the FFI call
        // Convert strings to C strings
        let function_name_c = std::ffi::CString::new(function_name)
            .map_err(|e| BamlError::invalid_argument(format!("Invalid function_name: {}", e)))?;
        let encoded_args_c = std::ffi::CString::new(encoded_args.clone())
            .map_err(|e| BamlError::invalid_argument(format!("Invalid encoded_args: {}", e)))?;
        
        let result_ptr = ffi::call_function_stream_from_c(
            self.runtime_ptr,
            function_name_c.as_ptr(),
            encoded_args_c.as_ptr(),
            encoded_args.len(),
            call_id as u32,
        );
        
        // Check if the call failed (non-null pointer indicates error)
        if !result_ptr.is_null() {
            return Err(BamlError::Runtime(anyhow::anyhow!("FFI streaming function call failed")));
        }

        Ok(BamlStream::new(stream_receiver))
    }

    /// Get the runtime pointer (for advanced use cases)
    pub fn runtime_ptr(&self) -> *const c_void {
        self.runtime_ptr
    }
}

impl Drop for BamlClient {
    fn drop(&mut self) {
        // Clean up the runtime pointer when the client is dropped
        if !self.runtime_ptr.is_null() {
            let _ = ffi::destroy_baml_runtime(self.runtime_ptr);
        }
    }
}

/// Stream wrapper for BAML function streaming results
pub struct BamlStream {
    receiver: async_mpsc::UnboundedReceiver<StreamEvent>,
}

impl BamlStream {
    fn new(receiver: async_mpsc::UnboundedReceiver<StreamEvent>) -> Self {
        Self { receiver }
    }
}

impl Stream for BamlStream {
    type Item = BamlResult<StreamState<BamlValue>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(event)) => {
                if event.success {
                    // Parse the JSON response into a BamlValue
                    match serde_json::from_str::<BamlValue>(&event.data) {
                        Ok(baml_value) => {
                            let stream_state = if event.is_final {
                                StreamState::Final(baml_value)
                            } else {
                                StreamState::Partial(baml_value)
                            };
                            Poll::Ready(Some(Ok(stream_state)))
                        }
                        Err(e) => Poll::Ready(Some(Err(BamlError::deserialization(format!(
                            "Failed to parse stream event: {}",
                            e
                        ))))),
                    }
                } else {
                    Poll::Ready(Some(Err(BamlError::Runtime(anyhow::anyhow!(
                        "Stream event failed: {}",
                        event.data
                    )))))
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

// BamlStream is our implementation of function result streaming

/// Builder for creating BAML clients with custom configuration
#[derive(Default)]
pub struct BamlClientBuilder {
    env_vars: HashMap<String, String>,
    root_path: Option<String>,
    files: HashMap<String, String>,
    directory: Option<std::path::PathBuf>,
}

impl BamlClientBuilder {
    /// Create a new client builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an environment variable
    pub fn env_var<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables
    pub fn env_vars<I, K, V>(mut self, env_vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (key, value) in env_vars {
            self.env_vars.insert(key.into(), value.into());
        }
        self
    }

    /// Set the root path for file content loading
    pub fn root_path<S: Into<String>>(mut self, path: S) -> Self {
        self.root_path = Some(path.into());
        self
    }

    /// Add a file with content
    pub fn file<K: Into<String>, V: Into<String>>(mut self, path: K, content: V) -> Self {
        self.files.insert(path.into(), content.into());
        self
    }

    /// Set multiple files
    pub fn files<I, K, V>(mut self, files: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (path, content) in files {
            self.files.insert(path.into(), content.into());
        }
        self
    }

    /// Set directory to load BAML files from
    #[cfg(not(target_arch = "wasm32"))]
    pub fn directory<P: Into<std::path::PathBuf>>(mut self, path: P) -> Self {
        self.directory = Some(path.into());
        self
    }

    /// Build the client
    pub fn build(mut self) -> BamlResult<BamlClient> {
        // Add environment variables if none explicitly set
        if self.env_vars.is_empty() {
            self.env_vars = std::env::vars().collect();
        }

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(directory) = self.directory {
            return BamlClient::from_directory(directory, self.env_vars);
        }

        if !self.files.is_empty() {
            let root_path = self.root_path.unwrap_or_else(|| ".".to_string());
            return BamlClient::from_file_content(&root_path, &self.files, self.env_vars);
        }

        BamlClient::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let builder = BamlClientBuilder::new()
            .env_var("TEST_VAR", "test_value")
            .root_path("/tmp");

        // We can't actually build without valid BAML files, but we can test the builder construction
        assert_eq!(
            builder.env_vars.get("TEST_VAR"),
            Some(&"test_value".to_string())
        );
    }
}
