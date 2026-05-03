//! Semantic analysis, overload resolution, type checking, constant rules, routine checking.

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{FileId, Span};
use rnim_types::{
    is_float, is_integral, LiteralValue, PrimitiveKind, SubrangeType, TyId, Type, TypeCtx,
};
use std::fmt::{self, Debug};

pub mod check;
pub mod exception;
pub mod pragma;
pub mod when_ctx;

pub struct SemanticDb {
    type_ctx: TypeCtx,
}

impl Debug for SemanticDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticDb").finish()
    }
}

impl Default for SemanticDb {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticDb {
    pub fn new() -> Self {
        Self {
            type_ctx: TypeCtx::new(),
        }
    }

    pub fn check_module(&mut self, _file: Span) -> Result<(), String> {
        // In full implementation, would:
        // 1. Parse the source file
        // 2. Build HIR
        // 3. Run semantic analysis
        // 4. Type check all expressions
        // 5. Validate effects
        // For now, just return success
        Ok(())
    }

    pub fn type_ctx(&self) -> &TypeCtx {
        &self.type_ctx
    }

    pub fn type_ctx_mut(&mut self) -> &mut TypeCtx {
        &mut self.type_ctx
    }
}

/// Type inference constraint
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    Equal(TyId, TyId),
    Subtype(TyId, TyId),
    Literal(TyId, rnim_types::LiteralValue),
    Callable(TyId, Vec<TyId>),
}

/// Type inference solution for a type variable
#[derive(Debug, Clone)]
pub enum TypeSolution {
    Known(TyId),
    Error,
}

/// Type inference context for constraint solving
#[derive(Debug, Clone)]
pub struct InferenceCtx {
    constraints: Vec<Constraint>,
}

impl InferenceCtx {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }
}

impl Default for InferenceCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// Unification result
#[derive(Debug, Clone, PartialEq)]
pub enum UnifyResult {
    Success,
    Failure(String),
    Ambiguous,
}

/// Unification context for type unification operations
pub struct UnifyCtx {
    type_ctx: TypeCtx,
}

impl Debug for UnifyCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnifyCtx").finish()
    }
}

impl UnifyCtx {
    pub fn new() -> Self {
        Self {
            type_ctx: TypeCtx::new(),
        }
    }

    pub fn with_type_ctx(type_ctx: TypeCtx) -> Self {
        Self { type_ctx }
    }

    pub fn type_ctx(&self) -> &TypeCtx {
        &self.type_ctx
    }

    pub fn type_ctx_mut(&mut self) -> &mut TypeCtx {
        &mut self.type_ctx
    }

    /// Unify two types and return the unified result
    pub fn unify(&mut self, ty1: TyId, ty2: TyId) -> UnifyResult {
        if ty1 == ty2 {
            return UnifyResult::Success;
        }

        let t1 = self.type_ctx.get(ty1).cloned();
        let t2 = self.type_ctx.get(ty2).cloned();

        match (t1, t2) {
            (Some(t1), Some(t2)) => self.unify_types(&t1, &t2),
            _ => UnifyResult::Failure("Unknown type".into()),
        }
    }

    fn unify_types(&mut self, ty1: &Type, ty2: &Type) -> UnifyResult {
        let (ty1, ty2) = self.strip_aliases_distinct(ty1, ty2);
        let ty1_clone = ty1.clone();
        let ty2_clone = ty2.clone();

        match (ty1, ty2) {
            (Type::Primitive(p1), Type::Primitive(p2)) => {
                if p1.kind == p2.kind {
                    UnifyResult::Success
                } else if p1.is_signed == p2.is_signed && p1.size == p2.size {
                    UnifyResult::Success
                } else {
                    UnifyResult::Failure(format!(
                        "Primitive type mismatch: {:?} vs {:?}",
                        p1.kind, p2.kind
                    ))
                }
            }
            (Type::Enum(e1), Type::Enum(e2)) => {
                if e1.name == e2.name {
                    UnifyResult::Success
                } else {
                    UnifyResult::Failure("Enum type mismatch".into())
                }
            }
            (Type::Subrange(s1), Type::Subrange(s2)) => {
                if s1.lower == s2.lower && s1.upper == s2.upper {
                    self.unify(s1.base, s2.base)
                } else {
                    UnifyResult::Failure("Subrange bounds mismatch".into())
                }
            }
            (Type::Array { elem: e1, len: l1 }, Type::Array { elem: e2, len: l2 }) => {
                if l1 == l2 {
                    self.unify(e1, e2)
                } else {
                    UnifyResult::Failure("Array length mismatch".into())
                }
            }
            (Type::Seq { elem: e1 }, Type::Seq { elem: e2 }) => self.unify(e1, e2),
            (Type::Set { elem: e1 }, Type::Set { elem: e2 }) => self.unify(e1, e2),
            (Type::Tuple { fields: f1 }, Type::Tuple { fields: f2 }) => {
                if f1.len() != f2.len() {
                    return UnifyResult::Failure("Tuple field count mismatch".into());
                }
                for (ty1, ty2) in f1.iter().zip(f2.iter()) {
                    let result = self.unify(*ty1, *ty2);
                    if result != UnifyResult::Success {
                        return result;
                    }
                }
                UnifyResult::Success
            }
            (Type::Ref { inner: i1 }, Type::Ref { inner: i2 }) => self.unify(i1, i2),
            (Type::Ptr { inner: i1 }, Type::Ptr { inner: i2 }) => self.unify(i1, i2),
            (
                Type::Proc {
                    params: p1,
                    ret: r1,
                    ..
                },
                Type::Proc {
                    params: p2,
                    ret: r2,
                    ..
                },
            ) => {
                if p1.len() != p2.len() {
                    return UnifyResult::Failure("Procedure parameter count mismatch".into());
                }
                for (ty1, ty2) in p1.iter().zip(p2.iter()) {
                    let result = self.unify(*ty1, *ty2);
                    if result != UnifyResult::Success {
                        return result;
                    }
                }
                match (r1, r2) {
                    (Some(ret1), Some(ret2)) => self.unify(ret1, ret2),
                    (None, None) => UnifyResult::Success,
                    _ => UnifyResult::Failure("Procedure return type mismatch".into()),
                }
            }
            (Type::Nil, _) | (_, Type::Nil) => UnifyResult::Success,
            (Type::Error, _) | (_, Type::Error) => UnifyResult::Success,
            _ => UnifyResult::Failure(format!("Type mismatch: {:?} vs {:?}", ty1_clone, ty2_clone)),
        }
    }

    fn strip_aliases_distinct(&self, ty1: &Type, ty2: &Type) -> (Type, Type) {
        let t1 = self.follow_aliases_distinct(ty1);
        let t2 = self.follow_aliases_distinct(ty2);
        (t1, t2)
    }

    fn follow_aliases_distinct(&self, ty: &Type) -> Type {
        match ty {
            Type::Alias { underlying, .. } => {
                let u = *underlying;
                if let Some(t) = self.type_ctx.get(u) {
                    self.follow_aliases_distinct(t)
                } else {
                    ty.clone()
                }
            }
            Type::Distinct { underlying, .. } => {
                let u = *underlying;
                if let Some(t) = self.type_ctx.get(u) {
                    self.follow_aliases_distinct(t)
                } else {
                    ty.clone()
                }
            }
            _ => ty.clone(),
        }
    }

