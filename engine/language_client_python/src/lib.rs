mod abort_controller;
mod errors;
mod parse_py_type;
mod runtime;
mod types;

use std::sync::atomic::{AtomicBool, Ordering};

// Flag to prevent recursive signal handler calls
static HANDLING_SIGNAL: AtomicBool = AtomicBool::new(false);

/// Install a signal handler for SIGSEGV that prints a backtrace before crashing.
/// This helps debug fork-safety issues.
fn install_sigsegv_handler() {
    unsafe {
        // Set up a signal handler for SIGSEGV
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = sigsegv_handler as usize;
        action.sa_flags = libc::SA_SIGINFO;

        if libc::sigaction(libc::SIGSEGV, &action, std::ptr::null_mut()) != 0 {
            eprintln!("[BAML] Warning: Failed to install SIGSEGV handler");
        } else {
            eprintln!("[BAML] SIGSEGV handler installed for debugging, PID={}", std::process::id());
        }
    }
}

extern "C" fn sigsegv_handler(sig: libc::c_int, _info: *mut libc::siginfo_t, _ctx: *mut libc::c_void) {
    // Prevent recursive calls
    if HANDLING_SIGNAL.swap(true, Ordering::SeqCst) {
        // Already handling a signal, just abort
        unsafe { libc::_exit(128 + sig) };
    }

    // Use write() directly instead of eprintln! since we're in a signal handler
    // and eprintln! may not be signal-safe
    let msg = b"\n\n=== SIGSEGV CAUGHT ===\nPID: ";
    unsafe { libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len()) };

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

    let msg3 = b"\n=== END SIGSEGV ===\n";
    unsafe { libc::write(2, msg3.as_ptr() as *const libc::c_void, msg3.len()) };

    // Re-raise the signal with default handler to get proper exit code
    unsafe {
        libc::signal(libc::SIGSEGV, libc::SIG_DFL);
        libc::raise(libc::SIGSEGV);
    }
}

use pyo3::{
    prelude::{pyfunction, pymodule, PyAnyMethods, PyModule, PyResult},
    types::PyModuleMethods,
    wrap_pyfunction, Bound, Python,
};

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
    install_sigsegv_handler();

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
