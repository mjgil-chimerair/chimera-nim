//! Documentation generation from Nim modules, comments, exported symbols, and doc config.
//!
//! This module provides HTML/JSON docs generation from exported symbols,
//! doc comments, runnable examples metadata, module index, and cross-links.

use pulldown_cmark::{html, Options, Parser, Tag, TagEnd};
#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{FileId, Span};
use std::collections::HashMap;

/// Documentation output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocFormat {
    Html,
    Json,
    Markdown,
}

impl Default for DocFormat {
    fn default() -> Self {
        DocFormat::Html
    }
}

/// A documented symbol (procedure, type, constant, etc.)
#[derive(Debug, Clone)]
pub struct DocSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub brief: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ParamDoc>,
    pub returns: Option<String>,
    pub examples: Vec<ExampleDoc>,
    pub see_also: Vec<String>,
}

/// Kind of documented symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Module,
    Proc,
    Func,
    Method,
    Iterator,
    Converter,
    Template,
    Macro,
    Type,
    Enum,
    Object,
    Ref,
    Ptr,
    Var,
    Let,
    Const,
    Import,
    Export,
}

/// A parameter documentation
#[derive(Debug, Clone)]
pub struct ParamDoc {
    pub name: String,
    pub doc_type: String,
    pub description: Option<String>,
}

/// An example with code and optional output
#[derive(Debug, Clone)]
pub struct ExampleDoc {
    pub code: String,
    pub output: Option<String>,
}

/// Module documentation
#[derive(Debug, Clone)]
pub struct ModuleDoc {
    pub name: String,
    pub file_id: FileId,
    pub module_doc: Option<String>,
    pub symbols: Vec<DocSymbol>,
    pub imports: Vec<String>,
    pub generated_at: u64,
}

/// Documentation builder configuration
#[derive(Debug, Clone)]
pub struct DocConfig {
    pub format: DocFormat,
    pub include_private: bool,
    pub include_examples: bool,
    pub link_examples: bool,
    pub syntax_highlighting: bool,
    pub show_line_numbers: bool,
    pub inline_examples: bool,
}

impl Default for DocConfig {
    fn default() -> Self {
        DocConfig {
            format: DocFormat::Html,
            include_private: false,
            include_examples: true,
            link_examples: true,
            syntax_highlighting: true,
            show_line_numbers: false,
            inline_examples: true,
        }
    }
}

/// Documentation builder
pub struct DocBuilder {
    config: DocConfig,
    modules: HashMap<String, ModuleDoc>,
    current_module: Option<String>,
}

impl DocBuilder {
    /// Create a new doc builder
    pub fn new(config: DocConfig) -> Self {
        DocBuilder {
            config,
            modules: HashMap::new(),
            current_module: None,
        }
    }

    /// Create with default config
    pub fn with_default_config() -> Self {
        Self::new(DocConfig::default())
    }

    /// Set the current module
    pub fn set_module(&mut self, name: &str, file_id: FileId) {
        let module = ModuleDoc {
            name: name.to_string(),
            file_id,
            module_doc: None,
            symbols: Vec::new(),
            imports: Vec::new(),
            generated_at: current_timestamp(),
        };
        self.modules.insert(name.to_string(), module);
        self.current_module = Some(name.to_string());
    }

    /// Add module documentation
    pub fn add_module_doc(&mut self, doc: &str) {
        if let Some(ref name) = self.current_module {
            if let Some(module) = self.modules.get_mut(name) {
                module.module_doc = Some(doc.to_string());
            }
        }
    }

    /// Add an import to current module
    pub fn add_import(&mut self, import_name: &str) {
        if let Some(ref name) = self.current_module {
            if let Some(module) = self.modules.get_mut(name) {
                module.imports.push(import_name.to_string());
            }
        }
    }

    /// Add a documented symbol
    pub fn add_symbol(&mut self, symbol: DocSymbol) {
        if let Some(ref name) = self.current_module {
            if let Some(module) = self.modules.get_mut(name) {
                module.symbols.push(symbol);
            }
        }
    }

