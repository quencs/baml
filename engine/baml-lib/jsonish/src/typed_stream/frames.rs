//! Parse Frames and Completion State
//!
//! Defines the frame types for tracking parse state at different nesting levels,
//! along with CompletionState for streaming support.

use std::collections::HashSet;

use super::expected_set::ExpectedTypeSet;
use super::schema_index::TypeId;

/// Completion state for values during streaming
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionState {
    /// Value is fully observed and syntactically closed
    Complete,
    /// Value is present but syntactically incomplete (unterminated string, unclosed brace)
    Incomplete,
    /// Value has not yet appeared in the stream (placeholder)
    Pending,
}

impl Default for CompletionState {
    fn default() -> Self {
        CompletionState::Pending
    }
}

impl CompletionState {
    /// Combine two completion states (for parent-child relationships)
    /// Parent is Complete only if all children are Complete
    pub fn combine(self, other: Self) -> Self {
        match (self, other) {
            (CompletionState::Complete, CompletionState::Complete) => CompletionState::Complete,
            (CompletionState::Pending, _) | (_, CompletionState::Pending) => {
                CompletionState::Incomplete
            }
            _ => CompletionState::Incomplete,
        }
    }

    /// Check if this state indicates the value is "done" (not pending)
    pub fn is_observed(&self) -> bool {
        !matches!(self, CompletionState::Pending)
    }
}

/// Frame types for the parse stack
#[derive(Debug, Clone)]
pub enum Frame {
    Object(ObjectFrame),
    Array(ArrayFrame),
    Value(ValueFrame),
}

impl Frame {
    /// Get the expected type set for this frame
    pub fn expected(&self) -> &ExpectedTypeSet {
        match self {
            Frame::Object(obj) => &obj.expected,
            Frame::Array(arr) => &arr.expected_elem,
            Frame::Value(val) => &val.expected,
        }
    }

    /// Get mutable reference to expected type set
    pub fn expected_mut(&mut self) -> &mut ExpectedTypeSet {
        match self {
            Frame::Object(obj) => &mut obj.expected,
            Frame::Array(arr) => &mut arr.expected_elem,
            Frame::Value(val) => &mut val.expected,
        }
    }

    /// Get the completion state
    pub fn completion(&self) -> CompletionState {
        match self {
            Frame::Object(obj) => obj.completion,
            Frame::Array(arr) => arr.completion,
            Frame::Value(val) => val.completion,
        }
    }
}

/// Frame for parsing objects/classes
#[derive(Debug, Clone)]
pub struct ObjectFrame {
    /// Which types we might be parsing
    pub expected: ExpectedTypeSet,
    /// Keys we've seen
    pub seen_keys: HashSet<String>,
    /// Current key being parsed (after `:` but before value)
    pub pending_key: Option<String>,
    /// Parsed field values so far (with completion state per field)
    pub fields: Vec<(String, ParsedValue)>,
    /// Whether we've seen the closing `}`
    pub closed: bool,
    /// Tracks whether the object itself is complete
    pub completion: CompletionState,
    /// Whether we're in "key position" (expecting a key or })
    pub expecting_key: bool,
}

impl ObjectFrame {
    pub fn new(expected: ExpectedTypeSet) -> Self {
        ObjectFrame {
            expected,
            seen_keys: HashSet::new(),
            pending_key: None,
            fields: Vec::new(),
            closed: false,
            completion: CompletionState::Incomplete, // Start as incomplete until `}` seen
            expecting_key: true,
        }
    }

    /// Derive completion state from children
    pub fn derive_completion(&self) -> CompletionState {
        if !self.closed {
            return CompletionState::Incomplete;
        }
        // Object is complete only if all fields are complete
        if self
            .fields
            .iter()
            .all(|(_, v)| v.completion == CompletionState::Complete)
        {
            CompletionState::Complete
        } else {
            CompletionState::Incomplete
        }
    }

    /// Add a field to the object
    pub fn add_field(&mut self, key: String, value: ParsedValue) {
        self.fields.push((key.clone(), value));
        self.seen_keys.insert(key);
        self.pending_key = None;
        self.expecting_key = true;
    }

