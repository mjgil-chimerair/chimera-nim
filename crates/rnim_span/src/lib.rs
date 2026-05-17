//! Source files, byte/line/column mapping, stable spans, hygiene contexts.
//!
//! This module provides:
//! - UTF-8 byte offset spans (true byte positions)
//! - Line/column mapping with grapheme awareness
//! - Virtual files for macro-generated code
//! - Include stacks for error reporting
//! - Hygiene IDs for macro expansion tracking

#[cfg(test)]
use rnim_allocator as _;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A file identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId(pub u32);

impl FileId {
    pub fn new(id: u32) -> Self {
        FileId(id)
    }
    pub fn index(&self) -> u32 {
        self.0
    }
}

/// A virtual file ID for macro-generated code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VirtualFileId(u32);

impl VirtualFileId {
    pub fn new(id: u32) -> Self {
        VirtualFileId(id)
    }
}

/// Hygiene ID for tracking macro expansion provenance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HygieneId(u32);

impl HygieneId {
    pub fn new(id: u32) -> Self {
        HygieneId(id)
    }
    pub fn root() -> Self {
        HygieneId(0)
    }
}

/// A byte offset range (UTF-8 byte positions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        Self { file, start, end }
    }

    pub fn empty(file: FileId) -> Self {
        Self {
            file,
            start: 0,
            end: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    pub fn contains(&self, other: Span) -> bool {
        self.file == other.file && self.start <= other.start && self.end >= other.end
    }

    pub fn start_line_col(&self, sm: &SourceMap) -> Option<(Line, Column)> {
        sm.span_to_line_col(*self)
    }

    pub fn end_line_col(&self, sm: &SourceMap) -> Option<(Line, Column)> {
        let file = sm.get(self.file)?;
        let mut line = Line(0);
        let mut col = Column(0);
        let mut byte_offset = 0u32;

        for c in file.source.chars() {
            if byte_offset >= self.end {
                return Some((line, col));
            }
            if c == '\n' {
                line.0 += 1;
                col = Column(0);
            } else {
                col = Column(col.0 + 1);
            }
            byte_offset += c.len_utf8() as u32;
        }
        if byte_offset >= self.end {
            Some((line, col))
        } else {
            None
        }
    }
}

/// Line number (0-indexed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Line(pub u32);

impl Line {
    pub fn new(line: u32) -> Self {
        Line(line)
    }
    pub fn index(&self) -> u32 {
        self.0
    }
}

/// Column offset (character-based, not byte)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Column(pub u32);

impl Column {
    pub fn new(col: u32) -> Self {
        Column(col)
    }
}

impl std::ops::Add<usize> for Column {
    type Output = Column;
    fn add(self, rhs: usize) -> Column {
        Column(self.0 + rhs as u32)
    }
}

/// Source file content and metadata
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub file_id: FileId,
    pub name: Arc<str>,
    pub source: Arc<str>,
    /// Optional source root for relative paths
    pub source_root: Option<Arc<str>>,
}

impl SourceFile {
    pub fn new(file_id: FileId, name: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> Self {
        Self {
            file_id,
            name: name.into(),
            source: source.into(),
            source_root: None,
        }
    }

    pub fn with_root(
        file_id: FileId,
        name: impl Into<Arc<str>>,
        source: impl Into<Arc<str>>,
        root: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            file_id,
            name: name.into(),
            source: source.into(),
            source_root: Some(root.into()),
        }
    }

    /// Get a byte slice for a span
    pub fn get_span(&self, span: Span) -> Option<&str> {
        if span.file != self.file_id {
            return None;
        }
        let start = span.start as usize;
        let end = span.end as usize;
        if start > end || end > self.source.len() {
            return None;
        }
        Some(&self.source[start..end])
    }

    /// Get the number of lines
    pub fn num_lines(&self) -> u32 {
        let count = self.source.chars().filter(|c| *c == '\n').count() as u32;
        if self.source.is_empty() {
            0
        } else if self.source.ends_with('\n') {
            count
        } else {
            count + 1
        }
    }

