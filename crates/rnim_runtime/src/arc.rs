//! ARC (Automatic Reference Counting) memory model implementation.
//!
//! This module provides:
//! - Atomic reference counting for thread-safe ARC/ORC
//! - Strong and weak reference support
//! - Cycle detection infrastructure for ORC (Oblivious Reference Counting)
//! - Hook integration for destructor scheduling

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Atomic reference count wrapper
#[derive(Debug)]
pub struct AtomicRefcount {
    count: AtomicUsize,
}

impl AtomicRefcount {
    /// Create a new atomic refcount with initial value
    pub fn new(initial: usize) -> Self {
        AtomicRefcount {
            count: AtomicUsize::new(initial),
        }
    }

    /// Get the current count
    pub fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    /// Increment the reference count
    #[inline]
    pub fn inc(&self) -> usize {
        self.count.fetch_add(1, Ordering::SeqCst)
    }

    /// Decrement the reference count, returning the new value
    #[inline]
    pub fn dec(&self) -> usize {
        self.count.fetch_sub(1, Ordering::SeqCst)
    }

    /// Check if count is zero (dropped)
    pub fn is_solid_dropped(&self) -> bool {
        self.count.load(Ordering::SeqCst) == 0
    }

    /// Try to decrement - returns true if successful (count > 0 after decrement)
    #[inline]
    pub fn try_dec(&self) -> bool {
        let current = self.count.load(Ordering::SeqCst);
        if current == 0 {
            return false;
        }
        self.count
            .compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
}

impl Default for AtomicRefcount {
    fn default() -> Self {
        Self::new(1)
    }
}

/// Reference count state combining strong and weak counts
#[derive(Debug)]
pub struct RefcountState {
    pub strong: AtomicRefcount,
    pub weak: AtomicRefcount,
}

impl RefcountState {
    /// Create a new refcount state
    pub fn new() -> Self {
        RefcountState {
            strong: AtomicRefcount::new(1),
            weak: AtomicRefcount::new(0),
        }
    }

    /// Create with specific initial strong count
    pub fn with_strong(initial: usize) -> Self {
        RefcountState {
            strong: AtomicRefcount::new(initial),
            weak: AtomicRefcount::new(0),
        }
    }

    /// Check if strong count is zero (object can be destroyed)
    pub fn is_solid_dropped(&self) -> bool {
        self.strong.is_solid_dropped()
    }

    /// Check if weak count is zero (no more weak references)
    pub fn is_weak_dropped(&self) -> bool {
        self.weak.is_solid_dropped()
    }

    /// Check if this is the last strong reference
    pub fn is_last_strong(&self) -> bool {
        self.strong.get() == 1
    }
}

impl Default for RefcountState {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook kinds for ARC/ORC lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArcHookKind {
    /// Called when strong count reaches zero (destroy)
    Destroy,
    /// Called when weak count reaches zero
    DropWeak,
    /// Called to trace references for cycle detection
    Trace,
}

impl ArcHookKind {
    pub fn name(&self) -> &'static str {
        match self {
            ArcHookKind::Destroy => "=destroy",
            ArcHookKind::DropWeak => "=dropWeak",
            ArcHookKind::Trace => "=trace",
        }
    }
}

/// A trait for types that support ARC operations
pub trait ArcSafe: 'static + Send + Sync {
    fn get_type_name() -> &'static str;

    fn destroy_hook() -> Option<fn(*mut std::ffi::c_void)> {
        None
    }

    fn trace_hook() -> Option<fn(*mut std::ffi::c_void, fn(*mut std::ffi::c_void))> {
        None
    }
}

/// Tracer callback for cycle detection
pub type TracerCallback = fn(*mut std::ffi::c_void);

/// Tracer for visiting references
pub trait Tracer {
    fn visit(&mut self, ptr: *mut std::ffi::c_void);
}

/// No-op tracer for counting references
#[derive(Default)]
pub struct CountTracer {
    pub count: usize,
}

impl CountTracer {
    pub fn new() -> Self {
        CountTracer { count: 0 }
    }
}

impl Tracer for CountTracer {
    fn visit(&mut self, _ptr: *mut std::ffi::c_void) {
        self.count += 1;
    }
}

