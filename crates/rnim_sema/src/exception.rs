//! Exception system semantics, try/except/finally, raise, and defer support.

use rnim_span::Span;
use std::collections::HashMap;

/// Exception base class hierarchy
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExceptionType {
    /// The root Exception type
    Exception,
    /// SystemExit exception
    Exit,
    /// ValueError exception
    ValueError,
    /// IndexError exception
    IndexError,
    /// KeyError exception
    KeyError,
    /// RuntimeError exception
    RuntimeError,
    /// IOError exception
    IOError,
    /// OSError exception
    OSError,
    /// Custom/user-defined exception
    Custom(String),
}

/// An exception value
#[derive(Debug, Clone, PartialEq)]
pub struct Exception {
    pub typ: ExceptionType,
    pub message: String,
    pub cause: Option<Box<Exception>>,
    pub span: Span,
}

impl Exception {
    pub fn new(typ: ExceptionType, message: String, span: Span) -> Self {
        Exception {
            typ,
            message,
            cause: None,
            span,
        }
    }

    pub fn with_cause(mut self, cause: Exception) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }
}

/// Exception handling clause
#[derive(Debug, Clone, PartialEq)]
pub struct ExceptClause {
    /// Exception type to catch (None means catch all)
    pub catch_type: Option<ExceptionType>,
    /// Variable name to bind the exception to
    pub bind_name: Option<String>,
    /// Statements in the except block
    pub body: Vec<String>,
    pub span: Span,
}

/// Try statement components
#[derive(Debug, Clone, PartialEq)]
pub struct TryStmt {
    /// Statements in the try block
    pub try_body: Vec<String>,
    /// Except clauses
    pub excepts: Vec<ExceptClause>,
    /// Statements in the finally block
    pub finally_body: Option<Vec<String>>,
    pub span: Span,
}

/// Defer statement
#[derive(Debug, Clone, PartialEq)]
pub struct DeferStmt {
    /// Statements to run on scope exit
    pub body: Vec<String>,
    pub span: Span,
}

/// Exception information stored in the type system
#[derive(Debug, Clone, PartialEq)]
pub struct ExceptionInfo {
    pub typ: ExceptionType,
    pub name: String,
    pub parent: Option<ExceptionType>,
}

impl ExceptionInfo {
    pub fn new(typ: ExceptionType, name: &str) -> Self {
        ExceptionInfo {
            typ,
            name: name.to_string(),
            parent: None,
        }
    }

    pub fn with_parent(mut self, parent: ExceptionType) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Check if this exception is a subtype of another (using provided registry)
    pub fn is_subtype_of(&self, other: &ExceptionType, registry: &ExceptionRegistry) -> bool {
        if &self.typ == other {
            return true;
        }
        if let Some(ref parent_type) = self.parent {
            if parent_type == other {
                return true;
            }
            // Recursively check the parent's hierarchy
            if let Some(parent_info) = registry.get_info(parent_type) {
                return parent_info.is_subtype_of(other, registry);
            }
        }
        false
    }
}

/// Registry of exception types
#[derive(Debug, Clone, Default)]
pub struct ExceptionRegistry {
    exceptions: HashMap<ExceptionType, ExceptionInfo>,
}

impl ExceptionRegistry {
    pub fn new() -> Self {
        let mut registry = ExceptionRegistry::default();
        registry.init_builtins();
        registry
    }

    fn init_builtins(&mut self) {
        // Exception is the root
        self.register(ExceptionInfo::new(ExceptionType::Exception, "Exception"));

        // Direct subclasses of Exception
        self.register(
            ExceptionInfo::new(ExceptionType::Exit, "SystemExit")
                .with_parent(ExceptionType::Exception),
        );
        self.register(
            ExceptionInfo::new(ExceptionType::ValueError, "ValueError")
                .with_parent(ExceptionType::Exception),
        );
        self.register(
            ExceptionInfo::new(ExceptionType::IndexError, "IndexError")
                .with_parent(ExceptionType::Exception),
        );
        self.register(
            ExceptionInfo::new(ExceptionType::KeyError, "KeyError")
                .with_parent(ExceptionType::Exception),
        );
        self.register(
            ExceptionInfo::new(ExceptionType::RuntimeError, "RuntimeError")
                .with_parent(ExceptionType::Exception),
        );
        self.register(
            ExceptionInfo::new(ExceptionType::IOError, "IOError")
                .with_parent(ExceptionType::Exception),
        );
        self.register(
            ExceptionInfo::new(ExceptionType::OSError, "OSError")
                .with_parent(ExceptionType::Exception),
        );
    }

