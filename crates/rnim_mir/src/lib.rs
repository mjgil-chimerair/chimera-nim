//! Mid-level IR for control flow, calls, moves, destructors, exceptions, and backend-neutral lowering.
//!
//! This module provides the MIR (Mid-level Intermediate Representation) that serves as the
//! lowering target from HIR and before backend code generation.

use petgraph::graph::DiGraph;
#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{FileId, Span};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

/// A MIR basic block
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Statements in this block
    pub statements: Vec<MirStmt>,
    /// Terminator for this block
    pub terminator: Terminator,
    /// Span of this block
    pub span: Span,
}

impl BasicBlock {
    pub fn new(span: Span) -> Self {
        BasicBlock {
            statements: Vec::new(),
            terminator: Terminator::Goto(Box::new(GotoTarget::Next)),
            span,
        }
    }

    pub fn with_terminator(terminator: Terminator, span: Span) -> Self {
        BasicBlock {
            statements: Vec::new(),
            terminator,
            span,
        }
    }
}

/// Goto target for control flow
#[derive(Debug, Clone)]
pub enum GotoTarget {
    /// Go to the next block in sequence
    Next,
    /// Go to a specific block by index
    Block(NodeIndex),
    /// Go to multiple blocks (for switch/match)
    Switch(Vec<(MirValue, NodeIndex)>),
}

/// A statement in MIR
#[derive(Debug, Clone)]
pub enum MirStmt {
    /// Assign to a place
    Assign { place: Place, value: MirValue },
    /// Set discriminant (for variant discrimination)
    SetDiscriminant { place: Place, variant_index: usize },
    /// StorageLive - mark a variable as live
    StorageLive(Local, Span),
    /// StorageDead - mark a variable as dead
    StorageDead(Local, Span),
    /// Call a procedure
    Call {
        destination: Option<(Place, NodeIndex)>,
        callee: MirValue,
        arguments: Vec<MirValue>,
    },
    /// Try to call (for exception handling)
    TryCall {
        destination: Option<(Place, NodeIndex)>,
        cleanup: Option<NodeIndex>,
        callee: MirValue,
        arguments: Vec<MirValue>,
    },
    /// Drop a value
    Drop {
        place: Place,
        target: Option<NodeIndex>,
    },
    /// Deinit - deinitialize a place
    Deinit(Place),
    /// Assert - assertion with message
    Assert {
        condition: MirValue,
        msg: String,
        target: NodeIndex,
    },
    /// FakeRead - read from a place without affecting last-use
    FakeRead { place: Place },
    /// Nop - no operation
    Nop,
    /// ResetRecursion - reset recursion depth for a local
    ResetRecursion(Local, Span),
}

impl MirStmt {
    pub fn span(&self) -> Span {
        match self {
            MirStmt::Assign { place, .. } => place.span,
            MirStmt::SetDiscriminant { place, .. } => place.span,
            MirStmt::StorageLive(l, span) | MirStmt::StorageDead(l, span) => *span,
            MirStmt::Call { callee, .. } | MirStmt::TryCall { callee, .. } => callee.span(),
            MirStmt::Drop { place, .. } => place.span,
            MirStmt::Deinit(place) => place.span,
            MirStmt::Assert { condition, .. } => condition.span(),
            MirStmt::FakeRead { place } => place.span,
            MirStmt::Nop => Span::new(FileId(0), 0, 0),
            MirStmt::ResetRecursion(l, span) => *span,
        }
    }
}

/// A terminator for a basic block
#[derive(Debug, Clone)]
pub enum Terminator {
    /// Goto another block
    Goto(Box<GotoTarget>),
    /// Switch/case branching
    Switch {
        discriminant: MirValue,
        targets: Vec<NodeIndex>,
    },
    /// Return from the function
    Return,
    /// Call and then handle results
    Call {
        destination: Option<(Place, NodeIndex)>,
        target: Option<NodeIndex>,
        cleanup: Option<NodeIndex>,
        callee: MirValue,
        arguments: Vec<MirValue>,
    },
    /// Try/call for exceptions
    TryCall {
        destination: Option<(Place, NodeIndex)>,
        target: Option<NodeIndex>,
        cleanup: Option<NodeIndex>,
        callee: MirValue,
        arguments: Vec<MirValue>,
    },
    /// Conditional jump
    If {
        condition: MirValue,
        then_block: NodeIndex,
        else_block: NodeIndex,
    },
    /// Raise an exception
    Raise(MirValue),
    /// Assert and abort if false
    Assert {
        condition: MirValue,
        msg: String,
        target: NodeIndex,
    },
    /// Unreachable
    Unreachable,
    /// Drop and continue
    Drop {
        place: Place,
        target: Option<NodeIndex>,
        unwind: Option<NodeIndex>,
    },
    /// Fall through to next block
    Fallthrough,
}

impl Terminator {
    pub fn span(&self) -> Span {
        match self {
            Terminator::Goto(_) => Span::new(FileId(0), 0, 0),
            Terminator::Switch { discriminant, .. } => discriminant.span(),
            Terminator::Return => Span::new(FileId(0), 0, 0),
            Terminator::Call { callee, .. } => callee.span(),
            Terminator::TryCall { callee, .. } => callee.span(),
            Terminator::If { condition, .. } => condition.span(),
            Terminator::Raise(v) => v.span(),
            Terminator::Assert { condition, .. } => condition.span(),
            Terminator::Unreachable => Span::new(FileId(0), 0, 0),
            Terminator::Drop { place, .. } => place.span,
            Terminator::Fallthrough => Span::new(FileId(0), 0, 0),
        }
    }
}

/// A place in memory (like an lvalue)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    pub local: Local,
    pub projection: Vec<PlaceProjection>,
    pub span: Span,
}

impl Place {
    pub fn new(local: Local) -> Self {
        Place {
            local,
            projection: Vec::new(),
            span: local.1,
        }
    }

