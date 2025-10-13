//! Dynamic loader for the shared C FFI surface exposed by `baml_cffi`.
//! At runtime we discover and `dlopen` the `cdylib`, mirroring the approach
//! used by the Go language client to keep ABI validation consistent.

#![cfg(any(target_os = "macos", target_os = "linux"))]

use core::ffi::c_void;
use std::{
    collections::HashSet,
    env,
    ffi::CStr,
    os::raw::c_char,
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{anyhow, Context};
use libloading::Library;
use once_cell::sync::OnceCell;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const LIB_BASE_NAME: &str = "libbaml_cffi";
const LIB_EXT: &str = if cfg!(target_os = "macos") {
    "dylib"
} else {
    "so"
};
const LIBRARY_PATH_ENV: &str = "BAML_LIBRARY_PATH";

static SHARED_LIBRARY_OVERRIDE: OnceCell<Mutex<Option<PathBuf>>> = OnceCell::new();

/// Raw buffer returned across the FFI boundary.
#[repr(C)]
pub struct Buffer {
    pub ptr: *const c_char,
    pub len: usize,
}

/// Callback signature for FFI result/error notifications.
pub type CallbackFn =
    extern "C" fn(call_id: u32, is_done: i32, content: *const c_char, length: usize);

/// Callback signature for tick notifications during streaming calls.
pub type OnTickCallbackFn = extern "C" fn(call_id: u32);

struct BamlLibrary {
    _library: Library,
    register_callbacks: unsafe extern "C" fn(CallbackFn, CallbackFn, OnTickCallbackFn),
    create_baml_runtime:
        unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *const c_void,
    destroy_baml_runtime: unsafe extern "C" fn(*const c_void),
    call_function_from_c: unsafe extern "C" fn(
        *const c_void,
        *const c_char,
        *const c_char,
        usize,
        u32,
    ) -> *const c_void,
    call_function_parse_from_c: unsafe extern "C" fn(
        *const c_void,
        *const c_char,
        *const c_char,
        usize,
        u32,
    ) -> *const c_void,
    call_function_stream_from_c: unsafe extern "C" fn(
        *const c_void,
        *const c_char,
        *const c_char,
        usize,
        u32,
    ) -> *const c_void,
    call_object_constructor: unsafe extern "C" fn(*const c_char, usize) -> Buffer,
    call_object_method: unsafe extern "C" fn(*const c_void, *const c_char, usize) -> Buffer,
    free_buffer: unsafe extern "C" fn(Buffer),
    invoke_runtime_cli: unsafe extern "C" fn(*const *const c_char) -> i32,
    version: unsafe extern "C" fn() -> *const c_char,
}

impl BamlLibrary {
    fn load() -> anyhow::Result<Self> {
        let library_path =
            resolve_library_path().context("failed to resolve baml_cffi library location")?;

        let library = unsafe { Library::new(&library_path) }
            .with_context(|| format!("failed to load {}", library_path.display()))?;

        unsafe {
            let register_callbacks =
                *library.get::<unsafe extern "C" fn(CallbackFn, CallbackFn, OnTickCallbackFn)>(
                    b"register_callbacks\0",
                )?;
            let create_baml_runtime =
                *library.get::<unsafe extern "C" fn(
                    *const c_char,
                    *const c_char,
                    *const c_char,
                ) -> *const c_void>(b"create_baml_runtime\0")?;
            let destroy_baml_runtime =
                *library.get::<unsafe extern "C" fn(*const c_void)>(b"destroy_baml_runtime\0")?;
            let call_function_from_c =
                *library.get::<unsafe extern "C" fn(
                    *const c_void,
                    *const c_char,
                    *const c_char,
                    usize,
                    u32,
                ) -> *const c_void>(b"call_function_from_c\0")?;
            let call_function_parse_from_c =
                *library.get::<unsafe extern "C" fn(
                    *const c_void,
                    *const c_char,
                    *const c_char,
                    usize,
                    u32,
                ) -> *const c_void>(b"call_function_parse_from_c\0")?;
            let call_function_stream_from_c =
                *library.get::<unsafe extern "C" fn(
                    *const c_void,
                    *const c_char,
                    *const c_char,
                    usize,
                    u32,
                ) -> *const c_void>(b"call_function_stream_from_c\0")?;
            let call_object_constructor =
                *library.get::<unsafe extern "C" fn(*const c_char, usize) -> Buffer>(
                    b"call_object_constructor\0",
                )?;
            let call_object_method =
                *library
                    .get::<unsafe extern "C" fn(*const c_void, *const c_char, usize) -> Buffer>(
                        b"call_object_method\0",
                    )?;
            let free_buffer = *library.get::<unsafe extern "C" fn(Buffer)>(b"free_buffer\0")?;
            let invoke_runtime_cli = *library
                .get::<unsafe extern "C" fn(*const *const c_char) -> i32>(
                    b"invoke_runtime_cli\0",
                )?;
            let version = *library.get::<unsafe extern "C" fn() -> *const c_char>(b"version\0")?;

            let lib = Self {
                _library: library,
                register_callbacks,
                create_baml_runtime,
                destroy_baml_runtime,
                call_function_from_c,
                call_function_parse_from_c,
                call_function_stream_from_c,
                call_object_constructor,
                call_object_method,
                free_buffer,
                invoke_runtime_cli,
                version,
            };

            lib.ensure_version_matches()?;
            Ok(lib)
        }
    }

    fn ensure_version_matches(&self) -> anyhow::Result<()> {
        let version_ptr = unsafe { (self.version)() };
        if version_ptr.is_null() {
            return Err(anyhow!("version pointer returned null"));
        }
        let version = unsafe { CStr::from_ptr(version_ptr) }
            .to_str()
            .context("version string was not valid UTF-8")?;
        if version != VERSION {
            return Err(anyhow!(
                "version mismatch: Rust client expects {VERSION}, shared library reports {version}"
            ));
        }
        Ok(())
    }
}

static LIBRARY: OnceCell<Mutex<BamlLibrary>> = OnceCell::new();

fn with_library<F, R>(callback: F) -> R
where
    F: FnOnce(&BamlLibrary) -> R,
{
    let lock = LIBRARY.get_or_init(|| {
        let library = BamlLibrary::load().unwrap_or_else(|err| {
            panic!("Failed to load baml_cffi cdylib: {err:#}");
        });
        Mutex::new(library)
    });

    let guard = lock.lock().expect("baml_cffi loader mutex poisoned");
    callback(&guard)
}

pub fn register_callbacks(
    callback_fn: CallbackFn,
    error_callback_fn: CallbackFn,
    on_tick_callback_fn: OnTickCallbackFn,
) {
    let func = with_library(|lib| lib.register_callbacks);
    unsafe { func(callback_fn, error_callback_fn, on_tick_callback_fn) }
}

pub fn create_baml_runtime(
    root_path: *const c_char,
    src_files_json: *const c_char,
    env_vars_json: *const c_char,
) -> *const c_void {
    let func = with_library(|lib| lib.create_baml_runtime);
    unsafe { func(root_path, src_files_json, env_vars_json) }
}

pub fn destroy_baml_runtime(runtime: *const c_void) {
    let func = with_library(|lib| lib.destroy_baml_runtime);
    unsafe { func(runtime) }
}

pub fn call_function_from_c(
    runtime: *const c_void,
    function_name: *const c_char,
    encoded_args: *const c_char,
    length: usize,
    id: u32,
) -> *const c_void {
    let func = with_library(|lib| lib.call_function_from_c);
    unsafe { func(runtime, function_name, encoded_args, length, id) }
}

pub fn call_function_parse_from_c(
    runtime: *const c_void,
    function_name: *const c_char,
    encoded_args: *const c_char,
    length: usize,
    id: u32,
) -> *const c_void {
    let func = with_library(|lib| lib.call_function_parse_from_c);
    unsafe { func(runtime, function_name, encoded_args, length, id) }
}

pub fn call_function_stream_from_c(
    runtime: *const c_void,
    function_name: *const c_char,
    encoded_args: *const c_char,
    length: usize,
    id: u32,
) -> *const c_void {
    let func = with_library(|lib| lib.call_function_stream_from_c);
    unsafe { func(runtime, function_name, encoded_args, length, id) }
}

pub fn call_object_constructor(encoded_args: *const c_char, length: usize) -> Buffer {
    let func = with_library(|lib| lib.call_object_constructor);
    unsafe { func(encoded_args, length) }
}

pub fn call_object_method(
    runtime: *const c_void,
    encoded_args: *const c_char,
    length: usize,
) -> Buffer {
    let func = with_library(|lib| lib.call_object_method);
    unsafe { func(runtime, encoded_args, length) }
}

pub fn free_buffer(buf: Buffer) {
    let func = with_library(|lib| lib.free_buffer);
    unsafe { func(buf) }
}

pub fn invoke_runtime_cli(args: *const *const c_char) -> i32 {
    let func = with_library(|lib| lib.invoke_runtime_cli);
    unsafe { func(args) }
}

pub fn version() -> *const c_char {
    let func = with_library(|lib| lib.version);
    unsafe { func() }
}

/// Retrieve the version string reported by the dynamically loaded BAML library.
pub fn get_library_version() -> Result<String, String> {
    let ptr = with_library(|lib| unsafe { (lib.version)() });
    if ptr.is_null() {
        return Err("version pointer returned null".to_string());
    }

    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|err| format!("version string contained invalid UTF-8: {err}"))
}