/// Trait for types that can be traced by the GC
pub trait Traceable {
    fn trace(&self, tracer: &mut dyn Tracer);
}

/// Cycle detection state
#[derive(Debug, Default)]
pub enum CycleStatus {
    /// No cycle detected
    #[default]
    None,
    /// Potential cycle detected, needs further investigation
    Potential,
    /// Confirmed cycle, needs collection
    Confirmed,
    /// Cycle is being processed
    Collecting,
}

/// ARC state for tracking objects
#[derive(Debug)]
pub struct ArcState {
    pub type_name: String,
    pub refcount: RefcountState,
    pub cycle_status: CycleStatus,
    pub is_weak: bool,
}

impl ArcState {
    pub fn new(type_name: &str) -> Self {
        ArcState {
            type_name: type_name.to_string(),
            refcount: RefcountState::new(),
            cycle_status: CycleStatus::None,
            is_weak: false,
        }
    }

    pub fn with_weak(type_name: &str) -> Self {
        ArcState {
            type_name: type_name.to_string(),
            refcount: RefcountState::new(),
            cycle_status: CycleStatus::None,
            is_weak: true,
        }
    }

    pub fn inc_strong(&mut self) {
        self.refcount.strong.inc();
    }

    pub fn dec_strong(&mut self) -> usize {
        self.refcount.strong.dec()
    }

    pub fn inc_weak(&mut self) {
        self.refcount.weak.inc();
    }

    pub fn dec_weak(&mut self) -> usize {
        self.refcount.weak.dec()
    }

    pub fn is_dropped(&self) -> bool {
        self.refcount.is_solid_dropped()
    }

    pub fn strong_count(&self) -> usize {
        self.refcount.strong.get()
    }

    pub fn weak_count(&self) -> usize {
        self.refcount.weak.get()
    }
}

/// Hook registration for ARC lifecycle
#[derive(Debug, Clone)]
pub struct ArcHookDef {
    pub kind: ArcHookKind,
    pub type_name: String,
    pub hook_fn: Option<fn(*mut std::ffi::c_void)>,
}

impl ArcHookDef {
    pub fn new(kind: ArcHookKind, type_name: &str) -> Self {
        ArcHookDef {
            kind,
            type_name: type_name.to_string(),
            hook_fn: None,
        }
    }

    pub fn with_hook(kind: ArcHookKind, type_name: &str, hook: fn(*mut std::ffi::c_void)) -> Self {
        ArcHookDef {
            kind,
            type_name: type_name.to_string(),
            hook_fn: Some(hook),
        }
    }
}

/// ARC hook registry
#[derive(Debug, Clone, Default)]
pub struct ArcHookRegistry {
    hooks: HashMap<String, Vec<ArcHookDef>>,
}

impl ArcHookRegistry {
    pub fn new() -> Self {
        ArcHookRegistry {
            hooks: HashMap::new(),
        }
    }

    pub fn register(&mut self, type_name: &str, hook: ArcHookDef) {
        self.hooks
            .entry(type_name.to_string())
            .or_default()
            .push(hook);
    }

    pub fn get_hooks(&self, type_name: &str) -> Option<&Vec<ArcHookDef>> {
        self.hooks.get(type_name)
    }

    pub fn has_destroy_hook(&self, type_name: &str) -> bool {
        self.hooks
            .get(type_name)
            .map(|hooks| hooks.iter().any(|h| h.kind == ArcHookKind::Destroy))
            .unwrap_or(false)
    }

    pub fn has_trace_hook(&self, type_name: &str) -> bool {
        self.hooks
            .get(type_name)
            .map(|hooks| hooks.iter().any(|h| h.kind == ArcHookKind::Trace))
            .unwrap_or(false)
    }
}

/// Schedule a destructor for later execution
#[derive(Debug, Clone)]
pub struct ScheduledDestructor {
    pub type_name: String,
    pub hook_fn: Option<fn(*mut std::ffi::c_void)>,
    pub object_ptr: *mut std::ffi::c_void,
}

impl ScheduledDestructor {
    pub fn new(type_name: &str, ptr: *mut std::ffi::c_void) -> Self {
        ScheduledDestructor {
            type_name: type_name.to_string(),
            hook_fn: None,
            object_ptr: ptr,
        }
    }

