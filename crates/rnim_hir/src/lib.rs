//! Resolved high-level IR after name lookup, desugaring, hygiene, and early semantic passes.

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{FileId, Span};
use rnim_symbols::{ScopeId, SymbolId};
use rustc_hash::FxHashMap;

/// A resolved HIR module
#[derive(Debug, Clone)]
pub struct HirModule {
    pub file_id: FileId,
    pub items: Vec<HirItem>,
    pub scope: ScopeId,
}

/// Items in a HIR module (procedures, types, variables, etc.)
#[derive(Debug, Clone)]
pub enum HirItem {
    Proc(HirProc),
    Func(HirFunc),
    Type(HirTypeDef),
    Var(HirVar),
    Let(HirLet),
    Const(HirConst),
    Import(HirImport),
    Export(HirExport),
}

/// A procedure definition
#[derive(Debug, Clone)]
pub struct HirProc {
    pub name: SymbolId,
    pub span: Span,
    pub params: Vec<HirParam>,
    pub return_type: Option<HirType>,
    pub body: Vec<HirStmt>,
    pub scope: ScopeId,
}

/// A function definition
#[derive(Debug, Clone)]
pub struct HirFunc {
    pub name: SymbolId,
    pub span: Span,
    pub params: Vec<HirParam>,
    pub return_type: Option<HirType>,
    pub body: Vec<HirStmt>,
    pub scope: ScopeId,
}

/// A parameter definition
#[derive(Debug, Clone)]
pub struct HirParam {
    pub name: SymbolId,
    pub span: Span,
    pub typ: HirType,
    pub is_mutable: bool,
}

/// A type definition
#[derive(Debug, Clone)]
pub struct HirTypeDef {
    pub name: SymbolId,
    pub span: Span,
    pub generics: Vec<HirGenericParam>,
    pub body: HirType,
}

/// A generic parameter
#[derive(Debug, Clone)]
pub struct HirGenericParam {
    pub name: SymbolId,
    pub span: Span,
    pub constraint: Option<HirType>,
}

/// A variable declaration
#[derive(Debug, Clone)]
pub struct HirVar {
    pub name: SymbolId,
    pub span: Span,
    pub typ: HirType,
    pub initializer: Option<HirExpr>,
    pub is_mutable: bool,
}

/// A let-binding
#[derive(Debug, Clone)]
pub struct HirLet {
    pub name: SymbolId,
    pub span: Span,
    pub typ: HirType,
    pub initializer: HirExpr,
}

/// A const declaration
#[derive(Debug, Clone)]
pub struct HirConst {
    pub name: SymbolId,
    pub span: Span,
    pub typ: HirType,
    pub value: HirExpr,
}

/// An import statement
#[derive(Debug, Clone)]
pub struct HirImport {
    pub span: Span,
    pub module: SymbolId,
    pub symbols: Vec<(SymbolId, Option<SymbolId>)>, // (original, alias)
    pub is_explicit: bool,
}

/// An export statement
#[derive(Debug, Clone)]
pub struct HirExport {
    pub span: Span,
    pub symbols: Vec<SymbolId>,
}

