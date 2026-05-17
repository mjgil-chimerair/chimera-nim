//! Incremental compilation query database with salsa-style invalidation.
//!
//! This module provides the query infrastructure for incremental compilation:
//! - Source file queries
//! - Parsed module queries
//! - AST queries
//! - HIR queries
//! - Type checking queries
//! - Symbol resolution queries

#![allow(unused_imports)]

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{FileId, Span};
use rustc_hash::FxHashMap;
use std::sync::Arc;

mod derived;
mod input;

pub use derived::*;
pub use input::*;

/// Per-file data stored in the database
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileData {
    pub id: FileId,
    pub name: String,
    pub source: Arc<str>,
}

/// Module parsed from source
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModule {
    pub file_id: FileId,
    pub text: Arc<str>,
}

/// Module analyzed for symbols
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanalyzedModule {
    pub file_id: FileId,
    pub symbols: FxHashMap<String, SymbolEntry>,
}

/// Symbol entry in a module
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolEntry {
    pub name: String,
    pub span: Span,
    pub kind: SymbolKind,
}

/// Kind of symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Proc,
    Func,
    Method,
    Iterator,
    Converter,
    Template,
    Macro,
    Type,
    Var,
    Let,
    Const,
    Param,
    Field,
    EnumValue,
    Module,
    Import,
    Label,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_data() {
        let data = FileData {
            id: FileId::new(0),
            name: "test.nim".to_string(),
            source: Arc::from("let x = 1;"),
        };
        assert_eq!(data.name, "test.nim");
        assert_eq!(data.source.as_ref(), "let x = 1;");
    }

    #[test]
    fn test_symbol_kind() {
        assert_eq!(SymbolKind::Proc, SymbolKind::Proc);
        assert_ne!(SymbolKind::Proc, SymbolKind::Func);
        assert_ne!(SymbolKind::Type, SymbolKind::Var);
    }

    #[test]
    fn test_parsed_module() {
        let module = ParsedModule {
            file_id: FileId::new(1),
            text: Arc::from("proc foo() = discard"),
        };
        assert_eq!(module.file_id, FileId::new(1));
        assert_eq!(module.text.as_ref(), "proc foo() = discard");
    }

    #[test]
    fn test_symbol_entry() {
        let entry = SymbolEntry {
            name: "foo".to_string(),
            span: Span::new(FileId::new(0), 0, 3),
            kind: SymbolKind::Proc,
        };
        assert_eq!(entry.name, "foo");
        assert_eq!(entry.kind, SymbolKind::Proc);
    }

    #[test]
    fn test_semanalyzed_module_empty() {
        let module = SemanalyzedModule {
            file_id: FileId::new(0),
            symbols: FxHashMap::default(),
        };
        assert_eq!(module.file_id, FileId::new(0));
        assert!(module.symbols.is_empty());
    }

    #[test]
    fn test_source_file_new() {
        let source = SourceFile::new(FileId::new(0), "test.nim", "let x = 1;");
        assert_eq!(source.file_id, FileId::new(0));
        assert_eq!(source.name, "test.nim");
        assert_eq!(source.text.as_ref(), "let x = 1;");
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.nim_cache, ".nimcache");
        assert_eq!(config.target_triple, "x86_64-unknown-linux-gnu");
    }
}
