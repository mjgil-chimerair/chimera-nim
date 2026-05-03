//! Concurrency primitives for thread-safe operations.
//!
//! This module provides basic concurrency primitives for thread communication
//! and synchronization.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Thread identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadId(u64);

impl ThreadId {
    pub fn new(id: u64) -> Self {
        ThreadId(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for ThreadId {
    fn default() -> Self {
        ThreadId(0)
    }
}

/// Thread priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadPriority {
    Low,
    Normal,
    High,
    Realtime,
}

impl Default for ThreadPriority {
    fn default() -> Self {
        ThreadPriority::Normal
    }
}

/// Thread configuration
#[derive(Debug)]
pub struct ThreadConfig {
    pub name: Option<String>,
    pub priority: ThreadPriority,
    pub stack_size: Option<usize>,
}

impl Default for ThreadConfig {
    fn default() -> Self {
        ThreadConfig {
            name: None,
            priority: ThreadPriority::Normal,
            stack_size: None,
        }
    }
}

impl ThreadConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn priority(mut self, priority: ThreadPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = Some(size);
        self
    }
}

/// Thread handle
#[derive(Debug)]
pub struct Thread {
    id: ThreadId,
    name: Option<String>,
}

/// Get the current thread's ID
pub fn current_thread_id() -> ThreadId {
    ThreadId(1)
}

/// Channel error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelError {
    Closed,
    Empty,
    Full,
}

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelError::Closed => write!(f, "channel closed"),
            ChannelError::Empty => write!(f, "channel empty"),
            ChannelError::Full => write!(f, "channel full"),
        }
    }
}

impl std::error::Error for ChannelError {}

/// Channel inner state
#[derive(Debug)]
struct ChannelInner<T> {
    queue: VecDeque<T>,
    closed: bool,
}

impl<T> ChannelInner<T> {
    fn new() -> Self {
        ChannelInner {
            queue: VecDeque::new(),
            closed: false,
        }
    }
}

/// Sender half of a channel
#[derive(Debug)]
pub struct Sender<T> {
    inner: std::sync::Arc<std::sync::Mutex<ChannelInner<T>>>,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), ChannelError> {
        let mut inner = self.inner.lock().unwrap();
        if inner.closed {
            return Err(ChannelError::Closed);
        }
        inner.queue.push_back(value);
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.inner.lock().unwrap().closed
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender {
            inner: self.inner.clone(),
        }
    }
}

/// Receiver half of a channel
#[derive(Debug)]
pub struct Receiver<T> {
    inner: std::sync::Arc<std::sync::Mutex<ChannelInner<T>>>,
}

impl<T> Receiver<T> {
    pub fn try_recv(&self) -> Result<T, ChannelError> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(value) = inner.queue.pop_front() {
            Ok(value)
        } else if inner.closed {
            Err(ChannelError::Closed)
        } else {
            Err(ChannelError::Empty)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().queue.is_empty()
    }

    pub fn is_closed(&self) -> bool {
        self.inner.lock().unwrap().closed
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().queue.len()
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Receiver {
            inner: self.inner.clone(),
        }
    }
}

/// Create a channel
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = std::sync::Arc::new(std::sync::Mutex::new(ChannelInner::new()));
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver { inner },
    )
}

/// Mutex for mutual exclusion
#[derive(Debug)]
pub struct Mutex<T> {
    inner: std::sync::Mutex<T>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Mutex {
            inner: std::sync::Mutex::new(value),
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, T> {
        self.inner.lock().unwrap()
    }

    pub fn try_lock(&self) -> Option<std::sync::MutexGuard<'_, T>> {
        self.inner.try_lock().ok()
    }
}

impl<T> Default for Mutex<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// Read-write mutex
#[derive(Debug)]
pub struct RwLock<T> {
    inner: std::sync::RwLock<T>,
}

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        RwLock {
            inner: std::sync::RwLock::new(value),
        }
    }

    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, T> {
        self.inner.read().unwrap()
    }

    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, T> {
        self.inner.write().unwrap()
    }

    pub fn try_read(&self) -> Option<std::sync::RwLockReadGuard<'_, T>> {
        self.inner.try_read().ok()
    }

    pub fn try_write(&self) -> Option<std::sync::RwLockWriteGuard<'_, T>> {
        self.inner.try_write().ok()
    }
}

impl<T> Default for RwLock<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// Once initialization
#[derive(Debug)]
pub struct Once<T> {
    inner: std::sync::Once,
    value: std::cell::UnsafeCell<Option<T>>,
}

impl<T> Once<T> {
    pub fn new() -> Self {
        Once {
            inner: std::sync::Once::new(),
            value: std::cell::UnsafeCell::new(None),
        }
    }

    pub fn call_once<F>(&self, f: F)
    where
        F: FnOnce() -> T,
    {
        self.inner.call_once(|| unsafe {
            *self.value.get() = Some(f());
        });
    }

    pub fn get(&self) -> Option<&T> {
        if self.inner.is_completed() {
            unsafe { (*self.value.get()).as_ref() }
        } else {
            None
        }
    }
}

impl<T> Default for Once<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Atomic counter
#[derive(Debug)]
pub struct AtomicCounter {
    count: AtomicUsize,
}

impl AtomicCounter {
    pub fn new(initial: usize) -> Self {
        AtomicCounter {
            count: AtomicUsize::new(initial),
        }
    }

    pub fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    pub fn inc(&self) -> usize {
        self.count.fetch_add(1, Ordering::SeqCst)
    }

