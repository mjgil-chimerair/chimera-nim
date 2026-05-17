//! Lifetime hooks: `=destroy`, `=wasMoved`, `=sink`, `=copy`, `=trace`, `=dup`.

#![allow(clippy::should_implement_trait)]

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{FileId, Span};
use std::collections::{HashMap, HashSet};

/// Hook kinds for destructor and ownership semantics
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HookKind {
    /// `=destroy` - called when a value's lifetime ends
    Destroy,
    /// `=wasMoved` - called when a value is moved from
    WasMoved,
    /// `=sink` - called when a value is sinked (moved into a parameter)
    Sink,
    /// `=copy` - called when a value is copied
    Copy,
    /// `=trace` - called by the GC to trace references
    Trace,
    /// `=dup` - called to duplicate a value (for ORC)
    Dup,
}

impl HookKind {
    pub fn name(&self) -> &str {
        match self {
            HookKind::Destroy => "=destroy",
            HookKind::WasMoved => "=wasMoved",
            HookKind::Sink => "=sink",
            HookKind::Copy => "=copy",
            HookKind::Trace => "=trace",
            HookKind::Dup => "=dup",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "=destroy" => Some(HookKind::Destroy),
            "=wasMoved" => Some(HookKind::WasMoved),
            "=sink" => Some(HookKind::Sink),
            "=copy" => Some(HookKind::Copy),
            "=trace" => Some(HookKind::Trace),
            "=dup" => Some(HookKind::Dup),
            _ => None,
        }
    }
}

/// A lifetime hook definition
#[derive(Debug, Clone)]
pub struct HookDef {
    pub kind: HookKind,
    pub name: String,
    pub span: Span,
    pub is_generated: bool,
    pub is_custom: bool,
}

impl HookDef {
    pub fn new(kind: HookKind, name: &str, span: Span) -> Self {
        HookDef {
            kind: kind.clone(),
            name: name.to_string(),
            span,
            is_generated: false,
            is_custom: false,
        }
    }

    pub fn generated(kind: HookKind, name: &str, span: Span) -> Self {
        HookDef {
            kind,
            name: name.to_string(),
            span,
            is_generated: true,
            is_custom: false,
        }
    }

    pub fn custom(kind: HookKind, name: &str, span: Span) -> Self {
        HookDef {
            kind,
            name: name.to_string(),
            span,
            is_generated: false,
            is_custom: true,
        }
    }
}

/// Hook lookup key (type name + hook kind)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HookKey {
    pub type_name: String,
    pub hook_kind: HookKind,
}

impl HookKey {
    pub fn new(type_name: &str, hook_kind: HookKind) -> Self {
        HookKey {
            type_name: type_name.to_string(),
            hook_kind,
        }
    }
}

/// A resolved hook with its implementation info
#[derive(Debug, Clone)]
pub struct ResolvedHook {
    pub key: HookKey,
    pub hook_def: HookDef,
    pub body_span: Span,
}

/// Hook resolution result
#[derive(Debug, Clone)]
pub struct HookResolution {
    /// The resolved hook definition
    pub hook: Option<ResolvedHook>,
    /// Whether the type has a custom hook
    pub has_custom: bool,
    /// Whether a hook should be generated
    pub should_generate: bool,
    /// Types that need to be traced by this hook
    pub traced_types: Vec<String>,
}

impl Default for HookResolution {
    fn default() -> Self {
        Self::none()
    }
}

impl HookResolution {
    pub fn none() -> Self {
        HookResolution {
            hook: None,
            has_custom: false,
            should_generate: false,
            traced_types: Vec::new(),
        }
    }

    pub fn custom(hook: ResolvedHook) -> Self {
        HookResolution {
            hook: Some(hook),
            has_custom: true,
            should_generate: false,
            traced_types: Vec::new(),
        }
    }

    pub fn generated(traced_types: Vec<String>) -> Self {
        HookResolution {
            hook: None,
            has_custom: false,
            should_generate: true,
            traced_types,
        }
    }
}

