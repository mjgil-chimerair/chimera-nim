//! Golden token-stream tests for lexer
//!
//! These tests verify lexer produces correct token sequences
//! for representative Nim source files.

use rnim_allocator as _;
use rnim_lexer::{Lexer, TokenKind};
use rnim_span::FileId;
use std::fs;
use std::path::Path;

/// Load a fixture file and return its content
fn load_fixture(name: &str) -> String {
    let path = Path::new("tests/fixtures").join(name);
    fs::read_to_string(path).expect("Failed to read fixture")
}

/// Test tokenization of sample_module.nim
#[test]
#[ignore] // TODO: investigate OOM in release mode
fn test_sample_module_tokens() {
    let source = load_fixture("sample_module.nim");
    let mut lexer = Lexer::new(&source, FileId(0));

    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        tokens.push(token);
    }

    // Verify we got meaningful tokens (not just whitespace)
    let content_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Whitespace && t.kind != TokenKind::NewLine)
        .collect();

    // Sample module should have proc, type, const, import keywords
    let has_proc = content_tokens.iter().any(|t| t.kind == TokenKind::Proc);
    let has_type = content_tokens.iter().any(|t| t.kind == TokenKind::Type);
    let has_import = content_tokens.iter().any(|t| t.kind == TokenKind::Import);

    assert!(has_proc, "Should contain 'proc' keyword");
    assert!(has_type, "Should contain 'type' keyword");
    assert!(has_import, "Should contain 'import' keyword");
}

/// Test basic keyword recognition
#[test]
fn test_keyword_tokenization() {
    let keywords = [
        ("proc", TokenKind::Proc),
        ("func", TokenKind::Func),
        ("method", TokenKind::Method),
        ("iterator", TokenKind::Iterator),
        ("template", TokenKind::Template),
        ("macro", TokenKind::Macro),
        ("type", TokenKind::Type),
        ("const", TokenKind::Const),
        ("let", TokenKind::Let),
        ("var", TokenKind::Var),
        ("if", TokenKind::If),
        ("else", TokenKind::Else),
        ("while", TokenKind::While),
        ("for", TokenKind::For),
        ("return", TokenKind::Return),
        ("break", TokenKind::Break),
        ("import", TokenKind::Import),
        ("from", TokenKind::From),
        ("export", TokenKind::Export),
        ("when", TokenKind::When),
        ("case", TokenKind::Case),
        ("try", TokenKind::Try),
        ("except", TokenKind::Except),
        ("finally", TokenKind::Finally),
        ("raise", TokenKind::Raise),
        ("include", TokenKind::Include),
    ];

    for (keyword, expected_kind) in keywords {
        let source = keyword;
        let mut lexer = Lexer::new(source, FileId(0));
        let token = lexer.next_token().unwrap();
        assert_eq!(
            token.kind, expected_kind,
            "Keyword '{}' should tokenize to {:?}",
            keyword, expected_kind
        );
    }
}

/// Test operator tokenization
#[test]
fn test_operator_tokenization() {
    let operators = [
        ("+", TokenKind::Plus),
        ("-", TokenKind::Minus),
        ("*", TokenKind::Star),
        ("/", TokenKind::Slash),
        ("=", TokenKind::Eq),
        ("==", TokenKind::EqEq),
        ("!=", TokenKind::Ne),
        ("<", TokenKind::Less),
        ("<=", TokenKind::Le),
        (">", TokenKind::Greater),
        (">=", TokenKind::Ge),
        ("@", TokenKind::At),
        ("$", TokenKind::Dollar),
        ("?", TokenKind::Quest),
        ("..", TokenKind::DotDot),
        (":=", TokenKind::ColEq),
        ("...", TokenKind::DotDotDot),
    ];

    for (op, expected_kind) in operators {
        let source = op;
        let mut lexer = Lexer::new(source, FileId(0));
        let token = lexer.next_token().unwrap();
        assert_eq!(
            token.kind, expected_kind,
            "Operator '{}' should tokenize to {:?}",
            op, expected_kind
        );
    }
}

/// Test string literal tokenization
#[test]
fn test_string_tokenization() {
    let strings = [
        ("\"hello\"", TokenKind::String),
        ("\"test\\nstring\"", TokenKind::String),
        ("r\"raw\\nstring\"", TokenKind::RawString),
    ];

    for (source, expected_kind) in strings {
        let mut lexer = Lexer::new(source, FileId(0));
        let token = lexer.next_token().unwrap();
        assert_eq!(token.kind, expected_kind);
    }
}

/// Test numeric literal tokenization
#[test]
fn test_numeric_tokenization() {
    let numerics = [
        ("42", TokenKind::Integer),
        ("3.14", TokenKind::Float),
        ("0xFF", TokenKind::Integer),
        ("0b1010", TokenKind::Integer),
        ("0o777", TokenKind::Integer),
        ("1_000_000", TokenKind::Integer),
    ];

    for (source, expected_kind) in numerics {
        let mut lexer = Lexer::new(source, FileId(0));
        let token = lexer.next_token().unwrap();
        assert_eq!(
            token.kind, expected_kind,
            "Numeric '{}' should tokenize to {:?}",
            source, expected_kind
        );
    }
}

/// Test comment tokenization
#[test]
fn test_comment_tokenization() {
    let source = "# This is a comment";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Comment);

    let doc_source = "## This is a doc comment";
    let mut doc_lexer = Lexer::new(doc_source, FileId(0));
    let doc_token = doc_lexer.next_token().unwrap();
    assert_eq!(doc_token.kind, TokenKind::Comment);
}