    pub fn dec(&self) -> usize {
        self.count.fetch_sub(1, Ordering::SeqCst)
    }

    pub fn fetch_max(&self, val: usize) -> usize {
        let mut current = self.count.load(Ordering::SeqCst);
        loop {
            if current >= val {
                return current;
            }
            match self
                .count
                .compare_exchange(current, val, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return current,
                Err(actual) => current = actual,
            }
        }
    }

    pub fn fetch_min(&self, val: usize) -> usize {
        let mut current = self.count.load(Ordering::SeqCst);
        loop {
            if current <= val {
                return current;
            }
            match self
                .count
                .compare_exchange(current, val, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return current,
                Err(actual) => current = actual,
            }
        }
    }
}

impl Default for AtomicCounter {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Atomic flag
#[derive(Debug)]
pub struct AtomicFlag {
    flag: AtomicBool,
}

impl AtomicFlag {
    pub fn new(initial: bool) -> Self {
        AtomicFlag {
            flag: AtomicBool::new(initial),
        }
    }

    pub fn get(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    pub fn set(&self, value: bool) {
        self.flag.store(value, Ordering::SeqCst)
    }

    pub fn test_and_set(&self) -> bool {
        self.flag.swap(true, Ordering::SeqCst)
    }

    pub fn test_and_clear(&self) -> bool {
        self.flag.swap(false, Ordering::SeqCst)
    }
}

impl Default for AtomicFlag {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_id() {
        let id = ThreadId::new(42);
        assert_eq!(id.as_u64(), 42);
    }

    #[test]
    fn test_thread_config() {
        let config = ThreadConfig::new()
            .name("test")
            .priority(ThreadPriority::High)
            .stack_size(1024 * 1024);
        assert_eq!(config.name, Some("test".to_string()));
        assert_eq!(config.priority, ThreadPriority::High);
        assert_eq!(config.stack_size, Some(1024 * 1024));
    }

    #[test]
    fn test_current_thread_id() {
        let id = current_thread_id();
        assert_eq!(id.as_u64(), 1);
    }

    #[test]
    fn test_channel() {
        let (tx, rx) = channel::<i32>();
        tx.send(42).unwrap();
        assert_eq!(rx.try_recv().unwrap(), 42);
    }

    #[test]
    fn test_channel_empty() {
        let (_, rx) = channel::<i32>();
        assert!(rx.is_empty());
        assert!(matches!(rx.try_recv(), Err(ChannelError::Empty)));
    }

    #[test]
    fn test_channel_closed() {
        let (tx, rx) = channel::<i32>();
        assert!(!rx.is_closed());
    }

    #[test]
    fn test_channel_clone() {
        let (tx1, rx) = channel::<i32>();
        let tx2 = tx1.clone();
        tx1.send(1).unwrap();
        tx2.send(2).unwrap();
        drop(tx1);
        drop(tx2);
        let mut values = vec![];
        while let Ok(v) = rx.try_recv() {
            values.push(v);
        }
        assert_eq!(values, vec![1, 2]);
    }

    #[test]
    fn test_channel_error_display() {
        assert_eq!(ChannelError::Closed.to_string(), "channel closed");
        assert_eq!(ChannelError::Empty.to_string(), "channel empty");
        assert_eq!(ChannelError::Full.to_string(), "channel full");
    }

    #[test]
    fn test_mutex() {
        let mutex = Mutex::new(0);
        {
            let mut guard = mutex.lock();
            *guard = 42;
        }
        assert_eq!(*mutex.lock(), 42);
    }

    #[test]
    fn test_mutex_try_lock() {
        let mutex = Mutex::new(0);
        {
            let _guard = mutex.lock();
            assert!(mutex.try_lock().is_none());
        }
        assert!(mutex.try_lock().is_some());
    }

    #[test]
    fn test_rwlock() {
        let rwlock = RwLock::new(vec![1, 2, 3]);
        {
            let reader = rwlock.read();
            assert_eq!(&*reader, &[1, 2, 3]);
        }
        {
            let mut writer = rwlock.write();
            writer.push(4);
        }
        assert_eq!(&*rwlock.read(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_once() {
        let once = Once::new();
        let mut called = false;
        once.call_once(|| {
            called = true;
            42
        });
        assert!(called);
        assert_eq!(once.get(), Some(&42));
    }

    #[test]
    fn test_atomic_counter() {
        let counter = AtomicCounter::new(5);
        assert_eq!(counter.get(), 5);
        counter.inc();
        assert_eq!(counter.get(), 6);
        counter.dec();
        assert_eq!(counter.get(), 5);
    }

    #[test]
    fn test_atomic_counter_fetch_max() {
        let counter = AtomicCounter::new(3);
        let old = counter.fetch_max(10);
        assert_eq!(old, 3);
        assert_eq!(counter.get(), 10);
    }

    #[test]
    fn test_atomic_counter_fetch_min() {
        let counter = AtomicCounter::new(10);
        let old = counter.fetch_min(3);
        assert_eq!(old, 10);
        assert_eq!(counter.get(), 3);
    }

    #[test]
    fn test_atomic_flag() {
        let flag = AtomicFlag::new(false);
        assert!(!flag.get());
        flag.set(true);
        assert!(flag.get());
        assert!(flag.test_and_set()); // should return true since flag was set
        assert!(flag.get());
        assert!(flag.test_and_clear()); // should return true since flag was set
        assert!(!flag.get());
    }
}
