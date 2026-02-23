//! VM debugger data structures.

use bex_vm_types::{StackIndex, Value, bytecode::LineTableEntry};

/// User breakpoint expressed in source coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DebugBreakpoint {
    /// Numeric file identifier (`Span.file_id.as_u32()`).
    pub file_id: u32,
    /// 1-indexed source line.
    pub line: usize,
}

/// Current debugger stepping mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DebugStepMode {
    /// Run freely unless a breakpoint or pause request hits.
    #[default]
    Continue,
    /// Stop at the next sequence point in any frame.
    StepIn,
    /// Stop at the next sequence point in the current frame (or after unwinding).
    StepOver { start_depth: usize },
    /// Stop at the next sequence point after unwinding at least one frame.
    StepOut { start_depth: usize },
}

/// Why the VM yielded a debug stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugStopReason {
    Breakpoint,
    Step,
    Pause,
}

/// VM stop snapshot for debugger consumers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugStop {
    pub reason: DebugStopReason,
    pub frame_depth: usize,
    pub function_name: String,
    pub pc: usize,
    pub line_entry: LineTableEntry,
}

/// Frame view for debugger stack traces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugStackFrame {
    pub frame_depth: usize,
    pub function_name: String,
    pub pc: usize,
    pub line_entry: Option<LineTableEntry>,
}

/// In-scope local bound to a concrete stack slot/value.
#[derive(Clone, Debug, PartialEq)]
pub struct DebugScopedLocal {
    pub name: String,
    pub slot: usize,
    pub stack_index: StackIndex,
    pub value: Value,
}
