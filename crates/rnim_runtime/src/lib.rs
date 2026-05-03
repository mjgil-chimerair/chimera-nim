//! Runtime core types and operations.
//!
//! This module implements the runtime support for:
//! - `string`: UTF-8 heap-allocated strings with refcount
//! - `seq[T]`: Dynamic sequences with refcount
//! - `ref T`: Heap-allocated references with refcount
//! - `set`: Bit sets for small cardinalities
//! - Exception representation
//! - Stack trace capture

#[cfg(test)]
use rnim_allocator as _;
pub mod arc;
pub mod async_rt;
pub mod concurrency;

pub use arc::orc::{
    is_potential_cycle, needs_cycle_check, CycleCollector, CycleMark, GcTrace, GcTracer,
    ObjectHeader, RootSet,
};
pub use arc::{
    ArcHookDef, ArcHookKind, ArcHookRegistry, ArcLifecycle, ArcSafe, ArcState, AtomicRefcount,
    CountTracer, CycleStatus, PlannedDestroy, RefcountState, ScheduledDestructor, Traceable,
    Tracer, TracerCallback,
};

pub use concurrency::{
    channel, AtomicCounter, AtomicFlag, ChannelError, Mutex, Once, Receiver, RwLock, Sender,
    Thread, ThreadConfig, ThreadId, ThreadPriority,
};

pub use async_rt::{
    make_waker, AsyncJoinHandle, AsyncResult, AsyncRuntime, AsyncTask, JoinAll, SelectAll, Sleep,
};

/// Initialize the runtime
pub fn initialize() {
    // Runtime initialization happens implicitly via static constructors
}

/// A Nim string wrapper for reference counting
#[derive(Debug, Clone)]
pub struct NimString {
    data: String,
    refcount: usize,
}

impl NimString {
    /// Create a new empty string
    pub fn empty() -> Self {
        NimString {
            data: String::new(),
            refcount: 1,
        }
    }

    /// Create a new string from a Rust string
    pub fn new(s: &str) -> Self {
        NimString {
            data: s.to_string(),
            refcount: 1,
        }
    }

    /// Get the string data as a Rust string slice
    pub fn as_str(&self) -> &str {
        &self.data
    }

    /// Get the length of the string in bytes
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Get the reference count
    pub fn refcount(&self) -> usize {
        self.refcount
    }

    /// Set the reference count
    pub fn set_refcount(&mut self, rc: usize) {
        self.refcount = rc;
    }
}

impl Default for NimString {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<String> for NimString {
    fn from(s: String) -> Self {
        NimString {
            data: s,
            refcount: 1,
        }
    }
}

impl From<&str> for NimString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl std::ops::Deref for NimString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

/// A Nim sequence type
#[derive(Debug, Clone)]
pub struct NimSeq<T> {
    data: Vec<T>,
    refcount: usize,
}

impl<T> NimSeq<T> {
    /// Create a new empty sequence
    pub fn empty() -> Self {
        NimSeq {
            data: Vec::new(),
            refcount: 1,
        }
    }

    /// Create a new sequence with the given elements
    pub fn new(items: &[T]) -> Self
    where
        T: Clone,
    {
        NimSeq {
            data: items.to_vec(),
            refcount: 1,
        }
    }

    /// Get the length of the sequence
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Get the capacity of the sequence
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Check if the sequence is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the reference count
    pub fn refcount(&self) -> usize {
        self.refcount
    }

    /// Set the reference count
    pub fn set_refcount(&mut self, rc: usize) {
        self.refcount = rc;
    }

    /// Get an element at an index
    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    /// Get a mutable reference to an element at an index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    /// Push an element to the end
    pub fn push(&mut self, elem: T) {
        self.data.push(elem);
    }

    /// Pop an element from the end
    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }
}

impl<T> Default for NimSeq<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> std::ops::Deref for NimSeq<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

/// A Nim reference type
#[derive(Debug)]
pub struct NimRef<T> {
    data: Option<T>,
    refcount: usize,
    is_weak: bool,
}

impl<T> NimRef<T> {
    /// Create a new reference pointing to data
    pub fn new(data: T) -> Self {
        NimRef {
            data: Some(data),
            refcount: 1,
            is_weak: false,
        }
    }

    /// Create a nil reference
    pub fn nil() -> Self {
        NimRef {
            data: None,
            refcount: 0,
            is_weak: false,
        }
    }

    /// Check if the reference is nil
    pub fn is_nil(&self) -> bool {
        self.data.is_none()
    }

    /// Get the reference count
    pub fn refcount(&self) -> usize {
        self.refcount
    }

    /// Set the reference count
    pub fn set_refcount(&mut self, rc: usize) {
        self.refcount = rc;
    }

    /// Get a reference to the data
    pub fn get(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// Get a mutable reference to the data
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }
}

impl<T> Default for NimRef<T> {
    fn default() -> Self {
        Self::nil()
    }
}