    pub fn with_hook(
        type_name: &str,
        ptr: *mut std::ffi::c_void,
        hook: fn(*mut std::ffi::c_void),
    ) -> Self {
        ScheduledDestructor {
            type_name: type_name.to_string(),
            hook_fn: Some(hook),
            object_ptr: ptr,
        }
    }

    /// Execute the destructor
    pub fn execute(&self) {
        if let Some(hook) = self.hook_fn {
            hook(self.object_ptr);
        }
    }
}

/// Planned destroy for scope exit
#[derive(Debug, Clone)]
pub struct PlannedDestroy {
    pub type_name: String,
    pub hook_name: String,
}

/// ARC lifecycle manager
#[derive(Debug, Default)]
pub struct ArcLifecycle {
    registry: ArcHookRegistry,
    pending_destructors: VecDeque<ScheduledDestructor>,
    cycle_candidates: HashSet<String>,
}

impl ArcLifecycle {
    pub fn new() -> Self {
        ArcLifecycle {
            registry: ArcHookRegistry::new(),
            pending_destructors: VecDeque::new(),
            cycle_candidates: HashSet::new(),
        }
    }

    pub fn with_registry(registry: ArcHookRegistry) -> Self {
        ArcLifecycle {
            registry,
            pending_destructors: VecDeque::new(),
            cycle_candidates: HashSet::new(),
        }
    }

    /// Schedule a destructor for execution
    pub fn schedule_destructor(&mut self, dtor: ScheduledDestructor) {
        self.pending_destructors.push_back(dtor);
    }

    /// Execute pending destructors
    pub fn flush_destructors(&mut self) {
        while let Some(dtor) = self.pending_destructors.pop_front() {
            dtor.execute();
        }
    }

    /// Check if there are pending destructors
    pub fn has_pending(&self) -> bool {
        !self.pending_destructors.is_empty()
    }

    /// Register a cycle candidate type
    pub fn register_cycle_candidate(&mut self, type_name: &str) {
        self.cycle_candidates.insert(type_name.to_string());
    }

    /// Check if a type might be involved in cycles
    pub fn is_cycle_candidate(&self, type_name: &str) -> bool {
        self.cycle_candidates.contains(type_name)
    }

    /// Register a destroy hook
    pub fn register_destroy_hook(&mut self, type_name: &str, hook: fn(*mut std::ffi::c_void)) {
        self.registry.register(
            type_name,
            ArcHookDef::with_hook(ArcHookKind::Destroy, type_name, hook),
        );
    }

    /// Register a trace hook
    pub fn register_trace_hook(&mut self, type_name: &str, hook: fn(*mut std::ffi::c_void)) {
        self.registry.register(
            type_name,
            ArcHookDef::with_hook(ArcHookKind::Trace, type_name, hook),
        );
    }

    /// Get the hook registry
    pub fn registry(&self) -> &ArcHookRegistry {
        &self.registry
    }
}

/// ORC (Oblivious Reference Counting) support
pub mod orc {
    use super::*;
    use std::collections::HashSet;

    /// Cycle detection thresholds
    pub const CYCLE_CHECK_THRESHOLD: usize = 1024;
    pub const MAX_REFCOUNT: usize = usize::MAX / 2;

    /// Check if refcount is above threshold for cycle checking
    pub fn needs_cycle_check(count: usize) -> bool {
        count > CYCLE_CHECK_THRESHOLD
    }

    /// Check if refcount indicates potential cycle
    pub fn is_potential_cycle(count: usize) -> bool {
        count > MAX_REFCOUNT / 4
    }

    /// Mark a reference as possibly cyclic
    #[derive(Debug, Default)]
    pub struct CycleMark {
        pub marked: bool,
        pub on_stack: bool,
    }

    impl CycleMark {
        pub fn new() -> Self {
            CycleMark {
                marked: false,
                on_stack: false,
            }
        }

        pub fn mark(&mut self) {
            self.marked = true;
        }

        pub fn unmark(&mut self) {
            self.marked = false;
        }

