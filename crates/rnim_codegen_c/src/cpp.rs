//! C++ and Objective-C interop support.
//!
//! This module provides dialect selection, name mangling, and FFI
//! support for C++ and Objective-C backends.

/// C++ dialect/standard selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CppDialect {
    /// C++98
    Cpp98,
    /// C++11
    Cpp11,
    /// C++14
    Cpp14,
    /// C++17
    #[default]
    Cpp17,
    /// C++20
    Cpp20,
    /// C++23
    Cpp23,
}

impl CppDialect {
    /// Returns the mangled name prefix for this C++ standard
    pub fn mangled_prefix(&self) -> &'static str {
        match self {
            CppDialect::Cpp98 | CppDialect::Cpp11 => "_Z",
            CppDialect::Cpp14 | CppDialect::Cpp17 | CppDialect::Cpp20 | CppDialect::Cpp23 => "_Z",
        }
    }

    /// Returns true if RTTI is available in this dialect
    pub fn has_rtti(&self) -> bool {
        match self {
            CppDialect::Cpp98 => false,
            CppDialect::Cpp11
            | CppDialect::Cpp14
            | CppDialect::Cpp17
            | CppDialect::Cpp20
            | CppDialect::Cpp23 => true,
        }
    }

    /// Returns true if modules are available in this dialect
    pub fn has_modules(&self) -> bool {
        match self {
            CppDialect::Cpp20 | CppDialect::Cpp23 => true,
            CppDialect::Cpp98 | CppDialect::Cpp11 | CppDialect::Cpp14 | CppDialect::Cpp17 => false,
        }
    }

    /// Returns the standard flag for compilation
    pub fn compile_flag(&self) -> &'static str {
        match self {
            CppDialect::Cpp98 => "-std=c++98",
            CppDialect::Cpp11 => "-std=c++11",
            CppDialect::Cpp14 => "-std=c++14",
            CppDialect::Cpp17 => "-std=c++17",
            CppDialect::Cpp20 => "-std=c++20",
            CppDialect::Cpp23 => "-std=c++23",
        }
    }
}

/// C++ name mangler
#[derive(Debug, Clone)]
pub struct CppNameMangler {
    dialect: CppDialect,
}

impl CppNameMangler {
    pub fn new(dialect: CppDialect) -> Self {
        CppNameMangler { dialect }
    }

    /// Mangle a function name according to C++ Itanium ABI
    pub fn mangle_function(&self, name: &str, params: &[&str]) -> String {
        let prefix = self.dialect.mangled_prefix();
        let mut result = prefix.to_string();

        // Encode parameter types
        for param in params {
            result.push_str(&self.mangle_type(param));
        }

        // Add function name length and name
        result.push_str(&format!("{}{}", name.len(), name));
        result.push('E');

        result
    }

    /// Mangle a type name
    fn mangle_type(&self, ty: &str) -> String {
        match ty {
            "void" => "v".to_string(),
            "bool" => "b".to_string(),
            "char" => "c".to_string(),
            "int" => "i".to_string(),
            "unsigned int" | "unsigned" => "j".to_string(),
            "long" => "l".to_string(),
            "unsigned long" => "m".to_string(),
            "long long" => "x".to_string(),
            "unsigned long long" => "y".to_string(),
            "float" => "f".to_string(),
            "double" => "d".to_string(),
            "long double" => "e".to_string(),
            s if s.starts_with("unsigned ") => format!(
                "{}_{}",
                "unsigned".len(),
                self.mangle_type(&s["unsigned ".len()..])
            ),
            s if s.starts_with("signed ") => self.mangle_type(&s["signed ".len()..]),
            s if s.ends_with('*') => format!("P{}", self.mangle_type(&s[..s.len() - 1])),
            s if s.ends_with("&&") => format!("O{}", self.mangle_type(&s[..s.len() - 2])),
            s if s.ends_with('&') => format!("R{}", self.mangle_type(&s[..s.len() - 1])),
            "wchar_t" => "w".to_string(),
            "char16_t" => "Ds".to_string(),
            "char32_t" => "Di".to_string(),
            "char8_t" => "Du".to_string(),
            "decltype(nullptr)" => "Dn".to_string(),
            _ => {
                // For unknown types, encode length-prefixed name
                format!("{}{}", ty.len(), ty)
            }
        }
    }

