//! Type representation, unification, conversions, concepts/type classes, generic instantiation.

use fnv::FnvHasher;
#[cfg(test)]
use rnim_allocator as _;
use rnim_span::Span;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

/// Type ID for interning types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyId(u32);

impl TyId {
    pub fn new(index: u32) -> Self {
        TyId(index)
    }

    pub fn index(&self) -> u32 {
        self.0
    }
}

impl Default for TyId {
    fn default() -> Self {
        TyId(u32::MAX)
    }
}

/// A fast hash map using FnvHasher
pub type FxHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FnvHasher>>;

/// Primitive type kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    Bool,
    Char,
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float,
    Float32,
    Float64,
}

/// Ordinal types include integers, chars, enums, and subranges
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OrdinalKind {
    Integer,
    Char,
    Enum,
    Subrange,
}

/// A primitive or ordinal type
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct PrimitiveType {
    pub kind: PrimitiveKind,
    pub size: u32,
    pub is_signed: bool,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
}

impl PrimitiveType {
    pub fn new_int(size: u32, is_signed: bool) -> Self {
        let bits = size.saturating_mul(8).max(1);
        let (min_val, max_val) = if is_signed {
            let (min_v, max_v) = if bits >= 64 {
                (i64::MIN, i64::MAX)
            } else {
                let half = 1i64 << (bits - 1);
                (-half, half - 1)
            };
            (Some(min_v), Some(max_v))
        } else {
            let max_v = if bits >= 64 {
                i64::MAX
            } else {
                ((1u64 << bits) - 1) as i64
            };
            (Some(0), Some(max_v))
        };
        PrimitiveType {
            kind: if is_signed {
                PrimitiveKind::Int
            } else {
                PrimitiveKind::Uint
            },
            size,
            is_signed,
            min_value: min_val,
            max_value: max_val,
        }
    }

    pub fn new_float(size: u32) -> Self {
        PrimitiveType {
            kind: if size == 4 {
                PrimitiveKind::Float32
            } else {
                PrimitiveKind::Float64
            },
            size,
            is_signed: true,
            min_value: None,
            max_value: None,
        }
    }

    pub fn ordinal_kind(&self) -> OrdinalKind {
        match self.kind {
            PrimitiveKind::Bool
            | PrimitiveKind::Int
            | PrimitiveKind::Int8
            | PrimitiveKind::Int16
            | PrimitiveKind::Int32
            | PrimitiveKind::Int64
            | PrimitiveKind::Uint
            | PrimitiveKind::Uint8
            | PrimitiveKind::Uint16
            | PrimitiveKind::Uint32
            | PrimitiveKind::Uint64 => OrdinalKind::Integer,
            PrimitiveKind::Char => OrdinalKind::Char,
            PrimitiveKind::Float | PrimitiveKind::Float32 | PrimitiveKind::Float64 => {
                panic!("Floats are not ordinal")
            }
        }
    }
}

