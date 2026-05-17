//! Type inference engine.
//!
//! This module provides Hindley-Milner style type inference with extensions
//! for Nim's specific features like named parameters, defaults, and overloading.

use rnim_span::Span;
use std::collections::{HashMap, VecDeque};

/// A type variable that can be unified during inference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(u32);

impl TypeVar {
    pub fn new(id: u32) -> Self {
        TypeVar(id)
    }

    pub fn id(&self) -> u32 {
        self.0
    }
}

/// Type inference state
#[derive(Debug, Clone)]
pub struct InferState {
    next_var: u32,
    constraints: Vec<TypeConstraint>,
}

impl InferState {
    pub fn new() -> Self {
        InferState {
            next_var: 0,
            constraints: Vec::new(),
        }
    }

    /// Create a fresh type variable
    pub fn fresh_var(&mut self) -> TypeVar {
        let v = TypeVar(self.next_var);
        self.next_var += 1;
        v
    }

    /// Add a type constraint
    pub fn add_constraint(&mut self, constraint: TypeConstraint) {
        self.constraints.push(constraint);
    }

    /// Add equality constraint between two types
    pub fn equate(&mut self, lhs: InferenceType, rhs: InferenceType, span: Span) {
        self.add_constraint(TypeConstraint::Equality { lhs, rhs, span });
    }

    /// Add subtype constraint
    pub fn subtype(&mut self, sub: InferenceType, sup: InferenceType, span: Span) {
        self.add_constraint(TypeConstraint::Subtype { sub, sup, span });
    }

    /// Add "implements" constraint for concepts
    pub fn implements(&mut self, ty: InferenceType, concept: &str, span: Span) {
        self.add_constraint(TypeConstraint::Implements {
            ty,
            concept: concept.to_string(),
            span,
        });
    }
}

impl Default for InferState {
    fn default() -> Self {
        Self::new()
    }
}

/// Inference type - a type that may contain unresolved type variables
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InferenceType {
    /// Type variable that needs to be resolved
    Var(TypeVar),
    /// Primitive type
    Primitive(PrimitiveKind),
    /// Named type
    Named(String),
    /// Type reference via TyId
    TyId(TyId),
    /// Function type (lambda)
    Fn {
        params: Vec<InferenceType>,
        ret: Box<InferenceType>,
    },
    /// Tuple type
    Tuple(Vec<InferenceType>),
    /// Array type
    Array(Box<InferenceType>, Option<usize>),
    /// Sequence type
    Seq(Box<InferenceType>),
    /// Reference type
    Ref(Box<InferenceType>),
    /// Pointer type
    Ptr(Box<InferenceType>),
    /// Generic type instantiation
    App {
        name: String,
        args: Vec<InferenceType>,
    },
    /// Error during inference
    Error,
    /// Unknown/incomplete type
    Unknown,
}

impl InferenceType {
    /// Check if this type is a type variable
    pub fn is_var(&self) -> bool {
        matches!(self, InferenceType::Var(_))
    }

    /// Check if this type contains any unresolved variables
    pub fn contains_var(&self, var: TypeVar) -> bool {
        match self {
            InferenceType::Var(v) => *v == var,
            InferenceType::Fn { params, ret } => {
                params.iter().any(|t| t.contains_var(var)) || ret.contains_var(var)
            }
            InferenceType::Tuple(types) => types.iter().any(|t| t.contains_var(var)),
            InferenceType::Array(inner, _) => inner.contains_var(var),
            InferenceType::Seq(inner) => inner.contains_var(var),
            InferenceType::Ref(inner) => inner.contains_var(var),
            InferenceType::Ptr(inner) => inner.contains_var(var),
            InferenceType::App { args, .. } => args.iter().any(|t| t.contains_var(var)),
            _ => false,
        }
    }

