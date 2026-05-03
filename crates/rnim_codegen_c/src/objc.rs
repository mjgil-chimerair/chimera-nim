//! Objective-C specific interop support.
//!
//! This module provides Objective-C runtime support, message send
//! mangling, and class/property emission helpers.

use crate::cpp::ObjCSelector;
use rnim_span::Span;

/// Objective-C runtime type encoding
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjCTypeEncoding {
    /// Character
    Char,
    /// Short
    Short,
    /// Integer
    Int,
    /// Long
    Long,
    /// Long long
    LongLong,
    /// Float
    Float,
    /// Double
    Double,
    /// Long double
    LongDouble,
    /// Signed char
    SignedChar,
    /// Unsigned char
    UnsignedChar,
    /// Unsigned short
    UnsignedShort,
    /// Unsigned int
    UnsignedInt,
    /// Unsigned long
    UnsignedLong,
    /// Unsigned long long
    UnsignedLongLong,
    /// Void
    Void,
    /// Character string (char*)
    CharString,
    /// Object type
    Object,
    /// Class type
    Class,
    /// SEL type
    Sel,
    /// Unknown/Complex
    Unknown,
}

impl ObjCTypeEncoding {
    /// Get the type encoding character for this type
    pub fn encoding(&self) -> char {
        match self {
            ObjCTypeEncoding::Char => 'c',
            ObjCTypeEncoding::Short => 's',
            ObjCTypeEncoding::Int => 'i',
            ObjCTypeEncoding::Long => 'l',
            ObjCTypeEncoding::LongLong => 'q',
            ObjCTypeEncoding::Float => 'f',
            ObjCTypeEncoding::Double => 'd',
            ObjCTypeEncoding::LongDouble => 'D',
            ObjCTypeEncoding::SignedChar => 'c',
            ObjCTypeEncoding::UnsignedChar => 'C',
            ObjCTypeEncoding::UnsignedShort => 'S',
            ObjCTypeEncoding::UnsignedInt => 'I',
            ObjCTypeEncoding::UnsignedLong => 'L',
            ObjCTypeEncoding::UnsignedLongLong => 'Q',
            ObjCTypeEncoding::Void => 'v',
            ObjCTypeEncoding::CharString => '*',
            ObjCTypeEncoding::Object => '@',
            ObjCTypeEncoding::Class => '#',
            ObjCTypeEncoding::Sel => ':',
            ObjCTypeEncoding::Unknown => '?',
        }
    }