fn resolve_library_path() -> anyhow::Result<PathBuf> {
    if let Some(path) = shared_library_override()? {
        return Ok(path);
    }

    if let Ok(env_path) = env::var(LIBRARY_PATH_ENV) {
        let path = PathBuf::from(env_path);
        if path.exists() {
            return Ok(path);
        }
        return Err(anyhow!(
            "{LIBRARY_PATH_ENV} was set to {}, but the file does not exist",
            path.display()
        ));
    }

    let names = library_file_candidates();
    let mut candidates = Vec::new();

    if let Some(path) = option_env!("BAML_CFFI_DEFAULT_LIBRARY_PATH") {
        candidates.push(PathBuf::from(path));
    }

    if let Some(dir) = option_env!("BAML_CFFI_PROFILE_DIR") {
        for name in &names {
            candidates.push(PathBuf::from(dir).join(&name));
        }
    }

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow!("unable to resolve workspace root from CARGO_MANIFEST_DIR"))?
        .to_path_buf();

    let profile_name = option_env!("BAML_CFFI_PROFILE_NAME").unwrap_or("debug");
    let target_triple = option_env!("BAML_CFFI_TARGET_TRIPLE");

    for name in &names {
        candidates.push(workspace_root.join("target").join(profile_name).join(&name));
        if let Some(triple) = target_triple {
            candidates.push(
                workspace_root
                    .join("target")
                    .join(triple)
                    .join(profile_name)
                    .join(&name),
            );
        }
    }

    for fallback_profile in ["debug", "release"] {
        if fallback_profile == profile_name {
            continue;
        }
        for name in &names {
            candidates.push(
                workspace_root
                    .join("target")
                    .join(fallback_profile)
                    .join(&name),
            );
            if let Some(triple) = target_triple {
                candidates.push(
                    workspace_root
                        .join("target")
                        .join(triple)
                        .join(fallback_profile)
                        .join(&name),
                );
            }
        }
    }

    for dir in system_library_dirs() {
        for name in &names {
            candidates.push(dir.join(&name));
        }
    }

    let mut seen = HashSet::new();
    for candidate in candidates {
        if candidate.exists() && seen.insert(candidate.clone()) {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "unable to locate {LIB_BASE_NAME}.{LIB_EXT}; set {LIBRARY_PATH_ENV} to the full path of the shared library"
    ))
}