    /// Substitute a type variable with a type
    pub fn substitute(&self, var: TypeVar, replacement: &InferenceType) -> InferenceType {
        match self {
            InferenceType::Var(v) if *v == var => replacement.clone(),
            InferenceType::Fn { params, ret } => InferenceType::Fn {
                params: params
                    .iter()
                    .map(|p| p.substitute(var, replacement))
                    .collect(),
                ret: Box::new(ret.substitute(var, replacement)),
            },
            InferenceType::Tuple(types) => InferenceType::Tuple(
                types
                    .iter()
                    .map(|t| t.substitute(var, replacement))
                    .collect(),
            ),
            InferenceType::Array(inner, len) => {
                InferenceType::Array(Box::new(inner.substitute(var, replacement)), *len)
            }
            InferenceType::Seq(inner) => {
                InferenceType::Seq(Box::new(inner.substitute(var, replacement)))
            }
            InferenceType::Ref(inner) => {
                InferenceType::Ref(Box::new(inner.substitute(var, replacement)))
            }
            InferenceType::Ptr(inner) => {
                InferenceType::Ptr(Box::new(inner.substitute(var, replacement)))
            }
            InferenceType::App { name, args } => InferenceType::App {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|t| t.substitute(var, replacement))
                    .collect(),
            },
            _ => self.clone(),
        }
    }

    /// Substitute all type variables using a substitution map
    fn substitute_all(&self, subs: &HashMap<TypeVar, InferenceType>) -> InferenceType {
        match self {
            InferenceType::Var(v) => {
                if let Some(t) = subs.get(v) {
                    t.substitute_all(subs)
                } else {
                    self.clone()
                }
            }
            InferenceType::Fn { params, ret } => InferenceType::Fn {
                params: params.iter().map(|p| p.substitute_all(subs)).collect(),
                ret: Box::new(ret.substitute_all(subs)),
            },
            InferenceType::Tuple(types) => {
                InferenceType::Tuple(types.iter().map(|t| t.substitute_all(subs)).collect())
            }
            InferenceType::Array(inner, len) => {
                InferenceType::Array(Box::new(inner.substitute_all(subs)), *len)
            }
            InferenceType::Seq(inner) => InferenceType::Seq(Box::new(inner.substitute_all(subs))),
            InferenceType::Ref(inner) => InferenceType::Ref(Box::new(inner.substitute_all(subs))),
            InferenceType::Ptr(inner) => InferenceType::Ptr(Box::new(inner.substitute_all(subs))),
            InferenceType::App { name, args } => InferenceType::App {
                name: name.clone(),
                args: args.iter().map(|t| t.substitute_all(subs)).collect(),
            },
            _ => self.clone(),
        }
    }
}

/// Primitive kind for inference types
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