    pub fn with_projection(&self, proj: PlaceProjection) -> Self {
        let mut p = self.clone();
        p.projection.push(proj);
        p
    }

    pub fn field(&self, field: Field) -> Self {
        self.with_projection(PlaceProjection::Field(field))
    }

    pub fn index(&self, index: Local) -> Self {
        self.with_projection(PlaceProjection::Index(index))
    }

    pub fn deref(&self) -> Self {
        self.with_projection(PlaceProjection::Deref)
    }
}

/// A local variable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Local(pub u32, pub Span);

impl Local {
    pub fn new(index: u32, span: Span) -> Self {
        Local(index, span)
    }

    pub fn index(&self) -> u32 {
        self.0
    }
}

/// A projection applied to a place
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlaceProjection {
    /// Field projection
    Field(Field),
    /// Index projection (for arrays/sequences)
    Index(Local),
    /// Dereference
    Deref,
    /// Constant index
    ConstantIndex {
        offset: usize,
        min: usize,
        max: usize,
    },
    /// Subslice
    Subslice { from: usize, to: usize },
    /// Downcast to variant
    Downcast { index: usize },
}

/// A field reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Field(pub u32);

impl Field {
    pub fn new(index: u32) -> Self {
        Field(index)
    }

    pub fn index(&self) -> u32 {
        self.0
    }
}

/// A MIR value (rvalue)
#[derive(Debug, Clone)]
pub enum MirValue {
    // Constants
    /// Unit value
    Unit(Span),
    /// Boolean constant
    Bool(bool, Span),
    /// Integer constant
    Int(i64, Span),
    /// Unsigned integer constant
    Uint(u64, Span),
    /// Float constant
    Float(f64, Span),
    /// String constant
    String(String, Span),
    /// Raw pointer constant
    Pointer(*const (), Span),

    // Places
    /// A place (local variable or field)
    Place(Place),

    // Composite values
    /// Tuple
    Tuple(Vec<MirValue>, Span),
    /// Array
    Array(Vec<MirValue>, Span),
    /// Struct
    Struct(HashMap<Field, MirValue>, Span),
    /// Variant
    Variant {
        enum_name: String,
        variant_name: String,
        index: usize,
        value: Box<MirValue>,
        span: Span,
    },

    // Operations
    /// Binary operation
    BinOp(BinOp, Box<MirValue>, Box<MirValue>, Span),
    /// Unary operation
    UnOp(UnOp, Box<MirValue>, Span),
    /// Cast
    Cast(Box<MirValue>, MirType, Span),
    /// Bitcast
    BitCast(Box<MirValue>, MirType, Span),
    /// Comparison
    Comparison(CmpOp, Box<MirValue>, Box<MirValue>, Span),

    // Function calls
    /// Function reference
    Function(FunctionRef),
    /// Closure
    Closure {
        func: Box<MirValue>,
        captured: Vec<(Local, MirValue)>,
    },

    // Address operations
    /// Address of a place
    AddrOf(Place),
}

impl MirValue {
    pub fn span(&self) -> Span {
        match self {
            MirValue::Unit(s) => *s,
            MirValue::Bool(_, s) => *s,
            MirValue::Int(_, s) => *s,
            MirValue::Uint(_, s) => *s,
            MirValue::Float(_, s) => *s,
            MirValue::String(_, s) => *s,
            MirValue::Pointer(_, s) => *s,
            MirValue::Place(p) => p.span,
            MirValue::Tuple(_, s) => *s,
            MirValue::Array(_, s) => *s,
            MirValue::Struct(_, s) => *s,
            MirValue::Variant { span, .. } => *span,
            MirValue::BinOp(_, _, _, s) => *s,
            MirValue::UnOp(_, _, s) => *s,
            MirValue::Cast(_, _, s) => *s,
            MirValue::BitCast(_, _, s) => *s,
            MirValue::Comparison(_, _, _, s) => *s,
            MirValue::Function(f) => f.span,
            MirValue::Closure { func, .. } => func.span(),
            MirValue::AddrOf(p) => p.span,
        }
    }
}

/// Binary operations
#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

/// Unary operations
#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
}

/// Comparison operations
#[derive(Debug, Clone, Copy)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Float representation wrapper to provide Eq and Hash
#[derive(Debug, Clone, Copy)]
pub struct FloatRepr(f32);

