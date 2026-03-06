//! BamlRuntime PyO3 class - wraps `Arc<dyn Bex>`.

use std::sync::Arc;

use bex_project::Bex;
use bridge_ctypes::{HANDLE_TABLE, external_to_baml_value, kwargs_to_bex_values};
use prost::Message;
use pyo3::{
    PyObject, Python,
    prelude::{PyResult, pymethods},
    pyclass,
};

use crate::{
    abort_controller::AbortController,
    errors::{BamlInvalidArgumentError, bridge_error_to_py, runtime_error_to_py},
    types::collector::Collector,
};

/// Per-call context for `BamlRuntime.call_function`.
///
/// Bundles tracing, collectors, cancellation, and streaming callbacks
/// into a single object. Constructed on the Python side:
///
/// ```python
/// ctx = CallContext(
///     host_span_manager=mgr,
///     collectors=[c],
///     abort_controller=ac,
///     stream_callback=lambda s: print(s),
///     tick_callback=lambda s: print(s),
/// )
/// runtime.call_function("fn", args, ctx)
/// ```
#[pyclass]
pub struct CallContext {
    #[pyo3(get, set)]
    host_span_manager: Option<PyObject>,
    #[pyo3(get, set)]
    abort_controller: Option<PyObject>,
    #[pyo3(get, set)]
    stream_callback: Option<PyObject>,
    #[pyo3(get, set)]
    tick_callback: Option<PyObject>,
    collectors: Option<PyObject>,
}

#[pymethods]
impl CallContext {
    #[new]
    #[pyo3(signature = (host_span_manager=None, collectors=None, abort_controller=None, stream_callback=None, tick_callback=None))]
    fn new(
        host_span_manager: Option<PyObject>,
        collectors: Option<PyObject>,
        abort_controller: Option<PyObject>,
        stream_callback: Option<PyObject>,
        tick_callback: Option<PyObject>,
    ) -> Self {
        Self {
            host_span_manager,
            collectors,
            abort_controller,
            stream_callback,
            tick_callback,
        }
    }
}

/// The main BAML runtime, wrapping a `dyn Bex` instance.
#[pyclass]
pub struct BamlRuntime {
    bex: Arc<dyn Bex>,
}

#[pymethods]
impl BamlRuntime {
    /// Create a runtime from in-memory BAML source files.
    ///
    /// # Arguments
    /// * `root_path` - Root path for BAML files
    /// * `files` - Map of filename to file content
    #[staticmethod]
    fn initialize(
        root_path: String,
        files: std::collections::HashMap<String, String>,
    ) -> PyResult<Self> {
        match bridge_cffi::engine::initialize_runtime(&root_path, files) {
            Ok(bex) => Ok(BamlRuntime { bex }),
            Err(e) => Err(bridge_error_to_py(e)),
        }
    }

    /// Call a BAML function asynchronously.
    ///
    /// # Arguments
    /// * `function_name` - Name of the BAML function to call
    /// * `args_proto` - Protobuf-encoded HostFunctionArguments bytes
    /// * `call_ctx` - Optional call context with tracing, collectors, and streaming callbacks
    #[pyo3(signature = (function_name, args_proto, call_ctx=None))]
    fn call_function<'py>(
        &self,
        py: Python<'py>,
        function_name: String,
        args_proto: Vec<u8>,
        call_ctx: Option<&CallContext>,
    ) -> PyResult<PyObject> {
        let bex = self.bex.clone();
        let kwargs = decode_args(&args_proto, &function_name)?;
        let ctx = build_call_context(py, call_ctx)?;

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let result = bex
                .call_function(&function_name, kwargs, ctx.build())
                .await
                .map_err(runtime_error_to_py)?;

            let handle_options = bridge_ctypes::HandleTableOptions::for_in_process();
            let baml_value = external_to_baml_value(&result, &handle_options).map_err(|e| {
                pyo3::PyErr::new::<BamlInvalidArgumentError, _>(format!(
                    "Failed to encode result: {e}"
                ))
            })?;

            Ok(baml_value.encode_to_vec())
        })
        .map(pyo3::Bound::into)
    }

    /// Call a BAML function synchronously (blocking).
    ///
    /// # Arguments
    /// * `function_name` - Name of the BAML function to call
    /// * `args_proto` - Protobuf-encoded HostFunctionArguments bytes
    /// * `call_ctx` - Optional call context with tracing, collectors, and streaming callbacks
    #[pyo3(signature = (function_name, args_proto, call_ctx=None))]
    fn call_function_sync(
        &self,
        py: Python<'_>,
        function_name: String,
        args_proto: Vec<u8>,
        call_ctx: Option<&CallContext>,
    ) -> PyResult<Vec<u8>> {
        let bex = self.bex.clone();
        let kwargs = decode_args(&args_proto, &function_name)?;
        let ctx = build_call_context(py, call_ctx)?;

        let rt = bridge_cffi::engine::get_tokio_runtime().map_err(bridge_error_to_py)?;

        let result = py
            .allow_threads(|| rt.block_on(bex.call_function(&function_name, kwargs, ctx.build())))
            .map_err(runtime_error_to_py)?;

        let handle_options = bridge_ctypes::HandleTableOptions::for_in_process();
        let baml_value = external_to_baml_value(&result, &handle_options).map_err(|e| {
            pyo3::PyErr::new::<BamlInvalidArgumentError, _>(format!("Failed to encode result: {e}"))
        })?;

        Ok(baml_value.encode_to_vec())
    }
}

