//! Object inheritance and method dispatch.
//!
//! This module provides support for object inheritance, method resolution,
//! dynamic dispatch via vtables, and override checking.

use rnim_span::Span;
use std::collections::{HashMap, HashSet};

/// A method signature for a routine
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodSig {
    pub name: String,
    pub params: Vec<MethodParam>,
    pub ret_type: Option<String>,
    pub calling_convention: String,
}

impl MethodSig {
    pub fn new(name: &str) -> Self {
        MethodSig {
            name: name.to_string(),
            params: Vec::new(),
            ret_type: None,
            calling_convention: "nimcall".to_string(),
        }
    }

    pub fn with_param(mut self, name: &str, ty: &str) -> Self {
        self.params.push(MethodParam {
            name: name.to_string(),
            param_type: ty.to_string(),
        });
        self
    }

    pub fn with_return(mut self, ret: &str) -> Self {
        self.ret_type = Some(ret.to_string());
        self
    }
}

/// A parameter in a method signature
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodParam {
    pub name: String,
    pub param_type: String,
}

/// A declared method on a type
#[derive(Debug, Clone)]
pub struct Method {
    pub sig: MethodSig,
    pub body_span: Option<Span>,
    pub is_override: bool,
    pub is_final: bool,
}

impl Method {
    pub fn new(sig: MethodSig) -> Self {
        Method {
            sig,
            body_span: None,
            is_override: false,
            is_final: false,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.body_span = Some(span);
        self
    }

    pub fn with_override(mut self) -> Self {
        self.is_override = true;
        self
    }

    pub fn with_final(mut self) -> Self {
        self.is_final = true;
        self
    }
}

/// A method table (vtable) for dynamic dispatch
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodTable {
    /// Type this table is for
    pub type_name: String,
    /// Methods indexed by name (for single-dispatch)
    /// For multiple methods with same name (overloading), store all
    methods: Vec<(MethodSig, usize)>, // (signature, implementation index)
}

impl MethodTable {
    pub fn new(type_name: &str) -> Self {
        MethodTable {
            type_name: type_name.to_string(),
            methods: Vec::new(),
        }
    }

    /// Add a method to the vtable
    pub fn add_method(&mut self, sig: MethodSig, impl_index: usize) {
        self.methods.push((sig, impl_index));
    }

    /// Get method by name
    pub fn get_method(&self, name: &str) -> Option<&(MethodSig, usize)> {
        self.methods.iter().find(|(sig, _)| sig.name == name)
    }

    /// Get all methods
    pub fn methods(&self) -> &[(MethodSig, usize)] {
        &self.methods
    }

    /// Get method count
    pub fn len(&self) -> usize {
        self.methods.len()
    }

    pub fn is_empty(&self) -> bool {
        self.methods.is_empty()
    }
}

/// Object hierarchy for tracking inheritance relationships
#[derive(Debug, Clone, Default)]
pub struct ObjectHierarchy {
    /// Parent type for each type
    parents: HashMap<String, String>,
    /// Children for each type (reverse lookup)
    children: HashMap<String, HashSet<String>>,
    /// Methods defined on each type
    methods: HashMap<String, HashSet<String>>,
}

impl ObjectHierarchy {
    pub fn new() -> Self {
        ObjectHierarchy {
            parents: HashMap::new(),
            children: HashMap::new(),
            methods: HashMap::new(),
        }
    }

    /// Set the parent of a type
    pub fn set_parent(&mut self, type_name: &str, parent_name: &str) {
        // Remove from old parent's children if exists
        if let Some(old_parent) = self.parents.get(type_name) {
            if let Some(old_children) = self.children.get_mut(old_parent) {
                old_children.remove(type_name);
            }
        }

        // Set new parent
        self.parents
            .insert(type_name.to_string(), parent_name.to_string());

        // Add to new parent's children
        self.children
            .entry(parent_name.to_string())
            .or_default()
            .insert(type_name.to_string());
    }

    /// Get the parent of a type
    pub fn get_parent(&self, type_name: &str) -> Option<&str> {
        self.parents.get(type_name).map(|s| s.as_str())
    }