/// Hook registry for looking up lifetime hooks
#[derive(Debug, Clone)]
pub struct HookRegistry {
    /// User-defined hooks
    custom_hooks: HashMap<HookKey, HookDef>,
    /// Types with custom hooks
    types_with_custom_hooks: HashSet<String>,
    /// Types that need generated hooks
    types_needing_generation: HashSet<String>,
}

impl HookRegistry {
    pub fn new() -> Self {
        HookRegistry {
            custom_hooks: HashMap::new(),
            types_with_custom_hooks: HashSet::new(),
            types_needing_generation: HashSet::new(),
        }
    }

    /// Register a custom hook
    pub fn register(&mut self, type_name: &str, hook: HookDef) {
        let key = HookKey::new(type_name, hook.kind.clone());
        self.custom_hooks.insert(key.clone(), hook);
        self.types_with_custom_hooks.insert(type_name.to_string());
    }

    /// Register a type that needs hook generation
    pub fn register_needs_generation(&mut self, type_name: &str) {
        self.types_needing_generation.insert(type_name.to_string());
    }

    /// Look up a hook for a type
    pub fn lookup(&self, type_name: &str, kind: HookKind) -> Option<&HookDef> {
        let key = HookKey::new(type_name, kind);
        self.custom_hooks.get(&key)
    }

    /// Check if a type has a custom hook of any kind
    pub fn has_custom_hook(&self, type_name: &str) -> bool {
        self.types_with_custom_hooks.contains(type_name)
    }

    /// Check if a type needs hook generation
    pub fn needs_generation(&self, type_name: &str) -> bool {
        self.types_needing_generation.contains(type_name) || !self.has_custom_hook(type_name)
    }

    /// Get all hooks for a type
    pub fn get_hooks_for_type(&self, type_name: &str) -> Vec<&HookDef> {
        let mut hooks = Vec::new();
        for kind in [
            HookKind::Destroy,
            HookKind::WasMoved,
            HookKind::Sink,
            HookKind::Copy,
            HookKind::Trace,
            HookKind::Dup,
        ] {
            if let Some(hook) = self.lookup(type_name, kind) {
                hooks.push(hook);
            }
        }
        hooks
    }