        pub fn push_stack(&mut self) {
            self.on_stack = true;
        }

        pub fn pop_stack(&mut self) {
            self.on_stack = false;
        }
    }

    /// Trait for objects that can be traced by the GC
    pub trait GcTrace {
        fn trace(&self, tracer: &mut GcTracer);
    }

    /// Tracer for GC visiting
    #[derive(Default)]
    pub struct GcTracer {
        pub visited: HashSet<usize>,
        pub references: VecDeque<usize>,
        pub callback: Option<fn(*mut std::ffi::c_void)>,
    }

    impl GcTracer {
        pub fn new() -> Self {
            GcTracer {
                visited: HashSet::new(),
                references: VecDeque::new(),
                callback: None,
            }
        }

        pub fn with_callback(cb: fn(*mut std::ffi::c_void)) -> Self {
            GcTracer {
                visited: HashSet::new(),
                references: VecDeque::new(),
                callback: Some(cb),
            }
        }

        pub fn visit(&mut self, ptr: *mut std::ffi::c_void) {
            let addr = ptr as usize;
            if !self.visited.contains(&addr) {
                self.visited.insert(addr);
                self.references.push_back(addr);
                if let Some(cb) = self.callback {
                    cb(ptr);
                }
            }
        }

        pub fn is_visited(&self, ptr: *mut std::ffi::c_void) -> bool {
            self.visited.contains(&(ptr as usize))
        }
    }

    /// Object header for GC tracking
    #[derive(Debug)]
    pub struct ObjectHeader {
        pub refcount: AtomicRefcount,
        pub cycle_mark: CycleMark,
        pub type_name: String,
        pub is_root: bool,
    }

    impl ObjectHeader {
        pub fn new(type_name: &str) -> Self {
            ObjectHeader {
                refcount: AtomicRefcount::new(1),
                cycle_mark: CycleMark::new(),
                type_name: type_name.to_string(),
                is_root: false,
            }
        }

        pub fn with_refcount(type_name: &str, initial: usize) -> Self {
            ObjectHeader {
                refcount: AtomicRefcount::new(initial),
                cycle_mark: CycleMark::new(),
                type_name: type_name.to_string(),
                is_root: false,
            }
        }

        pub fn inc(&self) -> usize {
            self.refcount.inc()
        }

        pub fn dec(&self) -> usize {
            self.refcount.dec()
        }

        pub fn is_dropped(&self) -> bool {
            self.refcount.is_solid_dropped()
        }
    }

    /// Root set for GC
    #[derive(Debug, Default)]
    pub struct RootSet {
        roots: HashSet<usize>,
    }

    impl RootSet {
        pub fn new() -> Self {
            RootSet {
                roots: HashSet::new(),
            }
        }

        pub fn add(&mut self, ptr: *mut std::ffi::c_void) {
            self.roots.insert(ptr as usize);
        }

        pub fn remove(&mut self, ptr: *mut std::ffi::c_void) {
            self.roots.remove(&(ptr as usize));
        }

        pub fn contains(&self, ptr: *mut std::ffi::c_void) -> bool {
            self.roots.contains(&(ptr as usize))
        }

        pub fn len(&self) -> usize {
            self.roots.len()
        }

        pub fn is_empty(&self) -> bool {
            self.roots.is_empty()
        }
    }

    /// ORC Cycle Collector
    #[derive(Debug, Default)]
    pub struct CycleCollector {
        root_set: RootSet,
        candidate_pool: HashSet<usize>,
        triColor_pending: VecDeque<usize>,
        cycle_threshold: usize,
    }

    impl CycleCollector {
        pub fn new() -> Self {
            CycleCollector {
                root_set: RootSet::new(),
                candidate_pool: HashSet::new(),
                triColor_pending: VecDeque::new(),
                cycle_threshold: CYCLE_CHECK_THRESHOLD,
            }
        }

        /// Register a root reference
        pub fn register_root(&mut self, ptr: *mut std::ffi::c_void) {
            self.root_set.add(ptr);
        }

        /// Unregister a root reference
        pub fn unregister_root(&mut self, ptr: *mut std::ffi::c_void) {
            self.root_set.remove(ptr);
        }