/// Enum type with named values
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct EnumType {
    pub name: Box<str>,
    pub values: Vec<EnumValue>,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct EnumValue {
    pub name: Box<str>,
    pub ordinal: i64,
    pub span: Span,
}

/// Subrange type with bounds
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct SubrangeType {
    pub base: TyId,
    pub lower: i64,
    pub upper: i64,
}

/// Check if a type is ordinal (can be used in case statements, for loops, etc.)
pub fn is_ordinal(ty: &Type) -> bool {
    match ty {
        Type::Primitive(p) => matches!(
            p.kind,
            PrimitiveKind::Bool
                | PrimitiveKind::Char
                | PrimitiveKind::Int
                | PrimitiveKind::Int8
                | PrimitiveKind::Int16
                | PrimitiveKind::Int32
                | PrimitiveKind::Int64
                | PrimitiveKind::Uint
                | PrimitiveKind::Uint8
                | PrimitiveKind::Uint16
                | PrimitiveKind::Uint32
                | PrimitiveKind::Uint64
        ),
        _ => false,
    }
}

/// Check if a type is integral
pub fn is_integral(ty: &Type) -> bool {
    match ty {
        Type::Primitive(p) => matches!(
            p.kind,
            PrimitiveKind::Bool
                | PrimitiveKind::Char
                | PrimitiveKind::Int
                | PrimitiveKind::Int8
                | PrimitiveKind::Int16
                | PrimitiveKind::Int32
                | PrimitiveKind::Int64
                | PrimitiveKind::Uint
                | PrimitiveKind::Uint8
                | PrimitiveKind::Uint16
                | PrimitiveKind::Uint32
                | PrimitiveKind::Uint64
        ),
        _ => false,
    }
}

/// Check if a type is floating point
pub fn is_float(ty: &Type) -> bool {
    match ty {
        Type::Primitive(p) => matches!(
            p.kind,
            PrimitiveKind::Float | PrimitiveKind::Float32 | PrimitiveKind::Float64
        ),
        _ => false,
    }
}

/// Check if two primitive types are compatible
pub fn primitive_compatible(a: &PrimitiveType, b: &PrimitiveType) -> bool {
    a.kind == b.kind || (a.is_signed == b.is_signed && a.size == b.size)
}

/// Get the default value for a primitive type
pub fn default_primitive_value(kind: PrimitiveKind) -> LiteralValue {
    match kind {
        PrimitiveKind::Bool => LiteralValue::Bool(false),
        PrimitiveKind::Char => LiteralValue::Char('\0'),
        PrimitiveKind::Int
        | PrimitiveKind::Int8
        | PrimitiveKind::Int16
        | PrimitiveKind::Int32
        | PrimitiveKind::Int64 => LiteralValue::Int(0),
        PrimitiveKind::Uint
        | PrimitiveKind::Uint8
        | PrimitiveKind::Uint16
        | PrimitiveKind::Uint32
        | PrimitiveKind::Uint64 => LiteralValue::Uint(0),
        PrimitiveKind::Float | PrimitiveKind::Float32 | PrimitiveKind::Float64 => {
            LiteralValue::Float(0.0)
        }
    }
}

/// Literal values for constants
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Bool(bool),
    Char(char),
    Int(i64),
    Uint(u64),
    Float(f64),
    String(Box<str>),
}

/// Type representation
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Type {
    /// A primitive type
    Primitive(PrimitiveType),
    /// An enum type
    Enum(EnumType),
    /// A subrange type
    Subrange(SubrangeType),
    /// Named type alias
    Alias { name: Box<str>, underlying: TyId },
    /// A distinct type
    Distinct { name: Box<str>, underlying: TyId },
    /// Object type with fields
    Object {
        fields: Vec<Field>,
        base: Option<TyId>,
    },
    /// Array type
    Array { elem: TyId, len: Option<usize> },
    /// Open array (varargs)
    OpenArray { elem: TyId },
    /// Sequence type
    Seq { elem: TyId },
    /// Set type
    Set { elem: TyId },
    /// Tuple type
    Tuple { fields: Vec<TyId> },
    /// Reference type
    Ref { inner: TyId },
    /// Pointer type
    Ptr { inner: TyId },
    /// Procedure/function type
    Proc {
        params: Vec<TyId>,
        ret: Option<TyId>,
        calling_convention: CallingConvention,
    },
    /// Type descriptor (metatype)
    TypeDesc { inner: Option<TyId> },
    /// Static type (compile-time known)
    Static { inner: TyId },
    /// Varargs
    Varargs { elem: TyId },
    /// Nil type
    Nil,
    /// Error type (for error recovery)
    Error,
    /// Void (no value)
    Void,
}

/// Field in an object or tuple
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Field {
    pub name: Box<str>,
    pub typ: TyId,
    pub offset: Option<u32>,
}

/// Calling convention for procedures
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum CallingConvention {
    Nimcall,
    Cdecl,
    Stdcall,
    Syscall,
    Inline,
    NoInline,
    Fastcall,
    Thiscall,
    Pascal,
}

impl Default for CallingConvention {
    fn default() -> Self {
        CallingConvention::Nimcall
    }
}

/// Type context for interning and managing types
#[derive(Default)]
pub struct TypeCtx {
    types: FxHashMap<TyId, Type>,
    primitives: FxHashMap<PrimitiveKind, TyId>,
    canonical: FxHashMap<Type, TyId>,
    next_id: u32,
}

impl TypeCtx {
    pub fn new() -> Self {
        let mut ctx = TypeCtx {
            types: FxHashMap::default(),
            primitives: FxHashMap::default(),
            canonical: FxHashMap::default(),
            next_id: 0,
        };
        // Initialize primitive types
        ctx.init_primitives();
        ctx
    }