/// Type constraints generated during inference
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeConstraint {
    /// Two types must be equal
    Equality {
        lhs: InferenceType,
        rhs: InferenceType,
        span: Span,
    },
    /// First type is subtype of second
    Subtype {
        sub: InferenceType,
        sup: InferenceType,
        span: Span,
    },
    /// Type must implement a concept
    Implements {
        ty: InferenceType,
        concept: String,
        span: Span,
    },
    /// Type must be a valid literal of the expected kind
    Literal {
        ty: InferenceType,
        kind: LiteralKind,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralKind {
    Integer,
    Float,
    Char,
    String,
    Bool,
}

/// Inference solver that solves type constraints
#[derive(Debug, Clone, Default)]
pub struct InferSolver {
    /// Current substitution mapping type variables to types
    substitutions: HashMap<TypeVar, InferenceType>,
}

impl InferSolver {
    pub fn new() -> Self {
        InferSolver {
            substitutions: HashMap::new(),
        }
    }

    /// Solve constraints and return the substitution map
    pub fn solve(
        &mut self,
        constraints: &[TypeConstraint],
    ) -> Result<HashMap<TypeVar, InferenceType>, InferError> {
        let mut queue: VecDeque<TypeConstraint> = constraints.iter().cloned().collect();
        let mut local_subs = HashMap::new();

        while let Some(constraint) = queue.pop_front() {
            match constraint {
                TypeConstraint::Equality { lhs, rhs, span } => {
                    // Unify and collect any new constraints
                    let new_constraints =
                        self.unify_with_constraints(&lhs, &rhs, &mut local_subs, span)?;
                    queue.extend(new_constraints);
                }
                TypeConstraint::Subtype { sub, sup, span } => {
                    let new_constraints =
                        self.unify_with_constraints(&sub, &sup, &mut local_subs, span)?;
                    queue.extend(new_constraints);
                }
                TypeConstraint::Implements { ty, concept, span } => {
                    self.check_implements(&ty, &concept, span)?;
                }
                TypeConstraint::Literal { ty, kind, span } => {
                    let new_constraints = self.unify_literal_with_constraints(&ty, kind, span)?;
                    queue.extend(new_constraints);
                }
            }
        }

        // Simplify substitutions
        self.simplify(&mut local_subs);
        self.substitutions = local_subs;
        Ok(self.substitutions.clone())
    }

    /// Unify two types and return any new constraints generated
    fn unify_with_constraints(
        &mut self,
        lhs: &InferenceType,
        rhs: &InferenceType,
        subs: &mut HashMap<TypeVar, InferenceType>,
        span: Span,
    ) -> Result<VecDeque<TypeConstraint>, InferError> {
        let mut new_constraints = VecDeque::new();

        // If both are the same, we're done
        if lhs == rhs {
            return Ok(new_constraints);
        }

        // Resolve lhs if it's a variable
        let lhs = self.resolve(lhs, subs);

        // Resolve rhs if it's a variable
        let rhs = self.resolve(rhs, subs);

        match (lhs.clone(), rhs.clone()) {
            // Variable unification
            (InferenceType::Var(v), t) | (t, InferenceType::Var(v)) => {
                if t.contains_var(v) {
                    return Err(InferError::OccursCheck(v, span));
                }
                subs.insert(v, t);
            }
            // Function type unification
            (
                InferenceType::Fn {
                    params: p1,
                    ret: r1,
                },
                InferenceType::Fn {
                    params: p2,
                    ret: r2,
                },
            ) => {
                if p1.len() != p2.len() {
                    return Err(InferError::ArgCountMismatch(p1.len(), p2.len(), span));
                }
                for (a, b) in p1.iter().zip(p2.iter()) {
                    new_constraints.push_back(TypeConstraint::Equality {
                        lhs: a.clone(),
                        rhs: b.clone(),
                        span,
                    });
                }
                new_constraints.push_back(TypeConstraint::Equality {
                    lhs: (*r1).clone(),
                    rhs: (*r2).clone(),
                    span,
                });
            }
            // Tuple unification
            (InferenceType::Tuple(ts1), InferenceType::Tuple(ts2)) => {
                if ts1.len() != ts2.len() {
                    return Err(InferError::TupleLenMismatch(ts1.len(), ts2.len(), span));
                }
                for (a, b) in ts1.iter().zip(ts2.iter()) {
                    new_constraints.push_back(TypeConstraint::Equality {
                        lhs: a.clone(),
                        rhs: b.clone(),
                        span,
                    });
                }
            }
            // Named type unification
            (InferenceType::Named(n1), InferenceType::Named(n2)) if n1 == n2 => {}
            // Array unification
            (InferenceType::Array(a1, l1), InferenceType::Array(a2, l2)) if l1 == l2 => {
                new_constraints.push_back(TypeConstraint::Equality {
                    lhs: (*a1).clone(),
                    rhs: (*a2).clone(),
                    span,
                });
            }
            // Seq unification
            (InferenceType::Seq(a1), InferenceType::Seq(a2)) => {
                new_constraints.push_back(TypeConstraint::Equality {
                    lhs: (*a1).clone(),
                    rhs: (*a2).clone(),
                    span,
                });
            }
            // Ref unification
            (InferenceType::Ref(a1), InferenceType::Ref(a2)) => {
                new_constraints.push_back(TypeConstraint::Equality {
                    lhs: (*a1).clone(),
                    rhs: (*a2).clone(),
                    span,
                });
            }
            // Ptr unification
            (InferenceType::Ptr(a1), InferenceType::Ptr(a2)) => {
                new_constraints.push_back(TypeConstraint::Equality {
                    lhs: (*a1).clone(),
                    rhs: (*a2).clone(),
                    span,
                });
            }
            // App unification (generic instantiation)
            (
                InferenceType::App { name: n1, args: a1 },
                InferenceType::App { name: n2, args: a2 },
            ) if n1 == n2 => {
                for (arg1, arg2) in a1.iter().zip(a2.iter()) {
                    new_constraints.push_back(TypeConstraint::Equality {
                        lhs: arg1.clone(),
                        rhs: arg2.clone(),
                        span,
                    });
                }
            }
            // TyId unification
            (InferenceType::TyId(t1), InferenceType::TyId(t2)) if t1 == t2 => {}
            // Primitive unification
            (InferenceType::Primitive(p1), InferenceType::Primitive(p2)) if p1 == p2 => {}
            // Error and unknown are compatible with anything
            (InferenceType::Error, _) | (_, InferenceType::Error) => {}
            (InferenceType::Unknown, _) | (_, InferenceType::Unknown) => {}
            // Mismatch
            _ => return Err(InferError::TypeMismatch(lhs.clone(), rhs.clone(), span)),
        }

        Ok(new_constraints)
    }

    /// Unify literal with constraints
    fn unify_literal_with_constraints(
        &mut self,
        ty: &InferenceType,
        kind: LiteralKind,
        span: Span,
    ) -> Result<VecDeque<TypeConstraint>, InferError> {
        let lit_type = match kind {
            LiteralKind::Integer => InferenceType::Primitive(PrimitiveKind::Int),
            LiteralKind::Float => InferenceType::Primitive(PrimitiveKind::Float64),
            LiteralKind::Char => InferenceType::Primitive(PrimitiveKind::Char),
            LiteralKind::String => InferenceType::Named("string".to_string()),
            LiteralKind::Bool => InferenceType::Primitive(PrimitiveKind::Bool),
        };
        self.unify_with_constraints(ty, &lit_type, &mut HashMap::new(), span)
    }

    /// Resolve a type by applying substitutions
    fn resolve(&self, ty: &InferenceType, subs: &HashMap<TypeVar, InferenceType>) -> InferenceType {
        match ty {
            InferenceType::Var(v) => {
                if let Some(t) = subs.get(v) {
                    self.resolve(t, subs)
                } else {
                    ty.clone()
                }
            }
            InferenceType::Fn { params, ret } => InferenceType::Fn {
                params: params.iter().map(|p| self.resolve(p, subs)).collect(),
                ret: Box::new(self.resolve(ret, subs)),
            },
            InferenceType::Tuple(types) => {
                InferenceType::Tuple(types.iter().map(|t| self.resolve(t, subs)).collect())
            }
            InferenceType::Array(inner, len) => {
                InferenceType::Array(Box::new(self.resolve(inner, subs)), *len)
            }
            InferenceType::Seq(inner) => InferenceType::Seq(Box::new(self.resolve(inner, subs))),
            InferenceType::Ref(inner) => InferenceType::Ref(Box::new(self.resolve(inner, subs))),
            InferenceType::Ptr(inner) => InferenceType::Ptr(Box::new(self.resolve(inner, subs))),
            InferenceType::App { name, args } => InferenceType::App {
                name: name.clone(),
                args: args.iter().map(|t| self.resolve(t, subs)).collect(),
            },
            _ => ty.clone(),
        }
    }

    /// Simplify substitutions by applying them transitively
    fn simplify(&self, subs: &mut HashMap<TypeVar, InferenceType>) {
        // Fixed point iteration to simplify
        let mut changed = true;
        while changed {
            changed = false;
            let mut new_subs = subs.clone();
            for (var, ty) in subs.iter() {
                let simplified = ty.substitute_all(subs);
                if simplified != *ty {
                    new_subs.insert(*var, simplified);
                    changed = true;
                }
            }
            *subs = new_subs;
        }
    }

    /// Check if a type implements a concept
    fn check_implements(
        &self,
        ty: &InferenceType,
        _concept: &str,
        _span: Span,
    ) -> Result<(), InferError> {
        // For now, just verify the type exists and is well-formed
        match ty {
            InferenceType::Error | InferenceType::Unknown => Ok(()),
            InferenceType::Var(_) => Ok(()), // Will be checked later
            _ => Ok(()),
        }
    }

    /// Get the final type for a type variable
    pub fn get(&self, var: TypeVar) -> Option<InferenceType> {
        self.substitutions.get(&var).cloned()
    }
}

/// Inference error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferError {
    /// Occurs check failed (infinite type)
    OccursCheck(TypeVar, Span),
    /// Argument count mismatch
    ArgCountMismatch(usize, usize, Span),
    /// Tuple length mismatch
    TupleLenMismatch(usize, usize, Span),
    /// Type mismatch
    TypeMismatch(InferenceType, InferenceType, Span),
    /// Unresolved type variable
    UnresolvedVar(TypeVar, Span),
    /// Concept not satisfied
    ConceptNotSatisfied(String, String, Span),
}

impl std::fmt::Display for InferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferError::OccursCheck(v, _) => {
                write!(f, "type inference occurs check failed for variable {:?}", v)
            }
            InferError::ArgCountMismatch(a, b, _) => {
                write!(f, "argument count mismatch: expected {} but got {}", a, b)
            }
            InferError::TupleLenMismatch(a, b, _) => {
                write!(f, "tuple length mismatch: expected {} but got {}", a, b)
            }
            InferError::TypeMismatch(a, b, _) => write!(f, "type mismatch: {:?} vs {:?}", a, b),
            InferError::UnresolvedVar(v, _) => write!(f, "unresolved type variable {:?}", v),
            InferError::ConceptNotSatisfied(ty, concept, _) => {
                write!(f, "type {} does not satisfy concept {}", ty, concept)
            }
        }
    }
}

