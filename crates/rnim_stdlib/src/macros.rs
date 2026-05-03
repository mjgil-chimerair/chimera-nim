//! Macros module support.
//!
//! This module provides macro support including `macro macro`,
//! `bindSym`, `quote do`, and other macro-related constructs.

use rnim_span::{FileId, Span};

/// Result of macro expansion
#[derive(Debug, Clone)]
pub struct MacroResult {
    pub expanded_code: String,
    pub success: bool,
    pub errors: Vec<MacroError>,
}

#[derive(Debug, Clone)]
pub struct MacroError {
    pub message: String,
    pub span: Span,
}

impl MacroResult {
    pub fn new(expanded_code: String) -> Self {
        MacroResult {
            expanded_code,
            success: true,
            errors: Vec::new(),
        }
    }

    pub fn error(message: &str, span: Span) -> Self {
        MacroResult {
            expanded_code: String::new(),
            success: false,
            errors: vec![MacroError {
                message: message.to_string(),
                span,
            }],
        }
    }
}

/// Macro expansion engine
#[derive(Debug, Clone, Default)]
pub struct MacroExpander {
    hygiene_enabled: bool,
}

impl MacroExpander {
    pub fn new() -> Self {
        MacroExpander {
            hygiene_enabled: true,
        }
    }

    /// Expand a macro call
    pub fn expand(&self, macro_name: &str, args: &[String], _span: Span) -> MacroResult {
        match macro_name {
            "macro" => self.expand_macro_macro(args),
            "bindSym" => self.expand_bind_sym(args),
            "quote" => self.expand_quote(args),
            _ => MacroResult::error(
                &format!("unknown macro: {}", macro_name),
                Span::new(FileId(0), 0, 0),
            ),
        }
    }

    fn expand_macro_macro(&self, args: &[String]) -> MacroResult {
        // Handle macro definition
        if args.is_empty() {
            return MacroResult::error("macro requires name and body", Span::new(FileId(0), 0, 0));
        }
        MacroResult::new(format!("macro {}*() = discard", args[0]))
    }

    fn expand_bind_sym(&self, args: &[String]) -> MacroResult {
        // bindSym creates a reference to a symbol at compile time
        if args.is_empty() {
            return MacroResult::error("bindSym requires symbol name", Span::new(FileId(0), 0, 0));
        }
        MacroResult::new(format!("nnkSym({})", args[0]))
    }

    fn expand_quote(&self, args: &[String]) -> MacroResult {
        // quote do creates an AST literal
        if args.is_empty() {
            return MacroResult::error("quote do requires body", Span::new(FileId(0), 0, 0));
        }
        MacroResult::new(format!("nnkStmtList({})", args.join("")))
    }

    /// Enable/disable hygiene
    pub fn set_hygiene(&mut self, enabled: bool) {
        self.hygiene_enabled = enabled;
    }

    /// Check if hygiene is enabled
    pub fn is_hygiene_enabled(&self) -> bool {
        self.hygiene_enabled
    }
}

/// Macro environment for tracking symbol bindings
#[derive(Debug, Clone, Default)]
pub struct MacroEnv {
    bindings: Vec<(String, String)>,
}

impl MacroEnv {
    pub fn new() -> Self {
        MacroEnv {
            bindings: Vec::new(),
        }
    }

    pub fn bind(&mut self, name: &str, value: &str) {
        self.bindings.push((name.to_string(), value.to_string()));
    }

    pub fn lookup(&self, name: &str) -> Option<String> {
        self.bindings
            .iter()
            .rfind(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_expander_new() {
        let expander = MacroExpander::new();
        assert!(expander.is_hygiene_enabled());
    }

    #[test]
    fn test_macro_expander_set_hygiene() {
        let mut expander = MacroExpander::new();
        expander.set_hygiene(false);
        assert!(!expander.is_hygiene_enabled());
    }

    #[test]
    fn test_macro_result_success() {
        let result = MacroResult::new("expanded".to_string());
        assert!(result.success);
        assert_eq!(result.expanded_code, "expanded");
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_macro_result_error() {
        let result = MacroResult::error("test error", Span::new(FileId(0), 0, 0));
        assert!(!result.success);
        assert!(result.errors.len() == 1);
    }

    #[test]
    fn test_macro_env_new() {
        let env = MacroEnv::new();
        assert!(env.bindings.is_empty());
    }

    #[test]
    fn test_macro_env_bind_lookup() {
        let mut env = MacroEnv::new();
        env.bind("x", "value_x");
        assert_eq!(env.lookup("x"), Some("value_x".to_string()));
    }

    #[test]
    fn test_macro_env_lookup_not_found() {
        let env = MacroEnv::new();
        assert_eq!(env.lookup("undefined"), None);
    }

    #[test]
    fn test_expand_macro_macro() {
        let expander = MacroExpander::new();
        let result = expander.expand(
            "macro",
            &["myMacro".to_string()],
            Span::new(FileId(0), 0, 0),
        );
        assert!(result.success);
        assert!(result.expanded_code.contains("myMacro"));
    }

    #[test]
    fn test_expand_bind_sym() {
        let expander = MacroExpander::new();
        let result = expander.expand("bindSym", &["foo".to_string()], Span::new(FileId(0), 0, 0));
        assert!(result.success);
    }

    #[test]
    fn test_expand_quote() {
        let expander = MacroExpander::new();
        let result = expander.expand(
            "quote",
            &["echo('hello')".to_string()],
            Span::new(FileId(0), 0, 0),
        );
        assert!(result.success);
    }

    #[test]
    fn test_expand_unknown_macro() {
        let expander = MacroExpander::new();
        let result = expander.expand("unknownMacro", &[], Span::new(FileId(0), 0, 0));
        assert!(!result.success);
    }
}
