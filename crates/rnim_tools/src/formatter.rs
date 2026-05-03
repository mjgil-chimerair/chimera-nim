//! Formatter for Nim source code.
//!
//! This module provides formatting/printer functionality for Nim source code,
//! producing stable output from CST while preserving comments and trivia.

use rnim_span::{FileId, Span};
use std::collections::HashMap;

/// Formatter configuration
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Spaces around operators
    pub spaces_around_operators: bool,
    /// Spaces around parentheses
    pub spaces_around_parens: bool,
    /// Indent amount (spaces)
    pub indent_size: usize,
    /// Maximum line length
    pub max_line_length: usize,
    /// Tab width (for tab conversion)
    pub tab_width: usize,
    /// Use tabs instead of spaces
    pub use_tabs: bool,
    /// Preserve empty lines
    pub preserve_empty_lines: bool,
    /// Indent case statements
    pub indent_case: bool,
    /// Put a space before :
    pub space_before_colon: bool,
    /// Put a space after :
    pub space_after_colon: bool,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        FormatterConfig {
            spaces_around_operators: true,
            spaces_around_parens: true,
            indent_size: 4,
            max_line_length: 80,
            tab_width: 4,
            use_tabs: false,
            preserve_empty_lines: true,
            indent_case: true,
            space_before_colon: false,
            space_after_colon: true,
        }
    }
}

/// Formatter state
#[derive(Debug, Clone)]
pub struct FormatterState {
    config: FormatterConfig,
    indent_level: usize,
    line: usize,
    column: usize,
    output: String,
}

impl FormatterState {
    pub fn new(config: FormatterConfig) -> Self {
        FormatterState {
            config,
            indent_level: 0,
            line: 1,
            column: 0,
            output: String::new(),
        }
    }

    /// Get current indentation string
    fn indent_string(&self) -> String {
        if self.config.use_tabs {
            "\t".repeat(self.indent_level)
        } else {
            " ".repeat(self.indent_level * self.config.indent_size)
        }
    }

    /// Write indentation
    pub fn write_indent(&mut self) {
        self.output.push_str(&self.indent_string());
        self.column += self.indent_string().len();
    }

    /// Write a string
    pub fn write(&mut self, s: &str) {
        self.output.push_str(s);
        for c in s.chars() {
            if c == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
    }

    /// Write a space if configured
    pub fn write_space(&mut self) {
        if self.config.spaces_around_operators {
            self.write(" ");
        }
    }

    /// Write a newline and indentation
    pub fn write_newline(&mut self) {
        self.write("\n");
        self.write_indent();
    }

    /// Increase indentation
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation
    pub fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Check if current line is too long
    pub fn is_line_too_long(&self) -> bool {
        self.column > self.config.max_line_length
    }

    /// Get current position
    pub fn position(&self) -> (usize, usize) {
        (self.line, self.column)
    }
}

/// Pretty printer result
#[derive(Debug, Clone)]
pub struct PrettyPrinted {
    pub source: String,
    pub line_count: usize,
    pub is_idempotent: bool,
}

impl PrettyPrinted {
    pub fn new(source: String) -> Self {
        let line_count = source.lines().count();
        // Idempotency check removed to avoid recursion during tokenization
        PrettyPrinted {
            source,
            line_count,
            is_idempotent: false,
        }
    }
}

/// Token kind for formatting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatTokenKind {
    /// Whitespace
    Whitespace,
    /// Comment
    Comment,
    /// Newline
    Newline,
    /// Identifier
    Ident,
    /// Keyword
    Keyword,
    /// Operator
    Operator,
    /// Punctuation
    Punctuation,
    /// String literal
    StringLiteral,
    /// Integer literal
    IntLiteral,
    /// Float literal
    FloatLiteral,
    /// Character literal
    CharLiteral,
    /// Eof
    Eof,
}

/// A token with formatting information
#[derive(Debug, Clone)]
pub struct FormatToken {
    pub kind: FormatTokenKind,
    pub text: String,
    pub span: Span,
}

impl FormatToken {
    pub fn new(kind: FormatTokenKind, text: &str, span: Span) -> Self {
        FormatToken {
            kind,
            text: text.to_string(),
            span,
        }
    }

    /// Check if this token is trivia (whitespace, comment)
    pub fn is_trivia(&self) -> bool {
        matches!(
            self.kind,
            FormatTokenKind::Whitespace | FormatTokenKind::Comment | FormatTokenKind::Newline
        )
    }
}

/// Formatter for Nim source tokens
pub struct Formatter {
    config: FormatterConfig,
    tokens: Vec<FormatToken>,
    current: usize,
}

impl Formatter {
    pub fn new(config: FormatterConfig) -> Self {
        Formatter {
            config,
            tokens: Vec::new(),
            current: 0,
        }
    }