/// Build a `FunctionCallContextBuilder` from the Python-side `CallContext`.
fn build_call_context(
    py: Python<'_>,
    call_ctx: Option<&CallContext>,
) -> PyResult<bex_project::FunctionCallContextBuilder> {
    let call_id = bex_project::CallId::next();
    let mut builder = bex_project::FunctionCallContextBuilder::new(call_id);

    let Some(ctx) = call_ctx else {
        return Ok(builder);
    };

    // Extract host span context
    if let Some(ref hsm_obj) = ctx.host_span_manager {
        let hsm: pyo3::PyRef<'_, crate::types::HostSpanManager> = hsm_obj.extract(py)?;
        if let Some(host_ctx) = hsm.host_span_context() {
            builder = builder.with_host_ctx(host_ctx);
        }
    }

    // Extract collectors
    if let Some(ref colls_obj) = ctx.collectors {
        let colls: Vec<pyo3::PyRef<'_, Collector>> = colls_obj.extract(py)?;
        let collector_arcs: Vec<Arc<bex_events::Collector>> =
            colls.iter().map(|c| c.inner_arc()).collect();
        builder = builder.with_collectors(collector_arcs);
    }

    // Extract cancellation token
    if let Some(ref ac_obj) = ctx.abort_controller {
        let ac: pyo3::PyRef<'_, AbortController> = ac_obj.extract(py)?;
        builder = builder.with_cancel_token(ac.token());
    }

    // Wire streaming callbacks
    if let Some(ref cb) = ctx.stream_callback {
        let cb = cb.clone_ref(py);
        builder = builder.with_stream_callback(Arc::new(move |value: String| {
            Python::with_gil(|py| {
                if let Err(e) = cb.call1(py, (&value,)) {
                    log::error!("Error calling stream_callback: {e:?}");
                }
            });
        }));
    }

    if let Some(ref cb) = ctx.tick_callback {
        let cb = cb.clone_ref(py);
        builder = builder.with_tick_callback(Arc::new(move |events: String| {
            Python::with_gil(|py| {
                if let Err(e) = cb.call1(py, (&events,)) {
                    log::error!("Error calling tick_callback: {e:?}");
                }
            });
        }));
    }

    Ok(builder)
}

/// Decode protobuf-encoded function arguments into `BexArgs`.
fn decode_args(args_proto: &[u8], function_name: &str) -> PyResult<bex_project::BexArgs> {
    let args = bridge_ctypes::baml::cffi::CallFunctionArgs::decode(args_proto).map_err(|e| {
        pyo3::PyErr::new::<BamlInvalidArgumentError, _>(format!(
            "Failed to decode arguments for function '{function_name}': {e}"
        ))
    })?;

    let kwargs = kwargs_to_bex_values(args.kwargs, &HANDLE_TABLE).map_err(|e| {
        pyo3::PyErr::new::<BamlInvalidArgumentError, _>(format!(
            "Failed to convert arguments for function '{function_name}': {e}"
        ))
    })?;

    Ok(kwargs.into())
}