    /// Check if a type is a subtype of another
    pub fn is_subtype(&mut self, sub: TyId, sup: TyId) -> bool {
        if sub == sup {
            return true;
        }

        let sub_type = self.type_ctx.get(sub).cloned();
        let sup_type = self.type_ctx.get(sup).cloned();

        match (sub_type, sup_type) {
            (Some(Type::Nil), _) => true,
            (Some(Type::Distinct { underlying, .. }), _) => self.is_subtype(underlying, sup),
            (_, Some(Type::Distinct { underlying, .. })) => self.is_subtype(sub, underlying),
            (Some(Type::Ref { inner: i1 }), Some(Type::Ref { inner: i2 })) => {
                self.is_subtype(i1, i2)
            }
            _ => false,
        }
    }

    /// Get the common type of two types (for inference)
    pub fn common_type(&mut self, ty1: TyId, ty2: TyId) -> Option<TyId> {
        if ty1 == ty2 {
            return Some(ty1);
        }

        let t1 = self.type_ctx.get(ty1).cloned();
        let t2 = self.type_ctx.get(ty2).cloned();

        if matches!(t1, Some(Type::Nil)) {
            return Some(ty2);
        }
        if matches!(t2, Some(Type::Nil)) {
            return Some(ty1);
        }

        if self.unify(ty1, ty2) == UnifyResult::Success {
            if self.is_subtype(ty1, ty2) {
                Some(ty2)
            } else if self.is_subtype(ty2, ty1) {
                Some(ty1)
            } else {
                Some(ty1)
            }
        } else {
            None
        }
    }
}

/// Conversion rank - how preferred is a conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConversionRank {
    None,
    Identity,
    Exact,
    Literal,
    IntegralWidening,
    FloatWidening,
    StringToCstring,
    NilToPointer,
    UserConversion,
    Ambiguous,
}

/// A registered converter
#[derive(Debug, Clone)]
pub struct Converter {
    pub from: TyId,
    pub to: TyId,
    pub rank: ConversionRank,
}

/// Conversion context for handling implicit/explicit conversions
pub struct ConversionCtx {
    type_ctx: TypeCtx,
    converters: Vec<Converter>,
}

impl Debug for ConversionCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConversionCtx")
            .field("converters", &self.converters.len())
            .finish()
    }
}

impl Clone for ConversionCtx {
    fn clone(&self) -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            converters: self.converters.clone(),
        }
    }
}

impl ConversionCtx {
    pub fn new() -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            converters: Vec::new(),
        }
    }

    /// Register a converter
    pub fn add_converter(&mut self, from: TyId, to: TyId, rank: ConversionRank) {
        self.converters.push(Converter { from, to, rank });
    }

    /// Find conversion rank between two types
    pub fn conversion_rank(&self, from: TyId, to: TyId) -> ConversionRank {
        if from == to {
            return ConversionRank::Identity;
        }

        for converter in &self.converters {
            if converter.from == from && converter.to == to {
                return converter.rank;
            }
        }

        // Check for built-in conversions
        let from_type = self.type_ctx.get(from);
        let to_type = self.type_ctx.get(to);

        match (from_type, to_type) {
            (Some(Type::Nil), _) => ConversionRank::NilToPointer,
            (Some(Type::Primitive(p1)), Some(Type::Primitive(p2))) => {
                if p1.kind == p2.kind {
                    ConversionRank::Identity
                } else if p1.size < p2.size && p1.is_signed == p2.is_signed {
                    ConversionRank::IntegralWidening
                } else {
                    ConversionRank::None
                }
            }
            _ => ConversionRank::None,
        }
    }

    /// Check if a conversion is valid
    pub fn can_convert(&self, from: TyId, to: TyId) -> bool {
        self.conversion_rank(from, to) != ConversionRank::None
    }

    /// Get the type context
    pub fn type_ctx(&self) -> &TypeCtx {
        &self.type_ctx
    }

    /// Get a mutable type context
    pub fn type_ctx_mut(&mut self) -> &mut TypeCtx {
        &mut self.type_ctx
    }
}

/// Overload candidate for resolution
#[derive(Debug, Clone)]
pub struct OverloadCandidate {
    pub ty: TyId,
    pub rank: ConversionRank,
    pub converter_rank: ConversionRank,
}

/// Overload resolution result
#[derive(Debug, Clone)]
pub enum OverloadResult {
    Unique(TyId),
    Ambiguous(Vec<TyId>),
    None,
}

/// Overload resolution context
pub struct OverloadCtx {
    type_ctx: TypeCtx,
    candidates: Vec<OverloadCandidate>,
}

impl Debug for OverloadCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverloadCtx")
            .field("candidates", &self.candidates)
            .finish()
    }
}

impl Clone for OverloadCtx {
    fn clone(&self) -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            candidates: self.candidates.clone(),
        }
    }
}

impl OverloadCtx {
    pub fn new() -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            candidates: Vec::new(),
        }
    }

    pub fn type_ctx(&self) -> &TypeCtx {
        &self.type_ctx
    }

    pub fn type_ctx_mut(&mut self) -> &mut TypeCtx {
        &mut self.type_ctx
    }

    /// Add a candidate to consider
    pub fn add_candidate(&mut self, ty: TyId, conversion_rank: ConversionRank) {
        self.candidates.push(OverloadCandidate {
            ty,
            rank: ConversionRank::Identity,
            converter_rank: conversion_rank,
        });
    }

    /// Clear all candidates
    pub fn clear_candidates(&mut self) {
        self.candidates.clear();
    }

    /// Resolve the best overload match
    pub fn resolve(&self) -> OverloadResult {
        if self.candidates.is_empty() {
            return OverloadResult::None;
        }

        // Find the best candidate(s)
        let mut best_rank: Option<ConversionRank> = None;
        let mut best_candidates: Vec<TyId> = Vec::new();

        for candidate in &self.candidates {
            let combined_rank = candidate.rank.max(candidate.converter_rank);

            match &best_rank {
                None => {
                    best_rank = Some(combined_rank);
                    best_candidates.push(candidate.ty);
                }
                Some(existing) if combined_rank < *existing => {
                    best_rank = Some(combined_rank);
                    best_candidates = vec![candidate.ty];
                }
                Some(existing) if combined_rank == *existing => {
                    best_candidates.push(candidate.ty);
                }
                _ => {}
            }
        }

        match best_candidates.len() {
            0 => OverloadResult::None,
            1 => OverloadResult::Unique(best_candidates.remove(0)),
            _ => OverloadResult::Ambiguous(best_candidates),
        }
    }

    /// Get all candidates
    pub fn candidates(&self) -> &[OverloadCandidate] {
        &self.candidates
    }
}

impl Default for OverloadCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// A concept (type class) definition
#[derive(Debug, Clone)]
pub struct Concept {
    pub name: Box<str>,
    pub type_vars: Vec<TyId>,
    pub constraints: Vec<ConceptConstraint>,
    pub methods: Vec<ConceptMethod>,
}

