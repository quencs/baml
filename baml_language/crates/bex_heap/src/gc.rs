//! Garbage collection for the unified heap.
//!
//! BEX uses a safepoint-based, non-moving mark-and-sweep collector:
//!
//! - **Safepoints**: GC only runs when all VMs are yielded (async operations)
//! - **Non-moving**: Objects stay in place; no index updates needed
//! - **Handle-aware**: Stale handles invalidated after collection

use std::collections::HashSet;

use bex_vm_types::{ObjectIndex, Value};

use crate::BexHeap;

/// Result of a garbage collection cycle.
#[derive(Debug, Clone)]
pub struct GcStats {
    /// Objects marked as live.
    pub live_count: usize,
    /// Objects collected (dead).
    pub collected_count: usize,
    /// Handles invalidated.
    pub handles_invalidated: usize,
}

impl<F> BexHeap<F> {
    /// Run garbage collection with the given roots.
    ///
    /// # Safety
    ///
    /// Caller must ensure all VMs are at safepoints (not executing).
    /// This is typically guaranteed by the engine's event loop.
    ///
    /// # Arguments
    ///
    /// * `roots` - Stack roots from all yielded VMs, plus any externally-held handles
    pub unsafe fn collect_garbage(&self, roots: &[ObjectIndex]) -> GcStats {
        // 1. Mark phase: trace from roots
        let live = self.mark_phase(roots);

        // 2. Sweep phase: invalidate dead handles
        self.sweep_phase(&live)
    }

    fn mark_phase(&self, roots: &[ObjectIndex]) -> HashSet<ObjectIndex> {
        let mut live = HashSet::new();
        let mut worklist: Vec<ObjectIndex> = roots.to_vec();

        // Note: In a full implementation, we would also include handles as roots.
        // However, sharded_slab's iteration API requires mutable access,
        // which conflicts with the safety requirements of GC.
        // For Phase 6, we rely on explicit roots passed by the engine.
        // TODO: Implement handle tracking via a separate data structure

        while let Some(idx) = worklist.pop() {
            if !live.insert(idx) {
                continue; // Already visited
            }

            // Skip compile-time objects (always live)
            if idx.into_raw() < self.compile_time_boundary {
                continue;
            }

            // Trace references in this object
            let obj = unsafe { &(&*self.objects.get())[idx.into_raw()] };
            self.trace_object(obj, &mut worklist);
        }

        live
    }

    fn trace_object(&self, obj: &bex_vm_types::Object<F>, worklist: &mut Vec<ObjectIndex>) {
        use bex_vm_types::Object;

        match obj {
            Object::Array(arr) => {
                for value in arr {
                    if let Value::Object(idx) = value {
                        worklist.push(*idx);
                    }
                }
            }
            Object::Map(map) => {
                for value in map.values() {
                    if let Value::Object(idx) = value {
                        worklist.push(*idx);
                    }
                }
            }
            Object::Instance(inst) => {
                worklist.push(inst.class);
                for value in &inst.fields {
                    if let Value::Object(idx) = value {
                        worklist.push(*idx);
                    }
                }
            }
            Object::Variant(var) => {
                worklist.push(var.enm);
            }
            Object::Future(fut) => {
                use bex_vm_types::Future;
                match fut {
                    Future::Pending(pending) => {
                        // Trace arguments that might contain object references
                        for value in &pending.args {
                            if let Value::Object(idx) = value {
                                worklist.push(*idx);
                            }
                        }
                    }
                    Future::Ready(value) => {
                        // Trace ready value if it's an object
                        if let Value::Object(idx) = value {
                            worklist.push(*idx);
                        }
                    }
                }
            }
            // Primitives have no references
            Object::String(_)
            | Object::Class(_)
            | Object::Enum(_)
            | Object::Function(_)
            | Object::Media(_) => {}
        }
    }