    /// Get field by name
    pub fn get_field(&self, key: &str) -> Option<&ParsedValue> {
        self.fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

/// Frame for parsing arrays/lists
#[derive(Debug, Clone)]
pub struct ArrayFrame {
    /// Element type
    pub expected_elem: ExpectedTypeSet,
    /// Current element index
    pub index: usize,
    /// Parsed elements (with completion state per element)
    pub elements: Vec<ParsedValue>,
    /// Whether we've seen the closing `]`
    pub closed: bool,
    /// Tracks whether the array itself is complete
    pub completion: CompletionState,
    /// Whether we're expecting a new element (vs. comma or ])
    pub expecting_element: bool,
}

impl ArrayFrame {
    pub fn new(expected_elem: ExpectedTypeSet) -> Self {
        ArrayFrame {
            expected_elem,
            index: 0,
            elements: Vec::new(),
            closed: false,
            completion: CompletionState::Incomplete,
            expecting_element: true,
        }
    }

    /// Derive completion state from children
    pub fn derive_completion(&self) -> CompletionState {
        if !self.closed {
            return CompletionState::Incomplete;
        }
        if self
            .elements
            .iter()
            .all(|v| v.completion == CompletionState::Complete)
        {
            CompletionState::Complete
        } else {
            CompletionState::Incomplete
        }
    }

    /// Add an element to the array
    pub fn add_element(&mut self, value: ParsedValue) {
        self.elements.push(value);
        self.index += 1;
        self.expecting_element = false;
    }
}

/// Frame for parsing simple values
#[derive(Debug, Clone)]
pub struct ValueFrame {
    /// Which types we might be parsing
    pub expected: ExpectedTypeSet,
    /// Completion state of the value being parsed
    pub completion: CompletionState,
    /// The parsed value (if any)
    pub value: Option<ParsedValue>,
}

impl ValueFrame {
    pub fn new(expected: ExpectedTypeSet) -> Self {
        ValueFrame {
            expected,
            completion: CompletionState::Pending,
            value: None,
        }
    }

    /// Set the parsed value
    pub fn set_value(&mut self, value: ParsedValue) {
        self.completion = value.completion;
        self.value = Some(value);
    }
}

/// Intermediate parsed value WITH completion state
#[derive(Debug, Clone)]
pub struct ParsedValue {
    pub value: ParsedValueKind,
    pub completion: CompletionState,
    /// Resolved type ID (if known)
    pub type_id: Option<TypeId>,
}

/// Kinds of parsed values
#[derive(Debug, Clone)]
pub enum ParsedValueKind {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Object {
        type_id: Option<TypeId>,
        fields: Vec<(String, ParsedValue)>,
    },
    Array(Vec<ParsedValue>),
    /// Placeholder for unparsed/pending values
    Placeholder,
}

impl ParsedValue {
    /// Create a complete value
    pub fn complete(value: ParsedValueKind) -> Self {
        ParsedValue {
            value,
            completion: CompletionState::Complete,
            type_id: None,
        }
    }

    /// Create an incomplete value
    pub fn incomplete(value: ParsedValueKind) -> Self {
        ParsedValue {
            value,
            completion: CompletionState::Incomplete,
            type_id: None,
        }
    }

    /// Create a pending (placeholder) value
    pub fn pending() -> Self {
        ParsedValue {
            value: ParsedValueKind::Placeholder,
            completion: CompletionState::Pending,
            type_id: None,
        }
    }

    /// Create a null value
    pub fn null(complete: bool) -> Self {
        ParsedValue {
            value: ParsedValueKind::Null,
            completion: if complete {
                CompletionState::Complete
            } else {
                CompletionState::Incomplete
            },
            type_id: None,
        }
    }

    /// Create a boolean value
    pub fn bool(v: bool) -> Self {
        ParsedValue {
            value: ParsedValueKind::Bool(v),
            completion: CompletionState::Complete,
            type_id: None,
        }
    }

    /// Create an integer value
    pub fn int(v: i64, complete: bool) -> Self {
        ParsedValue {
            value: ParsedValueKind::Int(v),
            completion: if complete {
                CompletionState::Complete
            } else {
                CompletionState::Incomplete
            },
            type_id: None,
        }
    }

    /// Create a float value
    pub fn float(v: f64, complete: bool) -> Self {
        ParsedValue {
            value: ParsedValueKind::Float(v),
            completion: if complete {
                CompletionState::Complete
            } else {
                CompletionState::Incomplete
            },
            type_id: None,
        }
    }

    /// Create a string value
    pub fn string(s: String, complete: bool) -> Self {
        ParsedValue {
            value: ParsedValueKind::String(s),
            completion: if complete {
                CompletionState::Complete
            } else {
                CompletionState::Incomplete
            },
            type_id: None,
        }
    }

    /// Create an object value
    pub fn object(fields: Vec<(String, ParsedValue)>, complete: bool, type_id: Option<TypeId>) -> Self {
        let completion = if complete {
            // Check if all fields are complete
            if fields.iter().all(|(_, v)| v.completion == CompletionState::Complete) {
                CompletionState::Complete
            } else {
                CompletionState::Incomplete
            }
        } else {
            CompletionState::Incomplete
        };

        ParsedValue {
            value: ParsedValueKind::Object { type_id, fields },
            completion,
            type_id,
        }
    }

    /// Create an array value
    pub fn array(elements: Vec<ParsedValue>, complete: bool) -> Self {
        let completion = if complete {
            if elements.iter().all(|v| v.completion == CompletionState::Complete) {
                CompletionState::Complete
            } else {
                CompletionState::Incomplete
            }
        } else {
            CompletionState::Incomplete
        };

        ParsedValue {
            value: ParsedValueKind::Array(elements),
            completion,
            type_id: None,
        }
    }

    /// Check if this is a null value
    pub fn is_null(&self) -> bool {
        matches!(self.value, ParsedValueKind::Null)
    }

    /// Check if this is a placeholder
    pub fn is_placeholder(&self) -> bool {
        matches!(self.value, ParsedValueKind::Placeholder)
    }

    /// Get string value if this is a string
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            ParsedValueKind::String(s) => Some(s),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_state_combine() {
        assert_eq!(
            CompletionState::Complete.combine(CompletionState::Complete),
            CompletionState::Complete
        );
        assert_eq!(
            CompletionState::Complete.combine(CompletionState::Incomplete),
            CompletionState::Incomplete
        );
        assert_eq!(
            CompletionState::Complete.combine(CompletionState::Pending),
            CompletionState::Incomplete
        );
    }

    #[test]
    fn test_object_frame() {
        let mut frame = ObjectFrame::new(ExpectedTypeSet::single(0));

        frame.add_field("name".to_string(), ParsedValue::string("test".to_string(), true));
        assert!(frame.seen_keys.contains("name"));
        assert!(frame.get_field("name").is_some());

        assert_eq!(frame.derive_completion(), CompletionState::Incomplete);

        frame.closed = true;
        assert_eq!(frame.derive_completion(), CompletionState::Complete);
    }

    #[test]
    fn test_array_frame() {
        let mut frame = ArrayFrame::new(ExpectedTypeSet::single(0));

        frame.add_element(ParsedValue::int(1, true));
        frame.add_element(ParsedValue::int(2, true));

        assert_eq!(frame.elements.len(), 2);
        assert_eq!(frame.derive_completion(), CompletionState::Incomplete);

        frame.closed = true;
        assert_eq!(frame.derive_completion(), CompletionState::Complete);
    }

    #[test]
    fn test_parsed_value_constructors() {
        let v = ParsedValue::complete(ParsedValueKind::Int(42));
        assert_eq!(v.completion, CompletionState::Complete);

        let v = ParsedValue::incomplete(ParsedValueKind::String("test".to_string()));
        assert_eq!(v.completion, CompletionState::Incomplete);

        let v = ParsedValue::pending();
        assert_eq!(v.completion, CompletionState::Pending);
        assert!(v.is_placeholder());
    }
}
