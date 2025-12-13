//! Core Parser
//!
//! Type-directed parser that uses bounded beam search for union handling.
//! Incrementally parses JSON-ish input while narrowing type candidates.

use std::borrow::Cow;

use baml_types::TypeIR;

use super::expected_set::{ExpectedTypeSet, DEFAULT_BEAM_K};
use super::frames::{
    ArrayFrame, CompletionState, Frame, ObjectFrame, ParsedValue, ParsedValueKind, ValueFrame,
};
use super::lexer::{Lexer, QuoteStyle, Token};
use super::schema_index::{LiteralKind, PrimitiveKind, SchemaIndex, TypeId, TypeInfo, TypeKind};
use super::session::ParseSession;
use crate::deserializer::types::BamlValueWithFlags;

/// Result of processing a chunk
pub struct ParseUpdate {
    /// Whether any tokens were processed
    pub progressed: bool,
    /// Suggested next keys to look for
    pub next_keys: Vec<KeyHint>,
}

/// Hint for expected keys in current context
#[derive(Debug, Clone)]
pub struct KeyHint {
    pub key: String,
    pub required: bool,
    pub discriminative: bool, // True if only some union variants have this key
}

/// The typed stream parser
pub struct TypedStreamParser {
    pub schema: SchemaIndex,
    beam_k: usize,
}

impl TypedStreamParser {
    /// Create a new parser for the given root type
    pub fn new(root: &TypeIR, beam_k: usize) -> Self {
        let schema = SchemaIndex::build(root, None);
        TypedStreamParser { schema, beam_k }
    }

    /// Create a new parser with context from OutputFormatContent
    pub fn new_with_context(
        root: &TypeIR,
        of: &internal_baml_jinja::types::OutputFormatContent,
        beam_k: usize,
    ) -> Self {
        let schema = SchemaIndex::build(root, Some(of));
        TypedStreamParser { schema, beam_k }
    }

    /// Create a new parse session
    pub fn new_session(&self) -> ParseSession {
        let root_id = self.schema.root_id();
        let root_frame = self.frame_for_type_id(root_id);
        ParseSession::new(root_frame)
    }

    /// Create a frame for a given type ID
    ///
    /// For field values, we generally want to use Value frames that transition to the
    /// appropriate frame type when we see the opening token (LBrace for objects, LBracket for arrays).
    /// The exception is Class/Map types which always start with `{`, so we can directly use Object frames.
    fn frame_for_type_id(&self, type_id: TypeId) -> Frame {
        let info = self.schema.get(type_id);

        match info.map(|i| &i.kind) {
            Some(TypeKind::Class { .. }) => {
                // Classes always start with `{`, can use Object frame directly
                Frame::Object(ObjectFrame::new(ExpectedTypeSet::single(type_id)))
            }
            Some(TypeKind::Map { .. }) => {
                // Maps always start with `{`, can use Object frame directly
                Frame::Object(ObjectFrame::new(ExpectedTypeSet::single(type_id)))
            }
            Some(TypeKind::List { .. }) | Some(TypeKind::Tuple { .. }) => {
                // Lists/tuples need to wait for `[` token to confirm array context
                // Use Value frame which will transition to Array when `[` is seen
                Frame::Value(ValueFrame::new(ExpectedTypeSet::single(type_id)))
            }
            Some(TypeKind::Union { variants, .. }) => {
                Frame::Value(ValueFrame::new(ExpectedTypeSet::from_union(
                    variants,
                    self.beam_k,
                )))
            }
            Some(TypeKind::Optional { inner }) => self.frame_for_type_id(*inner),
            Some(TypeKind::RecursiveAlias { target: Some(inner), .. }) => {
                // Recursive aliases delegate to their resolved inner type
                self.frame_for_type_id(*inner)
            }
            _ => Frame::Value(ValueFrame::new(ExpectedTypeSet::single(type_id))),
        }
    }

    /// Process a chunk of input
    pub fn ingest(&self, session: &mut ParseSession, chunk: &str) -> anyhow::Result<ParseUpdate> {
        session.append(chunk);

        let mut lexer = Lexer::new();
        lexer.append(&session.buffer[session.scan_offset..]);

        let tokens = lexer.drain_tokens();
        let mut progressed = false;

        for tok in tokens {
            if self.process_token(session, tok)? {
                progressed = true;
            }
        }

        // Update scan offset
        session.scan_offset = session.buffer.len()
            - lexer
                .incomplete_token()
                .map(|_| 0)
                .unwrap_or(session.buffer.len() - session.scan_offset);

        Ok(ParseUpdate {
            progressed,
            next_keys: self.next_keys(session),
        })
    }