        /// Check if a pointer is a root
        pub fn is_root(&self, ptr: *mut std::ffi::c_void) -> bool {
            self.root_set.contains(ptr)
        }

        /// Register a cycle candidate
        pub fn register_candidate(&mut self, ptr: *mut std::ffi::c_void) {
            self.candidate_pool.insert(ptr as usize);
        }

        /// Check if a pointer is a candidate
        pub fn is_candidate(&self, ptr: *mut std::ffi::c_void) -> bool {
            self.candidate_pool.contains(&(ptr as usize))
        }

        /// Add to tri-color pending set
        pub fn mark_pending(&mut self, ptr: *mut std::ffi::c_void) {
            self.triColor_pending.push_back(ptr as usize);
        }

        /// Get next pending pointer
        pub fn get_pending(&mut self) -> Option<usize> {
            self.triColor_pending.pop_front()
        }

        /// Check if pending set is empty
        pub fn has_pending(&self) -> bool {
            !self.triColor_pending.is_empty()
        }

        /// Set cycle detection threshold
        pub fn set_threshold(&mut self, threshold: usize) {
            self.cycle_threshold = threshold;
        }

        /// Check if an object needs cycle checking
        pub fn needs_check(&self, refcount: usize) -> bool {
            refcount > self.cycle_threshold
        }

        /// Clear all collected state
        pub fn reset(&mut self) {
            self.candidate_pool.clear();
            self.triColor_pending.clear();
        }

        /// Get root set
        pub fn roots(&self) -> &RootSet {
            &self.root_set
        }
    }
}

#[cfg(test)]
mod tests {
    use super::orc::{
        is_potential_cycle, needs_cycle_check, CycleCollector, CycleMark, GcTracer, ObjectHeader,
        RootSet,
    };
    use super::*;

    #[test]
    fn test_atomic_refcount_new() {
        let rc = AtomicRefcount::new(1);
        assert_eq!(rc.get(), 1);
    }

    #[test]
    fn test_atomic_refcount_inc() {
        let rc = AtomicRefcount::new(1);
        rc.inc();
        assert_eq!(rc.get(), 2);
    }

    #[test]
    fn test_atomic_refcount_dec() {
        let rc = AtomicRefcount::new(2);
        let old_val = rc.dec();
        assert_eq!(old_val, 2);
        assert_eq!(rc.get(), 1);
    }

    #[test]
    fn test_atomic_refcount_is_solid_dropped() {
        let rc = AtomicRefcount::new(1);
        assert!(!rc.is_solid_dropped());
        rc.dec();
        assert!(rc.is_solid_dropped());
    }

    #[test]
    fn test_atomic_refcount_try_dec() {
        let rc = AtomicRefcount::new(0);
        assert!(!rc.try_dec());

        let rc = AtomicRefcount::new(1);
        assert!(rc.try_dec());
        assert_eq!(rc.get(), 0);
        assert!(!rc.try_dec());
    }

    #[test]
    fn test_refcount_state_new() {
        let state = RefcountState::new();
        assert_eq!(state.strong.get(), 1);
        assert_eq!(state.weak.get(), 0);
    }

    #[test]
    fn test_refcount_state_with_strong() {
        let state = RefcountState::with_strong(5);
        assert_eq!(state.strong.get(), 5);
        assert_eq!(state.weak.get(), 0);
    }

    #[test]
    fn test_refcount_state_is_solid_dropped() {
        let state = RefcountState::new();
        assert!(!state.is_solid_dropped());
        state.strong.dec();
        assert!(state.is_solid_dropped());
    }

    #[test]
    fn test_refcount_state_is_last_strong() {
        let state = RefcountState::with_strong(1);
        assert!(state.is_last_strong());

        state.strong.inc();
        assert!(!state.is_last_strong());
    }

    #[test]
    fn test_arc_state_new() {
        let state = ArcState::new("MyType");
        assert_eq!(state.type_name, "MyType");
        assert!(!state.is_weak);
        assert_eq!(state.strong_count(), 1);
        assert_eq!(state.weak_count(), 0);
    }

    #[test]
    fn test_arc_state_with_weak() {
        let state = ArcState::with_weak("MyType");
        assert!(state.is_weak);
    }

