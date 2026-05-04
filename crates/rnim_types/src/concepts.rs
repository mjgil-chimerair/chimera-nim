//! Concepts (type classes) for generic constraints.
//!
//! This module provides concept declaration, matching, and satisfaction checking
//! for Nim's concept-based type constraints.

use rnim_span::Span;
use std::collections::{HashMap, HashSet};

/// A concept member requirement (method, type, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConceptMember {
    /// Required method with signature
    Method {
        name: String,
        params: Vec<ConceptParam>,
        ret_type: Option<String>,
    },
    /// Required type member
    Type(String),
    /// Required static member
    Static { name: String, ty: String },
}

/// A parameter in a concept method signature
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptParam {
    pub name: String,
    pub param_type: String,
}

/// A declared concept with its requirements
#[derive(Debug, Clone)]
pub struct Concept {
    pub name: String,
    pub type_params: Vec<String>,
    pub members: Vec<ConceptMember>,
    pub parent_concepts: Vec<String>,
    pub span: Span,
}

impl Concept {
    pub fn new(name: &str, type_params: Vec<String>, span: Span) -> Self {
        Concept {
            name: name.to_string(),
            type_params,
            members: Vec::new(),
            parent_concepts: Vec::new(),
            span,
        }
    }

    pub fn with_member(mut self, member: ConceptMember) -> Self {
        self.members.push(member);
        self
    }

    pub fn with_parent(mut self, parent: &str) -> Self {
        self.parent_concepts.push(parent.to_string());
        self
    }
}

/// Result of checking if a type satisfies a concept
#[derive(Debug, Clone)]
pub struct ConceptMatchResult {
    pub satisfied: bool,
    pub missing_members: Vec<String>,
    pub type_errors: Vec<String>,
}

impl ConceptMatchResult {
    pub fn satisfied() -> Self {
        ConceptMatchResult {
            satisfied: true,
            missing_members: Vec::new(),
            type_errors: Vec::new(),
        }
    }

    pub fn not_satisfied(missing: Vec<String>, errors: Vec<String>) -> Self {
        ConceptMatchResult {
            satisfied: false,
            missing_members: missing,
            type_errors: errors,
        }
    }
}

/// Concept resolution context
#[derive(Debug, Clone, Default)]
pub struct ConceptCtx {
    /// Registered concepts by name
    concepts: HashMap<String, Concept>,
    /// Implementations: (type_name, concept_name) -> impl info
    implementations: HashMap<(String, String), TypeImplementation>,
    /// Cache for match results
    match_cache: HashMap<(String, String), ConceptMatchResult>,
}

impl ConceptCtx {
    pub fn new() -> Self {
        ConceptCtx {
            concepts: HashMap::new(),
            implementations: HashMap::new(),
            match_cache: HashMap::new(),
        }
    }

    /// Register a concept
    pub fn register_concept(&mut self, concept: Concept) {
        self.concepts.insert(concept.name.clone(), concept);
        self.match_cache.clear();
    }

    /// Get a concept by name
    pub fn get_concept(&self, name: &str) -> Option<&Concept> {
        self.concepts.get(name)
    }

    /// Get all concept names
    pub fn concept_names(&self) -> impl Iterator<Item = &str> {
        self.concepts.keys().map(|s| s.as_str())
    }

    /// Register that a type implements a concept
    pub fn register_implementation(
        &mut self,
        type_name: &str,
        concept_name: &str,
        impl_info: TypeImplementation,
    ) {
        let key = (type_name.to_string(), concept_name.to_string());
        self.implementations.insert(key, impl_info);
        self.match_cache.clear();
    }

    /// Check if a type satisfies a concept
    pub fn check_satisfaction(
        &mut self,
        type_name: &str,
        concept_name: &str,
    ) -> ConceptMatchResult {
        let key = (type_name.to_string(), concept_name.to_string());

        // Check cache first
        if let Some(result) = self.match_cache.get(&key) {
            return result.clone();
        }

        let result = self.compute_match(type_name, concept_name);
        self.match_cache.insert(key, result.clone());
        result
    }

