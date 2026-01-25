mod abort_controller;
mod errors;
mod parse_py_type;
mod runtime;
mod types;

use std::sync::atomic::{AtomicBool, Ordering};

// Flag to prevent recursive signal handler calls
static HANDLING_SIGNAL: AtomicBool = AtomicBool::new(false);

/// Install signal handlers for crash signals that print a backtrace before crashing.
/// This helps debug fork-safety issues.
fn install_signal_handlers() {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = crash_signal_handler as usize;
        action.sa_flags = libc::SA_SIGINFO;

        // Install handler for SIGSEGV (signal 11)
        if libc::sigaction(libc::SIGSEGV, &action, std::ptr::null_mut()) != 0 {
            eprintln!("[BAML] Warning: Failed to install SIGSEGV handler");
        }

        // Install handler for SIGTRAP (signal 5)
        if libc::sigaction(libc::SIGTRAP, &action, std::ptr::null_mut()) != 0 {
            eprintln!("[BAML] Warning: Failed to install SIGTRAP handler");
        }

        // Install handler for SIGBUS (signal 10)
        if libc::sigaction(libc::SIGBUS, &action, std::ptr::null_mut()) != 0 {
            eprintln!("[BAML] Warning: Failed to install SIGBUS handler");
        }

        // Install handler for SIGABRT (signal 6)
        if libc::sigaction(libc::SIGABRT, &action, std::ptr::null_mut()) != 0 {
            eprintln!("[BAML] Warning: Failed to install SIGABRT handler");
        }
    }
}

fn signal_name(sig: libc::c_int) -> &'static str {
    match sig {
        libc::SIGSEGV => "SIGSEGV",
        libc::SIGTRAP => "SIGTRAP",
        libc::SIGBUS => "SIGBUS",
        libc::SIGABRT => "SIGABRT",
        _ => "UNKNOWN",
    }
}

extern "C" fn crash_signal_handler(sig: libc::c_int, _info: *mut libc::siginfo_t, _ctx: *mut libc::c_void) {
    // Prevent recursive calls
    if HANDLING_SIGNAL.swap(true, Ordering::SeqCst) {
        // Already handling a signal, just abort
        unsafe { libc::_exit(128 + sig) };
    }

    // Use write() directly instead of eprintln! since we're in a signal handler
    // and eprintln! may not be signal-safe
    let sig_name = signal_name(sig);
    let header = format!("\n\n=== {} (signal {}) CAUGHT ===\nPID: ", sig_name, sig);
    unsafe { libc::write(2, header.as_ptr() as *const libc::c_void, header.len()) };

    let pid = std::process::id();
    let pid_str = format!("{}\n", pid);
    unsafe { libc::write(2, pid_str.as_ptr() as *const libc::c_void, pid_str.len()) };

    let msg2 = b"\nBacktrace:\n";
    unsafe { libc::write(2, msg2.as_ptr() as *const libc::c_void, msg2.len()) };

    // Capture and print backtrace
    // Note: This may not be fully signal-safe, but it's our best effort for debugging
    let bt = backtrace::Backtrace::new();
    let bt_str = format!("{:?}\n", bt);
    unsafe { libc::write(2, bt_str.as_ptr() as *const libc::c_void, bt_str.len()) };

    let footer = format!("\n=== END {} ===\n", sig_name);
    unsafe { libc::write(2, footer.as_ptr() as *const libc::c_void, footer.len()) };

    // Re-raise the signal with default handler to get proper exit code
    unsafe {
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
    }
}

use std::future::Future;

use pyo3::{
    prelude::{pyfunction, pymodule, PyAnyMethods, PyModule, PyResult},
    types::PyModuleMethods,
    wrap_pyfunction, Bound, BoundObject, IntoPyObject, Py, Python,
};