    /// Get line content (excluding newline)
    pub fn get_line(&self, line: Line) -> Option<&str> {
        let mut current_line = 0u32;
        let mut start = None;
        let mut end = self.source.len();

        for (i, c) in self.source.char_indices() {
            if start.is_none() && current_line == line.0 {
                start = Some(i);
            }
            if c == '\n' {
                if current_line == line.0 {
                    end = i;
                    break;
                }
                current_line += 1;
            }
        }

        if current_line < line.0 {
            return None;
        }
        if let Some(s) = start {
            Some(&self.source[s..end])
        } else {
            None
        }
    }

    /// Convert byte offset to line and column
    pub fn byte_to_line_col(&self, byte_offset: u32) -> Option<(Line, Column)> {
        let mut line = Line(0);
        let mut col = Column(0);
        let mut current_byte = 0u32;

        for c in self.source.chars() {
            if current_byte >= byte_offset {
                return Some((line, col));
            }
            if c == '\n' {
                line.0 += 1;
                col = Column(0);
            } else {
                col = col + 1;
            }
            current_byte += c.len_utf8() as u32;
        }

        if current_byte >= byte_offset {
            Some((line, col))
        } else {
            None
        }
    }

    /// Convert line and column to byte offset
    pub fn line_col_to_byte(&self, line: Line, col: Column) -> Option<u32> {
        let mut current_line = 0u32;
        let mut current_col = 0u32;
        let mut byte_offset = 0u32;

        for c in self.source.chars() {
            if current_line == line.0 && current_col == col.0 {
                return Some(byte_offset);
            }
            if c == '\n' {
                current_line += 1;
                current_col = 0;
            } else {
                current_col += 1;
            }
            byte_offset += c.len_utf8() as u32;
        }

        if current_line == line.0 && current_col == col.0 {
            Some(byte_offset)
        } else {
            None
        }
    }
}

/// Virtual file for macro-generated code
#[derive(Debug, Clone)]
pub struct VirtualFile {
    pub id: VirtualFileId,
    pub parent: Option<(FileId, Span)>,
    pub name: Arc<str>,
    pub source: Arc<str>,
    pub expansion_hygiene: HygieneId,
}

impl VirtualFile {
    pub fn new(id: VirtualFileId, name: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> Self {
        Self {
            id,
            parent: None,
            name: name.into(),
            source: source.into(),
            expansion_hygiene: HygieneId::root(),
        }
    }

    pub fn with_parent(
        id: VirtualFileId,
        name: impl Into<Arc<str>>,
        source: impl Into<Arc<str>>,
        parent: FileId,
        parent_span: Span,
    ) -> Self {
        Self {
            id,
            parent: Some((parent, parent_span)),
            name: name.into(),
            source: source.into(),
            expansion_hygiene: HygieneId::root(),
        }
    }
}

/// Include stack entry for tracking macro expansion chain
#[derive(Debug, Clone)]
pub struct IncludeEntry {
    pub file: FileId,
    pub span: Span,
    pub virtual_file: Option<VirtualFileId>,
    pub hygiene: HygieneId,
}

/// Source map holding all source files
#[derive(Debug, Default)]
pub struct SourceMap {
    files: FxHashMap<FileId, Arc<SourceFile>>,
    virtual_files: FxHashMap<VirtualFileId, Arc<VirtualFile>>,
    next_file_id: u32,
    next_virtual_id: u32,
    /// Include stack for error reporting
    include_stack: Vec<IncludeEntry>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source file and return its FileId
    pub fn add_file(&mut self, file: SourceFile) -> FileId {
        let id = file.file_id;
        self.files.insert(id, Arc::new(file));
        id
    }

    /// Create a new file with auto-generated ID
    pub fn add_file_with_name(
        &mut self,
        name: impl Into<Arc<str>>,
        source: impl Into<Arc<str>>,
    ) -> FileId {
        let id = FileId(self.next_file_id);
        self.next_file_id += 1;
        let file = SourceFile::new(id, name, source);
        self.files.insert(id, Arc::new(file));
        id
    }

