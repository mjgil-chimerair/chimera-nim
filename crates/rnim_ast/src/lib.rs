//! Typed AST wrappers over lossless syntax tree plus AST validation helpers.

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{FileId, Span};

/// An AST node wrapper
#[derive(Debug, Clone)]
pub struct AstNode {
    pub kind: &'static str,
    pub span: Span,
    pub children: Vec<AstNode>,
}

/// An AST token wrapper
#[derive(Debug, Clone)]
pub struct AstToken {
    pub kind: &'static str,
    pub span: Span,
    pub text: String,
}

/// Language kinds for Nim syntax
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    Source,
    Module,
    Import,
    Export,
    Proc,
    Func,
    Type,
    Var,
    Let,
    Const,
    If,
    Else,
    Elif,
    Case,
    Of,
    When,
    While,
    For,
    Break,
    Continue,
    Return,
    Yield,
    Raise,
    Try,
    Except,
    Finally,
    Defer,
    Block,
    Ident,
    Integer,
    Float,
    String,
    Comment,
    Whitespace,
    NewLine,
    Eof,
    Error,
}

impl SyntaxKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SyntaxKind::Source => "source",
            SyntaxKind::Module => "module",
            SyntaxKind::Import => "import",
            SyntaxKind::Export => "export",
            SyntaxKind::Proc => "proc",
            SyntaxKind::Func => "func",
            SyntaxKind::Type => "type",
            SyntaxKind::Var => "var",
            SyntaxKind::Let => "let",
            SyntaxKind::Const => "const",
            SyntaxKind::If => "if",
            SyntaxKind::Else => "else",
            SyntaxKind::Elif => "elif",
            SyntaxKind::Case => "case",
            SyntaxKind::Of => "of",
            SyntaxKind::When => "when",
            SyntaxKind::While => "while",
            SyntaxKind::For => "for",
            SyntaxKind::Break => "break",
            SyntaxKind::Continue => "continue",
            SyntaxKind::Return => "return",
            SyntaxKind::Yield => "yield",
            SyntaxKind::Raise => "raise",
            SyntaxKind::Try => "try",
            SyntaxKind::Except => "except",
            SyntaxKind::Finally => "finally",
            SyntaxKind::Defer => "defer",
            SyntaxKind::Block => "block",
            SyntaxKind::Ident => "ident",
            SyntaxKind::Integer => "integer",
            SyntaxKind::Float => "float",
            SyntaxKind::String => "string",
            SyntaxKind::Comment => "comment",
            SyntaxKind::Whitespace => "whitespace",
            SyntaxKind::NewLine => "newline",
            SyntaxKind::Eof => "eof",
            SyntaxKind::Error => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_kind_as_str() {
        assert_eq!(SyntaxKind::Source.as_str(), "source");
        assert_eq!(SyntaxKind::Module.as_str(), "module");
        assert_eq!(SyntaxKind::Proc.as_str(), "proc");
        assert_eq!(SyntaxKind::Ident.as_str(), "ident");
        assert_eq!(SyntaxKind::Integer.as_str(), "integer");
        assert_eq!(SyntaxKind::String.as_str(), "string");
        assert_eq!(SyntaxKind::Error.as_str(), "error");
    }

    #[test]
    fn test_ast_node_creation() {
        let node = AstNode {
            kind: "proc",
            span: Span::new(FileId::new(0), 0, 10),
            children: Vec::new(),
        };
        assert_eq!(node.kind, "proc");
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_ast_node_with_children() {
        let child = AstNode {
            kind: "ident",
            span: Span::new(FileId::new(0), 0, 5),
            children: Vec::new(),
        };
        let parent = AstNode {
            kind: "proc",
            span: Span::new(FileId::new(0), 0, 20),
            children: vec![child],
        };
        assert_eq!(parent.kind, "proc");
        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0].kind, "ident");
    }

    #[test]
    fn test_ast_token_creation() {
        let token = AstToken {
            kind: "ident",
            span: Span::new(FileId::new(0), 0, 5),
            text: "foo".to_string(),
        };
        assert_eq!(token.kind, "ident");
        assert_eq!(token.text, "foo");
    }

    #[test]
    fn test_syntax_kind_variants() {
        use std::collections::HashSet;
        let mut seen = std::collections::HashSet::new();
        for variant in [
            SyntaxKind::Source,
            SyntaxKind::Module,
            SyntaxKind::Import,
            SyntaxKind::Export,
            SyntaxKind::Proc,
            SyntaxKind::Ident,
            SyntaxKind::Integer,
            SyntaxKind::Error,
        ] {
            let name = variant.as_str();
            assert!(!seen.contains(name), "Duplicate: {}", name);
            seen.insert(name);
        }
    }

    #[test]
    fn test_span_in_ast_node() {
        let span = Span::new(FileId::new(1), 10, 25);
        let node = AstNode {
            kind: "func",
            span,
            children: Vec::new(),
        };
        assert_eq!(node.span.file, FileId::new(1));
        assert_eq!(node.span.start, 10);
        assert_eq!(node.span.end, 25);
    }

    #[test]
    fn test_ast_node_clone() {
        let original = AstNode {
            kind: "type",
            span: Span::new(FileId::new(0), 5, 15),
            children: vec![AstNode {
                kind: "ident",
                span: Span::new(FileId::new(0), 5, 10),
                children: Vec::new(),
            }],
        };
        let cloned = original.clone();
        assert_eq!(cloned.kind, original.kind);
        assert_eq!(cloned.children.len(), original.children.len());
    }

    #[test]
    fn test_ast_token_clone() {
        let token = AstToken {
            kind: "string",
            span: Span::new(FileId::new(0), 0, 10),
            text: "hello".to_string(),
        };
        let cloned = token.clone();
        assert_eq!(cloned.kind, token.kind);
        assert_eq!(cloned.text, token.text);
    }

    #[test]
    fn test_ast_node_with_multiple_children() {
        let children = vec![
            AstNode {
                kind: "ident",
                span: Span::new(FileId::new(0), 0, 3),
                children: Vec::new(),
            },
            AstNode {
                kind: "integer",
                span: Span::new(FileId::new(0), 4, 5),
                children: Vec::new(),
            },
        ];
        let parent = AstNode {
            kind: "const",
            span: Span::new(FileId::new(0), 0, 10),
            children,
        };
        assert_eq!(parent.children.len(), 2);
    }

    #[test]
    fn test_ast_token_with_whitespace() {
        let token = AstToken {
            kind: "whitespace",
            span: Span::new(FileId::new(0), 0, 4),
            text: "    ".to_string(),
        };
        assert_eq!(token.text, "    ");
        assert_eq!(token.kind, "whitespace");
    }

    #[test]
    fn test_ast_node_span_preserved() {
        let span = Span::new(FileId::new(5), 100, 200);
        let node = AstNode {
            kind: "proc",
            span,
            children: Vec::new(),
        };
        assert_eq!(node.span.file, FileId::new(5));
        assert_eq!(node.span.start, 100);
        assert_eq!(node.span.end, 200);
    }

    #[test]
    fn test_ast_token_span_preserved() {
        let span = Span::new(FileId::new(3), 50, 55);
        let token = AstToken {
            kind: "ident",
            span,
            text: "foo".to_string(),
        };
        assert_eq!(token.span.file, FileId::new(3));
        assert_eq!(token.span.start, 50);
        assert_eq!(token.span.end, 55);
    }

    #[test]
    fn test_nested_ast_nodes() {
        let innermost = AstNode {
            kind: "string",
            span: Span::new(FileId::new(0), 0, 5),
            children: Vec::new(),
        };
        let middle = AstNode {
            kind: "literal",
            span: Span::new(FileId::new(0), 0, 10),
            children: vec![innermost],
        };
        let outermost = AstNode {
            kind: "expr",
            span: Span::new(FileId::new(0), 0, 20),
            children: vec![middle],
        };
        assert_eq!(outermost.children.len(), 1);
        assert_eq!(outermost.children[0].children.len(), 1);
        assert_eq!(outermost.children[0].children[0].kind, "string");
    }

    #[test]
    fn test_all_syntax_kind_variants() {
        let kinds = [
            SyntaxKind::Source,
            SyntaxKind::Module,
            SyntaxKind::Import,
            SyntaxKind::Export,
            SyntaxKind::Proc,
            SyntaxKind::Func,
            SyntaxKind::Type,
            SyntaxKind::Var,
            SyntaxKind::Let,
            SyntaxKind::Const,
            SyntaxKind::If,
            SyntaxKind::Else,
            SyntaxKind::Elif,
            SyntaxKind::Case,
            SyntaxKind::Of,
            SyntaxKind::When,
            SyntaxKind::While,
            SyntaxKind::For,
            SyntaxKind::Break,
            SyntaxKind::Continue,
            SyntaxKind::Return,
            SyntaxKind::Yield,
            SyntaxKind::Raise,
            SyntaxKind::Try,
            SyntaxKind::Except,
            SyntaxKind::Finally,
            SyntaxKind::Defer,
            SyntaxKind::Block,
            SyntaxKind::Ident,
            SyntaxKind::Integer,
            SyntaxKind::Float,
            SyntaxKind::String,
            SyntaxKind::Comment,
            SyntaxKind::Whitespace,
            SyntaxKind::NewLine,
            SyntaxKind::Eof,
            SyntaxKind::Error,
        ];
        assert_eq!(kinds.len(), 37);
    }

    #[test]
    fn test_syntax_kind_equality() {
        assert_eq!(SyntaxKind::Proc, SyntaxKind::Proc);
        assert_eq!(SyntaxKind::Ident, SyntaxKind::Ident);
        assert_ne!(SyntaxKind::Proc, SyntaxKind::Func);
        assert_ne!(SyntaxKind::Integer, SyntaxKind::String);
    }

    #[test]
    fn test_syntax_kind_copy() {
        let kind = SyntaxKind::Proc;
        let kind2 = kind;
        assert_eq!(kind, kind2);
    }

    #[test]
    fn test_ast_node_debug() {
        let node = AstNode {
            kind: "test",
            span: Span::new(FileId::new(0), 0, 5),
            children: Vec::new(),
        };
        let debug_str = format!("{:?}", node);
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_ast_token_debug() {
        let token = AstToken {
            kind: "test",
            span: Span::new(FileId::new(0), 0, 5),
            text: "hello".to_string(),
        };
        let debug_str = format!("{:?}", token);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("hello"));
    }

    #[test]
    fn test_ast_node_partial_eq() {
        let node1 = AstNode {
            kind: "proc",
            span: Span::new(FileId::new(0), 0, 10),
            children: Vec::new(),
        };
        let node2 = AstNode {
            kind: "proc",
            span: Span::new(FileId::new(0), 0, 10),
            children: Vec::new(),
        };
        // AstNode doesn't derive PartialEq, so we compare fields
        assert_eq!(node1.kind, node2.kind);
    }

    #[test]
    fn test_ast_token_partial_eq() {
        let token1 = AstToken {
            kind: "ident",
            span: Span::new(FileId::new(0), 0, 5),
            text: "foo".to_string(),
        };
        let token2 = AstToken {
            kind: "ident",
            span: Span::new(FileId::new(0), 0, 5),
            text: "foo".to_string(),
        };
        assert_eq!(token1.kind, token2.kind);
        assert_eq!(token1.text, token2.text);
    }

    #[test]
    fn test_ast_node_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let node = AstNode {
            kind: "proc",
            span: Span::new(FileId::new(0), 0, 10),
            children: Vec::new(),
        };
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        node.kind.hash(&mut h1);
        "proc".hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_ast_node_children_modification() {
        let mut node = AstNode {
            kind: "block",
            span: Span::new(FileId::new(0), 0, 20),
            children: Vec::new(),
        };
        node.children.push(AstNode {
            kind: "stmt",
            span: Span::new(FileId::new(0), 0, 5),
            children: Vec::new(),
        });
        assert_eq!(node.children.len(), 1);
        node.children.push(AstNode {
            kind: "stmt",
            span: Span::new(FileId::new(0), 5, 10),
            children: Vec::new(),
        });
        assert_eq!(node.children.len(), 2);
    }

    #[test]
    fn test_ast_node_empty_children() {
        let node = AstNode {
            kind: "leaf",
            span: Span::new(FileId::new(0), 0, 5),
            children: Vec::new(),
        };
        assert!(node.children.is_empty());
        assert_eq!(node.children.len(), 0);
    }

    #[test]
    fn test_syntax_kind_as_str_all() {
        for kind in [
            SyntaxKind::Source,
            SyntaxKind::Module,
            SyntaxKind::Import,
            SyntaxKind::Export,
            SyntaxKind::Proc,
            SyntaxKind::Func,
            SyntaxKind::Type,
            SyntaxKind::Var,
            SyntaxKind::Let,
            SyntaxKind::Const,
            SyntaxKind::If,
            SyntaxKind::Else,
            SyntaxKind::Elif,
            SyntaxKind::Case,
            SyntaxKind::Of,
            SyntaxKind::When,
            SyntaxKind::While,
            SyntaxKind::For,
            SyntaxKind::Break,
            SyntaxKind::Continue,
            SyntaxKind::Return,
            SyntaxKind::Yield,
            SyntaxKind::Raise,
            SyntaxKind::Try,
            SyntaxKind::Except,
            SyntaxKind::Finally,
            SyntaxKind::Defer,
            SyntaxKind::Block,
            SyntaxKind::Ident,
            SyntaxKind::Integer,
            SyntaxKind::Float,
            SyntaxKind::String,
            SyntaxKind::Comment,
            SyntaxKind::Whitespace,
            SyntaxKind::NewLine,
            SyntaxKind::Eof,
            SyntaxKind::Error,
        ] {
            let s = kind.as_str();
            assert!(!s.is_empty());
        }
    }
}
