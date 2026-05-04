//! Custom pragma and annotation support.

use rnim_span::Span;
use std::collections::HashMap;

/// Built-in pragma kinds
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PragmaKind {
    /// Import pragma for FFI
    Import,
    /// Export pragma for FFI
    Export,
    /// Dynlib pragma for dynamic library loading
    Dynlib,
    /// Cdecl calling convention
    Cdecl,
    /// Stdcall calling convention
    Stdcall,
    /// Inline pragma
    Inline,
    /// NoInline pragma
    NoInline,
    /// Compile-time pragma
    CompileTime,
    /// Unsafe pragma
    Unsafe,
    /// Deprecated pragma
    Deprecated,
    /// Error pragma
    Error,
    /// Warning pragma
    Warning,
    /// Hint pragma
    Hint,
    /// Thread-local pragma
    ThreadLocal,
    ///GcSafe pragma
    GcSafe,
    /// Register pragma
    Register,
    /// Volatile pragma
    Volatile,
    /// Aligned pragma
    Aligned,
    /// UncheckedReturn pragma
    UncheckedReturn,
    /// LinearScan pragma
    LinearScan,
    /// TrMacros pragma
    TrMacros,
    /// EffectsDefined pragma
    EffectsDefined,
    /// Used pragma
    Used,
    /// ByRef pragma
    ByRef,
    /// ByCopy pragma
    ByCopy,
    /// NoReturn pragma
    NoReturn,
    /// NoSideEffect pragma
    NoSideEffect,
    /// SideEffect pragma
    SideEffect,
    /// Nimcall pragma
    Nimcall,
    /// Fastcall pragma
    Fastcall,
    /// Thiscall pragma
    Thiscall,
    /// Syscall pragma
    Syscall,
    /// Pascal pragma
    Pascal,
    /// User-defined/custom pragma
    Custom(String),
}

/// A pragma argument
#[derive(Debug, Clone, PartialEq)]
pub enum PragmaArg {
    /// No argument
    None,
    /// Integer argument
    Int(i64),
    /// String argument
    Str(String),
    /// Symbol/identifier argument
    Symbol(String),
    /// Expression argument
    Expr(String),
}

/// A single pragma entry
#[derive(Debug, Clone, PartialEq)]
pub struct Pragma {
    pub kind: PragmaKind,
    pub args: Vec<PragmaArg>,
    pub span: Span,
}

/// A collection of pragmas for a declaration
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PragmaList {
    pub pragmas: Vec<Pragma>,
}

impl PragmaList {
    pub fn new() -> Self {
        PragmaList { pragmas: vec![] }
    }

    /// Add a pragma to the list
    pub fn add(&mut self, pragma: Pragma) {
        self.pragmas.push(pragma);
    }

    /// Check if this pragma list contains a specific kind
    pub fn contains(&self, kind: &PragmaKind) -> bool {
        self.pragmas.iter().any(|p| &p.kind == kind)
    }

    /// Get all pragmas of a specific kind
    pub fn get_all(&self, kind: &PragmaKind) -> Vec<&Pragma> {
        self.pragmas.iter().filter(|p| &p.kind == kind).collect()
    }

    /// Check if this is a custom pragma (not built-in)
    pub fn has_custom_pragmas(&self) -> bool {
        self.pragmas
            .iter()
            .any(|p| matches!(p.kind, PragmaKind::Custom(_)))
    }

    /// Get the first pragma of a kind, if any
    pub fn get_first(&self, kind: &PragmaKind) -> Option<&Pragma> {
        self.pragmas.iter().find(|p| &p.kind == kind)
    }

    /// Get the first argument for a pragma of a given kind
    pub fn get_first_arg(&self, kind: &PragmaKind) -> Option<&PragmaArg> {
        self.get_first(kind).and_then(|p| p.args.first())
    }
}

/// Known pragma definitions
#[derive(Debug, Clone)]
pub struct PragmaDef {
    pub name: String,
    pub kind: PragmaKind,
    pub takes_argument: bool,
    pub is_backend: bool,
}