    /// Get all ancestors of a type (transitive closure)
    pub fn get_ancestors(&self, type_name: &str) -> Vec<&str> {
        let mut ancestors = Vec::new();
        let mut current = type_name;

        while let Some(parent) = self.get_parent(current) {
            ancestors.push(parent);
            current = parent;
        }

        ancestors
    }

    /// Get all descendants of a type
    pub fn get_descendants(&self, type_name: &str) -> Vec<&str> {
        let mut descendants = Vec::new();
        let mut queue = vec![type_name];

        while let Some(current) = queue.pop() {
            if let Some(children) = self.children.get(current) {
                for child in children {
                    descendants.push(child.as_str());
                    queue.push(child);
                }
            }
        }

        descendants
    }

    /// Check if type1 is an ancestor of type2
    pub fn is_ancestor(&self, type1: &str, type2: &str) -> bool {
        let mut current = type2;
        while let Some(parent) = self.get_parent(current) {
            if parent == type1 {
                return true;
            }
            current = parent;
        }
        false
    }

    /// Register that a type has a method
    pub fn register_method(&mut self, type_name: &str, method_name: &str) {
        self.methods
            .entry(type_name.to_string())
            .or_default()
            .insert(method_name.to_string());
    }

    /// Check if a type has a method directly defined
    pub fn has_method(&self, type_name: &str, method_name: &str) -> bool {
        self.methods
            .get(type_name)
            .map(|m| m.contains(method_name))
            .unwrap_or(false)
    }

    /// Get all methods defined on a type
    pub fn get_methods(&self, type_name: &str) -> HashSet<&str> {
        self.methods
            .get(type_name)
            .map(|m| m.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Check if type1 is a subtype of type2
    pub fn is_subtype(&self, type1: &str, type2: &str) -> bool {
        if type1 == type2 {
            return true;
        }
        self.is_ancestor(type2, type1)
    }
}

/// Override checker for validating method overrides
#[derive(Debug, Clone, Default)]
pub struct OverrideChecker {
    hierarchy: ObjectHierarchy,
}

impl OverrideChecker {
    pub fn new() -> Self {
        OverrideChecker {
            hierarchy: ObjectHierarchy::new(),
        }
    }

    /// Get the hierarchy
    pub fn hierarchy(&self) -> &ObjectHierarchy {
        &self.hierarchy
    }

    /// Get mutable hierarchy
    pub fn hierarchy_mut(&mut self) -> &mut ObjectHierarchy {
        &mut self.hierarchy
    }

    /// Check if overriding is valid and return error if not
    #[allow(unused_variables)]
    pub fn check_override(
        &self,
        type_name: &str,
        method_name: &str,
        new_sig: &MethodSig,
    ) -> Result<(), OverrideError> {
        // Check if parent has this method
        if let Some(parent) = self.hierarchy.get_parent(type_name) {
            // Method exists in parent - check if this is a valid override
            // In Nim, you can only override methods marked as overridable
            // For now, we allow override if the parent has the method
            if !self.hierarchy.has_method(parent, method_name) {
                return Err(OverrideError::MethodNotFound {
                    type_name: parent.to_string(),
                    method_name: method_name.to_string(),
                });
            }

            // Check signature compatibility
            // For now, just check name matching - more complex checking would
            // verify parameter types and return type compatibility
        }

        Ok(())
    }

    /// Check for final method violation
    pub fn check_final(&self, type_name: &str, method_name: &str) -> Result<(), OverrideError> {
        // Walk up the hierarchy to find if any ancestor declares this method as final
        let mut current = type_name;
        while let Some(parent) = self.hierarchy.get_parent(current) {
            if self.hierarchy.has_method(parent, method_name) {
                // In a full implementation, we'd check if the method was marked final
                // For now, we allow all overrides
            }
            current = parent;
        }
        Ok(())
    }
}

/// Override error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverrideError {
    MethodNotFound {
        type_name: String,
        method_name: String,
    },
    FinalMethodOverride {
        type_name: String,
        method_name: String,
    },
    SignatureMismatch {
        method_name: String,
        expected: String,
        found: String,
    },
}

impl std::fmt::Display for OverrideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverrideError::MethodNotFound {
                type_name,
                method_name,
            } => {
                write!(
                    f,
                    "method '{}' not found in parent type '{}'",
                    method_name, type_name
                )
            }
            OverrideError::FinalMethodOverride {
                type_name,
                method_name,
            } => {
                write!(
                    f,
                    "cannot override final method '{}' in type '{}'",
                    method_name, type_name
                )
            }
            OverrideError::SignatureMismatch {
                method_name,
                expected,
                found,
            } => {
                write!(
                    f,
                    "override '{}' has incompatible signature: expected {} but found {}",
                    method_name, expected, found
                )
            }
        }
    }
}