    /// Register an exception type
    pub fn register(&mut self, info: ExceptionInfo) {
        self.exceptions.insert(info.typ.clone(), info);
    }

    /// Get exception info for a type
    pub fn get_info(&self, typ: &ExceptionType) -> Option<&ExceptionInfo> {
        self.exceptions.get(typ)
    }

    /// Check if an exception type is a subtype of another
    pub fn is_subtype(&self, child: &ExceptionType, parent: &ExceptionType) -> bool {
        if child == parent {
            return true;
        }
        if let Some(info) = self.exceptions.get(child) {
            return info.is_subtype_of(parent, self);
        }
        false
    }

    /// Check if a type is registered
    pub fn contains(&self, typ: &ExceptionType) -> bool {
        self.exceptions.contains_key(typ)
    }
}

/// Effect set for exception tracking
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectSet {
    /// Exceptions that can be raised
    raises: Vec<ExceptionType>,
    /// Tags for side effects
    tags: Vec<String>,
}

impl EffectSet {
    pub fn new() -> Self {
        EffectSet {
            raises: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Add an exception to the effect set
    pub fn add_raise(&mut self, typ: ExceptionType) {
        if !self.raises.contains(&typ) {
            self.raises.push(typ);
        }
    }

    /// Add a tag to the effect set
    pub fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
        }
    }

    /// Merge another effect set into this one
    pub fn merge(&mut self, other: &EffectSet) {
        for exc in &other.raises {
            self.add_raise(exc.clone());
        }
        for tag in &other.tags {
            self.add_tag(tag);
        }
    }

    /// Check if this effect set includes a specific exception
    pub fn includes_exception(&self, typ: &ExceptionType) -> bool {
        self.raises.iter().any(|e| e == typ)
    }

    /// Get all exceptions in this set
    pub fn get_raises(&self) -> &[ExceptionType] {
        &self.raises
    }

    /// Get all tags in this set
    pub fn get_tags(&self) -> &[String] {
        &self.tags
    }
}

/// Exception checker for semantic analysis
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExceptionChecker {
    registry: ExceptionRegistry,
    /// Current effect set being computed
    current_effects: EffectSet,
}

impl ExceptionChecker {
    pub fn new() -> Self {
        ExceptionChecker {
            registry: ExceptionRegistry::new(),
            current_effects: EffectSet::new(),
        }
    }

    /// Get the exception registry
    pub fn registry(&self) -> &ExceptionRegistry {
        &self.registry
    }

    /// Check if an exception can be caught by an except clause
    pub fn can_catch(&self, raised: &ExceptionType, caught: &ExceptionType) -> bool {
        if caught == &ExceptionType::Exception {
            return true;
        }
        self.registry.is_subtype(raised, caught)
    }

    /// Get the effect set for a try/except statement
    /// Removes exceptions that are caught by the except clauses
    pub fn analyze_try(&self, try_effects: &EffectSet, excepts: &[ExceptClause]) -> EffectSet {
        let mut result = try_effects.clone();
        for exc in excepts {
            if let Some(ref catch_type) = exc.catch_type {
                // Remove this exception type from the result since it's caught
                result
                    .raises
                    .retain(|t| !self.registry.is_subtype(t, catch_type));
            }
        }
        result
    }

    /// Check if a statement has no exception effects
    #[allow(unused_variables)]
    pub fn is_no_raise(&self, stmt: &str) -> bool {
        false
    }
}

