//! Segment Extraction
//!
//! Identifies candidate JSON-ish spans from raw LLM output.
//! Handles markdown fenced blocks, embedded JSON objects/arrays, and fallback.

use std::ops::Range;

/// Candidate span with score
#[derive(Debug, Clone)]
pub struct CandidateSpan {
    pub range: Range<usize>,
    pub score: i32,
    pub kind: SpanKind,
}

/// Kind of span that was identified
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// ``` fenced code block
    MarkdownFence,
    /// JSON object { ... }
    GreppedObject,
    /// JSON array [ ... ]
    GreppedArray,
    /// Primitive value (true/false/null/number)
    Primitive,
    /// Full input as fallback
    FullInput,
}

/// Extract candidate JSON-ish spans from raw input
///
/// Returns spans sorted by priority (highest score first).
pub fn extract_spans(input: &str, max_spans: usize) -> Vec<CandidateSpan> {
    let mut spans = Vec::new();

    // 1) Find markdown fenced blocks (highest priority)
    spans.extend(find_markdown_fences(input));

    // 2) Find {...} spans
    spans.extend(find_json_objects(input));

    // 3) Find [...] spans
    spans.extend(find_json_arrays(input));

    // 4) Find primitive values (true/false/null/numbers)
    spans.extend(find_primitives(input));

    // 5) Fallback: full input (lowest priority)
    if spans.is_empty() || max_spans > spans.len() {
        spans.push(CandidateSpan {
            range: 0..input.len(),
            score: -100, // Low priority
            kind: SpanKind::FullInput,
        });
    }

    // Sort by score descending
    spans.sort_by_key(|s| std::cmp::Reverse(s.score));

    // Remove duplicates (same range)
    spans.dedup_by(|a, b| a.range == b.range);

    // Keep top N
    spans.truncate(max_spans);

    spans
}

/// Find markdown fenced code blocks
fn find_markdown_fences(input: &str) -> Vec<CandidateSpan> {
    let mut spans = Vec::new();
    let fence_pattern = "```";

    let mut pos = 0;
    while let Some(start_offset) = input[pos..].find(fence_pattern) {
        let abs_start = pos + start_offset;

        // Find end of opening fence line (lang tag + newline)
        let after_ticks = abs_start + 3;
        let line_end = input[after_ticks..]
            .find('\n')
            .map(|i| after_ticks + i)
            .unwrap_or(input.len());

        // Extract language tag
        let lang_tag = input[after_ticks..line_end].trim();
        let content_start = (line_end + 1).min(input.len());

        // Find closing fence
        if let Some(end_offset) = input[content_start..].find(fence_pattern) {
            let content_end = content_start + end_offset;

            // Trim trailing newline from content
            let content_end_trimmed = if content_end > 0 && input[..content_end].ends_with('\n') {
                content_end - 1
            } else {
                content_end
            };

            // Score based on language tag
            let lang_score = match lang_tag.to_lowercase().as_str() {
                "json" => 120,
                "javascript" | "js" => 110,
                "" => 100,
                _ => 90,
            };

            spans.push(CandidateSpan {
                range: content_start..content_end_trimmed,
                score: lang_score,
                kind: SpanKind::MarkdownFence,
            });

            pos = content_end + 3;
        } else {
            // Unclosed fence - treat rest as content (incomplete)
            spans.push(CandidateSpan {
                range: content_start..input.len(),
                score: 50, // Lower priority for incomplete
                kind: SpanKind::MarkdownFence,
            });
            break;
        }
    }

    spans
}