    /// Add a virtual file
    pub fn add_virtual_file(&mut self, file: VirtualFile) -> VirtualFileId {
        let id = file.id;
        self.virtual_files.insert(id, Arc::new(file));
        id
    }

    /// Create a new virtual file with auto-generated ID
    pub fn add_virtual_file_with_name(
        &mut self,
        name: impl Into<Arc<str>>,
        source: impl Into<Arc<str>>,
    ) -> VirtualFileId {
        let id = VirtualFileId(self.next_virtual_id);
        self.next_virtual_id += 1;
        let file = VirtualFile::new(id, name, source);
        self.virtual_files.insert(id, Arc::new(file));
        id
    }

    /// Get a source file by ID
    pub fn get(&self, id: FileId) -> Option<&Arc<SourceFile>> {
        self.files.get(&id)
    }

    /// Get a virtual file by ID
    pub fn get_virtual(&self, id: VirtualFileId) -> Option<&Arc<VirtualFile>> {
        self.virtual_files.get(&id)
    }

    /// Convert span to line and column
    pub fn span_to_line_col(&self, span: Span) -> Option<(Line, Column)> {
        let file = self.get(span.file)?;
        file.byte_to_line_col(span.start)
    }

    /// Format span for diagnostics
    pub fn format_span(&self, span: Span) -> String {
        if let Some(file) = self.get(span.file) {
            if let Some((line, col)) = file.byte_to_line_col(span.start) {
                return format!("{}:{}:{}", file.name, line.0 + 1, col.0 + 1);
            }
        }
        format!("<unknown>:{}:{}", span.start, span.end)
    }

    /// Push an include onto the stack
    pub fn push_include(&mut self, entry: IncludeEntry) {
        self.include_stack.push(entry);
    }

    /// Pop an include from the stack
    pub fn pop_include(&mut self) -> Option<IncludeEntry> {
        self.include_stack.pop()
    }

    /// Get the current include stack
    pub fn get_include_stack(&self) -> &[IncludeEntry] {
        &self.include_stack
    }

