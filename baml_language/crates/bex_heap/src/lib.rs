//! BEX Unified Heap
//!
//! This crate provides the unified heap for the BEX virtual machine,
//! implementing a CLR/JVM-style architecture with:
//!
//! - Lock-free field writes via `UnsafeCell<Vec<Object>>`
//! - Per-VM Thread-Local Allocation Buffers (TLABs)
//! - Handle table for FFI/external boundary
//!
//! # Architecture
//!
//! ```text
//! BexEngine owns Arc<BexHeap>
//!     │
//!     ├── BexVm1 has Tlab (exclusive allocation region)
//!     ├── BexVm2 has Tlab (exclusive allocation region)
//!     └── BexVm3 has Tlab (exclusive allocation region)
//! ```
//!
//! Each VM allocates into its own TLAB without contention. The heap
//! stores all objects in a single `Vec<Object>` with a boundary
//! separating compile-time objects (permanent) from runtime objects
//! (collectible by GC).

mod heap;
mod tlab;

// Re-export types from bex_external_types for convenience
pub use bex_external_types::{ExternalValue, Handle, Snapshot};
pub use heap::{BexHeap, DEFAULT_TLAB_SIZE, HeapStats};
pub use tlab::{Tlab, TlabChunk};