    /// Generate documentation for all modules
    pub fn build(&self) -> Result<String, DocError> {
        match self.config.format {
            DocFormat::Html => self.build_html(),
            DocFormat::Json => self.build_json(),
            DocFormat::Markdown => self.build_markdown(),
        }
    }

    /// Build HTML documentation
    fn build_html(&self) -> Result<String, DocError> {
        let mut output = String::new();

        // Header
        output.push_str("<!DOCTYPE html>\n");
        output.push_str("<html>\n<head>\n");
        output.push_str("<meta charset=\"utf-8\">\n");
        output.push_str("<title>Nim Documentation</title>\n");
        output.push_str("<style>\n");
        output.push_str("body { font-family: -apple-system, sans-serif; max-width: 900px; margin: 0 auto; padding: 20px; }\n");
        output.push_str("h1, h2, h3 { color: #333; }\n");
        output.push_str(".symbol { margin: 20px 0; padding: 10px; border: 1px solid #ddd; }\n");
        output.push_str(".symbol-kind { color: #666; font-size: 0.9em; }\n");
        output.push_str("code { background: #f5f5f5; padding: 2px 5px; }\n");
        output.push_str("pre { background: #f5f5f5; padding: 10px; overflow-x: auto; }\n");
        output.push_str(".example { margin: 10px 0; }\n");
        output.push_str("</style>\n");
        output.push_str("</head>\n<body>\n");
        output.push_str("<h1>Nim Documentation</h1>\n");

        for (name, module) in &self.modules {
            output.push_str(&format!("<h2>Module: {}</h2>\n", name));

            if let Some(ref doc) = module.module_doc {
                let parser = Parser::new(doc);
                html::push_html(&mut output, parser);
            }

            for symbol in &module.symbols {
                output.push_str("<div class=\"symbol\">\n");
                output.push_str(&format!(
                    "<span class=\"symbol-kind\">{:?}</span>\n",
                    symbol.kind
                ));
                output.push_str(&format!("<h3>{}</h3>\n", symbol.name));

                if let Some(ref brief) = symbol.brief {
                    output.push_str(&format!("<p>{}</p>\n", escape_html(brief)));
                }

                if let Some(ref desc) = symbol.description {
                    let parser = Parser::new(desc);
                    html::push_html(&mut output, parser);
                }

                if !symbol.parameters.is_empty() {
                    output.push_str("<h4>Parameters</h4>\n<ul>\n");
                    for param in &symbol.parameters {
                        output.push_str(&format!("<li><code>{}</code>: ", param.name));
                        if let Some(ref desc) = param.description {
                            output.push_str(&escape_html(desc));
                        }
                        output.push_str("</li>\n");
                    }
                    output.push_str("</ul>\n");
                }

                if let Some(ref ret) = symbol.returns {
                    output.push_str(&format!(
                        "<p><strong>Returns:</strong> {}</p>\n",
                        escape_html(ret)
                    ));
                }

                if !symbol.examples.is_empty() && self.config.include_examples {
                    output.push_str("<h4>Examples</h4>\n");
                    for example in &symbol.examples {
                        output.push_str("<div class=\"example\"><pre><code>");
                        output.push_str(&escape_html(&example.code));
                        output.push_str("</code></pre>\n");
                        if let Some(ref output_text) = example.output {
                            output.push_str(&format!(
                                "<p><em>Output:</em> {}</p>\n",
                                escape_html(output_text)
                            ));
                        }
                        output.push_str("</div>\n");
                    }
                }

                output.push_str("</div>\n");
            }
        }

        output.push_str("</body>\n</html>\n");
        Ok(output)
    }