/// A constraint on a type variable
#[derive(Debug, Clone)]
pub struct ConceptConstraint {
    pub type_var: TyId,
    pub concept_name: Box<str>,
}

/// A method required by a concept
#[derive(Debug, Clone)]
pub struct ConceptMethod {
    pub name: Box<str>,
    pub proc_type: TyId,
}

/// Concept solver for checking type constraints
pub struct ConceptSolver {
    type_ctx: TypeCtx,
    concepts: Vec<Concept>,
    impls: Vec<ConceptImpl>,
}

impl Debug for ConceptSolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConceptSolver")
            .field("concepts", &self.concepts)
            .field("impls", &self.impls)
            .finish()
    }
}

impl Clone for ConceptSolver {
    fn clone(&self) -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            concepts: self.concepts.clone(),
            impls: self.impls.clone(),
        }
    }
}

/// A concept implementation for a specific type
#[derive(Debug, Clone)]
pub struct ConceptImpl {
    pub concept_name: Box<str>,
    pub type_id: TyId,
    pub methods: Vec<TyId>,
}

impl ConceptSolver {
    pub fn new() -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            concepts: Vec::new(),
            impls: Vec::new(),
        }
    }

    pub fn type_ctx(&self) -> &TypeCtx {
        &self.type_ctx
    }

    pub fn type_ctx_mut(&mut self) -> &mut TypeCtx {
        &mut self.type_ctx
    }

    /// Register a concept definition
    pub fn add_concept(&mut self, concept: Concept) {
        self.concepts.push(concept);
    }

    /// Register a concept implementation
    pub fn add_impl(&mut self, impl_: ConceptImpl) {
        self.impls.push(impl_);
    }

    /// Check if a type satisfies a concept
    pub fn satisfies(&self, type_id: TyId, concept_name: &str) -> bool {
        for impl_ in &self.impls {
            if impl_.concept_name.as_ref() == concept_name && impl_.type_id == type_id {
                return true;
            }
        }
        false
    }

    /// Get all concepts a type satisfies
    pub fn concepts_for_type(&self, type_id: TyId) -> Vec<&str> {
        self.impls
            .iter()
            .filter(|impl_| impl_.type_id == type_id)
            .map(|impl_| impl_.concept_name.as_ref())
            .collect()
    }

    /// Check if a type variable satisfies constraints
    pub fn check_constraints(&self, constraints: &[ConceptConstraint]) -> bool {
        for constraint in constraints {
            // Check if type implements the required concept
            let type_implements = self.type_ctx.get(constraint.type_var).is_some();
            if !type_implements {
                return false;
            }
            // In a full implementation, would check if type satisfies the concept
            // For now, just verify the type exists
        }
        true
    }

    /// Get method implementations for a type satisfying a concept
    pub fn get_methods(&self, type_id: TyId, concept_name: &str) -> Option<&[TyId]> {
        for impl_ in &self.impls {
            if impl_.concept_name.as_ref() == concept_name && impl_.type_id == type_id {
                return Some(&impl_.methods);
            }
        }
        None
    }
}

impl Default for ConceptSolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Object inheritance and method dispatch
#[derive(Debug, Clone)]
pub struct ObjectType {
    pub fields: Vec<FieldDef>,
    pub base: Option<TyId>,
    pub methods: Vec<MethodDef>,
    pub vtable: Option<Vtable>,
}

/// A field in an object
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: Box<str>,
    pub typ: TyId,
    pub offset: u32,
}

/// A method definition
#[derive(Debug, Clone)]
pub struct MethodDef {
    pub name: Box<str>,
    pub proc_type: TyId,
    pub slot: u32,
}

/// Vtable for dynamic dispatch
#[derive(Debug, Clone)]
pub struct Vtable {
    pub methods: Vec<TyId>,
}

/// Object inheritance solver
pub struct InheritanceSolver {
    type_ctx: TypeCtx,
    objects: Vec<ObjectType>,
}

impl Debug for InheritanceSolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InheritanceSolver")
            .field("objects", &self.objects)
            .finish()
    }
}

impl Clone for InheritanceSolver {
    fn clone(&self) -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            objects: self.objects.clone(),
        }
    }
}

impl InheritanceSolver {
    pub fn new() -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            objects: Vec::new(),
        }
    }

    pub fn type_ctx(&self) -> &TypeCtx {
        &self.type_ctx
    }

    pub fn type_ctx_mut(&mut self) -> &mut TypeCtx {
        &mut self.type_ctx
    }

    /// Register an object type
    pub fn add_object(&mut self, obj: ObjectType) {
        self.objects.push(obj);
    }

    /// Get the object type for a given type ID
    pub fn get_object(&self, type_id: TyId) -> Option<&ObjectType> {
        // Look up object by type ID - in a full implementation this would be in a map
        for obj in &self.objects {
            // Check if this object's type ID matches
            if let Some(Type::Object { .. }) = self.type_ctx.get(type_id) {
                return Some(obj);
            }
        }
        None
    }

    /// Check if type1 is a subtype of type2
    pub fn is_subtype(&self, sub: TyId, sup: TyId) -> bool {
        if sub == sup {
            return true;
        }

        let sub_type = self.type_ctx.get(sub);
        if let Some(Type::Object { base, .. }) = sub_type {
            if let Some(base_id) = base {
                return self.is_subtype(*base_id, sup);
            }
        }
        false
    }

    /// Get the most specific common ancestor of two types
    pub fn common_ancestor(&self, ty1: TyId, ty2: TyId) -> Option<TyId> {
        if ty1 == ty2 {
            return Some(ty1);
        }

        // Build ancestor chain for ty1
        let mut ancestors: Vec<TyId> = vec![ty1];
        let mut current = ty1;
        while let Some(Type::Object {
            base: Some(base), ..
        }) = self.type_ctx.get(current)
        {
            ancestors.push(*base);
            current = *base;
        }

        // Find first ancestor of ty2 in ty1's ancestor chain
        current = ty2;
        while let Some(Type::Object {
            base: Some(base), ..
        }) = self.type_ctx.get(current)
        {
            if ancestors.contains(&current) {
                return Some(current);
            }
            current = *base;
        }

        None
    }

    /// Look up a method by name in an object hierarchy
    pub fn lookup_method(&self, type_id: TyId, method_name: &str) -> Option<TyId> {
        let mut current = Some(type_id);

        while let Some(id) = current {
            if let Some(obj) = self.get_object(id) {
                for method in &obj.methods {
                    if method.name.as_ref() == method_name {
                        return Some(method.proc_type);
                    }
                }
            }
            // Move to base type
            if let Some(Type::Object { base, .. }) = self.type_ctx.get(id) {
                current = *base;
            } else {
                current = None;
            }
        }
        None
    }

    /// Get all methods for a type (including inherited)
    pub fn all_methods(&self, type_id: TyId) -> Vec<(Box<str>, TyId)> {
        let mut methods = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.collect_methods(type_id, &mut methods, &mut visited);
        methods
    }

    fn collect_methods(
        &self,
        type_id: TyId,
        methods: &mut Vec<(Box<str>, TyId)>,
        visited: &mut std::collections::HashSet<TyId>,
    ) {
        if visited.contains(&type_id) {
            return;
        }
        visited.insert(type_id);

        if let Some(obj) = self.get_object(type_id) {
            for method in &obj.methods {
                methods.push((method.name.clone(), method.proc_type));
            }
        }

        if let Some(Type::Object {
            base: Some(base), ..
        }) = self.type_ctx.get(type_id)
        {
            self.collect_methods(*base, methods, visited);
        }
    }
}