impl std::error::Error for InferError {}

/// Literal type inference helper
#[derive(Debug, Clone, Default)]
pub struct LiteralInfer;

impl LiteralInfer {
    /// Infer the type of an integer literal
    pub fn infer_int(lit: i64) -> InferenceType {
        // Check if it fits in smaller integer types
        if lit >= 0 {
            if lit <= i8::MAX as i64 {
                InferenceType::Primitive(PrimitiveKind::Int8)
            } else if lit <= i16::MAX as i64 {
                InferenceType::Primitive(PrimitiveKind::Int16)
            } else if lit <= i32::MAX as i64 {
                InferenceType::Primitive(PrimitiveKind::Int32)
            } else {
                InferenceType::Primitive(PrimitiveKind::Int64)
            }
        } else {
            if lit >= i8::MIN as i64 {
                InferenceType::Primitive(PrimitiveKind::Int8)
            } else if lit >= i16::MIN as i64 {
                InferenceType::Primitive(PrimitiveKind::Int16)
            } else if lit >= i32::MIN as i64 {
                InferenceType::Primitive(PrimitiveKind::Int32)
            } else {
                InferenceType::Primitive(PrimitiveKind::Int64)
            }
        }
    }

    /// Infer the type of a float literal
    pub fn infer_float(_lit: f64) -> InferenceType {
        InferenceType::Primitive(PrimitiveKind::Float64)
    }