    /// Mangle a constructor name
    pub fn mangle_constructor(&self, class_name: &str) -> String {
        format!("C1{}{}", class_name.len(), class_name)
    }

    /// Mangle a destructor name
    pub fn mangle_destructor(&self, class_name: &str) -> String {
        format!("D1{}{}", class_name.len(), class_name)
    }
}

/// Objective-C backend mode
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ObjCMode {
    /// Traditional Objective-C
    #[default]
    ObjC,
    /// Objective-C++ (C++ with Objective-C)
    ObjCPlusPlus,
}

/// Objective-C selector representation
#[derive(Debug, Clone, PartialEq)]
pub struct ObjCSelector {
    pub parts: Vec<String>,
}

impl ObjCSelector {
    /// Parse an Objective-C selector from a string like "initWithFrame:"
    pub fn parse(selector_str: &str) -> Self {
        let parts: Vec<String> = selector_str
            .split(':')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        ObjCSelector { parts }
    }

    /// Get the selector key (e.g., "initWithFrame" for "initWithFrame:")
    pub fn key(&self) -> String {
        if self.parts.len() == 1 {
            self.parts[0].clone()
        } else {
            self.parts.join(":")
        }
    }

    /// Number of arguments this selector expects
    pub fn arg_count(&self) -> usize {
        self.parts.len()
    }
}

/// Objective-C class/method emission helper
#[derive(Debug, Clone)]
pub struct ObjCEmitter {
    mode: ObjCMode,
}

impl ObjCEmitter {
    pub fn new(mode: ObjCMode) -> Self {
        ObjCEmitter { mode }
    }

    /// Emit an @interface declaration
    pub fn emit_interface(
        &self,
        class_name: &str,
        superclass: Option<&str>,
        protocols: &[&str],
    ) -> String {
        let super_str = superclass.map_or("@interface Object".to_string(), |s| {
            format!("@interface {}", s)
        });
        let class_str = format!(
            "{} : {}",
            class_name,
            super_str.trim_start_matches("@interface ")
        );
        let protocol_str = if protocols.is_empty() {
            String::new()
        } else {
            format!(" <{}>", protocols.join(", "))
        };
        format!("{}{}", class_str, protocol_str)
    }

    /// Emit an @implementation declaration
    pub fn emit_implementation(&self, class_name: &str, _superclass: Option<&str>) -> String {
        format!("@implementation {}", class_name)
    }

    /// Emit a method declaration
    pub fn emit_method(
        &self,
        is_class_method: bool,
        selector: &ObjCSelector,
        return_type: &str,
        param_type: Option<&str>,
    ) -> String {
        let method_type = if is_class_method { "+" } else { "-" };
        let _ret = format!("({})", return_type);

        if let Some(param) = param_type {
            let sel_key = selector.key();
            format!(
                "{} ({})[{}{}: {}]",
                method_type,
                return_type,
                class_name_for_selector(selector),
                sel_key,
                param
            )
        } else {
            let sel_key = selector.key();
            format!(
                "{} ({})[{}{}]",
                method_type,
                return_type,
                class_name_for_selector(selector),
                sel_key
            )
        }
    }
}

/// Helper to get class name from selector (for emission)
fn class_name_for_selector(_selector: &ObjCSelector) -> String {
    "self".to_string()
}

/// Compile command entry for build systems
#[derive(Debug, Clone)]
pub struct CompileCommand {
    pub directory: String,
    pub command: String,
    pub file: String,
}

impl CompileCommand {
    /// Create a new compile command
    pub fn new(directory: String, command: String, file: String) -> Self {
        CompileCommand {
            directory,
            command,
            file,
        }
    }

    /// Serialize to JSON format for tooling
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"directory": "{}", "command": "{}", "file": "{}"}}"#,
            escape_json(&self.directory),
            escape_json(&self.command),
            escape_json(&self.file)
        )
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// C++ FFI interop helpers
#[derive(Debug, Clone)]
pub struct CppInterop {
    dialect: CppDialect,
}

impl CppInterop {
    pub fn new(dialect: CppDialect) -> Self {
        CppInterop { dialect }
    }

