//! Tolerant Streaming Lexer
//!
//! A JSON-ish lexer that handles:
//! - Standard JSON tokens
//! - Unquoted keys and values
//! - Comments (// and /* */)
//! - Trailing commas
//! - Triple-backtick code blocks
//! - Incomplete tokens across chunk boundaries

use std::borrow::Cow;

/// Quote style for string tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteStyle {
    Double,   // "..."
    Single,   // '...'
    Backtick, // `...`
    Unquoted, // bare identifier or value
}

/// Token types produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    // Structural tokens - always complete (atomic)
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
    Colon,    // :
    Comma,    // ,

    // Value tokens - carry completion state
    String {
        content: Cow<'a, str>,
        quote: QuoteStyle,
        complete: bool,
    },
    Number {
        raw: Cow<'a, str>,
        complete: bool,
    },
    True,
    False,
    Null,

    // Special: triple-backtick code blocks as values
    CodeBlock {
        lang: Option<Cow<'a, str>>,
        content: Cow<'a, str>,
        complete: bool,
    },
}

impl<'a> Token<'a> {
    /// Returns the completion state of this token
    pub fn is_complete(&self) -> bool {
        match self {
            // Structural tokens are always complete
            Token::LBrace
            | Token::RBrace
            | Token::LBracket
            | Token::RBracket
            | Token::Colon
            | Token::Comma
            | Token::True
            | Token::False
            | Token::Null => true,

            // Value tokens carry their own state
            Token::String { complete, .. } => *complete,
            Token::Number { complete, .. } => *complete,
            Token::CodeBlock { complete, .. } => *complete,
        }
    }

    /// Convert to owned version (for storage across chunk boundaries)
    pub fn into_owned(self) -> Token<'static> {
        match self {
            Token::LBrace => Token::LBrace,
            Token::RBrace => Token::RBrace,
            Token::LBracket => Token::LBracket,
            Token::RBracket => Token::RBracket,
            Token::Colon => Token::Colon,
            Token::Comma => Token::Comma,
            Token::True => Token::True,
            Token::False => Token::False,
            Token::Null => Token::Null,
            Token::String {
                content,
                quote,
                complete,
            } => Token::String {
                content: Cow::Owned(content.into_owned()),
                quote,
                complete,
            },
            Token::Number { raw, complete } => Token::Number {
                raw: Cow::Owned(raw.into_owned()),
                complete,
            },
            Token::CodeBlock {
                lang,
                content,
                complete,
            } => Token::CodeBlock {
                lang: lang.map(|l| Cow::Owned(l.into_owned())),
                content: Cow::Owned(content.into_owned()),
                complete,
            },
        }
    }
}

/// Lexer mode for handling incomplete constructs
#[derive(Debug, Clone, Default)]
enum LexMode {
    #[default]
    Normal,
    InString {
        quote: char,
        escaped: bool,
        content: String,
    },
    InLineComment,
    InBlockComment {
        saw_star: bool,
    },
    InCodeBlock {
        backtick_count: u8,
        lang: Option<String>,
        saw_opening_newline: bool,
        content: String,
        closing_backticks: u8,
    },
    InUnquotedValue {
        content: String,
    },
    InNumber {
        content: String,
    },
    InTripleQuotedString {
        quote: char,
        content: String,
        closing_quotes: u8, // Count of consecutive closing quotes seen
    },
}

/// Streaming tolerant lexer
pub struct Lexer {
    /// Buffered input not yet fully consumed
    buffer: String,
    /// Current position in buffer
    pos: usize,
    /// Mode for handling incomplete constructs across chunks
    mode: LexMode,
    /// Accumulated tokens
    tokens: Vec<Token<'static>>,
}

impl Lexer {
    pub fn new() -> Self {
        Lexer {
            buffer: String::new(),
            pos: 0,
            mode: LexMode::Normal,
            tokens: Vec::new(),
        }
    }

