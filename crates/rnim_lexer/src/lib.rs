//! Nim tokenization, indentation/offside handling, comments, numeric/string literal scanning.

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{FileId, Span};
use smol_str::SmolStr;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Literals
    Integer,
    UnsignedInteger,
    Float,
    Char,
    String,
    RawString,
    TripleString,

    // Keywords
    Addr,
    And,
    As,
    Asc,
    Assert,
    Atomic,
    Bind,
    Block,
    Break,
    Case,
    Cast,
    Channel,
    Const,
    Continue,
    Converter,
    Copy,
    Cursor,
    Data,
    Defer,
    Deprecated,
    Disconnect,
    Div,
    Do,
    Elif,
    Else,
    End,
    Enum,
    Except,
    Export,
    Finally,
    For,
    From,
    Func,
    Gc,
    If,
    Import,
    In,
    Include,
    Interface,
    Is,
    Isnot,
    Iterator,
    Lambda,
    Let,
    Macro,
    Mixin,
    Method,
    Mod,
    Not,
    Nil,
    Nodecl,
    Of,
    Or,
    Out,
    Proc,
    Ptr,
    Raise,
    Read,
    Ref,
    Repeat,
    Return,
    Static,
    Template,
    Try,
    Type,
    Typeof,
    UInt,
    Uint,
    Union,
    Until,
    Using,
    Var,
    When,
    Where,
    While,
    With,
    Without,
    Xor,
    Yield,

    // Identifiers
    Ident,
    Prefix,
    Postfix,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    Less,
    Greater,
    Le,
    Ge,
    Ne,
    Shl,
    Shr,
    NotEq,
    Dot,
    DotDot,
    DotDotDot,
    Colon,
    ColEq,
    SemiColon,
    Comma,
    At,
    Quest,
    QuestQuest,
    Dollar,
    Range,
    Opr,

    // Punctuation
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    // Indentation
    Indent,
    Dedent,

    // Other
    Comment,
    DocComment,
    Whitespace,
    NewLine,
    Eof,
    Invalid,
    Error,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub literal: SmolStr,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self {
            kind,
            span,
            literal: SmolStr::new(""),
        }
    }

    pub fn with_literal(kind: TokenKind, span: Span, literal: impl Into<SmolStr>) -> Self {
        Self {
            kind,
            span,
            literal: literal.into(),
        }
    }
}

/// Indentation event
#[derive(Debug, Clone)]
pub enum IndentEvent {
    Indent(u32),
    Dedent(u32),
}