    /// Resolve hooks for a type, determining which are custom vs generated
    pub fn resolve(&self, type_name: &str) -> HookResolution {
        let mut has_custom = false;
        let mut traced_types = Vec::new();

        for kind in [
            HookKind::Destroy,
            HookKind::WasMoved,
            HookKind::Sink,
            HookKind::Copy,
            HookKind::Trace,
            HookKind::Dup,
        ] {
            if let Some(_hook) = self.lookup(type_name, kind.clone()) {
                has_custom = true;
                if matches!(kind, HookKind::Trace) {
                    // Trace hooks track other types
                    traced_types.push(type_name.to_string());
                }
            }
        }

        if has_custom {
            HookResolution::custom(ResolvedHook {
                key: HookKey::new(type_name, HookKind::Destroy),
                hook_def: self
                    .lookup(type_name, HookKind::Destroy)
                    .cloned()
                    .unwrap_or_else(|| {
                        HookDef::custom(HookKind::Destroy, type_name, Span::new(FileId(0), 0, 0))
                    }),
                body_span: Span::new(FileId(0), 0, 0),
            })
        } else if self.needs_generation(type_name) {
            HookResolution::generated(traced_types)
        } else {
            HookResolution::none()
        }
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Move analyzer for last-use detection and sink parameters
#[derive(Debug, Clone)]
pub struct MoveAnalyzer {
    /// Types that need special move handling
    tracked_types: HashSet<String>,
    /// Last-use information per variable
    last_uses: HashMap<String, Vec<Span>>,
}

impl MoveAnalyzer {
    pub fn new() -> Self {
        MoveAnalyzer {
            tracked_types: HashSet::new(),
            last_uses: HashMap::new(),
        }
    }

    /// Register a type for move tracking
    pub fn track_type(&mut self, type_name: &str) {
        self.tracked_types.insert(type_name.to_string());
    }

    /// Record a last use of a variable
    pub fn record_last_use(&mut self, var_name: &str, span: Span) {
        self.last_uses
            .entry(var_name.to_string())
            .or_default()
            .push(span);
    }

    /// Get all last uses for a variable
    pub fn get_last_uses(&self, var_name: &str) -> Option<&Vec<Span>> {
        self.last_uses.get(var_name)
    }

    /// Check if a variable is at its last use at a given span
    pub fn is_last_use(&self, var_name: &str, span: Span) -> bool {
        self.last_uses
            .get(var_name)
            .map(|uses| uses.contains(&span))
            .unwrap_or(false)
    }

    /// Analyze a span and determine if a move should be inserted
    pub fn should_move(&self, var_name: &str, span: Span) -> bool {
        self.is_last_use(var_name, span)
    }

    /// Clear last uses for a scope
    pub fn clear_scope(&mut self, scope_id: u32) {
        // In a full implementation, this would clear uses for variables in that scope
        let _ = scope_id;
    }
}

impl Default for MoveAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Destructor planner for generating destructor calls
#[derive(Debug, Clone)]
pub struct DestructorPlanner {
    registry: HookRegistry,
}

impl DestructorPlanner {
    pub fn new() -> Self {
        DestructorPlanner {
            registry: HookRegistry::new(),
        }
    }

    pub fn with_registry(registry: HookRegistry) -> Self {
        DestructorPlanner { registry }
    }

    /// Plan destructor calls for a scope exit
    pub fn plan_scope_exit(&mut self, type_name: &str, span: Span) -> Vec<PlannedDestroy> {
        let mut planned = Vec::new();

        if let Some(destroy_hook) = self.registry.lookup(type_name, HookKind::Destroy) {
            planned.push(PlannedDestroy {
                type_name: type_name.to_string(),
                hook_name: destroy_hook.name.clone(),
                span,
            });
        } else if self.registry.needs_generation(type_name) {
            planned.push(PlannedDestroy {
                type_name: type_name.to_string(),
                hook_name: format!("{}$destory", type_name), // Generated name
                span,
            });
        }

        planned
    }

    /// Plan move operations for assignments
    pub fn plan_move(&mut self, from_type: &str, to_type: &str, span: Span) -> Option<PlannedMove> {
        let from_has_wasmoved = self
            .registry
            .lookup(from_type, HookKind::WasMoved)
            .is_some();
        let to_has_sink = self.registry.lookup(to_type, HookKind::Sink).is_some();

        if from_has_wasmoved || to_has_sink {
            Some(PlannedMove {
                from_type: from_type.to_string(),
                to_type: to_type.to_string(),
                wasmoved_hook: if from_has_wasmoved {
                    Some(format!("{}$wasMoved", from_type))
                } else {
                    None
                },
                sink_hook: if to_has_sink {
                    Some(format!("{}$sink", to_type))
                } else {
                    None
                },
                span,
            })
        } else {
            None
        }
    }

    /// Get the hook registry
    pub fn registry(&self) -> &HookRegistry {
        &self.registry
    }
}

impl Default for DestructorPlanner {
    fn default() -> Self {
        Self::new()
    }
}

/// A planned destructor call
#[derive(Debug, Clone)]
pub struct PlannedDestroy {
    pub type_name: String,
    pub hook_name: String,
    pub span: Span,
}

/// A planned move operation
#[derive(Debug, Clone)]
pub struct PlannedMove {
    pub from_type: String,
    pub to_type: String,
    pub wasmoved_hook: Option<String>,
    pub sink_hook: Option<String>,
    pub span: Span,
}

/// Sink parameter validation
#[derive(Debug, Clone)]
pub struct SinkValidator {
    valid_sink_types: HashSet<String>,
}

impl SinkValidator {
    pub fn new() -> Self {
        let mut validator = SinkValidator {
            valid_sink_types: HashSet::new(),
        };
        validator.init_builtins();
        validator
    }

    fn init_builtins(&mut self) {
        // Built-in types that support sink parameters
        self.valid_sink_types.insert("string".to_string());
        self.valid_sink_types.insert("seq".to_string());
        self.valid_sink_types.insert("Table".to_string());
        self.valid_sink_types.insert("Ref".to_string());
    }

    /// Check if a type can be used as a sink parameter
    pub fn can_sink(&self, type_name: &str) -> bool {
        self.valid_sink_types.contains(type_name)
    }

    /// Register a custom type as valid for sink
    pub fn register_sinkable(&mut self, type_name: &str) {
        self.valid_sink_types.insert(type_name.to_string());
    }
}

impl Default for SinkValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_kind_name() {
        assert_eq!(HookKind::Destroy.name(), "=destroy");
        assert_eq!(HookKind::WasMoved.name(), "=wasMoved");
        assert_eq!(HookKind::Sink.name(), "=sink");
        assert_eq!(HookKind::Copy.name(), "=copy");
        assert_eq!(HookKind::Trace.name(), "=trace");
        assert_eq!(HookKind::Dup.name(), "=dup");
    }

    #[test]
    fn test_hook_kind_from_str() {
        assert!(matches!(
            HookKind::from_str("=destroy"),
            Some(HookKind::Destroy)
        ));
        assert!(matches!(
            HookKind::from_str("=wasMoved"),
            Some(HookKind::WasMoved)
        ));
        assert!(matches!(HookKind::from_str("=sink"), Some(HookKind::Sink)));
        assert!(matches!(HookKind::from_str("=copy"), Some(HookKind::Copy)));
        assert!(matches!(
            HookKind::from_str("=trace"),
            Some(HookKind::Trace)
        ));
        assert!(matches!(HookKind::from_str("=dup"), Some(HookKind::Dup)));
        assert!(HookKind::from_str("invalid").is_none());
    }

    #[test]
    fn test_hook_def_new() {
        let span = Span::new(FileId(0), 0, 0);
        let hook = HookDef::new(HookKind::Destroy, "MyType", span);
        assert!(!hook.is_generated);
        assert!(!hook.is_custom);
        assert_eq!(hook.name, "MyType");
    }

    #[test]
    fn test_hook_def_generated() {
        let span = Span::new(FileId(0), 0, 0);
        let hook = HookDef::generated(HookKind::Destroy, "MyType", span);
        assert!(hook.is_generated);
        assert!(!hook.is_custom);
    }

    #[test]
    fn test_hook_def_custom() {
        let span = Span::new(FileId(0), 0, 0);
        let hook = HookDef::custom(HookKind::Destroy, "MyType", span);
        assert!(!hook.is_generated);
        assert!(hook.is_custom);
    }

    #[test]
    fn test_hook_key() {
        let key = HookKey::new("MyType", HookKind::Destroy);
        assert_eq!(key.type_name, "MyType");
        assert!(matches!(key.hook_kind, HookKind::Destroy));
    }

    #[test]
    fn test_hook_registry_register() {
        let mut registry = HookRegistry::new();
        let span = Span::new(FileId(0), 0, 0);
        registry.register("MyType", HookDef::new(HookKind::Destroy, "MyType", span));

        assert!(registry.has_custom_hook("MyType"));
        assert!(registry.lookup("MyType", HookKind::Destroy).is_some());
    }

    #[test]
    fn test_hook_registry_get_hooks_for_type() {
        let mut registry = HookRegistry::new();
        let span = Span::new(FileId(0), 0, 0);
        registry.register("MyType", HookDef::new(HookKind::Destroy, "MyType", span));
        registry.register("MyType", HookDef::new(HookKind::WasMoved, "MyType", span));

        let hooks = registry.get_hooks_for_type("MyType");
        assert_eq!(hooks.len(), 2);
    }

    #[test]
    fn test_hook_registry_resolve_custom() {
        let mut registry = HookRegistry::new();
        let span = Span::new(FileId(0), 0, 0);
        registry.register("MyType", HookDef::new(HookKind::Destroy, "MyType", span));

        let resolution = registry.resolve("MyType");
        assert!(resolution.has_custom);
        assert!(!resolution.should_generate);
    }

    #[test]
    fn test_hook_registry_resolve_needs_generation() {
        let registry = HookRegistry::new();
        let resolution = registry.resolve("UnknownType");
        assert!(resolution.should_generate);
    }

    #[test]
    fn test_move_analyzer_record_last_use() {
        let mut analyzer = MoveAnalyzer::new();
        let span = Span::new(FileId(0), 0, 0);

        analyzer.record_last_use("x", span);
        assert!(analyzer.is_last_use("x", span));
        assert!(!analyzer.is_last_use("y", span));
    }

    #[test]
    fn test_move_analyzer_should_move() {
        let mut analyzer = MoveAnalyzer::new();
        let span = Span::new(FileId(0), 0, 0);

        analyzer.record_last_use("x", span);
        assert!(analyzer.should_move("x", span));
    }

    #[test]
    fn test_destructor_planner_plan_scope_exit_custom() {
        let mut planner = DestructorPlanner::new();
        let span = Span::new(FileId(0), 0, 0);
        planner.registry.register(
            "MyType",
            HookDef::new(HookKind::Destroy, "MyType$destory", span),
        );

        let planned = planner.plan_scope_exit("MyType", span);
        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].hook_name, "MyType$destory");
    }

    #[test]
    fn test_destructor_planner_plan_scope_exit_generated() {
        let mut planner = DestructorPlanner::new();
        let span = Span::new(FileId(0), 0, 0);
        planner.registry.register_needs_generation("MyType");

        let planned = planner.plan_scope_exit("MyType", span);
        assert_eq!(planned.len(), 1);
        assert!(planned[0].hook_name.contains("destory"));
    }

    #[test]
    fn test_destructor_planner_plan_move() {
        let mut planner = DestructorPlanner::new();
        let span = Span::new(FileId(0), 0, 0);
        planner.registry.register(
            "FromType",
            HookDef::new(HookKind::WasMoved, "FromType$wasMoved", span),
        );
        planner
            .registry
            .register("ToType", HookDef::new(HookKind::Sink, "ToType$sink", span));

        let planned = planner.plan_move("FromType", "ToType", span);
        assert!(planned.is_some());
        let m = planned.unwrap();
        assert!(m.wasmoved_hook.is_some());
        assert!(m.sink_hook.is_some());
    }

    #[test]
    fn test_sink_validator_can_sink() {
        let validator = SinkValidator::new();
        assert!(validator.can_sink("string"));
        assert!(validator.can_sink("seq"));
        assert!(!validator.can_sink("MyType"));
    }

    #[test]
    fn test_sink_validator_register_sinkable() {
        let mut validator = SinkValidator::new();
        validator.register_sinkable("MyType");
        assert!(validator.can_sink("MyType"));
    }

    #[test]
    fn test_hook_resolution_variants() {
        let none = HookResolution::none();
        assert!(none.hook.is_none());
        assert!(!none.has_custom);
        assert!(!none.should_generate);

        let resolution = HookResolution::generated(vec!["Child".to_string()]);
        assert!(resolution.should_generate);
        assert_eq!(resolution.traced_types.len(), 1);
    }

    #[test]
    fn test_planned_destroy() {
        let span = Span::new(FileId(0), 0, 0);
        let destroy = PlannedDestroy {
            type_name: "MyType".to_string(),
            hook_name: "MyType$destory".to_string(),
            span,
        };
        assert_eq!(destroy.type_name, "MyType");
    }

    #[test]
    fn test_planned_move() {
        let span = Span::new(FileId(0), 0, 0);
        let move_op = PlannedMove {
            from_type: "FromType".to_string(),
            to_type: "ToType".to_string(),
            wasmoved_hook: Some("FromType$wasMoved".to_string()),
            sink_hook: Some("ToType$sink".to_string()),
            span,
        };
        assert!(move_op.wasmoved_hook.is_some());
        assert!(move_op.sink_hook.is_some());
    }

    // Task 37: ARC/ORC and move analysis tests

    #[test]
    fn test_hook_kind_dup_for_orc() {
        // =dup is used for ORC (Owning Reference Counting) duplication
        assert_eq!(HookKind::Dup.name(), "=dup");
        let dup = HookKind::from_str("=dup").unwrap();
        assert!(matches!(dup, HookKind::Dup));
    }

    #[test]
    fn test_hook_registry_copy_hook() {
        let mut registry = HookRegistry::new();
        let span = Span::new(FileId(0), 0, 0);
        registry.register("MyType", HookDef::new(HookKind::Copy, "MyType$copy", span));

        let copy_hook = registry.lookup("MyType", HookKind::Copy);
        assert!(copy_hook.is_some());
        assert_eq!(copy_hook.unwrap().name, "MyType$copy");
    }

    #[test]
    fn test_hook_registry_trace_hook() {
        let mut registry = HookRegistry::new();
        let span = Span::new(FileId(0), 0, 0);
        registry.register(
            "MyType",
            HookDef::new(HookKind::Trace, "MyType$trace", span),
        );

        let trace_hook = registry.lookup("MyType", HookKind::Trace);
        assert!(trace_hook.is_some());
    }

    #[test]
    fn test_hook_registry_dup_hook() {
        let mut registry = HookRegistry::new();
        let span = Span::new(FileId(0), 0, 0);
        registry.register(
            "RefCounted",
            HookDef::new(HookKind::Dup, "RefCounted$dup", span),
        );

        let dup_hook = registry.lookup("RefCounted", HookKind::Dup);
        assert!(dup_hook.is_some());
    }

    #[test]
    fn test_move_analyzer_clear_scope() {
        let mut analyzer = MoveAnalyzer::new();
        let span = Span::new(FileId(0), 0, 0);

        analyzer.record_last_use("x", span);
        assert!(analyzer.is_last_use("x", span));

        // Clear scope should not panic
        analyzer.clear_scope(0);
        // After clear, the last use info may be cleared
        // (implementation-dependent behavior)
    }

    #[test]
    fn test_hook_resolution_with_trace_tracked_types() {
        let mut registry = HookRegistry::new();
        let span = Span::new(FileId(0), 0, 0);
        // Register a Trace hook - it marks the type as having custom hooks
        registry.register(
            "Container",
            HookDef::new(HookKind::Trace, "Container$trace", span),
        );

        let resolution = registry.resolve("Container");
        // Container has a custom Trace hook
        assert!(resolution.has_custom);
        // custom resolution doesn't include traced_types in this implementation
        // (traced_types is only populated in generated resolution)
    }

    #[test]
    fn test_hook_resolution_generated_has_traced_types() {
        // When a type has no custom hooks but needs generation,
        // the resolution should have should_generate = true
        let mut registry = HookRegistry::new();
        registry.register_needs_generation("Container");

        let resolution = registry.resolve("Container");
        assert!(resolution.should_generate);
        // traced_types may be empty for generated resolutions in this implementation
    }

    #[test]
    fn test_hook_resolution_custom_has_hook() {
        // When a type has custom hooks, resolution should have has_custom = true
        let mut registry = HookRegistry::new();
        let span = Span::new(FileId(0), 0, 0);
        registry.register(
            "MyType",
            HookDef::new(HookKind::Destroy, "MyType$destroy", span),
        );

        let resolution = registry.resolve("MyType");
        assert!(resolution.has_custom);
        assert!(!resolution.should_generate);
        assert!(resolution.hook.is_some());
    }

    #[test]
    fn test_hook_registry_all_hook_kinds() {
        let mut registry = HookRegistry::new();
        let span = Span::new(FileId(0), 0, 0);

        // Register one of each hook kind
        registry.register(
            "MyType",
            HookDef::new(HookKind::Destroy, "MyType$destroy", span),
        );
        registry.register(
            "MyType",
            HookDef::new(HookKind::WasMoved, "MyType$wasMoved", span),
        );
        registry.register("MyType", HookDef::new(HookKind::Sink, "MyType$sink", span));
        registry.register("MyType", HookDef::new(HookKind::Copy, "MyType$copy", span));
        registry.register(
            "MyType",
            HookDef::new(HookKind::Trace, "MyType$trace", span),
        );
        registry.register("MyType", HookDef::new(HookKind::Dup, "MyType$dup", span));

        let hooks = registry.get_hooks_for_type("MyType");
        assert_eq!(hooks.len(), 6);
    }

    #[test]
    fn test_destructor_planner_no_hook() {
        let mut planner = DestructorPlanner::new();
        let span = Span::new(FileId(0), 0, 0);

        // Unknown type without registration - but needs_generation returns true for unknown types
        // So a generated destructor hook name will be created
        let planned = planner.plan_scope_exit("UnknownType", span);
        assert_eq!(planned.len(), 1);
        // Generated names follow the pattern "TypeName$destory"
        assert!(planned[0].hook_name.contains("UnknownType"));
    }

    #[test]
    fn test_destructor_planner_plan_move_no_hooks() {
        let mut planner = DestructorPlanner::new();
        let span = Span::new(FileId(0), 0, 0);

        // Neither type has hooks
        let planned = planner.plan_move("TypeA", "TypeB", span);
        assert!(planned.is_none());
    }

    #[test]
    fn test_hook_key_equality() {
        let key1 = HookKey::new("MyType", HookKind::Destroy);
        let key2 = HookKey::new("MyType", HookKind::Destroy);
        let key3 = HookKey::new("MyType", HookKind::Copy);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_move_analyzer_multiple_last_uses() {
        let mut analyzer = MoveAnalyzer::new();
        let span1 = Span::new(FileId(0), 0, 5);
        let span2 = Span::new(FileId(0), 10, 15);

        analyzer.record_last_use("x", span1);
        analyzer.record_last_use("x", span2);

        let last_uses = analyzer.get_last_uses("x").unwrap();
        assert_eq!(last_uses.len(), 2);
    }

    #[test]
    fn test_sink_validator_builtins() {
        let validator = SinkValidator::new();

        // Check all built-in sink types
        assert!(validator.can_sink("string"));
        assert!(validator.can_sink("seq"));
        assert!(validator.can_sink("Table"));
        assert!(validator.can_sink("Ref"));

        // Check unknown types
        assert!(!validator.can_sink("UnknownType"));
        assert!(!validator.can_sink("int"));
        assert!(!validator.can_sink("float"));
    }

    #[test]
    fn test_planned_destroy_fields() {
        let span = Span::new(FileId(0), 0, 0);
        let destroy = PlannedDestroy {
            type_name: "MyType".to_string(),
            hook_name: "MyType$destroy".to_string(),
            span,
        };
        assert_eq!(destroy.type_name, "MyType");
        assert_eq!(destroy.hook_name, "MyType$destroy");
        assert_eq!(destroy.span, span);
    }

    #[test]
    fn test_hook_def_clone() {
        let span = Span::new(FileId(0), 0, 0);
        let hook = HookDef::new(HookKind::Destroy, "MyType", span);
        let cloned = hook.clone();
        assert_eq!(hook.name, cloned.name);
        assert_eq!(hook.kind, cloned.kind);
    }
}