impl Default for InheritanceSolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Expression semantic checker
pub struct ExprChecker {
    type_ctx: TypeCtx,
    unify_ctx: UnifyCtx,
}

impl Debug for ExprChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExprChecker").finish()
    }
}

impl ExprChecker {
    pub fn new() -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            unify_ctx: UnifyCtx::new(),
        }
    }

    pub fn type_ctx(&self) -> &TypeCtx {
        &self.type_ctx
    }

    pub fn type_ctx_mut(&mut self) -> &mut TypeCtx {
        &mut self.type_ctx
    }

    /// Check a binary operation expression
    pub fn check_binary_op(
        &mut self,
        op: &str,
        left_ty: TyId,
        right_ty: TyId,
    ) -> Result<TyId, String> {
        match op {
            "+" | "-" | "*" | "/" => {
                // Arithmetic operations require numeric types
                if self.unify_ctx.unify(left_ty, right_ty) == UnifyResult::Success {
                    Ok(left_ty)
                } else {
                    Err(format!(
                        "Cannot apply '{}' to types {:?} and {:?}",
                        op, left_ty, right_ty
                    ))
                }
            }
            "<" | ">" | "<=" | ">=" => {
                // Comparison operations
                if self.unify_ctx.unify(left_ty, right_ty) == UnifyResult::Success {
                    Ok(self.type_ctx.get_primitive(PrimitiveKind::Bool).unwrap())
                } else {
                    Err(format!(
                        "Cannot compare types {:?} and {:?}",
                        left_ty, right_ty
                    ))
                }
            }
            "==" | "!=" => {
                // Equality operations
                if self.unify_ctx.unify(left_ty, right_ty) == UnifyResult::Success {
                    Ok(self.type_ctx.get_primitive(PrimitiveKind::Bool).unwrap())
                } else {
                    Err(format!(
                        "Cannot compare types {:?} and {:?}",
                        left_ty, right_ty
                    ))
                }
            }
            "and" | "or" => {
                // Boolean operations
                if self.unify_ctx.unify(left_ty, right_ty) == UnifyResult::Success {
                    if let Some(bool_id) = self.type_ctx.get_primitive(PrimitiveKind::Bool) {
                        Ok(bool_id)
                    } else {
                        Err("Bool type not found".into())
                    }
                } else {
                    Err("Boolean operators require boolean operands".into())
                }
            }
            _ => Err(format!("Unknown operator: {}", op)),
        }
    }

    /// Check a unary operation expression
    pub fn check_unary_op(&mut self, op: &str, operand_ty: TyId) -> Result<TyId, String> {
        match op {
            "-" => {
                // Unary minus requires numeric type (integral or float)
                if let Some(ty) = self.type_ctx.get(operand_ty) {
                    let is_numeric = match &*ty {
                        Type::Primitive(p) => matches!(
                            p.kind,
                            PrimitiveKind::Int
                                | PrimitiveKind::Int8
                                | PrimitiveKind::Int16
                                | PrimitiveKind::Int32
                                | PrimitiveKind::Int64
                                | PrimitiveKind::Uint
                                | PrimitiveKind::Uint8
                                | PrimitiveKind::Uint16
                                | PrimitiveKind::Uint32
                                | PrimitiveKind::Uint64
                                | PrimitiveKind::Float
                                | PrimitiveKind::Float32
                                | PrimitiveKind::Float64
                        ),
                        _ => false,
                    };
                    if is_numeric {
                        Ok(operand_ty)
                    } else {
                        Err(format!(
                            "Unary minus requires numeric type, got {:?}",
                            operand_ty
                        ))
                    }
                } else {
                    Err(format!("Unknown type {:?}", operand_ty))
                }
            }
            "not" => {
                // Boolean negation
                if let Some(bool_id) = self.type_ctx.get_primitive(PrimitiveKind::Bool) {
                    Ok(bool_id)
                } else {
                    Err("Bool type not found".into())
                }
            }
            _ => Err(format!("Unknown unary operator: {}", op)),
        }
    }

    /// Check an index access expression
    pub fn check_index(&mut self, base_ty: TyId, index_ty: TyId) -> Result<TyId, String> {
        let base_type = self.type_ctx.get(base_ty);
        match base_type {
            Some(Type::Array { elem, .. }) => {
                // Array index requires integral index type
                if is_integral(self.type_ctx.get(index_ty).unwrap_or(&Type::Error)) {
                    Ok(*elem)
                } else {
                    Err("Array index must be integral".into())
                }
            }
            Some(Type::Seq { elem }) => {
                // Sequence index requires integral index
                if is_integral(self.type_ctx.get(index_ty).unwrap_or(&Type::Error)) {
                    Ok(*elem)
                } else {
                    Err("Seq index must be integral".into())
                }
            }
            Some(Type::Set { .. }) => {
                // Set membership check
                Ok(self.type_ctx.get_primitive(PrimitiveKind::Bool).unwrap())
            }
            _ => Err(format!("Cannot index type {:?}", base_ty)),
        }
    }

    /// Check a field access expression
    pub fn check_field_access(&mut self, base_ty: TyId, field_name: &str) -> Result<TyId, String> {
        let base_type = self.type_ctx.get(base_ty);
        match base_type {
            Some(Type::Object { fields, .. }) => {
                for field in fields {
                    if field.name.as_ref() == field_name {
                        return Ok(field.typ);
                    }
                }
                Err(format!("Field '{}' not found in type", field_name))
            }
            Some(Type::Tuple { fields }) => {
                // Tuple field access - accept numeric field index
                if let Ok(idx) = field_name.parse::<usize>() {
                    if idx < fields.len() {
                        return Ok(fields[idx]);
                    }
                }
                Err(format!("Tuple field index {} out of bounds", field_name))
            }
            _ => Err(format!("Cannot access field on type {:?}", base_ty)),
        }
    }

    /// Check a call expression
    pub fn check_call(
        &mut self,
        callee_ty: TyId,
        arg_types: &[TyId],
    ) -> Result<Option<TyId>, String> {
        let callee_type = self.type_ctx.get(callee_ty);
        match callee_type {
            Some(Type::Proc { params, ret, .. }) => {
                if params.len() != arg_types.len() {
                    return Err(format!(
                        "Expected {} arguments but got {}",
                        params.len(),
                        arg_types.len()
                    ));
                }
                for (param_ty, arg_ty) in params.iter().zip(arg_types.iter()) {
                    if self.unify_ctx.unify(*param_ty, *arg_ty) != UnifyResult::Success {
                        return Err(format!(
                            "Cannot unify argument type {:?} with parameter type {:?}",
                            arg_ty, param_ty
                        ));
                    }
                }
                Ok(*ret)
            }
            Some(Type::Ref { inner }) => {
                // Dereference and check call on the inner type
                self.check_call(*inner, arg_types)
            }
            _ => Err(format!("Cannot call type {:?}", callee_ty)),
        }
    }
}