    fn init_primitives(&mut self) {
        // Bool
        let bool_type = PrimitiveType {
            kind: PrimitiveKind::Bool,
            size: 1,
            is_signed: false,
            min_value: Some(0),
            max_value: Some(1),
        };
        let bool_id = self.intern_type(Type::Primitive(bool_type));
        self.primitives.insert(PrimitiveKind::Bool, bool_id);

        // Char
        let char_type = PrimitiveType {
            kind: PrimitiveKind::Char,
            size: 4,
            is_signed: false,
            min_value: Some(0),
            max_value: Some(0x10FFFF),
        };
        let char_id = self.intern_type(Type::Primitive(char_type));
        self.primitives.insert(PrimitiveKind::Char, char_id);

        // Platform-dependent int
        let int_type = PrimitiveType::new_int(8, true);
        let int_id = self.intern_type(Type::Primitive(int_type));
        self.primitives.insert(PrimitiveKind::Int, int_id);

        // Float64
        let float_type = PrimitiveType::new_float(8);
        let float_id = self.intern_type(Type::Primitive(float_type));
        self.primitives.insert(PrimitiveKind::Float64, float_id);
    }

    pub fn intern_type(&mut self, ty: Type) -> TyId {
        if let Some(&id) = self.canonical.get(&ty) {
            return id;
        }
        let id = TyId(self.next_id);
        self.next_id += 1;
        self.types.insert(id, ty.clone());
        self.canonical.insert(ty, id);
        id
    }

    pub fn get(&self, id: TyId) -> Option<&Type> {
        self.types.get(&id)
    }

    pub fn get_primitive(&self, kind: PrimitiveKind) -> Option<TyId> {
        self.primitives.get(&kind).copied()
    }

    /// Get or create a primitive type
    pub fn get_or_create_primitive(&mut self, kind: PrimitiveKind) -> TyId {
        if let Some(&id) = self.primitives.get(&kind) {
            return id;
        }
        let prim = match kind {
            PrimitiveKind::Bool => PrimitiveType {
                kind,
                size: 1,
                is_signed: false,
                min_value: Some(0),
                max_value: Some(1),
            },
            PrimitiveKind::Char => PrimitiveType {
                kind,
                size: 4,
                is_signed: false,
                min_value: Some(0),
                max_value: Some(0x10FFFF),
            },
            PrimitiveKind::Int => PrimitiveType::new_int(8, true),
            PrimitiveKind::Int8 => PrimitiveType::new_int(1, true),
            PrimitiveKind::Int16 => PrimitiveType::new_int(2, true),
            PrimitiveKind::Int32 => PrimitiveType::new_int(4, true),
            PrimitiveKind::Int64 => PrimitiveType::new_int(8, true),
            PrimitiveKind::Uint => PrimitiveType::new_int(8, false),
            PrimitiveKind::Uint8 => PrimitiveType::new_int(1, false),
            PrimitiveKind::Uint16 => PrimitiveType::new_int(2, false),
            PrimitiveKind::Uint32 => PrimitiveType::new_int(4, false),
            PrimitiveKind::Uint64 => PrimitiveType::new_int(8, false),
            PrimitiveKind::Float | PrimitiveKind::Float32 => PrimitiveType::new_float(4),
            PrimitiveKind::Float64 => PrimitiveType::new_float(8),
        };
        let id = self.intern_type(Type::Primitive(prim));
        self.primitives.insert(kind, id);
        id
    }

    /// Create an enum type
    pub fn create_enum(&mut self, name: Box<str>, values: Vec<EnumValue>) -> TyId {
        self.intern_type(Type::Enum(EnumType { name, values }))
    }

    /// Create a subrange type
    pub fn create_subrange(&mut self, base: TyId, lower: i64, upper: i64) -> TyId {
        self.intern_type(Type::Subrange(SubrangeType { base, lower, upper }))
    }

    /// Create a distinct type
    pub fn create_distinct(&mut self, name: Box<str>, underlying: TyId) -> TyId {
        self.intern_type(Type::Distinct { name, underlying })
    }

    /// Create a type alias
    pub fn create_alias(&mut self, name: Box<str>, underlying: TyId) -> TyId {
        self.intern_type(Type::Alias { name, underlying })
    }

    /// Create an object type
    pub fn create_object(&mut self, fields: Vec<Field>, base: Option<TyId>) -> TyId {
        self.intern_type(Type::Object { fields, base })
    }

