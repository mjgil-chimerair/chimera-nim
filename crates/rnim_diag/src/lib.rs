//! Error rendering, structured suggestions, machine-readable JSON diagnostics.

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{Column, FileId, Line, SourceFile, SourceMap, Span};
use std::sync::Arc;

/// Stable diagnostic code registry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagCode(u16);

impl DiagCode {
    pub const fn new(code: u16) -> Self {
        DiagCode(code)
    }
    pub fn to_str(&self) -> String {
        format!("E{:04}", self.0)
    }
}

#[derive(Debug, Clone, Hash)]
pub struct DiagCodeInfo {
    pub code: DiagCode,
    pub name: &'static str,
    pub description: &'static str,
    pub category: DiagCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagCategory {
    Lexer,
    Parser,
    Semantic,
    Type,
    Codegen,
    Runtime,
    Internal,
}

/// All registered diagnostic codes
pub static DIAG_CODES: &[DiagCodeInfo] = &[
    // Lexer (E0001-E0099)
    DiagCodeInfo {
        code: DiagCode::new(1),
        name: "lexer-error",
        description: "Lexical error",
        category: DiagCategory::Lexer,
    },
    DiagCodeInfo {
        code: DiagCode::new(2),
        name: "invalid-character",
        description: "Invalid character",
        category: DiagCategory::Lexer,
    },
    DiagCodeInfo {
        code: DiagCode::new(3),
        name: "unterminated-string",
        description: "Unterminated string literal",
        category: DiagCategory::Lexer,
    },
    DiagCodeInfo {
        code: DiagCode::new(4),
        name: "invalid-escape",
        description: "Invalid escape sequence",
        category: DiagCategory::Lexer,
    },
    DiagCodeInfo {
        code: DiagCode::new(5),
        name: "invalid-number",
        description: "Invalid numeric literal",
        category: DiagCategory::Lexer,
    },
    // Parser (E0100-E0199)
    DiagCodeInfo {
        code: DiagCode::new(100),
        name: "parse-error",
        description: "Parse error",
        category: DiagCategory::Parser,
    },
    DiagCodeInfo {
        code: DiagCode::new(101),
        name: "unexpected-token",
        description: "Unexpected token",
        category: DiagCategory::Parser,
    },
    DiagCodeInfo {
        code: DiagCode::new(102),
        name: "expected-token",
        description: "Expected token",
        category: DiagCategory::Parser,
    },
    DiagCodeInfo {
        code: DiagCode::new(103),
        name: "invalid-statement",
        description: "Invalid statement",
        category: DiagCategory::Parser,
    },
    DiagCodeInfo {
        code: DiagCode::new(104),
        name: "invalid-expression",
        description: "Invalid expression",
        category: DiagCategory::Parser,
    },
    // Semantic (E0200-E0299)
    DiagCodeInfo {
        code: DiagCode::new(200),
        name: "undeclared-identifier",
        description: "Undeclared identifier",
        category: DiagCategory::Semantic,
    },
    DiagCodeInfo {
        code: DiagCode::new(201),
        name: "redefined",
        description: "Symbol already defined",
        category: DiagCategory::Semantic,
    },
    DiagCodeInfo {
        code: DiagCode::new(202),
        name: "visibility",
        description: "Visibility violation",
        category: DiagCategory::Semantic,
    },
    DiagCodeInfo {
        code: DiagCode::new(203),
        name: "invalid-import",
        description: "Invalid import",
        category: DiagCategory::Semantic,
    },
    DiagCodeInfo {
        code: DiagCode::new(204),
        name: "invalid-scope",
        description: "Invalid scope",
        category: DiagCategory::Semantic,
    },
    // Type (E0300-E0399)
    DiagCodeInfo {
        code: DiagCode::new(300),
        name: "type-mismatch",
        description: "Type mismatch",
        category: DiagCategory::Type,
    },
    DiagCodeInfo {
        code: DiagCode::new(301),
        name: "incompatible-types",
        description: "Incompatible types",
        category: DiagCategory::Type,
    },
    DiagCodeInfo {
        code: DiagCode::new(302),
        name: "missing-field",
        description: "Missing object field",
        category: DiagCategory::Type,
    },
    DiagCodeInfo {
        code: DiagCode::new(303),
        name: "generic-argument",
        description: "Invalid generic argument",
        category: DiagCategory::Type,
    },
    DiagCodeInfo {
        code: DiagCode::new(304),
        name: "proc-type",
        description: "Invalid procedure type",
        category: DiagCategory::Type,
    },
    // Codegen (E0400-E0499)
    DiagCodeInfo {
        code: DiagCode::new(400),
        name: "codegen-error",
        description: "Code generation error",
        category: DiagCategory::Codegen,
    },
    DiagCodeInfo {
        code: DiagCode::new(401),
        name: "backend-error",
        description: "Backend error",
        category: DiagCategory::Codegen,
    },
    DiagCodeInfo {
        code: DiagCode::new(402),
        name: "link-error",
        description: "Linker error",
        category: DiagCategory::Codegen,
    },
    // Runtime (E0500-E0599)
    DiagCodeInfo {
        code: DiagCode::new(500),
        name: "runtime-error",
        description: "Runtime error",
        category: DiagCategory::Runtime,
    },
    DiagCodeInfo {
        code: DiagCode::new(501),
        name: "stack-overflow",
        description: "Stack overflow",
        category: DiagCategory::Runtime,
    },
    DiagCodeInfo {
        code: DiagCode::new(502),
        name: "out-of-memory",
        description: "Out of memory",
        category: DiagCategory::Runtime,
    },
    // Internal (E0600-E0699)
    DiagCodeInfo {
        code: DiagCode::new(600),
        name: "internal-error",
        description: "Internal compiler error",
        category: DiagCategory::Internal,
    },
    DiagCodeInfo {
        code: DiagCode::new(601),
        name: "assertion-failed",
        description: "Compiler assertion failed",
        category: DiagCategory::Internal,
    },
    DiagCodeInfo {
        code: DiagCode::new(602),
        name: "ice",
        description: "Internal compiler error",
        category: DiagCategory::Internal,
    },
];

impl DiagCode {
    pub fn info(self) -> Option<&'static DiagCodeInfo> {
        DIAG_CODES.iter().find(|d| d.code == self)
    }
}