    /// Check if a name needs mangling (is a C++ keyword or overloaded)
    pub fn needs_mangling(&self, name: &str) -> bool {
        // C++ keywords that need mangling
        let keywords = [
            "alignas",
            "alignof",
            "and",
            "and_eq",
            "asm",
            "auto",
            "bitand",
            "bitor",
            "bool",
            "break",
            "case",
            "catch",
            "char",
            "class",
            "compl",
            "const",
            "constexpr",
            "const_cast",
            "continue",
            "decltype",
            "default",
            "delete",
            "do",
            "double",
            "dynamic_cast",
            "else",
            "enum",
            "explicit",
            "export",
            "extern",
            "false",
            "float",
            "for",
            "friend",
            "goto",
            "if",
            "inline",
            "int",
            "long",
            "mutable",
            "namespace",
            "new",
            "noexcept",
            "not",
            "not_eq",
            "nullptr",
            "operator",
            "or",
            "or_eq",
            "private",
            "protected",
            "public",
            "register",
            "reinterpret_cast",
            "return",
            "short",
            "signed",
            "sizeof",
            "static",
            "static_assert",
            "static_cast",
            "struct",
            "switch",
            "template",
            "this",
            "thread_local",
            "throw",
            "true",
            "try",
            "typedef",
            "typeid",
            "typename",
            "union",
            "unsigned",
            "using",
            "virtual",
            "void",
            "volatile",
            "wchar_t",
            "while",
            "xor",
            "xor_eq",
        ];
        keywords.contains(&name)
    }

    /// Get the mangled name for an exported symbol
    pub fn mangle_export(&self, name: &str) -> String {
        if self.needs_mangling(name) {
            format!("_{}", name)
        } else {
            name.to_string()
        }
    }