    #[test]
    fn test_arc_state_inc_dec_strong() {
        let mut state = ArcState::new("MyType");
        state.inc_strong();
        assert_eq!(state.strong_count(), 2);

        let old_val = state.dec_strong();
        assert_eq!(old_val, 2);
        assert_eq!(state.strong_count(), 1);
    }

    #[test]
    fn test_arc_state_inc_dec_weak() {
        let mut state = ArcState::new("MyType");
        state.inc_weak();
        assert_eq!(state.weak_count(), 1);

        let old_val = state.dec_weak();
        assert_eq!(old_val, 1);
        assert_eq!(state.weak_count(), 0);
    }

    #[test]
    fn test_arc_state_is_dropped() {
        let state = ArcState::new("MyType");
        assert!(!state.is_dropped());

        let mut state = ArcState::new("MyType");
        state.dec_strong();
        assert!(state.is_dropped());
    }

    #[test]
    fn test_arc_hook_kind_name() {
        assert_eq!(ArcHookKind::Destroy.name(), "=destroy");
        assert_eq!(ArcHookKind::DropWeak.name(), "=dropWeak");
        assert_eq!(ArcHookKind::Trace.name(), "=trace");
    }

    #[test]
    fn test_arc_hook_def_new() {
        let hook = ArcHookDef::new(ArcHookKind::Destroy, "MyType");
        assert_eq!(hook.type_name, "MyType");
        assert!(hook.hook_fn.is_none());
    }

    #[test]
    fn test_arc_hook_registry_register() {
        let mut registry = ArcHookRegistry::new();
        let hook = ArcHookDef::new(ArcHookKind::Destroy, "MyType");
        registry.register("MyType", hook);

        assert!(registry.has_destroy_hook("MyType"));
        assert!(!registry.has_trace_hook("MyType"));
    }

    #[test]
    fn test_arc_hook_registry_get_hooks() {
        let mut registry = ArcHookRegistry::new();
        registry.register("MyType", ArcHookDef::new(ArcHookKind::Destroy, "MyType"));
        registry.register("MyType", ArcHookDef::new(ArcHookKind::Trace, "MyType"));

        let hooks = registry.get_hooks("MyType");
        assert_eq!(hooks.unwrap().len(), 2);
    }

    #[test]
    fn test_scheduled_destructor_new() {
        let dtor = ScheduledDestructor::new("MyType", std::ptr::null_mut());
        assert_eq!(dtor.type_name, "MyType");
        assert!(dtor.hook_fn.is_none());
    }

    #[test]
    fn test_arc_lifecycle_new() {
        let lifecycle = ArcLifecycle::new();
        assert!(!lifecycle.has_pending());
    }

    #[test]
    fn test_arc_lifecycle_schedule_destructor() {
        let mut lifecycle = ArcLifecycle::new();
        let dtor = ScheduledDestructor::new("MyType", std::ptr::null_mut());
        lifecycle.schedule_destructor(dtor);
        assert!(lifecycle.has_pending());
    }

    #[test]
    fn test_arc_lifecycle_register_cycle_candidate() {
        let mut lifecycle = ArcLifecycle::new();
        lifecycle.register_cycle_candidate("MyType");
        assert!(lifecycle.is_cycle_candidate("MyType"));
        assert!(!lifecycle.is_cycle_candidate("OtherType"));
    }

    #[test]
    fn test_orc_cycle_check_threshold() {
        assert!(!needs_cycle_check(100));
        assert!(needs_cycle_check(2000));
    }

    #[test]
    fn test_orc_is_potential_cycle() {
        assert!(!is_potential_cycle(100));
        assert!(is_potential_cycle(usize::MAX / 3));
    }

    #[test]
    fn test_orc_cycle_mark() {
        let mut mark = CycleMark::new();
        assert!(!mark.marked);
        assert!(!mark.on_stack);

        mark.mark();
        assert!(mark.marked);

        mark.push_stack();
        assert!(mark.on_stack);

        mark.pop_stack();
        assert!(!mark.on_stack);

        mark.unmark();
        assert!(!mark.marked);
    }