    /// Get the include chain for a span
    #[allow(dead_code)]
    pub fn get_include_chain(&self, _file: FileId) -> Vec<IncludeEntry> {
        self.include_stack.clone()
    }
}

#[allow(dead_code)]
fn overlaps(a: Span, b: Span) -> bool {
    a.file == b.file && a.start < b.end && b.start < a.end
}

#[allow(dead_code)]
fn touches(a: Span, b: Span) -> bool {
    a.file == b.file && (a.end == b.start || b.end == a.start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_id() {
        let id = FileId::new(42);
        assert_eq!(id.index(), 42);
    }

    #[test]
    fn test_span_new() {
        let span = Span::new(FileId::new(1), 10, 20);
        assert_eq!(span.file, FileId::new(1));
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 20);
        assert_eq!(span.len(), 10);
    }

    #[test]
    fn test_span_empty() {
        let span = Span::empty(FileId::new(1));
        assert!(span.is_empty());
    }

    #[test]
    fn test_source_file_creation() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "let x = 1;\nlet y = 2;\n");
        assert_eq!(file.num_lines(), 2);
    }

    #[test]
    fn test_source_file_byte_to_line_col() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "let x = 1;\nlet y = 2;\n");
        if let Some((line, col)) = file.byte_to_line_col(0) {
            assert_eq!(line.0, 0);
            assert_eq!(col.0, 0);
        }
        if let Some((line, col)) = file.byte_to_line_col(3) {
            assert_eq!(line.0, 0);
            assert_eq!(col.0, 3);
        }
    }

    #[test]
    fn test_source_map_add_file() {
        let mut sm = SourceMap::new();
        let file = SourceFile::new(FileId::new(0), "test.nim", "test");
        let id = sm.add_file(file);
        assert_eq!(id, FileId::new(0));
        assert!(sm.get(id).is_some());
    }

    #[test]
    fn test_source_map_virtual_file() {
        let mut sm = SourceMap::new();
        let id = sm.add_virtual_file_with_name("macro", "expanded code");
        assert!(sm.get_virtual(id).is_some());
    }

    #[test]
    fn test_span_contains() {
        let outer = Span::new(FileId::new(0), 0, 100);
        let inner = Span::new(FileId::new(0), 20, 80);
        assert!(outer.contains(inner));
        assert!(!inner.contains(outer));
    }

    #[test]
    fn test_hygiene_id() {
        let id = HygieneId::new(42);
        assert_eq!(id.0, 42);
        assert_eq!(HygieneId::root().0, 0);
    }

    #[test]
    fn test_span_intersection() {
        let file = FileId::new(0);
        let span1 = Span::new(file, 0, 10);
        let span2 = Span::new(file, 5, 15);
        let span3 = Span::new(file, 10, 20);
        let span4 = Span::new(file, 20, 30);

        // span1 and span2 intersect
        assert!(span1.contains(span2) || span2.contains(span1) || overlaps(span1, span2));
        // span1 and span3 touch at boundary (span1.end == span3.start)
        assert!(touches(span1, span3));
        // span1 and span4 don't intersect
        assert!(!overlaps(span1, span4));
    }

    #[test]
    fn test_span_len() {
        let file = FileId::new(0);
        let span = Span::new(file, 5, 15);
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_span_empty_at_eof() {
        let file = FileId::new(0);
        let empty = Span::empty(file);
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_source_file_get_span() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "hello world");
        let span = Span::new(FileId::new(0), 0, 5);
        assert_eq!(file.get_span(span), Some("hello"));
    }

    #[test]
    fn test_source_file_get_span_out_of_bounds() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "hello");
        let span = Span::new(FileId::new(0), 0, 100);
        assert!(file.get_span(span).is_none());
    }

    #[test]
    fn test_line_col_to_byte() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "hello\nworld");
        // Line 0, Col 0 should be byte 0
        assert_eq!(file.line_col_to_byte(Line(0), Column(0)), Some(0));
        // Line 0, Col 5 should be byte 5 (newline)
        assert_eq!(file.line_col_to_byte(Line(0), Column(5)), Some(5));
        // Line 1, Col 0 should be byte 6 (after newline)
        assert_eq!(file.line_col_to_byte(Line(1), Column(0)), Some(6));
    }

    #[test]
    fn test_source_file_num_lines() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "line1\nline2\nline3");
        assert_eq!(file.num_lines(), 3);
    }

    #[test]
    fn test_source_file_num_lines_empty() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "");
        assert_eq!(file.num_lines(), 0);
    }

    #[test]
    fn test_source_file_num_lines_ends_with_newline() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "line1\nline2\n");
        assert_eq!(file.num_lines(), 2);
    }

    #[test]
    fn test_line_column_bounds() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "ab");
        assert!(file.line_col_to_byte(Line(10), Column(0)).is_none());
        assert!(file.line_col_to_byte(Line(0), Column(100)).is_none());
    }

    #[test]
    fn test_file_id_equality() {
        let a = FileId::new(1);
        let b = FileId::new(1);
        let c = FileId::new(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_hygiene_id_equality() {
        let a = HygieneId::new(1);
        let b = HygieneId::new(1);
        let c = HygieneId::new(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(HygieneId::root(), HygieneId::new(0));
    }

    #[test]
    fn test_virtual_file_id() {
        let id = VirtualFileId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_source_file_with_root() {
        let file = SourceFile::with_root(FileId::new(0), "test.nim", "let x = 1", "/project");
        assert!(file.source_root.is_some());
        assert_eq!(&*file.source_root.unwrap(), "/project");
    }

    #[test]
    fn test_source_file_get_span_same_file() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "hello world");
        let span = Span::new(FileId::new(0), 0, 5);
        assert_eq!(file.get_span(span), Some("hello"));
    }

    #[test]
    fn test_source_file_get_span_wrong_file() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "hello world");
        let span = Span::new(FileId::new(1), 0, 5);
        assert_eq!(file.get_span(span), None);
    }

    #[test]
    fn test_source_file_get_span_out_of_order() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "hello");
        let span = Span::new(FileId::new(0), 10, 5);
        assert_eq!(file.get_span(span), None);
    }

    #[test]
    fn test_source_file_get_line() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "line0\nline1\nline2");
        assert_eq!(file.get_line(Line(0)), Some("line0"));
        assert_eq!(file.get_line(Line(1)), Some("line1"));
        assert_eq!(file.get_line(Line(2)), Some("line2"));
    }

    #[test]
    fn test_source_file_get_line_out_of_bounds() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "line0\nline1");
        assert_eq!(file.get_line(Line(10)), None);
    }

    #[test]
    fn test_source_file_get_line_empty() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "");
        assert_eq!(file.get_line(Line(0)), None);
    }

    #[test]
    fn test_source_map_get_undefined_file() {
        let sm = SourceMap::new();
        assert!(sm.get(FileId::new(0)).is_none());
    }

    #[test]
    fn test_source_map_get_virtual_undefined() {
        let sm = SourceMap::new();
        assert!(sm.get_virtual(VirtualFileId::new(0)).is_none());
    }

    #[test]
    fn test_span_end_line_col() {
        let mut sm = SourceMap::new();
        let file = SourceFile::new(FileId::new(0), "test.nim", "let x = 1;\nlet y = 2;\n");
        sm.add_file(file);
        let span = Span::new(FileId::new(0), 0, 10);
        assert!(span.end_line_col(&sm).is_some());
    }

    #[test]
    fn test_span_end_line_col_out_of_bounds() {
        let mut sm = SourceMap::new();
        let file = SourceFile::new(FileId::new(0), "test.nim", "let");
        sm.add_file(file);
        let span = Span::new(FileId::new(0), 0, 100);
        assert!(span.end_line_col(&sm).is_none());
    }

    #[test]
    fn test_source_map_format_span() {
        let mut sm = SourceMap::new();
        let file = SourceFile::new(FileId::new(0), "test.nim", "let x = 1;\n");
        sm.add_file(file);
        let span = Span::new(FileId::new(0), 0, 3);
        let formatted = sm.format_span(span);
        assert!(formatted.contains("test.nim"));
    }

    #[test]
    fn test_source_map_format_span_unknown() {
        let sm = SourceMap::new();
        let span = Span::new(FileId::new(999), 0, 10);
        let formatted = sm.format_span(span);
        assert!(formatted.contains("<unknown>"));
    }

    #[test]
    fn test_include_stack_push_pop() {
        let mut sm = SourceMap::new();
        let entry = IncludeEntry {
            file: FileId::new(0),
            span: Span::new(FileId::new(0), 0, 10),
            virtual_file: None,
            hygiene: HygieneId::root(),
        };
        sm.push_include(entry);
        assert_eq!(sm.get_include_stack().len(), 1);
        let popped = sm.pop_include();
        assert!(popped.is_some());
        assert_eq!(sm.get_include_stack().len(), 0);
    }

    #[test]
    fn test_include_stack_empty_pop() {
        let mut sm = SourceMap::new();
        assert!(sm.pop_include().is_none());
    }

    #[test]
    fn test_include_chain() {
        let mut sm = SourceMap::new();
        let entry = IncludeEntry {
            file: FileId::new(0),
            span: Span::new(FileId::new(0), 0, 10),
            virtual_file: None,
            hygiene: HygieneId::root(),
        };
        sm.push_include(entry);
        let chain = sm.get_include_chain(FileId::new(0));
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn test_virtual_file_with_parent() {
        let file = VirtualFile::with_parent(
            VirtualFileId::new(0),
            "macro.nim",
            "expanded",
            FileId::new(0),
            Span::new(FileId::new(0), 0, 10),
        );
        assert!(file.parent.is_some());
        assert_eq!(file.expansion_hygiene, HygieneId::root());
    }

    #[test]
    fn test_source_map_span_to_line_col() {
        let mut sm = SourceMap::new();
        let file = SourceFile::new(FileId::new(0), "test.nim", "hello\nworld\n");
        sm.add_file(file);
        let span = Span::new(FileId::new(0), 0, 5);
        let result = sm.span_to_line_col(span);
        assert!(result.is_some());
        let (line, col) = result.unwrap();
        assert_eq!(line.0, 0);
        assert_eq!(col.0, 0);
    }

    #[test]
    fn test_source_map_span_to_line_col_unknown_file() {
        let sm = SourceMap::new();
        let span = Span::new(FileId::new(999), 0, 5);
        assert!(sm.span_to_line_col(span).is_none());
    }

    #[test]
    fn test_span_contains_self() {
        let span = Span::new(FileId::new(0), 5, 15);
        assert!(span.contains(span));
    }

    #[test]
    fn test_span_contains_different_file() {
        let span1 = Span::new(FileId::new(0), 0, 10);
        let span2 = Span::new(FileId::new(1), 0, 10);
        assert!(!span1.contains(span2));
    }

    #[test]
    fn test_span_contains_boundary() {
        let outer = Span::new(FileId::new(0), 0, 10);
        let exact = Span::new(FileId::new(0), 0, 10);
        assert!(outer.contains(exact));
    }

    #[test]
    fn test_column_add() {
        let col = Column::new(5) + 3;
        assert_eq!(col.0, 8);
    }

    #[test]
    fn test_line_new() {
        let line = Line::new(10);
        assert_eq!(line.index(), 10);
    }

    #[test]
    fn test_file_id_index() {
        let id = FileId::new(100);
        assert_eq!(id.index(), 100);
    }

    #[test]
    fn test_source_file_num_lines_trailing_newline() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "a\nb\nc\n");
        assert_eq!(file.num_lines(), 3);
    }

    #[test]
    fn test_source_file_num_lines_no_newline() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "abc");
        assert_eq!(file.num_lines(), 1);
    }

    #[test]
    fn test_source_file_byte_to_line_col_multiline() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "ab\ncd\nef");
        assert_eq!(file.byte_to_line_col(0), Some((Line(0), Column(0))));
        assert_eq!(file.byte_to_line_col(3), Some((Line(1), Column(0))));
        assert_eq!(file.byte_to_line_col(6), Some((Line(2), Column(0))));
        assert_eq!(file.byte_to_line_col(8), Some((Line(2), Column(2))));
    }

    #[test]
    fn test_source_file_line_col_to_byte_multiline() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "ab\ncd\nef");
        assert_eq!(file.line_col_to_byte(Line(0), Column(0)), Some(0));
        assert_eq!(file.line_col_to_byte(Line(0), Column(2)), Some(2));
        assert_eq!(file.line_col_to_byte(Line(1), Column(0)), Some(3));
        assert_eq!(file.line_col_to_byte(Line(2), Column(0)), Some(6));
    }

    #[test]
    fn test_source_file_line_col_to_byte_out_of_bounds() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "ab");
        assert_eq!(file.line_col_to_byte(Line(0), Column(10)), None);
    }

    #[test]
    fn test_source_file_byte_to_line_col_out_of_bounds() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "ab");
        assert_eq!(file.byte_to_line_col(100), None);
    }

    #[test]
    fn test_unicode_handling() {
        let file = SourceFile::new(FileId::new(0), "test.nim", "αβγδ");
        // Greek letters are 2 bytes each: α=0-1, β=2-3, γ=4-5, δ=6-7
        assert_eq!(file.byte_to_line_col(0), Some((Line(0), Column(0))));
        assert_eq!(file.byte_to_line_col(2), Some((Line(0), Column(1))));
        assert_eq!(file.byte_to_line_col(4), Some((Line(0), Column(2))));
        assert_eq!(file.line_col_to_byte(Line(0), Column(2)), Some(4));
    }
}