/// Registry of known pragmas
#[derive(Debug, Clone)]
pub struct PragmaRegistry {
    pragmas: HashMap<String, PragmaDef>,
}

impl PragmaRegistry {
    pub fn new() -> Self {
        let mut registry = PragmaRegistry {
            pragmas: HashMap::new(),
        };
        registry.init_builtins();
        registry
    }

    fn init_builtins(&mut self) {
        // Calling convention pragmas
        self.add_builtin("cdecl", PragmaKind::Cdecl, false, false);
        self.add_builtin("stdcall", PragmaKind::Stdcall, false, false);
        self.add_builtin("nimcall", PragmaKind::Nimcall, false, false);
        self.add_builtin("fastcall", PragmaKind::Fastcall, false, false);
        self.add_builtin("thiscall", PragmaKind::Thiscall, false, false);
        self.add_builtin("syscall", PragmaKind::Syscall, false, false);
        self.add_builtin("pascal", PragmaKind::Pascal, false, false);

        // FFI pragmas
        self.add_builtin("importc", PragmaKind::Import, true, false);
        self.add_builtin("import", PragmaKind::Import, true, false);
        self.add_builtin("exportc", PragmaKind::Export, true, false);
        self.add_builtin("export", PragmaKind::Export, true, false);
        self.add_builtin("dynlib", PragmaKind::Dynlib, true, false);
        self.add_builtin("header", PragmaKind::Import, true, false);
        self.add_builtin("passc", PragmaKind::Custom("passc".to_string()), true, true);
        self.add_builtin("passl", PragmaKind::Custom("passl".to_string()), true, true);
        self.add_builtin(
            "compile",
            PragmaKind::Custom("compile".to_string()),
            true,
            true,
        );
        self.add_builtin("link", PragmaKind::Custom("link".to_string()), true, true);

        // Optimization pragmas
        self.add_builtin("inline", PragmaKind::Inline, false, false);
        self.add_builtin("noinline", PragmaKind::NoInline, false, false);
        self.add_builtin("deprecated", PragmaKind::Deprecated, true, false);
        self.add_builtin("error", PragmaKind::Error, true, false);
        self.add_builtin("warning", PragmaKind::Warning, true, false);
        self.add_builtin("hint", PragmaKind::Hint, true, false);
        self.add_builtin("compiletime", PragmaKind::CompileTime, false, false);
        self.add_builtin("unsafe", PragmaKind::Unsafe, false, false);

        // Memory/GC pragmas
        self.add_builtin("thread", PragmaKind::ThreadLocal, false, false);
        self.add_builtin("gcsafe", PragmaKind::GcSafe, false, false);
        self.add_builtin("register", PragmaKind::Register, false, false);
        self.add_builtin("volatile", PragmaKind::Volatile, false, false);
        self.add_builtin("aligned", PragmaKind::Aligned, true, false);

        // Effect tracking pragmas
        self.add_builtin(
            "raises",
            PragmaKind::Custom("raises".to_string()),
            true,
            false,
        );
        self.add_builtin("tags", PragmaKind::Custom("tags".to_string()), true, false);
        self.add_builtin(
            "effects",
            PragmaKind::Custom("effects".to_string()),
            true,
            false,
        );

        // Procedure attributes
        self.add_builtin("uncheckedreturn", PragmaKind::UncheckedReturn, false, false);
        self.add_builtin("linearscan", PragmaKind::LinearScan, false, false);
        self.add_builtin("trmachros", PragmaKind::TrMacros, false, false);
        self.add_builtin("effectsdefined", PragmaKind::EffectsDefined, true, false);
        self.add_builtin("used", PragmaKind::Used, false, false);
        self.add_builtin("byref", PragmaKind::ByRef, false, false);
        self.add_builtin("bycopy", PragmaKind::ByCopy, false, false);
        self.add_builtin("noreturn", PragmaKind::NoReturn, false, false);
        self.add_builtin("nosideeffect", PragmaKind::NoSideEffect, false, false);
        self.add_builtin("sideeffect", PragmaKind::SideEffect, false, false);
    }

