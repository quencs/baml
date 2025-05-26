use std::ops::Deref;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use baml_runtime::tracingv2::storage::storage::{Collector, FunctionLog};

pub struct RawPtrWrapper<T> {
    inner: Arc<T>,
    persist: AtomicBool,
}

impl<T> RawPtrWrapper<T> {
    pub fn from_raw(object: *const libc::c_void, persist: bool) -> Self {
        Self {
            inner: unsafe { Arc::from_raw(object as *const T) },
            persist: AtomicBool::new(persist),
        }
    }

    pub fn destroy(self) {
        self.persist
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

impl<T> Deref for RawPtrWrapper<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Drop for RawPtrWrapper<T> {
    fn drop(&mut self) {
        if self.persist.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = Arc::into_raw(self.inner.clone());
        }
    }
}

pub type CollectorWrapper = RawPtrWrapper<Collector>;
pub type FunctionLogWrapper = RawPtrWrapper<Mutex<FunctionLog>>;
