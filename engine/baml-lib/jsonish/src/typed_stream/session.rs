//! Parse Session
//!
//! Maintains state for an ongoing parse operation, including the frame stack,
//! recursion tracking, and buffer management.

use std::collections::HashMap;

use super::frames::{Frame, ParsedValue};
use super::schema_index::TypeId;

/// Per-parse state
pub struct ParseSession {
    /// Input buffer (append-only)
    pub buffer: String,
    /// Byte offset of last processed position
    pub scan_offset: usize,
    /// Frame stack
    pub stack: Vec<Frame>,
    /// Recursion tracking: (TypeId, depth) -> visit count
    pub recursion_visits: HashMap<(TypeId, usize), u32>,
    /// Maximum allowed depth
    pub max_depth: usize,
    /// Maximum visits per type per depth
    pub max_visits_per_type: u32,
    /// Final parsed result (when complete)
    pub result: Option<ParsedValue>,
    /// Whether we're in streaming mode
    pub streaming: bool,
}

impl ParseSession {
    /// Create a new session with the given root frame
    pub fn new(root_frame: Frame) -> Self {
        ParseSession {
            buffer: String::new(),
            scan_offset: 0,
            stack: vec![root_frame],
            recursion_visits: HashMap::new(),
            max_depth: 64,
            max_visits_per_type: 16,
            result: None,
            streaming: false,
        }
    }

    /// Create a new session in streaming mode
    pub fn new_streaming(root_frame: Frame) -> Self {
        let mut session = Self::new(root_frame);
        session.streaming = true;
        session
    }

    /// Append input to the buffer
    pub fn append(&mut self, chunk: &str) {
        self.buffer.push_str(chunk);
    }

    /// Get the current (topmost) frame
    pub fn current_frame(&self) -> Option<&Frame> {
        self.stack.last()
    }

    /// Get mutable reference to current frame
    pub fn current_frame_mut(&mut self) -> Option<&mut Frame> {
        self.stack.last_mut()
    }

    /// Push a new frame onto the stack
    pub fn push_frame(&mut self, frame: Frame) -> bool {
        if self.stack.len() >= self.max_depth {
            return false; // Depth limit exceeded
        }
        self.stack.push(frame);
        true
    }

    /// Pop the current frame from the stack
    pub fn pop_frame(&mut self) -> Option<Frame> {
        self.stack.pop()
    }

    /// Get the current stack depth
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Check and record recursion visit
    ///
    /// Returns `true` if the visit is allowed, `false` if recursion limit exceeded
    pub fn check_recursion(&mut self, type_id: TypeId) -> bool {
        let depth = self.stack.len();
        let key = (type_id, depth);
        let count = self.recursion_visits.entry(key).or_insert(0);
        *count += 1;
        *count <= self.max_visits_per_type
    }

    /// Check if we're at the root level
    pub fn is_root(&self) -> bool {
        self.stack.len() == 1
    }

    /// Get all frames (for debugging)
    pub fn frames(&self) -> &[Frame] {
        &self.stack
    }

    /// Set the final result
    pub fn set_result(&mut self, value: ParsedValue) {
        self.result = Some(value);
    }

    /// Take the result (consumes it)
    pub fn take_result(&mut self) -> Option<ParsedValue> {
        self.result.take()
    }

    /// Check if parsing is complete
    pub fn is_complete(&self) -> bool {
        self.result.is_some() || self.stack.is_empty()
    }
}

/// Configuration for parse sessions
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum stack depth
    pub max_depth: usize,
    /// Maximum visits per type at each depth
    pub max_visits_per_type: u32,
    /// Beam width for union handling
    pub beam_k: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig {
            max_depth: 64,
            max_visits_per_type: 16,
            beam_k: 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed_stream::expected_set::ExpectedTypeSet;
    use crate::typed_stream::frames::ObjectFrame;

    #[test]
    fn test_session_creation() {
        let frame = Frame::Object(ObjectFrame::new(ExpectedTypeSet::single(0)));
        let session = ParseSession::new(frame);

        assert_eq!(session.depth(), 1);
        assert!(session.is_root());
        assert!(!session.is_complete());
    }

    #[test]
    fn test_frame_stack() {
        let frame = Frame::Object(ObjectFrame::new(ExpectedTypeSet::single(0)));
        let mut session = ParseSession::new(frame);

        // Push some frames
        for i in 1..5 {
            let f = Frame::Object(ObjectFrame::new(ExpectedTypeSet::single(i)));
            assert!(session.push_frame(f));
        }

        assert_eq!(session.depth(), 5);
        assert!(!session.is_root());

        // Pop frames
        while session.depth() > 1 {
            assert!(session.pop_frame().is_some());
        }

        assert!(session.is_root());
    }

    #[test]
    fn test_recursion_tracking() {
        let frame = Frame::Object(ObjectFrame::new(ExpectedTypeSet::single(0)));
        let mut session = ParseSession::new(frame);
        session.max_visits_per_type = 3;

        // First few visits should succeed
        assert!(session.check_recursion(42));
        assert!(session.check_recursion(42));
        assert!(session.check_recursion(42));

        // Fourth visit should fail
        assert!(!session.check_recursion(42));
    }

    #[test]
    fn test_depth_limit() {
        let frame = Frame::Object(ObjectFrame::new(ExpectedTypeSet::single(0)));
        let mut session = ParseSession::new(frame);
        session.max_depth = 5;

        // Should be able to push up to depth limit
        for i in 1..5 {
            let f = Frame::Object(ObjectFrame::new(ExpectedTypeSet::single(i)));
            assert!(session.push_frame(f));
        }

        // Next push should fail
        let f = Frame::Object(ObjectFrame::new(ExpectedTypeSet::single(99)));
        assert!(!session.push_frame(f));
    }
}