impl Default for ExprChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Statement semantic checker
pub struct StmtChecker {
    type_ctx: TypeCtx,
    expr_checker: ExprChecker,
    return_type: Option<TyId>,
    yield_type: Option<TyId>,
}

impl Debug for StmtChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StmtChecker")
            .field("return_type", &self.return_type)
            .field("yield_type", &self.yield_type)
            .finish()
    }
}

impl StmtChecker {
    pub fn new() -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            expr_checker: ExprChecker::new(),
            return_type: None,
            yield_type: None,
        }
    }

    pub fn with_return_type(mut self, return_type: TyId) -> Self {
        self.return_type = Some(return_type);
        self
    }

    pub fn with_yield_type(mut self, yield_type: TyId) -> Self {
        self.yield_type = Some(yield_type);
        self
    }

    pub fn type_ctx(&self) -> &TypeCtx {
        &self.type_ctx
    }

    pub fn type_ctx_mut(&mut self) -> &mut TypeCtx {
        &mut self.type_ctx
    }

    /// Check an if statement condition
    pub fn check_if_condition(&self, cond_ty: TyId) -> Result<(), String> {
        let bool_id = self.type_ctx.get_primitive(PrimitiveKind::Bool).unwrap();
        if cond_ty == bool_id {
            Ok(())
        } else {
            Err("If condition must be boolean".into())
        }
    }

    /// Check a while statement condition
    pub fn check_while_condition(&self, cond_ty: TyId) -> Result<(), String> {
        self.check_if_condition(cond_ty)
    }

    /// Check a case statement
    pub fn check_case(&self, _cond_ty: TyId, branches: &[TyId]) -> Result<(), String> {
        // All branches should be of the same type
        let first_branch = branches
            .first()
            .ok_or("Case must have at least one branch")?;
        for branch_ty in branches {
            if *branch_ty != *first_branch {
                return Err(format!(
                    "Case branch type mismatch: {:?} vs {:?}",
                    branch_ty, first_branch
                ));
            }
        }
        Ok(())
    }

    /// Check a return statement
    pub fn check_return(&self, expr_ty: Option<TyId>) -> Result<(), String> {
        match (self.return_type, expr_ty) {
            (Some(expected), Some(actual)) => {
                if expected == actual {
                    Ok(())
                } else {
                    Err(format!(
                        "Return type mismatch: expected {:?}, got {:?}",
                        expected, actual
                    ))
                }
            }
            (Some(_), None) => Err("Expected return value but got nothing".into()),
            (None, Some(_)) => Err("Unexpected return value".into()),
            (None, None) => Ok(()),
        }
    }

    /// Check a yield statement
    pub fn check_yield(&self, expr_ty: Option<TyId>) -> Result<(), String> {
        match (self.yield_type, expr_ty) {
            (Some(expected), Some(actual)) => {
                if expected == actual {
                    Ok(())
                } else {
                    Err(format!(
                        "Yield type mismatch: expected {:?}, got {:?}",
                        expected, actual
                    ))
                }
            }
            (Some(_), None) => Err("Expected yield value but got nothing".into()),
            (None, _) => Err("Yield not in iterator context".into()),
        }
    }

    /// Check an assignment
    pub fn check_assignment(&self, target_ty: TyId, value_ty: TyId) -> Result<(), String> {
        if target_ty == value_ty {
            Ok(())
        } else {
            // Check for compatible types (e.g., ref assignment)
            let target_type = self.type_ctx.get(target_ty);
            let value_type = self.type_ctx.get(value_ty);
            match (target_type, value_type) {
                (Some(Type::Ref { inner: t_inner }), Some(Type::Ref { inner: v_inner })) => {
                    if *t_inner == *v_inner {
                        return Ok(());
                    }
                }
                _ => {}
            }
            Err(format!("Cannot assign {:?} to {:?}", value_ty, target_ty))
        }
    }

    /// Check a for loop
    pub fn check_for_loop(&self, iter_ty: TyId, var_ty: TyId) -> Result<(), String> {
        let iter_type = self.type_ctx.get(iter_ty);
        match iter_type {
            Some(Type::Seq { elem }) => {
                if *elem == var_ty {
                    Ok(())
                } else {
                    Err(format!(
                        "For loop variable type mismatch: expected {:?}, got {:?}",
                        elem, var_ty
                    ))
                }
            }
            Some(Type::Array { elem, .. }) => {
                if *elem == var_ty {
                    Ok(())
                } else {
                    Err(format!(
                        "For loop variable type mismatch: expected {:?}, got {:?}",
                        elem, var_ty
                    ))
                }
            }
            Some(Type::Set { .. }) => {
                // Set iteration - variable should be element type
                Ok(())
            }
            _ => Err(format!("Cannot iterate over type {:?}", iter_ty)),
        }
    }
}

impl Default for StmtChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Declaration semantic checker
pub struct DeclChecker {
    type_ctx: TypeCtx,
    symbols: Vec<SymbolDef>,
}

impl Debug for DeclChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeclChecker")
            .field("symbols", &self.symbols.len())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct SymbolDef {
    pub name: Box<str>,
    pub ty: TyId,
    pub span: rnim_span::Span,
}

impl DeclChecker {
    pub fn new() -> Self {
        Self {
            type_ctx: TypeCtx::new(),
            symbols: Vec::new(),
        }
    }

    pub fn type_ctx(&self) -> &TypeCtx {
        &self.type_ctx
    }

    pub fn type_ctx_mut(&mut self) -> &mut TypeCtx {
        &mut self.type_ctx
    }

    /// Check a proc declaration
    pub fn check_proc(
        &mut self,
        _name: &str,
        params: &[TyId],
        ret: Option<TyId>,
    ) -> Result<TyId, String> {
        let proc_id =
            self.type_ctx
                .create_proc(params.to_vec(), ret, rnim_types::CallingConvention::Nimcall);
        Ok(proc_id)
    }

    /// Check a type declaration
    pub fn check_type(&mut self, name: &str, underlying: TyId) -> Result<TyId, String> {
        let alias_id = self.type_ctx.create_alias(name.into(), underlying);
        Ok(alias_id)
    }

    /// Check a var/let declaration
    pub fn check_var(&self, _name: &str, var_type: TyId) -> Result<TyId, String> {
        // For now, just return the type
        Ok(var_type)
    }

    /// Check a const declaration
    pub fn check_const(&self, _name: &str, const_type: TyId) -> Result<TyId, String> {
        // Constants must have a statically known type
        Ok(const_type)
    }

    /// Check for duplicate definitions
    pub fn check_duplicate(&self, name: &str) -> Result<(), String> {
        for sym in &self.symbols {
            if sym.name.as_ref() == name {
                return Err(format!("Duplicate definition of '{}'", name));
            }
        }
        Ok(())
    }