/// HIR expressions
#[derive(Debug, Clone)]
pub enum HirExpr {
    // Literals
    Unit {
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Integer {
        value: i64,
        span: Span,
    },
    Float {
        value: f64,
        span: Span,
    },
    String {
        value: Box<str>,
        span: Span,
    },
    Char {
        value: char,
        span: Span,
    },

    // Paths and identifiers
    Path {
        id: SymbolId,
        span: Span,
    },
    Ident {
        id: SymbolId,
        span: Span,
    },

    // Blocks and control
    Block {
        stmts: Vec<HirStmt>,
        expr: Option<Box<HirExpr>>,
        span: Span,
    },
    Return {
        expr: Option<Box<HirExpr>>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    If {
        condition: Box<HirExpr>,
        then_block: Box<HirExpr>,
        else_expr: Option<Box<HirExpr>>,
        span: Span,
    },
    While {
        condition: Box<HirExpr>,
        body: Box<HirExpr>,
        span: Span,
    },
    For {
        variable: SymbolId,
        iterable: Box<HirExpr>,
        body: Box<HirExpr>,
        span: Span,
    },
    Case {
        expr: Box<HirExpr>,
        branches: Vec<CaseBranch>,
        else_expr: Option<Box<HirExpr>>,
        span: Span,
    },

    // Calls and indexing
    Call {
        func: Box<HirExpr>,
        args: Vec<HirExpr>,
        span: Span,
    },
    Command {
        func: Box<HirExpr>,
        args: Vec<HirExpr>,
        span: Span,
    }, // Nim command syntax
    Index {
        base: Box<HirExpr>,
        index: Box<HirExpr>,
        span: Span,
    },

    // Operators
    Binary {
        op: HirBinOp,
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        span: Span,
    },
    Unary {
        op: HirUnOp,
        operand: Box<HirExpr>,
        span: Span,
    },

    // Field access and deref
    Field {
        base: Box<HirExpr>,
        field: SymbolId,
        span: Span,
    },
    TupleIndex {
        base: Box<HirExpr>,
        index: u32,
        span: Span,
    },
    Deref {
        expr: Box<HirExpr>,
        span: Span,
    },
    AddrOf {
        expr: Box<HirExpr>,
        span: Span,
    },

    // Type expressions
    Cast {
        expr: Box<HirExpr>,
        ty: HirType,
        span: Span,
    },
    TypeConv {
        expr: Box<HirExpr>,
        ty: HirType,
        span: Span,
    },

    // Lambdas and procs
    Lambda {
        params: Vec<HirParam>,
        return_type: Option<HirType>,
        body: Box<HirExpr>,
        span: Span,
    },
    Proc {
        params: Vec<HirParam>,
        return_type: Option<HirType>,
        body: Vec<HirStmt>,
        span: Span,
    },

    // Misc
    Range {
        start: Box<HirExpr>,
        end: Box<HirExpr>,
        span: Span,
    },
    Raise {
        expr: Box<HirExpr>,
        span: Span,
    },
    Try {
        body: Vec<HirStmt>,
        branches: Vec<TryBranch>,
        finally_expr: Option<Box<HirExpr>>,
        span: Span,
    },
    Defer {
        body: Box<HirExpr>,
        span: Span,
    },
}

/// A case branch in a case statement
#[derive(Debug, Clone)]
pub struct CaseBranch {
    pub patterns: Vec<HirExpr>,
    pub body: Box<HirExpr>,
}

/// A try branch (exception handler)
#[derive(Debug, Clone)]
pub struct TryBranch {
    pub exception_type: Option<HirType>,
    pub variable: Option<SymbolId>,
    pub body: Vec<HirStmt>,
}

/// Binary operators
#[derive(Debug, Clone, Copy)]
pub enum HirBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shl,
    Shr,
    And,
    Or,
    Xor,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    EqEq,
    DotDot,
}

/// Unary operators
#[derive(Debug, Clone, Copy)]
pub enum HirUnOp {
    Neg,
    Pos,
    Not,
    BitNot,
}

/// HIR statements
#[derive(Debug, Clone)]
pub enum HirStmt {
    Expr {
        expr: HirExpr,
    },
    Assignment {
        lhs: HirExpr,
        rhs: HirExpr,
        span: Span,
    },
    Init {
        name: SymbolId,
        typ: HirType,
        rhs: HirExpr,
        span: Span,
    },
    Mutate {
        name: SymbolId,
        op: HirBinOp,
        rhs: HirExpr,
        span: Span,
    },
    Discard {
        expr: HirExpr,
        span: Span,
    },
    Return {
        expr: Option<HirExpr>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Raise {
        expr: HirExpr,
        span: Span,
    },
    Defer {
        expr: HirExpr,
        span: Span,
    },
    Block {
        stmts: Vec<HirStmt>,
        span: Span,
    },
}

/// HIR types
#[derive(Debug, Clone)]
pub enum HirType {
    // Primitive types
    Unit,
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

    // Compound types
    Array {
        element: Box<HirType>,
        size: Option<Box<HirExpr>>,
    },
    Seq {
        element: Box<HirType>,
    },
    Set {
        element: Box<HirType>,
    },
    Tuple {
        fields: Vec<(Option<Box<str>>, HirType)>,
    },
    Object {
        fields: Vec<(SymbolId, HirType)>,
        base: Option<Box<HirType>>,
    },

