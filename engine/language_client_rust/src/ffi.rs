//! FFI bindings to the shared BAML runtime library
//! 
//! This module provides Rust FFI bindings to the same `baml_cffi.dylib`
//! that Go, Python, and TypeScript use. This ensures we use the exact
//! same runtime logic across all languages.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::Once;
use libloading::Library;
use once_cell::sync::OnceCell;
use crate::{BamlResult, BamlError};

/// Global library handle - loaded once and shared across all clients
static LIBRARY: OnceCell<Library> = OnceCell::new();
static INIT_ONCE: Once = Once::new();

/// Function pointer types matching the C ABI from baml_cffi
type VersionFn = unsafe extern "C" fn() -> *const c_char;
type CreateBamlRuntimeFn = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *const c_void;
type DestroyBamlRuntimeFn = unsafe extern "C" fn(*const c_void);
type CallFunctionFromCFn = unsafe extern "C" fn(*const c_void, *const c_char, *const c_char, usize, u32) -> *const c_void;
type CallFunctionStreamFromCFn = unsafe extern "C" fn(*const c_void, *const c_char, *const c_char, usize, u32) -> *const c_void;

/// Callback function types for async operations
type CallbackFn = unsafe extern "C" fn(u32, i32, *const i8, i32);
type OnTickCallbackFn = unsafe extern "C" fn(u32);

/// FFI function wrappers - loaded dynamically from the shared library
struct BamlFfiFunctions {
    pub version: VersionFn,
    pub create_baml_runtime: CreateBamlRuntimeFn,
    pub destroy_baml_runtime: DestroyBamlRuntimeFn,
    pub call_function_from_c: CallFunctionFromCFn,
    pub call_function_stream_from_c: CallFunctionStreamFromCFn,
}

/// Global FFI functions - loaded once and cached
static FFI_FUNCTIONS: OnceCell<BamlFfiFunctions> = OnceCell::new();

/// Initialize the BAML FFI library
/// 
/// This loads the shared library and resolves all function symbols.
/// It's called automatically on first use, but can be called explicitly
/// to handle any initialization errors early.
pub fn init_library() -> BamlResult<()> {
    INIT_ONCE.call_once(|| {
        if let Err(e) = load_library() {
            eprintln!("Failed to load BAML library: {}", e);
            std::process::abort();
        }
    });
    Ok(())
}

fn load_library() -> BamlResult<()> {
    // Try to find the library in various locations
    let library_paths = get_library_search_paths();
    
    let library = library_paths.iter()
        .find_map(|path| {
            match unsafe { Library::new(path) } {
                Ok(lib) => Some(lib),
                Err(_) => None,
            }
        })
        .ok_or_else(|| BamlError::Configuration(format!(
            "Could not load BAML library. Searched paths: {:?}",
            library_paths
        )))?;

    // Load function symbols
    let ffi_functions = unsafe {
        BamlFfiFunctions {
            version: *library.get(b"version\0")
                .map_err(|e| BamlError::Configuration(format!("Failed to load 'version' symbol: {}", e)))?,
            create_baml_runtime: *library.get(b"create_baml_runtime\0")
                .map_err(|e| BamlError::Configuration(format!("Failed to load 'create_baml_runtime' symbol: {}", e)))?,
            destroy_baml_runtime: *library.get(b"destroy_baml_runtime\0")
                .map_err(|e| BamlError::Configuration(format!("Failed to load 'destroy_baml_runtime' symbol: {}", e)))?,
            call_function_from_c: *library.get(b"call_function_from_c\0")
                .map_err(|e| BamlError::Configuration(format!("Failed to load 'call_function_from_c' symbol: {}", e)))?,
            call_function_stream_from_c: *library.get(b"call_function_stream_from_c\0")
                .map_err(|e| BamlError::Configuration(format!("Failed to load 'call_function_stream_from_c' symbol: {}", e)))?,
        }
    };

    // Store the library handle to prevent it from being dropped
    LIBRARY.set(library).map_err(|_| BamlError::Configuration("Failed to store library handle".to_string()))?;
    
    // Store the function pointers
    FFI_FUNCTIONS.set(ffi_functions).map_err(|_| BamlError::Configuration("Failed to store FFI functions".to_string()))?;

    Ok(())
}

fn get_library_search_paths() -> Vec<String> {
    let mut paths = Vec::new();
    
    // 1. Environment variable
    if let Ok(path) = std::env::var("BAML_LIBRARY_PATH") {
        paths.push(path);
    }
    
    // 2. Current directory
    #[cfg(target_os = "macos")]
    paths.push("./libbaml_cffi.dylib".to_string());
    #[cfg(target_os = "linux")]
    paths.push("./libbaml_cffi.so".to_string());
    #[cfg(target_os = "windows")]
    paths.push("./baml_cffi.dll".to_string());
    
    // 3. Target directory (for development)
    let target_dir = std::env::current_dir()
        .ok()
        .and_then(|mut p| {
            // Navigate up to find target directory
            loop {
                p.push("target");
                p.push("debug");
                if p.exists() {
                    return Some(p);
                }
                p.pop();
                p.pop();
                if !p.pop() {
                    return None;
                }
            }
        });
        
    if let Some(mut target_path) = target_dir {
        #[cfg(target_os = "macos")]
        target_path.push("libbaml_cffi.dylib");
        #[cfg(target_os = "linux")]
        target_path.push("libbaml_cffi.so");
        #[cfg(target_os = "windows")]
        target_path.push("baml_cffi.dll");
        
        if let Some(path_str) = target_path.to_str() {
            paths.push(path_str.to_string());
        }
    }
    
    // 4. System library paths
    #[cfg(target_os = "macos")]
    {
        paths.push("/usr/local/lib/libbaml_cffi.dylib".to_string());
        paths.push("/opt/homebrew/lib/libbaml_cffi.dylib".to_string());
    }
    #[cfg(target_os = "linux")]
    {
        paths.push("/usr/local/lib/libbaml_cffi.so".to_string());
        paths.push("/usr/lib/libbaml_cffi.so".to_string());
    }
    
    paths
}