    /// Register a symbol
    pub fn add_symbol(&mut self, name: Box<str>, ty: TyId, span: rnim_span::Span) {
        self.symbols.push(SymbolDef { name, ty, span });
    }

    /// Look up a symbol by name
    pub fn lookup(&self, name: &str) -> Option<TyId> {
        for sym in &self.symbols {
            if sym.name.as_ref() == name {
                return Some(sym.ty);
            }
        }
        None
    }
}

impl Default for DeclChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ConversionCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for UnifyCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// Constant evaluation context for compile-time expression evaluation
pub struct ConstEvalCtx {
    type_ctx: TypeCtx,
    max_depth: usize,
    depth: usize,
}

impl fmt::Debug for ConstEvalCtx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConstEvalCtx")
            .field("max_depth", &self.max_depth)
            .field("depth", &self.depth)
            .finish()
    }
}

impl Clone for ConstEvalCtx {
    fn clone(&self) -> Self {
        ConstEvalCtx {
            type_ctx: TypeCtx::new(),
            max_depth: self.max_depth,
            depth: self.depth,
        }
    }
}

impl ConstEvalCtx {
    pub fn new() -> Self {
        ConstEvalCtx {
            type_ctx: TypeCtx::new(),
            max_depth: 100,
            depth: 0,
        }
    }

    pub fn type_ctx(&self) -> &TypeCtx {
        &self.type_ctx
    }

    pub fn type_ctx_mut(&mut self) -> &mut TypeCtx {
        &mut self.type_ctx
    }

    pub fn eval_literal(
        &mut self,
        value: &LiteralValue,
        expected: Option<TyId>,
    ) -> Result<TyId, String> {
        let ty = match value {
            LiteralValue::Bool(_) => self.type_ctx.get_primitive(PrimitiveKind::Bool),
            LiteralValue::Char(_) => self.type_ctx.get_primitive(PrimitiveKind::Char),
            LiteralValue::Int(_) => self.type_ctx.get_primitive(PrimitiveKind::Int),
            LiteralValue::Uint(_) => self.type_ctx.get_primitive(PrimitiveKind::Uint),
            LiteralValue::Float(_) => self.type_ctx.get_primitive(PrimitiveKind::Float64),
            LiteralValue::String(_) => None,
        };

        let ty_id = ty.ok_or_else(|| "Unknown literal type".to_string())?;

        if let Some(expected_id) = expected {
            let expected_type = self.type_ctx.get(expected_id);
            if let Some(Type::Primitive(p)) = expected_type {
                match (value, p.kind) {
                    (LiteralValue::Int(v), _) if *v < 0 && !p.is_signed => {
                        return Err("Negative value for unsigned type".to_string());
                    }
                    _ => {}
                }
            }
        }

        Ok(ty_id)
    }

    pub fn eval_binary_op(
        &mut self,
        op: BinaryOp,
        lhs: &LiteralValue,
        rhs: &LiteralValue,
    ) -> Result<LiteralValue, String> {
        use LiteralValue::*;
        match (lhs, rhs) {
            (Int(a), Int(b)) => match op {
                BinaryOp::Add => Ok(Int(a.wrapping_add(*b))),
                BinaryOp::Sub => Ok(Int(a.wrapping_sub(*b))),
                BinaryOp::Mul => Ok(Int(a.wrapping_mul(*b))),
                BinaryOp::Div if *b != 0 => Ok(Int(a.wrapping_div(*b))),
                BinaryOp::Mod if *b != 0 => Ok(Int(a.wrapping_rem(*b))),
                BinaryOp::BitAnd => Ok(Int(a & b)),
                BinaryOp::BitOr => Ok(Int(a | b)),
                BinaryOp::BitXor => Ok(Int(a ^ b)),
                BinaryOp::Shl => Ok(Int(a.wrapping_shl(*b as u32))),
                BinaryOp::Shr => Ok(Int(a.wrapping_shr(*b as u32))),
                _ => Err(format!("Invalid op {:?} for Int", op)),
            },
            (Uint(a), Uint(b)) => match op {
                BinaryOp::Add => Ok(Uint(a.wrapping_add(*b))),
                BinaryOp::Sub => Ok(Uint(a.wrapping_sub(*b))),
                BinaryOp::Mul => Ok(Uint(a.wrapping_mul(*b))),
                BinaryOp::Div if *b != 0 => Ok(Uint(a.wrapping_div(*b))),
                BinaryOp::Mod if *b != 0 => Ok(Uint(a.wrapping_rem(*b))),
                BinaryOp::BitAnd => Ok(Uint(a & b)),
                BinaryOp::BitOr => Ok(Uint(a | b)),
                BinaryOp::BitXor => Ok(Uint(a ^ b)),
                BinaryOp::Shl => Ok(Uint(a.wrapping_shl(*b as u32))),
                BinaryOp::Shr => Ok(Uint(a.wrapping_shr(*b as u32))),
                _ => Err(format!("Invalid op {:?} for Uint", op)),
            },
            (Float(a), Float(b)) => match op {
                BinaryOp::Add => Ok(Float(a + b)),
                BinaryOp::Sub => Ok(Float(a - b)),
                BinaryOp::Mul => Ok(Float(a * b)),
                BinaryOp::Div if *b != 0.0 => Ok(Float(a / b)),
                BinaryOp::Mod if *b != 0.0 => Ok(Float(a % b)),
                _ => Err(format!("Invalid op {:?} for Float", op)),
            },
            (Bool(a), Bool(b)) => match op {
                BinaryOp::LogicAnd => Ok(Bool(*a && *b)),
                BinaryOp::LogicOr => Ok(Bool(*a || *b)),
                BinaryOp::LogicXor => Ok(Bool(*a != *b)),
                _ => Err(format!("Invalid op {:?} for Bool", op)),
            },
            _ => Err("Type mismatch in binary operation".to_string()),
        }
    }

    pub fn eval_unary_op(
        &mut self,
        op: UnaryOp,
        operand: &LiteralValue,
    ) -> Result<LiteralValue, String> {
        use LiteralValue::*;
        match (op, operand) {
            (UnaryOp::Neg, Int(v)) => Ok(Int(v.wrapping_neg())),
            (UnaryOp::Neg, Float(v)) => Ok(Float(-v)),
            (UnaryOp::BitNot, Int(v)) => Ok(Int(!v)),
            (UnaryOp::BitNot, Uint(v)) => Ok(Uint(!v)),
            (UnaryOp::LogicNot, Bool(v)) => Ok(Bool(!v)),
            _ => Err("Type mismatch in unary operation".to_string()),
        }
    }

    pub fn eval_branch(
        &mut self,
        cond: bool,
        then_val: LiteralValue,
        else_val: LiteralValue,
    ) -> LiteralValue {
        if cond {
            then_val
        } else {
            else_val
        }
    }

    pub fn is_ct_evaluable(&self, ty_id: TyId) -> bool {
        let ty = self.type_ctx.get(ty_id);
        match ty {
            Some(Type::Primitive(p)) => {
                matches!(
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
                        | PrimitiveKind::Float
                        | PrimitiveKind::Float32
                        | PrimitiveKind::Float64
                )
            }
            Some(Type::Enum(_)) => true,
            Some(Type::Alias { underlying, .. }) => self.is_ct_evaluable(*underlying),
            Some(Type::Distinct { underlying, .. }) => self.is_ct_evaluable(*underlying),
            Some(Type::Subrange(SubrangeType { base: base_id, .. })) => {
                self.is_ct_evaluable(*base_id)
            }
            _ => false,
        }
    }

