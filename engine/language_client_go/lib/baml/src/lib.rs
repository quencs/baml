use std::{ffi::CStr, path::Path};

extern crate baml_runtime;
use baml_runtime::BamlRuntime;

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
    let env_vars = std::env::vars().into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
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

/// Type for the callback function.
/// The callback receives a pointer to a C string containing the JSON result.
pub type ResultCallback = extern "C" fn(result: *const c_char);

/// Extern "C" function that returns immediately, scheduling the async call.
/// Once the asynchronous function completes, the provided callback is invoked.
#[no_mangle]
pub extern "C" fn call_function_from_c(
    runtime: *const libc::c_void,
    function_name: *const c_char,
    kwargs: *const CKwargs,
    callback: ResultCallback,
) {
    // Safety: assume that the pointers provided are valid.
    let runtime = unsafe { &*(runtime as *const BamlRuntime) };

    // Convert the function name.
    let func_name = match unsafe { CStr::from_ptr(function_name) }.to_str() {
        Ok(s) => s.to_owned(),
        Err(_) => {
            callback(ptr::null());
            return;
        }
    };

    // Convert keyword arguments.
    let keyword_args = unsafe { ckwargs_to_map(kwargs) };

    let ctx = runtime.create_ctx_manager(BamlValue::String("cffi".to_string()), None);

    // Spawn an async task to await the future and call the callback when done.
    // Ensure that a Tokio runtime is running in your application.
    tokio::spawn(async move {
        let future = runtime.call_function(func_name, &keyword_args, &ctx, None, None);
        let (result, _) = future.await;
        let result_str = match result {
            Ok(result) => result.to_string(),
            Err(_) => String::new(),
        };
        let c_result = CString::new(result_str).unwrap();
        callback(c_result.into_raw());
        // Note: Responsibility for freeing the returned string lies with the caller.
    });
}


// This is present so it's easy to test that the code works natively in Rust via `cargo test`
#[cfg(test)]
pub mod test {

    use std::ffi::CString;
    use super::*;

    // This is meant to do the same stuff as the main function in the .go files
    #[test]
    fn simulated_main_function () {
        hello(CString::new("world").unwrap().into_raw());
        whisper(CString::new("this is code from Rust").unwrap().into_raw());
    }
}

// In your Rust code that becomes libhello.dylib
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Using a static HashMap to store multiple callbacks with an ID
static CALLBACKS: Lazy<Mutex<HashMap<u32, extern "C" fn(*const libc::c_char)>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

#[no_mangle]
pub extern "C" fn register_callback(id: u32, callback: extern "C" fn(*const libc::c_char)) -> bool {
    let mut callbacks = CALLBACKS.lock().unwrap();
    callbacks.insert(id, callback);
    true
}

#[no_mangle]
pub extern "C" fn unregister_callback(id: u32) -> bool {
    let mut callbacks = CALLBACKS.lock().unwrap();
    callbacks.remove(&id).is_some()
}

#[no_mangle]
pub extern "C" fn trigger_callback(id: u32, message: *const libc::c_char) -> bool {
    let callbacks = CALLBACKS.lock().unwrap();
    if let Some(callback) = callbacks.get(&id) {
        unsafe {
            callback(message);
        }
        true
    } else {
        false
    }
}