/// Find JSON object spans {...}
fn find_json_objects(input: &str) -> Vec<CandidateSpan> {
    let mut spans = Vec::new();
    let char_indices: Vec<_> = input.char_indices().collect();

    for &(pos, c) in char_indices.iter() {
        if c == '{' {
            if let Some(end) = find_matching_brace(input, pos, '{', '}') {
                // Complete object - score based on position and content
                let content_len = end - pos + 1;
                let position_score = if pos == 0 { 10 } else { 0 }; // Boost for start of input
                let size_score = (content_len.min(1000) / 100) as i32; // Bigger objects score higher

                spans.push(CandidateSpan {
                    range: pos..end + 1,
                    score: 50 + position_score + size_score,
                    kind: SpanKind::GreppedObject,
                });
            } else {
                // Incomplete object - use rest of input (for streaming)
                let position_score = if pos == 0 { 10 } else { 0 };
                spans.push(CandidateSpan {
                    range: pos..input.len(),
                    score: 30 + position_score, // Lower priority than complete objects
                    kind: SpanKind::GreppedObject,
                });
            }
        }
    }

    spans
}

/// Find JSON array spans [...]
fn find_json_arrays(input: &str) -> Vec<CandidateSpan> {
    let mut spans = Vec::new();

    for (pos, c) in input.char_indices() {
        if c == '[' {
            if let Some(end) = find_matching_brace(input, pos, '[', ']') {
                // Complete array
                let content_len = end - pos + 1;
                let position_score = if pos == 0 { 10 } else { 0 };
                let size_score = (content_len.min(1000) / 100) as i32;

                spans.push(CandidateSpan {
                    range: pos..end + 1,
                    score: 45 + position_score + size_score, // Slightly lower than objects
                    kind: SpanKind::GreppedArray,
                });
            } else {
                // Incomplete array - use rest of input (for streaming)
                let position_score = if pos == 0 { 10 } else { 0 };
                spans.push(CandidateSpan {
                    range: pos..input.len(),
                    score: 25 + position_score, // Lower priority than complete arrays
                    kind: SpanKind::GreppedArray,
                });
            }
        }
    }

    spans
}

/// Find primitive values in text (true/false/null/numbers)
fn find_primitives(input: &str) -> Vec<CandidateSpan> {
    let mut spans = Vec::new();
    let lower_input = input.to_lowercase();

    // First check for boolean ambiguity - if both "true" AND "false" appear as words,
    // don't extract either as primitives (ambiguous input should fail parsing)
    let has_true = has_word_boundary_match(&lower_input, input, "true");
    let has_false = has_word_boundary_match(&lower_input, input, "false");
    let bool_ambiguous = has_true && has_false;

    // Boolean keywords to search for (case-insensitive)
    // Only extract if not ambiguous
    if !bool_ambiguous {
        let bool_keywords = [
            ("true", true),
            ("false", false),
        ];

        for (keyword, _value) in &bool_keywords {
            let mut search_pos = 0;
            while let Some(offset) = lower_input[search_pos..].find(keyword) {
                let abs_pos = search_pos + offset;
                let end_pos = abs_pos + keyword.len();

                // Check word boundaries
                let before_ok = abs_pos == 0 || !input[..abs_pos].chars().last().unwrap_or(' ').is_alphanumeric();
                let after_ok = end_pos == input.len() || !input[end_pos..].chars().next().unwrap_or(' ').is_alphanumeric();

                if before_ok && after_ok {
                    // Higher score for standalone booleans at end of text or after "answer:"/"is"
                    let context_score = if abs_pos > 0 {
                        let before = &lower_input[..abs_pos];
                        if before.trim_end().ends_with(':') || before.trim_end().ends_with("is") ||
                           before.trim_end().ends_with("answer") {
                            20
                        } else {
                            0
                        }
                    } else {
                        10
                    };

                    spans.push(CandidateSpan {
                        range: abs_pos..end_pos,
                        score: 30 + context_score,
                        kind: SpanKind::Primitive,
                    });
                }
                search_pos = end_pos;
            }
        }
    }

    // null keyword
    {
        let keyword = "null";
        let mut search_pos = 0;
        while let Some(offset) = lower_input[search_pos..].find(keyword) {
            let abs_pos = search_pos + offset;
            let end_pos = abs_pos + keyword.len();

            let before_ok = abs_pos == 0 || !input[..abs_pos].chars().last().unwrap_or(' ').is_alphanumeric();
            let after_ok = end_pos == input.len() || !input[end_pos..].chars().next().unwrap_or(' ').is_alphanumeric();

            if before_ok && after_ok {
                spans.push(CandidateSpan {
                    range: abs_pos..end_pos,
                    score: 25,
                    kind: SpanKind::Primitive,
                });
            }
            search_pos = end_pos;
        }
    }

    spans
}