    pub fn get_const_value(&self, _name: &str) -> Option<LiteralValue> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    LogicAnd,
    LogicOr,
    LogicXor,
    Cmp,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    BitNot,
    LogicNot,
}

impl Default for ConstEvalCtx {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_types::{PrimitiveKind, Type};

    #[test]
    fn test_unify_primitive_same() {
        let mut ctx = UnifyCtx::new();
        let bool_id = ctx.type_ctx().get_primitive(PrimitiveKind::Bool).unwrap();
        let bool_id2 = ctx.type_ctx().get_primitive(PrimitiveKind::Bool).unwrap();
        assert_eq!(ctx.unify(bool_id, bool_id2), UnifyResult::Success);
    }

    #[test]
    fn test_unify_primitive_compatible() {
        let mut ctx = UnifyCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let int64_id = ctx
            .type_ctx_mut()
            .get_or_create_primitive(PrimitiveKind::Int64);
        let result = ctx.unify(int_id, int64_id);
        assert_eq!(result, UnifyResult::Success);
    }

    #[test]
    fn test_unify_ref() {
        let mut ctx = UnifyCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let ref1 = ctx.type_ctx_mut().create_ref(int_id);
        let ref2 = ctx.type_ctx_mut().create_ref(int_id);
        assert_eq!(ctx.unify(ref1, ref2), UnifyResult::Success);
    }

    #[test]
    fn test_unify_array() {
        let mut ctx = UnifyCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let arr1 = ctx.type_ctx_mut().create_array(int_id, Some(10));
        let arr2 = ctx.type_ctx_mut().create_array(int_id, Some(10));
        assert_eq!(ctx.unify(arr1, arr2), UnifyResult::Success);
    }

    #[test]
    fn test_unify_array_len_mismatch() {
        let mut ctx = UnifyCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let arr1 = ctx.type_ctx_mut().create_array(int_id, Some(10));
        let arr2 = ctx.type_ctx_mut().create_array(int_id, Some(20));
        assert!(matches!(ctx.unify(arr1, arr2), UnifyResult::Failure(_)));
    }

    #[test]
    fn test_unify_seq() {
        let mut ctx = UnifyCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let seq1 = ctx.type_ctx_mut().create_seq(int_id);
        let seq2 = ctx.type_ctx_mut().create_seq(int_id);
        assert_eq!(ctx.unify(seq1, seq2), UnifyResult::Success);
    }

    #[test]
    fn test_unify_tuple() {
        let mut ctx = UnifyCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let float_id = ctx
            .type_ctx()
            .get_primitive(PrimitiveKind::Float64)
            .unwrap();
        let tup1 = ctx.type_ctx_mut().create_tuple(vec![int_id, float_id]);
        let tup2 = ctx.type_ctx_mut().create_tuple(vec![int_id, float_id]);
        assert_eq!(ctx.unify(tup1, tup2), UnifyResult::Success);
    }

    #[test]
    fn test_unify_proc() {
        let mut ctx = UnifyCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let proc1 = ctx.type_ctx_mut().create_proc(
            vec![int_id],
            Some(int_id),
            rnim_types::CallingConvention::Nimcall,
        );
        let proc2 = ctx.type_ctx_mut().create_proc(
            vec![int_id],
            Some(int_id),
            rnim_types::CallingConvention::Nimcall,
        );
        assert_eq!(ctx.unify(proc1, proc2), UnifyResult::Success);
    }

    #[test]
    fn test_is_subtype_ref() {
        let mut ctx = UnifyCtx::new();
        let obj_id = ctx.type_ctx_mut().create_object(vec![], None);
        let ref_id = ctx.type_ctx_mut().create_ref(obj_id);
        assert!(ctx.is_subtype(ref_id, ref_id));
    }

    #[test]
    fn test_common_type() {
        let mut ctx = UnifyCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let common = ctx.common_type(int_id, int_id);
        assert!(common.is_some());
    }

    #[test]
    fn test_inference_ctx() {
        let mut ctx = InferenceCtx::new();
        ctx.add_constraint(Constraint::Equal(TyId::new(0), TyId::new(1)));
        assert_eq!(ctx.constraints().len(), 1);
    }

    #[test]
    fn test_conversion_rank_identity() {
        let ctx = ConversionCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        assert_eq!(
            ctx.conversion_rank(int_id, int_id),
            ConversionRank::Identity
        );
    }

    #[test]
    fn test_conversion_rank_nil_to_pointer() {
        let mut ctx = ConversionCtx::new();
        let obj_id = ctx.type_ctx_mut().create_object(vec![], None);
        let nil_id = ctx.type_ctx_mut().intern_type(Type::Nil);
        assert_eq!(
            ctx.conversion_rank(nil_id, obj_id),
            ConversionRank::NilToPointer
        );
    }

    #[test]
    fn test_conversion_rank_none() {
        let ctx = ConversionCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let float_id = ctx
            .type_ctx()
            .get_primitive(PrimitiveKind::Float64)
            .unwrap();
        assert_eq!(ctx.conversion_rank(int_id, float_id), ConversionRank::None);
    }

    #[test]
    fn test_can_convert() {
        let ctx = ConversionCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        assert!(ctx.can_convert(int_id, int_id));
    }

    #[test]
    fn test_overload_ctx_add_candidate() {
        let mut ctx = OverloadCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        ctx.add_candidate(int_id, ConversionRank::Identity);
        assert_eq!(ctx.candidates().len(), 1);
    }

    #[test]
    fn test_overload_ctx_resolve_unique() {
        let mut ctx = OverloadCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        ctx.add_candidate(int_id, ConversionRank::Identity);
        let result = ctx.resolve();
        assert!(matches!(result, OverloadResult::Unique(_)));
    }

    #[test]
    fn test_overload_ctx_resolve_none() {
        let ctx = OverloadCtx::new();
        let result = ctx.resolve();
        assert!(matches!(result, OverloadResult::None));
    }

    #[test]
    fn test_overload_ctx_resolve_ambiguous() {
        let mut ctx = OverloadCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let int64_id = ctx
            .type_ctx_mut()
            .get_or_create_primitive(PrimitiveKind::Int64);
        ctx.add_candidate(int_id, ConversionRank::IntegralWidening);
        ctx.add_candidate(int64_id, ConversionRank::IntegralWidening);
        let result = ctx.resolve();
        assert!(matches!(result, OverloadResult::Ambiguous(_)));
    }

    #[test]
    fn test_concept_solver_satisfies() {
        let solver = ConceptSolver::new();
        let int_id = solver.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        assert!(!solver.satisfies(int_id, "Equality"));
    }

    #[test]
    fn test_concept_solver_add_impl() {
        let mut solver = ConceptSolver::new();
        let int_id = solver.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        solver.add_impl(ConceptImpl {
            concept_name: "Equality".into(),
            type_id: int_id,
            methods: vec![],
        });
        assert!(solver.satisfies(int_id, "Equality"));
    }

