//! Tests for type system

use rnim_allocator as _;
use rnim_span::{FileId, Span};
use rnim_types::{
    default_primitive_value, is_float, is_integral, is_ordinal, primitive_compatible,
    CallingConvention, EnumValue, Field, LiteralValue, OrdinalKind, PrimitiveKind, PrimitiveType,
    TyId, Type, TypeCtx,
};

fn make_span() -> Span {
    Span::new(FileId(0), 0, 0)
}

#[test]
fn test_type_ctx_creation() {
    let ctx = TypeCtx::new();
    // Should have initialized primitives
    assert!(ctx.get_primitive(PrimitiveKind::Bool).is_some());
    assert!(ctx.get_primitive(PrimitiveKind::Char).is_some());
}

#[test]
fn test_type_ctx_intern_primitive() {
    let ctx = TypeCtx::new();
    let int_type_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let int_type = ctx.get(int_type_id);
    assert!(int_type.is_some());
}

#[test]
fn test_primitive_type_new_int() {
    let prim = PrimitiveType::new_int(4, true);
    assert_eq!(prim.kind, PrimitiveKind::Int);
    assert!(prim.is_signed);
    assert_eq!(prim.size, 4);
    assert!(prim.min_value.is_some());
    assert!(prim.max_value.is_some());
}

#[test]
fn test_primitive_type_new_float() {
    let prim = PrimitiveType::new_float(8);
    assert!(matches!(prim.kind, PrimitiveKind::Float64));
    assert!(prim.is_signed);
    assert_eq!(prim.size, 8);
}

#[test]
fn test_primitive_type_ordinal_kind() {
    let int_prim = PrimitiveType::new_int(4, true);
    assert_eq!(int_prim.ordinal_kind(), OrdinalKind::Integer);

    let char_prim = PrimitiveType {
        kind: PrimitiveKind::Char,
        size: 4,
        is_signed: false,
        min_value: Some(0),
        max_value: Some(0x10FFFF),
    };
    assert_eq!(char_prim.ordinal_kind(), OrdinalKind::Char);
}

#[test]
fn test_is_ordinal() {
    let int_type = Type::Primitive(PrimitiveType::new_int(4, true));
    assert!(is_ordinal(&int_type));

    let char_type = Type::Primitive(PrimitiveType {
        kind: PrimitiveKind::Char,
        size: 4,
        is_signed: false,
        min_value: Some(0),
        max_value: Some(0x10FFFF),
    });
    assert!(is_ordinal(&char_type));

    let float_type = Type::Primitive(PrimitiveType::new_float(8));
    assert!(!is_ordinal(&float_type));

    let seq_type = Type::Seq { elem: TyId::new(0) };
    assert!(!is_ordinal(&seq_type));
}

#[test]
fn test_is_integral() {
    let int_type = Type::Primitive(PrimitiveType::new_int(4, true));
    assert!(is_integral(&int_type));

    let bool_type = Type::Primitive(PrimitiveType {
        kind: PrimitiveKind::Bool,
        size: 1,
        is_signed: false,
        min_value: Some(0),
        max_value: Some(1),
    });
    assert!(is_integral(&bool_type));

    let float_type = Type::Primitive(PrimitiveType::new_float(8));
    assert!(!is_integral(&float_type));
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
    assert!(primitive_compatible(&a, &b));

    let c = PrimitiveType::new_int(8, true);
    assert!(primitive_compatible(&a, &c)); // same signedness, different size

    let d = PrimitiveType::new_int(4, false);
    assert!(!primitive_compatible(&a, &d)); // different signedness
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
fn test_create_enum() {
    let mut ctx = TypeCtx::new();
    let values = vec![
        EnumValue {
            name: "A".into(),
            ordinal: 0,
            span: make_span(),
        },
        EnumValue {
            name: "B".into(),
            ordinal: 1,
            span: make_span(),
        },
        EnumValue {
            name: "C".into(),
            ordinal: 2,
            span: make_span(),
        },
    ];
    let enum_id = ctx.create_enum("Color".into(), values);

    let enum_type = ctx.get(enum_id);
    assert!(enum_type.is_some());
    if let Type::Enum(e) = enum_type.unwrap() {
        assert_eq!(e.name.as_ref(), "Color");
        assert_eq!(e.values.len(), 3);
    } else {
        panic!("Expected Enum type");
    }
}

#[test]
fn test_create_subrange() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let subrange_id = ctx.create_subrange(int_id, 0, 100);

    let subrange_type = ctx.get(subrange_id);
    assert!(subrange_type.is_some());
    if let Type::Subrange(s) = subrange_type.unwrap() {
        assert_eq!(s.lower, 0);
        assert_eq!(s.upper, 100);
    } else {
        panic!("Expected Subrange type");
    }
}

#[test]
fn test_create_distinct() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let distinct_id = ctx.create_distinct("UserId".into(), int_id);

    let distinct_type = ctx.get(distinct_id);
    assert!(distinct_type.is_some());
    if let Type::Distinct { name, underlying } = distinct_type.unwrap() {
        assert_eq!(name.as_ref(), "UserId");
        assert_eq!(*underlying, int_id);
    } else {
        panic!("Expected Distinct type");
    }
}

