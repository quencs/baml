//! Unified heap for BEX virtual machine.
//!
//! The heap stores all objects in a single `Vec<Object>` with:
//! - Compile-time objects at indices 0..compile_time_boundary (permanent)
//! - Runtime objects at indices compile_time_boundary.. (collectible)
//!
//! # Thread Safety
//!
//! The heap uses `UnsafeCell<Vec<Object>>` for lock-free field writes.
//! Safety is ensured by:
//! - TLABs give each VM exclusive write access to its allocation region
//! - BAML has no global mutable variables, so independent calls can't race
//! - GC only runs when all VMs are at safepoints (yielded)

use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use bex_external_types::{Handle, WeakHeapRef};
use bex_vm_types::{Object, ObjectIndex};
use sharded_slab::Slab;

use crate::tlab::TlabChunk;

/// Default TLAB chunk size (number of object slots).
pub const DEFAULT_TLAB_SIZE: usize = 1024;

/// Statistics about heap usage.
#[derive(Clone, Copy, Debug, Default)]
pub struct HeapStats {
    /// Total objects allocated (including compile-time).
    pub total_objects: usize,
    /// Compile-time objects (permanent).
    pub compile_time_objects: usize,
    /// Runtime objects (collectible).
    pub runtime_objects: usize,
    /// Number of active handles.
    pub active_handles: usize,
    /// Number of TLAB chunks allocated.
    pub tlab_chunks: usize,
}

/// Unified heap for the BEX virtual machine.
///
/// All heap-allocated objects live here. The heap is shared across
/// all VM instances via `Arc<BexHeap<F>>`.
///
/// The type parameter `F` represents the native function type stored
/// in `Object::Function` variants. This allows the heap to be generic
/// over the native function implementation without creating circular
/// dependencies between crates.
///
/// # Example
///
/// ```ignore
/// // In bex_engine, instantiate with concrete NativeFunction type:
/// let heap: Arc<BexHeap<NativeFunction>> = BexHeap::new(compile_time_objects);
///
/// // In tests or when native functions aren't needed:
/// let heap: Arc<BexHeap<()>> = BexHeap::new(vec![]);
/// ```
pub struct BexHeap<F> {
    /// Object storage. Uses UnsafeCell for lock-free field writes.
    ///
    /// # Safety
    ///
    /// Access is safe because:
    /// - Each VM has exclusive write access to its TLAB region
    /// - BAML has no global mutable state
    /// - GC only runs at safepoints when no VMs are executing
    objects: UnsafeCell<Vec<Object<F>>>,

    /// Index of first runtime-allocated object.
    ///
    /// Objects at 0..compile_time_boundary are permanent (never collected).
    /// Objects at compile_time_boundary.. are runtime (collectible by GC).
    compile_time_boundary: usize,

    /// Next TLAB chunk start index.
    ///
    /// When a VM needs a new TLAB, it atomically increments this by the
    /// chunk size to reserve its region.
    next_chunk: AtomicUsize,

    /// Handle table for external/FFI boundary.
    ///
    /// Maps slab keys to ObjectIndex values. Handles provide safe,
    /// validated access to heap objects from external code.
    handles: Slab<ObjectIndex>,

    /// TLAB chunk size for new allocations.
    tlab_size: usize,

    /// Lock for growing the objects vector (rare operation).
    ///
    /// Only held during Vec resizing when a TLAB chunk allocation needs to grow
    /// the backing storage. This doesn't affect fast-path allocation which is
    /// lock-free within a TLAB.
    growth_lock: Mutex<()>,

    /// Phantom data to hold the type parameter.
    _marker: PhantomData<F>,
}

// SAFETY: BexHeap<F> is Send + Sync when F is Send + Sync because:
// - objects: UnsafeCell is accessed safely via TLAB exclusivity and growth_lock
// - compile_time_boundary: immutable after construction
// - next_chunk: AtomicUsize is thread-safe
// - handles: sharded_slab::Slab is thread-safe
// - tlab_size: immutable after construction
// - growth_lock: Mutex is thread-safe
unsafe impl<F: Send + Sync> Send for BexHeap<F> {}
unsafe impl<F: Send + Sync> Sync for BexHeap<F> {}

// Implement WeakHeapRef trait from bex_external_types
impl<F> WeakHeapRef for BexHeap<F>
where
    F: Send + Sync,
{
    fn release_handle(&self, slab_key: usize) {
        self.handles.remove(slab_key);
    }
}