    /// Create an array type
    pub fn create_array(&mut self, elem: TyId, len: Option<usize>) -> TyId {
        self.intern_type(Type::Array { elem, len })
    }

    /// Create an open array type
    pub fn create_open_array(&mut self, elem: TyId) -> TyId {
        self.intern_type(Type::OpenArray { elem })
    }

    /// Create a seq type
    pub fn create_seq(&mut self, elem: TyId) -> TyId {
        self.intern_type(Type::Seq { elem })
    }

    /// Create a set type
    pub fn create_set(&mut self, elem: TyId) -> TyId {
        self.intern_type(Type::Set { elem })
    }

    /// Create a tuple type
    pub fn create_tuple(&mut self, fields: Vec<TyId>) -> TyId {
        self.intern_type(Type::Tuple { fields })
    }

    /// Create a ref type
    pub fn create_ref(&mut self, inner: TyId) -> TyId {
        self.intern_type(Type::Ref { inner })
    }

    /// Create a ptr type
    pub fn create_ptr(&mut self, inner: TyId) -> TyId {
        self.intern_type(Type::Ptr { inner })
    }

    /// Create a proc type
    pub fn create_proc(
        &mut self,
        params: Vec<TyId>,
        ret: Option<TyId>,
        cc: CallingConvention,
    ) -> TyId {
        self.intern_type(Type::Proc {
            params,
            ret,
            calling_convention: cc,
        })
    }

    /// Create a typedesc type
    pub fn create_typedesc(&mut self, inner: Option<TyId>) -> TyId {
        self.intern_type(Type::TypeDesc { inner })
    }

    /// Create a static type
    pub fn create_static(&mut self, inner: TyId) -> TyId {
        self.intern_type(Type::Static { inner })
    }