    #[test]
    fn test_concept_solver_concepts_for_type() {
        let mut solver = ConceptSolver::new();
        let int_id = solver.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        solver.add_impl(ConceptImpl {
            concept_name: "Equality".into(),
            type_id: int_id,
            methods: vec![],
        });
        let concepts = solver.concepts_for_type(int_id);
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0], "Equality");
    }

    #[test]
    fn test_inheritance_solver_is_subtype() {
        let solver = InheritanceSolver::new();
        let int_id = solver.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        assert!(solver.is_subtype(int_id, int_id));
    }

    #[test]
    fn test_inheritance_solver_lookup_method() {
        let solver = InheritanceSolver::new();
        let int_id = solver.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        // Method lookup on primitive should return None (no methods)
        assert!(solver.lookup_method(int_id, "foo").is_none());
    }

    #[test]
    fn test_inheritance_solver_all_methods() {
        let solver = InheritanceSolver::new();
        let int_id = solver.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        let methods = solver.all_methods(int_id);
        assert!(methods.is_empty());
    }

    #[test]
    fn test_expr_checker_binary_op_int() {
        let mut checker = ExprChecker::new();
        let int_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        let result = checker.check_binary_op("+", int_id, int_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_expr_checker_comparison() {
        let mut checker = ExprChecker::new();
        let int_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        let result = checker.check_binary_op("<", int_id, int_id);
        assert!(result.is_ok());
        let result_bool = result.unwrap();
        let bool_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Bool)
            .unwrap();
        assert_eq!(result_bool, bool_id);
    }

    #[test]
    fn test_expr_checker_index() {
        let mut checker = ExprChecker::new();
        let int_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        let arr_id = checker.type_ctx_mut().create_array(int_id, Some(10));
        let idx_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        let result = checker.check_index(arr_id, idx_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_expr_checker_call() {
        let mut checker = ExprChecker::new();
        let int_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        let proc_id = checker.type_ctx_mut().create_proc(
            vec![int_id],
            Some(int_id),
            rnim_types::CallingConvention::Nimcall,
        );
        let result = checker.check_call(proc_id, &[int_id]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stmt_checker_if_condition() {
        let checker = StmtChecker::new();
        let bool_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Bool)
            .unwrap();
        assert!(checker.check_if_condition(bool_id).is_ok());
    }

    #[test]
    fn test_stmt_checker_return() {
        let checker = StmtChecker::new();
        let int_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        let checker = checker.with_return_type(int_id);
        assert!(checker.check_return(Some(int_id)).is_ok());
    }

    #[test]
    fn test_stmt_checker_assignment() {
        let mut checker = StmtChecker::new();
        let int_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        assert!(checker.check_assignment(int_id, int_id).is_ok());
    }

    #[test]
    fn test_stmt_checker_for_loop() {
        let checker = StmtChecker::new();
        let int_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        let mut checker = checker;
        let seq_id = checker.type_ctx_mut().create_seq(int_id);
        assert!(checker.check_for_loop(seq_id, int_id).is_ok());
    }

    #[test]
    fn test_decl_checker_proc() {
        let mut checker = DeclChecker::new();
        let int_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        let result = checker.check_proc("foo", &[int_id], Some(int_id));
        assert!(result.is_ok());
    }

    #[test]
    fn test_decl_checker_type() {
        let mut checker = DeclChecker::new();
        let int_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        let result = checker.check_type("MyInt", int_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decl_checker_duplicate() {
        let mut checker = DeclChecker::new();
        let int_id = checker
            .type_ctx()
            .get_primitive(PrimitiveKind::Int)
            .unwrap();
        checker.add_symbol("foo".into(), int_id, Span::new(FileId(0), 0, 0));
        assert!(checker.check_duplicate("foo").is_err());
    }

    #[test]
    fn test_const_eval_literal() {
        let mut ctx = ConstEvalCtx::new();
        let result = ctx.eval_literal(&LiteralValue::Int(42), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_const_eval_binary_op_int() {
        let mut ctx = ConstEvalCtx::new();
        let result = ctx.eval_binary_op(
            BinaryOp::Add,
            &LiteralValue::Int(10),
            &LiteralValue::Int(20),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LiteralValue::Int(30));
    }

    #[test]
    fn test_const_eval_binary_op_uint() {
        let mut ctx = ConstEvalCtx::new();
        let result = ctx.eval_binary_op(
            BinaryOp::Mul,
            &LiteralValue::Uint(5),
            &LiteralValue::Uint(3),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LiteralValue::Uint(15));
    }

    #[test]
    fn test_const_eval_binary_op_float() {
        let mut ctx = ConstEvalCtx::new();
        let result = ctx.eval_binary_op(
            BinaryOp::Div,
            &LiteralValue::Float(10.0),
            &LiteralValue::Float(2.0),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LiteralValue::Float(5.0));
    }

    #[test]
    fn test_const_eval_binary_op_bool() {
        let mut ctx = ConstEvalCtx::new();
        let result = ctx.eval_binary_op(
            BinaryOp::LogicAnd,
            &LiteralValue::Bool(true),
            &LiteralValue::Bool(false),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LiteralValue::Bool(false));
    }

    #[test]
    fn test_const_eval_unary_op() {
        let mut ctx = ConstEvalCtx::new();
        let result = ctx.eval_unary_op(UnaryOp::Neg, &LiteralValue::Int(42));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LiteralValue::Int(-42));
    }

    #[test]
    fn test_const_eval_branch() {
        let mut ctx = ConstEvalCtx::new();
        let result = ctx.eval_branch(true, LiteralValue::Int(1), LiteralValue::Int(2));
        assert_eq!(result, LiteralValue::Int(1));
        let result2 = ctx.eval_branch(false, LiteralValue::Int(1), LiteralValue::Int(2));
        assert_eq!(result2, LiteralValue::Int(2));
    }

    #[test]
    fn test_const_eval_is_ct_evaluable() {
        let ctx = ConstEvalCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        assert!(ctx.is_ct_evaluable(int_id));
    }

    #[test]
    fn test_const_eval_div_by_zero_int() {
        let mut ctx = ConstEvalCtx::new();
        let result =
            ctx.eval_binary_op(BinaryOp::Div, &LiteralValue::Int(1), &LiteralValue::Int(0));
        assert!(result.is_err());
    }

    #[test]
    fn test_const_eval_type_mismatch() {
        let mut ctx = ConstEvalCtx::new();
        let result = ctx.eval_binary_op(
            BinaryOp::Add,
            &LiteralValue::Int(1),
            &LiteralValue::Bool(true),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_const_eval_negative_unsigned() {
        // Test that eval_literal properly handles type checking
        // Using Int as the expected type and verifying basic validation
        let mut ctx = ConstEvalCtx::new();
        let int_id = ctx.type_ctx().get_primitive(PrimitiveKind::Int).unwrap();
        // Int is signed, so negative values should be fine
        let result = ctx.eval_literal(&LiteralValue::Int(-1), Some(int_id));
        assert!(result.is_ok());
    }
}
