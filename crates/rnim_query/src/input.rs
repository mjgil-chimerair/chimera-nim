//! Input queries - foundational inputs to the query system.

use rnim_span::{FileId, SourceMap};
use std::sync::Arc;

/// Source file input - a FileId with associated source text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub file_id: FileId,
    pub name: String,
    pub text: Arc<str>,
}

impl SourceFile {
    pub fn new(file_id: FileId, name: &str, text: &str) -> Self {
        Self {
            file_id,
            name: name.to_string(),
            text: Arc::from(text),
        }
    }
}

/// Configuration input
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub nim_cache: String,
    pub target_triple: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            nim_cache: ".nimcache".to_string(),
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
        }
    }
}