impl std::error::Error for OverrideError {}

/// Inheritance solver for method resolution
#[derive(Debug, Clone, Default)]
pub struct InheritanceSolver {
    hierarchy: ObjectHierarchy,
    method_tables: HashMap<String, MethodTable>,
}

impl InheritanceSolver {
    pub fn new() -> Self {
        InheritanceSolver {
            hierarchy: ObjectHierarchy::new(),
            method_tables: HashMap::new(),
        }
    }

    /// Get the hierarchy
    pub fn hierarchy(&self) -> &ObjectHierarchy {
        &self.hierarchy
    }

    /// Get mutable hierarchy
    pub fn hierarchy_mut(&mut self) -> &mut ObjectHierarchy {
        &mut self.hierarchy
    }

    /// Register a type with its parent
    pub fn register_inheritance(&mut self, type_name: &str, parent_name: &str) {
        self.hierarchy.set_parent(type_name, parent_name);
        // Inherit methods from parent - collect owned strings first to avoid borrow conflict
        let parent_methods: Vec<String> = self
            .hierarchy
            .get_methods(parent_name)
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        for method_name in parent_methods {
            self.hierarchy.register_method(type_name, &method_name);
        }
    }

    /// Register a method on a type
    pub fn register_method(&mut self, type_name: &str, method: &Method) {
        self.hierarchy.register_method(type_name, &method.sig.name);

        // Build or update the method table
        let table = self
            .method_tables
            .entry(type_name.to_string())
            .or_insert_with(|| MethodTable::new(type_name));

        // In a full implementation, we'd store the actual implementation index
        // and handle virtual dispatch
        table.add_method(method.sig.clone(), 0);
    }

    /// Get the method table for a type
    pub fn get_method_table(&self, type_name: &str) -> Option<&MethodTable> {
        self.method_tables.get(type_name)
    }

    /// Resolve a method call - find the correct implementation
    pub fn resolve_method(&self, type_name: &str, method_name: &str) -> Option<&MethodTable> {
        // First check if the type has the method
        if self.hierarchy.has_method(type_name, method_name) {
            return self.method_tables.get(type_name);
        }

        // Walk up the hierarchy
        let mut current = type_name;
        while let Some(parent) = self.hierarchy.get_parent(current) {
            if self.hierarchy.has_method(parent, method_name) {
                return self.method_tables.get(parent);
            }
            current = parent;
        }

        None
    }

    /// Check if a type is a subtype of another
    pub fn is_subtype(&self, type1: &str, type2: &str) -> bool {
        self.hierarchy.is_subtype(type1, type2)
    }