    // Pointer types
    Ref {
        inner: Box<HirType>,
    },
    Ptr {
        inner: Box<HirType>,
    },
    MutRef {
        inner: Box<HirType>,
    },

    // Procedure types
    Proc {
        params: Vec<HirType>,
        return_type: Option<Box<HirType>>,
        raises: bool,
    },

    // Type constructors
    Distinct {
        inner: Box<HirType>,
    },
    Alias {
        name: SymbolId,
    },
    Generic {
        name: SymbolId,
        args: Vec<HirType>,
    },

    // Special
    TypeDesc,
    Void,
    Auto,
    Any,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hir_expr_integer() {
        let expr = HirExpr::Integer {
            value: 42,
            span: Span::new(FileId::new(0), 0, 2),
        };
        assert!(matches!(expr, HirExpr::Integer { value: 42, .. }));
    }

    #[test]
    fn test_hir_type_primitive() {
        let t = HirType::Int;
        assert!(matches!(t, HirType::Int));
    }

    #[test]
    fn test_hir_type_ref() {
        let t = HirType::Ref {
            inner: Box::new(HirType::Int),
        };
        assert!(matches!(t, HirType::Ref { .. }));
    }

    #[test]
    fn test_hir_binary_op() {
        let op = HirBinOp::Add;
        assert!(matches!(op, HirBinOp::Add));
    }

    #[test]
    fn test_hir_unary_op() {
        let op = HirUnOp::Not;
        assert!(matches!(op, HirUnOp::Not));
    }

    #[test]
    fn test_hir_module_new() {
        let module = HirModule {
            file_id: FileId::new(0),
            items: vec![],
            scope: ScopeId::default(),
        };
        assert_eq!(module.file_id, FileId::new(0));
        assert!(module.items.is_empty());
    }

    #[test]
    fn test_hir_proc_struct() {
        let proc = HirProc {
            name: SymbolId::default(),
            span: Span::new(FileId::new(0), 0, 10),
            params: vec![],
            return_type: Some(HirType::Int),
            body: vec![],
            scope: ScopeId::default(),
        };
        assert!(proc.params.is_empty());
        assert!(proc.return_type.is_some());
        assert!(proc.body.is_empty());
    }

    #[test]
    fn test_hir_func_struct() {
        let func = HirFunc {
            name: SymbolId::default(),
            span: Span::new(FileId::new(0), 0, 10),
            params: vec![],
            return_type: Some(HirType::Int),
            body: vec![],
            scope: ScopeId::default(),
        };
        assert!(func.params.is_empty());
        assert!(func.return_type.is_some());
    }

    #[test]
    fn test_hir_param() {
        let param = HirParam {
            name: SymbolId::default(),
            span: Span::new(FileId::new(0), 0, 5),
            typ: HirType::Int,
            is_mutable: true,
        };
        assert!(param.is_mutable);
        assert!(matches!(param.typ, HirType::Int));
    }

    #[test]
    fn test_hir_var() {
        let var = HirVar {
            name: SymbolId::default(),
            span: Span::new(FileId::new(0), 0, 10),
            typ: HirType::Int,
            initializer: Some(HirExpr::Integer {
                value: 42,
                span: Span::new(FileId::new(0), 5, 7),
            }),
            is_mutable: true,
        };
        assert!(var.is_mutable);
        assert!(var.initializer.is_some());
    }

    #[test]
    fn test_hir_let() {
        let let_binding = HirLet {
            name: SymbolId::default(),
            span: Span::new(FileId::new(0), 0, 10),
            typ: HirType::Int,
            initializer: HirExpr::Integer {
                value: 42,
                span: Span::new(FileId::new(0), 5, 7),
            },
        };
        assert!(matches!(let_binding.typ, HirType::Int));
    }

    #[test]
    fn test_hir_const() {
        let constant = HirConst {
            name: SymbolId::default(),
            span: Span::new(FileId::new(0), 0, 10),
            typ: HirType::Int,
            value: HirExpr::Integer {
                value: 100,
                span: Span::new(FileId::new(0), 5, 8),
            },
        };
        assert!(matches!(constant.typ, HirType::Int));
    }