    /// Generate extern "C" block content
    pub fn emit_extern_c_block(&self, names: &[&str]) -> String {
        let mut output = String::from("extern \"C\" {\n");
        for name in names {
            output.push_str(&format!("    {};\n", name));
        }
        output.push_str("}\n");
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_dialect_default() {
        let dialect = CppDialect::default();
        assert_eq!(dialect, CppDialect::Cpp17);
    }

    #[test]
    fn test_cpp_dialect_compile_flag() {
        assert_eq!(CppDialect::Cpp11.compile_flag(), "-std=c++11");
        assert_eq!(CppDialect::Cpp20.compile_flag(), "-std=c++20");
    }

    #[test]
    fn test_cpp_dialect_rtti() {
        assert!(!CppDialect::Cpp98.has_rtti());
        assert!(CppDialect::Cpp11.has_rtti());
        assert!(CppDialect::Cpp17.has_rtti());
    }

    #[test]
    fn test_cpp_dialect_modules() {
        assert!(!CppDialect::Cpp17.has_modules());
        assert!(CppDialect::Cpp20.has_modules());
        assert!(CppDialect::Cpp23.has_modules());
    }

    #[test]
    fn test_cpp_name_mangler_new() {
        let mangler = CppNameMangler::new(CppDialect::Cpp17);
        assert_eq!(mangler.dialect, CppDialect::Cpp17);
    }

    #[test]
    fn test_cpp_name_mangler_primitive_types() {
        let mangler = CppNameMangler::new(CppDialect::Cpp17);
        assert_eq!(mangler.mangle_type("void"), "v");
        assert_eq!(mangler.mangle_type("int"), "i");
        assert_eq!(mangler.mangle_type("float"), "f");
        assert_eq!(mangler.mangle_type("double"), "d");
    }

    #[test]
    fn test_cpp_name_mangler_pointer() {
        let mangler = CppNameMangler::new(CppDialect::Cpp17);
        assert_eq!(mangler.mangle_type("int*"), "Pi");
        assert_eq!(mangler.mangle_type("void*"), "Pv");
    }

    #[test]
    fn test_cpp_name_mangler_reference() {
        let mangler = CppNameMangler::new(CppDialect::Cpp17);
        assert_eq!(mangler.mangle_type("int&"), "Ri");
        assert_eq!(mangler.mangle_type("double&"), "Rd");
    }

    #[test]
    fn test_cpp_name_mangler_rvalue_reference() {
        let mangler = CppNameMangler::new(CppDialect::Cpp17);
        assert_eq!(mangler.mangle_type("int&&"), "Oi");
    }

    #[test]
    fn test_cpp_name_mangler_constructor() {
        let mangler = CppNameMangler::new(CppDialect::Cpp17);
        assert_eq!(mangler.mangle_constructor("Foo"), "C13Foo");
    }

    #[test]
    fn test_cpp_name_mangler_destructor() {
        let mangler = CppNameMangler::new(CppDialect::Cpp17);
        assert_eq!(mangler.mangle_destructor("Foo"), "D13Foo");
    }

    #[test]
    fn test_objc_selector_parse() {
        let sel = ObjCSelector::parse("initWithFrame:");
        assert_eq!(sel.parts, vec!["initWithFrame"]);
    }

    #[test]
    fn test_objc_selector_parse_multi_arg() {
        let sel = ObjCSelector::parse("setValue:forKey:");
        assert_eq!(sel.parts, vec!["setValue", "forKey"]);
    }

    #[test]
    fn test_objc_selector_arg_count() {
        let sel = ObjCSelector::parse("initWithFrame:");
        assert_eq!(sel.arg_count(), 1);

        let sel2 = ObjCSelector::parse("setValue:forKey:");
        assert_eq!(sel2.arg_count(), 2);
    }

    #[test]
    fn test_objc_selector_key() {
        let sel = ObjCSelector::parse("initWithFrame:");
        assert_eq!(sel.key(), "initWithFrame");
    }

    #[test]
    fn test_objc_selector_key_multi_arg() {
        let sel = ObjCSelector::parse("setValue:forKey:");
        assert_eq!(sel.key(), "setValue:forKey");
    }

    #[test]
    fn test_objc_mode_default() {
        let mode = ObjCMode::default();
        assert_eq!(mode, ObjCMode::ObjC);
    }

    #[test]
    fn test_objc_emitter_new() {
        let emitter = ObjCEmitter::new(ObjCMode::ObjC);
        assert_eq!(emitter.mode, ObjCMode::ObjC);
    }

    #[test]
    fn test_objc_emitter_interface() {
        let emitter = ObjCEmitter::new(ObjCMode::ObjC);
        let result = emitter.emit_interface("MyView", Some("NSView"), &["NSCoding"]);
        assert!(result.contains("MyView"));
        assert!(result.contains("NSView"));
    }

    #[test]
    fn test_objc_emitter_implementation() {
        let emitter = ObjCEmitter::new(ObjCMode::ObjC);
        let result = emitter.emit_implementation("MyView", Some("NSView"));
        assert!(result.contains("MyView"));
    }

    #[test]
    fn test_compile_command_new() {
        let cmd = CompileCommand::new(
            "/path/to/dir".to_string(),
            "g++ -o output input.cpp".to_string(),
            "input.cpp".to_string(),
        );
        assert_eq!(cmd.directory, "/path/to/dir");
        assert_eq!(cmd.file, "input.cpp");
    }

    #[test]
    fn test_compile_command_to_json() {
        let cmd = CompileCommand::new(
            "/path".to_string(),
            "g++ -c file.cpp".to_string(),
            "file.cpp".to_string(),
        );
        let json = cmd.to_json();
        assert!(json.contains("\"/path\""));
        assert!(json.contains("g++ -c file.cpp"));
        assert!(json.contains("file.cpp"));
    }

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello"), "hello");
        assert_eq!(escape_json("hello\"world"), "hello\\\"world");
        assert_eq!(escape_json("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_cpp_interop_new() {
        let interop = CppInterop::new(CppDialect::Cpp17);
        assert_eq!(interop.dialect, CppDialect::Cpp17);
    }

    #[test]
    fn test_cpp_interop_needs_mangling() {
        let interop = CppInterop::new(CppDialect::Cpp17);
        // Keywords need mangling
        assert!(interop.needs_mangling("class"));
        assert!(interop.needs_mangling("int"));
        assert!(interop.needs_mangling("namespace"));
        // Regular names don't
        assert!(!interop.needs_mangling("myFunction"));
        assert!(!interop.needs_mangling("foo_bar"));
    }

    #[test]
    fn test_cpp_interop_mangle_export() {
        let interop = CppInterop::new(CppDialect::Cpp17);
        // Keywords get underscore prefix
        assert_eq!(interop.mangle_export("class"), "_class");
        // Normal names stay the same
        assert_eq!(interop.mangle_export("myFunc"), "myFunc");
    }

    #[test]
    fn test_cpp_interop_extern_c_block() {
        let interop = CppInterop::new(CppDialect::Cpp17);
        let block = interop.emit_extern_c_block(&["foo", "bar"]);
        assert!(block.contains("extern \"C\""));
        assert!(block.contains("foo"));
        assert!(block.contains("bar"));
    }
}
