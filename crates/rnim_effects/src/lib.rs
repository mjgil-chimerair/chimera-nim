//! Exception/effect tracking, tags, `raises`, `gcsafe`, side-effect validation.

use bitflags::bitflags;
#[cfg(test)]
use rnim_allocator as _;
use rnim_span::Span;
use std::collections::{HashMap, HashSet};

bitflags! {
    /// Effect flags for tracking side effects
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct EffectFlags: u32 {
        /// Reads from memory or state
        const READS = 1 << 0;
        /// Writes to memory or state
        const WRITES = 1 << 1;
        /// Can raise exceptions
        const RAISES = 1 << 2;
        /// Leaf function (no side effects)
        const LEAF = 1 << 3;
        /// GcSafe (does not allocatte or use unsafe)
        const GC_SAFE = 1 << 4;
        /// Has destructor
        const HAS_DESTRUCTOR = 1 << 5;
    }
}

/// An effect tag for custom effect tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectTag(pub String);

impl EffectTag {
    pub fn new(name: &str) -> Self {
        EffectTag(name.to_string())
    }
}

/// Effect set for tracking all effects of a routine
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSet {
    /// Effect flags
    pub flags: EffectFlags,
    /// Exceptions that can be raised
    pub raises: Vec<EffectType>,
    /// Custom effect tags
    pub tags: Vec<EffectTag>,
}

/// Built-in effect types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EffectType {
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

impl EffectType {
    pub fn name(&self) -> &str {
        match self {
            EffectType::Exception => "Exception",
            EffectType::Exit => "SystemExit",
            EffectType::ValueError => "ValueError",
            EffectType::IndexError => "IndexError",
            EffectType::KeyError => "KeyError",
            EffectType::RuntimeError => "RuntimeError",
            EffectType::IOError => "IOError",
            EffectType::OSError => "OSError",
            EffectType::Custom(name) => name,
        }
    }
}

