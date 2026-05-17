//! FFI and backend pragma support.
//!
//! This module provides support for FFI pragmas like importc, exportc,
//! dynlib, calling conventions, and symbol naming controls.

use rnim_span::Span;
use std::collections::HashMap;

/// Calling convention for FFI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallConv {
    Cdecl,
    Stdcall,
    Syscall,
    Inline,
    Fastcall,
    Thiscall,
    Pascal,
    Nimcall,
}

impl CallConv {
    /// Get the string representation for C/C++ compilation
    pub fn as_str(&self) -> &'static str {
        match self {
            CallConv::Cdecl => "__cdecl",
            CallConv::Stdcall => "__stdcall",
            CallConv::Syscall => "__syscall",
            CallConv::Inline => "inline",
            CallConv::Fastcall => "__fastcall",
            CallConv::Thiscall => "__thiscall",
            CallConv::Pascal => "__pascal",
            CallConv::Nimcall => "nimcall",
        }
    }

    /// Parse from Nim pragma string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "cdecl" => Some(CallConv::Cdecl),
            "stdcall" => Some(CallConv::Stdcall),
            "syscall" => Some(CallConv::Syscall),
            "inline" => Some(CallConv::Inline),
            "fastcall" => Some(CallConv::Fastcall),
            "thiscall" => Some(CallConv::Thiscall),
            "pascal" => Some(CallConv::Pascal),
            "nimcall" => Some(CallConv::Nimcall),
            _ => None,
        }
    }
}

/// Default to Cdecl
impl Default for CallConv {
    fn default() -> Self {
        CallConv::Cdecl
    }
}

/// FFI pragma configuration
#[derive(Debug, Clone)]
pub struct FfiConfig {
    /// Import C function from library
    pub importc: Option<String>,
    /// Export function to C
    pub exportc: Option<String>,
    /// Dynamic library to load
    pub dynlib: Option<String>,
    /// C calling convention
    pub calling_convention: CallConv,
    /// Header file to include
    pub header: Option<String>,
    /// Pass to C compiler
    pub passc: Option<String>,
    /// Pass to linker
    pub passl: Option<String>,
    /// Compile this object file
    pub compile: Option<String>,
    /// Link this object file
    pub link: Option<String>,
    /// Use deprecated import
    pub deprecated: Option<String>,
    /// Importobj C++ mangled name
    pub import_obj: Option<String>,
    /// Importcpp for C++ interop
    pub importcpp: Option<String>,
    /// Importobjcpp for C++ with object mangling
    pub import_objcpp: Option<String>,
    /// Importassembly for assembly interop
    pub import_objasm: Option<String>,
}

impl FfiConfig {
    pub fn new() -> Self {
        FfiConfig {
            importc: None,
            exportc: None,
            dynlib: None,
            calling_convention: CallConv::default(),
            header: None,
            passc: None,
            passl: None,
            compile: None,
            link: None,
            deprecated: None,
            import_obj: None,
            importcpp: None,
            import_objcpp: None,
            import_objasm: None,
        }
    }

    /// Check if this is an import config
    pub fn is_import(&self) -> bool {
        self.importc.is_some()
            || self.import_obj.is_some()
            || self.importcpp.is_some()
            || self.import_objcpp.is_some()
            || self.import_objasm.is_some()
    }

    /// Check if this is an export config
    pub fn is_export(&self) -> bool {
        self.exportc.is_some()
    }

    /// Get the symbol name to use
    pub fn symbol_name(&self, default_name: &str) -> String {
        if let Some(ref name) = self.exportc {
            name.clone()
        } else if let Some(ref name) = self.importc {
            name.clone()
        } else {
            default_name.to_string()
        }
    }

    /// Get dynamic library name if set
    pub fn dynlib_name(&self) -> Option<&str> {
        self.dynlib.as_deref()
    }
}

impl Default for FfiConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Link specification
#[derive(Debug, Clone)]
pub struct LinkSpec {
    pub name: String,
    pub kind: LinkKind,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Static,
    Dynamic,
    Framework,
}

impl LinkSpec {
    pub fn new(name: &str, kind: LinkKind) -> Self {
        LinkSpec {
            name: name.to_string(),
            kind,
            path: None,
        }
    }

    pub fn with_path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

    /// Generate linker flag
    pub fn linker_flag(&self) -> String {
        match self.kind {
            LinkKind::Dynamic => format!("-l{}", self.name),
            LinkKind::Static => {
                if let Some(ref path) = self.path {
                    path.to_string()
                } else {
                    format!("-l:lib{}.a", self.name)
                }
            }
            LinkKind::Framework => format!("-framework {}", self.name),
        }
    }
}