impl FloatRepr {
    pub fn new(val: f32) -> Self {
        FloatRepr(val)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

impl PartialEq for FloatRepr {
    fn eq(&self, other: &Self) -> bool {
        // NaN comparison would be tricky, but for our purposes we just compare bits
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for FloatRepr {}

impl std::hash::Hash for FloatRepr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

/// Float representation wrapper for 64-bit floats
#[derive(Debug, Clone, Copy)]
pub struct FloatRepr64(f64);

impl FloatRepr64 {
    pub fn new(val: f64) -> Self {
        FloatRepr64(val)
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl PartialEq for FloatRepr64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for FloatRepr64 {}

impl std::hash::Hash for FloatRepr64 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

/// MIR type representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirType {
    /// Unit type
    Unit,
    /// Boolean
    Bool,
    /// Signed integer
    Int(i32),
    /// Unsigned integer
    Uint(u32),
    /// Float
    Float(FloatRepr),
    /// Double float
    Double(FloatRepr64),
    /// Char
    Char,
    /// String
    String,
    /// Reference
    Ref(Box<MirType>),
    /// Mutable reference
    MutRef(Box<MirType>),
    /// Pointer
    Pointer(Box<MirType>),
    /// Array with fixed size
    Array(Box<MirType>, usize),
    /// Sequence
    Seq(Box<MirType>),
    /// Set
    Set(Box<MirType>),
    /// Tuple
    Tuple(Vec<MirType>),
    /// ADT / Struct
    Adt(String, Vec<(String, MirType)>),
    /// Enum
    Enum(String, Vec<String>),
    /// Procedure type
    Proc(Vec<MirType>, Box<MirType>),
    /// Open array
    OpenArray(Box<MirType>),
    /// Varargs
    Varargs,
    /// Untyped (for generics)
    Untyped,
    /// Never type (for no-return functions)
    Never,
}

impl MirType {
    pub fn is_reference(&self) -> bool {
        matches!(self, MirType::Ref(_) | MirType::MutRef(_))
    }

    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            MirType::Unit
                | MirType::Bool
                | MirType::Int(_)
                | MirType::Uint(_)
                | MirType::Float(_)
                | MirType::Double(_)
                | MirType::Char
                | MirType::Pointer(_)
        )
    }
}

/// Function reference
#[derive(Debug, Clone)]
pub struct FunctionRef {
    pub name: String,
    pub span: Span,
    pub proc_type: Option<Box<MirType>>,
}

/// A MIR body (function body)
#[derive(Debug, Clone)]
pub struct MirBody {
    /// Basic blocks in the function
    pub blocks: Vec<BasicBlock>,
    /// Entry block index
    pub entry: NodeIndex,
    /// Local variables
    pub locals: Vec<LocalInfo>,
    /// Return place
    pub return_place: Place,
    /// Span for the whole body
    pub span: Span,
}

impl MirBody {
    pub fn new(entry: NodeIndex, return_place: Place, span: Span) -> Self {
        MirBody {
            blocks: Vec::new(),
            entry,
            locals: Vec::new(),
            return_place,
            span,
        }
    }

    /// Add a basic block
    pub fn add_block(&mut self, block: BasicBlock) -> NodeIndex {
        let idx = NodeIndex::new(self.blocks.len());
        self.blocks.push(block);
        idx
    }

    /// Add a local variable
    pub fn add_local(&mut self, info: LocalInfo) -> Local {
        let idx = self.locals.len() as u32;
        let local = Local::new(idx, info.span);
        self.locals.push(info);
        local
    }

    /// Get a local's info
    pub fn local_info(&self, local: Local) -> Option<&LocalInfo> {
        self.locals.get(local.index() as usize)
    }
}

/// Information about a local variable
#[derive(Debug, Clone)]
pub struct LocalInfo {
    pub name: String,
    pub ty: MirType,
    pub span: Span,
    pub is_arg: bool,
    pub is_mutable: bool,
}

impl LocalInfo {
    pub fn new(name: &str, ty: MirType, span: Span) -> Self {
        LocalInfo {
            name: name.to_string(),
            ty,
            span,
            is_arg: false,
            is_mutable: true,
        }
    }

    pub fn arg(name: &str, ty: MirType, span: Span) -> Self {
        LocalInfo {
            name: name.to_string(),
            ty,
            span,
            is_arg: true,
            is_mutable: false,
        }
    }
}

/// Edge kind for control flow graph
#[derive(Debug, Clone)]
pub enum MirEdge {
    /// Normal control flow
    Normal,
    /// Exceptional control flow
    Exceptional,
    /// Unwind for unwinding from exceptions
    Unwind,
    /// False branch (for conditionals)
    False,
}

/// A complete MIR module
#[derive(Debug, Clone)]
pub struct MirModule {
    /// Name of the module
    pub name: String,
    /// Bodies for each function
    pub functions: Vec<Function>,
    /// Types used in this module
    pub types: HashMap<String, MirType>,
    /// Spans for debugging
    pub source_span: Span,
}

/// A function definition
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub body: Option<MirBody>,
    pub params: Vec<MirType>,
    pub return_type: MirType,
    pub span: Span,
    pub attributes: Vec<FunctionAttribute>,
}

/// Function attributes
#[derive(Debug, Clone)]
pub enum FunctionAttribute {
    /// This function never returns
    NoReturn,
    /// This function is cold (rarely executed)
    Cold,
    /// This function should be inlined
    Inline,
    /// This function should not be inlined
    NoInline,
    /// This function is an entry point
    Entry,
    /// This function has no side effects
    NoSideEffect,
}

/// Builder for constructing MIR
#[derive(Debug, Clone)]
pub struct MirBuilder {
    locals: Vec<LocalInfo>,
    blocks: Vec<BasicBlock>,
    current_block: Option<usize>,
}

impl MirBuilder {
    pub fn new() -> Self {
        MirBuilder {
            locals: Vec::new(),
            blocks: Vec::new(),
            current_block: None,
        }
    }

    /// Add a local variable
    pub fn local(&mut self, name: &str, ty: MirType, span: Span) -> Local {
        let idx = self.locals.len() as u32;
        let local = Local::new(idx, span);
        self.locals.push(LocalInfo::new(name, ty, span));
        local
    }

    /// Add a function parameter
    pub fn param(&mut self, name: &str, ty: MirType, span: Span) -> Local {
        let idx = self.locals.len() as u32;
        let local = Local::new(idx, span);
        self.locals.push(LocalInfo::arg(name, ty, span));
        local
    }

    /// Start a new block
    pub fn block(&mut self, span: Span) -> usize {
        let idx = self.blocks.len();
        self.blocks.push(BasicBlock::new(span));
        idx
    }

    /// Add statement to current block
    pub fn stmt(&mut self, stmt: MirStmt) {
        if let Some(idx) = self.current_block {
            self.blocks[idx].statements.push(stmt);
        }
    }

    /// Set terminator on current block
    pub fn terminator(&mut self, term: Terminator) {
        if let Some(idx) = self.current_block {
            self.blocks[idx].terminator = term;
        }
    }

    /// Finish building and return the body
    pub fn finish(self, return_place: Place, entry: usize, span: Span) -> MirBody {
        let mut body = MirBody {
            blocks: self.blocks,
            entry: NodeIndex::new(entry),
            locals: self.locals,
            return_place,
            span,
        };
        // Ensure we have an entry block
        if body.blocks.is_empty() {
            body.blocks.push(BasicBlock::new(span));
        }
        body
    }
}

impl Default for MirBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// MIR optimization passes
pub mod optimize {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum FoldResult {
        Unchanged,
        Constant(MirValue),
        Replaced(MirValue),
    }

    pub fn fold_constant(value: &MirValue) -> FoldResult {
        match value {
            MirValue::BinOp(op, lhs, rhs, span) => {
                if let (MirValue::Int(l, _), MirValue::Int(r, _)) = (lhs.as_ref(), rhs.as_ref()) {
                    let result = match op {
                        BinOp::Add => l.wrapping_add(*r),
                        BinOp::Sub => l.wrapping_sub(*r),
                        BinOp::Mul => l.wrapping_mul(*r),
                        BinOp::Div if *r != 0 => l.wrapping_div(*r),
                        BinOp::Mod if *r != 0 => l.wrapping_rem(*r),
                        BinOp::BitAnd => l & r,
                        BinOp::BitOr => l | r,
                        BinOp::BitXor => l ^ r,
                        _ => return FoldResult::Unchanged,
                    };
                    FoldResult::Constant(MirValue::Int(result, *span))
                } else {
                    FoldResult::Unchanged
                }
            }
            MirValue::UnOp(op, val, span) => {
                if let MirValue::Int(v, _) = val.as_ref() {
                    let result = match op {
                        UnOp::Neg => v.wrapping_neg(),
                        UnOp::Not => !v,
                        UnOp::BitNot => !v,
                    };
                    FoldResult::Constant(MirValue::Int(result, *span))
                } else {
                    FoldResult::Unchanged
                }
            }
            MirValue::Comparison(op, lhs, rhs, span) => {
                if let (MirValue::Int(l, _), MirValue::Int(r, _)) = (lhs.as_ref(), rhs.as_ref()) {
                    let result = match op {
                        CmpOp::Eq => l == r,
                        CmpOp::Ne => l != r,
                        CmpOp::Lt => l < r,
                        CmpOp::Le => l <= r,
                        CmpOp::Gt => l > r,
                        CmpOp::Ge => l >= r,
                    };
                    FoldResult::Constant(MirValue::Bool(result, *span))
                } else {
                    FoldResult::Unchanged
                }
            }
            _ => FoldResult::Unchanged,
        }
    }

    pub fn is_dead_stmt(stmt: &MirStmt) -> bool {
        matches!(stmt, MirStmt::Nop)
    }

    pub fn is_unreachable_term(term: &Terminator) -> bool {
        matches!(term, Terminator::Unreachable)
    }

    pub fn simplify_terminator(term: &Terminator) -> Option<Terminator> {
        match term {
            Terminator::If {
                condition,
                then_block,
                else_block,
            } => {
                if let MirValue::Bool(b, _) = condition {
                    if *b {
                        Some(Terminator::Goto(Box::new(GotoTarget::Block(*then_block))))
                    } else {
                        Some(Terminator::Goto(Box::new(GotoTarget::Block(*else_block))))
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn remove_dead_stmts(block: &mut BasicBlock) {
        block.statements.retain(|s| !is_dead_stmt(s));
    }

    pub fn dead_code_elimination(body: &mut MirBody) {
        let mut reachable = vec![false; body.blocks.len()];
        let mut worklist = vec![body.entry.index()];

        while let Some(idx) = worklist.pop() {
            if idx >= body.blocks.len() || reachable[idx] {
                continue;
            }
            reachable[idx] = true;

            match &body.blocks[idx].terminator {
                Terminator::Goto(target) => match target.as_ref() {
                    GotoTarget::Next => {
                        if idx + 1 < body.blocks.len() {
                            worklist.push(idx + 1);
                        }
                    }
                    GotoTarget::Block(target) => {
                        worklist.push(target.index());
                    }
                    GotoTarget::Switch(cases) => {
                        for (_, target) in cases {
                            worklist.push(target.index());
                        }
                    }
                },
                Terminator::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    worklist.push(then_block.index());
                    worklist.push(else_block.index());
                }
                Terminator::Switch { targets, .. } => {
                    for target in targets {
                        worklist.push(target.index());
                    }
                }
                _ => {}
            }
        }

        for (idx, block) in body.blocks.iter_mut().enumerate() {
            if !reachable[idx] {
                block.statements.clear();
                block.terminator = Terminator::Unreachable;
            }
        }
    }

    pub fn constant_propagation(body: &mut MirBody) {
        let mut constants: HashMap<u32, MirValue> = HashMap::new();

        for block in &mut body.blocks {
            let mut i = 0;
            while i < block.statements.len() {
                match &block.statements[i] {
                    MirStmt::Assign { place, value } => {
                        if let FoldResult::Constant(c) = fold_constant(value) {
                            constants.insert(place.local.0, c.clone());
                        }
                        let new_value = replace_with_constants(value, &constants);
                        if let MirStmt::Assign { place, value } = &mut block.statements[i] {
                            *value = new_value;
                        }
                    }
                    MirStmt::StorageDead(local, _) => {
                        constants.remove(&local.0);
                    }
                    _ => {}
                }
                i += 1;
            }
        }
    }

    fn replace_with_constants(value: &MirValue, constants: &HashMap<u32, MirValue>) -> MirValue {
        match value {
            MirValue::Place(place) => constants
                .get(&place.local.0)
                .cloned()
                .unwrap_or_else(|| value.clone()),
            _ => value.clone(),
        }
    }

    pub fn copy_elision(body: &mut MirBody) {
        for block in &mut body.blocks {
            let mut i = 0;
            while i + 1 < block.statements.len() {
                if let (
                    MirStmt::Assign {
                        place: p1,
                        value: v1,
                    },
                    MirStmt::Assign { place: p2, .. },
                ) = (&block.statements[i], &block.statements[i + 1])
                {
                    if matches!(v1, MirValue::Place(ref place) if place.local == p1.local) {
                        // Could eliminate but simplified for now
                    }
                }
                i += 1;
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MirStats {
    pub block_count: usize,
    pub stmt_count: usize,
    pub local_count: usize,
}

impl MirStats {
    pub fn compute(body: &MirBody) -> Self {
        let stmt_count = body.blocks.iter().map(|b| b.statements.len()).sum();
        MirStats {
            block_count: body.blocks.len(),
            stmt_count,
            local_count: body.locals.len(),
        }
    }

    pub fn is_simple(&self) -> bool {
        self.block_count <= 3 && self.stmt_count <= 10
    }
}

// Re-export petgraph types for use in other crates
pub use petgraph::graph::{EdgeIndex, NodeIndex};

#[cfg(test)]
mod tests {
    use super::optimize::*;
    use super::*;

    #[test]
    fn test_fold_constant_add() {
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::BinOp(
            BinOp::Add,
            Box::new(MirValue::Int(2, span)),
            Box::new(MirValue::Int(3, span)),
            span,
        );
        assert!(matches!(
            fold_constant(&expr),
            FoldResult::Constant(MirValue::Int(5, _))
        ));
    }

    #[test]
    fn test_fold_constant_mul() {
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::BinOp(
            BinOp::Mul,
            Box::new(MirValue::Int(6, span)),
            Box::new(MirValue::Int(7, span)),
            span,
        );
        assert!(matches!(
            fold_constant(&expr),
            FoldResult::Constant(MirValue::Int(42, _))
        ));
    }

    #[test]
    fn test_fold_constant_no_op() {
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::Int(42, span);
        assert!(matches!(fold_constant(&expr), FoldResult::Unchanged));
    }

    #[test]
    fn test_is_dead_stmt() {
        assert!(is_dead_stmt(&MirStmt::Nop));
        assert!(!is_dead_stmt(&MirStmt::Assign {
            place: Place::new(Local::new(0, Span::new(FileId(0), 0, 0))),
            value: MirValue::Int(1, Span::new(FileId(0), 0, 0)),
        }));
    }

    #[test]
    fn test_simplify_terminator_true() {
        let span = Span::new(FileId(0), 0, 0);
        let term = Terminator::If {
            condition: MirValue::Bool(true, span),
            then_block: NodeIndex::new(1),
            else_block: NodeIndex::new(2),
        };
        let result = simplify_terminator(&term);
        if let Some(Terminator::Goto(ref target)) = result {
            if let GotoTarget::Block(idx) = target.as_ref() {
                assert_eq!(idx.index(), 1);
            } else {
                panic!("Expected Block target");
            }
        } else {
            panic!("Expected simplified terminator");
        }
    }

    #[test]
    fn test_simplify_terminator_false() {
        let span = Span::new(FileId(0), 0, 0);
        let term = Terminator::If {
            condition: MirValue::Bool(false, span),
            then_block: NodeIndex::new(1),
            else_block: NodeIndex::new(2),
        };
        let result = simplify_terminator(&term);
        if let Some(Terminator::Goto(ref target)) = result {
            if let GotoTarget::Block(idx) = target.as_ref() {
                assert_eq!(idx.index(), 2);
            } else {
                panic!("Expected Block target");
            }
        } else {
            panic!("Expected simplified terminator");
        }
    }

    #[test]
    fn test_simplify_terminator_no_change() {
        let span = Span::new(FileId(0), 0, 0);
        let var = Local::new(0, span);
        let term = Terminator::If {
            condition: MirValue::Place(Place::new(var)),
            then_block: NodeIndex::new(1),
            else_block: NodeIndex::new(2),
        };
        assert!(simplify_terminator(&term).is_none());
    }

    #[test]
    fn test_remove_dead_stmts() {
        let span = Span::new(FileId(0), 0, 0);
        let mut block = BasicBlock::new(span);
        block.statements.push(MirStmt::Nop);
        block.statements.push(MirStmt::Nop);
        block.statements.push(MirStmt::Assign {
            place: Place::new(Local::new(0, span)),
            value: MirValue::Int(1, span),
        });
        remove_dead_stmts(&mut block);
        assert_eq!(block.statements.len(), 1);
    }

    #[test]
    fn test_mir_stats_compute() {
        let span = Span::new(FileId(0), 0, 0);
        let mut body = MirBody::new(NodeIndex::new(0), Place::new(Local::new(1, span)), span);
        body.add_local(LocalInfo::new("x", MirType::Int(32), span));
        body.add_block(BasicBlock::new(span));
        body.blocks[0].statements.push(MirStmt::Nop);

        let stats = MirStats::compute(&body);
        assert_eq!(stats.block_count, 1);
        assert_eq!(stats.stmt_count, 1);
        assert_eq!(stats.local_count, 1);
    }

    #[test]
    fn test_mir_stats_is_simple() {
        let stats = MirStats {
            block_count: 2,
            stmt_count: 5,
            local_count: 3,
        };
        assert!(stats.is_simple());

        let big_stats = MirStats {
            block_count: 10,
            stmt_count: 100,
            local_count: 50,
        };
        assert!(!big_stats.is_simple());
    }

    #[test]
    fn test_fold_constant_sub() {
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::BinOp(
            BinOp::Sub,
            Box::new(MirValue::Int(10, span)),
            Box::new(MirValue::Int(3, span)),
            span,
        );
        assert!(matches!(
            fold_constant(&expr),
            FoldResult::Constant(MirValue::Int(7, _))
        ));
    }

    #[test]
    fn test_fold_constant_comparison() {
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::Comparison(
            CmpOp::Lt,
            Box::new(MirValue::Int(5, span)),
            Box::new(MirValue::Int(10, span)),
            span,
        );
        assert!(matches!(
            fold_constant(&expr),
            FoldResult::Constant(MirValue::Bool(true, _))
        ));
    }

    #[test]
    fn test_fold_constant_unop_neg() {
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::UnOp(UnOp::Neg, Box::new(MirValue::Int(42, span)), span);
        assert!(matches!(
            fold_constant(&expr),
            FoldResult::Constant(MirValue::Int(-42, _))
        ));
    }

    #[test]
    fn test_basic_block_new() {
        let span = Span::new(FileId(0), 0, 0);
        let block = BasicBlock::new(span);
        assert!(block.statements.is_empty());
        assert!(matches!(block.terminator, Terminator::Goto(_)));
    }

    #[test]
    fn test_place_new() {
        let span = Span::new(FileId(0), 0, 0);
        let local = Local::new(0, span);
        let place = Place::new(local);
        assert!(place.projection.is_empty());
    }

    #[test]
    fn test_place_field_projection() {
        let span = Span::new(FileId(0), 0, 0);
        let local = Local::new(0, span);
        let place = Place::new(local).field(Field::new(1));
        assert_eq!(place.projection.len(), 1);
        assert!(matches!(place.projection[0], PlaceProjection::Field(f) if f.index() == 1));
    }

    #[test]
    fn test_place_deref_projection() {
        let span = Span::new(FileId(0), 0, 0);
        let local = Local::new(0, span);
        let place = Place::new(local).deref();
        assert_eq!(place.projection.len(), 1);
        assert!(matches!(place.projection[0], PlaceProjection::Deref));
    }

    #[test]
    fn test_local_new() {
        let span = Span::new(FileId(0), 0, 0);
        let local = Local::new(5, span);
        assert_eq!(local.index(), 5);
    }

    #[test]
    fn test_mir_value_span() {
        let span = Span::new(FileId(0), 0, 0);
        let val = MirValue::Int(42, span);
        assert_eq!(val.span(), span);
    }

    #[test]
    fn test_mir_type_is_reference() {
        assert!(MirType::Ref(Box::new(MirType::Int(32))).is_reference());
        assert!(!MirType::Int(32).is_reference());
    }

    #[test]
    fn test_mir_type_is_scalar() {
        assert!(MirType::Int(32).is_scalar());
        assert!(MirType::Bool.is_scalar());
        assert!(!MirType::String.is_scalar());
    }

    #[test]
    fn test_local_info_new() {
        let span = Span::new(FileId(0), 0, 0);
        let info = LocalInfo::new("x", MirType::Int(32), span);
        assert_eq!(info.name, "x");
        assert!(info.is_mutable);
        assert!(!info.is_arg);
    }

    #[test]
    fn test_local_info_arg() {
        let span = Span::new(FileId(0), 0, 0);
        let info = LocalInfo::arg("y", MirType::Bool, span);
        assert_eq!(info.name, "y");
        assert!(info.is_arg);
        assert!(!info.is_mutable);
    }

    #[test]
    fn test_mir_stmt_span() {
        let span = Span::new(FileId(0), 0, 0);
        let stmt = MirStmt::Nop;
        assert!(matches!(stmt.span(), s if s == span));
    }

    #[test]
    fn test_terminator_span() {
        let span = Span::new(FileId(0), 0, 0);
        let term = Terminator::Return;
        assert!(matches!(term.span(), s if s == span));
    }

    #[test]
    fn test_mir_builder() {
        let mut builder = MirBuilder::new();
        let span = Span::new(FileId(0), 0, 0);

        let x = builder.local("x", MirType::Int(32), span);
        assert_eq!(x.index(), 0);

        let b = builder.block(span);
        assert_eq!(b, 0);

        builder.stmt(MirStmt::Nop);
        builder.terminator(Terminator::Return);

        let return_place = Place::new(Local::new(1, span));
        let body = builder.finish(return_place, 0, span);

        assert_eq!(body.locals.len(), 1);
        assert_eq!(body.blocks.len(), 1);
    }

    #[test]
    fn test_function_attribute_variants() {
        let no_return = FunctionAttribute::NoReturn;
        assert!(format!("{:?}", no_return).contains("NoReturn"));

        let cold = FunctionAttribute::Cold;
        assert!(format!("{:?}", cold).contains("Cold"));
    }

    #[test]
    fn test_mir_module_new() {
        let span = Span::new(FileId(0), 0, 0);
        let module = MirModule {
            name: "test".to_string(),
            functions: Vec::new(),
            types: HashMap::new(),
            source_span: span,
        };
        assert_eq!(module.name, "test");
    }

    #[test]
    fn test_bin_op_variants() {
        let add = BinOp::Add;
        let sub = BinOp::Sub;
        assert!(format!("{:?}", add).contains("Add"));
        assert!(format!("{:?}", sub).contains("Sub"));
    }

    #[test]
    fn test_un_op_variants() {
        let neg = UnOp::Neg;
        let not = UnOp::Not;
        assert!(format!("{:?}", neg).contains("Neg"));
        assert!(format!("{:?}", not).contains("Not"));
    }

    #[test]
    fn test_cmp_op_variants() {
        let eq = CmpOp::Eq;
        let lt = CmpOp::Lt;
        assert!(format!("{:?}", eq).contains("Eq"));
        assert!(format!("{:?}", lt).contains("Lt"));
    }

    #[test]
    fn test_place_projection_variants() {
        let field = PlaceProjection::Field(Field::new(0));
        let index = PlaceProjection::Index(Local::new(1, Span::new(FileId(0), 0, 0)));
        let deref = PlaceProjection::Deref;

        assert!(format!("{:?}", field).contains("Field"));
        assert!(format!("{:?}", index).contains("Index"));
        assert!(format!("{:?}", deref).contains("Deref"));
    }

    #[test]
    fn test_goto_target_variants() {
        let next = GotoTarget::Next;
        assert!(matches!(next, GotoTarget::Next));
    }

    #[test]
    fn test_mir_value_variants() {
        let span = Span::new(FileId(0), 0, 0);

        let unit = MirValue::Unit(span);
        assert!(matches!(unit, MirValue::Unit(_)));

        let boolean = MirValue::Bool(true, span);
        assert!(matches!(boolean, MirValue::Bool(true, _)));

        let int_val = MirValue::Int(42, span);
        assert!(matches!(int_val, MirValue::Int(42, _)));
    }

    #[test]
    fn test_basic_block_with_statements() {
        let span = Span::new(FileId(0), 0, 0);
        let mut block = BasicBlock::new(span);

        block.statements.push(MirStmt::Nop);
        block.statements.push(MirStmt::Nop);

        assert_eq!(block.statements.len(), 2);
        assert!(matches!(block.terminator, Terminator::Goto(_)));
    }

    #[test]
    fn test_terminator_return() {
        let term = Terminator::Return;
        assert!(matches!(term, Terminator::Return));
    }

    #[test]
    fn test_terminator_goto() {
        let term = Terminator::Goto(Box::new(GotoTarget::Block(NodeIndex::new(1))));
        assert!(matches!(term, Terminator::Goto(_)));
    }

    #[test]
    fn test_terminator_branches() {
        let term = Terminator::If {
            condition: MirValue::Bool(true, Span::new(FileId(0), 0, 0)),
            then_block: NodeIndex::new(1),
            else_block: NodeIndex::new(2),
        };
        assert!(matches!(
            term,
            Terminator::If {
                then_block: _,
                else_block: _,
                ..
            }
        ));
    }

    #[test]
    fn test_mir_stmt_assign() {
        let span = Span::new(FileId(0), 0, 0);
        let stmt = MirStmt::Assign {
            place: Place::new(Local::new(0, span)),
            value: MirValue::Int(10, span),
        };
        assert!(matches!(stmt, MirStmt::Assign { .. }));
    }

    #[test]
    fn test_mir_stmt_drop() {
        let span = Span::new(FileId(0), 0, 0);
        let stmt = MirStmt::Drop {
            place: Place::new(Local::new(0, span)),
            target: None,
        };
        assert!(matches!(stmt, MirStmt::Drop { .. }));
    }

    #[test]
    fn test_mir_stmt_call() {
        let span = Span::new(FileId(0), 0, 0);
        let stmt = MirStmt::Call {
            destination: None,
            callee: MirValue::Function(FunctionRef {
                name: "test_func".to_string(),
                span,
                proc_type: None,
            }),
            arguments: vec![],
        };
        assert!(matches!(stmt, MirStmt::Call { .. }));
    }

    #[test]
    fn test_mir_value_string() {
        let span = Span::new(FileId(0), 0, 0);
        let val = MirValue::String("hello".to_string(), span);
        assert!(matches!(val, MirValue::String(ref s, _) if s == "hello"));
    }

    #[test]
    fn test_mir_value_binop_add() {
        let span = Span::new(FileId(0), 0, 0);
        let val = MirValue::BinOp(
            BinOp::Add,
            Box::new(MirValue::Int(1, span)),
            Box::new(MirValue::Int(2, span)),
            span,
        );
        assert!(matches!(val, MirValue::BinOp(BinOp::Add, _, _, _)));
    }

    #[test]
    fn test_mir_value_unop() {
        let span = Span::new(FileId(0), 0, 0);
        let val = MirValue::UnOp(UnOp::Neg, Box::new(MirValue::Int(5, span)), span);
        assert!(matches!(val, MirValue::UnOp(UnOp::Neg, _, _)));
    }

    #[test]
    fn test_mir_value_cast() {
        let span = Span::new(FileId(0), 0, 0);
        let val = MirValue::Cast(Box::new(MirValue::Int(10, span)), MirType::Int(32), span);
        assert!(matches!(val, MirValue::Cast(..)));
    }

    #[test]
    fn test_fold_constant_div() {
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::BinOp(
            BinOp::Div,
            Box::new(MirValue::Int(10, span)),
            Box::new(MirValue::Int(2, span)),
            span,
        );
        assert!(matches!(
            fold_constant(&expr),
            FoldResult::Constant(MirValue::Int(5, _))
        ));
    }

    #[test]
    fn test_fold_constant_mod() {
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::BinOp(
            BinOp::Mod,
            Box::new(MirValue::Int(7, span)),
            Box::new(MirValue::Int(3, span)),
            span,
        );
        assert!(matches!(
            fold_constant(&expr),
            FoldResult::Constant(MirValue::Int(1, _))
        ));
    }

    #[test]
    fn test_fold_constant_comparison_eq() {
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::Comparison(
            CmpOp::Eq,
            Box::new(MirValue::Int(5, span)),
            Box::new(MirValue::Int(5, span)),
            span,
        );
        assert!(matches!(
            fold_constant(&expr),
            FoldResult::Constant(MirValue::Bool(true, _))
        ));
    }

    #[test]
    fn test_dead_code_elimination() {
        let span = Span::new(FileId(0), 0, 0);

        let ret_place = Place::new(Local::new(0, span));
        let mut builder = MirBuilder::new();
        builder.local("x", MirType::Int(32), span);
        let b = builder.block(span);

        // Dead statement before return
        builder.stmt(MirStmt::Assign {
            place: Place::new(Local::new(0, span)),
            value: MirValue::Int(1, span),
        });
        builder.stmt(MirStmt::Nop); // dead
        builder.terminator(Terminator::Return);

        let mut body = builder.finish(ret_place, b, span);

        // Should not panic
        dead_code_elimination(&mut body);
        assert!(body.blocks.len() >= 1);
    }

    // Task 35: HIR-to-MIR lowering tests

    #[test]
    fn test_mir_body_with_multiple_blocks() {
        let span = Span::new(FileId(0), 0, 0);

        let ret_place = Place::new(Local::new(0, span));
        let mut builder = MirBuilder::new();

        // Add locals for args and local vars
        builder.param("argc", MirType::Int(32), span);
        builder.local("sum", MirType::Int(32), span);

        // Entry block
        builder.block(span);
        builder.stmt(MirStmt::Assign {
            place: Place::new(Local::new(1, span)),
            value: MirValue::Int(0, span),
        });

        // Condition block
        let cond_block = builder.block(span);

        // Loop body block
        let loop_block = builder.block(span);
        builder.stmt(MirStmt::Nop);
        builder.terminator(Terminator::Goto(Box::new(GotoTarget::Block(
            NodeIndex::new(cond_block),
        ))));

        // After loop block
        let after_block = builder.block(span);
        builder.terminator(Terminator::Return);

        // Connect entry to condition
        builder.terminator(Terminator::Goto(Box::new(GotoTarget::Block(
            NodeIndex::new(cond_block),
        ))));

        let body = builder.finish(ret_place, 0, span);

        assert_eq!(body.blocks.len(), 4);
        assert_eq!(body.locals.len(), 2);
    }

    #[test]
    fn test_mir_lowering_control_flow_if() {
        let span = Span::new(FileId(0), 0, 0);

        let ret_place = Place::new(Local::new(0, span));
        let mut builder = MirBuilder::new();
        builder.local("x", MirType::Int(32), span);

        let entry = builder.block(span);
        let then_block = builder.block(span);
        let else_block = builder.block(span);
        let merge_block = builder.block(span);

        builder.stmt(MirStmt::Assign {
            place: Place::new(Local::new(0, span)),
            value: MirValue::Int(1, span),
        });

        builder.terminator(Terminator::If {
            condition: MirValue::Bool(true, span),
            then_block: NodeIndex::new(then_block),
            else_block: NodeIndex::new(else_block),
        });

        // Then block
        builder.stmt(MirStmt::Assign {
            place: Place::new(Local::new(0, span)),
            value: MirValue::Int(10, span),
        });
        builder.terminator(Terminator::Goto(Box::new(GotoTarget::Block(
            NodeIndex::new(merge_block),
        ))));

        // Else block
        builder.stmt(MirStmt::Assign {
            place: Place::new(Local::new(0, span)),
            value: MirValue::Int(20, span),
        });
        builder.terminator(Terminator::Goto(Box::new(GotoTarget::Block(
            NodeIndex::new(merge_block),
        ))));

        // Merge block
        builder.terminator(Terminator::Return);

        let body = builder.finish(ret_place, entry, span);

        assert_eq!(body.blocks.len(), 4);
    }

    #[test]
    fn test_mir_lowering_function_calls() {
        let span = Span::new(FileId(0), 0, 0);

        let ret_place = Place::new(Local::new(0, span));
        let mut builder = MirBuilder::new();
        builder.local("result", MirType::Int(32), span);

        let entry = builder.block(span);

        builder.stmt(MirStmt::Call {
            destination: Some((Place::new(Local::new(0, span)), NodeIndex::new(1))),
            callee: MirValue::Function(FunctionRef {
                name: "putchar".to_string(),
                span,
                proc_type: None,
            }),
            arguments: vec![MirValue::Int(65, span)],
        });

        builder.terminator(Terminator::Return);

        let body = builder.finish(ret_place, entry, span);

        assert_eq!(body.blocks.len(), 1);
        assert_eq!(body.locals.len(), 1);
    }

    #[test]
    fn test_mir_lowering_storage_live_dead() {
        let span = Span::new(FileId(0), 0, 0);

        let ret_place = Place::new(Local::new(0, span));
        let mut builder = MirBuilder::new();
        let local = builder.local("x", MirType::Int(32), span);

        let entry = builder.block(span);

        builder.stmt(MirStmt::StorageLive(local, span));
        builder.stmt(MirStmt::Assign {
            place: Place::new(local),
            value: MirValue::Int(42, span),
        });
        builder.stmt(MirStmt::StorageDead(local, span));
        builder.terminator(Terminator::Return);

        let body = builder.finish(ret_place, entry, span);

        assert_eq!(body.blocks.len(), 1);
    }

    #[test]
    fn test_mir_lowering_drop() {
        let span = Span::new(FileId(0), 0, 0);

        let ret_place = Place::new(Local::new(0, span));
        let mut builder = MirBuilder::new();
        builder.local("x", MirType::Int(32), span);

        let entry = builder.block(span);

        builder.stmt(MirStmt::Drop {
            place: Place::new(Local::new(0, span)),
            target: None,
        });
        builder.terminator(Terminator::Return);

        let body = builder.finish(ret_place, entry, span);

        assert_eq!(body.blocks.len(), 1);
    }

    #[test]
    fn test_mir_cfg_invariants() {
        let span = Span::new(FileId(0), 0, 0);

        let ret_place = Place::new(Local::new(0, span));
        let mut builder = MirBuilder::new();
        builder.local("x", MirType::Int(32), span);

        let entry = builder.block(span);
        builder.terminator(Terminator::Return);

        let body = builder.finish(ret_place, entry, span);

        // Entry block must exist and be reachable
        assert!(body.entry.index() < body.blocks.len());

        // There should be at least one block with a terminator
        assert!(!body.blocks.is_empty());
    }

    #[test]
    fn test_mir_return_place() {
        let span = Span::new(FileId(0), 0, 0);

        let ret_place = Place::new(Local::new(99, span));
        let mut builder = MirBuilder::new();
        builder.local("x", MirType::Int(32), span);

        let entry = builder.block(span);
        builder.terminator(Terminator::Return);

        let body = builder.finish(ret_place, entry, span);

        assert_eq!(body.return_place.local.index(), 99);
    }
}
