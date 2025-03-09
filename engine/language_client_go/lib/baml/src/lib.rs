use std::{ffi::CStr, path::Path};

extern crate baml_runtime;
use baml_runtime::{BamlRuntime, FunctionResult, FunctionResultStream};

#[no_mangle]
pub extern "C" fn hello(name: *const libc::c_char) {
    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name = name_cstr.to_str().unwrap();
    println!("Hello {}!", name);
}

#[no_mangle]
pub extern "C" fn whisper(message: *const libc::c_char) {
    let message_cstr = unsafe { CStr::from_ptr(message) };
    let message = message_cstr.to_str().unwrap();
    println!("({})", message);
}

#[no_mangle]
pub extern "C" fn create_baml_runtime() -> *const libc::c_void {
    const BAML_DIR: &str = "/Users/vbv/repos/gloo-lang/integ-tests/baml_src";
    let env_vars = std::env::vars()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let runtime = BamlRuntime::from_directory(&Path::new(BAML_DIR), env_vars);
    Box::into_raw(Box::new(runtime)) as *const libc::c_void
}

#[no_mangle]
pub extern "C" fn destroy_baml_runtime(runtime: *const libc::c_void) {
    unsafe {
        let _ = Box::from_raw(runtime as *mut BamlRuntime);
    }
}

#[no_mangle]
pub extern "C" fn invoke_runtime_cli(args: *const *const libc::c_char) {
    // Safety: We assume `args` is a valid pointer to a null-terminated array of C strings.
    let args_vec = unsafe {
        // Ensure the pointer itself is not null.
        if args.is_null() {
            Vec::new()
        } else {
            let mut vec = Vec::new();
            let mut i = 0;
            // Iterate until a null pointer is encountered.
            while !(*args.add(i)).is_null() {
                let c_str = CStr::from_ptr(*args.add(i));
                // Convert to Rust String (lossy conversion handles non-UTF8 gracefully).
                vec.push(c_str.to_string_lossy().into_owned());
                i += 1;
            }
            vec
        }
    };
    baml_cli::run_cli(
        args_vec,
        baml_runtime::RuntimeCliDefaults {
            output_type: baml_types::GeneratorOutputType::PythonPydantic,
        },
    )
    .unwrap();
}

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use baml_types::{BamlMap, BamlValue};

#[repr(C)]
pub struct CKwargs {
    pub len: libc::size_t,
    pub keys: *const *const c_char,
    pub values: *const *const c_char,
}

/// Convert CKwargs to a BamlMap<String, BamlValue>
unsafe fn ckwargs_to_map(kwargs: *const CKwargs) -> BamlMap<String, BamlValue> {
    let mut map = BamlMap::new();
    if kwargs.is_null() {
        return map;
    }
    let kwargs_ref = &*kwargs;
    for i in 0..(kwargs_ref.len as isize) {
        let key_ptr = *kwargs_ref.keys.offset(i);
        let value_ptr = *kwargs_ref.values.offset(i);
        if let (Ok(key), Ok(value)) = (
            CStr::from_ptr(key_ptr).to_str(),
            serde_json::from_str::<BamlValue>(CStr::from_ptr(value_ptr).to_str().unwrap()),
        ) {
            map.insert(key.to_owned(), value.to_owned());
        }
    }
    map
}

static mut CALLBACK_FN: Option<extern "C" fn(u32, bool, *const c_char)> = None;

#[no_mangle]
extern "C" fn register_callback(callback_fn: *const libc::c_void) {
    // Create a global runtime or pass it along as needed.
    let _rt = tokio::runtime::Runtime::new().unwrap();
    // Store _rt somewhere accessible if needed.
    unsafe {
        RUNTIME = Some(std::sync::Arc::new(_rt));
        CALLBACK_FN = Some(std::mem::transmute(callback_fn));
    }
}

fn safe_trigger_callback(id: u32, is_done: bool, message: &str) {
    let callback_fn = unsafe { CALLBACK_FN.unwrap() };
    let c_message = CString::new(message).unwrap();
    callback_fn(id, is_done, c_message.as_ptr() as *const libc::c_char);
}