/// Compile flag specification
#[derive(Debug, Clone)]
pub struct CompileFlag {
    pub flag: String,
    pub path: Option<String>,
}

impl CompileFlag {
    pub fn new(flag: &str) -> Self {
        CompileFlag {
            flag: flag.to_string(),
            path: None,
        }
    }

    pub fn with_path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

    /// Generate compiler flag
    pub fn compiler_flag(&self) -> String {
        if let Some(ref path) = self.path {
            format!("{} {}", self.flag, path)
        } else {
            self.flag.clone()
        }
    }
}

/// Symbol naming rules
#[derive(Debug, Clone, Default)]
pub struct SymbolNamer {
    /// Custom naming rules
    rules: HashMap<String, String>,
    /// Export prefixes/suffixes
    export_prefix: String,
    export_suffix: String,
}

impl SymbolNamer {
    pub fn new() -> Self {
        SymbolNamer {
            rules: HashMap::new(),
            export_prefix: String::new(),
            export_suffix: String::new(),
        }
    }

    /// Add a naming rule for a specific symbol
    pub fn add_rule(&mut self, pattern: &str, replacement: &str) {
        self.rules
            .insert(pattern.to_string(), replacement.to_string());
    }

    /// Set export prefix
    pub fn set_export_prefix(&mut self, prefix: &str) {
        self.export_prefix = prefix.to_string();
    }

    /// Set export suffix
    pub fn set_export_suffix(&mut self, suffix: &str) {
        self.export_suffix = suffix.to_string();
    }

    /// Apply naming rules to a symbol
    pub fn name_symbol(&self, symbol: &str, is_export: bool) -> String {
        // Check if we have a specific rule
        if let Some(replacement) = self.rules.get(symbol) {
            return replacement.clone();
        }

        // Apply export prefix/suffix
        if is_export {
            format!("{}{}{}", self.export_prefix, symbol, self.export_suffix)
        } else {
            symbol.to_string()
        }
    }
}

/// FFI symbol resolution
#[derive(Debug, Clone)]
pub struct FfiSymbol {
    pub name: String,
    pub address: Option<usize>,
    pub is_function: bool,
    pub calling_convention: CallConv,
    pub span: Span,
}

impl FfiSymbol {
    pub fn new(name: &str, span: Span) -> Self {
        FfiSymbol {
            name: name.to_string(),
            address: None,
            is_function: true,
            calling_convention: CallConv::default(),
            span,
        }
    }

    pub fn with_address(mut self, addr: usize) -> Self {
        self.address = Some(addr);
        self
    }

    pub fn with_conv(mut self, conv: CallConv) -> Self {
        self.calling_convention = conv;
        self
    }
}

/// FFI error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FfiError {
    SymbolNotFound(String, Span),
    InvalidCallingConvention(String, Span),
    InvalidDynlib(String, Span),
    LinkError(String, Span),
    CompileError(String, Span),
}

impl std::fmt::Display for FfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FfiError::SymbolNotFound(s, _) => write!(f, "FFI symbol not found: {}", s),
            FfiError::InvalidCallingConvention(s, _) => {
                write!(f, "invalid calling convention: {}", s)
            }
            FfiError::InvalidDynlib(s, _) => write!(f, "invalid dynamic library: {}", s),
            FfiError::LinkError(s, _) => write!(f, "linker error: {}", s),
            FfiError::CompileError(s, _) => write!(f, "compile error: {}", s),
        }
    }
}