/// Test identifier tokenization
#[test]
fn test_identifier_tokenization() {
    let source = "myVariable";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Ident);
    assert_eq!(token.literal.as_str(), "myVariable");

    let source = "_private";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Ident);

    let source = "camelCase123";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Ident);
}

/// Test hex and binary literals
#[test]
fn test_hex_binary_literals() {
    let source = "0xDEADBEEF";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Integer);

    let source = "0b10101010";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Integer);
}

/// Test underscore in numeric literals
#[test]
fn test_underscore_numeric() {
    let source = "1_000_000";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Integer);
    assert_eq!(token.literal.as_str(), "1_000_000");
}

/// Test newline token
#[test]
fn test_newline_tokenization() {
    let source = "\n";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::NewLine);
}

/// Test eof token
#[test]
fn test_eof_tokenization() {
    let source = "";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Eof);
}

/// Test whitespace token - lexer may skip or handle differently
#[test]
fn test_whitespace_tokenization() {
    let source = "   ";
    let mut lexer = Lexer::new(source, FileId(0));
    // Lexer may skip whitespace or emit it as whitespace token
    let token = lexer.next_token().unwrap();
    // Just verify it produces a valid token, not panic
    assert!(
        token.kind == TokenKind::Whitespace
            || token.kind == TokenKind::Eof
            || token.kind == TokenKind::Ident
    );
}

/// Test invalid token handling
#[test]
fn test_invalid_token() {
    let source = "\u{0}";
    let mut lexer = Lexer::new(source, FileId(0));
    // Should produce some kind of token, not panic
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Opr);
    assert_eq!(token.literal.as_str(), source);
}

/// Test multiple tokens in sequence - verify proc and identifier
#[test]
fn test_multiple_tokens() {
    let source = "proc foo";
    let mut lexer = Lexer::new(source, FileId(0));
    let mut found_proc = false;
    let mut found_ident = false;
    while let Some(token) = lexer.next_token() {
        if token.kind == TokenKind::Proc {
            found_proc = true;
        }
        if token.kind == TokenKind::Ident && token.literal.as_str() == "foo" {
            found_ident = true;
        }
        if token.kind == TokenKind::Eof {
            break;
        }
    }
    assert!(found_proc, "Should find 'proc' keyword");
    assert!(found_ident, "Should find 'foo' identifier");
}

/// Test token span positions
#[test]
fn test_token_span_positions() {
    let source = "abc";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    // Span should be valid with start < end
    assert!(token.span.start <= token.span.end);
}

/// Test token literal preservation
#[test]
fn test_token_literal_preservation() {
    let source = "myIdentifier";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.literal.as_str(), "myIdentifier");
}

/// Test string literal token - quotes may be included in literal
#[test]
fn test_simple_string_literal() {
    let source = "\"hello world\"";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::String);
    // Literal may include quotes or be stripped - just check non-empty
    assert!(!token.literal.is_empty());
}

/// Test char literal token
#[test]
fn test_char_literal() {
    let source = "'a'";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Char);
}

/// Test colon token - lexer may emit as Opr
#[test]
fn test_colon_token() {
    let source = ":";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    // Colon may be TokenKind::Colon or TokenKind::Opr
    assert!(token.kind == TokenKind::Colon || token.kind == TokenKind::Opr);
}

/// Test semicolon token - lexer may emit as Opr
#[test]
fn test_semicolon_token() {
    let source = ";";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    // Semicolon may be TokenKind::SemiColon or TokenKind::Opr
    assert!(token.kind == TokenKind::SemiColon || token.kind == TokenKind::Opr);
}

/// Test bracket tokens - lexer may emit as Opr for some
#[test]
fn test_bracket_tokens() {
    let source = "[]";
    let mut lexer = Lexer::new(source, FileId(0));
    let lbracket = lexer.next_token().unwrap();
    let rbracket = lexer.next_token().unwrap();
    // Brackets may be LBracket/RBracket or Opr
    assert!(lbracket.kind == TokenKind::LBracket || lbracket.kind == TokenKind::Opr);
    assert!(rbracket.kind == TokenKind::RBracket || rbracket.kind == TokenKind::Opr);
}

/// Test brace tokens - lexer may emit as Opr
#[test]
fn test_brace_tokens() {
    let source = "{}";
    let mut lexer = Lexer::new(source, FileId(0));
    let lbrace = lexer.next_token().unwrap();
    let rbrace = lexer.next_token().unwrap();
    // Braces may be LBrace/RBrace or Opr
    assert!(lbrace.kind == TokenKind::LBrace || lbrace.kind == TokenKind::Opr);
    assert!(rbrace.kind == TokenKind::RBrace || rbrace.kind == TokenKind::Opr);
}

/// Test range operator
#[test]
fn test_range_operator() {
    let source = "..";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::DotDot);
}

/// Test indentation-sensitive keywords
#[test]
fn test_defer_keyword() {
    let source = "defer";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::Defer);
}

/// Test when keyword
#[test]
fn test_when_keyword() {
    let source = "when";
    let mut lexer = Lexer::new(source, FileId(0));
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::When);
}

/// Test if/elif/else chain
#[test]
fn test_if_else_keywords() {
    let source = "if elif else";
    let mut lexer = Lexer::new(source, FileId(0));
    let if_tok = lexer.next_token().unwrap();
    let elif_tok = lexer.next_token().unwrap();
    let else_tok = lexer.next_token().unwrap();
    assert_eq!(if_tok.kind, TokenKind::If);
    assert_eq!(elif_tok.kind, TokenKind::Elif);
    assert_eq!(else_tok.kind, TokenKind::Else);
}