impl<F> BexHeap<F> {
    /// Create a new heap with compile-time objects.
    ///
    /// The provided objects become permanent (never garbage collected).
    /// Runtime allocations will start after these objects.
    pub fn new(compile_time_objects: Vec<Object<F>>) -> Arc<Self> {
        Self::with_tlab_size(compile_time_objects, DEFAULT_TLAB_SIZE)
    }

    /// Create a new heap with custom TLAB size.
    pub fn with_tlab_size(compile_time_objects: Vec<Object<F>>, tlab_size: usize) -> Arc<Self> {
        let boundary = compile_time_objects.len();

        Arc::new(Self {
            objects: UnsafeCell::new(compile_time_objects),
            compile_time_boundary: boundary,
            next_chunk: AtomicUsize::new(boundary),
            handles: Slab::new(),
            tlab_size,
            growth_lock: Mutex::new(()),
            _marker: PhantomData,
        })
    }

    /// Get the compile-time boundary index.
    ///
    /// Objects before this index are permanent. Objects at or after
    /// this index are runtime allocations that can be garbage collected.
    pub fn compile_time_boundary(&self) -> usize {
        self.compile_time_boundary
    }

    /// Get the TLAB chunk size.
    pub fn tlab_size(&self) -> usize {
        self.tlab_size
    }

    /// Get a raw pointer to the objects vector.
    ///
    /// # Safety
    ///
    /// Caller must ensure no data races:
    /// - Only write to indices within your TLAB's exclusive region
    /// - Only read compile-time objects or objects you own
    pub unsafe fn objects_ptr(&self) -> *mut Vec<Object<F>> {
        self.objects.get()
    }

    /// Get the current number of objects in the heap.
    pub fn len(&self) -> usize {
        // SAFETY: Reading len is safe, we're not modifying the vec
        unsafe { (*self.objects.get()).len() }
    }

    /// Check if the heap is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Allocate a new TLAB chunk.
    ///
    /// This method is thread-safe. Multiple VMs can request chunks
    /// concurrently - each gets a unique, non-overlapping region.
    ///
    /// # Thread Safety
    ///
    /// Uses `fetch_add` with `SeqCst` ordering to ensure each caller
    /// gets a unique chunk range, even under concurrent access. The
    /// growth_lock protects the rare Vec resize operation.
    ///
    /// Returns a `TlabChunk` describing the exclusive region for the VM.
    /// The VM can then allocate objects within this region without locks.
    pub fn alloc_tlab_chunk(&self) -> TlabChunk {
        // Atomically reserve a chunk range
        let start = self.next_chunk.fetch_add(self.tlab_size, Ordering::SeqCst);
        let end = start + self.tlab_size;

        // Lock only for growth (rare operation)
        let _guard = self.growth_lock.lock().unwrap();

        // Grow backing storage if needed
        // SAFETY: Multiple threads may call this concurrently. The growth_lock
        // ensures only one thread resizes at a time. TLABs work with indices,
        // not pointers, so reallocation is safe.
        unsafe {
            let objects = &mut *self.objects.get();
            if objects.len() < end {
                objects.resize_with(end, || {
                    // Placeholder object - will be overwritten by TLAB alloc
                    Object::String(String::new())
                });
            }
        }

        TlabChunk { start, end }
    }

    /// Read an object by index (safe for compile-time objects).
    ///
    /// # Safety
    ///
    /// For runtime objects, caller must ensure no concurrent writes.
    pub unsafe fn get_object(&self, idx: ObjectIndex) -> &Object<F> {
        // SAFETY: Caller ensures no concurrent writes
        unsafe { &(&*self.objects.get())[idx.into_raw()] }
    }

    /// Get statistics about heap usage.
    pub fn stats(&self) -> HeapStats {
        let total = self.len();
        let tlab_chunks = self
            .next_chunk
            .load(Ordering::Relaxed)
            .saturating_sub(self.compile_time_boundary)
            .div_ceil(self.tlab_size);

        HeapStats {
            total_objects: total,
            compile_time_objects: self.compile_time_boundary,
            runtime_objects: total.saturating_sub(self.compile_time_boundary),
            active_handles: 0, // sharded_slab doesn't expose count
            tlab_chunks,
        }
    }
}