/// A Nim set type (bit set for small cardinalities)
#[derive(Debug, Clone, Default)]
pub struct NimSet<T> {
    /// Bit representation of the set
    bits: u64,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Into<u64> + TryFrom<u64> + Copy> NimSet<T> {
    /// Create a new empty set
    pub fn new() -> Self {
        NimSet {
            bits: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Insert an element into the set
    pub fn insert(&mut self, elem: T) {
        let idx: u64 = elem.into();
        self.bits |= 1 << idx;
    }

    /// Remove an element from the set
    pub fn remove(&mut self, elem: T) {
        let idx: u64 = elem.into();
        self.bits &= !(1 << idx);
    }

    /// Check if an element is in the set
    pub fn contains(&self, elem: T) -> bool {
        let idx: u64 = elem.into();
        (self.bits & (1 << idx)) != 0
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }

    /// Get the number of elements in the set
    pub fn len(&self) -> usize {
        self.bits.count_ones() as usize
    }

    /// Union this set with another
    pub fn union(&mut self, other: &NimSet<T>) {
        self.bits |= other.bits;
    }

    /// Intersect this set with another
    pub fn intersect(&mut self, other: &NimSet<T>) {
        self.bits &= other.bits;
    }

    /// Difference this set with another
    pub fn difference(&mut self, other: &NimSet<T>) {
        self.bits &= !other.bits;
    }
}

/// Exception representation in the runtime
#[derive(Debug, Clone)]
pub struct NimException {
    /// Exception type name
    pub type_name: String,
    /// Exception message
    pub message: String,
    /// Source file (if available)
    pub file: Option<String>,
    /// Line number (if available)
    pub line: u32,
}

impl NimException {
    /// Create a new exception
    pub fn new(type_name: &str, message: String) -> Self {
        NimException {
            type_name: type_name.to_string(),
            message,
            file: None,
            line: 0,
        }
    }

    /// Create with source location
    pub fn with_location(type_name: &str, message: String, file: String, line: u32) -> Self {
        NimException {
            type_name: type_name.to_string(),
            message,
            file: Some(file),
            line,
        }
    }
}

/// Stack trace frame
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Procedure name
    pub proc_name: String,
    /// Source file
    pub file: Option<String>,
    /// Line number
    pub line: Option<u32>,
    /// Column number
    pub col: Option<u32>,
}

/// Stack trace capture
#[derive(Debug, Clone, Default)]
pub struct StackTrace {
    pub frames: Vec<StackFrame>,
}

impl StackTrace {
    /// Create a new empty stack trace
    pub fn new() -> Self {
        StackTrace { frames: Vec::new() }
    }

    /// Add a frame to the stack trace
    pub fn push_frame(&mut self, frame: StackFrame) {
        self.frames.push(frame);
    }

    /// Get the number of frames
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Check if the stack trace is empty
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

/// Thread-local storage key
/// Uses thread-local storage with a key index.
/// In production, this would use platform-specific TLS (pthread on Unix, TlsAlloc on Windows).
/// This simplified version uses a static counter for key allocation.
#[derive(Debug)]
pub struct TlsKey<T> {
    key: usize,
    _phantom: std::marker::PhantomData<T>,
}

unsafe impl<T> Send for TlsKey<T> {}
unsafe impl<T> Sync for TlsKey<T> {}

impl<T> TlsKey<T> {
    /// Create a new thread-local storage key
    pub fn new() -> Option<Self> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static KEY_COUNTER: AtomicUsize = AtomicUsize::new(1);
        Some(TlsKey {
            key: KEY_COUNTER.fetch_add(1, Ordering::Relaxed),
            _phantom: std::marker::PhantomData,
        })
    }

    /// Set the value for the current thread
    pub fn set(&self, _value: &T) {
        // Note: Full implementation would store value in thread-local storage
        // keyed by self.key. This requires platform-specific TLS APIs.
    }

    /// Get the value for the current thread
    pub fn get(&self) -> Option<&T> {
        // Note: Full implementation would retrieve from thread-local storage
        None
    }
}

impl<T> Default for TlsKey<T> {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl<T> Drop for TlsKey<T> {
    fn drop(&mut self) {
        // Note: Full implementation would call platform TLS delete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nim_string_new() {
        let s = NimString::new("hello");
        assert_eq!(s.as_str(), "hello");
        assert_eq!(s.len(), 5);
        assert_eq!(s.refcount(), 1);
    }

    #[test]
    fn test_nim_string_from_str() {
        let s: NimString = "test".into();
        assert_eq!(s.as_str(), "test");
    }

    #[test]
    fn test_nim_string_deref() {
        let s = NimString::new("hello");
        assert_eq!(&*s, "hello");
    }

    #[test]
    fn test_nim_seq_new() {
        let seq: NimSeq<i32> = NimSeq::new(&[1, 2, 3]);
        assert_eq!(seq.len(), 3);
        assert_eq!(seq.get(0), Some(&1));
        assert_eq!(seq.get(2), Some(&3));
        assert!(seq.get(3).is_none());
    }

    #[test]
    fn test_nim_seq_empty() {
        let seq: NimSeq<i32> = NimSeq::empty();
        assert!(seq.is_empty());
        assert_eq!(seq.len(), 0);
    }

    #[test]
    fn test_nim_seq_operations() {
        let mut seq: NimSeq<i32> = NimSeq::empty();
        seq.push(1);
        seq.push(2);
        assert_eq!(seq.len(), 2);
        assert_eq!(seq.pop(), Some(2));
        assert_eq!(seq.len(), 1);
    }

    #[test]
    fn test_nim_ref_new() {
        let r = NimRef::new(42);
        assert!(!r.is_nil());
        assert_eq!(r.get(), Some(&42));
    }

    #[test]
    fn test_nim_ref_nil() {
        let r: NimRef<i32> = NimRef::nil();
        assert!(r.is_nil());
        assert_eq!(r.get(), None);
    }

    #[test]
    fn test_nim_ref_get_mut() {
        let mut r = NimRef::new(42);
        if let Some(val) = r.get_mut() {
            *val = 100;
        }
        assert_eq!(r.get(), Some(&100));
    }

    #[test]
    fn test_nim_set_insert_contains() {
        let mut set: NimSet<u8> = NimSet::new();
        assert!(!set.contains(0));
        set.insert(0);
        assert!(set.contains(0));
        assert!(!set.contains(1));
        set.insert(5);
        assert!(set.contains(5));
    }

    #[test]
    fn test_nim_set_operations() {
        let mut set1: NimSet<u8> = NimSet::new();
        set1.insert(0);
        set1.insert(1);
        set1.insert(2);

        let mut set2: NimSet<u8> = NimSet::new();
        set2.insert(1);
        set2.insert(2);
        set2.insert(3);

        // Test union
        let mut union_set = set1.clone();
        union_set.union(&set2);
        assert!(union_set.contains(0));
        assert!(union_set.contains(1));
        assert!(union_set.contains(2));
        assert!(union_set.contains(3));
        assert_eq!(union_set.len(), 4);

        // Test intersect
        let mut intersect_set = set1.clone();
        intersect_set.intersect(&set2);
        assert!(intersect_set.contains(1));
        assert!(intersect_set.contains(2));
        assert!(!intersect_set.contains(0));
        assert_eq!(intersect_set.len(), 2);

        // Test difference
        let mut diff_set = set1.clone();
        diff_set.difference(&set2);
        assert!(diff_set.contains(0));
        assert!(!diff_set.contains(1));
        assert!(!diff_set.contains(2));
        assert_eq!(diff_set.len(), 1);
    }

    #[test]
    fn test_nim_exception() {
        let exc = NimException::new("ValueError", "test error".to_string());
        assert_eq!(exc.type_name, "ValueError");
        assert_eq!(exc.message, "test error");
    }

    #[test]
    fn test_nim_exception_with_location() {
        let exc = NimException::with_location(
            "ValueError",
            "test error".to_string(),
            "main.nim".to_string(),
            10,
        );
        assert_eq!(exc.type_name, "ValueError");
        assert_eq!(exc.message, "test error");
        assert_eq!(exc.file, Some("main.nim".to_string()));
        assert_eq!(exc.line, 10);
    }

    #[test]
    fn test_stack_trace() {
        let mut trace = StackTrace::new();
        assert!(trace.is_empty());

        trace.push_frame(StackFrame {
            proc_name: "main".to_string(),
            file: Some("main.nim".to_string()),
            line: Some(10),
            col: Some(5),
        });

        assert_eq!(trace.len(), 1);
        assert_eq!(trace.frames[0].proc_name, "main");
    }

    #[test]
    fn test_nim_set_clone() {
        let mut set1: NimSet<u8> = NimSet::new();
        set1.insert(0);
        set1.insert(1);

        let set2 = set1.clone();
        assert!(set2.contains(0));
        assert!(set2.contains(1));
    }

    #[test]
    fn test_tls_key() {
        let key: TlsKey<i32> = TlsKey::new().unwrap();
        assert!(key.get().is_none());
    }

    #[test]
    fn test_nim_ref_refcount() {
        let mut r = NimRef::new(42);
        assert_eq!(r.refcount(), 1);
        r.set_refcount(5);
        assert_eq!(r.refcount(), 5);
    }

    #[test]
    fn test_nim_seq_refcount() {
        let mut seq: NimSeq<i32> = NimSeq::new(&[1, 2, 3]);
        assert_eq!(seq.refcount(), 1);
        seq.set_refcount(10);
        assert_eq!(seq.refcount(), 10);
    }

    #[test]
    fn test_nim_string_set_refcount() {
        let mut s = NimString::new("test");
        assert_eq!(s.refcount(), 1);
        s.set_refcount(100);
        assert_eq!(s.refcount(), 100);
    }
}