    fn add_builtin(&mut self, name: &str, kind: PragmaKind, takes_arg: bool, is_backend: bool) {
        self.pragmas.insert(
            name.to_string(),
            PragmaDef {
                name: name.to_string(),
                kind,
                takes_argument: takes_arg,
                is_backend,
            },
        );
    }

    /// Look up a pragma by name
    pub fn lookup(&self, name: &str) -> Option<&PragmaDef> {
        self.pragmas.get(name)
    }

    /// Check if a pragma name is known
    pub fn is_known(&self, name: &str) -> bool {
        self.pragmas.contains_key(name)
    }

    /// Register a custom pragma
    pub fn register_custom(&mut self, name: &str, is_backend: bool) {
        self.pragmas.insert(
            name.to_string(),
            PragmaDef {
                name: name.to_string(),
                kind: PragmaKind::Custom(name.to_string()),
                takes_argument: true,
                is_backend,
            },
        );
    }
}

impl Default for PragmaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Pragma validator
#[derive(Debug, Clone)]
pub struct PragmaValidator {
    registry: PragmaRegistry,
}

impl PragmaValidator {
    pub fn new() -> Self {
        PragmaValidator {
            registry: PragmaRegistry::new(),
        }
    }

    /// Validate a pragma list against a registry
    pub fn validate(&self, pragmas: &PragmaList) -> Result<(), PragmaError> {
        for pragma in &pragmas.pragmas {
            if let PragmaKind::Custom(name) = &pragma.kind {
                if !self.registry.is_known(name) {
                    // Check if it looks like an identifier (user-defined pragma)
                    if !is_valid_ident(name) {
                        return Err(PragmaError::InvalidName(name.clone()));
                    }
                }
            }
        }
        Ok(())
    }

    /// Get the registry
    pub fn registry(&self) -> &PragmaRegistry {
        &self.registry
    }
}

impl Default for PragmaValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Pragma validation error
#[derive(Debug, Clone, PartialEq)]
pub enum PragmaError {
    /// Unknown pragma name
    Unknown(String),
    /// Invalid pragma name syntax
    InvalidName(String),
    /// Invalid argument for pragma
    InvalidArg { pragma: String, expected: String },
    /// Pragma not allowed in this context
    NotAllowed(String),
}

