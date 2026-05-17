//! Fuzzing infrastructure for chimera-nim.
//!
//! This module provides fuzz targets and crash reduction for lexer, parser,
//! sema, and codegen components.

#[cfg(test)]
use rnim_allocator as _;

use std::path::PathBuf;

/// Fuzz target category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuzzTarget {
    /// Lexer fuzzing
    Lexer,
    /// Parser fuzzing
    Parser,
    /// Semantic analysis fuzzing
    Sema,
    /// Codegen fuzzing
    Codegen,
}

impl FuzzTarget {
    /// Get the name of this target
    pub fn name(&self) -> &'static str {
        match self {
            FuzzTarget::Lexer => "lexer",
            FuzzTarget::Parser => "parser",
            FuzzTarget::Sema => "sema",
            FuzzTarget::Codegen => "codegen",
        }
    }
}

/// Fuzz configuration
#[derive(Debug, Clone)]
pub struct FuzzConfig {
    /// Maximum input size to accept
    pub max_input_size: usize,
    /// Number of iterations to run
    pub iterations: usize,
    /// Whether to minimize crashes
    pub minimize_crashes: bool,
}

impl Default for FuzzConfig {
    fn default() -> Self {
        Self {
            max_input_size: 1024 * 1024, // 1MB
            iterations: 10_000_000,
            minimize_crashes: true,
        }
    }
}

/// Crash report for a fuzzing failure
#[derive(Debug, Clone)]
pub struct CrashReport {
    /// The fuzz target that produced the crash
    pub target: FuzzTarget,
    /// The input that caused the crash
    pub input: Vec<u8>,
    /// The crash kind
    pub crash_kind: CrashKind,
    /// Stack trace or error message
    pub error: String,
}

/// Kind of crash
#[derive(Debug, Clone)]
pub enum CrashKind {
    /// Panic in the compiler
    Panic(String),
    /// Segmentation fault
    SegmentationFault,
    /// Timeout
    Timeout,
    /// Memory exhaustion
    OOM,
    /// Illegal instruction
    IllegalInstruction,
}

/// Crash reducer to minimize failing inputs
#[derive(Debug, Clone)]
pub struct CrashReducer {
    /// Maximum reduction iterations
    pub max_iterations: usize,
}

impl Default for CrashReducer {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
        }
    }
}

impl CrashReducer {
    /// Reduce a crashing input to its minimal form
    pub fn reduce(&self, input: &[u8], _crash_kind: &CrashKind) -> Vec<u8> {
        // Simplified reduction - in production this would use a proper
        // delta-debugging or similar algorithm
        input.to_vec()
    }
}

/// Fuzz corpus entry
#[derive(Debug, Clone)]
pub struct CorpusEntry {
    /// Path to the corpus file
    pub path: PathBuf,
    /// Coverage count
    pub coverage: u32,
    /// Input data
    pub data: Vec<u8>,
}

impl CorpusEntry {
    /// Create a new corpus entry
    pub fn new(path: PathBuf, data: Vec<u8>) -> Self {
        Self {
            path,
            coverage: 0,
            data,
        }
    }
}

/// Fuzz corpus manager
#[derive(Debug, Clone, Default)]
pub struct FuzzCorpus {
    /// Entries in the corpus
    entries: Vec<CorpusEntry>,
    /// Maximum corpus size
    max_size: usize,
}