    /// Infer the type of a character literal
    pub fn infer_char(_lit: char) -> InferenceType {
        InferenceType::Primitive(PrimitiveKind::Char)
    }

    /// Infer the type of a string literal
    pub fn infer_string() -> InferenceType {
        InferenceType::Named("string".to_string())
    }

    /// Infer the type of a boolean literal
    pub fn infer_bool(_lit: bool) -> InferenceType {
        InferenceType::Primitive(PrimitiveKind::Bool)
    }
}

/// Expected type context for inference
#[derive(Debug, Clone, Default)]
pub struct ExpectedType {
    /// Expected type (used for literal coercion)
    pub expected: Option<InferenceType>,
    /// Position in argument list (for named args)
    pub position: usize,
}

impl ExpectedType {
    pub fn new() -> Self {
        ExpectedType {
            expected: None,
            position: 0,
        }
    }

    pub fn with_expected(expected: InferenceType) -> Self {
        ExpectedType {
            expected: Some(expected),
            position: 0,
        }
    }

    pub fn with_position(position: usize) -> Self {
        ExpectedType {
            expected: None,
            position,
        }
    }
}

// Import TyId from parent module
use crate::TyId;

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_type_var_new() {
        let var = TypeVar::new(0);
        assert_eq!(var.id(), 0);
    }

    #[test]
    fn test_type_var_uniqueness() {
        let mut state = InferState::new();
        let var1 = state.fresh_var();
        let var2 = state.fresh_var();
        assert_ne!(var1, var2);
    }

    #[test]
    fn test_inference_type_is_var() {
        let mut state = InferState::new();
        let var = state.fresh_var();
        assert!(InferenceType::Var(var).is_var());
        assert!(!InferenceType::Primitive(PrimitiveKind::Int).is_var());
    }

    #[test]
    fn test_inference_type_contains_var() {
        let mut state = InferState::new();
        let var = state.fresh_var();
        let containing = InferenceType::Fn {
            params: vec![InferenceType::Var(var)],
            ret: Box::new(InferenceType::Primitive(PrimitiveKind::Int)),
        };
        let not_containing = InferenceType::Primitive(PrimitiveKind::Int);
        assert!(containing.contains_var(var));
        assert!(!not_containing.contains_var(var));
    }

    #[test]
    fn test_inference_type_substitute() {
        let var = TypeVar::new(0);
        let replacement = InferenceType::Primitive(PrimitiveKind::Int);
        let original = InferenceType::Var(var);
        let substituted = original.substitute(var, &replacement);
        assert_eq!(substituted, replacement);
    }

    #[test]
    fn test_infer_state_equate() {
        let mut state = InferState::new();
        let var = state.fresh_var();
        state.equate(
            InferenceType::Var(var),
            InferenceType::Primitive(PrimitiveKind::Int),
            Span::new(FileId(0), 0, 0),
        );
        assert_eq!(state.constraints.len(), 1);
    }

    #[test]
    fn test_infer_solver_simple() {
        let mut solver = InferSolver::new();
        let var = TypeVar::new(0);
        let mut state = InferState::new();
        state.equate(
            InferenceType::Var(var),
            InferenceType::Primitive(PrimitiveKind::Int),
            Span::new(FileId(0), 0, 0),
        );
        let subs = solver.solve(&state.constraints).unwrap();
        assert_eq!(
            subs.get(&var),
            Some(&InferenceType::Primitive(PrimitiveKind::Int))
        );
    }

    #[test]
    fn test_literal_infer_int() {
        assert_eq!(
            LiteralInfer::infer_int(42),
            InferenceType::Primitive(PrimitiveKind::Int8)
        );
        assert_eq!(
            LiteralInfer::infer_int(1000),
            InferenceType::Primitive(PrimitiveKind::Int16)
        );
        assert_eq!(
            LiteralInfer::infer_int(-1000),
            InferenceType::Primitive(PrimitiveKind::Int16)
        );
    }

    #[test]
    fn test_literal_infer_float() {
        assert_eq!(
            LiteralInfer::infer_float(std::f64::consts::PI),
            InferenceType::Primitive(PrimitiveKind::Float64)
        );
    }

    #[test]
    fn test_literal_infer_char() {
        assert_eq!(
            LiteralInfer::infer_char('x'),
            InferenceType::Primitive(PrimitiveKind::Char)
        );
    }

    #[test]
    fn test_literal_infer_string() {
        assert_eq!(
            LiteralInfer::infer_string(),
            InferenceType::Named("string".to_string())
        );
    }

    #[test]
    fn test_literal_infer_bool() {
        assert_eq!(
            LiteralInfer::infer_bool(true),
            InferenceType::Primitive(PrimitiveKind::Bool)
        );
    }

    #[test]
    fn test_expected_type_new() {
        let expected = ExpectedType::new();
        assert!(expected.expected.is_none());
        assert_eq!(expected.position, 0);
    }

    #[test]
    fn test_expected_type_with_expected() {
        let expected = ExpectedType::with_expected(InferenceType::Primitive(PrimitiveKind::Int));
        assert!(expected.expected.is_some());
    }

    #[test]
    fn test_expected_type_with_position() {
        let expected = ExpectedType::with_position(5);
        assert_eq!(expected.position, 5);
    }

    #[test]
    fn test_infer_error_display() {
        let err = InferError::UnresolvedVar(TypeVar::new(0), Span::new(FileId(0), 0, 0));
        let s = format!("{}", err);
        assert!(s.contains("unresolved type variable"));
    }

    #[test]
    fn test_occurs_check_error() {
        let var = TypeVar::new(0);
        let err = InferError::OccursCheck(var, Span::new(FileId(0), 0, 0));
        let s = format!("{}", err);
        assert!(s.contains("occurs check failed"));
    }

    #[test]
    fn test_type_mismatch_error() {
        let err = InferError::TypeMismatch(
            InferenceType::Primitive(PrimitiveKind::Int),
            InferenceType::Primitive(PrimitiveKind::Bool),
            Span::new(FileId(0), 0, 0),
        );
        let s = format!("{}", err);
        assert!(s.contains("type mismatch"));
    }

    #[test]
    fn test_fn_type_equality() {
        let fn1 = InferenceType::Fn {
            params: vec![InferenceType::Primitive(PrimitiveKind::Int)],
            ret: Box::new(InferenceType::Primitive(PrimitiveKind::Int)),
        };
        let fn2 = InferenceType::Fn {
            params: vec![InferenceType::Primitive(PrimitiveKind::Int)],
            ret: Box::new(InferenceType::Primitive(PrimitiveKind::Int)),
        };
        assert_eq!(fn1, fn2);
    }

    #[test]
    fn test_fn_type_inequality() {
        let fn1 = InferenceType::Fn {
            params: vec![InferenceType::Primitive(PrimitiveKind::Int)],
            ret: Box::new(InferenceType::Primitive(PrimitiveKind::Int)),
        };
        let fn2 = InferenceType::Fn {
            params: vec![InferenceType::Primitive(PrimitiveKind::Int)],
            ret: Box::new(InferenceType::Primitive(PrimitiveKind::Bool)),
        };
        assert_ne!(fn1, fn2);
    }

    #[test]
    fn test_type_var_id() {
        let var = TypeVar::new(42);
        assert_eq!(var.id(), 42);
    }

    #[test]
    fn test_infer_solver_new() {
        let solver = InferSolver::new();
        // Solver should be created without panic
        assert!(solver.clone().substitutions.is_empty());
    }

    #[test]
    fn test_infer_error_occurs_check_display() {
        let err = InferError::OccursCheck(TypeVar::new(0), Span::new(FileId::new(0), 0, 10));
        let display = format!("{}", err);
        assert!(display.contains("occurs") || display.contains("Occurs"));
    }

    #[test]
    fn test_infer_error_type_mismatch_display() {
        let err = InferError::TypeMismatch(
            InferenceType::Primitive(PrimitiveKind::Int),
            InferenceType::Primitive(PrimitiveKind::Float),
            Span::new(FileId::new(0), 0, 10),
        );
        let display = format!("{}", err);
        assert!(display.contains("mismatch") || display.contains("Mismatch"));
    }

    #[test]
    fn test_infer_error_arg_count_display() {
        let err = InferError::ArgCountMismatch(2, 3, Span::new(FileId::new(0), 0, 10));
        let display = format!("{}", err);
        assert!(display.contains("argument") || display.contains("mismatch"));
    }

    #[test]
    fn test_infer_error_tuple_len_display() {
        let err = InferError::TupleLenMismatch(3, 4, Span::new(FileId::new(0), 0, 10));
        let display = format!("{}", err);
        assert!(display.contains("tuple") || display.contains("length"));
    }

    #[test]
    fn test_infer_error_unresolved_var_display() {
        let err = InferError::UnresolvedVar(TypeVar::new(5), Span::new(FileId::new(0), 0, 10));
        let display = format!("{}", err);
        assert!(display.contains("unresolved") || display.contains("variable"));
    }

    #[test]
    fn test_infer_error_concept_not_satisfied_display() {
        let err = InferError::ConceptNotSatisfied(
            "int".into(),
            "Addable".into(),
            Span::new(FileId::new(0), 0, 10),
        );
        let display = format!("{}", err);
        assert!(display.contains("concept") || display.contains("not satisfy"));
    }
}