fn library_file_candidates() -> Vec<String> {
    let mut names = Vec::with_capacity(2);
    names.push(format!("{LIB_BASE_NAME}.{LIB_EXT}"));
    if let Some(triple) = option_env!("BAML_CFFI_TARGET_TRIPLE") {
        names.push(format!("{LIB_BASE_NAME}-{triple}.{LIB_EXT}"));
    }
    names
}

fn system_library_dirs() -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![
            PathBuf::from("/usr/local/lib"),
            PathBuf::from("/opt/homebrew/lib"),
        ]
    }
    #[cfg(target_os = "linux")]
    {
        vec![PathBuf::from("/usr/local/lib"), PathBuf::from("/usr/lib")]
    }
}

fn shared_library_override() -> anyhow::Result<Option<PathBuf>> {
    let lock = SHARED_LIBRARY_OVERRIDE.get_or_init(|| Mutex::new(None));
    let path = lock.lock().expect("override mutex poisoned");
    Ok(path.clone())
}

pub fn set_shared_library_path<P: AsRef<Path>>(path: P) {
    if LIBRARY.get().is_some() {
        panic!("baml_cffi shared library already loaded; cannot change path");
    }
    let lock = SHARED_LIBRARY_OVERRIDE.get_or_init(|| Mutex::new(None));
    let mut guard = lock.lock().expect("override mutex poisoned");
    *guard = Some(path.as_ref().to_path_buf());
}

// Re-export the protobuf types generated alongside the C FFI surface.
pub use crate::baml;
