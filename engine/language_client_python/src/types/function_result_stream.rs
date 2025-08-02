use std::collections::HashMap;

use pyo3::{
    prelude::{pymethods, PyResult},
    PyObject, PyRefMut, Python,
};

use super::{function_results::FunctionResult, runtime_ctx_manager::RuntimeContextManager};
use crate::errors::BamlError;

crate::lang_wrapper!(
    FunctionResultStream,
    baml_runtime::FunctionResultStream, thread_safe,
    on_event: Option<PyObject>,
    tb: Option<baml_runtime::type_builder::TypeBuilder>,
    cb: Option<baml_runtime::client_registry::ClientRegistry>,
    env_vars: HashMap<String, String>
);

crate::lang_wrapper!(
    SyncFunctionResultStream,
    baml_runtime::FunctionResultStream, sync_thread_safe,
    on_event: Option<PyObject>,
    tb: Option<baml_runtime::type_builder::TypeBuilder>,
    cb: Option<baml_runtime::client_registry::ClientRegistry>,
    env_vars: HashMap<String, String>
);

impl FunctionResultStream {
    pub(crate) fn new(
        inner: baml_runtime::FunctionResultStream,
        event: Option<PyObject>,
        tb: Option<baml_runtime::type_builder::TypeBuilder>,
        cb: Option<baml_runtime::client_registry::ClientRegistry>,
        env_vars: HashMap<String, String>,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(inner)),
            on_event: event,
            tb,
            cb,
            env_vars,
        }
    }
}

impl SyncFunctionResultStream {
    pub(crate) fn new(
        inner: baml_runtime::FunctionResultStream,
        event: Option<PyObject>,
        tb: Option<baml_runtime::type_builder::TypeBuilder>,
        cb: Option<baml_runtime::client_registry::ClientRegistry>,
        env_vars: HashMap<String, String>,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(inner)),
            on_event: event,
            tb,
            cb,
            env_vars,
        }
    }
}

#[pymethods]
impl FunctionResultStream {
    /// Cancel the stream processing.
    /// This will stop any ongoing stream processing and clean up resources.
    /// Also cancels the underlying Rust HTTP requests.
    fn cancel(&self) -> PyResult<()> {
        // Cancel the underlying Rust stream
        let inner = self.inner.clone();
        tokio::spawn(async move {
            if let Ok(mut stream) = inner.try_lock() {
                stream.cancel();
            }
        });

        Ok(())
    }

    fn on_event(&mut self, py: Python, func: Option<PyObject>) -> PyResult<()> {
        self.on_event = func;
        Ok(())
    }

    fn done(&self, py: Python, ctx_manager: &RuntimeContextManager) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        let on_event = self.on_event.clone();
        let tb = self.tb.clone();
        let cb = self.cb.clone();
        let env_vars = self.env_vars.clone();
        let ctx_manager = ctx_manager.inner.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let on_event_fn = on_event.map(|callback| {
                move |result: baml_runtime::FunctionResult| {
                    Python::with_gil(|py| {
                        let py_result = FunctionResult::from(result);
                        if let Err(e) = callback.call1(py, (py_result,)) {
                            eprintln!("Error calling Python callback: {:?}", e);
                        }
                    });
                }
            });

            let result = inner
                .lock()
                .await
                .run(
                    None::<fn()>,
                    on_event_fn,
                    &ctx_manager,
                    tb.as_ref(),
                    cb.as_ref(),
                    env_vars,
                )
                .await;

            Python::with_gil(|py| {
                result
                    .0
                    .map(FunctionResult::from)
                    .map_err(BamlError::from_anyhow)
                    .map(|r| r.into_py(py))
            })
        })
    }
}

#[pymethods]
impl SyncFunctionResultStream {
    /// Cancel the stream processing (sync version).
    fn cancel(&self) -> PyResult<()> {
        // Cancel the underlying Rust stream
        if let Ok(mut stream) = self.inner.try_lock() {
            stream.cancel();
        }

        Ok(())
    }

    fn on_event(&mut self, py: Python, func: Option<PyObject>) -> PyResult<()> {
        self.on_event = func;
        Ok(())
    }

    fn done(&self, py: Python, ctx_manager: &RuntimeContextManager) -> PyResult<FunctionResult> {
        let inner = self.inner.clone();

        let on_event = self.on_event.clone();
        let tb = self.tb.clone();
        let cb = self.cb.clone();
        let env_vars = self.env_vars.clone();
        let ctx_manager = ctx_manager.inner.clone();

        let on_event_fn = on_event.map(|callback| {
            move |result: baml_runtime::FunctionResult| {
                Python::with_gil(|py| {
                    let py_result = FunctionResult::from(result);
                    if let Err(e) = callback.call1(py, (py_result,)) {
                        eprintln!("Error calling Python callback: {:?}", e);
                    }
                });
            }
        });

        let result = inner.lock().unwrap().run_sync(
            None::<fn()>,
            on_event_fn,
            &ctx_manager,
            tb.as_ref(),
            cb.as_ref(),
            env_vars,
        );

        result
            .0
            .map(FunctionResult::from)
            .map_err(BamlError::from_anyhow)
    }
}


        let mut locked = inner.lock().unwrap();
        let (res, _) = locked.run_sync(
            None::<fn()>,
            on_event,
            &ctx_mng,
            tb.as_ref(),
            cb.as_ref(),
            env_vars,
        );
        res.map(FunctionResult::from)
            .map_err(BamlError::from_anyhow)
    }
}