    #[test]
    fn test_hir_type_def() {
        let type_def = HirTypeDef {
            name: SymbolId::default(),
            span: Span::new(FileId::new(0), 0, 20),
            generics: vec![],
            body: HirType::Int,
        };
        assert!(type_def.generics.is_empty());
        assert!(matches!(type_def.body, HirType::Int));
    }

    #[test]
    fn test_hir_generic_param() {
        let gen_param = HirGenericParam {
            name: SymbolId::default(),
            span: Span::new(FileId::new(0), 0, 5),
            constraint: Some(HirType::Int),
        };
        assert!(gen_param.constraint.is_some());
    }

    #[test]
    fn test_hir_import_stmt() {
        let import = HirImport {
            span: Span::new(FileId::new(0), 0, 10),
            module: SymbolId::default(),
            symbols: vec![],
            is_explicit: true,
        };
        assert!(import.is_explicit);
    }

    #[test]
    fn test_hir_export_stmt() {
        let export = HirExport {
            span: Span::new(FileId::new(0), 0, 10),
            symbols: vec![],
        };
        assert!(export.symbols.is_empty());
    }

    #[test]
    fn test_hir_expr_bool() {
        let expr = HirExpr::Bool {
            value: true,
            span: Span::new(FileId::new(0), 0, 4),
        };
        assert!(matches!(expr, HirExpr::Bool { value: true, .. }));
    }

    #[test]
    fn test_hir_expr_string() {
        let expr = HirExpr::String {
            value: "hello".into(),
            span: Span::new(FileId::new(0), 0, 5),
        };
        assert!(matches!(expr, HirExpr::String { .. }));
    }