/// Fork-safe alternative to pyo3_async_runtimes::tokio::future_into_py.
///
/// This function works correctly in forked child processes by using BAML's
/// fork-safe tokio runtime instead of pyo3-async-runtimes' global runtime
/// (which becomes corrupted after fork).
///
/// The approach:
/// 1. Get BAML's fork-safe tokio runtime (creates fresh one if in forked child)
/// 2. Create a Python asyncio.Future
/// 3. Spawn the Rust future on our fork-safe runtime
/// 4. When complete, set the result on the Python future via call_soon_threadsafe
pub(crate) fn fork_safe_future_into_py<F, T>(py: Python<'_>, fut: F) -> PyResult<Bound<'_, pyo3::PyAny>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: for<'py> IntoPyObject<'py> + Send + 'static,
{
    // Get fork-safe runtime
    let rt = baml_runtime::BamlRuntime::get_tokio_singleton()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to get tokio runtime: {}", e)))?;

    // Get the running event loop
    let asyncio = py.import("asyncio")?;
    let event_loop = asyncio.call_method0("get_running_loop")?;

    // Create a Python future
    let py_future = event_loop.call_method0("create_future")?;

    // Clone references for the spawned task
    let py_future_ref: Py<pyo3::PyAny> = py_future.clone().unbind();
    let event_loop_ref: Py<pyo3::PyAny> = event_loop.clone().unbind();

    // Spawn on our fork-safe runtime
    rt.spawn(async move {
        let result = fut.await;

        // Set the result on the Python future from the Rust thread
        // We need to use call_soon_threadsafe since we're not on the Python thread
        Python::with_gil(|py| {
            let future = py_future_ref.bind(py);
            let loop_ = event_loop_ref.bind(py);

            match result {
                Ok(val) => {
                    match val.into_pyobject(py) {
                        Ok(py_val) => {
                            let py_obj: Py<pyo3::PyAny> = py_val.into_any().unbind();
                            let set_result = future.getattr("set_result").unwrap();
                            let _ = loop_.call_method1("call_soon_threadsafe", (set_result, py_obj));
                        }
                        Err(e) => {
                            let err: pyo3::PyErr = e.into();
                            let set_exception = future.getattr("set_exception").unwrap();
                            let _ = loop_.call_method1("call_soon_threadsafe", (set_exception, err.value(py)));
                        }
                    }
                }
                Err(e) => {
                    let set_exception = future.getattr("set_exception").unwrap();
                    let _ = loop_.call_method1("call_soon_threadsafe", (set_exception, e.value(py)));
                }
            }
        });
    });

    Ok(py_future)
}

#[pyfunction]
fn invoke_runtime_cli(py: Python) -> PyResult<i32> {
    match baml_cli::run_cli(
        py.import("sys")?
            .getattr("argv")?
            .extract::<Vec<String>>()?,
        baml_runtime::RuntimeCliDefaults {
            output_type: baml_types::GeneratorOutputType::PythonPydantic,
        },
    ) {
        Ok(exit_code) => Ok(exit_code.into()),
        Err(e) => Err(errors::BamlError::from_anyhow(e)),
    }
}

pub(crate) const MODULE_NAME: &str = "baml_py.baml_py";

#[pyfunction]
fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pyfunction]
fn get_log_level() -> PyResult<&'static str> {
    Ok(baml_log::get_log_level().as_str())
}

#[pyfunction]
fn set_log_level(level: &str) -> PyResult<()> {
    let _ = level.parse().map(baml_log::set_log_level);
    Ok(())
}

#[pyfunction]
fn set_log_json_mode(json: bool) -> PyResult<()> {
    baml_log::set_json_mode(json).map_err(errors::BamlError::from_anyhow)
}

#[pyfunction]
fn set_log_max_chunk_length(length: usize) -> PyResult<()> {
    baml_log::set_max_message_length(length).map_err(errors::BamlError::from_anyhow)
}

#[pymodule]
fn baml_py(m: Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<runtime::BamlRuntime>()?;
    m.add_class::<types::FunctionResult>()?;
    m.add_class::<types::FunctionResultStream>()?;
    m.add_class::<types::SyncFunctionResultStream>()?;
    m.add_class::<types::BamlImagePy>()?;
    m.add_class::<types::BamlAudioPy>()?;
    m.add_class::<types::BamlPdfPy>()?;
    m.add_class::<types::BamlVideoPy>()?;
    m.add_class::<types::RuntimeContextManager>()?;
    m.add_class::<types::BamlSpan>()?;
    m.add_class::<types::TypeBuilder>()?;
    m.add_class::<types::EnumBuilder>()?;
    m.add_class::<types::ClassBuilder>()?;
    m.add_class::<types::EnumValueBuilder>()?;
    m.add_class::<types::ClassPropertyBuilder>()?;
    m.add_class::<types::FieldType>()?;
    m.add_class::<types::ClientRegistry>()?;
    m.add_class::<abort_controller::AbortController>()?;

    m.add_class::<runtime::BamlLogEvent>()?;
    m.add_class::<runtime::LogEventMetadata>()?;
    m.add_class::<types::Collector>()?;
    m.add_class::<types::FunctionLog>()?;
    m.add_class::<types::LLMCall>()?;
    m.add_class::<types::Timing>()?;
    m.add_class::<types::LLMStreamCall>()?;
    m.add_class::<types::SSEResponse>()?;
    m.add_class::<types::StreamTiming>()?;
    m.add_class::<types::Usage>()?;
    m.add_class::<types::HTTPRequest>()?;
    m.add_wrapped(wrap_pyfunction!(invoke_runtime_cli))?;
    m.add_wrapped(wrap_pyfunction!(get_version))?;
    m.add_wrapped(wrap_pyfunction!(set_log_level))?;
    m.add_wrapped(wrap_pyfunction!(set_log_json_mode))?;
    m.add_wrapped(wrap_pyfunction!(get_log_level))?;
    m.add_wrapped(wrap_pyfunction!(set_log_max_chunk_length))?;
    errors::errors(&m)?;

    // Initialize the logger
    baml_log::init().map_err(errors::BamlError::from_anyhow)?;
    init_debug_logger();

    // Install SIGSEGV handler for debugging fork-safety issues
    install_signal_handlers();

    Ok(())
}

fn init_debug_logger() {
    // Regular formatting
    if let Err(e) =
        env_logger::try_init_from_env(env_logger::Env::new().filter("BAML_INTERNAL_LOG"))
    {
        eprintln!("Failed to initialize BAML DEBUG logger: {e:#}");
    }
}