    /// Process a single token
    fn process_token(&self, session: &mut ParseSession, tok: Token<'static>) -> anyhow::Result<bool> {
        let Some(frame) = session.current_frame_mut() else {
            // No frame - store value at root
            let value = self.token_to_value(&tok);
            session.set_result(value);
            return Ok(true);
        };

        match frame {
            Frame::Object(ref mut obj) => self.process_object_token(session, tok),
            Frame::Array(ref mut arr) => self.process_array_token(session, tok),
            Frame::Value(ref mut val) => self.process_value_token(session, tok),
        }
    }

    /// Process token in object context
    fn process_object_token(
        &self,
        session: &mut ParseSession,
        tok: Token<'static>,
    ) -> anyhow::Result<bool> {
        // Copy streaming flag before borrowing frame mutably
        let streaming = session.streaming;

        // Get object frame mutably
        let frame = session.current_frame_mut().unwrap();
        let Frame::Object(obj) = frame else {
            return Ok(false);
        };

        match tok {
            Token::RBrace => {
                // Close object
                obj.closed = true;
                obj.completion = obj.derive_completion();

                // Pop frame and propagate value
                let frame = session.pop_frame().unwrap();
                if let Frame::Object(obj) = frame {
                    let value = ParsedValue::object(obj.fields, obj.closed, obj.expected.best());
                    self.propagate_value(session, value)?;
                }
                Ok(true)
            }

            Token::String { content, .. } if obj.expecting_key || obj.pending_key.is_none() => {
                // This is a key
                let key = content.trim().to_string();
                obj.pending_key = Some(key.clone());
                obj.seen_keys.insert(key.clone());
                obj.expecting_key = false;

                // Narrow union candidates by observed key
                obj.expected.observe_key(&self.schema, &key, streaming);
                Ok(true)
            }

            // Accept numbers as keys (for maps and JSON with numeric keys)
            Token::Number { raw, .. } if obj.expecting_key || obj.pending_key.is_none() => {
                let key = raw.to_string();
                obj.pending_key = Some(key.clone());
                obj.seen_keys.insert(key.clone());
                obj.expecting_key = false;
                Ok(true)
            }

            // Accept true/false/null as keys (for JSON with these as keys)
            Token::True if obj.expecting_key || obj.pending_key.is_none() => {
                let key = "true".to_string();
                obj.pending_key = Some(key.clone());
                obj.seen_keys.insert(key.clone());
                obj.expecting_key = false;
                Ok(true)
            }
            Token::False if obj.expecting_key || obj.pending_key.is_none() => {
                let key = "false".to_string();
                obj.pending_key = Some(key.clone());
                obj.seen_keys.insert(key.clone());
                obj.expecting_key = false;
                Ok(true)
            }
            Token::Null if obj.expecting_key || obj.pending_key.is_none() => {
                let key = "null".to_string();
                obj.pending_key = Some(key.clone());
                obj.seen_keys.insert(key.clone());
                obj.expecting_key = false;
                Ok(true)
            }

            Token::Colon => {
                // After colon, expect value
                if let Some(key) = obj.pending_key.clone() {
                    // Push frame for field value
                    let field_type_id = self.resolve_field_type(&obj.expected, &key);
                    if let Some(type_id) = field_type_id {
                        let field_frame = self.frame_for_type_id(type_id);
                        session.push_frame(field_frame);
                    } else {
                        // Unknown key - push generic value frame
                        session.push_frame(Frame::Value(ValueFrame::new(ExpectedTypeSet::single(
                            self.schema.root_id(),
                        ))));
                    }
                }
                Ok(true)
            }

            Token::Comma => {
                // Between fields
                let frame = session.current_frame_mut().unwrap();
                if let Frame::Object(obj) = frame {
                    obj.pending_key = None;
                    obj.expecting_key = true;
                }
                Ok(true)
            }

            _ => {
                // Value token when we have a pending key
                let frame = session.current_frame_mut().unwrap();
                if let Frame::Object(obj) = frame {
                    if obj.pending_key.is_some() {
                        let value = self.token_to_value(&tok);
                        let key = obj.pending_key.take().unwrap();
                        obj.add_field(key, value);
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    /// Process token in array context
    fn process_array_token(
        &self,
        session: &mut ParseSession,
        tok: Token<'static>,
    ) -> anyhow::Result<bool> {
        let frame = session.current_frame_mut().unwrap();
        let Frame::Array(arr) = frame else {
            return Ok(false);
        };

        match tok {
            Token::RBracket => {
                // Close array
                arr.closed = true;
                arr.completion = arr.derive_completion();

                // Pop frame and propagate value
                let frame = session.pop_frame().unwrap();
                if let Frame::Array(arr) = frame {
                    let value = ParsedValue::array(arr.elements, arr.closed);
                    self.propagate_value(session, value)?;
                }
                Ok(true)
            }

            Token::Comma => {
                // Between elements
                let frame = session.current_frame_mut().unwrap();
                if let Frame::Array(arr) = frame {
                    arr.expecting_element = true;
                }
                Ok(true)
            }

            Token::LBrace => {
                // Nested object
                let frame = session.current_frame_mut().unwrap();
                if let Frame::Array(arr) = frame {
                    let elem_type = arr.expected_elem.best().unwrap_or(self.schema.root_id());
                    let obj_frame = self.frame_for_type_id(elem_type);
                    session.push_frame(obj_frame);
                }
                Ok(true)
            }

            Token::LBracket => {
                // Nested array - create Array frame directly since we already consumed the `[`
                let frame = session.current_frame_mut().unwrap();
                if let Frame::Array(arr) = frame {
                    let elem_expected = arr.expected_elem.clone();

                    // Get the nested element type by resolving through the current element type
                    let nested_elem = elem_expected
                        .best()
                        .and_then(|id| self.schema.list_element(id));

                    // Create array frame directly (we already saw the `[`)
                    let arr_frame = Frame::Array(ArrayFrame::new(
                        nested_elem
                            .map(ExpectedTypeSet::single)
                            .unwrap_or_else(|| elem_expected),
                    ));
                    session.push_frame(arr_frame);
                }
                Ok(true)
            }

            _ => {
                // Element value
                let value = self.token_to_value(&tok);
                let frame = session.current_frame_mut().unwrap();
                if let Frame::Array(arr) = frame {
                    arr.add_element(value);
                    arr.expecting_element = false;
                }
                Ok(true)
            }
        }
    }

    /// Process token in value context
    fn process_value_token(
        &self,
        session: &mut ParseSession,
        tok: Token<'static>,
    ) -> anyhow::Result<bool> {
        match &tok {
            Token::LBrace => {
                // Start object
                let frame = session.current_frame_mut().unwrap();
                let expected = frame.expected().clone();

                // Narrow to object-like types
                let mut narrowed = expected.clone();
                narrowed.narrow_by_structure(&self.schema, true);

                // Replace current frame with object frame
                session.pop_frame();
                let obj_frame = Frame::Object(ObjectFrame::new(narrowed));
                session.push_frame(obj_frame);
                Ok(true)
            }

            Token::LBracket => {
                // Start array
                let frame = session.current_frame_mut().unwrap();
                let expected = frame.expected().clone();

                // Narrow to array-like types
                let mut narrowed = expected.clone();
                narrowed.narrow_by_structure(&self.schema, false);

                // Get element type
                let elem_type = narrowed.best().and_then(|id| self.schema.list_element(id));

                session.pop_frame();
                let arr_frame = Frame::Array(ArrayFrame::new(
                    elem_type
                        .map(ExpectedTypeSet::single)
                        .unwrap_or_else(|| narrowed),
                ));
                session.push_frame(arr_frame);
                Ok(true)
            }

            _ => {
                // Primitive value
                let value = self.token_to_value(&tok);
                session.pop_frame();
                self.propagate_value(session, value)?;
                Ok(true)
            }
        }
    }

    /// Convert a token to a parsed value
    fn token_to_value(&self, tok: &Token<'static>) -> ParsedValue {
        match tok {
            Token::String {
                content, complete, ..
            } => ParsedValue::string(content.to_string(), *complete),
            Token::Number { raw, complete } => {
                // Strip thousand separators (commas) before parsing
                let cleaned: String = raw.chars().filter(|c| *c != ',').collect();
                // Try to parse as integer first, then float
                if let Ok(i) = cleaned.parse::<i64>() {
                    ParsedValue::int(i, *complete)
                } else if let Ok(f) = cleaned.parse::<f64>() {
                    ParsedValue::float(f, *complete)
                } else {
                    // Keep as string if parsing fails
                    ParsedValue::string(raw.to_string(), *complete)
                }
            }
            Token::True => ParsedValue::bool(true),
            Token::False => ParsedValue::bool(false),
            Token::Null => ParsedValue::null(true),
            Token::CodeBlock {
                content, complete, ..
            } => ParsedValue::string(content.to_string(), *complete),
            _ => ParsedValue::pending(),
        }
    }

    /// Propagate a value up to the parent frame
    fn propagate_value(
        &self,
        session: &mut ParseSession,
        value: ParsedValue,
    ) -> anyhow::Result<()> {
        match session.current_frame_mut() {
            Some(Frame::Object(obj)) => {
                if let Some(key) = obj.pending_key.take() {
                    obj.add_field(key, value);
                }
            }
            Some(Frame::Array(arr)) => {
                arr.add_element(value);
                arr.expecting_element = false;
            }
            Some(Frame::Value(val)) => {
                val.set_value(value);
            }
            None => {
                session.set_result(value);
            }
        }
        Ok(())
    }

    /// Resolve field type for a key in the expected type set
    fn resolve_field_type(&self, expected: &ExpectedTypeSet, key: &str) -> Option<TypeId> {
        for type_id in expected.all_candidates() {
            if let Some(info) = self.schema.get(type_id) {
                if let TypeKind::Class { fields, .. } = &info.kind {
                    // Fields are keyed by rendered_name
                    if let Some(field) = fields.get(key) {
                        return Some(field.type_id);
                    }
                }
                // For maps, return value type
                if let TypeKind::Map { value, .. } = &info.kind {
                    return Some(*value);
                }
            }
        }
        None
    }

    /// Compute next expected keys for current object frame
    pub fn next_keys(&self, session: &ParseSession) -> Vec<KeyHint> {
        let frame = match session.current_frame() {
            Some(Frame::Object(obj)) => obj,
            _ => return vec![],
        };

        let mut hints = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();

        for type_id in frame.expected.all_candidates() {
            if let Some(info) = self.schema.get(type_id) {
                if let TypeKind::Class {
                    fields, required, ..
                } = &info.kind
                {
                    for (key, field) in fields {
                        if frame.seen_keys.contains(key) {
                            continue;
                        }
                        if seen_keys.contains(key) {
                            continue;
                        }
                        seen_keys.insert(key.clone());

                        hints.push(KeyHint {
                            key: key.clone(),
                            required: required.contains(key),
                            discriminative: self.is_discriminative(&frame.expected, key),
                        });
                    }
                }
            }
        }

        // Sort: required first, then discriminative, then alphabetical
        hints.sort_by(|a, b| {
            match (a.required, b.required) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (a.discriminative, b.discriminative) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.key.cmp(&b.key),
                },
            }
        });

        hints
    }

    /// Check if a key is discriminative (not in all union variants)
    fn is_discriminative(&self, expected: &ExpectedTypeSet, key: &str) -> bool {
        let mut has_key_count = 0;
        let mut total_count = 0;

        for type_id in expected.all_candidates() {
            total_count += 1;
            if let Some(info) = self.schema.get(type_id) {
                if let TypeKind::Class { fields, .. } = &info.kind {
                    // Fields are keyed by rendered_name
                    if fields.contains_key(key) {
                        has_key_count += 1;
                    }
                }
            }
        }

        // Discriminative if not all candidates have the key
        has_key_count > 0 && has_key_count < total_count
    }

    /// Finalize and produce BamlValueWithFlags
    pub fn finish(
        &self,
        session: &ParseSession,
        streaming: bool,
    ) -> anyhow::Result<BamlValueWithFlags> {
        // Get the result value
        let value = if let Some(ref result) = session.result {
            result.clone()
        } else if !session.stack.is_empty() {
            // Build value from the entire stack (handles incomplete nested structures)
            self.build_value_from_stack(&session.stack)
        } else {
            return Err(anyhow::anyhow!("No result available"));
        };

        // Convert to BamlValueWithFlags
        super::coerce::convert_to_baml_value(&self.schema, self.schema.root_id(), &value, streaming)
    }

    /// Build a parsed value from the entire frame stack
    /// This handles incomplete nested structures by collapsing the stack bottom-up
    fn build_value_from_stack(&self, stack: &[Frame]) -> ParsedValue {
        if stack.is_empty() {
            return ParsedValue::pending();
        }

        // Start from the deepest (last) frame and work back up
        let mut current_value: Option<ParsedValue> = None;

        for frame in stack.iter().rev() {
            match frame {
                Frame::Object(obj) => {
                    let mut fields = obj.fields.clone();

                    // If there's a pending key and we have a value from deeper in the stack,
                    // add it to the fields
                    if let (Some(key), Some(val)) = (&obj.pending_key, current_value.take()) {
                        fields.push((key.clone(), val));
                    }

                    current_value = Some(ParsedValue::object(fields, obj.closed, obj.expected.best()));
                }
                Frame::Array(arr) => {
                    let mut elements = arr.elements.clone();

                    // If we have a pending element from deeper in the stack, add it
                    if let Some(val) = current_value.take() {
                        if arr.expecting_element || elements.is_empty() {
                            elements.push(val);
                        }
                    }

                    current_value = Some(ParsedValue::array(elements, arr.closed));
                }
                Frame::Value(val) => {
                    // Use the value if present, otherwise use what we built from deeper frames
                    if let Some(v) = &val.value {
                        current_value = Some(v.clone());
                    } else if current_value.is_none() {
                        current_value = Some(ParsedValue::pending());
                    }
                }
            }
        }

        current_value.unwrap_or_else(ParsedValue::pending)
    }

    /// Build a parsed value from the current frame state (for single frame)
    fn build_value_from_frame(&self, frame: &Frame, _streaming: bool) -> ParsedValue {
        match frame {
            Frame::Object(obj) => {
                ParsedValue::object(obj.fields.clone(), obj.closed, obj.expected.best())
            }
            Frame::Array(arr) => ParsedValue::array(arr.elements.clone(), arr.closed),
            Frame::Value(val) => val.value.clone().unwrap_or_else(ParsedValue::pending),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baml_types::{type_meta, TypeValue, StreamingMode};

    #[test]
    fn test_simple_object() {
        let ty = TypeIR::Class {
            name: "Test".to_string(),
            mode: StreamingMode::NonStreaming,
            dynamic: false,
            meta: type_meta::IR::default(),
        };
        let parser = TypedStreamParser::new(&ty, 8);
        let mut session = parser.new_session();

        parser.ingest(&mut session, r#"{"key": "value"}"#).unwrap();

        assert!(session.result.is_some() || session.current_frame().is_some());
    }

    #[test]
    fn test_simple_array() {
        let elem = Box::new(TypeIR::Primitive(TypeValue::Int, type_meta::IR::default()));
        let ty = TypeIR::List(elem, type_meta::IR::default());
        let parser = TypedStreamParser::new(&ty, 8);
        let mut session = parser.new_session();

        parser.ingest(&mut session, r#"[1, 2, 3]"#).unwrap();

        assert!(session.result.is_some() || session.current_frame().is_some());
    }

    #[test]
    fn test_nested_object() {
        let ty = TypeIR::Class {
            name: "Outer".to_string(),
            mode: StreamingMode::NonStreaming,
            dynamic: false,
            meta: type_meta::IR::default(),
        };
        let parser = TypedStreamParser::new(&ty, 8);
        let mut session = parser.new_session();

        parser
            .ingest(&mut session, r#"{"inner": {"nested": true}}"#)
            .unwrap();
    }

    #[test]
    fn test_streaming_partial() {
        let ty = TypeIR::Class {
            name: "Test".to_string(),
            mode: StreamingMode::Streaming,
            dynamic: false,
            meta: type_meta::IR::default(),
        };
        let parser = TypedStreamParser::new(&ty, 8);
        let mut session = ParseSession::new_streaming(parser.frame_for_type_id(parser.schema.root_id()));

        // Send partial input
        parser.ingest(&mut session, r#"{"key": "val"#).unwrap();

        // Should have partial state
        if let Some(Frame::Object(obj)) = session.current_frame() {
            assert!(obj.seen_keys.contains("key"));
        }
    }
}