    /// Append new chunk to buffer
    pub fn append(&mut self, chunk: &str) {
        self.buffer.push_str(chunk);
    }

    /// Extract all complete tokens, leaving incomplete state in buffer
    pub fn drain_tokens(&mut self) -> Vec<Token<'static>> {
        self.tokenize();
        std::mem::take(&mut self.tokens)
    }

    /// Get incomplete token if any (for streaming)
    pub fn incomplete_token(&self) -> Option<Token<'static>> {
        match &self.mode {
            LexMode::InString {
                quote, content, ..
            } => Some(Token::String {
                content: Cow::Owned(content.clone()),
                quote: match *quote {
                    '"' => QuoteStyle::Double,
                    '\'' => QuoteStyle::Single,
                    '`' => QuoteStyle::Backtick,
                    _ => QuoteStyle::Unquoted,
                },
                complete: false,
            }),
            LexMode::InNumber { content } => Some(Token::Number {
                raw: Cow::Owned(content.clone()),
                complete: false,
            }),
            LexMode::InCodeBlock { lang, content, .. } => Some(Token::CodeBlock {
                lang: lang.clone().map(Cow::Owned),
                content: Cow::Owned(content.clone()),
                complete: false,
            }),
            LexMode::InUnquotedValue { content } => Some(Token::String {
                content: Cow::Owned(content.clone()),
                quote: QuoteStyle::Unquoted,
                complete: false,
            }),
            LexMode::InTripleQuotedString { quote, content, .. } => Some(Token::String {
                content: Cow::Owned(content.clone()),
                quote: if *quote == '"' {
                    QuoteStyle::Double
                } else {
                    QuoteStyle::Single
                },
                complete: false,
            }),
            _ => None,
        }
    }

    fn tokenize(&mut self) {
        loop {
            match std::mem::take(&mut self.mode) {
                LexMode::Normal => {
                    if !self.lex_normal() {
                        break;
                    }
                }
                LexMode::InString {
                    quote,
                    escaped,
                    content,
                } => {
                    self.mode = LexMode::InString {
                        quote,
                        escaped,
                        content,
                    };
                    if !self.continue_string() {
                        break;
                    }
                }
                LexMode::InLineComment => {
                    self.skip_line_comment();
                }
                LexMode::InBlockComment { saw_star } => {
                    self.mode = LexMode::InBlockComment { saw_star };
                    if !self.skip_block_comment() {
                        break;
                    }
                }
                LexMode::InCodeBlock {
                    backtick_count,
                    lang,
                    saw_opening_newline,
                    content,
                    closing_backticks,
                } => {
                    self.mode = LexMode::InCodeBlock {
                        backtick_count,
                        lang,
                        saw_opening_newline,
                        content,
                        closing_backticks,
                    };
                    if !self.continue_code_block() {
                        break;
                    }
                }
                LexMode::InUnquotedValue { content } => {
                    self.mode = LexMode::InUnquotedValue { content };
                    if !self.continue_unquoted() {
                        break;
                    }
                }
                LexMode::InNumber { content } => {
                    self.mode = LexMode::InNumber { content };
                    if !self.continue_number() {
                        break;
                    }
                }
                LexMode::InTripleQuotedString { quote, content, closing_quotes } => {
                    self.mode = LexMode::InTripleQuotedString { quote, content, closing_quotes };
                    if !self.continue_triple_quoted_string() {
                        break;
                    }
                }
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.buffer[self.pos..].chars().next()
    }

    fn peek_ahead(&self, n: usize) -> &str {
        // Take n bytes, but make sure we end on a valid UTF-8 char boundary
        let rest = &self.buffer[self.pos..];
        let mut byte_len = 0;
        for c in rest.chars().take(n) {
            byte_len += c.len_utf8();
        }
        &rest[..byte_len.min(rest.len())]
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn lex_normal(&mut self) -> bool {
        self.skip_whitespace();

        let Some(c) = self.peek_char() else {
            return false;
        };

        match c {
            '{' => {
                self.advance();
                self.tokens.push(Token::LBrace);
                true
            }
            '}' => {
                self.advance();
                self.tokens.push(Token::RBrace);
                true
            }
            '[' => {
                self.advance();
                self.tokens.push(Token::LBracket);
                true
            }
            ']' => {
                self.advance();
                self.tokens.push(Token::RBracket);
                true
            }
            ':' => {
                self.advance();
                self.tokens.push(Token::Colon);
                true
            }
            ',' => {
                self.advance();
                self.tokens.push(Token::Comma);
                true
            }
            '"' if self.peek_ahead(3) == "\"\"\"" => self.start_triple_quoted_string('"'),
            '\'' if self.peek_ahead(3) == "'''" => self.start_triple_quoted_string('\''),
            '"' | '\'' => self.start_string(c),
            '`' if self.peek_ahead(3) == "```" => self.start_code_block(),
            '`' => self.start_string(c), // Single backtick string
            '/' if self.peek_ahead(2) == "//" => {
                self.mode = LexMode::InLineComment;
                self.pos += 2;
                true
            }
            '/' if self.peek_ahead(2) == "/*" => {
                self.mode = LexMode::InBlockComment { saw_star: false };
                self.pos += 2;
                true
            }
            _ if c == '-' || c == '+' || c.is_ascii_digit() => self.start_number(),
            _ => self.start_unquoted(),
        }
    }

    fn start_string(&mut self, quote: char) -> bool {
        self.advance(); // Skip opening quote
        self.mode = LexMode::InString {
            quote,
            escaped: false,
            content: String::new(),
        };
        self.continue_string()
    }

    fn continue_string(&mut self) -> bool {
        let LexMode::InString {
            quote,
            mut escaped,
            mut content,
        } = std::mem::take(&mut self.mode)
        else {
            return false;
        };

        while let Some(c) = self.peek_char() {
            if escaped {
                // Handle escape sequences
                let escaped_char = match c {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    '\'' => '\'',
                    '/' => '/',
                    'b' => '\u{0008}',
                    'f' => '\u{000C}',
                    _ => c, // Unknown escape, keep as-is
                };
                content.push(escaped_char);
                escaped = false;
                self.advance();
            } else if c == '\\' {
                escaped = true;
                self.advance();
            } else if c == quote {
                self.advance(); // Skip closing quote
                self.tokens.push(Token::String {
                    content: Cow::Owned(content),
                    quote: match quote {
                        '"' => QuoteStyle::Double,
                        '\'' => QuoteStyle::Single,
                        '`' => QuoteStyle::Backtick,
                        _ => QuoteStyle::Unquoted, // shouldn't happen
                    },
                    complete: true,
                });
                self.mode = LexMode::Normal;
                return true;
            } else {
                content.push(c);
                self.advance();
            }
        }

        // Incomplete string - save state
        self.mode = LexMode::InString {
            quote,
            escaped,
            content,
        };
        false
    }

    fn start_triple_quoted_string(&mut self, quote: char) -> bool {
        // Skip the three opening quotes
        self.advance(); // First quote
        self.advance(); // Second quote
        self.advance(); // Third quote
        self.mode = LexMode::InTripleQuotedString {
            quote,
            content: String::new(),
            closing_quotes: 0,
        };
        self.continue_triple_quoted_string()
    }

    fn continue_triple_quoted_string(&mut self) -> bool {
        let LexMode::InTripleQuotedString {
            quote,
            mut content,
            mut closing_quotes,
        } = std::mem::take(&mut self.mode)
        else {
            return false;
        };

        while let Some(c) = self.peek_char() {
            if c == quote {
                closing_quotes += 1;
                self.advance();

                if closing_quotes >= 3 {
                    // Found closing triple quote
                    // Trim leading/trailing newlines and apply dedent
                    let trimmed = content
                        .trim_start_matches('\n')
                        .trim_start_matches('\r')
                        .trim_end_matches('\n')
                        .trim_end_matches('\r')
                        .trim_end();

                    // Apply dedent: find minimum indentation and remove it from each line
                    let final_content = dedent_string(trimmed);

                    self.tokens.push(Token::String {
                        content: Cow::Owned(final_content),
                        quote: if quote == '"' {
                            QuoteStyle::Double
                        } else {
                            QuoteStyle::Single
                        },
                        complete: true,
                    });
                    self.mode = LexMode::Normal;
                    return true;
                }
            } else {
                // Not a quote - add any pending quotes to content
                for _ in 0..closing_quotes {
                    content.push(quote);
                }
                closing_quotes = 0;
                content.push(c);
                self.advance();
            }
        }

        // Incomplete - save state
        self.mode = LexMode::InTripleQuotedString {
            quote,
            content,
            closing_quotes,
        };
        false
    }

    fn start_number(&mut self) -> bool {
        let start = self.pos;
        let mut content = String::new();

        // Optional sign
        if let Some(c) = self.peek_char() {
            if c == '-' || c == '+' {
                content.push(c);
                self.advance();
            }
        }

        // Integer part - with support for thousand separators
        let mut digit_count_after_separator = 0;
        let mut has_separator = false;
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                content.push(c);
                self.advance();
                if has_separator {
                    digit_count_after_separator += 1;
                }
            } else if c == ',' {
                // Check if this looks like a thousand separator:
                // Must be followed by exactly 3 digits (or more with more separators)
                let remaining = &self.buffer[self.pos + 1..];
                let next_digits = remaining.chars().take_while(|c| c.is_ascii_digit()).count();
                if next_digits >= 3 && (next_digits == 3 || remaining.chars().nth(3) == Some(',')) {
                    // This looks like a thousand separator, include it
                    content.push(c);
                    self.advance();
                    has_separator = true;
                    digit_count_after_separator = 0;
                } else {
                    // This is a delimiter comma, stop
                    break;
                }
            } else {
                break;
            }
        }

        // Check for decimal or exponent
        if let Some(c) = self.peek_char() {
            if c == '.' {
                content.push(c);
                self.advance();
                while let Some(c) = self.peek_char() {
                    if c.is_ascii_digit() {
                        content.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        // Exponent
        if let Some(c) = self.peek_char() {
            if c == 'e' || c == 'E' {
                content.push(c);
                self.advance();
                if let Some(c) = self.peek_char() {
                    if c == '+' || c == '-' {
                        content.push(c);
                        self.advance();
                    }
                }
                while let Some(c) = self.peek_char() {
                    if c.is_ascii_digit() {
                        content.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        // Check if number is complete (followed by delimiter)
        let complete = match self.peek_char() {
            Some(c) if c.is_whitespace() || matches!(c, ',' | '}' | ']' | ':') => true,
            None => false, // End of input - might be incomplete
            _ => true,     // Other character follows, number is done
        };

        if content.is_empty() {
            // No valid number found, rewind
            self.pos = start;
            return self.start_unquoted();
        }

        self.tokens.push(Token::Number {
            raw: Cow::Owned(content),
            complete,
        });
        true
    }

    fn continue_number(&mut self) -> bool {
        let LexMode::InNumber { mut content } = std::mem::take(&mut self.mode) else {
            return false;
        };

        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
                content.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let complete = match self.peek_char() {
            Some(c) if c.is_whitespace() || matches!(c, ',' | '}' | ']' | ':') => true,
            None => false,
            _ => true,
        };

        if complete || self.peek_char().is_some() {
            self.tokens.push(Token::Number {
                raw: Cow::Owned(content),
                complete,
            });
            self.mode = LexMode::Normal;
            true
        } else {
            self.mode = LexMode::InNumber { content };
            false
        }
    }

    fn start_unquoted(&mut self) -> bool {
        let mut content = String::new();
        self.mode = LexMode::InUnquotedValue { content };
        self.continue_unquoted()
    }

    fn continue_unquoted(&mut self) -> bool {
        let LexMode::InUnquotedValue { mut content } = std::mem::take(&mut self.mode) else {
            return false;
        };

        // Collect until we hit a structural char
        // Note: We intentionally do NOT break on newlines, allowing multi-line
        // unquoted values like:
        //   b: hey world
        //   so that we can test this out,
        // The value continues until we hit a delimiter (comma, braces, brackets)
        while let Some(c) = self.peek_char() {
            if matches!(c, '{' | '}' | '[' | ']' | ':' | ',') {
                break;
            }
            content.push(c);
            self.advance();
        }

        let trimmed = content.trim();

        // Check for keywords - use exact case matching for JSON compatibility
        // (JSON requires lowercase true/false/null)
        match trimmed {
            "true" | "True" | "TRUE" => {
                self.tokens.push(Token::True);
                self.mode = LexMode::Normal;
                return true;
            }
            "false" | "False" | "FALSE" => {
                self.tokens.push(Token::False);
                self.mode = LexMode::Normal;
                return true;
            }
            // Only explicit "null" variants - NOT "none"/"None" which are ambiguous
            // (could be the string "None" or Python's None which means null)
            "null" | "NULL" => {
                self.tokens.push(Token::Null);
                self.mode = LexMode::Normal;
                return true;
            }
            _ => {}
        }

        // Check for number
        if let Ok(_) = trimmed.parse::<f64>() {
            self.tokens.push(Token::Number {
                raw: Cow::Owned(trimmed.to_string()),
                complete: true,
            });
            self.mode = LexMode::Normal;
            return true;
        }

        // Check completion
        let complete = self.peek_char().is_some();

        if trimmed.is_empty() && !complete {
            // No content and no delimiter - need more input
            self.mode = LexMode::InUnquotedValue { content };
            return false;
        }

        if !trimmed.is_empty() {
            self.tokens.push(Token::String {
                content: Cow::Owned(trimmed.to_string()),
                quote: QuoteStyle::Unquoted,
                complete,
            });
        }

        self.mode = LexMode::Normal;
        true
    }

    fn start_code_block(&mut self) -> bool {
        // Skip opening ```
        self.pos += 3;

        // Read language tag (until newline)
        let mut lang = String::new();
        while let Some(c) = self.peek_char() {
            if c == '\n' {
                self.advance();
                break;
            }
            lang.push(c);
            self.advance();
        }

        let lang = if lang.trim().is_empty() {
            None
        } else {
            Some(lang.trim().to_string())
        };

        self.mode = LexMode::InCodeBlock {
            backtick_count: 3,
            lang,
            saw_opening_newline: true,
            content: String::new(),
            closing_backticks: 0,
        };

        self.continue_code_block()
    }

    fn continue_code_block(&mut self) -> bool {
        let LexMode::InCodeBlock {
            backtick_count,
            lang,
            saw_opening_newline,
            mut content,
            mut closing_backticks,
        } = std::mem::take(&mut self.mode)
        else {
            return false;
        };

        while let Some(c) = self.peek_char() {
            if c == '`' {
                closing_backticks += 1;
                self.advance();

                if closing_backticks >= backtick_count {
                    // End of code block
                    // Remove trailing newline from content if present
                    if content.ends_with('\n') {
                        content.pop();
                    }

                    self.tokens.push(Token::CodeBlock {
                        lang: lang.map(Cow::Owned),
                        content: Cow::Owned(content),
                        complete: true,
                    });
                    self.mode = LexMode::Normal;
                    return true;
                }
            } else {
                // Add any backticks we collected as content
                for _ in 0..closing_backticks {
                    content.push('`');
                }
                closing_backticks = 0;
                content.push(c);
                self.advance();
            }
        }

        // Incomplete - save state
        self.mode = LexMode::InCodeBlock {
            backtick_count,
            lang,
            saw_opening_newline,
            content,
            closing_backticks,
        };
        false
    }

    fn skip_line_comment(&mut self) {
        while let Some(c) = self.peek_char() {
            self.advance();
            if c == '\n' {
                self.mode = LexMode::Normal;
                return;
            }
        }
        // End of input in comment - that's fine
        self.mode = LexMode::Normal;
    }

    fn skip_block_comment(&mut self) -> bool {
        let LexMode::InBlockComment { mut saw_star } = std::mem::take(&mut self.mode) else {
            return false;
        };

        while let Some(c) = self.peek_char() {
            self.advance();
            if saw_star && c == '/' {
                self.mode = LexMode::Normal;
                return true;
            }
            saw_star = c == '*';
        }

        // Incomplete block comment
        self.mode = LexMode::InBlockComment { saw_star };
        false
    }
}

impl Default for Lexer {
    fn default() -> Self {
        Self::new()
    }
}

/// Dedent a multi-line string by removing common leading whitespace
fn dedent_string(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();

    // Find minimum indentation (ignoring empty lines)
    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Remove the common indentation
    lines
        .iter()
        .map(|line| {
            if line.len() >= min_indent {
                &line[min_indent..]
            } else {
                line.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_object() {
        let mut lexer = Lexer::new();
        lexer.append(r#"{"key": "value"}"#);
        let tokens = lexer.drain_tokens();

        assert_eq!(tokens.len(), 5);
        assert!(matches!(tokens[0], Token::LBrace));
        assert!(matches!(tokens[1], Token::String { ref content, quote: QuoteStyle::Double, complete: true } if content == "key"));
        assert!(matches!(tokens[2], Token::Colon));
        assert!(matches!(tokens[3], Token::String { ref content, quote: QuoteStyle::Double, complete: true } if content == "value"));
        assert!(matches!(tokens[4], Token::RBrace));
    }

    #[test]
    fn test_unquoted_key() {
        let mut lexer = Lexer::new();
        lexer.append("{ foo: 1 }");
        let tokens = lexer.drain_tokens();

        assert_eq!(tokens.len(), 5);
        assert!(matches!(tokens[1], Token::String { ref content, quote: QuoteStyle::Unquoted, .. } if content == "foo"));
        assert!(matches!(tokens[3], Token::Number { ref raw, .. } if raw == "1"));
    }

    #[test]
    fn test_streaming_string() {
        let mut lexer = Lexer::new();
        lexer.append(r#""hel"#);
        let tokens = lexer.drain_tokens();
        assert!(tokens.is_empty());

        let incomplete = lexer.incomplete_token();
        assert!(matches!(incomplete, Some(Token::String { ref content, complete: false, .. }) if content == "hel"));

        lexer.append(r#"lo""#);
        let tokens = lexer.drain_tokens();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0], Token::String { ref content, complete: true, .. } if content == "hello"));
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new();
        lexer.append(r#"{ // comment
            "key": /* block */ "value"
        }"#);
        let tokens = lexer.drain_tokens();

        // Should skip comments
        assert!(tokens.iter().all(|t| !matches!(t, Token::String { ref content, .. } if content.contains("comment") || content.contains("block"))));
    }

    #[test]
    fn test_code_block() {
        let mut lexer = Lexer::new();
        lexer.append("```python\nprint('hello')\n```");
        let tokens = lexer.drain_tokens();

        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0], Token::CodeBlock { ref lang, ref content, complete: true }
            if lang.as_ref().map(|s| s.as_ref()) == Some("python") && content == "print('hello')"));
    }

    #[test]
    fn test_trailing_comma() {
        let mut lexer = Lexer::new();
        lexer.append(r#"[1, 2, 3,]"#);
        let tokens = lexer.drain_tokens();

        // Should produce: LBracket, 1, Comma, 2, Comma, 3, Comma, RBracket
        assert!(matches!(tokens.last(), Some(Token::RBracket)));
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new();
        lexer.append("[true, false, null]");
        let tokens = lexer.drain_tokens();

        // LBracket, True, Comma, False, Comma, Null, RBracket = 7 tokens
        assert_eq!(tokens.len(), 7);
        assert!(matches!(tokens[0], Token::LBracket));
        assert!(matches!(tokens[1], Token::True));
        assert!(matches!(tokens[2], Token::Comma));
        assert!(matches!(tokens[3], Token::False));
        assert!(matches!(tokens[4], Token::Comma));
        assert!(matches!(tokens[5], Token::Null));
        assert!(matches!(tokens[6], Token::RBracket));
    }

    #[test]
    fn test_escape_sequences() {
        let mut lexer = Lexer::new();
        lexer.append(r#""hello\nworld""#);
        let tokens = lexer.drain_tokens();

        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0], Token::String { ref content, .. } if content == "hello\nworld"));
    }

    #[test]
    fn test_code_block_inside_json_object() {
        let input = r#"{
  "code": ```
function test() { return {} }
```,
    "type": "code",
}"#;
        let mut lexer = Lexer::new();
        lexer.append(input);
        let tokens = lexer.drain_tokens();

        eprintln!("Tokens: {:?}", tokens);

        // Expected tokens:
        // LBrace, "code", Colon, CodeBlock, Comma, "type", Colon, "code", Comma, RBrace
        assert!(tokens.len() >= 9, "Expected at least 9 tokens, got {}", tokens.len());

        // Check that we have the type field
        let has_type_field = tokens.iter().any(|t| matches!(t, Token::String { ref content, .. } if content == "type"));
        assert!(has_type_field, "Should have 'type' string token");

        // Find the CodeBlock token
        let code_block = tokens.iter().find(|t| matches!(t, Token::CodeBlock { .. }));
        assert!(code_block.is_some(), "Should have a CodeBlock token");
    }

    #[test]
    fn test_code_block_with_template_literals() {
        // Test with content containing single backticks (template literals)
        let input = r#"{
  "code": ```
const mainBranch = getMainBranch();
console.log(`Main branch is: ${mainBranch}`);
console.error(`Error: ${error.message}`);
```,
    "type": "code",
}"#;
        let mut lexer = Lexer::new();
        lexer.append(input);
        let tokens = lexer.drain_tokens();

        eprintln!("Tokens with template literals: {:?}", tokens);

        // Check that we have the type field
        let has_type_field = tokens.iter().any(|t| matches!(t, Token::String { ref content, .. } if content == "type"));
        assert!(has_type_field, "Should have 'type' string token after code block with template literals");

        // Find the CodeBlock token and verify it contains the template literals
        let code_block = tokens.iter().find_map(|t| {
            if let Token::CodeBlock { content, .. } = t {
                Some(content.clone())
            } else {
                None
            }
        });
        assert!(code_block.is_some(), "Should have a CodeBlock token");
        let content = code_block.unwrap();
        assert!(content.contains("`Main branch is:"), "CodeBlock should contain template literal");
    }

    #[test]
    fn test_code_block_with_lang_tag() {
        // Test with language tag (like the failing test)
        let input = r#"{
  "code": ```typescript main.ts
    const async function main() {
      console.log("Hello, world!");
    }
```,
    "type": "code",
}"#;
        let mut lexer = Lexer::new();
        lexer.append(input);
        let tokens = lexer.drain_tokens();

        eprintln!("Tokens with lang tag: {:?}", tokens);

        // Check that we have the type field
        let has_type_field = tokens.iter().any(|t| matches!(t, Token::String { ref content, .. } if content == "type"));
        assert!(has_type_field, "Should have 'type' string token after code block with lang tag");

        // Find the CodeBlock token
        let code_block = tokens.iter().find(|t| matches!(t, Token::CodeBlock { .. }));
        assert!(code_block.is_some(), "Should have a CodeBlock token");
    }
}
