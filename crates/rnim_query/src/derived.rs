//! Derived queries for parsing and analysis.

use crate::{FileData, ParsedModule, SemanalyzedModule, SourceFile};
use rnim_span::FileId;
use std::sync::Arc;

/// Query: parse source text into a module
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseQuery {
    pub file_id: FileId,
}

/// Query: analyze a parsed module for symbols
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeQuery {
    pub file_id: FileId,
}

/// Execute parse query
pub fn parse_source(source: &SourceFile) -> ParsedModule {
    ParsedModule {
        file_id: source.file_id,
        text: source.text.clone(),
    }
}

/// Execute analyze query
pub fn analyze_module(parsed: &ParsedModule) -> SemanalyzedModule {
    SemanalyzedModule {
        file_id: parsed.file_id,
        symbols: Default::default(),
    }
}