    /// Compute whether a type matches a concept
    fn compute_match(&mut self, type_name: &str, concept_name: &str) -> ConceptMatchResult {
        // Check if there's a registered implementation
        let impl_key = (type_name.to_string(), concept_name.to_string());
        if let Some(_impl) = self.implementations.get(&impl_key) {
            return ConceptMatchResult::satisfied();
        }

        // Get the concept - extract data we need to avoid borrow conflicts
        let concept = match self.concepts.get(concept_name) {
            Some(c) => c,
            None => {
                return ConceptMatchResult::not_satisfied(
                    vec![format!("concept '{}' not found", concept_name)],
                    vec![],
                );
            }
        };

        // Extract what we need from the concept
        let parent_concepts: Vec<String> = concept.parent_concepts.clone();
        let member_checks: Vec<(String, bool)> = concept
            .members
            .iter()
            .map(|m| {
                (
                    self.member_name(m),
                    self.check_member(type_name, concept_name, m),
                )
            })
            .collect();

        // Check parent concepts first
        for parent_name in &parent_concepts {
            if !self.check_satisfaction(type_name, parent_name).satisfied {
                return ConceptMatchResult::not_satisfied(
                    vec![format!("parent concept '{}' not satisfied", parent_name)],
                    vec![],
                );
            }
        }

        // Check each member requirement
        let missing: Vec<String> = member_checks
            .into_iter()
            .filter(|(_, present)| !*present)
            .map(|(name, _)| name)
            .collect();

        if missing.is_empty() {
            ConceptMatchResult::satisfied()
        } else {
            ConceptMatchResult::not_satisfied(missing, vec![])
        }
    }

    /// Check if a type has a specific member
    fn check_member(&self, type_name: &str, concept_name: &str, member: &ConceptMember) -> bool {
        match member {
            ConceptMember::Method { name, .. } => {
                // Check if type has the method - look up in implementations
                let impl_key = (type_name.to_string(), concept_name.to_string());
                if let Some(impl_info) = self.implementations.get(&impl_key) {
                    impl_info.methods.contains(name)
                } else {
                    // Check built-in types for known methods
                    self.has_builtin_method(type_name, name)
                }
            }
            ConceptMember::Type(type_name) => {
                // Type member exists if it's a known type
                self.is_known_type(type_name)
            }
            ConceptMember::Static { name, .. } => {
                // Static members are checked via implementations
                let impl_key = (type_name.to_string(), concept_name.to_string());
                self.implementations
                    .get(&impl_key)
                    .map(|i| i.static_members.contains(name))
                    .unwrap_or(false)
            }
        }
    }

    /// Check if a built-in type has a method
    fn has_builtin_method(&self, type_name: &str, method_name: &str) -> bool {
        match type_name {
            "int" | "int8" | "int16" | "int32" | "int64" | "uint" | "uint8" | "uint16"
            | "uint32" | "uint64" => {
                matches!(
                    method_name,
                    "add"
                        | "sub"
                        | "mul"
                        | "div"
                        | "mod"
                        | "eq"
                        | "neq"
                        | "lt"
                        | "lte"
                        | "gt"
                        | "gte"
                        | "toString"
                        | "hash"
                        | "min"
                        | "max"
                )
            }
            "float" | "float32" | "float64" => {
                matches!(
                    method_name,
                    "add"
                        | "sub"
                        | "mul"
                        | "div"
                        | "mod"
                        | "eq"
                        | "neq"
                        | "lt"
                        | "lte"
                        | "gt"
                        | "gte"
                        | "toString"
                        | "hash"
                        | "floor"
                        | "ceil"
                        | "round"
                )
            }
            "string" => {
                matches!(
                    method_name,
                    "add"
                        | "eq"
                        | "neq"
                        | "len"
                        | "toString"
                        | "toInt"
                        | "toFloat"
                        | "substring"
                        | "startsWith"
                        | "endsWith"
                        | "contains"
                        | "hash"
                )
            }
            "char" => {
                matches!(method_name, "eq" | "neq" | "toString" | "toInt" | "hash")
            }
            "bool" => {
                matches!(method_name, "eq" | "neq" | "toString" | "toInt" | "hash")
            }
            _ => false,
        }
    }

    /// Check if a type name is known
    fn is_known_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "int"
                | "int8"
                | "int16"
                | "int32"
                | "int64"
                | "uint"
                | "uint8"
                | "uint16"
                | "uint32"
                | "uint64"
                | "float"
                | "float32"
                | "float64"
                | "string"
                | "char"
                | "bool"
                | "void"
                | "nil"
                | "Object"
                | "RootObj"
                | "seq"
                | "set"
                | "array"
        )
    }

    /// Get the name of a member for error messages
    fn member_name(&self, member: &ConceptMember) -> String {
        match member {
            ConceptMember::Method { name, .. } => name.clone(),
            ConceptMember::Type(name) => format!("type {}", name),
            ConceptMember::Static { name, .. } => format!("static {}", name),
        }
    }

    /// Clear the match cache
    pub fn clear_cache(&mut self) {
        self.match_cache.clear();
    }
}