impl FuzzCorpus {
    /// Create a new corpus with a maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_size,
        }
    }

    /// Add an entry to the corpus
    pub fn add(&mut self, entry: CorpusEntry) {
        // Simple deduplication by checking if we already have this data
        if !self.entries.iter().any(|e| e.data == entry.data) {
            self.entries.push(entry);
            // Evict oldest if over max size
            if self.entries.len() > self.max_size {
                self.entries.remove(0);
            }
        }
    }

    /// Get all entries
    pub fn entries(&self) -> &[CorpusEntry] {
        &self.entries
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Fuzz result summary
#[derive(Debug, Clone, Default)]
pub struct FuzzSummary {
    /// Total iterations run
    pub iterations: usize,
    /// Crashes found
    pub crashes: usize,
    /// Timeouts
    pub timeouts: usize,
    /// OOMs
    pub ooms: usize,
}

impl FuzzSummary {
    /// Create a new empty summary
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a crash
    pub fn record_crash(&mut self) {
        self.crashes += 1;
    }

    /// Record an iteration
    pub fn record_iteration(&mut self) {
        self.iterations += 1;
    }

    /// Check if any issues found
    pub fn has_issues(&self) -> bool {
        self.crashes > 0 || self.timeouts > 0 || self.ooms > 0
    }

    /// Record a timeout
    pub fn record_timeout(&mut self) {
        self.timeouts += 1;
    }

    /// Record an OOM
    pub fn record_oom(&mut self) {
        self.ooms += 1;
    }
}

use arbitrary::Arbitrary;

#[derive(Debug, Arbitrary)]
pub struct FuzzInput {
    pub source: String,
}

/// Fuzz the lexer with arbitrary input
pub fn fuzz_lexer(_input: FuzzInput) {
    // Fuzz targets are placeholders - full fuzzing requires
    // integration with the full compilation pipeline
    // In a production fuzzing setup, these would:
    // 1. Create a Lexer with the input source
    // 2. Iterate through all tokens
    // 3. Check for crashes or panics
}

/// Fuzz the parser with arbitrary input
pub fn fuzz_parser(_input: FuzzInput) {
    // Fuzz targets are placeholders - full fuzzing requires
    // integration with the full compilation pipeline
    // In a production fuzzing setup, these would:
    // 1. Create a Parser with the input source
    // 2. Parse to CST/AST
    // 3. Check for crashes or panics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_target_names() {
        assert_eq!(FuzzTarget::Lexer.name(), "lexer");
        assert_eq!(FuzzTarget::Parser.name(), "parser");
        assert_eq!(FuzzTarget::Sema.name(), "sema");
        assert_eq!(FuzzTarget::Codegen.name(), "codegen");
    }

    #[test]
    fn test_fuzz_config_default() {
        let config = FuzzConfig::default();
        assert_eq!(config.max_input_size, 1024 * 1024);
        assert_eq!(config.iterations, 10_000_000);
        assert!(config.minimize_crashes);
    }

    #[test]
    fn test_crash_reducer_default() {
        let reducer = CrashReducer::default();
        assert_eq!(reducer.max_iterations, 1000);
    }

    #[test]
    fn test_corpus_add_and_get() {
        let mut corpus = FuzzCorpus::new(10);
        assert!(corpus.is_empty());

        corpus.add(CorpusEntry::new(
            PathBuf::from("test1.nim"),
            b"hello".to_vec(),
        ));
        corpus.add(CorpusEntry::new(
            PathBuf::from("test2.nim"),
            b"world".to_vec(),
        ));

        assert_eq!(corpus.len(), 2);
    }

    #[test]
    fn test_corpus_deduplication() {
        let mut corpus = FuzzCorpus::new(10);
        corpus.add(CorpusEntry::new(
            PathBuf::from("test1.nim"),
            b"hello".to_vec(),
        ));
        corpus.add(CorpusEntry::new(
            PathBuf::from("test2.nim"),
            b"hello".to_vec(),
        ));

        // Should only have one entry
        assert_eq!(corpus.len(), 1);
    }

    #[test]
    fn test_corpus_eviction() {
        let mut corpus = FuzzCorpus::new(3);
        for i in 0..5 {
            corpus.add(CorpusEntry::new(
                PathBuf::from(format!("test{}.nim", i)),
                vec![i as u8],
            ));
        }

        // Should have max_size entries (3)
        assert_eq!(corpus.len(), 3);
    }

    #[test]
    fn test_fuzz_summary_new() {
        let summary = FuzzSummary::new();
        assert_eq!(summary.iterations, 0);
        assert_eq!(summary.crashes, 0);
        assert!(!summary.has_issues());
    }

    #[test]
    fn test_fuzz_summary_record_crash() {
        let mut summary = FuzzSummary::new();
        summary.record_crash();
        assert_eq!(summary.crashes, 1);
        assert!(summary.has_issues());
    }

    #[test]
    fn test_fuzz_summary_record_iteration() {
        let mut summary = FuzzSummary::new();
        summary.record_iteration();
        assert_eq!(summary.iterations, 1);
    }

    #[test]
    fn test_fuzz_summary_record_timeout() {
        let mut summary = FuzzSummary::new();
        summary.record_timeout();
        assert_eq!(summary.timeouts, 1);
        assert!(summary.has_issues());
    }

    #[test]
    fn test_fuzz_summary_record_oom() {
        let mut summary = FuzzSummary::new();
        summary.record_oom();
        assert_eq!(summary.ooms, 1);
        assert!(summary.has_issues());
    }

    #[test]
    fn test_crash_report_clone() {
        let report = CrashReport {
            target: FuzzTarget::Lexer,
            input: b"test input".to_vec(),
            crash_kind: CrashKind::Panic("test panic".to_string()),
            error: "stack trace".to_string(),
        };
        let cloned = report.clone();
        assert_eq!(cloned.target, report.target);
        assert_eq!(cloned.input, report.input);
    }

    #[test]
    fn test_crash_kind_variants() {
        let panic = CrashKind::Panic("error".to_string());
        assert!(matches!(panic, CrashKind::Panic(_)));
        assert!(matches!(
            CrashKind::SegmentationFault,
            CrashKind::SegmentationFault
        ));
        assert!(matches!(CrashKind::Timeout, CrashKind::Timeout));
        assert!(matches!(CrashKind::OOM, CrashKind::OOM));
        assert!(matches!(
            CrashKind::IllegalInstruction,
            CrashKind::IllegalInstruction
        ));
    }

    #[test]
    fn test_corpus_entry_new() {
        let entry = CorpusEntry::new(PathBuf::from("test.nim"), b"let x = 1".to_vec());
        assert_eq!(entry.coverage, 0);
        assert_eq!(entry.data, b"let x = 1");
    }

    #[test]
    fn test_corpus_is_empty() {
        let corpus = FuzzCorpus::new(10);
        assert!(corpus.is_empty());
    }

    #[test]
    fn test_crash_reducer_reduce() {
        let reducer = CrashReducer::default();
        let input = b"test input";
        let reduced = reducer.reduce(input, &CrashKind::Panic("test".to_string()));
        assert_eq!(reduced, input);
    }

    #[test]
    fn test_fuzz_target_all_variants() {
        assert!(matches!(FuzzTarget::Lexer, FuzzTarget::Lexer));
        assert!(matches!(FuzzTarget::Parser, FuzzTarget::Parser));
        assert!(matches!(FuzzTarget::Sema, FuzzTarget::Sema));
        assert!(matches!(FuzzTarget::Codegen, FuzzTarget::Codegen));
    }

    #[test]
    fn test_fuzz_config_custom() {
        let config = FuzzConfig {
            max_input_size: 2048,
            iterations: 1000,
            minimize_crashes: false,
        };
        assert_eq!(config.max_input_size, 2048);
        assert_eq!(config.iterations, 1000);
        assert!(!config.minimize_crashes);
    }

    #[test]
    fn test_fuzz_summary_multiple_crashes() {
        let mut summary = FuzzSummary::new();
        summary.record_crash();
        summary.record_crash();
        summary.record_crash();
        assert_eq!(summary.crashes, 3);
        assert!(summary.has_issues());
    }
}