/// Check if a keyword appears with word boundaries
fn has_word_boundary_match(lower_input: &str, input: &str, keyword: &str) -> bool {
    let mut search_pos = 0;
    while let Some(offset) = lower_input[search_pos..].find(keyword) {
        let abs_pos = search_pos + offset;
        let end_pos = abs_pos + keyword.len();

        let before_ok = abs_pos == 0 || !input[..abs_pos].chars().last().unwrap_or(' ').is_alphanumeric();
        let after_ok = end_pos == input.len() || !input[end_pos..].chars().next().unwrap_or(' ').is_alphanumeric();

        if before_ok && after_ok {
            return true;
        }
        search_pos = end_pos;
    }
    false
}

/// Find the matching closing brace, handling nesting, strings, and code blocks
fn find_matching_brace(input: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut string_char = '"';
    let mut in_code_block = false;
    let mut closing_backticks = 0u8;

    let slice = &input[start..];
    let mut char_iter = slice.char_indices().peekable();

    while let Some((byte_offset, c)) = char_iter.next() {
        let abs_pos = start + byte_offset;

        if escape {
            escape = false;
            continue;
        }

        // Handle code blocks (```)
        if in_code_block {
            if c == '`' {
                closing_backticks += 1;
                if closing_backticks >= 3 {
                    in_code_block = false;
                    closing_backticks = 0;
                }
            } else {
                closing_backticks = 0;
            }
            continue;
        }

        if in_string {
            match c {
                '\\' => escape = true,
                c if c == string_char => in_string = false,
                _ => {}
            }
            continue;
        }

        // Check for triple backticks (code block start)
        if c == '`' {
            // Look ahead to see if this is ```
            let remaining = &slice[byte_offset..];
            if remaining.starts_with("```") {
                in_code_block = true;
                closing_backticks = 0;
                // Skip the next two backticks
                char_iter.next();
                char_iter.next();
                continue;
            }
        }

        match c {
            '"' | '\'' | '`' => {
                in_string = true;
                string_char = c;
            }
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(abs_pos);
                }
            }
            _ => {}
        }
    }

    None
}