/// Information about a type's implementation of a concept
#[derive(Debug, Clone)]
pub struct TypeImplementation {
    pub type_name: String,
    pub concept_name: String,
    pub methods: HashSet<String>,
    pub static_members: HashSet<String>,
    pub associated_types: HashMap<String, String>,
    pub span: Span,
}

impl TypeImplementation {
    pub fn new(type_name: &str, concept_name: &str, span: Span) -> Self {
        TypeImplementation {
            type_name: type_name.to_string(),
            concept_name: concept_name.to_string(),
            methods: HashSet::new(),
            static_members: HashSet::new(),
            associated_types: HashMap::new(),
            span,
        }
    }

    pub fn with_method(mut self, method: &str) -> Self {
        self.methods.insert(method.to_string());
        self
    }

    pub fn with_static_member(mut self, member: &str) -> Self {
        self.static_members.insert(member.to_string());
        self
    }

    pub fn with_associated_type(mut self, name: &str, ty: &str) -> Self {
        self.associated_types
            .insert(name.to_string(), ty.to_string());
        self
    }
}

/// Concept solver for generic constraint solving
#[derive(Debug, Clone, Default)]
pub struct ConceptSolver {
    ctx: ConceptCtx,
}

impl ConceptSolver {
    pub fn new() -> Self {
        ConceptSolver {
            ctx: ConceptCtx::new(),
        }
    }

    /// Get the concept context
    pub fn context(&self) -> &ConceptCtx {
        &self.ctx
    }

    /// Get mutable concept context
    pub fn context_mut(&mut self) -> &mut ConceptCtx {
        &mut self.ctx
    }

    /// Register a concept
    pub fn add_concept(&mut self, concept: Concept) {
        self.ctx.register_concept(concept);
    }

    /// Register an implementation
    pub fn add_implementation(&mut self, impl_info: TypeImplementation) {
        let type_name = impl_info.type_name.clone();
        let concept_name = impl_info.concept_name.clone();
        self.ctx
            .register_implementation(&type_name, &concept_name, impl_info);
    }

    /// Check if a type satisfies a concept
    pub fn satisfies(&mut self, type_name: &str, concept_name: &str) -> ConceptMatchResult {
        self.ctx.check_satisfaction(type_name, concept_name)
    }

