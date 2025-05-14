use baml_runtime::runtime_interface::ExperimentalTracingInterface;
use baml_types::BamlValue;
use pyo3::prelude::{pymethods, PyResult};
use pyo3::{IntoPyObjectExt, PyObject, Python};

use crate::errors::{BamlError, BamlInvalidArgumentError};
use crate::parse_py_type::parse_py_type;

use super::runtime_ctx_manager::RuntimeContextManager;
use crate::runtime::BamlRuntime;

crate::lang_wrapper!(BamlSpan,
  Option<baml_runtime::tracing::TracingCall>,
  no_from,
  rt: std::sync::Arc<baml_runtime::BamlRuntime>
);

#[pymethods]
impl BamlSpan {
    #[staticmethod]
    fn new(
        py: Python<'_>,
        runtime: &BamlRuntime,
        function_name: &str,
        args: PyObject,
        ctx: &RuntimeContextManager,
    ) -> PyResult<Self> {
        let args = parse_py_type(args.into_bound(py).into_py_any(py)?, true)?
            .unwrap_or(BamlValue::Map(Default::default()));
        let Some(args_map) = args.as_map() else {
            return Err(BamlInvalidArgumentError::new_err("Failed to parse args"));
        };

        let call = runtime
            .inner
            .start_call(function_name, args_map, &ctx.inner);

        log::trace!("Starting call: {:#?} for {:?}\n", call, function_name);
        Ok(Self {
            inner: Some(call),
            rt: runtime.inner.clone(),
        })
    }

    // method to finish
    fn finish(
        &mut self,
        py: Python<'_>,
        result: PyObject,
        ctx: &RuntimeContextManager,
    ) -> PyResult<String> {
        log::trace!("Finishing call: {:?}", self.inner);
        let result = parse_py_type(result.into_bound(py).into_py_any(py)?, true)?;

        let call = self
            .inner
            .take()
            .ok_or_else(|| BamlError::new_err("Call already finished"))?;

        self.rt
            .finish_call(call, result, &ctx.inner)
            .map_err(BamlError::from_anyhow)
            .map(|u| u.to_string())
    }
}