/// Check if a span looks like valid JSON-ish content
pub fn looks_like_json(content: &str) -> bool {
    let trimmed = content.trim();

    // Check for JSON object, array, or string start
    if trimmed.starts_with('{') || trimmed.starts_with('[') ||
       trimmed.starts_with('"') || trimmed.starts_with('\'') {
        return true;
    }

    // Check for JSON-ish patterns
    let has_colon = trimmed.contains(':');
    let has_brace = trimmed.contains('{') || trimmed.contains('}');
    let has_bracket = trimmed.contains('[') || trimmed.contains(']');

    has_colon && (has_brace || has_bracket)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_block_with_braces() {
        // JSON object containing a code block with braces inside
        let input = r#"
{
  "code": ```
function test() { return {} }
```,
  "type": "code"
}
"#;
        let spans = extract_spans(input, 5);

        // Should find the complete JSON object
        let obj_span = spans
            .iter()
            .find(|s| s.kind == SpanKind::GreppedObject)
            .expect("Should find JSON object");
        let content = &input[obj_span.range.clone()];

        // The span should contain the entire object including the closing brace
        assert!(content.starts_with('{'));
        assert!(content.ends_with('}'));
        assert!(content.contains("type"));
    }

    #[test]
    fn test_code_block_complex_content() {
        // JSON object with code block containing template literals and braces
        let input = r#"
Here's some text:

{
  "code": ```
const x = `Hello ${world}`;
function test() {
  if (true) {
    console.log(`value: ${obj.field}`);
  }
}
```,
    "type": "code",
}

More text after.
"#;
        let spans = extract_spans(input, 5);

        // Should find the complete JSON object
        let obj_span = spans
            .iter()
            .find(|s| s.kind == SpanKind::GreppedObject)
            .expect("Should find JSON object");
        let content = &input[obj_span.range.clone()];

        // Verify the span is complete
        assert!(content.starts_with('{'), "Should start with {{, got: {:?}", &content[..20.min(content.len())]);
        assert!(content.ends_with('}'), "Should end with }}, got last 20: {:?}", &content[content.len().saturating_sub(20)..]);
        assert!(content.contains(r#""type": "code""#), "Should contain type field");
    }

    #[test]
    fn test_markdown_fence() {
        let input = r#"Here is the result:
```json
{"key": "value"}
```
Done!"#;

        let spans = extract_spans(input, 3);

        // First span should be the fenced content
        assert!(!spans.is_empty());
        assert_eq!(spans[0].kind, SpanKind::MarkdownFence);

        let content = &input[spans[0].range.clone()];
        assert!(content.contains(r#"{"key": "value"}"#));
    }

    #[test]
    fn test_grepped_object() {
        let input = r#"The answer is {"name": "test", "value": 42} as expected."#;

        let spans = extract_spans(input, 3);

        let obj_span = spans
            .iter()
            .find(|s| s.kind == SpanKind::GreppedObject)
            .unwrap();
        let content = &input[obj_span.range.clone()];
        assert!(content.starts_with('{'));
        assert!(content.ends_with('}'));
    }

    #[test]
    fn test_grepped_array() {
        let input = r#"Here are the numbers: [1, 2, 3, 4] and more."#;

        let spans = extract_spans(input, 3);

        let arr_span = spans
            .iter()
            .find(|s| s.kind == SpanKind::GreppedArray)
            .unwrap();
        let content = &input[arr_span.range.clone()];
        assert_eq!(content, "[1, 2, 3, 4]");
    }

    #[test]
    fn test_nested_braces() {
        let input = r#"{"outer": {"inner": [1, 2, {"deep": true}]}}"#;

        let spans = extract_spans(input, 3);

        let obj_span = spans
            .iter()
            .find(|s| s.kind == SpanKind::GreppedObject)
            .unwrap();
        let content = &input[obj_span.range.clone()];
        assert_eq!(content, input);
    }

    #[test]
    fn test_strings_with_braces() {
        let input = r#"{"text": "contains { and } and [ and ]"}"#;

        let spans = extract_spans(input, 3);

        let obj_span = spans
            .iter()
            .find(|s| s.kind == SpanKind::GreppedObject)
            .unwrap();
        let content = &input[obj_span.range.clone()];
        assert_eq!(content, input);
    }

    #[test]
    fn test_multiple_fences() {
        let input = r#"First:
```json
{"a": 1}
```
Second:
```json
{"b": 2}
```"#;

        let spans = extract_spans(input, 5);

        let fence_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.kind == SpanKind::MarkdownFence)
            .collect();
        assert_eq!(fence_spans.len(), 2);
    }

    #[test]
    fn test_fallback_to_full_input() {
        let input = "just plain text with no json";

        let spans = extract_spans(input, 3);

        assert!(!spans.is_empty());
        assert!(spans.iter().any(|s| s.kind == SpanKind::FullInput));
    }

    #[test]
    fn test_looks_like_json() {
        assert!(looks_like_json(r#"{"key": "value"}"#));
        assert!(looks_like_json(r#"[1, 2, 3]"#));
        assert!(looks_like_json(r#"  { "a": 1 }  "#));
        assert!(!looks_like_json("just plain text"));
    }
}