    #[test]
    fn test_count_tracer() {
        let mut tracer = CountTracer::new();
        tracer.visit(std::ptr::null_mut());
        tracer.visit(std::ptr::null_mut());
        assert_eq!(tracer.count, 2);
    }

    #[test]
    fn test_gc_tracer_new() {
        let tracer = GcTracer::new();
        assert!(tracer.visited.is_empty());
        assert!(tracer.references.is_empty());
    }

    #[test]
    fn test_gc_tracer_visit() {
        let mut tracer = GcTracer::new();
        let ptr1 = 100_usize as *mut std::ffi::c_void;
        let ptr2 = 200_usize as *mut std::ffi::c_void;

        tracer.visit(ptr1);
        tracer.visit(ptr1);
        tracer.visit(ptr2);

        assert_eq!(tracer.visited.len(), 2);
        assert_eq!(tracer.references.len(), 2);
    }

    #[test]
    fn test_gc_tracer_is_visited() {
        let mut tracer = GcTracer::new();
        let ptr = 100_usize as *mut std::ffi::c_void;

        assert!(!tracer.is_visited(ptr));
        tracer.visit(ptr);
        assert!(tracer.is_visited(ptr));
    }

    #[test]
    fn test_root_set_new() {
        let roots = RootSet::new();
        assert!(roots.is_empty());
        assert_eq!(roots.len(), 0);
    }

    #[test]
    fn test_root_set_add_remove() {
        let mut roots = RootSet::new();
        let ptr = 100_usize as *mut std::ffi::c_void;

        roots.add(ptr);
        assert!(roots.contains(ptr));
        assert_eq!(roots.len(), 1);

        roots.remove(ptr);
        assert!(!roots.contains(ptr));
        assert_eq!(roots.len(), 0);
    }

    #[test]
    fn test_object_header_new() {
        let header = ObjectHeader::new("MyType");
        assert_eq!(header.type_name, "MyType");
        assert!(!header.is_root);
        assert_eq!(header.refcount.get(), 1);
    }

    #[test]
    fn test_object_header_inc_dec() {
        let header = ObjectHeader::new("MyType");
        header.inc();
        assert_eq!(header.refcount.get(), 2);

        header.dec();
        assert_eq!(header.refcount.get(), 1);
    }

    #[test]
    fn test_object_header_is_dropped() {
        let header = ObjectHeader::new("MyType");
        assert!(!header.is_dropped());
        header.dec();
        assert!(header.is_dropped());
    }

    #[test]
    fn test_cycle_collector_new() {
        let collector = CycleCollector::new();
        // Use public API to verify initial state
        assert!(collector.roots().is_empty());
    }

    #[test]
    fn test_cycle_collector_register_root() {
        let mut collector = CycleCollector::new();
        let ptr = 100_usize as *mut std::ffi::c_void;

        collector.register_root(ptr);
        assert!(collector.is_root(ptr));
    }

    #[test]
    fn test_cycle_collector_register_candidate() {
        let mut collector = CycleCollector::new();
        let ptr = 100_usize as *mut std::ffi::c_void;

        collector.register_candidate(ptr);
        assert!(collector.is_candidate(ptr));
    }

    #[test]
    fn test_cycle_collector_mark_pending() {
        let mut collector = CycleCollector::new();
        let ptr = 100_usize as *mut std::ffi::c_void;

        collector.mark_pending(ptr);
        assert!(collector.has_pending());
        assert_eq!(collector.get_pending(), Some(100));
        assert!(!collector.has_pending());
    }

    #[test]
    fn test_cycle_collector_threshold() {
        let mut collector = CycleCollector::new();
        assert!(collector.needs_check(2000));
        collector.set_threshold(5000);
        assert!(!collector.needs_check(2000));
        assert!(collector.needs_check(6000));
    }

    #[test]
    fn test_cycle_collector_reset() {
        let mut collector = CycleCollector::new();
        collector.register_candidate(100_usize as *mut std::ffi::c_void);
        collector.mark_pending(200_usize as *mut std::ffi::c_void);

        collector.reset();
        // After reset, should have no pending and is_candidate returns false
        assert!(!collector.has_pending());
        assert!(!collector.is_candidate(100_usize as *mut std::ffi::c_void));
    }
}