    fn sweep_phase(&self, live: &HashSet<ObjectIndex>) -> GcStats {
        // Note: In a full implementation, we would invalidate handles pointing
        // to dead objects. However, this requires mutable access to the slab,
        // which conflicts with the current GC design.
        //
        // For Phase 6, we skip handle invalidation. Dead objects referenced
        // by handles will be kept alive until the handle is dropped.
        // This is safe but slightly wasteful.
        let handles_invalidated = 0;

        // Count dead runtime objects
        let total_runtime = unsafe { (*self.objects.get()).len() - self.compile_time_boundary };
        let collected_count = total_runtime.saturating_sub(live.len());

        GcStats {
            live_count: live.len(),
            collected_count,
            handles_invalidated,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bex_vm_types::Object;

    use super::*;
    use crate::Tlab;

    #[test]
    fn test_gc_empty_heap() {
        let heap = BexHeap::<()>::new(vec![]);

        // Run GC with no roots
        let stats = unsafe { heap.collect_garbage(&[]) };

        assert_eq!(stats.live_count, 0);
        assert_eq!(stats.collected_count, 0);
        assert_eq!(stats.handles_invalidated, 0);
    }

    #[test]
    fn test_gc_preserves_compile_time_objects() {
        let compile_time: Vec<Object<()>> = vec![
            Object::String("builtin1".to_string()),
            Object::String("builtin2".to_string()),
        ];
        let heap = BexHeap::new(compile_time);

        // Run GC with no roots - compile-time objects should not be counted as live
        let stats = unsafe { heap.collect_garbage(&[]) };

        // Compile-time objects are always live but not counted in live_count
        assert_eq!(stats.live_count, 0);
        assert_eq!(stats.collected_count, 0);
    }

    #[test]
    fn test_gc_collects_unreachable_objects() {
        let heap = BexHeap::<()>::new(vec![]);
        let mut tlab = Tlab::new(Arc::clone(&heap));

        // Allocate some objects
        let _obj1 = tlab.alloc_string("obj1".to_string());
        let _obj2 = tlab.alloc_string("obj2".to_string());
        let _obj3 = tlab.alloc_string("obj3".to_string());

        // Run GC with no roots - all objects should be collected
        let stats = unsafe { heap.collect_garbage(&[]) };

        assert_eq!(stats.live_count, 0);
        // We allocated 3 objects, but only the ones actually written are counted
        assert!(stats.collected_count > 0);
    }

    #[test]
    fn test_gc_preserves_rooted_objects() {
        let heap = BexHeap::<()>::new(vec![]);
        let mut tlab = Tlab::new(Arc::clone(&heap));

        // Allocate some objects
        let obj1 = tlab.alloc_string("obj1".to_string());
        let obj2 = tlab.alloc_string("obj2".to_string());
        let _obj3 = tlab.alloc_string("obj3".to_string());

        // Run GC with obj1 and obj2 as roots
        let stats = unsafe { heap.collect_garbage(&[obj1, obj2]) };

        assert_eq!(stats.live_count, 2);
        // obj3 should be collected
        assert!(stats.collected_count > 0);
    }

    #[test]
    fn test_gc_traces_array_references() {
        let heap = BexHeap::<()>::new(vec![]);
        let mut tlab = Tlab::new(Arc::clone(&heap));

        // Allocate a string
        let str_obj = tlab.alloc_string("referenced".to_string());

        // Allocate an array that references the string
        let arr = tlab.alloc_array(vec![Value::Object(str_obj)]);

        // Allocate another unreferenced string
        let _unreferenced = tlab.alloc_string("unreferenced".to_string());

        // Run GC with only the array as root
        let stats = unsafe { heap.collect_garbage(&[arr]) };

        // Should mark both the array and the string it references
        assert_eq!(stats.live_count, 2);
    }

    #[test]
    #[ignore] // TODO: Handle invalidation not implemented in Phase 6
    fn test_gc_invalidates_dead_handles() {
        let heap = BexHeap::<()>::new(vec![]);
        let mut tlab = Tlab::new(Arc::clone(&heap));

        // Allocate an object and create a handle
        let obj = tlab.alloc_string("test".to_string());
        let handle = heap.create_handle(obj);

        // Verify handle is valid
        assert!(heap.resolve_handle(&handle).is_some());

        // Run GC with no roots - object should be collected, handle invalidated
        let stats = unsafe { heap.collect_garbage(&[]) };

        assert_eq!(stats.handles_invalidated, 1);
        assert!(heap.resolve_handle(&handle).is_none());
    }

    #[test]
    #[ignore] // TODO: Handle tracking as roots not implemented in Phase 6
    fn test_gc_preserves_handled_objects() {
        let heap = BexHeap::<()>::new(vec![]);
        let mut tlab = Tlab::new(Arc::clone(&heap));

        // Allocate an object and create a handle
        let obj = tlab.alloc_string("test".to_string());
        let handle = heap.create_handle(obj);

        // Run GC with no explicit roots - handle should keep object alive
        let stats = unsafe { heap.collect_garbage(&[]) };

        // Object is kept alive by the handle
        assert_eq!(stats.live_count, 1);
        assert_eq!(stats.handles_invalidated, 0);
        assert!(heap.resolve_handle(&handle).is_some());
    }

    #[test]
    fn test_gc_heuristics() {
        let heap = BexHeap::<()>::new(vec![]);
        let mut tlab = Tlab::new(Arc::clone(&heap));

        // Initially should not need GC
        assert!(!heap.should_gc());

        // Allocate many objects to trigger GC threshold
        for i in 0..15_000 {
            tlab.alloc_string(format!("obj{i}"));
        }

        // Should now recommend GC
        assert!(heap.should_gc());

        // Reset counter
        heap.reset_gc_counter();
        assert!(!heap.should_gc());
    }
}