    /// Create a varargs type
    pub fn create_varargs(&mut self, elem: TyId) -> TyId {
        self.intern_type(Type::Varargs { elem })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_ty_id_index() {
        let id = TyId::new(42);
        assert_eq!(id.index(), 42);
    }

    #[test]
    fn test_ty_id_default() {
        let id = TyId::default();
        assert_eq!(id.index(), u32::MAX);
    }

    #[test]
    fn test_ty_id_equality() {
        let a = TyId::new(1);
        let b = TyId::new(1);
        let c = TyId::new(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_primitive_type_new_int() {
        let p = PrimitiveType::new_int(4, true);
        assert!(p.is_signed);
        assert_eq!(p.size, 4);
        assert!(p.min_value.is_some());
        assert!(p.max_value.is_some());
    }

    #[test]
    fn test_primitive_type_new_int_unsigned() {
        let p = PrimitiveType::new_int(4, false);
        assert!(!p.is_signed);
        assert_eq!(p.size, 4);
        assert_eq!(p.min_value, Some(0));
    }

    #[test]
    fn test_primitive_type_new_float() {
        let p = PrimitiveType::new_float(4);
        assert_eq!(p.kind, PrimitiveKind::Float32);
        assert!(p.is_signed);
    }

    #[test]
    fn test_primitive_type_new_float64() {
        let p = PrimitiveType::new_float(8);
        assert_eq!(p.kind, PrimitiveKind::Float64);
    }

    #[test]
    fn test_primitive_type_ordinal_kind() {
        let int_type = PrimitiveType::new_int(4, true);
        assert_eq!(int_type.ordinal_kind(), OrdinalKind::Integer);

        let char_type = PrimitiveType {
            kind: PrimitiveKind::Char,
            size: 1,
            is_signed: false,
            min_value: Some(0),
            max_value: Some(255),
        };
        assert_eq!(char_type.ordinal_kind(), OrdinalKind::Char);
    }

    #[test]
    fn test_ordinal_kind_variants() {
        assert!(matches!(OrdinalKind::Integer, OrdinalKind::Integer));
        assert!(matches!(OrdinalKind::Char, OrdinalKind::Char));
        assert!(matches!(OrdinalKind::Enum, OrdinalKind::Enum));
        assert!(matches!(OrdinalKind::Subrange, OrdinalKind::Subrange));
    }

    #[test]
    fn test_is_ordinal() {
        let int_type = Type::Primitive(PrimitiveType::new_int(4, true));
        assert!(is_ordinal(&int_type));

        let float_type = Type::Primitive(PrimitiveType::new_float(8));
        assert!(!is_ordinal(&float_type));

        let seq_type = Type::Seq { elem: TyId::new(0) };
        assert!(!is_ordinal(&seq_type));
    }

    #[test]
    fn test_is_integral() {
        let int_type = Type::Primitive(PrimitiveType::new_int(4, true));
        assert!(is_integral(&int_type));

        let float_type = Type::Primitive(PrimitiveType::new_float(8));
        assert!(!is_integral(&float_type));

        let bool_type = Type::Primitive(PrimitiveType {
            kind: PrimitiveKind::Bool,
            size: 1,
            is_signed: false,
            min_value: Some(0),
            max_value: Some(1),
        });
        assert!(is_integral(&bool_type));
    }

    #[test]
    fn test_is_float() {
        let float_type = Type::Primitive(PrimitiveType::new_float(8));
        assert!(is_float(&float_type));

        let int_type = Type::Primitive(PrimitiveType::new_int(4, true));
        assert!(!is_float(&int_type));
    }

    #[test]
    fn test_primitive_compatible() {
        let a = PrimitiveType::new_int(4, true);
        let b = PrimitiveType::new_int(4, true);
        let c = PrimitiveType::new_int(4, false);
        assert!(primitive_compatible(&a, &b));
        assert!(!primitive_compatible(&a, &c));
    }

    #[test]
    fn test_default_primitive_value() {
        assert_eq!(
            default_primitive_value(PrimitiveKind::Bool),
            LiteralValue::Bool(false)
        );
        assert_eq!(
            default_primitive_value(PrimitiveKind::Char),
            LiteralValue::Char('\0')
        );
        assert_eq!(
            default_primitive_value(PrimitiveKind::Int),
            LiteralValue::Int(0)
        );
        assert_eq!(
            default_primitive_value(PrimitiveKind::Uint),
            LiteralValue::Uint(0)
        );
        assert_eq!(
            default_primitive_value(PrimitiveKind::Float),
            LiteralValue::Float(0.0)
        );
    }

    #[test]
    fn test_literal_value_variants() {
        assert!(matches!(LiteralValue::Bool(true), LiteralValue::Bool(true)));
        assert!(matches!(LiteralValue::Char('a'), LiteralValue::Char('a')));
        assert!(matches!(LiteralValue::Int(42), LiteralValue::Int(42)));
        assert!(matches!(LiteralValue::Uint(42), LiteralValue::Uint(42)));
        assert!(matches!(
            LiteralValue::Float(3.14),
            LiteralValue::Float(3.14)
        ));
        let s = LiteralValue::String("hello".into());
        assert!(matches!(s, LiteralValue::String(_)));
    }

    #[test]
    fn test_type_primitive() {
        let p = PrimitiveType::new_int(8, true);
        let t = Type::Primitive(p);
        assert!(matches!(t, Type::Primitive(_)));
    }

    #[test]
    fn test_type_enum() {
        let e = EnumType {
            name: "Color".into(),
            values: vec![EnumValue {
                name: "Red".into(),
                ordinal: 0,
                span: Span::new(FileId::new(0), 0, 3),
            }],
        };
        let t = Type::Enum(e);
        assert!(matches!(t, Type::Enum(_)));
    }

    #[test]
    fn test_type_subrange() {
        let t = Type::Subrange(SubrangeType {
            base: TyId::new(1),
            lower: 1,
            upper: 10,
        });
        assert!(matches!(t, Type::Subrange(s) if s.lower == 1 && s.upper == 10));
    }

    #[test]
    fn test_type_alias() {
        let t = Type::Alias {
            name: "MyInt".into(),
            underlying: TyId::new(2),
        };
        assert!(matches!(t, Type::Alias { name, underlying: _ } if name.as_ref() == "MyInt"));
    }

    #[test]
    fn test_type_distinct() {
        let t = Type::Distinct {
            name: "Meter".into(),
            underlying: TyId::new(3),
        };
        assert!(matches!(t, Type::Distinct { name, underlying: _ } if name.as_ref() == "Meter"));
    }

    #[test]
    fn test_type_object() {
        let t = Type::Object {
            fields: vec![Field {
                name: "x".into(),
                typ: TyId::new(4),
                offset: Some(0),
            }],
            base: None,
        };
        assert!(matches!(t, Type::Object { fields, base: None } if fields.len() == 1));
    }

    #[test]
    fn test_type_object_with_base() {
        let t = Type::Object {
            fields: vec![],
            base: Some(TyId::new(5)),
        };
        assert!(matches!(
            t,
            Type::Object {
                fields: _,
                base: Some(_)
            }
        ));
    }

    #[test]
    fn test_type_array() {
        let t = Type::Array {
            elem: TyId::new(6),
            len: Some(10),
        };
        assert!(matches!(
            t,
            Type::Array {
                elem: _,
                len: Some(10)
            }
        ));
    }

    #[test]
    fn test_type_open_array() {
        let t = Type::OpenArray { elem: TyId::new(7) };
        assert!(matches!(t, Type::OpenArray { elem: _ }));
    }

    #[test]
    fn test_type_seq() {
        let t = Type::Seq { elem: TyId::new(8) };
        assert!(matches!(t, Type::Seq { elem: _ }));
    }

    #[test]
    fn test_type_set() {
        let t = Type::Set { elem: TyId::new(9) };
        assert!(matches!(t, Type::Set { elem: _ }));
    }

    #[test]
    fn test_type_tuple() {
        let t = Type::Tuple {
            fields: vec![TyId::new(10), TyId::new(11)],
        };
        assert!(matches!(t, Type::Tuple { fields } if fields.len() == 2));
    }

    #[test]
    fn test_type_ref() {
        let t = Type::Ref {
            inner: TyId::new(12),
        };
        assert!(matches!(t, Type::Ref { inner: _ }));
    }

    #[test]
    fn test_type_ptr() {
        let t = Type::Ptr {
            inner: TyId::new(13),
        };
        assert!(matches!(t, Type::Ptr { inner: _ }));
    }

    #[test]
    fn test_type_proc() {
        let t = Type::Proc {
            params: vec![TyId::new(14)],
            ret: Some(TyId::new(15)),
            calling_convention: CallingConvention::Nimcall,
        };
        assert!(matches!(
            t,
            Type::Proc {
                params: _,
                ret: Some(_),
                calling_convention: CallingConvention::Nimcall
            }
        ));
    }

    #[test]
    fn test_type_typedesc() {
        let t = Type::TypeDesc {
            inner: Some(TyId::new(16)),
        };
        assert!(matches!(t, Type::TypeDesc { inner: Some(_) }));
    }

    #[test]
    fn test_type_typedesc_none() {
        let t = Type::TypeDesc { inner: None };
        assert!(matches!(t, Type::TypeDesc { inner: None }));
    }

    #[test]
    fn test_type_static() {
        let t = Type::Static {
            inner: TyId::new(17),
        };
        assert!(matches!(t, Type::Static { inner: _ }));
    }

    #[test]
    fn test_type_varargs() {
        let t = Type::Varargs {
            elem: TyId::new(18),
        };
        assert!(matches!(t, Type::Varargs { elem: _ }));
    }

    #[test]
    fn test_type_nil() {
        let t = Type::Nil;
        assert!(matches!(t, Type::Nil));
    }

    #[test]
    fn test_type_error() {
        let t = Type::Error;
        assert!(matches!(t, Type::Error));
    }

    #[test]
    fn test_type_void() {
        let t = Type::Void;
        assert!(matches!(t, Type::Void));
    }

    #[test]
    fn test_field() {
        let f = Field {
            name: "value".into(),
            typ: TyId::new(19),
            offset: Some(4),
        };
        assert_eq!(f.name.as_ref(), "value");
        assert_eq!(f.typ, TyId::new(19));
        assert_eq!(f.offset, Some(4));
    }

    #[test]
    fn test_calling_convention_variants() {
        assert!(matches!(
            CallingConvention::Nimcall,
            CallingConvention::Nimcall
        ));
        assert!(matches!(CallingConvention::Cdecl, CallingConvention::Cdecl));
        assert!(matches!(
            CallingConvention::Stdcall,
            CallingConvention::Stdcall
        ));
        assert!(matches!(
            CallingConvention::Syscall,
            CallingConvention::Syscall
        ));
        assert!(matches!(
            CallingConvention::Inline,
            CallingConvention::Inline
        ));
        assert!(matches!(
            CallingConvention::NoInline,
            CallingConvention::NoInline
        ));
        assert!(matches!(
            CallingConvention::Fastcall,
            CallingConvention::Fastcall
        ));
        assert!(matches!(
            CallingConvention::Thiscall,
            CallingConvention::Thiscall
        ));
        assert!(matches!(
            CallingConvention::Pascal,
            CallingConvention::Pascal
        ));
    }

    #[test]
    fn test_calling_convention_equality() {
        assert_eq!(CallingConvention::Nimcall, CallingConvention::Nimcall);
        assert_ne!(CallingConvention::Nimcall, CallingConvention::Cdecl);
    }

    #[test]
    fn test_type_ctx_new() {
        let ctx = TypeCtx::default();
        assert_eq!(ctx.types.len(), 0);
    }

    #[test]
    fn test_type_ctx_intern_primitive() {
        let mut ctx = TypeCtx::default();
        let id = ctx.intern_type(Type::Primitive(PrimitiveType::new_int(4, true)));
        assert_eq!(id.index(), 0);
        assert_eq!(ctx.types.len(), 1);
    }

    #[test]
    fn test_type_ctx_intern_multiple() {
        let mut ctx = TypeCtx::default();
        let id1 = ctx.intern_type(Type::Primitive(PrimitiveType::new_int(4, true)));
        let id2 = ctx.intern_type(Type::Primitive(PrimitiveType::new_int(4, true)));
        let id3 = ctx.intern_type(Type::Primitive(PrimitiveType::new_int(8, false)));
        // Same type interned twice returns same ID
        assert_eq!(id1, id2);
        // Different type gets different ID
        assert_ne!(id1, id3);
        assert_eq!(ctx.types.len(), 2);
    }

    #[test]
    fn test_type_ctx_get() {
        let mut ctx = TypeCtx::default();
        let id = ctx.intern_type(Type::Primitive(PrimitiveType::new_int(4, true)));
        assert!(ctx.get(id).is_some());
        assert!(matches!(ctx.get(id), Some(Type::Primitive(_))));
    }

    #[test]
    fn test_type_ctx_get_invalid() {
        let ctx = TypeCtx::default();
        assert!(ctx.get(TyId::new(999)).is_none());
    }

    #[test]
    fn test_type_ctx_create_primitives() {
        let mut ctx = TypeCtx::default();
        let _ = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let _ = ctx.get_or_create_primitive(PrimitiveKind::Float);
        let _ = ctx.get_or_create_primitive(PrimitiveKind::Bool);
        let _ = ctx.get_or_create_primitive(PrimitiveKind::Char);
        assert!(ctx.types.len() >= 4);
    }

    #[test]
    fn test_type_ctx_create_array() {
        let mut ctx = TypeCtx::default();
        let elem = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let arr = ctx.create_array(elem, Some(10));
        assert!(arr.index() > elem.index());
    }

    #[test]
    fn test_type_ctx_create_seq() {
        let mut ctx = TypeCtx::default();
        let elem = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let seq = ctx.create_seq(elem);
        assert!(seq.index() > elem.index());
    }

    #[test]
    fn test_type_ctx_create_set() {
        let mut ctx = TypeCtx::default();
        let elem = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let set = ctx.create_set(elem);
        assert!(set.index() > elem.index());
    }

    #[test]
    fn test_type_ctx_create_tuple() {
        let mut ctx = TypeCtx::default();
        let a = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let b = ctx.get_or_create_primitive(PrimitiveKind::Float);
        let tup = ctx.create_tuple(vec![a, b]);
        assert!(tup.index() > b.index());
    }

    #[test]
    fn test_type_ctx_create_ref() {
        let mut ctx = TypeCtx::default();
        let inner = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let ref_type = ctx.create_ref(inner);
        assert!(ref_type.index() > inner.index());
    }

    #[test]
    fn test_type_ctx_create_ptr() {
        let mut ctx = TypeCtx::default();
        let inner = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let ptr = ctx.create_ptr(inner);
        assert!(ptr.index() > inner.index());
    }

    #[test]
    fn test_type_ctx_create_proc() {
        let mut ctx = TypeCtx::default();
        let param = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let ret = ctx.get_or_create_primitive(PrimitiveKind::Float);
        let proc_type = ctx.create_proc(vec![param], Some(ret), CallingConvention::Nimcall);
        assert!(proc_type.index() > ret.index());
    }

    #[test]
    fn test_type_ctx_create_typedesc() {
        let mut ctx = TypeCtx::default();
        let inner = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let td = ctx.create_typedesc(Some(inner));
        assert!(td.index() > inner.index());
    }

    #[test]
    fn test_type_ctx_create_static() {
        let mut ctx = TypeCtx::default();
        let inner = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let st = ctx.create_static(inner);
        assert!(st.index() > inner.index());
    }

    #[test]
    fn test_type_ctx_create_varargs() {
        let mut ctx = TypeCtx::default();
        let elem = ctx.get_or_create_primitive(PrimitiveKind::Int);
        let va = ctx.create_varargs(elem);
        assert!(va.index() > elem.index());
    }

    #[test]
    fn test_subrange_type() {
        let sr = SubrangeType {
            base: TyId::new(1),
            lower: 0,
            upper: 100,
        };
        assert_eq!(sr.lower, 0);
        assert_eq!(sr.upper, 100);
    }

    #[test]
    fn test_enum_type() {
        let e = EnumType {
            name: "Days".into(),
            values: vec![
                EnumValue {
                    name: "Monday".into(),
                    ordinal: 0,
                    span: Span::new(FileId::new(0), 0, 6),
                },
                EnumValue {
                    name: "Tuesday".into(),
                    ordinal: 1,
                    span: Span::new(FileId::new(0), 7, 14),
                },
            ],
        };
        assert_eq!(e.name.as_ref(), "Days");
        assert_eq!(e.values.len(), 2);
        assert_eq!(e.values[0].ordinal, 0);
        assert_eq!(e.values[1].ordinal, 1);
    }

    #[test]
    fn test_enum_value() {
        let ev = EnumValue {
            name: "Red".into(),
            ordinal: 5,
            span: Span::new(FileId::new(0), 0, 3),
        };
        assert_eq!(ev.name.as_ref(), "Red");
        assert_eq!(ev.ordinal, 5);
    }

    #[test]
    fn test_primitive_kind_variants() {
        assert!(matches!(PrimitiveKind::Bool, PrimitiveKind::Bool));
        assert!(matches!(PrimitiveKind::Char, PrimitiveKind::Char));
        assert!(matches!(PrimitiveKind::Int, PrimitiveKind::Int));
        assert!(matches!(PrimitiveKind::Int8, PrimitiveKind::Int8));
        assert!(matches!(PrimitiveKind::Int16, PrimitiveKind::Int16));
        assert!(matches!(PrimitiveKind::Int32, PrimitiveKind::Int32));
        assert!(matches!(PrimitiveKind::Int64, PrimitiveKind::Int64));
        assert!(matches!(PrimitiveKind::Uint, PrimitiveKind::Uint));
        assert!(matches!(PrimitiveKind::Uint8, PrimitiveKind::Uint8));
        assert!(matches!(PrimitiveKind::Uint16, PrimitiveKind::Uint16));
        assert!(matches!(PrimitiveKind::Uint32, PrimitiveKind::Uint32));
        assert!(matches!(PrimitiveKind::Uint64, PrimitiveKind::Uint64));
        assert!(matches!(PrimitiveKind::Float, PrimitiveKind::Float));
        assert!(matches!(PrimitiveKind::Float32, PrimitiveKind::Float32));
        assert!(matches!(PrimitiveKind::Float64, PrimitiveKind::Float64));
    }

    #[test]
    fn test_type_clone() {
        let t = Type::Primitive(PrimitiveType::new_int(4, true));
        let cloned = t.clone();
        assert_eq!(t, cloned);
    }

    #[test]
    fn test_type_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let t = Type::Primitive(PrimitiveType::new_int(4, true));
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        t.hash(&mut h1);
        t.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }
}

mod concepts;
mod conversions;
mod inference;
mod inheritance;
mod overload;

// Re-export inference types
pub use inference::{
    ExpectedType, InferError, InferSolver, InferState, InferenceType, LiteralInfer, LiteralKind,
    TypeConstraint, TypeVar,
};

// Re-export conversion types
pub use conversions::{
    find_best_conversion, ConversionGraph, ConversionRank, ConversionResult, Converter,
};

// Re-export overload types
pub use overload::{Candidate, CandidateMatch, MatchScore, OverloadResolver, ResolutionResult};

// Re-export concept types
pub use concepts::{
    Concept, ConceptCtx, ConceptMatchResult, ConceptMember, ConceptParam, ConceptSolver,
    TypeImplementation,
};

// Re-export inheritance types
pub use inheritance::{
    InheritanceSolver, Method, MethodParam, MethodSig, MethodTable, ObjectHierarchy,
    OverrideChecker, OverrideError,
};