    /// Build JSON documentation
    fn build_json(&self) -> Result<String, DocError> {
        let mut modules_json: Vec<String> = Vec::new();

        for (_name, module) in &self.modules {
            let mut symbols_json: Vec<String> = Vec::new();
            for symbol in &module.symbols {
                let params: Vec<String> = symbol
                    .parameters
                    .iter()
                    .map(|p| format!("{{\"name\":\"{}\",\"type\":\"{}\"}}", p.name, p.doc_type))
                    .collect();
                let examples: Vec<String> = symbol
                    .examples
                    .iter()
                    .map(|e| format!("{{\"code\":\"{}\"}}", e.code))
                    .collect();

                let sym_json = format!(
                    "{{\"name\":\"{}\",\"kind\":\"{:?}\",\"brief\":\"{}\",\"params\":[{}],\"examples\":[{}]}}",
                    symbol.name,
                    symbol.kind,
                    symbol.brief.as_deref().unwrap_or(""),
                    params.join(","),
                    examples.join(",")
                );
                symbols_json.push(sym_json);
            }

            let mod_json = format!(
                "{{\"name\":\"{}\",\"doc\":\"{}\",\"symbols\":[{}]}}",
                module.name,
                module.module_doc.as_deref().unwrap_or(""),
                symbols_json.join(",")
            );
            modules_json.push(mod_json);
        }

        Ok(format!("{{\"modules\":[{}]}}", modules_json.join(",")))
    }

    /// Build Markdown documentation
    fn build_markdown(&self) -> Result<String, DocError> {
        let mut output = String::new();

        output.push_str("# Nim Documentation\n\n");

        for (name, module) in &self.modules {
            output.push_str(&format!("## Module: {}\n\n", name));

            if let Some(ref doc) = module.module_doc {
                output.push_str(&format!("{}\n\n", doc));
            }

            for symbol in &module.symbols {
                output.push_str(&format!("### {} `{:?}`\n\n", symbol.name, symbol.kind));

                if let Some(ref brief) = symbol.brief {
                    output.push_str(&format!("{}\n\n", brief));
                }

                if !symbol.parameters.is_empty() {
                    output.push_str("**Parameters:**\n");
                    for param in &symbol.parameters {
                        output.push_str(&format!("- `{}` ({})\n", param.name, param.doc_type));
                    }
                    output.push_str("\n");
                }

                if let Some(ref ret) = symbol.returns {
                    output.push_str(&format!("**Returns:** {}\n\n", ret));
                }

                if !symbol.examples.is_empty() && self.config.include_examples {
                    output.push_str("**Examples:**\n\n");
                    for example in &symbol.examples {
                        output.push_str("```nim\n");
                        output.push_str(&example.code);
                        output.push_str("\n```\n");
                        if let Some(ref out) = example.output {
                            output.push_str(&format!("Output: `{out}`\n\n"));
                        }
                    }
                }

                output.push_str("---\n\n");
            }
        }

        Ok(output)
    }

    /// Get all modules
    pub fn modules(&self) -> &HashMap<String, ModuleDoc> {
        &self.modules
    }

    /// Get module by name
    pub fn get_module(&self, name: &str) -> Option<&ModuleDoc> {
        self.modules.get(name)
    }

    /// Get current module
    pub fn current_module(&self) -> Option<&ModuleDoc> {
        self.current_module
            .as_ref()
            .and_then(|n| self.modules.get(n))
    }

    /// Get symbol count
    pub fn symbol_count(&self) -> usize {
        self.modules.values().map(|m| m.symbols.len()).sum()
    }
}

impl Default for DocBuilder {
    fn default() -> Self {
        Self::with_default_config()
    }
}

/// Documentation error
#[derive(Debug, Clone)]
pub enum DocError {
    IoError(String),
    ParseError(String),
    InvalidInput(String),
}

impl std::fmt::Display for DocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocError::IoError(s) => write!(f, "IO error: {}", s),
            DocError::ParseError(s) => write!(f, "Parse error: {}", s),
            DocError::InvalidInput(s) => write!(f, "Invalid input: {}", s),
        }
    }
}