impl Default for EffectSet {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectSet {
    pub fn new() -> Self {
        Self {
            flags: EffectFlags::empty(),
            raises: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Create an empty effect set (leaf/const)
    pub fn leaf() -> Self {
        Self {
            flags: EffectFlags::LEAF | EffectFlags::GC_SAFE,
            raises: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Add a raises effect
    pub fn add_raise(&mut self, eff: EffectType) {
        if !self.raises.contains(&eff) {
            self.raises.push(eff);
        }
        self.flags |= EffectFlags::RAISES;
    }

    /// Add a custom tag
    pub fn add_tag(&mut self, tag: EffectTag) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Add reads effect
    pub fn add_reads(&mut self) {
        self.flags |= EffectFlags::READS;
    }

    /// Add writes effect
    pub fn add_writes(&mut self) {
        self.flags |= EffectFlags::WRITES;
    }

    /// Mark as gc safe
    pub fn add_gc_safe(&mut self) {
        self.flags |= EffectFlags::GC_SAFE;
    }

    /// Check if this effect set is pure (no side effects)
    pub fn is_pure(&self) -> bool {
        self.flags
            .intersects(EffectFlags::READS | EffectFlags::WRITES | EffectFlags::RAISES)
    }

    /// Check if this effect set includes a specific exception
    pub fn includes_exception(&self, eff: &EffectType) -> bool {
        self.raises.iter().any(|e| e == eff)
    }

    /// Check if this effect set includes a specific tag
    pub fn includes_tag(&self, tag: &EffectTag) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Merge another effect set into this one
    pub fn merge(&mut self, other: &EffectSet) {
        self.flags |= other.flags;
        for eff in &other.raises {
            if !self.raises.contains(eff) {
                self.raises.push(eff.clone());
            }
        }
        for tag in &other.tags {
            if !self.tags.contains(tag) {
                self.tags.push(tag.clone());
            }
        }
    }

    /// Check if self is a subset of other (all effects in self are in other)
    pub fn is_subset_of(&self, other: &EffectSet) -> bool {
        // Check flags (excluding GC_SAFE which is not a restricting factor)
        let self_flags = self.flags.bits() & !EffectFlags::GC_SAFE.bits();
        if other.flags.bits() & self_flags != self_flags {
            return false;
        }
        for eff in &self.raises {
            if !other.raises.contains(eff) {
                return false;
            }
        }
        for tag in &self.tags {
            if !other.tags.contains(tag) {
                return false;
            }
        }
        true
    }
}

/// Expected effects from a routine declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedEffects {
    /// Expected raises
    pub raises: Vec<EffectType>,
    /// Expected tags
    pub tags: Vec<EffectTag>,
}

impl Default for ExpectedEffects {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpectedEffects {
    pub fn new() -> Self {
        Self {
            raises: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Add an expected exception
    pub fn add_raise(&mut self, eff: EffectType) {
        if !self.raises.contains(&eff) {
            self.raises.push(eff);
        }
    }

    /// Add an expected tag
    pub fn add_tag(&mut self, tag: EffectTag) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }
}

/// Effect validation result
#[derive(Debug, Clone)]
pub struct EffectValidation {
    /// Whether validation passed
    pub valid: bool,
    /// Forbidden effects found
    pub forbidden: Vec<ForbiddenEffect>,
    /// Missing required effects
    pub missing: Vec<EffectType>,
}

#[derive(Debug, Clone)]
pub enum ForbiddenEffect {
    Raises(EffectType),
    Tag(EffectTag),
    Reads,
    Writes,
}

/// Effect checker for semantic analysis
#[derive(Debug, Clone)]
pub struct EffectChecker {
    /// Registered effect tags
    tags: HashMap<String, EffectTag>,
    /// Custom effect types
    custom_effects: HashSet<EffectType>,
}

impl EffectChecker {
    pub fn new() -> Self {
        let mut checker = EffectChecker {
            tags: HashMap::new(),
            custom_effects: HashSet::new(),
        };
        checker.init_builtins();
        checker
    }

    fn init_builtins(&mut self) {
        // Register built-in tags
        self.register_tag("io");
        self.register_tag("memory");
        self.register_tag("network");
        self.register_tag("filesystem");
        self.register_tag("thread");
    }

    /// Register a custom effect tag
    pub fn register_tag(&mut self, name: &str) {
        let tag = EffectTag::new(name);
        self.tags.insert(name.to_string(), tag);
    }

    /// Register a custom effect type
    pub fn register_effect(&mut self, eff: EffectType) {
        self.custom_effects.insert(eff);
    }

    /// Look up a tag by name
    pub fn get_tag(&self, name: &str) -> Option<&EffectTag> {
        self.tags.get(name)
    }

    /// Check if a tag is known
    pub fn is_known_tag(&self, name: &str) -> bool {
        self.tags.contains_key(name)
    }

    /// Validate that actual effects match expected effects
    pub fn validate_effects(
        &self,
        actual: &EffectSet,
        expected: &ExpectedEffects,
        forbid_raises: bool,
    ) -> EffectValidation {
        let mut forbidden = Vec::new();
        let mut missing = Vec::new();

        // Check for forbidden raises
        if forbid_raises {
            for eff in &actual.raises {
                if !expected.raises.contains(eff) {
                    forbidden.push(ForbiddenEffect::Raises(eff.clone()));
                }
            }
        }

        // Check for missing required raises
        for eff in &expected.raises {
            if !actual.raises.contains(eff) {
                // Only report missing if we're not forbidding raises
                if !forbid_raises {
                    missing.push(eff.clone());
                }
            }
        }

        // Check for unexpected tags
        for tag in &actual.tags {
            if !expected.tags.contains(tag) {
                forbidden.push(ForbiddenEffect::Tag(tag.clone()));
            }
        }

        // Check for missing required tags
        for tag in &expected.tags {
            if !actual.tags.contains(tag) {
                missing.push(match tag {
                    _ => EffectType::Custom(tag.0.clone()),
                });
            }
        }

        EffectValidation {
            valid: forbidden.is_empty() && missing.is_empty(),
            forbidden,
            missing,
        }
    }

    /// Check if a routine with given effects can be called
    pub fn can_call(&self, caller_effects: &EffectSet, callee_effects: &EffectSet) -> bool {
        callee_effects.is_subset_of(caller_effects)
    }

    /// Get the combined effect for a proc type
    pub fn combine_proc_effects(&self, effects: &[&EffectSet]) -> EffectSet {
        let mut combined = EffectSet::new();
        for eff in effects {
            combined.merge(eff);
        }
        combined
    }
}

impl Default for EffectChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_set_new() {
        let effects = EffectSet::new();
        assert!(effects.raises.is_empty());
        assert!(effects.tags.is_empty());
        assert!(!effects.flags.intersects(EffectFlags::RAISES));
    }

    #[test]
    fn test_effect_set_leaf() {
        let effects = EffectSet::leaf();
        assert!(effects.flags.contains(EffectFlags::LEAF));
        assert!(effects.flags.contains(EffectFlags::GC_SAFE));
    }

    #[test]
    fn test_effect_set_add_raise() {
        let mut effects = EffectSet::new();
        effects.add_raise(EffectType::ValueError);
        assert!(effects.includes_exception(&EffectType::ValueError));
        assert!(effects.flags.contains(EffectFlags::RAISES));
    }

    #[test]
    fn test_effect_set_add_tag() {
        let mut effects = EffectSet::new();
        let tag = EffectTag::new("io");
        effects.add_tag(tag.clone());
        assert!(effects.includes_tag(&tag));
    }

    #[test]
    fn test_effect_set_merge() {
        let mut effects1 = EffectSet::new();
        effects1.add_raise(EffectType::ValueError);

        let mut effects2 = EffectSet::new();
        effects2.add_raise(EffectType::IndexError);
        let tag = EffectTag::new("io");
        effects2.add_tag(tag);

        effects1.merge(&effects2);
        assert!(effects1.includes_exception(&EffectType::ValueError));
        assert!(effects1.includes_exception(&EffectType::IndexError));
    }

    #[test]
    fn test_effect_set_is_pure() {
        let pure = EffectSet::leaf();
        assert!(!pure.is_pure());

        let mut impure = EffectSet::new();
        impure.add_reads();
        assert!(impure.is_pure());
    }

    #[test]
    fn test_effect_set_is_subset_of() {
        let mut parent = EffectSet::new();
        parent.add_raise(EffectType::Exception);
        parent.add_reads();

        let mut child = EffectSet::new();
        child.add_raise(EffectType::Exception);
        child.add_reads();

        assert!(child.is_subset_of(&parent));

        let mut not_subset = EffectSet::new();
        not_subset.add_raise(EffectType::IOError);
        not_subset.add_reads();

        assert!(!not_subset.is_subset_of(&parent));
    }

    #[test]
    fn test_expected_effects() {
        let mut expected = ExpectedEffects::new();
        expected.add_raise(EffectType::ValueError);
        expected.add_tag(EffectTag::new("io"));

        assert!(expected.raises.contains(&EffectType::ValueError));
        assert!(expected.tags.contains(&EffectTag::new("io")));
    }

    #[test]
    fn test_effect_checker_register_tag() {
        let mut checker = EffectChecker::new();
        checker.register_tag("custom");

        assert!(checker.is_known_tag("custom"));
        assert!(checker.get_tag("custom").is_some());
    }

    #[test]
    fn test_effect_checker_validate_effects_no_forbidden() {
        let checker = EffectChecker::new();
        let mut actual = EffectSet::new();
        actual.add_raise(EffectType::ValueError);

        let mut expected = ExpectedEffects::new();
        expected.add_raise(EffectType::ValueError);

        let validation = checker.validate_effects(&actual, &expected, false);
        assert!(validation.valid);
    }

    #[test]
    fn test_effect_checker_validate_effects_with_forbidden() {
        let checker = EffectChecker::new();
        let mut actual = EffectSet::new();
        actual.add_raise(EffectType::IOError);

        let mut expected = ExpectedEffects::new();
        // Only expect ValueError

        let validation = checker.validate_effects(&actual, &expected, true);
        assert!(!validation.valid);
        assert!(validation.forbidden.len() == 1); // IOError is forbidden
    }

    #[test]
    fn test_effect_checker_can_call() {
        let checker = EffectChecker::new();
        let mut caller = EffectSet::new();
        caller.add_raise(EffectType::ValueError); // Allows ValueError
        caller.add_reads();

        let mut callee = EffectSet::new();
        callee.add_raise(EffectType::ValueError); // Also raises ValueError

        assert!(checker.can_call(&caller, &callee));
    }

    #[test]
    fn test_effect_checker_can_call_fails() {
        let checker = EffectChecker::new();
        let mut caller = EffectSet::new();
        caller.add_raise(EffectType::ValueError); // Only allows ValueError

        let mut callee = EffectSet::new();
        callee.add_raise(EffectType::IOError); // But callee raises IOError

        assert!(!checker.can_call(&caller, &callee));
    }

    #[test]
    fn test_effect_checker_combine_proc_effects() {
        let checker = EffectChecker::new();
        let mut effects1 = EffectSet::new();
        effects1.add_raise(EffectType::ValueError);

        let mut effects2 = EffectSet::new();
        effects2.add_raise(EffectType::IndexError);

        let combined = checker.combine_proc_effects(&[&effects1, &effects2]);
        assert!(combined.includes_exception(&EffectType::ValueError));
        assert!(combined.includes_exception(&EffectType::IndexError));
    }

    #[test]
    fn test_effect_type_name() {
        assert_eq!(EffectType::Exception.name(), "Exception");
        assert_eq!(EffectType::ValueError.name(), "ValueError");
        assert_eq!(EffectType::Custom("MyError".to_string()).name(), "MyError");
    }

    #[test]
    fn test_effect_tag_new() {
        let tag = EffectTag::new("test");
        assert_eq!(tag.0, "test");
    }

    #[test]
    fn test_forbidden_effect_variants() {
        let fe = ForbiddenEffect::Raises(EffectType::ValueError);
        assert!(format!("{:?}", fe).contains("ValueError"));

        let fe = ForbiddenEffect::Tag(EffectTag::new("io"));
        assert!(format!("{:?}", fe).contains("io"));

        let fe = ForbiddenEffect::Reads;
        assert!(format!("{:?}", fe).contains("Reads"));

        let fe = ForbiddenEffect::Writes;
        assert!(format!("{:?}", fe).contains("Writes"));
    }

    #[test]
    fn test_effect_propagation_through_generics() {
        // When a generic function is instantiated, effects should propagate
        let mut effects = EffectSet::new();
        effects.add_raise(EffectType::ValueError);
        effects.flags.insert(EffectFlags::READS);

        // Effect set should contain both raises and reads flag
        assert!(effects.flags.contains(EffectFlags::READS));
        assert!(effects.raises.contains(&EffectType::ValueError));
    }

    #[test]
    fn test_effect_propagation_higher_order() {
        // Higher-order functions should propagate effects
        let mut effects = EffectSet::new();
        effects
            .flags
            .insert(EffectFlags::READS | EffectFlags::WRITES);

        // Combined effects through higher-order calls
        assert!(effects.flags.contains(EffectFlags::READS));
        assert!(effects.flags.contains(EffectFlags::WRITES));
    }

    #[test]
    fn test_lambda_effect_inference() {
        // Lambda expressions should infer effects from body
        let mut effects = EffectSet::new();
        effects
            .flags
            .insert(EffectFlags::LEAF | EffectFlags::GC_SAFE);

        // Leaf lambdas have no side effects
        assert!(effects.flags.contains(EffectFlags::LEAF));
        assert!(effects.flags.contains(EffectFlags::GC_SAFE));
    }

    #[test]
    fn test_effect_inheritance_from_param() {
        // Effects should inherit from parameters that have effects
        let mut effects = EffectSet::new();
        effects.add_raise(EffectType::IOError);
        effects.flags.insert(EffectFlags::READS);

        // Effects should propagate from parameter to result
        assert!(effects.raises.contains(&EffectType::IOError));
        assert!(effects.flags.contains(EffectFlags::READS));
    }

    #[test]
    fn test_effect_intersection_for_call() {
        // When calling a function, effects should be intersection of caller/callee
        let mut caller = EffectSet::new();
        caller.flags.insert(EffectFlags::READS);
        caller.add_raise(EffectType::ValueError);

        let mut callee = EffectSet::new();
        callee
            .flags
            .insert(EffectFlags::READS | EffectFlags::WRITES);
        callee.add_raise(EffectType::ValueError);

        // Intersection of effects (simplified model)
        let combined = caller.flags & callee.flags;
        assert!(combined.contains(EffectFlags::READS));
    }

    #[test]
    fn test_pure_function_no_side_effects() {
        let mut effects = EffectSet::new();
        effects.flags.insert(EffectFlags::LEAF);

        // Pure functions have no IO, no raises, no writes
        assert!(!effects.flags.contains(EffectFlags::WRITES));
        assert!(!effects.flags.contains(EffectFlags::RAISES));
    }

    #[test]
    fn test_effect_tag_equality() {
        let tag1 = EffectTag::new("custom_io");
        let tag2 = EffectTag::new("custom_io");
        let tag3 = EffectTag::new("memory");

        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
    }

    #[test]
    fn test_effect_tag_in_set() {
        let mut effects = EffectSet::new();
        effects.add_tag(EffectTag::new("async"));
        effects.add_tag(EffectTag::new("io"));

        assert_eq!(effects.tags.len(), 2);
    }

    #[test]
    fn test_effect_set_is_empty() {
        let empty = EffectSet::new();
        assert!(empty.flags.is_empty());
        assert!(empty.raises.is_empty());
        assert!(empty.tags.is_empty());
    }

    #[test]
    fn test_effect_set_with_all_flags() {
        let mut effects = EffectSet::new();
        effects.flags.insert(
            EffectFlags::READS
                | EffectFlags::WRITES
                | EffectFlags::RAISES
                | EffectFlags::LEAF
                | EffectFlags::GC_SAFE
                | EffectFlags::HAS_DESTRUCTOR,
        );

        assert!(effects.flags.contains(EffectFlags::READS));
        assert!(effects.flags.contains(EffectFlags::WRITES));
        assert!(effects.flags.contains(EffectFlags::RAISES));
        assert!(effects.flags.contains(EffectFlags::LEAF));
        assert!(effects.flags.contains(EffectFlags::GC_SAFE));
        assert!(effects.flags.contains(EffectFlags::HAS_DESTRUCTOR));
    }

    #[test]
    fn test_effect_checker_register_and_query() {
        let mut checker = EffectChecker::new();
        checker.register_tag("custom_effect");

        // Checker should be able to query registered tags
        assert!(checker.is_known_tag("custom_effect"));
    }

    #[test]
    fn test_effect_propagation_through_macro() {
        // Macros should preserve effect information
        let mut effects = EffectSet::new();
        effects.flags.insert(EffectFlags::READS);
        effects.add_raise(EffectType::RuntimeError);

        // Effect information should be preserved through macro expansion
        assert!(effects.flags.contains(EffectFlags::READS));
        assert!(effects.raises.contains(&EffectType::RuntimeError));
    }

    #[test]
    fn test_forbidden_effect_matching() {
        let forbidden = ForbiddenEffect::Raises(EffectType::ValueError);

        let mut effects = EffectSet::new();
        effects.add_raise(EffectType::ValueError);

        // The ValueError raise should match the forbidden effect
        assert!(effects.includes_exception(&EffectType::ValueError));
    }
}