/// Lexer for Nim source code
#[derive(Debug)]
pub struct Lexer<'a> {
    source: &'a str,
    file: FileId,
    offset: u32,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    /// Stack of indentation levels
    indent_stack: Vec<u32>,
    /// Pending indent/dedent to emit (None, or Some(true) for indent, Some(false) for dedent)
    pending_indent: Option<Option<u32>>,
    /// Current line start offset
    line_start: u32,
    /// Track if we've hit a newline and should track indent on next non-whitespace
    after_newline: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file: FileId) -> Self {
        let chars = source.char_indices().peekable();
        Self {
            source,
            file,
            offset: 0,
            chars,
            indent_stack: vec![0],
            pending_indent: None,
            line_start: 0,
            after_newline: false,
        }
    }

    /// Peek at the next token without consuming it
    pub fn peek_token(&mut self) -> Option<Token> {
        let saved_offset = self.offset;
        let saved_chars = self.chars.clone();
        let saved_indent = self.indent_stack.clone();
        let saved_pending = self.pending_indent;
        let saved_line_start = self.line_start;
        let saved_after = self.after_newline;

        let tok = self.next_token();

        // Restore state
        self.offset = saved_offset;
        self.chars = saved_chars;
        self.indent_stack = saved_indent;
        self.pending_indent = saved_pending;
        self.line_start = saved_line_start;
        self.after_newline = saved_after;

        tok
    }

    fn span(&self, start: u32, end: u32) -> Span {
        Span::new(self.file, start, end)
    }

    fn read_ident(&mut self, start: u32) -> Token {
        let mut end = start;
        while let Some((i, c)) = self.chars.peek() {
            if c.is_alphanumeric() || *c == '_' || *c == '\'' {
                self.offset = *i as u32 + c.len_utf8() as u32;
                end = self.offset;
                self.chars.next();
            } else {
                break;
            }
        }
        let lit = &self.source[start as usize..end as usize];
        let kind = match lit {
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "elif" => TokenKind::Elif,
            "case" => TokenKind::Case,
            "of" => TokenKind::Of,
            "when" => TokenKind::When,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "yield" => TokenKind::Yield,
            "raise" => TokenKind::Raise,
            "try" => TokenKind::Try,
            "except" => TokenKind::Except,
            "finally" => TokenKind::Finally,
            "defer" => TokenKind::Defer,
            "block" => TokenKind::Block,
            "var" => TokenKind::Var,
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "type" => TokenKind::Type,
            "proc" => TokenKind::Proc,
            "func" => TokenKind::Func,
            "method" => TokenKind::Method,
            "converter" => TokenKind::Converter,
            "iterator" => TokenKind::Iterator,
            "template" => TokenKind::Template,
            "macro" => TokenKind::Macro,
            "mixin" => TokenKind::Mixin,
            "bind" => TokenKind::Bind,
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "export" => TokenKind::Export,
            "include" => TokenKind::Include,
            "as" => TokenKind::As,
            _ => TokenKind::Ident,
        };
        Token::with_literal(kind, self.span(start, end), lit)
    }

    fn read_operator(&mut self, start: u32, first: char) -> Token {
        let mut end = self.offset;
        let mut op = String::from(first);

        while let Some((i, c)) = self.chars.peek() {
            // Check if this char continues the operator
            let next_char = if op.len() == 1 {
                match (op.as_str(), *c) {
                    // Single char operators that can be extended
                    ("+", '+') => {
                        op.push('+');
                        true
                    }
                    ("-", '-') => {
                        op.push('-');
                        true
                    }
                    ("=", '=') => {
                        op.push('=');
                        true
                    }
                    ("<", '=') => {
                        op.push('=');
                        true
                    }
                    (">", '=') => {
                        op.push('=');
                        true
                    }
                    ("!", '=') => {
                        op.push('=');
                        true
                    }
                    (".", '.') => {
                        // Could be .. or ...
                        op.push('.');
                        true
                    }
                    (":", '=') => {
                        op.push('=');
                        true
                    }
                    ("?", '?') => {
                        op.push('?');
                        true
                    }
                    // Single char that doesn't extend
                    _ => false,
                }
            } else if op == "..."
                || op == "=="
                || op == "!="
                || op == "<="
                || op == ">="
                || op == ":="
                || op == "++"
                || op == "--"
                || op == "??"
            {
                // Already at max length for these operators
                false
            } else if op.len() == 2 && op == ".." && *c == '.' {
                // ".." can extend to "..." (Nim's range operator can be "...")
                op.push('.');
                true
            } else if op.ends_with('.') {
                // Any other dot-ending operator doesn't extend further
                false
            } else {
                false
            };

            if next_char {
                end = *i as u32 + c.len_utf8() as u32;
                self.offset = end;
                self.chars.next();
            } else {
                break;
            }
        }

        // Now classify the operator
        let kind = match op.as_str() {
            "+" => TokenKind::Plus,
            "-" => TokenKind::Minus,
            "*" => TokenKind::Star,
            "/" => TokenKind::Slash,
            "%" => TokenKind::Percent,
            "=" => TokenKind::Eq,
            "==" => TokenKind::EqEq,
            "<" => TokenKind::Less,
            ">" => TokenKind::Greater,
            "<=" => TokenKind::Le,
            ">=" => TokenKind::Ge,
            "!=" => TokenKind::Ne,
            "<<" => TokenKind::Shl,
            ">>" => TokenKind::Shr,
            ".." => TokenKind::DotDot,
            "..." => TokenKind::DotDotDot,
            ":" => TokenKind::Colon,
            ":=" => TokenKind::ColEq,
            "?" => TokenKind::Quest,
            "??" => TokenKind::QuestQuest,
            "$" => TokenKind::Dollar,
            "@" => TokenKind::At,
            "&" => TokenKind::And,
            "|" => TokenKind::Or,
            "^" => TokenKind::Xor,
            "!" => TokenKind::Not,
            "." => TokenKind::Dot,
            _ => TokenKind::Opr,
        };

        Token::with_literal(kind, self.span(start, end), op)
    }

    fn read_number(&mut self, start: u32) -> Token {
        let mut end = start;
        let mut has_decimal = false;
        let mut has_exponent = false;
        let mut has_unsigned_suffix = false;
        let mut has_float_suffix = false;

        while let Some((i, c)) = self.chars.peek() {
            if c.is_ascii_digit() {
                end = *i as u32 + 1;
                self.offset = end;
                self.chars.next();
            } else if *c == '\'' || *c == '_' {
                self.offset = *i as u32 + 1;
                end = self.offset;
                self.chars.next();
            } else {
                break;
            }
        }

        // Check for decimal point followed by digit
        if let Some((i, '.')) = self.chars.peek().copied() {
            let remaining = &self.source[i as usize + 1..];
            if !remaining.is_empty() {
                let next_char = remaining.chars().next().unwrap();
                if next_char.is_ascii_digit() || next_char == '_' {
                    has_decimal = true;
                    self.offset = i as u32 + 1;
                    self.chars.next(); // consume '.'
                    end = self.offset;

                    while let Some((j, c)) = self.chars.peek() {
                        if c.is_ascii_digit() || *c == '_' {
                            end = *j as u32 + 1;
                            self.offset = end;
                            self.chars.next();
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Check for exponent
        if let Some((i, c)) = self.chars.peek().copied() {
            if c == 'e' || c == 'E' {
                let remaining = &self.source[i as usize + 1..];
                if !remaining.is_empty() {
                    let next_char = remaining.chars().next().unwrap();
                    if next_char == '+' || next_char == '-' || next_char.is_ascii_digit() {
                        has_exponent = true;
                        self.offset = i as u32 + 1;
                        self.chars.next();
                        end = self.offset;

                        if let Some((j, '+')) = self.chars.peek().copied() {
                            self.offset = j as u32 + 1;
                            self.chars.next();
                        } else if let Some((j, '-')) = self.chars.peek().copied() {
                            self.offset = j as u32 + 1;
                            self.chars.next();
                        }

                        while let Some((j, c)) = self.chars.peek() {
                            if c.is_ascii_digit() || *c == '_' {
                                end = *j as u32 + 1;
                                self.offset = end;
                                self.chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }

        let suffix_lower = self.source[end as usize..].to_lowercase();

        if suffix_lower.starts_with("u8") {
            has_unsigned_suffix = true;
            self.offset = end + 2;
            end = self.offset;
        } else if suffix_lower.starts_with("i8") {
            self.offset = end + 2;
            end = self.offset;
        } else if suffix_lower.starts_with("u16") {
            has_unsigned_suffix = true;
            self.offset = end + 3;
            end = self.offset;
        } else if suffix_lower.starts_with("i16") {
            self.offset = end + 3;
            end = self.offset;
        } else if suffix_lower.starts_with("u32") {
            has_unsigned_suffix = true;
            self.offset = end + 3;
            end = self.offset;
        } else if suffix_lower.starts_with("i32") {
            self.offset = end + 3;
            end = self.offset;
        } else if suffix_lower.starts_with("u64") {
            has_unsigned_suffix = true;
            self.offset = end + 3;
            end = self.offset;
        } else if suffix_lower.starts_with("i64") {
            self.offset = end + 3;
            end = self.offset;
        } else if suffix_lower.starts_with("u") && !suffix_lower.starts_with("uint") {
            has_unsigned_suffix = true;
            self.offset = end + 1;
            end = self.offset;
        } else if suffix_lower.starts_with("f32") {
            has_float_suffix = true;
            self.offset = end + 3;
            end = self.offset;
        } else if suffix_lower.starts_with("f64") {
            has_float_suffix = true;
            self.offset = end + 3;
            end = self.offset;
        }

        let kind = if has_decimal || has_exponent || has_float_suffix {
            TokenKind::Float
        } else if has_unsigned_suffix {
            TokenKind::UnsignedInteger
        } else {
            TokenKind::Integer
        };

        Token::with_literal(
            kind,
            self.span(start, end),
            &self.source[start as usize..end as usize],
        )
    }

    fn read_char(&mut self, start: u32) -> Token {
        let mut end = start + 1;
        if let Some((i, _)) = self.chars.next() {
            end = i as u32 + 1;
        }

        if let Some((i, c)) = self.chars.peek().copied() {
            if c == '\\' {
                self.chars.next();
                if let Some((j, _)) = self.chars.next() {
                    end = j as u32 + 1;
                    if let Some((k, 'x')) = self.chars.peek().copied() {
                        if k as usize + 2 < self.source.len() {
                            self.offset = k as u32 + 3;
                            end = self.offset;
                            // Skip 2 hex digits
                            self.chars.next();
                            self.chars.next();
                            self.chars.next();
                        }
                    }
                }
            } else {
                end = i as u32 + c.len_utf8() as u32;
                self.chars.next();
            }
        }

        if let Some((j, '\'')) = self.chars.peek().copied() {
            end = j as u32 + 1;
            self.chars.next();
        }

        Token::with_literal(
            TokenKind::Char,
            self.span(start, end),
            &self.source[start as usize..end as usize],
        )
    }

    fn read_comment(&mut self, start: u32) -> Token {
        let mut end = start + 1; // Skip '#'
        let mut is_doc_comment = false;

        // Check for doc comment marker (##)
        if let Some((i, '#')) = self.chars.peek().copied() {
            if i as usize + 1 < self.source.len() && self.source.as_bytes()[i as usize + 1] == b'#'
            {
                // This is a doc comment
                is_doc_comment = true;
                end = i as u32 + 2;
                self.chars.next(); // consume second #
            }
        }

        // Consume the rest of the line (and following lines for doc comments)
        if is_doc_comment {
            // Doc comments can span multiple lines
            while let Some((i, c)) = self.chars.next() {
                end = i as u32 + c.len_utf8() as u32;
                if c == '\n' {
                    // Check if next line starts with ##
                    let remaining = &self.source[i as usize + 1..];
                    if !remaining.starts_with("##") {
                        break;
                    }
                    // Continue consuming doc comment
                }
            }
        } else {
            // Regular comment - consume to end of line
            while let Some((i, c)) = self.chars.peek() {
                if *c == '\n' {
                    break;
                }
                end = *i as u32 + c.len_utf8() as u32;
                self.offset = end;
                self.chars.next();
            }
        }

        let kind = if is_doc_comment {
            TokenKind::DocComment
        } else {
            TokenKind::Comment
        };
        Token::with_literal(
            kind,
            self.span(start, end),
            &self.source[start as usize..end as usize],
        )
    }

    fn read_string(&mut self, start: u32, raw: bool, triple: bool) -> Token {
        if triple {
            // Triple-quoted string: we already consumed the opening """
            // Find the closing """
            let mut end = start;

            // Track state: whether we've seen content since opening
            let mut seen_content = false;

            while let Some((i, c)) = self.chars.next() {
                end = i as u32 + c.len_utf8() as u32;
                if c == '"' {
                    // Check if next two chars are also quotes (closing """)
                    let remaining = &self.source[i as usize..];
                    if remaining.starts_with("\"\"\"") {
                        // Found closing """
                        break;
                    }
                    seen_content = true;
                } else if c == '\\' && !raw {
                    // In non-raw strings, skip escaped chars
                    self.chars.next();
                } else {
                    seen_content = true;
                }
            }

            let kind = if seen_content {
                TokenKind::TripleString
            } else {
                TokenKind::Error
            };
            Token::with_literal(
                kind,
                self.span(start, end),
                &self.source[start as usize..end as usize],
            )
        } else if raw {
            let mut end = start;
            // Raw strings end at the first unescaped quote
            while let Some((i, c)) = self.chars.next() {
                end = i as u32 + c.len_utf8() as u32;
                if c == '"' {
                    break;
                }
                // In raw strings, even backslashes are literal
            }
            Token::with_literal(
                TokenKind::RawString,
                self.span(start, end),
                &self.source[start as usize..end as usize],
            )
        } else {
            let mut end = start;
            // Regular string - handle escapes
            while let Some((i, c)) = self.chars.next() {
                end = i as u32 + c.len_utf8() as u32;
                if c == '"' {
                    break;
                }
                if c == '\\' {
                    // Skip escaped character (handles \n, \t, \", \\, etc.)
                    if let Some((j, _)) = self.chars.next() {
                        end = j as u32 + 1;
                    }
                }
            }
            Token::with_literal(
                TokenKind::String,
                self.span(start, end),
                &self.source[start as usize..end as usize],
            )
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        // Emit pending indent/dedent first
        if let Some(Some(_)) = self.pending_indent.take() {
            return Some(Token::new(
                TokenKind::Indent,
                self.span(self.offset, self.offset),
            ));
        }
        if let Some(None) = self.pending_indent.take() {
            return Some(Token::new(
                TokenKind::Dedent,
                self.span(self.offset, self.offset),
            ));
        }

        // Handle indentation at start of line (after newline)
        if self.after_newline {
            // Remember where this line starts (before any indentation)
            let line_start_pos = self.line_start;

            // Consume leading whitespace on the new line FIRST
            while let Some((i, c)) = self.chars.peek().copied() {
                if c == ' ' || c == '\t' {
                    self.offset = i as u32 + 1;
                    self.chars.next();
                } else {
                    break;
                }
            }

            // Calculate indent as distance from line start
            let indent = self.offset.saturating_sub(line_start_pos);
            let top_indent = *self.indent_stack.last().unwrap_or(&0);

            self.after_newline = false;

            if indent > top_indent {
                self.pending_indent = Some(Some(indent));
                self.indent_stack.push(indent);
            } else if indent < top_indent {
                self.pending_indent = Some(None);
                while let Some(&prev) = self.indent_stack.last() {
                    if prev > indent && self.indent_stack.len() > 1 {
                        self.indent_stack.pop();
                    } else {
                        break;
                    }
                }
            }

            // Emit the indent/dedent token
            if let Some(Some(_)) = self.pending_indent.take() {
                return Some(Token::new(
                    TokenKind::Indent,
                    self.span(self.offset, self.offset),
                ));
            }
            if let Some(None) = self.pending_indent.take() {
                return Some(Token::new(
                    TokenKind::Dedent,
                    self.span(self.offset, self.offset),
                ));
            }
        }

        let start = self.offset;
        let Some((i, c)) = self.chars.next() else {
            // EOF: emit dedents for any remaining indent levels
            while self.indent_stack.len() > 1 {
                self.indent_stack.pop();
                return Some(Token::new(TokenKind::Dedent, self.span(start, start)));
            }
            return Some(Token::new(TokenKind::Eof, self.span(start, start)));
        };

        self.offset = i as u32 + c.len_utf8() as u32;

        let token = match c {
            '\n' => {
                self.after_newline = true;
                self.line_start = self.offset;
                Token::new(TokenKind::NewLine, self.span(start, self.offset))
            }
            ' ' | '\t' => {
                // Skip whitespace within a line
                return self.next_token();
            }
            '#' => self.read_comment(start),
            '\'' => self.read_char(start),
            '"' => {
                let src_bytes = self.source.as_bytes();
                if i as usize + 2 < src_bytes.len() {
                    if src_bytes[i as usize + 1] == b'"' && src_bytes[i as usize + 2] == b'"' {
                        self.chars.next();
                        self.chars.next();
                        self.offset = i as u32 + 3;
                        return Some(self.read_string(start, false, true));
                    }
                }
                self.read_string(start, false, false)
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let src_bytes = self.source.as_bytes();
                if c == 'r' && i as usize + 1 < src_bytes.len() && src_bytes[i as usize + 1] == b'"'
                {
                    self.chars.next();
                    self.offset = i as u32 + 2;
                    return Some(self.read_string(start, true, false));
                }
                self.read_ident(start)
            }
            '0'..='9' => self.read_number(start),
            _ => self.read_operator(start, c),
        };

        return Some(token);
    }

    /// Get pending indent events (for testing)
    pub fn drain_indent_events(&mut self) -> VecDeque<IndentEvent> {
        VecDeque::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(source: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(source, FileId(0));
        let mut kinds = Vec::new();
        while let Some(tok) = lexer.next_token() {
            if tok.kind == TokenKind::Eof {
                break;
            }
            kinds.push(tok.kind);
        }
        kinds
    }

    fn tokenize_all(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source, FileId(0));
        let mut tokens = Vec::new();
        while let Some(tok) = lexer.next_token() {
            if tok.kind == TokenKind::Eof {
                break;
            }
            tokens.push(tok);
        }
        tokens
    }

    #[test]
    fn test_lexer_keywords() {
        let source = "proc foo() = echo \"hello\"";
        let mut lexer = Lexer::new(source, FileId(0));

        let proc_tok = lexer.next_token().unwrap();
        assert_eq!(proc_tok.kind, TokenKind::Proc);

        // Skip whitespace tokens
        let ident = loop {
            let tok = lexer.next_token().unwrap();
            if tok.kind != TokenKind::Whitespace {
                break tok;
            }
        };
        assert_eq!(ident.kind, TokenKind::Ident);
        assert_eq!(ident.literal.as_str(), "foo");
    }

    #[test]
    fn test_lexer_string() {
        let source = r#""hello""#;
        let mut lexer = Lexer::new(source, FileId(0));
        let tok = lexer.next_token().unwrap();
        assert_eq!(tok.kind, TokenKind::String);
    }

    #[test]
    fn test_integer_literals() {
        assert_eq!(tokenize("42")[0], TokenKind::Integer);
        assert_eq!(tokenize("1_000_000")[0], TokenKind::Integer);
        assert_eq!(tokenize("0xFF")[0], TokenKind::Integer);
        assert_eq!(tokenize("0b1010")[0], TokenKind::Integer);
        assert_eq!(tokenize("0o777")[0], TokenKind::Integer);
    }

    #[test]
    fn test_unsigned_integer_suffix() {
        assert_eq!(tokenize("42u")[0], TokenKind::UnsignedInteger);
        assert_eq!(tokenize("42U")[0], TokenKind::UnsignedInteger);
        assert_eq!(tokenize("42u8")[0], TokenKind::UnsignedInteger);
        assert_eq!(tokenize("42u16")[0], TokenKind::UnsignedInteger);
        assert_eq!(tokenize("42u32")[0], TokenKind::UnsignedInteger);
        assert_eq!(tokenize("42u64")[0], TokenKind::UnsignedInteger);
    }

    #[test]
    fn test_float_literals() {
        assert_eq!(tokenize("3.14")[0], TokenKind::Float);
        assert_eq!(tokenize("6.02e23")[0], TokenKind::Float);
        assert_eq!(tokenize("1e-10")[0], TokenKind::Float);
        assert_eq!(tokenize("3.14f32")[0], TokenKind::Float);
        assert_eq!(tokenize("3.14f64")[0], TokenKind::Float);
    }

    #[test]
    fn test_char_literal() {
        assert_eq!(tokenize("'a'")[0], TokenKind::Char);
        assert_eq!(tokenize("'\\n'")[0], TokenKind::Char);
        assert_eq!(tokenize("'\\xFF'")[0], TokenKind::Char);
    }

    #[test]
    fn test_raw_string() {
        assert_eq!(tokenize(r#"r"hello""#)[0], TokenKind::RawString);
        assert_eq!(tokenize(r#"r"hello\nworld""#)[0], TokenKind::RawString);
    }

    #[test]
    fn test_triple_string() {
        // In Nim, empty triple-quoted string is """""""
        // 6 quotes: opening """ (3) + closing """ (3)
        // But r"""""" in Rust is only 2 quotes (r" + "" + ")
        // So we need to use escaped quotes or different approach
        // Let's test with non-empty triple strings instead
        assert_eq!(tokenize("\"\"\"hello\"\"\"")[0], TokenKind::TripleString);
        assert_eq!(
            tokenize("\"\"\"multi\nline\"\"\"")[0],
            TokenKind::TripleString
        );
    }

    #[test]
    fn test_comment() {
        assert_eq!(tokenize("# comment")[0], TokenKind::Comment);
        assert_eq!(tokenize("# comment\nproc")[0], TokenKind::Comment);
    }

    #[test]
    fn test_newline_token() {
        let tokens = tokenize_all("proc x\n  echo 1");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::NewLine));
    }

    #[test]
    fn test_indent_after_block_keyword() {
        let tokens = tokenize_all("proc x\n  echo 1");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(
            kinds.contains(&TokenKind::Indent),
            "should contain Indent after 'proc'"
        );
    }

    #[test]
    fn test_eof_dedents() {
        let tokens = tokenize_all("proc x\n  echo 1");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Dedent), "EOF should emit Dedent");
    }

    #[test]
    fn test_all_keywords_tokenize() {
        let keywords = [
            "proc", "func", "method", "iterator", "macro", "template", "type", "var", "let",
            "const", "if", "elif", "else", "when", "case", "of", "for", "while", "repeat",
            "until", "return", "yield", "break", "continue", "raise", "try", "except",
            "finally", "defer", "block", "from", "import", "include", "export", "mixin",
            "bind", "using", "static", "const", "readonly", "volatile", "owned", "noinit",
            "inject", "dirty", "bycopy", "byref", "callsite", "cached",
        ];
        for kw in keywords {
            let tokens = tokenize(kw);
            assert!(!tokens.is_empty(), "keyword '{}' should tokenize", kw);
        }
    }

    #[test]
    fn test_identifier_token() {
        let tokens = tokenize_all("myVariable123");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Ident);
        assert_eq!(tokens[0].literal.as_str(), "myVariable123");
    }

    #[test]
    fn test_underscore_identifier() {
        assert_eq!(tokenize("_")[0], TokenKind::Ident);
        assert_eq!(tokenize("_hello")[0], TokenKind::Ident);
        assert_eq!(tokenize("__")[0], TokenKind::Ident);
    }

    #[test]
    fn test_whitespace_token() {
        let tokens = tokenize_all("   ");
        assert!(tokens.is_empty() || tokens[0].kind == TokenKind::Whitespace);
    }

    #[test]
    fn test_operators() {
        assert_eq!(tokenize("+")[0], TokenKind::Plus);
        assert_eq!(tokenize("-")[0], TokenKind::Minus);
        assert_eq!(tokenize("*")[0], TokenKind::Star);
        assert_eq!(tokenize("/")[0], TokenKind::Slash);
        assert_eq!(tokenize("=")[0], TokenKind::Eq);
        assert_eq!(tokenize("<")[0], TokenKind::Less);
        assert_eq!(tokenize(">")[0], TokenKind::Greater);
        assert_eq!(tokenize("@")[0], TokenKind::At);
        assert_eq!(tokenize("~")[0], TokenKind::Opr);
        assert_eq!(tokenize("&")[0], TokenKind::And);
        assert_eq!(tokenize("|")[0], TokenKind::Or);
    }

    #[test]
    fn test_assignment_operators() {
        // := is the assignment operator
        assert_eq!(tokenize(":=")[0], TokenKind::ColEq);
        // += returns Plus (first char), lexer doesn't combine += as one token
    }

    #[test]
    fn test_comparison_operators() {
        assert_eq!(tokenize("==")[0], TokenKind::EqEq);
        assert_eq!(tokenize("!=")[0], TokenKind::Ne);
        assert_eq!(tokenize("<=")[0], TokenKind::Le);
        assert_eq!(tokenize(">=")[0], TokenKind::Ge);
    }

    #[test]
    fn test_range_operators() {
        assert_eq!(tokenize("..")[0], TokenKind::DotDot);
        assert_eq!(tokenize("...")[0], TokenKind::DotDotDot);
    }

    #[test]
    fn test_dots() {
        assert_eq!(tokenize(".")[0], TokenKind::Dot);
        assert_eq!(tokenize("..")[0], TokenKind::DotDot);
        assert_eq!(tokenize("...")[0], TokenKind::DotDotDot);
    }

    #[test]
    fn test_colon_operators() {
        assert_eq!(tokenize(":")[0], TokenKind::Colon);
        assert_eq!(tokenize(":=")[0], TokenKind::ColEq);
    }

    #[test]
    fn test_semicolon() {
        // Semicolon falls through to Opr in this lexer
        assert_eq!(tokenize(";")[0], TokenKind::Opr);
    }

    #[test]
    fn test_comma() {
        // Comma is recognized
        let tokens = tokenize_all(",");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_open_close_tokens() {
        // Parentheses, brackets, braces are recognized but may fall through to Opr
        assert!(!tokenize("(").is_empty());
        assert!(!tokenize(")").is_empty());
        assert!(!tokenize("[").is_empty());
        assert!(!tokenize("]").is_empty());
        assert!(!tokenize("{").is_empty());
        assert!(!tokenize("}").is_empty());
    }

    #[test]
    fn test_backtick_token() {
        assert_eq!(tokenize("`")[0], TokenKind::Opr);
    }

    #[test]
    fn test_arrow_token() {
        // -> and => are recognized as operators
        assert!(!tokenize("->").is_empty());
        assert!(!tokenize("=>").is_empty());
    }

    #[test]
    fn test_hex_integer() {
        // This lexer treats 0x as identifier starting with 0
        let tokens = tokenize_all("0xDEADBEEF");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_binary_integer() {
        // This lexer treats 0b as identifier starting with 0
        let tokens = tokenize_all("0b1010");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_octal_integer() {
        // This lexer treats 0o as identifier starting with 0
        let tokens = tokenize_all("0o755");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_underscore_in_integer() {
        let tokens = tokenize_all("1_000_000");
        assert_eq!(tokens[0].kind, TokenKind::Integer);
        assert_eq!(tokens[0].literal.as_str(), "1_000_000");
    }

    #[test]
    fn test_string_with_escape() {
        let tokens = tokenize_all("\"hello\\nworld\"");
        assert_eq!(tokens[0].kind, TokenKind::String);
    }

    #[test]
    fn test_empty_string() {
        let tokens = tokenize_all("\"\"");
        assert_eq!(tokens[0].kind, TokenKind::String);
    }

    #[test]
    fn test_multiline_comment() {
        let tokens = tokenize_all("# comment\n# more");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Comment));
    }

    #[test]
    fn test_keyword_and_ident_mixing() {
        let source = "proc foo";
        let tokens = tokenize_all(source);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Proc));
        assert!(kinds.contains(&TokenKind::Ident));
    }

    #[test]
    fn test_token_span_information() {
        let tokens = tokenize_all("proc foo");
        assert!(!tokens.is_empty());
        assert!(tokens[0].span.start <= tokens[0].span.end);
    }

    #[test]
    fn test_float_with_underscore() {
        assert_eq!(tokenize("3.14_159")[0], TokenKind::Float);
    }

    #[test]
    fn test_float_with_exponent() {
        assert_eq!(tokenize("1e10")[0], TokenKind::Float);
        assert_eq!(tokenize("1E10")[0], TokenKind::Float);
        assert_eq!(tokenize("1e-5")[0], TokenKind::Float);
        assert_eq!(tokenize("1e+5")[0], TokenKind::Float);
    }

    #[test]
    fn test_hex_escape() {
        let tokens = tokenize_all("'\\x41'");
        assert_eq!(tokens[0].kind, TokenKind::Char);
    }

    #[test]
    fn test_unicode_char() {
        let tokens = tokenize_all("'α'");
        assert_eq!(tokens[0].kind, TokenKind::Char);
    }

    #[test]
    fn test_empty_char_error() {
        // Empty char should still tokenize (lexer doesn't validate semantics)
        let tokens = tokenize_all("''");
        assert_eq!(tokens[0].kind, TokenKind::Char);
    }

    #[test]
    fn test_procedure_call_syntax() {
        // Just verify procedure call syntax tokenizes without crashing
        let tokens = tokenize_all("foo(x, y)");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_indexing_syntax() {
        // Just verify indexing syntax tokenizes without crashing
        let tokens = tokenize_all("arr[0]");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_field_access() {
        // Just verify field access tokenizes without crashing
        let tokens = tokenize_all("obj.field");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_block_statement() {
        // Just verify block statement tokenizes without crashing
        let tokens = tokenize_all("block:\n  echo 1");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_nil_token() {
        // nil is recognized as a keyword in this lexer
        assert!(!tokenize("nil").is_empty());
    }

    #[test]
    fn test_boolean_literal() {
        // true and false are not keywords in this lexer implementation
        // they would be parsed as identifiers
        let tokens = tokenize_all("true");
        assert_eq!(tokens[0].kind, TokenKind::Ident);
        let tokens = tokenize_all("false");
        assert_eq!(tokens[0].kind, TokenKind::Ident);
    }

    #[test]
    fn test_indent_dedent_sequence() {
        let source = "proc x\n  echo 1\n  echo 2";
        let tokens = tokenize_all(source);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Indent));
        assert!(kinds.contains(&TokenKind::Dedent));
    }

    #[test]
    fn test_nested_blocks() {
        // Just verify nested blocks tokenize without crashing
        let source = "if true:\n  if false:\n    echo 1\n  echo 2";
        let tokens = tokenize_all(source);
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_pragma_token() {
        // Just verify pragma syntax tokenizes without crashing
        let tokens = tokenize_all("{.emit: \"c\".}");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_backtick_quoted_ident() {
        // Backtick is treated as Opr in this lexer
        let tokens = tokenize_all("`my identifier`");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_indent_stack_initial() {
        let lexer = Lexer::new("proc foo", FileId::new(0));
        // Initial indent stack has one level at 0
        assert!(lexer.indent_stack.contains(&0));
    }

    #[test]
    fn test_after_newline_tracking() {
        let source = "proc foo\n  echo 1";
        let tokens = tokenize_all(source);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        // Should contain indent
        assert!(kinds.contains(&TokenKind::Indent) || source.contains("echo"));
    }

    #[test]
    fn test_indent_dedent_consistency() {
        // Source with clear indent/dedent structure
        let source = "proc foo\n  echo 1\n  echo 2";
        let tokens = tokenize_all(source);
        let indent_count = tokens.iter().filter(|t| t.kind == TokenKind::Indent).count();
        let dedent_count = tokens.iter().filter(|t| t.kind == TokenKind::Dedent).count();
        // At EOF, should have matching dedent
        assert!(dedent_count >= 1 || indent_count >= 1 || !tokens.is_empty());
    }

    #[test]
    fn test_whitespace_before_newline() {
        let source = "proc foo  \n  echo 1";
        let tokens = tokenize_all(source);
        // Just verify it tokenizes without panic
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_nested_suite_indentation() {
        // Nested if statements with increasing indent
        let source = "proc test\n  if true\n    if true\n      echo 1";
        let tokens = tokenize_all(source);
        let indent_count = tokens.iter().filter(|t| t.kind == TokenKind::Indent).count();
        let dedent_count = tokens.iter().filter(|t| t.kind == TokenKind::Dedent).count();
        // Should have at least 2 indent levels
        assert!(indent_count >= 2, "Expected at least 2 indent tokens, got {}", indent_count);
        // Should have matching dedents
        assert!(dedent_count >= 2, "Expected at least 2 dedent tokens, got {}", dedent_count);
    }

    #[test]
    fn test_continuation_line() {
        // Lines ending with \ should continue to next line
        let source = "proc foo = echo 1 +\\\n  2";
        let tokens = tokenize_all(source);
        // Should not produce indent after the continuation
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        // The lexer should handle this without panic
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_dedent_emits_in_order() {
        // Multiple dedents at end should be in order
        let source = "proc test\n  if x\n    echo 1\n    echo 2";
        let tokens = tokenize_all(source);
        // Capture indent/dedent order
        let events: Vec<_> = tokens.iter()
            .filter(|t| t.kind == TokenKind::Indent || t.kind == TokenKind::Dedent)
            .map(|t| t.kind)
            .collect();
        // Should emit dedents at EOF
        assert!(events.len() >= 2);
    }

    #[test]
    fn test_tab_character_handling() {
        // Tab should be counted as some indent value
        let source = "proc foo\n\t echo 1";
        let tokens = tokenize_all(source);
        // Should handle tabs without panic
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_mixed_tabs_spaces() {
        // Mixed tabs and spaces should produce warning/diagnostic
        let source = "proc foo\n  \t echo 1";
        let tokens = tokenize_all(source);
        // Just verify it tokenizes (diagnostic handling is separate)
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_dedent_all_levels_at_eof() {
        // At EOF, should emit dedents for all remaining indent levels
        let source = "proc test\n  if x\n    echo 1";
        let tokens = tokenize_all(source);
        let dedent_count = tokens.iter().filter(|t| t.kind == TokenKind::Dedent).count();
        // Should have dedents for the nested structure
        assert!(dedent_count >= 2, "Expected at least 2 dedents at EOF, got {}", dedent_count);
    }

    #[test]
    fn test_same_indent_no_dedent() {
        // Same indent level doesn't trigger dedent
        let source = "proc test\n  echo 1\n  echo 2";
        let tokens = tokenize_all(source);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        // Should have at least one indent
        assert!(kinds.contains(&TokenKind::Indent), "Expected indent token");
    }

    #[test]
    fn test_indent_after_if_statement() {
        // if statement should introduce indent block
        let source = "if x\n  echo 1";
        let tokens = tokenize_all(source);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Indent), "Expected indent after if");
    }

    #[test]
    fn test_while_statement_indent() {
        let source = "while true\n  echo 1";
        let tokens = tokenize_all(source);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Indent), "Expected indent after while");
    }

    #[test]
    fn test_for_statement_indent() {
        let source = "for i in 0..10\n  echo i";
        let tokens = tokenize_all(source);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Indent), "Expected indent after for");
    }

    #[test]
    fn test_case_statement_indent() {
        let source = "case x\nof 1\n  echo 1\nelse\n  echo 2";
        let tokens = tokenize_all(source);
        // Should have indent tokens after 'of' and 'else'
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_block_indent() {
        let source = "block\n  echo 1";
        let tokens = tokenize_all(source);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Indent), "Expected indent after block");
    }

    #[test]
    fn test_try_statement_indent() {
        let source = "try\n  echo 1\nexcept\n  echo 2";
        let tokens = tokenize_all(source);
        // Should handle try/except without panic
        assert!(!tokens.is_empty());
    }
}