impl std::error::Error for FfiError {}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_call_conv_as_str() {
        assert_eq!(CallConv::Cdecl.as_str(), "__cdecl");
        assert_eq!(CallConv::Stdcall.as_str(), "__stdcall");
        assert_eq!(CallConv::Nimcall.as_str(), "nimcall");
    }

    #[test]
    fn test_call_conv_from_str() {
        assert_eq!(CallConv::from_str("cdecl"), Some(CallConv::Cdecl));
        assert_eq!(CallConv::from_str("stdcall"), Some(CallConv::Stdcall));
        assert_eq!(CallConv::from_str("unknown"), None);
    }

    #[test]
    fn test_call_conv_default() {
        let conv = CallConv::default();
        assert_eq!(conv, CallConv::Cdecl);
    }

    #[test]
    fn test_ffi_config_new() {
        let config = FfiConfig::new();
        assert!(!config.is_import());
        assert!(!config.is_export());
    }

    #[test]
    fn test_ffi_config_import() {
        let mut config = FfiConfig::new();
        config.importc = Some("libfoo".to_string());
        assert!(config.is_import());
    }

    #[test]
    fn test_ffi_config_export() {
        let mut config = FfiConfig::new();
        config.exportc = Some("foo".to_string());
        assert!(config.is_export());
    }

    #[test]
    fn test_ffi_config_symbol_name() {
        let config = FfiConfig::new();
        assert_eq!(config.symbol_name("default"), "default");

        let mut config = FfiConfig::new();
        config.importc = Some("customName".to_string());
        assert_eq!(config.symbol_name("default"), "customName");
    }

    #[test]
    fn test_ffi_config_dynlib_name() {
        let config = FfiConfig::new();
        assert!(config.dynlib_name().is_none());

        let mut config = FfiConfig::new();
        config.dynlib = Some("libc".to_string());
        assert_eq!(config.dynlib_name(), Some("libc"));
    }

    #[test]
    fn test_link_spec_new() {
        let spec = LinkSpec::new("mylib", LinkKind::Dynamic);
        assert_eq!(spec.name, "mylib");
        assert_eq!(spec.kind, LinkKind::Dynamic);
        assert!(spec.path.is_none());
    }

    #[test]
    fn test_link_spec_linker_flag() {
        let spec = LinkSpec::new("mylib", LinkKind::Dynamic);
        assert_eq!(spec.linker_flag(), "-lmylib");

        let spec = LinkSpec::new("mylib", LinkKind::Static);
        assert!(spec.linker_flag().contains("-l"));

        let spec = LinkSpec::new("Foundation", LinkKind::Framework);
        assert_eq!(spec.linker_flag(), "-framework Foundation");
    }

    #[test]
    fn test_link_spec_with_path() {
        let spec = LinkSpec::new("mylib", LinkKind::Static).with_path("/path/to/libmylib.a");
        assert_eq!(spec.path, Some("/path/to/libmylib.a".to_string()));
    }

    #[test]
    fn test_compile_flag_new() {
        let flag = CompileFlag::new("-O3");
        assert_eq!(flag.flag, "-O3");
        assert!(flag.path.is_none());
    }

    #[test]
    fn test_compile_flag_with_path() {
        let flag = CompileFlag::new("-include").with_path("header.h");
        assert_eq!(flag.path, Some("header.h".to_string()));
    }

    #[test]
    fn test_compile_flag_compiler_flag() {
        let flag = CompileFlag::new("-O3");
        assert_eq!(flag.compiler_flag(), "-O3");

        let flag = CompileFlag::new("-include").with_path("header.h");
        assert_eq!(flag.compiler_flag(), "-include header.h");
    }

    #[test]
    fn test_symbol_namer_new() {
        let namer = SymbolNamer::new();
        assert!(namer.rules.is_empty());
        assert!(namer.export_prefix.is_empty());
    }

    #[test]
    fn test_symbol_namer_add_rule() {
        let mut namer = SymbolNamer::new();
        namer.add_rule("foo", "bar");
        assert_eq!(namer.name_symbol("foo", false), "bar");
    }

    #[test]
    fn test_symbol_namer_export_suffix() {
        let mut namer = SymbolNamer::new();
        namer.set_export_suffix("_export");
        assert_eq!(namer.name_symbol("foo", true), "foo_export");
        assert_eq!(namer.name_symbol("foo", false), "foo");
    }

    #[test]
    fn test_symbol_namer_export_prefix() {
        let mut namer = SymbolNamer::new();
        namer.set_export_prefix("_");
        assert_eq!(namer.name_symbol("foo", true), "_foo");
        assert_eq!(namer.name_symbol("foo", false), "foo");
    }

    #[test]
    fn test_ffi_symbol_new() {
        let span = Span::new(FileId(0), 0, 0);
        let sym = FfiSymbol::new("foo", span);
        assert_eq!(sym.name, "foo");
        assert!(sym.address.is_none());
        assert!(sym.is_function);
    }

    #[test]
    fn test_ffi_symbol_with_address() {
        let span = Span::new(FileId(0), 0, 0);
        let sym = FfiSymbol::new("foo", span).with_address(0x1000);
        assert_eq!(sym.address, Some(0x1000));
    }

    #[test]
    fn test_ffi_symbol_with_conv() {
        let span = Span::new(FileId(0), 0, 0);
        let sym = FfiSymbol::new("foo", span).with_conv(CallConv::Stdcall);
        assert_eq!(sym.calling_convention, CallConv::Stdcall);
    }

    #[test]
    fn test_ffi_error_display() {
        let span = Span::new(FileId(0), 0, 0);
        let err = FfiError::SymbolNotFound("foo".to_string(), span);
        let s = format!("{}", err);
        assert!(s.contains("foo"));
    }
}