/// Check if a string is a valid Nim identifier
fn is_valid_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    // First char must be a letter or underscore
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    // Rest can be letters, digits, or underscores
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_pragma_list_new() {
        let list = PragmaList::new();
        assert!(list.pragmas.is_empty());
    }

    #[test]
    fn test_pragma_list_add() {
        let mut list = PragmaList::new();
        list.add(Pragma {
            kind: PragmaKind::Inline,
            args: vec![],
            span: Span::new(FileId(0), 0, 0),
        });
        assert!(list.contains(&PragmaKind::Inline));
    }

    #[test]
    fn test_pragma_list_get_first() {
        let mut list = PragmaList::new();
        list.add(Pragma {
            kind: PragmaKind::Deprecated,
            args: vec![PragmaArg::Str("Use newProc instead".to_string())],
            span: Span::new(FileId(0), 0, 0),
        });
        let first = list.get_first(&PragmaKind::Deprecated);
        assert!(first.is_some());
        assert_eq!(first.unwrap().args.len(), 1);
    }

    #[test]
    fn test_pragma_list_get_all() {
        let mut list = PragmaList::new();
        list.add(Pragma {
            kind: PragmaKind::Custom("custom1".to_string()),
            args: vec![],
            span: Span::new(FileId(0), 0, 0),
        });
        list.add(Pragma {
            kind: PragmaKind::Custom("custom2".to_string()),
            args: vec![],
            span: Span::new(FileId(0), 0, 0),
        });
        let custom = list.get_all(&PragmaKind::Custom("custom1".to_string()));
        assert_eq!(custom.len(), 1);
    }

    #[test]
    fn test_pragma_registry_lookup() {
        let registry = PragmaRegistry::new();
        assert!(registry.is_known("cdecl"));
        assert!(registry.is_known("importc"));
        assert!(registry.is_known("inline"));
        assert!(!registry.is_known("nonexistent"));
    }

    #[test]
    fn test_pragma_registry_lookup_def() {
        let registry = PragmaRegistry::new();
        let def = registry.lookup("importc");
        assert!(def.is_some());
        let def = def.unwrap();
        assert_eq!(def.kind, PragmaKind::Import);
        assert!(def.takes_argument);
    }

    #[test]
    fn test_pragma_registry_custom() {
        let mut registry = PragmaRegistry::new();
        registry.register_custom("myPragma", false);
        assert!(registry.is_known("myPragma"));
    }

    #[test]
    fn test_pragma_validator_validate() {
        let validator = PragmaValidator::new();
        let mut list = PragmaList::new();
        list.add(Pragma {
            kind: PragmaKind::Inline,
            args: vec![],
            span: Span::new(FileId(0), 0, 0),
        });
        assert!(validator.validate(&list).is_ok());
    }

    #[test]
    fn test_pragma_validator_custom() {
        let validator = PragmaValidator::new();
        let mut list = PragmaList::new();
        list.add(Pragma {
            kind: PragmaKind::Custom("userDefined".to_string()),
            args: vec![],
            span: Span::new(FileId(0), 0, 0),
        });
        // Valid identifiers should be allowed as custom pragmas
        assert!(validator.validate(&list).is_ok());
    }

    #[test]
    fn test_is_valid_ident() {
        assert!(is_valid_ident("foo"));
        assert!(is_valid_ident("fooBar"));
        assert!(is_valid_ident("_foo"));
        assert!(is_valid_ident("foo123"));
        assert!(is_valid_ident("_foo_bar_123"));
        assert!(!is_valid_ident(""));
        assert!(!is_valid_ident("123foo"));
        assert!(!is_valid_ident("foo-bar"));
        assert!(!is_valid_ident("foo.bar"));
    }

    #[test]
    fn test_pragma_arg_variants() {
        assert!(matches!(PragmaArg::None, PragmaArg::None));
        let int_arg = PragmaArg::Int(42);
        assert!(matches!(int_arg, PragmaArg::Int(_)));
        let str_arg = PragmaArg::Str(String::new());
        assert!(matches!(str_arg, PragmaArg::Str(_)));
        let sym_arg = PragmaArg::Symbol(String::new());
        assert!(matches!(sym_arg, PragmaArg::Symbol(_)));
        let expr_arg = PragmaArg::Expr(String::new());
        assert!(matches!(expr_arg, PragmaArg::Expr(_)));
    }

    #[test]
    fn test_pragma_kind_variants() {
        use PragmaKind::*;
        assert!(matches!(Cdecl, Cdecl));
        assert!(matches!(Import, Import));
        assert!(matches!(Inline, Inline));
        let custom = Custom(String::new());
        assert!(matches!(custom, Custom(_)));
    }

    #[test]
    fn test_has_custom_pragmas() {
        let mut list = PragmaList::new();
        assert!(!list.has_custom_pragmas());
        list.add(Pragma {
            kind: PragmaKind::Inline,
            args: vec![],
            span: Span::new(FileId(0), 0, 0),
        });
        assert!(!list.has_custom_pragmas());
        list.add(Pragma {
            kind: PragmaKind::Custom("mypragma".to_string()),
            args: vec![],
            span: Span::new(FileId(0), 0, 0),
        });
        assert!(list.has_custom_pragmas());
    }

    #[test]
    fn test_pragma_error_display() {
        let err = PragmaError::Unknown("unknownpragma".to_string());
        assert!(format!("{:?}", err).contains("unknownpragma"));

        let err = PragmaError::InvalidArg {
            pragma: "test".to_string(),
            expected: "int".to_string(),
        };
        assert!(format!("{:?}", err).contains("test"));
        assert!(format!("{:?}", err).contains("int"));
    }

    #[test]
    fn test_pragma_registry_new_has_builtins() {
        let registry = PragmaRegistry::new();
        assert!(registry.is_known("importc"));
        assert!(registry.is_known("deprecated"));
    }
}