    #[test]
    fn test_hir_expr_binary() {
        let expr = HirExpr::Binary {
            op: HirBinOp::Add,
            left: Box::new(HirExpr::Integer {
                value: 1,
                span: Span::new(FileId::new(0), 0, 1),
            }),
            right: Box::new(HirExpr::Integer {
                value: 2,
                span: Span::new(FileId::new(0), 2, 3),
            }),
            span: Span::new(FileId::new(0), 0, 3),
        };
        assert!(matches!(
            expr,
            HirExpr::Binary {
                op: HirBinOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn test_hir_expr_unary() {
        let expr = HirExpr::Unary {
            op: HirUnOp::Neg,
            operand: Box::new(HirExpr::Integer {
                value: 5,
                span: Span::new(FileId::new(0), 1, 2),
            }),
            span: Span::new(FileId::new(0), 0, 2),
        };
        assert!(matches!(
            expr,
            HirExpr::Unary {
                op: HirUnOp::Neg,
                ..
            }
        ));
    }

    #[test]
    fn test_hir_expr_call() {
        let expr = HirExpr::Call {
            func: Box::new(HirExpr::Ident {
                id: SymbolId::default(),
                span: Span::new(FileId::new(0), 0, 4),
            }),
            args: vec![],
            span: Span::new(FileId::new(0), 0, 5),
        };
        assert!(matches!(expr, HirExpr::Call { .. }));
    }

    #[test]
    fn test_hir_expr_if() {
        let expr = HirExpr::If {
            condition: Box::new(HirExpr::Bool {
                value: true,
                span: Span::new(FileId::new(0), 3, 7),
            }),
            then_block: Box::new(HirExpr::Integer {
                value: 1,
                span: Span::new(FileId::new(0), 9, 10),
            }),
            else_expr: None,
            span: Span::new(FileId::new(0), 0, 10),
        };
        assert!(matches!(expr, HirExpr::If { .. }));
    }

    #[test]
    fn test_hir_expr_block() {
        let expr = HirExpr::Block {
            stmts: vec![],
            expr: None,
            span: Span::new(FileId::new(0), 0, 2),
        };
        assert!(matches!(expr, HirExpr::Block { .. }));
    }

    #[test]
    fn test_hir_expr_return() {
        let expr = HirExpr::Return {
            expr: Some(Box::new(HirExpr::Integer {
                value: 42,
                span: Span::new(FileId::new(0), 7, 9),
            })),
            span: Span::new(FileId::new(0), 0, 9),
        };
        assert!(matches!(expr, HirExpr::Return { .. }));
    }

    #[test]
    fn test_hir_expr_break() {
        let expr = HirExpr::Break {
            span: Span::new(FileId::new(0), 0, 5),
        };
        assert!(matches!(expr, HirExpr::Break { .. }));
    }

    #[test]
    fn test_hir_expr_continue() {
        let expr = HirExpr::Continue {
            span: Span::new(FileId::new(0), 0, 8),
        };
        assert!(matches!(expr, HirExpr::Continue { .. }));
    }

    #[test]
    fn test_hir_expr_while() {
        let expr = HirExpr::While {
            condition: Box::new(HirExpr::Bool {
                value: true,
                span: Span::new(FileId::new(0), 6, 10),
            }),
            body: Box::new(HirExpr::Unit {
                span: Span::new(FileId::new(0), 12, 12),
            }),
            span: Span::new(FileId::new(0), 0, 12),
        };
        assert!(matches!(expr, HirExpr::While { .. }));
    }

    #[test]
    fn test_hir_expr_for() {
        let expr = HirExpr::For {
            variable: SymbolId::default(),
            iterable: Box::new(HirExpr::Range {
                start: Box::new(HirExpr::Integer {
                    value: 0,
                    span: Span::new(FileId::new(0), 4, 5),
                }),
                end: Box::new(HirExpr::Integer {
                    value: 10,
                    span: Span::new(FileId::new(0), 6, 8),
                }),
                span: Span::new(FileId::new(0), 4, 8),
            }),
            body: Box::new(HirExpr::Unit {
                span: Span::new(FileId::new(0), 11, 11),
            }),
            span: Span::new(FileId::new(0), 0, 11),
        };
        assert!(matches!(expr, HirExpr::For { .. }));
    }

    #[test]
    fn test_hir_expr_case() {
        let expr = HirExpr::Case {
            expr: Box::new(HirExpr::Integer {
                value: 1,
                span: Span::new(FileId::new(0), 5, 6),
            }),
            branches: vec![],
            else_expr: None,
            span: Span::new(FileId::new(0), 0, 6),
        };
        assert!(matches!(expr, HirExpr::Case { .. }));
    }

    #[test]
    fn test_hir_expr_raise() {
        let expr = HirExpr::Raise {
            expr: Box::new(HirExpr::String {
                value: "error".into(),
                span: Span::new(FileId::new(0), 6, 11),
            }),
            span: Span::new(FileId::new(0), 0, 11),
        };
        assert!(matches!(expr, HirExpr::Raise { .. }));
    }

    #[test]
    fn test_hir_expr_try() {
        let expr = HirExpr::Try {
            body: vec![],
            branches: vec![],
            finally_expr: None,
            span: Span::new(FileId::new(0), 0, 3),
        };
        assert!(matches!(expr, HirExpr::Try { .. }));
    }

    #[test]
    fn test_hir_expr_defer() {
        let expr = HirExpr::Defer {
            body: Box::new(HirExpr::Unit {
                span: Span::new(FileId::new(0), 6, 6),
            }),
            span: Span::new(FileId::new(0), 0, 6),
        };
        assert!(matches!(expr, HirExpr::Defer { .. }));
    }

    #[test]
    fn test_hir_expr_field_access() {
        let expr = HirExpr::Field {
            base: Box::new(HirExpr::Ident {
                id: SymbolId::default(),
                span: Span::new(FileId::new(0), 0, 3),
            }),
            field: SymbolId::default(),
            span: Span::new(FileId::new(0), 0, 6),
        };
        assert!(matches!(expr, HirExpr::Field { .. }));
    }

    #[test]
    fn test_hir_expr_deref() {
        let expr = HirExpr::Deref {
            expr: Box::new(HirExpr::Ident {
                id: SymbolId::default(),
                span: Span::new(FileId::new(0), 1, 2),
            }),
            span: Span::new(FileId::new(0), 0, 2),
        };
        assert!(matches!(expr, HirExpr::Deref { .. }));
    }

    #[test]
    fn test_hir_expr_address_of() {
        let expr = HirExpr::AddrOf {
            expr: Box::new(HirExpr::Ident {
                id: SymbolId::default(),
                span: Span::new(FileId::new(0), 1, 2),
            }),
            span: Span::new(FileId::new(0), 0, 2),
        };
        assert!(matches!(expr, HirExpr::AddrOf { .. }));
    }

    #[test]
    fn test_hir_expr_cast() {
        let expr = HirExpr::Cast {
            expr: Box::new(HirExpr::Integer {
                value: 42,
                span: Span::new(FileId::new(0), 0, 2),
            }),
            ty: HirType::Float,
            span: Span::new(FileId::new(0), 0, 8),
        };
        assert!(matches!(
            expr,
            HirExpr::Cast {
                ty: HirType::Float,
                ..
            }
        ));
    }

    #[test]
    fn test_hir_expr_lambda() {
        let expr = HirExpr::Lambda {
            params: vec![],
            return_type: None,
            body: Box::new(HirExpr::Unit {
                span: Span::new(FileId::new(0), 10, 10),
            }),
            span: Span::new(FileId::new(0), 0, 10),
        };
        assert!(matches!(expr, HirExpr::Lambda { .. }));
    }

    #[test]
    fn test_hir_expr_range() {
        let expr = HirExpr::Range {
            start: Box::new(HirExpr::Integer {
                value: 0,
                span: Span::new(FileId::new(0), 0, 1),
            }),
            end: Box::new(HirExpr::Integer {
                value: 10,
                span: Span::new(FileId::new(0), 2, 4),
            }),
            span: Span::new(FileId::new(0), 0, 4),
        };
        assert!(matches!(expr, HirExpr::Range { .. }));
    }

    #[test]
    fn test_hir_expr_index() {
        let expr = HirExpr::Index {
            base: Box::new(HirExpr::Ident {
                id: SymbolId::default(),
                span: Span::new(FileId::new(0), 0, 3),
            }),
            index: Box::new(HirExpr::Integer {
                value: 0,
                span: Span::new(FileId::new(0), 4, 5),
            }),
            span: Span::new(FileId::new(0), 0, 5),
        };
        assert!(matches!(expr, HirExpr::Index { .. }));
    }

    #[test]
    fn test_hir_stmt_expr() {
        let stmt = HirStmt::Expr {
            expr: HirExpr::Unit {
                span: Span::new(FileId::new(0), 0, 0),
            },
        };
        assert!(matches!(stmt, HirStmt::Expr { .. }));
    }

    #[test]
    fn test_hir_stmt_assignment() {
        let stmt = HirStmt::Assignment {
            lhs: HirExpr::Ident {
                id: SymbolId::default(),
                span: Span::new(FileId::new(0), 0, 3),
            },
            rhs: HirExpr::Integer {
                value: 42,
                span: Span::new(FileId::new(0), 6, 8),
            },
            span: Span::new(FileId::new(0), 0, 8),
        };
        assert!(matches!(stmt, HirStmt::Assignment { .. }));
    }

    #[test]
    fn test_hir_stmt_return() {
        let stmt = HirStmt::Return {
            expr: Some(HirExpr::Integer {
                value: 42,
                span: Span::new(FileId::new(0), 7, 9),
            }),
            span: Span::new(FileId::new(0), 0, 9),
        };
        assert!(matches!(stmt, HirStmt::Return { .. }));
    }

    #[test]
    fn test_hir_stmt_break() {
        let stmt = HirStmt::Break {
            span: Span::new(FileId::new(0), 0, 5),
        };
        assert!(matches!(stmt, HirStmt::Break { .. }));
    }

    #[test]
    fn test_hir_stmt_continue() {
        let stmt = HirStmt::Continue {
            span: Span::new(FileId::new(0), 0, 8),
        };
        assert!(matches!(stmt, HirStmt::Continue { .. }));
    }

    #[test]
    fn test_hir_stmt_raise() {
        let stmt = HirStmt::Raise {
            expr: HirExpr::String {
                value: "error".into(),
                span: Span::new(FileId::new(0), 6, 11),
            },
            span: Span::new(FileId::new(0), 0, 11),
        };
        assert!(matches!(stmt, HirStmt::Raise { .. }));
    }

    #[test]
    fn test_hir_stmt_discard() {
        let stmt = HirStmt::Discard {
            expr: HirExpr::Unit {
                span: Span::new(FileId::new(0), 8, 8),
            },
            span: Span::new(FileId::new(0), 0, 8),
        };
        assert!(matches!(stmt, HirStmt::Discard { .. }));
    }

    #[test]
    fn test_hir_stmt_block() {
        let stmt = HirStmt::Block {
            stmts: vec![],
            span: Span::new(FileId::new(0), 0, 2),
        };
        assert!(matches!(stmt, HirStmt::Block { .. }));
    }

    #[test]
    fn test_hir_type_compound_ref() {
        let t = HirType::Ref {
            inner: Box::new(HirType::Int),
        };
        assert!(matches!(t, HirType::Ref { .. }));
    }

    #[test]
    fn test_hir_type_compound_array() {
        let t = HirType::Array {
            element: Box::new(HirType::Int),
            size: None,
        };
        assert!(matches!(t, HirType::Array { .. }));
    }

    #[test]
    fn test_hir_type_compound_seq() {
        let t = HirType::Seq {
            element: Box::new(HirType::Int),
        };
        assert!(matches!(t, HirType::Seq { .. }));
    }

    #[test]
    fn test_hir_type_compound_tuple() {
        let t = HirType::Tuple {
            fields: vec![(None, HirType::Int), (None, HirType::Char)],
        };
        assert!(matches!(t, HirType::Tuple { .. }));
    }

    #[test]
    fn test_hir_type_compound_object() {
        let t = HirType::Object {
            fields: vec![],
            base: None,
        };
        assert!(matches!(t, HirType::Object { .. }));
    }

    #[test]
    fn test_hir_type_proc() {
        let t = HirType::Proc {
            params: vec![],
            return_type: Some(Box::new(HirType::Unit)),
            raises: false,
        };
        assert!(matches!(t, HirType::Proc { .. }));
    }

    #[test]
    fn test_hir_binop_all_variants() {
        let ops = [
            HirBinOp::Add,
            HirBinOp::Sub,
            HirBinOp::Mul,
            HirBinOp::Div,
            HirBinOp::Mod,
            HirBinOp::Shl,
            HirBinOp::Shr,
            HirBinOp::And,
            HirBinOp::Or,
            HirBinOp::Xor,
            HirBinOp::Eq,
            HirBinOp::Ne,
            HirBinOp::Lt,
            HirBinOp::Le,
            HirBinOp::Gt,
            HirBinOp::Ge,
            HirBinOp::EqEq,
            HirBinOp::DotDot,
        ];
        for op in ops {
            assert!(matches!(op, _));
        }
    }

    #[test]
    fn test_hir_unop_all_variants() {
        assert!(matches!(HirUnOp::Neg, HirUnOp::Neg));
        assert!(matches!(HirUnOp::Pos, HirUnOp::Pos));
        assert!(matches!(HirUnOp::Not, HirUnOp::Not));
        assert!(matches!(HirUnOp::BitNot, HirUnOp::BitNot));
    }

    #[test]
    fn test_hir_item_variants() {
        let items = vec![
            HirItem::Proc(HirProc {
                name: SymbolId::default(),
                span: Span::new(FileId::new(0), 0, 10),
                params: vec![],
                return_type: None,
                body: vec![],
                scope: ScopeId::default(),
            }),
            HirItem::Type(HirTypeDef {
                name: SymbolId::default(),
                span: Span::new(FileId::new(0), 0, 10),
                generics: vec![],
                body: HirType::Int,
            }),
            HirItem::Var(HirVar {
                name: SymbolId::default(),
                span: Span::new(FileId::new(0), 0, 10),
                typ: HirType::Int,
                initializer: None,
                is_mutable: true,
            }),
            HirItem::Let(HirLet {
                name: SymbolId::default(),
                span: Span::new(FileId::new(0), 0, 10),
                typ: HirType::Int,
                initializer: HirExpr::Integer {
                    value: 42,
                    span: Span::new(FileId::new(0), 5, 7),
                },
            }),
            HirItem::Const(HirConst {
                name: SymbolId::default(),
                span: Span::new(FileId::new(0), 0, 10),
                typ: HirType::Int,
                value: HirExpr::Integer {
                    value: 42,
                    span: Span::new(FileId::new(0), 5, 7),
                },
            }),
        ];
        assert_eq!(items.len(), 5);
    }
}