    /// Parse an Objective-C type encoding string
    pub fn parse_encoding(encoding: &str) -> Vec<ObjCTypeEncoding> {
        let mut result = Vec::new();
        let mut chars = encoding.chars().peekable();

        while let Some(&c) = chars.peek() {
            match c {
                'c' => result.push(ObjCTypeEncoding::Char),
                's' => result.push(ObjCTypeEncoding::Short),
                'i' => result.push(ObjCTypeEncoding::Int),
                'l' => result.push(ObjCTypeEncoding::Long),
                'q' => result.push(ObjCTypeEncoding::LongLong),
                'f' => result.push(ObjCTypeEncoding::Float),
                'd' => result.push(ObjCTypeEncoding::Double),
                'D' => result.push(ObjCTypeEncoding::LongDouble),
                'C' => result.push(ObjCTypeEncoding::UnsignedChar),
                'S' => result.push(ObjCTypeEncoding::UnsignedShort),
                'I' => result.push(ObjCTypeEncoding::UnsignedInt),
                'L' => result.push(ObjCTypeEncoding::UnsignedLong),
                'Q' => result.push(ObjCTypeEncoding::UnsignedLongLong),
                'v' => result.push(ObjCTypeEncoding::Void),
                '*' => result.push(ObjCTypeEncoding::CharString),
                '@' => result.push(ObjCTypeEncoding::Object),
                '#' => result.push(ObjCTypeEncoding::Class),
                ':' => result.push(ObjCTypeEncoding::Sel),
                '^' => {
                    // Skip pointer modifier, next char is the pointed-to type
                    chars.next();
                    if let Some(&next) = chars.peek() {
                        let mut single = String::new();
                        single.push(next);
                        result.extend(ObjCTypeEncoding::parse_encoding(&single));
                    }
                }
                'b' => {
                    // Bit field - skip digit count
                    chars.next();
                    while let Some(&next) = chars.peek() {
                        if next.is_ascii_digit() {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
                'r' => {
                    // Const modifier - skip
                    chars.next();
                    continue;
                }
                'n' | 'o' | 'O' | 'R' => {
                    // in/out/bycopy/byref/restrict - skip
                    chars.next();
                    continue;
                }
                '?' => result.push(ObjCTypeEncoding::Unknown),
                _ => {}
            }
            chars.next();
        }

        result
    }
}

/// Objective-C property attributes
#[derive(Debug, Clone, Default)]
pub struct ObjCPropertyAttrs {
    /// Readonly property
    pub readonly: bool,
    /// Copy property
    pub copy: bool,
    /// Retain property
    pub retain: bool,
    /// Nonatomic property
    pub nonatomic: bool,
    /// Getter name
    pub getter: Option<String>,
    /// Setter name
    pub setter: Option<String>,
}

impl ObjCPropertyAttrs {
    /// Create default property attributes
    pub fn new() -> Self {
        ObjCPropertyAttrs::default()
    }

    /// Add readonly attribute
    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }

    /// Add copy attribute
    pub fn copy(mut self) -> Self {
        self.copy = true;
        self
    }

    /// Add retain attribute
    pub fn retain(mut self) -> Self {
        self.retain = true;
        self
    }

    /// Add nonatomic attribute
    pub fn nonatomic(mut self) -> Self {
        self.nonatomic = true;
        self
    }

    /// Set getter method name
    pub fn getter(mut self, name: &str) -> Self {
        self.getter = Some(name.to_string());
        self
    }

    /// Set setter method name
    pub fn setter(mut self, name: &str) -> Self {
        self.setter = Some(name.to_string());
        self
    }

    /// Emit property declaration
    pub fn emit(&self, name: &str, type_encoding: &str) -> String {
        let mut attrs = Vec::new();

        attrs.push("@property".to_string());

        if self.readonly {
            attrs.push("(readonly)".to_string());
        } else {
            let mut prop_attrs = String::from("(");
            let mut first = true;

            if self.copy {
                if !first {
                    prop_attrs.push_str(", ");
                }
                prop_attrs.push_str("copy");
                first = false;
            }
            if self.retain {
                if !first {
                    prop_attrs.push_str(", ");
                }
                prop_attrs.push_str("retain");
                first = false;
            }
            if self.nonatomic {
                if !first {
                    prop_attrs.push_str(", ");
                }
                prop_attrs.push_str("nonatomic");
                first = false;
            }
            if first {
                prop_attrs.push_str("assign");
            }
            prop_attrs.push(')');
            attrs.push(prop_attrs);
        }

        if let Some(ref getter) = self.getter {
            attrs.push(format!("getter={}", getter));
        }
        if let Some(ref setter) = self.setter {
            attrs.push(format!("setter={}", setter));
        }

        format!("{} {} {}", attrs.join(" "), type_encoding, name)
    }
}

/// Objective-C class info for runtime
#[derive(Debug, Clone)]
pub struct ObjCClassInfo {
    /// Class name
    pub name: String,
    /// Superclass name
    pub superclass: Option<String>,
    /// Instance size
    pub instance_size: usize,
    /// Ivars
    pub ivars: Vec<ObjCIvarInfo>,
}

impl ObjCClassInfo {
    /// Create new class info
    pub fn new(name: &str) -> Self {
        ObjCClassInfo {
            name: name.to_string(),
            superclass: None,
            instance_size: 0,
            ivars: Vec::new(),
        }
    }

    /// Set superclass
    pub fn with_superclass(mut self, superclass: &str) -> Self {
        self.superclass = Some(superclass.to_string());
        self
    }

    /// Add ivar
    pub fn add_ivar(&mut self, name: &str, encoding: ObjCTypeEncoding) {
        self.ivars.push(ObjCIvarInfo {
            name: name.to_string(),
            encoding,
        });
    }
}

/// Objective-C instance variable info
#[derive(Debug, Clone)]
pub struct ObjCIvarInfo {
    /// Ivar name
    pub name: String,
    /// Type encoding
    pub encoding: ObjCTypeEncoding,
}

/// Objective-C method signature
#[derive(Debug, Clone)]
pub struct ObjCMethodSignature {
    /// Return type encoding
    pub return_type: Vec<ObjCTypeEncoding>,
    /// Argument type encodings
    pub argument_types: Vec<ObjCTypeEncoding>,
}

impl ObjCMethodSignature {
    /// Create new method signature
    pub fn new(return_type: Vec<ObjCTypeEncoding>) -> Self {
        ObjCMethodSignature {
            return_type,
            argument_types: Vec::new(),
        }
    }

    /// Add argument type
    pub fn add_argument(&mut self, encoding: ObjCTypeEncoding) {
        self.argument_types.push(encoding);
    }

    /// Get encoding string for signature
    pub fn to_encoding_string(&self) -> String {
        let mut result = self
            .return_type
            .iter()
            .map(|e| e.encoding())
            .collect::<String>();
        for arg in &self.argument_types {
            result.push(arg.encoding());
        }
        result
    }
}

/// Message send helper for calling Objective-C methods
#[derive(Debug, Clone)]
pub struct ObjCMessageSend {
    /// Target expression
    target: String,
    /// Selector
    selector: ObjCSelector,
}

impl ObjCMessageSend {
    /// Create a new message send
    pub fn new(target: &str, selector: ObjCSelector) -> Self {
        ObjCMessageSend {
            target: target.to_string(),
            selector,
        }
    }

    /// Emit message send expression
    pub fn emit(&self, args: &[&str]) -> String {
        let sel_key = self.selector.key();
        if args.is_empty() {
            format!("[{} {}]", self.target, sel_key)
        } else if args.len() == 1 {
            format!("[{} {}: {}]", self.target, sel_key, args[0])
        } else {
            // Multi-argument case
            let parts: Vec<&str> = sel_key.split(':').collect();
            let mut result = format!("[{} ", self.target);
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    result.push_str(" ");
                }
                result.push_str(part);
                result.push(':');
                if i < args.len() {
                    result.push_str(&format!("{}: {}", part, args[i]));
                }
            }
            result.push(']');
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objc_type_encoding_char() {
        assert_eq!(ObjCTypeEncoding::Char.encoding(), 'c');
    }

    #[test]
    fn test_objc_type_encoding_int() {
        assert_eq!(ObjCTypeEncoding::Int.encoding(), 'i');
    }

    #[test]
    fn test_objc_type_encoding_object() {
        assert_eq!(ObjCTypeEncoding::Object.encoding(), '@');
    }

    #[test]
    fn test_objc_type_encoding_parse_simple() {
        let encodings = ObjCTypeEncoding::parse_encoding("i@:");
        assert_eq!(encodings.len(), 3);
        assert_eq!(encodings[0], ObjCTypeEncoding::Int);
        assert_eq!(encodings[1], ObjCTypeEncoding::Object);
        assert_eq!(encodings[2], ObjCTypeEncoding::Sel);
    }

    #[test]
    fn test_objc_type_encoding_parse_pointer() {
        let encodings = ObjCTypeEncoding::parse_encoding("^i");
        // Pointer is consumed and replaced with pointed-to type
        assert_eq!(encodings.len(), 1);
        assert_eq!(encodings[0], ObjCTypeEncoding::Int);
    }

    #[test]
    fn test_objc_type_encoding_parse_const() {
        let encodings = ObjCTypeEncoding::parse_encoding("ri");
        assert_eq!(encodings.len(), 1);
        assert_eq!(encodings[0], ObjCTypeEncoding::Int);
    }

    #[test]
    fn test_objc_property_attrs_new() {
        let attrs = ObjCPropertyAttrs::new();
        assert!(!attrs.readonly);
        assert!(!attrs.copy);
        assert!(!attrs.retain);
        assert!(!attrs.nonatomic);
    }

    #[test]
    fn test_objc_property_attrs_readonly() {
        let attrs = ObjCPropertyAttrs::new().readonly();
        assert!(attrs.readonly);
    }

    #[test]
    fn test_objc_property_attrs_copy_retain() {
        let attrs = ObjCPropertyAttrs::new().copy().retain();
        assert!(attrs.copy);
        assert!(attrs.retain);
    }

    #[test]
    fn test_objc_property_attrs_emit() {
        let attrs = ObjCPropertyAttrs::new().nonatomic();
        let prop = attrs.emit("myProperty", "NSString *");
        assert!(prop.contains("@property"));
        assert!(prop.contains("nonatomic"));
        assert!(prop.contains("NSString"));
        assert!(prop.contains("myProperty"));
    }

    #[test]
    fn test_objc_property_attrs_emit_readonly() {
        let attrs = ObjCPropertyAttrs::new().readonly().getter("isReady");
        let prop = attrs.emit("ready", "BOOL");
        assert!(prop.contains("(readonly)"));
        assert!(prop.contains("getter=isReady"));
    }

    #[test]
    fn test_objc_class_info_new() {
        let info = ObjCClassInfo::new("MyClass");
        assert_eq!(info.name, "MyClass");
        assert!(info.superclass.is_none());
        assert_eq!(info.instance_size, 0);
        assert!(info.ivars.is_empty());
    }

    #[test]
    fn test_objc_class_info_with_superclass() {
        let info = ObjCClassInfo::new("MyClass").with_superclass("NSObject");
        assert_eq!(info.superclass, Some("NSObject".to_string()));
    }

    #[test]
    fn test_objc_class_info_add_ivar() {
        let mut info = ObjCClassInfo::new("MyClass");
        info.add_ivar("value", ObjCTypeEncoding::Int);
        assert_eq!(info.ivars.len(), 1);
        assert_eq!(info.ivars[0].name, "value");
    }

    #[test]
    fn test_objc_method_signature_new() {
        let sig = ObjCMethodSignature::new(vec![ObjCTypeEncoding::Void]);
        assert_eq!(sig.return_type.len(), 1);
        assert!(sig.argument_types.is_empty());
    }

    #[test]
    fn test_objc_method_signature_add_argument() {
        let mut sig = ObjCMethodSignature::new(vec![ObjCTypeEncoding::Int]);
        sig.add_argument(ObjCTypeEncoding::Object);
        assert_eq!(sig.argument_types.len(), 1);
    }

    #[test]
    fn test_objc_method_signature_to_encoding_string() {
        let mut sig = ObjCMethodSignature::new(vec![ObjCTypeEncoding::Int]);
        sig.add_argument(ObjCTypeEncoding::Object);
        assert_eq!(sig.to_encoding_string(), "i@");
    }

    #[test]
    fn test_objc_message_send_new() {
        let send = ObjCMessageSend::new("obj", ObjCSelector::parse("init"));
        assert_eq!(send.target, "obj");
    }

    #[test]
    fn test_objc_message_send_emit_no_args() {
        let send = ObjCMessageSend::new("obj", ObjCSelector::parse("description"));
        let result = send.emit(&[]);
        assert_eq!(result, "[obj description]");
    }

    #[test]
    fn test_objc_message_send_emit_one_arg() {
        let send = ObjCMessageSend::new("obj", ObjCSelector::parse("initWithFrame:"));
        let result = send.emit(&["NSMakeRect(0,0,100,100)"]);
        assert!(result.contains("initWithFrame:"));
        assert!(result.contains("NSMakeRect"));
    }
}