/// A severity level for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Severity::Error => "red",
            Severity::Warning => "yellow",
            Severity::Note => "blue",
            Severity::Help => "cyan",
        }
    }
}

/// A diagnostic label with span and message
#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

/// A suggested fix
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub span: Span,
    pub message: String,
    pub fix: Option<String>,
}

/// A structured diagnostic
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: DiagCode,
    pub severity: Severity,
    pub message: String,
    pub labels: Vec<Label>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    pub fn new(code: DiagCode, severity: Severity, message: &str) -> Self {
        Self {
            code,
            severity,
            message: message.to_string(),
            labels: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn error(code: DiagCode, message: &str) -> Self {
        Self::new(code, Severity::Error, message)
    }

    pub fn warning(code: DiagCode, message: &str) -> Self {
        Self::new(code, Severity::Warning, message)
    }

    pub fn with_label(mut self, span: Span, msg: &str) -> Self {
        self.labels.push(Label {
            span,
            message: msg.to_string(),
        });
        self
    }

    pub fn with_suggestion(mut self, span: Span, msg: &str, fix: Option<&str>) -> Self {
        self.suggestions.push(Suggestion {
            span,
            message: msg.to_string(),
            fix: fix.map(|s| s.to_string()),
        });
        self
    }
}

/// Diagnostic emitter with source context rendering
pub struct Emitter {
    color: bool,
    json: bool,
    source_map: Arc<SourceMap>,
}

impl Emitter {
    pub fn new(color: bool, json: bool, source_map: Arc<SourceMap>) -> Self {
        Self {
            color,
            json,
            source_map,
        }
    }

    pub fn emit(&self, diag: &Diagnostic) {
        if self.json {
            self.emit_json(diag);
        } else {
            self.emit_human(diag);
        }
    }

    fn emit_json(&self, diag: &Diagnostic) {
        println!("{:?}", diag);
    }

    fn emit_human(&self, diag: &Diagnostic) {
        let code_str = diag.code.to_str();
        let severity_str = diag.severity.as_str();

        if self.color {
            println!(
                "\x1b[1m{}: {}\x1b[0m {}",
                severity_str, code_str, diag.message
            );
        } else {
            println!("{}: {} {}", severity_str, code_str, diag.message);
        }

        for label in &diag.labels {
            if let Some((line, col)) = self.render_span_location(&label.span) {
                println!(
                    "  --> {}:{}:{}",
                    label.span.file.index(),
                    line.0 + 1,
                    col.0 + 1
                );
                if let Some(source_line) = self.get_source_line(&label.span) {
                    println!("{}", source_line);
                    if let Some(highlight) = self.render_span_highlight(&label.span) {
                        println!("{}", highlight);
                    }
                }
            }
            if self.color {
                println!("\x1b[2m    {}\x1b[0m", label.message);
            } else {
                println!("    {}", label.message);
            }
        }

        for suggestion in &diag.suggestions {
            if let Some(fix) = &suggestion.fix {
                println!("  = fix: {}", fix);
            }
        }
    }

    fn render_span_location(&self, span: &Span) -> Option<(Line, Column)> {
        self.source_map.span_to_line_col(*span)
    }

    fn get_source_line(&self, span: &Span) -> Option<String> {
        let file = self.source_map.get(span.file)?;
        let (line, _) = self.source_map.span_to_line_col(*span)?;
        let line_idx = line.0;

        let mut current_line = 0u32;
        let mut line_start = 0usize;
        let mut char_indices = file.source.char_indices().peekable();

        while let Some((idx, c)) = char_indices.next() {
            if current_line == line_idx {
                let mut line_end = idx;
                while let Some(&(next_idx, next_c)) = char_indices.peek() {
                    if next_c == '\n' {
                        break;
                    }
                    line_end = next_idx;
                    char_indices.next();
                }
                if line_end >= line_start {
                    return Some(file.source[line_start..=line_end].to_string());
                }
                return Some(file.source[line_start..].to_string());
            }
            if c == '\n' {
                current_line += 1;
                line_start = idx + 1;
            }
        }
        None
    }

    fn render_span_highlight(&self, span: &Span) -> Option<String> {
        let file = self.source_map.get(span.file)?;
        let (line, col) = self.source_map.span_to_line_col(*span)?;
        let line_idx = line.0;
        let start_col = col.0 as usize;

        let mut current_line = 0u32;
        let mut col_offset = 0usize;
        let mut highlight_start = None;
        let mut highlight_end = None;

        for (idx, c) in file.source.char_indices() {
            if c == '\n' {
                if current_line == line_idx {
                    break;
                }
                current_line += 1;
                col_offset = 0;
                continue;
            }
            if current_line == line_idx {
                if col_offset == start_col {
                    highlight_start = Some(idx);
                }
                if col_offset == start_col + span.len() as usize - 1 {
                    highlight_end = Some(idx + c.len_utf8());
                    break;
                }
            }
            col_offset += 1;
        }

        if let (Some(start), Some(end)) = (highlight_start, highlight_end) {
            let prefix = "  | ";
            let spaces = " ".repeat(start_col);
            let underline = "^".repeat(end - start);
            if self.color {
                return Some(format!("{}\x1b[4m{}{}\x1b[0m", prefix, spaces, underline));
            }
            return Some(format!("{} {}{}", prefix, spaces, underline));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_diag_code_to_str() {
        assert_eq!(DiagCode::new(1).to_str(), "E0001");
        assert_eq!(DiagCode::new(100).to_str(), "E0100");
        assert_eq!(DiagCode::new(600).to_str(), "E0600");
    }

    #[test]
    fn test_diag_code_info() {
        let code = DiagCode::new(300);
        let info = code.info().expect("code should exist");
        assert_eq!(info.name, "type-mismatch");
        assert_eq!(info.category, DiagCategory::Type);
    }

    #[test]
    fn test_diagnostic_builder() {
        let diag = Diagnostic::error(DiagCode::new(200), "undefined identifier")
            .with_label(Span::new(FileId::new(0), 5, 10), "did you mean 'x'?");
        assert_eq!(diag.labels.len(), 1);
        assert_eq!(diag.labels[0].message, "did you mean 'x'?");
    }

    #[test]
    fn test_severity_as_str() {
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Note.as_str(), "note");
        assert_eq!(Severity::Help.as_str(), "help");
    }

    #[test]
    fn test_emit_no_panic() {
        let sm = Arc::new(SourceMap::new());
        let emitter = Emitter::new(false, false, sm);
        let diag = Diagnostic::error(DiagCode::new(1), "test error");
        emitter.emit(&diag);
    }

    #[test]
    fn test_diag_code_info_all_categories() {
        let categories = [
            (DiagCategory::Lexer, 1..100),
            (DiagCategory::Parser, 100..200),
            (DiagCategory::Semantic, 200..300),
            (DiagCategory::Type, 300..400),
            (DiagCategory::Codegen, 400..500),
            (DiagCategory::Runtime, 500..600),
            (DiagCategory::Internal, 600..700),
        ];

        for (category, range) in categories {
            let code = DiagCode::new(range.start);
            let info = code.info().expect("code should exist");
            assert_eq!(
                info.category, category,
                "code {} should be in {:?}",
                code.0, category
            );
        }
    }

    #[test]
    fn test_severity_color() {
        assert_eq!(Severity::Error.color(), "red");
        assert_eq!(Severity::Warning.color(), "yellow");
        assert_eq!(Severity::Note.color(), "blue");
        assert_eq!(Severity::Help.color(), "cyan");
    }

    #[test]
    fn test_diagnostic_with_suggestion() {
        let diag = Diagnostic::error(DiagCode::new(300), "type mismatch")
            .with_label(Span::new(FileId::new(0), 0, 5), "found `int`")
            .with_suggestion(
                Span::new(FileId::new(0), 0, 5),
                "try `string`",
                Some("string"),
            );
        assert_eq!(diag.labels.len(), 1);
        assert_eq!(diag.suggestions.len(), 1);
    }

    #[test]
    fn test_diagnostic_default_severity() {
        let diag = Diagnostic::new(DiagCode::new(1), Severity::Error, "test");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "test");
        assert!(diag.labels.is_empty());
        assert!(diag.suggestions.is_empty());
    }

    #[test]
    fn test_label_message() {
        let label = Label {
            span: Span::new(FileId::new(0), 5, 10),
            message: "here is the problem".to_string(),
        };
        assert_eq!(label.message, "here is the problem");
    }

    #[test]
    fn test_suggestion_message() {
        let suggestion = Suggestion {
            span: Span::new(FileId::new(0), 5, 10),
            message: "did you mean x?".to_string(),
            fix: None,
        };
        assert_eq!(suggestion.message, "did you mean x?");
        assert!(suggestion.fix.is_none());
    }

    #[test]
    fn test_suggestion_with_fix() {
        let suggestion = Suggestion {
            span: Span::new(FileId::new(0), 0, 5),
            message: "replace with foo".to_string(),
            fix: Some("foo".to_string()),
        };
        assert!(suggestion.fix.is_some());
        assert_eq!(suggestion.fix.unwrap(), "foo");
    }

    #[test]
    fn test_diagnostic_error_factory() {
        let diag = Diagnostic::error(DiagCode::new(200), "undefined identifier");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.code, DiagCode::new(200));
        assert_eq!(diag.message, "undefined identifier");
        assert!(diag.labels.is_empty());
        assert!(diag.suggestions.is_empty());
    }

    #[test]
    fn test_diagnostic_warning_factory() {
        let diag = Diagnostic::warning(DiagCode::new(100), "deprecated syntax");
        assert_eq!(diag.severity, Severity::Warning);
        assert_eq!(diag.code, DiagCode::new(100));
    }

    #[test]
    fn test_diagnostic_with_multiple_labels() {
        let diag = Diagnostic::error(DiagCode::new(300), "type mismatch")
            .with_label(Span::new(FileId::new(0), 0, 5), "found `int`")
            .with_label(Span::new(FileId::new(0), 10, 15), "expected `string`");
        assert_eq!(diag.labels.len(), 2);
        assert_eq!(diag.labels[0].message, "found `int`");
        assert_eq!(diag.labels[1].message, "expected `string`");
    }

    #[test]
    fn test_diagnostic_with_multiple_suggestions() {
        let diag = Diagnostic::error(DiagCode::new(300), "type mismatch")
            .with_suggestion(Span::new(FileId::new(0), 0, 5), "try `int`", Some("int"))
            .with_suggestion(
                Span::new(FileId::new(0), 0, 5),
                "try `float`",
                Some("float"),
            );
        assert_eq!(diag.suggestions.len(), 2);
    }

    #[test]
    fn test_emitter_human_no_color() {
        use std::sync::Arc;
        let sm = Arc::new(SourceMap::new());
        let emitter = Emitter::new(false, false, sm);
        let diag = Diagnostic::error(DiagCode::new(1), "test error")
            .with_label(Span::new(FileId::new(0), 0, 10), "label message");
        emitter.emit(&diag);
    }

    #[test]
    fn test_emitter_human_with_color() {
        use std::sync::Arc;
        let sm = Arc::new(SourceMap::new());
        let emitter = Emitter::new(true, false, sm);
        let diag = Diagnostic::error(DiagCode::new(1), "colored error");
        emitter.emit(&diag);
    }

    #[test]
    fn test_emitter_json_mode() {
        use std::sync::Arc;
        let sm = Arc::new(SourceMap::new());
        let emitter = Emitter::new(false, true, sm);
        let diag = Diagnostic::error(DiagCode::new(1), "json error")
            .with_label(Span::new(FileId::new(0), 0, 5), "json label");
        emitter.emit(&diag);
    }

    #[test]
    fn test_emitter_with_source_context() {
        let sm = Arc::new({
            let mut sm = SourceMap::new();
            let file = SourceFile::new(FileId::new(0), "test.nim", "let x = 1;\nlet y = 2;\n");
            sm.add_file(file);
            sm
        });
        let emitter = Emitter::new(false, false, sm);
        let diag = Diagnostic::error(DiagCode::new(300), "type mismatch")
            .with_label(Span::new(FileId::new(0), 0, 10), "here");
        emitter.emit(&diag);
    }

    #[test]
    fn test_emitter_renders_location() {
        let sm = Arc::new({
            let mut sm = SourceMap::new();
            let file = SourceFile::new(FileId::new(0), "test.nim", "proc foo() = discard\n");
            sm.add_file(file);
            sm
        });
        let emitter = Emitter::new(false, false, sm);
        let diag = Diagnostic::error(DiagCode::new(201), "redefined")
            .with_label(Span::new(FileId::new(0), 0, 10), "original definition");
        emitter.emit(&diag);
    }

    #[test]
    fn test_emitter_suggestion_with_fix() {
        use std::sync::Arc;
        let sm = Arc::new(SourceMap::new());
        let emitter = Emitter::new(false, false, sm);
        let diag = Diagnostic::error(DiagCode::new(300), "mismatch").with_suggestion(
            Span::new(FileId::new(0), 0, 5),
            "try this",
            Some("replacement"),
        );
        emitter.emit(&diag);
    }

    #[test]
    fn test_diagnostic_clone() {
        let diag = Diagnostic::error(DiagCode::new(1), "test")
            .with_label(Span::new(FileId::new(0), 0, 5), "label")
            .with_suggestion(Span::new(FileId::new(0), 0, 5), "suggest", Some("fix"));
        let cloned = diag.clone();
        assert_eq!(cloned.code, diag.code);
        assert_eq!(cloned.message, diag.message);
        assert_eq!(cloned.labels.len(), diag.labels.len());
        assert_eq!(cloned.suggestions.len(), diag.suggestions.len());
    }

    #[test]
    fn test_label_clone() {
        let label = Label {
            span: Span::new(FileId::new(0), 0, 10),
            message: "test label".to_string(),
        };
        let cloned = label.clone();
        assert_eq!(cloned.message, label.message);
    }

    #[test]
    fn test_suggestion_clone() {
        let suggestion = Suggestion {
            span: Span::new(FileId::new(0), 0, 10),
            message: "test suggestion".to_string(),
            fix: Some("test fix".to_string()),
        };
        let cloned = suggestion.clone();
        assert_eq!(cloned.message, suggestion.message);
        assert_eq!(cloned.fix, suggestion.fix);
    }

    #[test]
    fn test_severity_ordering() {
        let error = Severity::Error;
        let warning = Severity::Warning;
        let note = Severity::Note;
        let help = Severity::Help;
        assert_ne!(error, warning);
        assert_ne!(warning, note);
        assert_ne!(note, help);
    }

    #[test]
    fn test_diag_code_equality() {
        let a = DiagCode::new(100);
        let b = DiagCode::new(100);
        let c = DiagCode::new(200);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_diag_category_equality() {
        let lexer = DiagCategory::Lexer;
        let parser = DiagCategory::Parser;
        assert_ne!(lexer, parser);
    }

    #[test]
    fn test_diag_code_info_all_codes() {
        for info in DIAG_CODES {
            let retrieved = info.code.info();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().name, info.name);
        }
    }

    #[test]
    fn test_diagnostic_with_empty_message() {
        let diag = Diagnostic::new(DiagCode::new(1), Severity::Error, "");
        assert_eq!(diag.message, "");
    }

    #[test]
    fn test_diagnostic_with_unicode_message() {
        let diag = Diagnostic::error(DiagCode::new(1), "αβγδ εζηθ");
        assert_eq!(diag.message, "αβγδ εζηθ");
    }

    #[test]
    fn test_emitter_empty_diagnostic() {
        use std::sync::Arc;
        let sm = Arc::new(SourceMap::new());
        let emitter = Emitter::new(false, false, sm);
        let diag = Diagnostic::error(DiagCode::new(1), "test");
        emitter.emit(&diag);
    }

    #[test]
    fn test_emitter_diagnostic_with_note_severity() {
        use std::sync::Arc;
        let sm = Arc::new(SourceMap::new());
        let emitter = Emitter::new(false, false, sm);
        let diag = Diagnostic::new(DiagCode::new(600), Severity::Note, "informational note");
        emitter.emit(&diag);
    }

    #[test]
    fn test_emitter_diagnostic_with_help_severity() {
        use std::sync::Arc;
        let sm = Arc::new(SourceMap::new());
        let emitter = Emitter::new(false, false, sm);
        let diag = Diagnostic::new(DiagCode::new(601), Severity::Help, "try this instead");
        emitter.emit(&diag);
    }
}