/// Parse a doc comment and extract documentation
pub fn parse_doc_comment(comment: &str) -> DocSymbol {
    let mut symbol = DocSymbol {
        name: String::new(),
        kind: SymbolKind::Proc,
        span: Span::new(FileId(0), 0, 0),
        brief: None,
        description: None,
        parameters: Vec::new(),
        returns: None,
        examples: Vec::new(),
        see_also: Vec::new(),
    };

    // Parse Nim doc comments (##)
    let lines: Vec<&str> = comment.lines().collect();
    let mut in_description = false;
    let mut description_parts = Vec::new();

    for line in lines {
        let trimmed = line.trim_start_matches('#').trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("**") && trimmed.ends_with("**") {
            let section = &trimmed[2..trimmed.len() - 2].to_lowercase();
            match section.as_str() {
                "params" | "parameters" => {
                    in_description = false;
                }
                "return" | "returns" => {
                    in_description = false;
                    symbol.returns = Some(trimmed.to_string());
                }
                "see also" | "see" => {
                    in_description = false;
                }
                "example" | "examples" => {
                    in_description = false;
                }
                _ => {}
            }
        } else if !trimmed.starts_with("**") {
            description_parts.push(trimmed);
        }
    }

    if !description_parts.is_empty() {
        symbol.brief = description_parts.first().map(|s| s.to_string());
        symbol.description = Some(description_parts.join("\n"));
    }

    symbol
}

/// Extract parameter names from a procedure signature
pub fn extract_params(signature: &str) -> Vec<String> {
    let mut params = Vec::new();

    // Find parentheses content
    if let Some(start) = signature.find('(') {
        if let Some(end) = signature.rfind(')') {
            let params_str = &signature[start + 1..end];
            for part in params_str.split(',') {
                let part = part.trim();
                // Get first word (parameter name)
                // Handle patterns like "name: Type" or "name = default"
                if let Some(name) = part.split_whitespace().next() {
                    let name = name.trim_matches(':');
                    if !name.is_empty() && name != "=" && name != ":" {
                        params.push(name.to_string());
                    }
                }
            }
        }
    }

    params
}