#[test]
fn test_create_alias() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let alias_id = ctx.create_alias("MyInt".into(), int_id);

    let alias_type = ctx.get(alias_id);
    assert!(alias_type.is_some());
    if let Type::Alias { name, underlying } = alias_type.unwrap() {
        assert_eq!(name.as_ref(), "MyInt");
        assert_eq!(*underlying, int_id);
    } else {
        panic!("Expected Alias type");
    }
}

#[test]
fn test_create_object() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let fields = vec![
        Field {
            name: "x".into(),
            typ: int_id,
            offset: Some(0),
        },
        Field {
            name: "y".into(),
            typ: int_id,
            offset: Some(8),
        },
    ];
    let obj_id = ctx.create_object(fields, None);

    let obj_type = ctx.get(obj_id);
    assert!(obj_type.is_some());
    if let Type::Object { fields: f, base } = obj_type.unwrap() {
        assert_eq!(f.len(), 2);
        assert!(base.is_none());
    } else {
        panic!("Expected Object type");
    }
}

#[test]
fn test_create_array() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let arr_id = ctx.create_array(int_id, Some(10));

    let arr_type = ctx.get(arr_id);
    assert!(arr_type.is_some());
    if let Type::Array { elem, len } = arr_type.unwrap() {
        assert_eq!(*elem, int_id);
        assert_eq!(len.unwrap(), 10);
    } else {
        panic!("Expected Array type");
    }
}

#[test]
fn test_create_seq() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let seq_id = ctx.create_seq(int_id);

    let seq_type = ctx.get(seq_id);
    assert!(seq_type.is_some());
    if let Type::Seq { elem } = seq_type.unwrap() {
        assert_eq!(*elem, int_id);
    } else {
        panic!("Expected Seq type");
    }
}

#[test]
fn test_create_set() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let set_id = ctx.create_set(int_id);

    let set_type = ctx.get(set_id);
    assert!(set_type.is_some());
    if let Type::Set { elem } = set_type.unwrap() {
        assert_eq!(*elem, int_id);
    } else {
        panic!("Expected Set type");
    }
}

#[test]
fn test_create_tuple() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let float_id = ctx.get_primitive(PrimitiveKind::Float64).unwrap();
    let tuple_id = ctx.create_tuple(vec![int_id, float_id]);

    let tuple_type = ctx.get(tuple_id);
    assert!(tuple_type.is_some());
    if let Type::Tuple { fields } = tuple_type.unwrap() {
        assert_eq!(fields.len(), 2);
    } else {
        panic!("Expected Tuple type");
    }
}

#[test]
fn test_create_ref() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let ref_id = ctx.create_ref(int_id);

    let ref_type = ctx.get(ref_id);
    assert!(ref_type.is_some());
    if let Type::Ref { inner } = ref_type.unwrap() {
        assert_eq!(*inner, int_id);
    } else {
        panic!("Expected Ref type");
    }
}

#[test]
fn test_create_ptr() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let ptr_id = ctx.create_ptr(int_id);

    let ptr_type = ctx.get(ptr_id);
    assert!(ptr_type.is_some());
    if let Type::Ptr { inner } = ptr_type.unwrap() {
        assert_eq!(*inner, int_id);
    } else {
        panic!("Expected Ptr type");
    }
}

#[test]
fn test_create_proc() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let proc_id = ctx.create_proc(
        vec![int_id, int_id],
        Some(int_id),
        CallingConvention::Nimcall,
    );

    let proc_type = ctx.get(proc_id);
    assert!(proc_type.is_some());
    if let Type::Proc {
        params,
        ret,
        calling_convention,
    } = proc_type.unwrap()
    {
        assert_eq!(params.len(), 2);
        assert!(ret.is_some());
        assert_eq!(*calling_convention, CallingConvention::Nimcall);
    } else {
        panic!("Expected Proc type");
    }
}

#[test]
fn test_create_typedesc() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let td_id = ctx.create_typedesc(Some(int_id));

    let td_type = ctx.get(td_id);
    assert!(td_type.is_some());
    if let Type::TypeDesc { inner } = td_type.unwrap() {
        assert!(inner.is_some());
    } else {
        panic!("Expected TypeDesc type");
    }
}

#[test]
fn test_create_static() {
    let mut ctx = TypeCtx::new();
    let int_id = ctx.get_primitive(PrimitiveKind::Int).unwrap();
    let static_id = ctx.create_static(int_id);

    let static_type = ctx.get(static_id);
    assert!(static_type.is_some());
    if let Type::Static { inner } = static_type.unwrap() {
        assert_eq!(*inner, int_id);
    } else {
        panic!("Expected Static type");
    }
}

#[test]
fn test_calling_convention_default() {
    let cc = CallingConvention::default();
    assert_eq!(cc, CallingConvention::Nimcall);
}

#[test]
fn test_ty_id_default() {
    let id = TyId::default();
    assert_eq!(id.index(), u32::MAX);
}

#[test]
fn test_ty_id_new() {
    let id = TyId::new(42);
    assert_eq!(id.index(), 42);
}