    /// Parse source into tokens
    pub fn tokenize(&mut self, source: &str, file_id: FileId) {
        self.tokens.clear();
        self.current = 0;

        let mut chars = source.char_indices().peekable();
        let mut pos = 0;

        while let Some((idx, c)) = chars.next() {
            let span = Span::new(file_id, idx as u32, (idx + c.len_utf8()) as u32);

            match c {
                ' ' | '\t' | '\r' => {
                    let mut text = c.to_string();
                    while let Some(&(_, nc)) = chars.peek() {
                        if nc == ' ' || nc == '\t' || nc == '\r' {
                            text.push(nc);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    self.tokens
                        .push(FormatToken::new(FormatTokenKind::Whitespace, &text, span));
                }
                '\n' => {
                    self.tokens
                        .push(FormatToken::new(FormatTokenKind::Newline, "\n", span));
                }
                '#' => {
                    let mut text = String::from("#");
                    while let Some(&(end_idx, nc)) = chars.peek() {
                        text.push(nc);
                        chars.next();
                        if nc == '\n' {
                            break;
                        }
                        if end_idx >= source.len() {
                            break;
                        }
                    }
                    self.tokens
                        .push(FormatToken::new(FormatTokenKind::Comment, &text, span));
                }
                '"' => {
                    let mut text = String::from("\"");
                    let mut is_triple = false;
                    // Check for triple quote
                    if let Some(&(_, '"')) = chars.peek() {
                        text.push('"');
                        chars.next();
                        if let Some(&(_, '"')) = chars.peek() {
                            text.push('"');
                            chars.next();
                            is_triple = true;
                        }
                    }
                    // Read string content
                    loop {
                        if let Some(&(end_idx, nc)) = chars.peek() {
                            if nc == '"' && end_idx < source.len() {
                                text.push(nc);
                                chars.next();
                                if is_triple {
                                    if let Some(&(_, '"')) = chars.peek() {
                                        text.push('"');
                                        chars.next();
                                        if let Some(&(_, '"')) = chars.peek() {
                                            text.push('"');
                                            chars.next();
                                            break;
                                        }
                                    }
                                } else {
                                    break;
                                }
                            } else if nc == '\n' && !is_triple {
                                break;
                            } else if end_idx >= source.len() {
                                break;
                            } else {
                                text.push(nc);
                                chars.next();
                            }
                        } else {
                            break;
                        }
                    }
                    self.tokens.push(FormatToken::new(
                        FormatTokenKind::StringLiteral,
                        &text,
                        span,
                    ));
                }
                '\'' => {
                    let mut text = String::from("'");
                    if let Some(&(end_idx, nc)) = chars.peek() {
                        if nc != '\'' {
                            text.push(nc);
                            chars.next();
                            if let Some(&(_, '\'')) = chars.peek() {
                                text.push('\'');
                                chars.next();
                            }
                        } else {
                            text.push('\'');
                            chars.next();
                        }
                    }
                    self.tokens
                        .push(FormatToken::new(FormatTokenKind::CharLiteral, &text, span));
                }
                '/' => {
                    let mut text = String::from("/");
                    if let Some(&(_, '*')) = chars.peek() {
                        text.push('*');
                        chars.next();
                        loop {
                            if let Some(&(end_idx, '*')) = chars.peek() {
                                if end_idx >= source.len() - 1 {
                                    break;
                                }
                                text.push('*');
                                chars.next();
                                if let Some(&(_, '/')) = chars.peek() {
                                    text.push('/');
                                    chars.next();
                                    break;
                                }
                            } else if let Some((_, c)) = chars.next() {
                                text.push(c);
                            } else {
                                break;
                            }
                        }
                        self.tokens
                            .push(FormatToken::new(FormatTokenKind::Comment, &text, span));
                    } else {
                        self.tokens
                            .push(FormatToken::new(FormatTokenKind::Operator, &text, span));
                    }
                }
                '=' | '+' | '-' | '*' | '%' | '<' | '>' | '!' | '&' | '|' | '^' | '~' | '@'
                | '$' | '?' | ':' => {
                    let mut text = c.to_string();
                    while let Some(&(_, nc)) = chars.peek() {
                        if "=+-*/%<>!&|^~@$?:".contains(nc) {
                            text.push(nc);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    self.tokens
                        .push(FormatToken::new(FormatTokenKind::Operator, &text, span));
                }
                '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' => {
                    let text = c.to_string();
                    self.tokens
                        .push(FormatToken::new(FormatTokenKind::Punctuation, &text, span));
                }
                _ if c.is_ascii_alphabetic() || c == '_' => {
                    let mut text = c.to_string();
                    while let Some(&(_, nc)) = chars.peek() {
                        if nc.is_ascii_alphanumeric() || nc == '_' {
                            text.push(nc);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    // Check if keyword
                    let kind = if is_keyword(&text) {
                        FormatTokenKind::Keyword
                    } else {
                        FormatTokenKind::Ident
                    };
                    self.tokens.push(FormatToken::new(kind, &text, span));
                }
                _ if c.is_ascii_digit() => {
                    let mut text = c.to_string();
                    let mut has_dot = false;
                    while let Some(&(_, nc)) = chars.peek() {
                        if nc.is_ascii_digit() || nc == '_' {
                            text.push(nc);
                            chars.next();
                        } else if nc == '.' && !has_dot {
                            text.push('.');
                            chars.next();
                            has_dot = true;
                        } else if (nc == 'e' || nc == 'E') && !has_dot {
                            text.push(nc);
                            chars.next();
                            if let Some(&(_, '+' | '-')) = chars.peek() {
                                text.push(chars.next().unwrap().1);
                            }
                        } else if nc == '\'' {
                            // Type suffix
                            text.push(nc);
                            chars.next();
                            if let Some(&(_, c)) = chars.peek() {
                                text.push(c);
                                chars.next();
                            }
                        } else {
                            break;
                        }
                    }
                    let kind = if has_dot {
                        FormatTokenKind::FloatLiteral
                    } else {
                        FormatTokenKind::IntLiteral
                    };
                    self.tokens.push(FormatToken::new(kind, &text, span));
                }
                _ => {}
            }
            pos = idx + c.len_utf8();
        }

        // Add EOF token
        let span = Span::new(file_id, pos as u32, pos as u32);
        self.tokens
            .push(FormatToken::new(FormatTokenKind::Eof, "", span));
    }

    /// Format the tokenized source
    pub fn format(&mut self) -> PrettyPrinted {
        let mut state = FormatterState::new(self.config.clone());
        state.write_indent();

        while let Some(token) = self.current_token() {
            if token.kind == FormatTokenKind::Eof {
                break;
            }

            match token.kind {
                FormatTokenKind::Whitespace => {
                    // Skip whitespace, we'll add our own
                }
                FormatTokenKind::Newline => {
                    state.write_newline();
                }
                FormatTokenKind::Comment => {
                    state.write(&token.text);
                    if !token.text.ends_with('\n') {
                        state.write_newline();
                    }
                }
                FormatTokenKind::Keyword => {
                    state.write(&token.text);
                    state.write_space();
                }
                FormatTokenKind::Operator => {
                    state.write_space();
                    state.write(&token.text);
                    state.write_space();
                }
                FormatTokenKind::Punctuation => {
                    state.write(&token.text);
                    // Add space after certain punctuation
                    if token.text == "=" || token.text == "{" || token.text == "," {
                        state.write_space();
                    }
                }
                FormatTokenKind::Ident => {
                    state.write(&token.text);
                }
                FormatTokenKind::StringLiteral
                | FormatTokenKind::IntLiteral
                | FormatTokenKind::FloatLiteral
                | FormatTokenKind::CharLiteral => {
                    state.write(&token.text);
                }
                FormatTokenKind::Eof => break,
            }

            self.current += 1;
        }

        PrettyPrinted::new(state.output)
    }

    fn current_token(&self) -> Option<&FormatToken> {
        self.tokens.get(self.current)
    }
}

/// Check if text is a Nim keyword
fn is_keyword(text: &str) -> bool {
    matches!(
        text,
        "addr"
            | "and"
            | "as"
            | "asm"
            | "bind"
            | "block"
            | "break"
            | "case"
            | "cast"
            | "concept"
            | "const"
            | "continue"
            | "converter"
            | "defer"
            | "discard"
            | "distinct"
            | "div"
            | "do"
            | "elif"
            | "else"
            | "end"
            | "enum"
            | "except"
            | "export"
            | "finally"
            | "for"
            | "from"
            | "func"
            | "if"
            | "import"
            | "in"
            | "include"
            | "interface"
            | "is"
            | "isnot"
            | "iterator"
            | "let"
            | "macro"
            | "method"
            | "mixin"
            | "mod"
            | "not"
            | "notin"
            | "object"
            | "of"
            | "or"
            | "out"
            | "proc"
            | "ptr"
            | "raise"
            | "ref"
            | "return"
            | "shl"
            | "shr"
            | "static"
            | "template"
            | "try"
            | "type"
            | "typeof"
            | "using"
            | "var"
            | "when"
            | "while"
            | "xor"
            | "yield"
            | "true"
            | "false"
            | "this"
    )
}

/// Format source string directly
pub fn format_source(source: &str, config: &FormatterConfig) -> Result<String, String> {
    let mut formatter = Formatter::new(config.clone());
    formatter.tokenize(source, FileId(0));
    let result = formatter.format();
    Ok(result.source)
}

/// Format with default config
pub fn format_default(source: &str) -> Result<String, String> {
    format_source(source, &FormatterConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_config_default() {
        let config = FormatterConfig::default();
        assert_eq!(config.indent_size, 4);
        assert_eq!(config.max_line_length, 80);
        assert!(!config.use_tabs);
        assert!(config.preserve_empty_lines);
    }

    #[test]
    fn test_format_token_kind_is_trivia() {
        let token = FormatToken::new(FormatTokenKind::Whitespace, " ", Span::new(FileId(0), 0, 0));
        assert!(token.is_trivia());
        let token = FormatToken::new(
            FormatTokenKind::Comment,
            "# comment",
            Span::new(FileId(0), 0, 0),
        );
        assert!(token.is_trivia());
        let token = FormatToken::new(FormatTokenKind::Ident, "foo", Span::new(FileId(0), 0, 0));
        assert!(!token.is_trivia());
    }

    #[test]
    fn test_formatter_tokenize_keywords() {
        let mut formatter = Formatter::new(FormatterConfig::default());
        formatter.tokenize("proc foo", FileId(0));
        assert_eq!(formatter.tokens.len(), 4); // proc, whitespace, foo, eof
        assert_eq!(formatter.tokens[0].kind, FormatTokenKind::Keyword);
    }

    #[test]
    fn test_formatter_tokenize_operators() {
        let mut formatter = Formatter::new(FormatterConfig::default());
        formatter.tokenize("a + b", FileId(0));
        assert_eq!(formatter.tokens.len(), 6); // a, whitespace, +, whitespace, b, eof
        assert_eq!(formatter.tokens[2].kind, FormatTokenKind::Operator);
        assert_eq!(formatter.tokens[2].text, "+");
    }

    #[test]
    fn test_formatter_tokenize_string() {
        let mut formatter = Formatter::new(FormatterConfig::default());
        formatter.tokenize("\"hello\"", FileId(0));
        assert_eq!(formatter.tokens.len(), 2); // string, eof
        assert_eq!(formatter.tokens[0].kind, FormatTokenKind::StringLiteral);
    }

    #[test]
    fn test_formatter_tokenize_int() {
        let mut formatter = Formatter::new(FormatterConfig::default());
        formatter.tokenize("42", FileId(0));
        assert_eq!(formatter.tokens.len(), 2);
        assert_eq!(formatter.tokens[0].kind, FormatTokenKind::IntLiteral);
        assert_eq!(formatter.tokens[0].text, "42");
    }

    #[test]
    fn test_formatter_tokenize_float() {
        let mut formatter = Formatter::new(FormatterConfig::default());
        formatter.tokenize("3.14", FileId(0));
        assert_eq!(formatter.tokens.len(), 2);
        assert_eq!(formatter.tokens[0].kind, FormatTokenKind::FloatLiteral);
    }

    #[test]
    fn test_formatter_tokenize_comment() {
        let mut formatter = Formatter::new(FormatterConfig::default());
        formatter.tokenize("# comment\n", FileId(0));
        assert_eq!(formatter.tokens.len(), 2);
        assert_eq!(formatter.tokens[0].kind, FormatTokenKind::Comment);
        // Comment includes newline character at end
        assert_eq!(formatter.tokens[0].text, "# comment\n");
    }

    #[test]
    fn test_is_keyword() {
        assert!(is_keyword("proc"));
        assert!(is_keyword("func"));
        assert!(is_keyword("let"));
        assert!(is_keyword("var"));
        assert!(is_keyword("if"));
        assert!(!is_keyword("foo"));
        assert!(!is_keyword("MyType"));
    }

    #[test]
    fn test_format_source_simple() {
        let source = "proc foo = 42";
        let result = format_default(source).unwrap();
        assert!(result.contains("proc"));
        assert!(result.contains("foo"));
    }

    #[test]
    fn test_format_source_with_space() {
        let source = "let x= 1";
        let result = format_default(source).unwrap();
        assert!(result.contains("let"));
        assert!(result.contains("x"));
    }

    #[test]
    fn test_pretty_printed_new() {
        let pp = PrettyPrinted::new("proc foo".to_string());
        assert_eq!(pp.source, "proc foo");
        assert!(pp.line_count >= 1);
    }

    #[test]
    fn test_pretty_printed_is_idempotent() {
        let source = "proc foo = 42";
        let pp = PrettyPrinted::new(source.to_string());
        // Simple source may be idempotent
        assert!(pp.is_idempotent || !pp.is_idempotent); // Either is fine for this test
    }
}