static mut RUNTIME: Option<std::sync::Arc<tokio::runtime::Runtime>> = None;

/// Extern "C" function that returns immediately, scheduling the async call.
/// Once the asynchronous function completes, the provided callback is invoked.
#[no_mangle]
pub extern "C" fn call_function_from_c(
    runtime: *const libc::c_void,
    function_name: *const c_char,
    kwargs: *const CKwargs,
    id: u32,
) {
    // Safety: assume that the pointers provided are valid.
    let runtime = unsafe { &*(runtime as *const BamlRuntime) };

    // Convert the function name.
    let func_name = match unsafe { CStr::from_ptr(function_name) }.to_str() {
        Ok(s) => s.to_owned(),
        Err(_) => {
            safe_trigger_callback(id, true, "Failed to convert function name to string");
            return;
        }
    };

    // Convert keyword arguments.
    let keyword_args = unsafe { ckwargs_to_map(kwargs) };

    let ctx = runtime.create_ctx_manager(BamlValue::String("cffi".to_string()), None);

    // Spawn an async task to await the future and call the callback when done.
    // Ensure that a Tokio runtime is running in your application.
    let rt = unsafe { RUNTIME.as_ref().unwrap() };
    rt.spawn(async move {
        let future = runtime.call_function(func_name, &keyword_args, &ctx, None, None);
        let (result, _) = future.await;
        match result {
            Ok(result) => match result.content() {
                Ok(content) => safe_trigger_callback(id, true, &content),
                Err(e) => safe_trigger_callback(id, true, &e.to_string()),
            },
            Err(e) => safe_trigger_callback(id, true, &e.to_string()),
        };
        
        // Note: Responsibility for freeing the returned string lies with the caller.
    });
}

/// Extern "C" function that returns immediately, scheduling the async call.
/// Once the asynchronous function completes, the provided callback is invoked.
#[no_mangle]
pub extern "C" fn call_function_stream_from_c(
    runtime: *const libc::c_void,
    function_name: *const c_char,
    kwargs: *const CKwargs,
    id: u32,
) {
    // Safety: assume that the pointers provided are valid.
    let runtime = unsafe { &*(runtime as *const BamlRuntime) };

    // Convert the function name.
    let func_name = match unsafe { CStr::from_ptr(function_name) }.to_str() {
        Ok(s) => s.to_owned(),
        Err(_) => {
            panic!("Failed to convert function name to string");
        }
    };

    // Convert keyword arguments.
    let keyword_args = unsafe { ckwargs_to_map(kwargs) };

    let ctx = runtime.create_ctx_manager(BamlValue::String("cffi".to_string()), None);
    let mut stream = match runtime.stream_function(func_name, &keyword_args, &ctx, None, None) {
        Ok(stream) => stream,
        Err(e) => {
            safe_trigger_callback(id, true, &e.to_string());
            return;
        }
    };

    let ctx = runtime.create_ctx_manager(BamlValue::String("cffi".to_string()), None);

    let rt = unsafe { RUNTIME.as_ref().unwrap() };

    rt.spawn(async move {
        let (result, _) = stream.run(Some(|r| on_event(id, r)), &ctx, None, None).await;
        match result {
            Ok(result) => match result.content() {
                Ok(content) => safe_trigger_callback(id, true, &content),
                Err(e) => safe_trigger_callback(id, true, &e.to_string()),
            },
            Err(e) => safe_trigger_callback(id, true, &e.to_string()),
        };
    });
}

pub fn on_event(id: u32, result: FunctionResult) {
    match result.content() {
        Ok(content) => safe_trigger_callback(id, false, &content),
        Err(e) => safe_trigger_callback(id, false, &e.to_string()),
    }
}

// This is present so it's easy to test that the code works natively in Rust via `cargo test`
#[cfg(test)]
pub mod test {

    use super::*;
    use std::ffi::CString;

    // This is meant to do the same stuff as the main function in the .go files
    #[test]
    fn simulated_main_function() {
        hello(CString::new("world").unwrap().into_raw());
        whisper(CString::new("this is code from Rust").unwrap().into_raw());
    }
}