    /// Get the most specific common ancestor (least upper bound) of two types
    pub fn least_upper_bound(&self, type1: &str, type2: &str) -> Option<String> {
        if type1 == type2 {
            return Some(type1.to_string());
        }

        // Get ancestors of type1
        let ancestors1: HashSet<_> = std::iter::once(type1)
            .chain(self.hierarchy.get_ancestors(type1))
            .collect();

        // Get ancestors of type2 (including type2 itself)
        let ancestors2: Vec<_> = std::iter::once(type2)
            .chain(self.hierarchy.get_ancestors(type2))
            .collect();

        // Find the first ancestor of type2 that is also an ancestor of type1
        for ancestor in ancestors2 {
            if ancestors1.contains(ancestor) {
                return Some(ancestor.to_string());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_sig_new() {
        let sig = MethodSig::new("myMethod");
        assert_eq!(sig.name, "myMethod");
        assert!(sig.params.is_empty());
        assert!(sig.ret_type.is_none());
    }

    #[test]
    fn test_method_sig_with_params() {
        let sig = MethodSig::new("add")
            .with_param("a", "int")
            .with_param("b", "int")
            .with_return("int");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.ret_type, Some("int".to_string()));
    }

    #[test]
    fn test_method_new() {
        let method = Method::new(MethodSig::new("myMethod"));
        assert!(!method.is_override);
        assert!(!method.is_final);
    }

    #[test]
    fn test_method_with_override() {
        let method = Method::new(MethodSig::new("myMethod")).with_override();
        assert!(method.is_override);
    }

    #[test]
    fn test_method_table_new() {
        let table = MethodTable::new("MyType");
        assert_eq!(table.type_name, "MyType");
        assert!(table.is_empty());
    }

    #[test]
    fn test_method_table_add_method() {
        let mut table = MethodTable::new("MyType");
        table.add_method(MethodSig::new("method1"), 0);
        assert_eq!(table.len(), 1);
        assert!(table.get_method("method1").is_some());
    }

    #[test]
    fn test_object_hierarchy_new() {
        let hierarchy = ObjectHierarchy::new();
        assert!(hierarchy.get_parent("Child").is_none());
    }

    #[test]
    fn test_object_hierarchy_set_parent() {
        let mut hierarchy = ObjectHierarchy::new();
        hierarchy.set_parent("Child", "Parent");
        assert_eq!(hierarchy.get_parent("Child"), Some("Parent"));
    }

    #[test]
    fn test_object_hierarchy_ancestors() {
        let mut hierarchy = ObjectHierarchy::new();
        hierarchy.set_parent("Child", "Parent");
        hierarchy.set_parent("Parent", "GrandParent");
        let ancestors = hierarchy.get_ancestors("Child");
        assert_eq!(ancestors, vec!["Parent", "GrandParent"]);
    }

    #[test]
    fn test_object_hierarchy_is_ancestor() {
        let mut hierarchy = ObjectHierarchy::new();
        hierarchy.set_parent("Child", "Parent");
        hierarchy.set_parent("Parent", "GrandParent");
        assert!(hierarchy.is_ancestor("GrandParent", "Child"));
        assert!(!hierarchy.is_ancestor("Child", "GrandParent"));
    }

    #[test]
    fn test_object_hierarchy_descendants() {
        let mut hierarchy = ObjectHierarchy::new();
        hierarchy.set_parent("Child", "Parent");
        hierarchy.set_parent("Parent", "GrandParent");
        let descendants = hierarchy.get_descendants("GrandParent");
        assert!(descendants.contains(&"Parent"));
        assert!(descendants.contains(&"Child"));
    }

    #[test]
    fn test_object_hierarchy_register_method() {
        let mut hierarchy = ObjectHierarchy::new();
        hierarchy.register_method("MyType", "myMethod");
        assert!(hierarchy.has_method("MyType", "myMethod"));
        assert!(!hierarchy.has_method("MyType", "otherMethod"));
    }

    #[test]
    fn test_object_hierarchy_is_subtype() {
        let mut hierarchy = ObjectHierarchy::new();
        hierarchy.set_parent("Child", "Parent");
        assert!(hierarchy.is_subtype("Child", "Parent"));
        assert!(hierarchy.is_subtype("Child", "Child"));
        assert!(!hierarchy.is_subtype("Parent", "Child"));
    }

    #[test]
    fn test_override_checker_new() {
        let checker = OverrideChecker::new();
        assert!(checker.hierarchy().get_parent("Test").is_none());
    }

    #[test]
    fn test_override_checker_check_override() {
        let mut checker = OverrideChecker::new();
        checker.hierarchy_mut().set_parent("Child", "Parent");
        checker
            .hierarchy_mut()
            .register_method("Parent", "myMethod");

        let result = checker.check_override("Child", "myMethod", &MethodSig::new("myMethod"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_override_checker_method_not_found() {
        let mut checker = OverrideChecker::new();
        checker.hierarchy_mut().set_parent("Child", "Parent");
        // Don't register the method on Parent

        let result = checker.check_override("Child", "myMethod", &MethodSig::new("myMethod"));
        assert!(matches!(result, Err(OverrideError::MethodNotFound { .. })));
    }

    #[test]
    fn test_inheritance_solver_new() {
        let solver = InheritanceSolver::new();
        assert!(solver.hierarchy().get_parent("Test").is_none());
    }

    #[test]
    fn test_inheritance_solver_register_inheritance() {
        let mut solver = InheritanceSolver::new();
        solver.register_inheritance("Child", "Parent");
        assert_eq!(solver.hierarchy().get_parent("Child"), Some("Parent"));
    }

    #[test]
    fn test_inheritance_solver_register_method() {
        let mut solver = InheritanceSolver::new();
        solver.register_method("MyType", &Method::new(MethodSig::new("myMethod")));
        assert!(solver.hierarchy().has_method("MyType", "myMethod"));
    }

    #[test]
    fn test_inheritance_solver_resolve_method_direct() {
        let mut solver = InheritanceSolver::new();
        solver.register_method("MyType", &Method::new(MethodSig::new("myMethod")));

        let table = solver.resolve_method("MyType", "myMethod");
        assert!(table.is_some());
    }

    #[test]
    fn test_inheritance_solver_resolve_method_inherited() {
        let mut solver = InheritanceSolver::new();
        solver.register_inheritance("Child", "Parent");
        solver.register_method("Parent", &Method::new(MethodSig::new("myMethod")));

        let table = solver.resolve_method("Child", "myMethod");
        assert!(table.is_some());
        assert_eq!(table.unwrap().type_name, "Parent");
    }

    #[test]
    fn test_inheritance_solver_is_subtype() {
        let mut solver = InheritanceSolver::new();
        solver.register_inheritance("Child", "Parent");
        assert!(solver.is_subtype("Child", "Parent"));
        assert!(!solver.is_subtype("Parent", "Child"));
    }

    #[test]
    fn test_inheritance_solver_least_upper_bound() {
        let mut solver = InheritanceSolver::new();
        solver.register_inheritance("Left", "Common");
        solver.register_inheritance("Right", "Common");

        let lub = solver.least_upper_bound("Left", "Right");
        assert_eq!(lub, Some("Common".to_string()));
    }

    #[test]
    fn test_inheritance_solver_least_upper_bound_same_type() {
        let solver = InheritanceSolver::new();
        let lub = solver.least_upper_bound("MyType", "MyType");
        assert_eq!(lub, Some("MyType".to_string()));
    }

    #[test]
    fn test_least_upper_bound_no_common_ancestor() {
        let mut solver = InheritanceSolver::new();
        solver.register_inheritance("Left", "Parent1");
        solver.register_inheritance("Right", "Parent2");

        let lub = solver.least_upper_bound("Left", "Right");
        assert!(lub.is_none());
    }

    #[test]
    fn test_method_param_creation() {
        let param = MethodParam {
            name: "x".to_string(),
            param_type: "int".to_string(),
        };
        assert_eq!(param.name, "x");
        assert_eq!(param.param_type, "int");
    }

    #[test]
    fn test_method_table_with_multiple_methods() {
        let mut table = MethodTable::new("MyType");
        table.add_method(MethodSig::new("foo").with_return("int"), 0);
        table.add_method(MethodSig::new("bar").with_return("string"), 1);
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn test_object_hierarchy_base_type() {
        let mut hierarchy = ObjectHierarchy::new();
        hierarchy.set_parent("Child", "Base");
        assert!(hierarchy.is_ancestor("Base", "Child"));
        assert!(!hierarchy.is_ancestor("Child", "Base"));
    }

    #[test]
    fn test_object_hierarchy_descendant_check() {
        let mut hierarchy = ObjectHierarchy::new();
        hierarchy.set_parent("GrandChild", "Child");
        hierarchy.set_parent("Child", "Base");

        let descendants = hierarchy.get_descendants("Base");
        assert!(descendants.contains(&"Child"));
        assert!(descendants.contains(&"GrandChild"));
    }

    #[test]
    fn test_override_checker_valid_override() {
        let mut checker = OverrideChecker::new();
        checker.hierarchy_mut().set_parent("Child", "Parent");
        checker.hierarchy_mut().register_method("Parent", "method");

        // Adding overriding method should not fail
        let result = checker.check_override("Child", "method", &MethodSig::new("method"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_inheritance_solver_multiple_inheritance_paths() {
        let mut solver = InheritanceSolver::new();
        solver.register_inheritance("Child", "Parent");
        solver.register_inheritance("Child", "Base");

        // Child should be subtype of Base
        assert!(solver.is_subtype("Child", "Base"));
    }
}