/// Get the version of the loaded BAML library
pub fn get_library_version() -> BamlResult<String> {
    init_library()?;
    
    let ffi = FFI_FUNCTIONS.get()
        .ok_or_else(|| BamlError::Configuration("FFI functions not initialized".to_string()))?;
    
    let version_ptr = unsafe { (ffi.version)() };
    if version_ptr.is_null() {
        return Err(BamlError::Configuration("Version function returned null".to_string()));
    }
    
    let version_cstr = unsafe { CStr::from_ptr(version_ptr) };
    let version_str = version_cstr.to_str()
        .map_err(|e| BamlError::Configuration(format!("Invalid UTF-8 in version string: {}", e)))?;
    
    Ok(version_str.to_string())
}

/// Create a BAML runtime instance
pub fn create_baml_runtime(
    root_path: &str,
    src_files_json: &str,
    env_vars_json: &str,
) -> BamlResult<*const c_void> {
    init_library()?;
    
    let ffi = FFI_FUNCTIONS.get()
        .ok_or_else(|| BamlError::Configuration("FFI functions not initialized".to_string()))?;
    
    let root_path_cstr = CString::new(root_path)
        .map_err(|e| BamlError::invalid_argument(format!("Invalid root_path: {}", e)))?;
    let src_files_cstr = CString::new(src_files_json)
        .map_err(|e| BamlError::invalid_argument(format!("Invalid src_files_json: {}", e)))?;
    let env_vars_cstr = CString::new(env_vars_json)
        .map_err(|e| BamlError::invalid_argument(format!("Invalid env_vars_json: {}", e)))?;
    
    let runtime_ptr = unsafe {
        (ffi.create_baml_runtime)(
            root_path_cstr.as_ptr(),
            src_files_cstr.as_ptr(),
            env_vars_cstr.as_ptr(),
        )
    };
    
    if runtime_ptr.is_null() {
        return Err(BamlError::Configuration("Failed to create BAML runtime".to_string()));
    }
    
    Ok(runtime_ptr)
}

/// Destroy a BAML runtime instance
pub fn destroy_baml_runtime(runtime_ptr: *const c_void) -> BamlResult<()> {
    if runtime_ptr.is_null() {
        return Ok(());
    }
    
    let ffi = FFI_FUNCTIONS.get()
        .ok_or_else(|| BamlError::Configuration("FFI functions not initialized".to_string()))?;
    
    unsafe {
        (ffi.destroy_baml_runtime)(runtime_ptr);
    }
    
    Ok(())
}

/// Call a BAML function (async, returns immediately with callback)
pub fn call_function_from_c(
    runtime_ptr: *const c_void,
    function_name: &str,
    encoded_args: &str,
    id: u32,
) -> BamlResult<*const c_void> {
    if runtime_ptr.is_null() {
        return Err(BamlError::invalid_argument("Runtime pointer is null"));
    }
    
    let ffi = FFI_FUNCTIONS.get()
        .ok_or_else(|| BamlError::Configuration("FFI functions not initialized".to_string()))?;
    
    let function_name_cstr = CString::new(function_name)
        .map_err(|e| BamlError::invalid_argument(format!("Invalid function_name: {}", e)))?;
    let encoded_args_cstr = CString::new(encoded_args)
        .map_err(|e| BamlError::invalid_argument(format!("Invalid encoded_args: {}", e)))?;
    
    let result_ptr = unsafe {
        (ffi.call_function_from_c)(
            runtime_ptr,
            function_name_cstr.as_ptr(),
            encoded_args_cstr.as_ptr(),
            encoded_args.len(),
            id,
        )
    };
    
    // Note: result_ptr being null is not necessarily an error for async calls
    // The result will come via callback
    Ok(result_ptr)
}

/// Call a BAML function with streaming (async, returns immediately with callback)
pub fn call_function_stream_from_c(
    runtime_ptr: *const c_void,
    function_name: &str,
    encoded_args: &str,
    id: u32,
) -> BamlResult<*const c_void> {
    if runtime_ptr.is_null() {
        return Err(BamlError::invalid_argument("Runtime pointer is null"));
    }
    
    let ffi = FFI_FUNCTIONS.get()
        .ok_or_else(|| BamlError::Configuration("FFI functions not initialized".to_string()))?;
    
    let function_name_cstr = CString::new(function_name)
        .map_err(|e| BamlError::invalid_argument(format!("Invalid function_name: {}", e)))?;
    let encoded_args_cstr = CString::new(encoded_args)
        .map_err(|e| BamlError::invalid_argument(format!("Invalid encoded_args: {}", e)))?;
    
    let result_ptr = unsafe {
        (ffi.call_function_stream_from_c)(
            runtime_ptr,
            function_name_cstr.as_ptr(),
            encoded_args_cstr.as_ptr(),
            encoded_args.len(),
            id,
        )
    };
    
    Ok(result_ptr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_loading() {
        // This will only pass if the library is available
        match init_library() {
            Ok(_) => {
                // Test version function
                match get_library_version() {
                    Ok(version) => println!("BAML library version: {}", version),
                    Err(e) => println!("Could not get version: {}", e),
                }
            }
            Err(e) => {
                println!("Library not available for testing: {}", e);
                // This is expected in most test environments
            }
        }
    }
}