impl Default for ExceptionChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_exception_new() {
        let exc = Exception::new(
            ExceptionType::ValueError,
            "Invalid value".to_string(),
            Span::new(FileId::new(0), 0, 0),
        );
        assert!(matches!(exc.typ, ExceptionType::ValueError));
        assert_eq!(exc.message, "Invalid value");
        assert!(exc.cause.is_none());
    }

    #[test]
    fn test_exception_with_cause() {
        let cause = Exception::new(
            ExceptionType::IOError,
            "File not found".to_string(),
            Span::new(FileId::new(0), 0, 0),
        );
        let exc = Exception::new(
            ExceptionType::RuntimeError,
            "Operation failed".to_string(),
            Span::new(FileId::new(0), 0, 0),
        )
        .with_cause(cause);
        assert!(exc.cause.is_some());
    }

    #[test]
    fn test_exception_registry_new() {
        let registry = ExceptionRegistry::new();
        assert!(registry.contains(&ExceptionType::ValueError));
        assert!(registry.contains(&ExceptionType::IndexError));
    }

    #[test]
    fn test_exception_info_parent() {
        let registry = ExceptionRegistry::new();
        let info = ExceptionInfo::new(ExceptionType::ValueError, "ValueError")
            .with_parent(ExceptionType::Exception);
        assert!(info.is_subtype_of(&ExceptionType::Exception, &registry));
    }

    #[test]
    fn test_exception_info_no_parent() {
        let registry = ExceptionRegistry::new();
        let info = ExceptionInfo::new(ExceptionType::Exception, "Exception");
        assert!(info.is_subtype_of(&ExceptionType::Exception, &registry));
    }

    #[test]
    fn test_exception_registry_is_subtype() {
        let registry = ExceptionRegistry::new();
        assert!(registry.is_subtype(&ExceptionType::ValueError, &ExceptionType::Exception));
        assert!(registry.is_subtype(&ExceptionType::Exception, &ExceptionType::Exception));
        assert!(!registry.is_subtype(&ExceptionType::ValueError, &ExceptionType::IndexError));
    }

    #[test]
    fn test_effect_set_new() {
        let effects = EffectSet::new();
        assert!(effects.get_raises().is_empty());
        assert!(effects.get_tags().is_empty());
    }

    #[test]
    fn test_effect_set_add_raise() {
        let mut effects = EffectSet::new();
        effects.add_raise(ExceptionType::ValueError);
        assert!(effects.includes_exception(&ExceptionType::ValueError));
        assert!(!effects.includes_exception(&ExceptionType::IndexError));
    }

    #[test]
    fn test_effect_set_add_tag() {
        let mut effects = EffectSet::new();
        effects.add_tag("io");
        effects.add_tag("memory");
        assert_eq!(effects.get_tags().len(), 2);
    }

    #[test]
    fn test_effect_set_merge() {
        let mut effects1 = EffectSet::new();
        effects1.add_raise(ExceptionType::ValueError);

        let mut effects2 = EffectSet::new();
        effects2.add_raise(ExceptionType::IndexError);
        effects2.add_tag("io");

        effects1.merge(&effects2);
        assert!(effects1.includes_exception(&ExceptionType::ValueError));
        assert!(effects1.includes_exception(&ExceptionType::IndexError));
        assert_eq!(effects1.get_tags().len(), 1);
    }

    #[test]
    fn test_exception_checker_can_catch() {
        let checker = ExceptionChecker::new();
        assert!(checker.can_catch(&ExceptionType::ValueError, &ExceptionType::Exception));
        assert!(checker.can_catch(&ExceptionType::ValueError, &ExceptionType::ValueError));
        assert!(!checker.can_catch(&ExceptionType::Exception, &ExceptionType::ValueError));
    }

    #[test]
    fn test_exception_checker_analyze_try() {
        let checker = ExceptionChecker::new();
        let mut try_effects = EffectSet::new();
        try_effects.add_raise(ExceptionType::ValueError);

        let excepts = vec![ExceptClause {
            catch_type: Some(ExceptionType::ValueError),
            bind_name: Some("e".to_string()),
            body: vec!["echo e".to_string()],
            span: Span::new(FileId::new(0), 0, 0),
        }];

        let result = checker.analyze_try(&try_effects, &excepts);
        assert!(!result.includes_exception(&ExceptionType::ValueError));
    }

    #[test]
    fn test_exception_type_variants() {
        assert!(matches!(ExceptionType::Exception, ExceptionType::Exception));
        assert!(matches!(
            ExceptionType::ValueError,
            ExceptionType::ValueError
        ));
        assert!(matches!(
            ExceptionType::IndexError,
            ExceptionType::IndexError
        ));
        let custom = ExceptionType::Custom("MyError".to_string());
        assert!(matches!(custom, ExceptionType::Custom(_)));
    }

    #[test]
    fn test_try_stmt() {
        let try_stmt = TryStmt {
            try_body: vec!["echo 1".to_string()],
            excepts: vec![ExceptClause {
                catch_type: Some(ExceptionType::ValueError),
                bind_name: None,
                body: vec!["echo error".to_string()],
                span: Span::new(FileId::new(0), 0, 0),
            }],
            finally_body: Some(vec!["echo done".to_string()]),
            span: Span::new(FileId::new(0), 0, 0),
        };
        assert_eq!(try_stmt.try_body.len(), 1);
        assert_eq!(try_stmt.excepts.len(), 1);
        assert!(try_stmt.finally_body.is_some());
    }

    #[test]
    fn test_defer_stmt() {
        let defer_stmt = DeferStmt {
            body: vec!["close()".to_string()],
            span: Span::new(FileId::new(0), 0, 0),
        };
        assert_eq!(defer_stmt.body.len(), 1);
    }

    #[test]
    fn test_exception_checker_try_body() {
        let try_body = vec!["echo 1".to_string()];
        let excepts = vec![ExceptClause {
            span: Span::new(FileId::new(0), 0, 0),
            catch_type: Some(ExceptionType::ValueError),
            bind_name: None,
            body: vec!["echo error".to_string()],
        }];
        let try_stmt = TryStmt {
            try_body,
            excepts,
            finally_body: None,
            span: Span::new(FileId::new(0), 0, 0),
        };
        assert_eq!(try_stmt.try_body.len(), 1);
        assert_eq!(try_stmt.excepts.len(), 1);
        assert!(try_stmt.finally_body.is_none());
    }

    #[test]
    fn test_effect_set_includes_multiple() {
        let mut effects = EffectSet::new();
        effects.add_raise(ExceptionType::ValueError);
        effects.add_raise(ExceptionType::IOError);
        assert!(effects.includes_exception(&ExceptionType::ValueError));
        assert!(effects.includes_exception(&ExceptionType::IOError));
        assert!(!effects.includes_exception(&ExceptionType::IndexError));
    }

    #[test]
    fn test_effect_set_empty_after_clear() {
        let mut effects = EffectSet::new();
        effects.add_raise(ExceptionType::ValueError);
        effects.add_tag("io");
        effects.raises.clear();
        effects.tags.clear();
        assert!(effects.get_raises().is_empty());
        assert!(effects.get_tags().is_empty());
    }

    #[test]
    fn test_exception_type_display() {
        assert!(format!("{:?}", ExceptionType::ValueError).contains("ValueError"));
        assert!(format!("{:?}", ExceptionType::RuntimeError).contains("RuntimeError"));
        assert!(format!("{:?}", ExceptionType::IOError).contains("IOError"));
    }

    #[test]
    fn test_exception_checker_registry() {
        let checker = ExceptionChecker::new();
        let registry = checker.registry();
        assert!(registry.contains(&ExceptionType::ValueError));
    }

    #[test]
    fn test_exception_info_name() {
        let info = ExceptionInfo::new(ExceptionType::ValueError, "ValueError");
        assert!(info.name.contains("ValueError"));
    }
}