    /// Find all concepts a type satisfies
    pub fn find_satisfying_concepts(&mut self, type_name: &str) -> Vec<String> {
        let concept_names: Vec<String> = self.ctx.concept_names().map(String::from).collect();
        concept_names
            .into_iter()
            .filter(|name| self.ctx.check_satisfaction(type_name, name).satisfied)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_concept_new() {
        let concept = Concept::new("Addable", vec!["T".to_string()], Span::new(FileId(0), 0, 0));
        assert_eq!(concept.name, "Addable");
        assert_eq!(concept.type_params.len(), 1);
        assert!(concept.members.is_empty());
    }

    #[test]
    fn test_concept_with_member() {
        let concept = Concept::new("Addable", vec!["T".to_string()], Span::new(FileId(0), 0, 0))
            .with_member(ConceptMember::Type("int".to_string()));
        assert_eq!(concept.members.len(), 1);
    }

    #[test]
    fn test_concept_with_parent() {
        let concept = Concept::new(
            "SubAddable",
            vec!["T".to_string()],
            Span::new(FileId(0), 0, 0),
        )
        .with_parent("Addable");
        assert_eq!(concept.parent_concepts.len(), 1);
    }

    #[test]
    fn test_concept_ctx_new() {
        let ctx = ConceptCtx::new();
        assert!(ctx.concepts.is_empty());
    }

    #[test]
    fn test_concept_ctx_register_concept() {
        let mut ctx = ConceptCtx::new();
        let concept = Concept::new("Addable", vec!["T".to_string()], Span::new(FileId(0), 0, 0));
        ctx.register_concept(concept);
        assert!(ctx.get_concept("Addable").is_some());
    }

    #[test]
    fn test_concept_ctx_register_implementation() {
        let mut ctx = ConceptCtx::new();
        let impl_info = TypeImplementation::new("int", "Addable", Span::new(FileId(0), 0, 0))
            .with_method("add");
        ctx.register_implementation("int", "Addable", impl_info);
        let result = ctx.check_satisfaction("int", "Addable");
        assert!(result.satisfied);
    }

    #[test]
    fn test_concept_match_int_addable() {
        let mut solver = ConceptSolver::new();
        let concept = Concept::new("Addable", vec!["T".to_string()], Span::new(FileId(0), 0, 0))
            .with_member(ConceptMember::Method {
                name: "add".to_string(),
                params: vec![
                    ConceptParam {
                        name: "a".to_string(),
                        param_type: "T".to_string(),
                    },
                    ConceptParam {
                        name: "b".to_string(),
                        param_type: "T".to_string(),
                    },
                ],
                ret_type: Some("T".to_string()),
            });
        solver.add_concept(concept);

        let impl_info = TypeImplementation::new("int", "Addable", Span::new(FileId(0), 0, 0))
            .with_method("add");
        solver.add_implementation(impl_info);

        let result = solver.satisfies("int", "Addable");
        assert!(result.satisfied);
    }

    #[test]
    fn test_concept_match_missing_method() {
        let mut solver = ConceptSolver::new();
        // Use a method that int doesn't have built-in
        let concept = Concept::new(
            "Iteratable",
            vec!["T".to_string()],
            Span::new(FileId(0), 0, 0),
        )
        .with_member(ConceptMember::Method {
            name: "iter".to_string(),
            params: vec![],
            ret_type: Some("T".to_string()),
        });
        solver.add_concept(concept);
        // No implementation registered

        let result = solver.satisfies("int", "Iteratable");
        assert!(!result.satisfied);
        assert!(result.missing_members.contains(&"iter".to_string()));
    }

    #[test]
    fn test_concept_match_builtin_method() {
        let mut solver = ConceptSolver::new();
        let concept = Concept::new(
            "EqComparable",
            vec!["T".to_string()],
            Span::new(FileId(0), 0, 0),
        )
        .with_member(ConceptMember::Method {
            name: "eq".to_string(),
            params: vec![
                ConceptParam {
                    name: "a".to_string(),
                    param_type: "T".to_string(),
                },
                ConceptParam {
                    name: "b".to_string(),
                    param_type: "T".to_string(),
                },
            ],
            ret_type: Some("bool".to_string()),
        });
        solver.add_concept(concept);

        // int has built-in 'eq' method
        let result = solver.satisfies("int", "EqComparable");
        assert!(result.satisfied);
    }

    #[test]
    fn test_concept_match_parent_concept() {
        let mut solver = ConceptSolver::new();

        // Use methods that int doesn't have built-in
        let parent = Concept::new(
            "Container",
            vec!["T".to_string()],
            Span::new(FileId(0), 0, 0),
        )
        .with_member(ConceptMember::Method {
            name: "contains".to_string(),
            params: vec![],
            ret_type: Some("bool".to_string()),
        });
        solver.add_concept(parent);

        let child = Concept::new(
            "IndexedContainer",
            vec!["T".to_string()],
            Span::new(FileId(0), 0, 0),
        )
        .with_parent("Container")
        .with_member(ConceptMember::Method {
            name: "at".to_string(),
            params: vec![],
            ret_type: Some("T".to_string()),
        });
        solver.add_concept(child);

        // Register implementation for parent only
        let impl_info = TypeImplementation::new("MyList", "Container", Span::new(FileId(0), 0, 0))
            .with_method("contains");
        solver.add_implementation(impl_info);

        // Should fail - child requires at method which MyList doesn't have
        let result = solver.satisfies("MyList", "IndexedContainer");
        assert!(!result.satisfied);
    }

    #[test]
    fn test_type_implementation_new() {
        let impl_info = TypeImplementation::new("MyType", "MyConcept", Span::new(FileId(0), 0, 0));
        assert_eq!(impl_info.type_name, "MyType");
        assert_eq!(impl_info.concept_name, "MyConcept");
    }

    #[test]
    fn test_type_implementation_with_method() {
        let impl_info = TypeImplementation::new("MyType", "MyConcept", Span::new(FileId(0), 0, 0))
            .with_method("method1")
            .with_method("method2");
        assert!(impl_info.methods.contains("method1"));
        assert!(impl_info.methods.contains("method2"));
    }

    #[test]
    fn test_type_implementation_with_associated_type() {
        let impl_info = TypeImplementation::new("MyType", "MyConcept", Span::new(FileId(0), 0, 0))
            .with_associated_type("Result", "int");
        assert_eq!(
            impl_info.associated_types.get("Result"),
            Some(&"int".to_string())
        );
    }

    #[test]
    fn test_find_satisfying_concepts() {
        let mut solver = ConceptSolver::new();

        let concept1 = Concept::new("Addable", vec!["T".to_string()], Span::new(FileId(0), 0, 0))
            .with_member(ConceptMember::Method {
                name: "add".to_string(),
                params: vec![],
                ret_type: Some("T".to_string()),
            });
        solver.add_concept(concept1);

        let concept2 = Concept::new(
            "EqComparable",
            vec!["T".to_string()],
            Span::new(FileId(0), 0, 0),
        )
        .with_member(ConceptMember::Method {
            name: "eq".to_string(),
            params: vec![],
            ret_type: Some("bool".to_string()),
        });
        solver.add_concept(concept2);

        // int satisfies both
        let impl1 = TypeImplementation::new("int", "Addable", Span::new(FileId(0), 0, 0))
            .with_method("add");
        solver.add_implementation(impl1);

        // int has built-in eq
        let concepts = solver.find_satisfying_concepts("int");
        assert!(concepts.contains(&"Addable".to_string()));
        assert!(concepts.contains(&"EqComparable".to_string()));
    }

    #[test]
    fn test_concept_member_method_name() {
        let member = ConceptMember::Method {
            name: "myMethod".to_string(),
            params: vec![],
            ret_type: None,
        };
        let ctx = ConceptCtx::new();
        assert_eq!(ctx.member_name(&member), "myMethod");
    }

    #[test]
    fn test_concept_member_type_name() {
        let member = ConceptMember::Type("MyType".to_string());
        let ctx = ConceptCtx::new();
        assert_eq!(ctx.member_name(&member), "type MyType");
    }

    #[test]
    fn test_concept_member_static_name() {
        let member = ConceptMember::Static {
            name: "myStatic".to_string(),
            ty: "int".to_string(),
        };
        let ctx = ConceptCtx::new();
        assert_eq!(ctx.member_name(&member), "static myStatic");
    }

    #[test]
    fn test_concept_match_result_satisfied() {
        let result = ConceptMatchResult::satisfied();
        assert!(result.satisfied);
        assert!(result.missing_members.is_empty());
        assert!(result.type_errors.is_empty());
    }

    #[test]
    fn test_concept_match_result_not_satisfied() {
        let result = ConceptMatchResult::not_satisfied(
            vec!["method1".to_string()],
            vec!["type error".to_string()],
        );
        assert!(!result.satisfied);
        assert_eq!(result.missing_members.len(), 1);
        assert_eq!(result.type_errors.len(), 1);
    }

    #[test]
    fn test_concept_register_and_lookup() {
        let mut ctx = ConceptCtx::new();
        let concept = Concept::new("TestConcept", vec![], Span::new(FileId::new(0), 0, 10));
        ctx.register_concept(concept);
        assert!(ctx.get_concept("TestConcept").is_some());
    }

    #[test]
    fn test_concept_equality() {
        let c1 = Concept::new("Test", vec![], Span::new(FileId::new(0), 0, 4));
        let c2 = Concept::new("Test", vec![], Span::new(FileId::new(0), 5, 9));
        let c3 = Concept::new("Other", vec![], Span::new(FileId::new(0), 0, 5));
        assert_eq!(c1.name, c2.name);
        assert_ne!(c1.name, c3.name);
    }

    #[test]
    fn test_concept_member_method_variant() {
        let member = ConceptMember::Method {
            name: "push".to_string(),
            params: vec![],
            ret_type: None,
        };
        assert!(matches!(member, ConceptMember::Method { .. }));
    }

    #[test]
    fn test_concept_member_type() {
        let member = ConceptMember::Type("Value".to_string());
        assert!(matches!(member, ConceptMember::Type(_)));
    }

    #[test]
    fn test_concept_member_static() {
        let member = ConceptMember::Static {
            name: "default".to_string(),
            ty: "int".to_string(),
        };
        assert!(matches!(member, ConceptMember::Static { .. }));
    }

    #[test]
    fn test_type_implementation_multiple_methods() {
        let impl_type = TypeImplementation::new("MyType", "Container", Span::new(FileId::new(0), 0, 10))
            .with_method("foo")
            .with_method("bar");
        assert_eq!(impl_type.methods.len(), 2);
    }

    #[test]
    fn test_concept_ctx_empty() {
        let ctx = ConceptCtx::new();
        assert!(ctx.get_concept("nonexistent").is_none());
    }
}