impl<F> BexHeap<F>
where
    F: Send + Sync + 'static,
{
    /// Create a handle to an object.
    ///
    /// Handles are used at the FFI boundary to give external code safe
    /// access to heap objects.
    pub fn create_handle(self: &Arc<Self>, idx: ObjectIndex) -> Handle {
        let slab_key = self.handles.insert(idx).expect("handle table full");

        // Use Handle::new from bex_external_types, passing self as WeakHeapRef
        Handle::new(slab_key, idx, Arc::clone(self) as Arc<dyn WeakHeapRef>)
    }

    /// Resolve a handle to its ObjectIndex.
    ///
    /// Returns None if the handle has been invalidated (e.g., by GC).
    pub fn resolve_handle(&self, handle: &Handle) -> Option<ObjectIndex> {
        self.handles.get(handle.slab_key()).map(|entry| *entry)
    }
}

impl<F> std::fmt::Debug for BexHeap<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BexHeap")
            .field("len", &self.len())
            .field("compile_time_boundary", &self.compile_time_boundary)
            .field("tlab_size", &self.tlab_size)
            .finish()
    }
}

// Static assertions to verify thread safety
const _: () = {
    const fn assert_send<T: Send>() {}
    const fn assert_sync<T: Sync>() {}

    // BexHeap must be Send + Sync for Arc<BexHeap> to work across threads
    assert_send::<BexHeap<()>>();
    assert_sync::<BexHeap<()>>();
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_heap_empty() {
        let heap = BexHeap::<()>::new(vec![]);
        assert_eq!(heap.len(), 0);
        assert_eq!(heap.compile_time_boundary(), 0);
    }

    #[test]
    fn test_new_heap_with_objects() {
        let objects: Vec<Object<()>> = vec![
            Object::String("hello".to_string()),
            Object::String("world".to_string()),
        ];
        let heap = BexHeap::new(objects);
        assert_eq!(heap.len(), 2);
        assert_eq!(heap.compile_time_boundary(), 2);
    }

    #[test]
    fn test_alloc_tlab_chunk() {
        let heap = BexHeap::<()>::with_tlab_size(vec![], 100);

        let chunk1 = heap.alloc_tlab_chunk();
        assert_eq!(chunk1.start, 0);
        assert_eq!(chunk1.end, 100);

        let chunk2 = heap.alloc_tlab_chunk();
        assert_eq!(chunk2.start, 100);
        assert_eq!(chunk2.end, 200);

        // Heap should have grown to accommodate chunks
        assert!(heap.len() >= 200);
    }

    #[test]
    fn test_handle_create_resolve() {
        let objects: Vec<Object<()>> = vec![Object::String("test".to_string())];
        let heap = BexHeap::new(objects);

        let handle = heap.create_handle(ObjectIndex::from_raw(0));
        let resolved = heap.resolve_handle(&handle);
        assert_eq!(resolved, Some(ObjectIndex::from_raw(0)));
    }

    #[test]
    fn test_handle_clone_and_drop() {
        let objects: Vec<Object<()>> = vec![Object::String("test".to_string())];
        let heap = BexHeap::new(objects);

        let handle1 = heap.create_handle(ObjectIndex::from_raw(0));
        let handle2 = handle1.clone();

        // Both handles resolve
        assert!(heap.resolve_handle(&handle1).is_some());
        assert!(heap.resolve_handle(&handle2).is_some());

        // Drop first clone
        drop(handle1);

        // Second clone still resolves
        assert!(heap.resolve_handle(&handle2).is_some());

        // Drop second clone - this should release the slab entry
        drop(handle2);

        // Heap is still valid after all handles dropped
        assert_eq!(heap.len(), 1);
    }

    #[test]
    fn test_heap_stats() {
        let compile_time: Vec<Object<()>> = vec![Object::String("builtin".to_string())];
        let heap = BexHeap::with_tlab_size(compile_time, 50);

        let stats = heap.stats();
        assert_eq!(stats.compile_time_objects, 1);
        assert_eq!(stats.total_objects, 1);
        assert_eq!(stats.runtime_objects, 0);

        // Allocate a TLAB chunk
        let _chunk = heap.alloc_tlab_chunk();

        let stats = heap.stats();
        assert_eq!(stats.tlab_chunks, 1);
        assert!(stats.total_objects >= 51); // Expanded for TLAB
    }
}