/// Process markdown content into HTML
pub fn process_markdown(content: &str) -> String {
    let parser = Parser::new(content);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Escape HTML special characters
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Generate documentation index
pub fn generate_index(modules: &[&ModuleDoc]) -> String {
    let mut output = String::new();

    output.push_str("# Module Index\n\n");

    for module in modules {
        output.push_str(&format!("- [{}]({}.html) - ", module.name, module.name));
        if let Some(ref doc) = module.module_doc {
            // First line of module doc as description
            if let Some(first_line) = doc.lines().next() {
                output.push_str(&escape_html(first_line));
            }
        }
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_builder_new() {
        let builder = DocBuilder::with_default_config();
        assert_eq!(builder.symbol_count(), 0);
    }

    #[test]
    fn test_doc_builder_with_config() {
        let config = DocConfig {
            format: DocFormat::Json,
            include_private: true,
            include_examples: false,
            link_examples: false,
            syntax_highlighting: false,
            show_line_numbers: true,
            inline_examples: false,
        };
        let builder = DocBuilder::new(config);
        assert_eq!(builder.symbol_count(), 0);
    }

    #[test]
    fn test_doc_builder_set_module() {
        let mut builder = DocBuilder::with_default_config();
        builder.set_module("testmodule", FileId(0));
        assert!(builder.get_module("testmodule").is_some());
    }

    #[test]
    fn test_doc_builder_add_symbol() {
        let mut builder = DocBuilder::with_default_config();
        builder.set_module("testmodule", FileId(0));

        let symbol = DocSymbol {
            name: "testProc".to_string(),
            kind: SymbolKind::Proc,
            span: Span::new(FileId(0), 10, 20),
            brief: Some("A test procedure".to_string()),
            description: None,
            parameters: vec![],
            returns: None,
            examples: vec![],
            see_also: vec![],
        };

        builder.add_symbol(symbol);
        let module = builder.get_module("testmodule").unwrap();
        assert_eq!(module.symbols.len(), 1);
    }

    #[test]
    fn test_doc_builder_add_module_doc() {
        let mut builder = DocBuilder::with_default_config();
        builder.set_module("testmodule", FileId(0));
        builder.add_module_doc("This is the module documentation.");

        let module = builder.current_module().unwrap();
        assert!(module.module_doc.is_some());
        assert_eq!(
            module.module_doc.as_ref().unwrap(),
            "This is the module documentation."
        );
    }

    #[test]
    fn test_doc_builder_add_import() {
        let mut builder = DocBuilder::with_default_config();
        builder.set_module("testmodule", FileId(0));
        builder.add_import("strutils");
        builder.add_import("tables");

        let module = builder.current_module().unwrap();
        assert_eq!(module.imports.len(), 2);
    }

    #[test]
    fn test_doc_builder_build_html() {
        let mut builder = DocBuilder::with_default_config();
        builder.set_module("testmodule", FileId(0));
        builder.add_module_doc("Module doc here.");

        let symbol = DocSymbol {
            name: "testProc".to_string(),
            kind: SymbolKind::Proc,
            span: Span::new(FileId(0), 10, 20),
            brief: Some("A brief description".to_string()),
            description: Some("Detailed description".to_string()),
            parameters: vec![],
            returns: Some("nothing".to_string()),
            examples: vec![],
            see_also: vec![],
        };

        builder.add_symbol(symbol);

        let result = builder.build();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("<html>"));
        assert!(html.contains("testmodule"));
        assert!(html.contains("testProc"));
    }

    #[test]
    fn test_doc_builder_build_json() {
        let mut config = DocConfig::default();
        config.format = DocFormat::Json;
        let mut builder = DocBuilder::new(config);
        builder.set_module("testmodule", FileId(0));

        let result = builder.build();
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("\"modules\""));
        assert!(json.contains("testmodule"));
    }

    #[test]
    fn test_doc_builder_build_markdown() {
        let mut config = DocConfig::default();
        config.format = DocFormat::Markdown;
        let mut builder = DocBuilder::new(config);
        builder.set_module("testmodule", FileId(0));

        let result = builder.build();
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("# Nim Documentation"));
        assert!(md.contains("Module: testmodule"));
    }

    #[test]
    fn test_parse_doc_comment() {
        let comment = "## This is a brief description\n##\n## This is the detailed description.";
        let symbol = parse_doc_comment(comment);
        assert!(symbol.brief.is_some());
        assert!(symbol.description.is_some());
    }

    #[test]
    fn test_extract_params() {
        let sig = "proc test(a: int, b: string): void";
        let params = extract_params(sig);
        assert_eq!(params.len(), 2);
        assert!(params.contains(&"a".to_string()));
        assert!(params.contains(&"b".to_string()));
    }

    #[test]
    fn test_extract_params_empty() {
        let sig = "proc test(): void";
        let params = extract_params(sig);
        assert!(params.is_empty());
    }

    #[test]
    fn test_escape_html() {
        let input = "Hello <world> & \"test\"";
        let escaped = escape_html(input);
        assert_eq!(escaped, "Hello &lt;world&gt; &amp; &quot;test&quot;");
    }

    #[test]
    fn test_process_markdown() {
        let md = "Hello **world**";
        let html = process_markdown(md);
        assert!(html.contains("<strong>world</strong>"));
    }

    #[test]
    fn test_symbol_kind_variants() {
        use SymbolKind::*;
        assert_eq!(Module, SymbolKind::Module);
        assert_eq!(Proc, SymbolKind::Proc);
        assert_eq!(Func, SymbolKind::Func);
        assert_eq!(Type, SymbolKind::Type);
    }

    #[test]
    fn test_doc_format_default() {
        let format = DocFormat::default();
        assert_eq!(format, DocFormat::Html);
    }

    #[test]
    fn test_doc_error_display() {
        let err = DocError::IoError("test error".to_string());
        let s = format!("{}", err);
        assert!(s.contains("IO error"));
    }

    #[test]
    fn test_generate_index() {
        let module = ModuleDoc {
            name: "mymodule".to_string(),
            file_id: FileId(0),
            module_doc: Some("My module description".to_string()),
            symbols: vec![],
            imports: vec![],
            generated_at: 0,
        };
        let index = generate_index(&[&module]);
        assert!(index.contains("Module Index"));
        assert!(index.contains("mymodule"));
    }

    #[test]
    fn test_doc_builder_multiple_modules() {
        let mut builder = DocBuilder::with_default_config();
        builder.set_module("module1", FileId(0));
        builder.set_module("module2", FileId(1));

        assert_eq!(builder.modules.len(), 2);
    }
}
