//! Minimal system module for Nim compatibility.
//!
//! This module provides the core types, operators, hooks, and magic
//! symbols required by the Nim compiler for basic operation.

/// System module version
pub const VERSION: &str = "2.2.0";

/// Nil pointer constant
pub const NIL: isize = 0;

/// Size of a pointer in bytes
pub const PTR_SIZE: usize = 8;

/// Size of a byte
pub const BYTE_SIZE: usize = 1;

/// Maximum integer value for int type
pub const MAX_INT: i64 = i64::MAX;

/// Minimum integer value for int type
pub const MIN_INT: i64 = i64::MIN;

/// Maximum value for signed 8-bit integer
pub const MAX_INT8: i8 = i8::MAX;

/// Minimum value for signed 8-bit integer
pub const MIN_INT8: i8 = i8::MIN;

/// Maximum value for signed 16-bit integer
pub const MAX_INT16: i16 = i16::MAX;

/// Minimum value for signed 16-bit integer
pub const MIN_INT16: i16 = i16::MIN;

/// Maximum value for signed 32-bit integer
pub const MAX_INT32: i32 = i32::MAX;

/// Minimum value for signed 32-bit integer
pub const MIN_INT32: i32 = i32::MIN;

/// Maximum value for signed 64-bit integer
pub const MAX_INT64: i64 = i64::MAX;

/// Minimum value for signed 64-bit integer
pub const MIN_INT64: i64 = i64::MIN;

/// Maximum value for unsigned 8-bit integer
pub const MAX_UINT8: u8 = u8::MAX;

/// Maximum value for unsigned 16-bit integer
pub const MAX_UINT16: u16 = u16::MAX;

/// Maximum value for unsigned 32-bit integer
pub const MAX_UINT32: u32 = u32::MAX;

/// Maximum value for unsigned 64-bit integer
pub const MAX_UINT64: u64 = u64::MAX;

/// pi constant
pub const PI: f64 = 3.14159265358979323846;

/// tau constant (2*pi)
pub const TAU: f64 = 6.28318530717958647692;

/// epsilon for float comparisons
pub const EPSILON: f64 = 1.0e-12;

/// Infinity value
pub const INF: f64 = f64::INFINITY;

/// Negative infinity value
pub const NEGINF: f64 = f64::NEG_INFINITY;

/// Not a number value
pub const NAN: f64 = f64::NAN;

/// Represents a boolean true value (magic constant)
pub const TRUE: i32 = 1;

/// Represents a boolean false value (magic constant)
pub const FALSE: i32 = 0;

/// Empty tuple constant
pub const EMPTY: () = ();

/// Panic/assertion hook function type
pub type PanicHook = fn(msg: &str, file: &str, line: u32, col: u32) -> !;

/// Global panic hook
static mut PANIC_HOOK: PanicHook = default_panic_hook;

/// Set the global panic hook
pub fn set_panic_hook(hook: PanicHook) {
    unsafe {
        PANIC_HOOK = hook;
    }
}

/// Default panic hook that aborts
fn default_panic_hook(msg: &str, file: &str, line: u32, _col: u32) -> ! {
    eprintln!("Assertion failed: {} at {}:{}", msg, file, line);
    std::process::abort();
}

/// Built-in echo function for printing
pub fn echo(args: &[&str]) {
    println!("{}", args.join(" "));
}

/// Built-in quit function
pub fn quit(code: i32) -> ! {
    std::process::exit(code);
}

/// Built-in halt function
pub fn halt() -> ! {
    std::process::exit(1);
}

/// Built-in abort function
pub fn abort() -> ! {
    std::process::abort();
}

/// Magic marker for compiler-generated symbols
pub const MAGIC_MARKER: &str = "__nim";

/// Represents the result type used by exception handling
pub enum NimResult<T> {
    Ok(T),
    Error(Box<String>),
}

impl<T> NimResult<T> {
    pub fn is_ok(&self) -> bool {
        matches!(self, NimResult::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }

    pub fn get(self) -> T {
        match self {
            NimResult::Ok(value) => value,
            NimResult::Error(_) => panic!("Result is Error"),
        }
    }

    pub fn get_or(self, default: T) -> T {
        match self {
            NimResult::Ok(value) => value,
            NimResult::Error(_) => default,
        }
    }
}

/// Option type for partial operations
pub enum NimOption<T> {
    Some(T),
    None,
}

impl<T> NimOption<T> {
    pub fn is_some(&self) -> bool {
        matches!(self, NimOption::Some(_))
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    pub fn get(self) -> T {
        match self {
            NimOption::Some(value) => value,
            NimOption::None => panic!("Option is None"),
        }
    }

    pub fn get_or(self, default: T) -> T {
        match self {
            NimOption::Some(value) => value,
            NimOption::None => default,
        }
    }
}

/// String type for Nim compatibility
pub struct NimString {
    data: Vec<u8>,
    len: usize,
}

impl NimString {
    pub fn new(s: &str) -> NimString {
        let bytes = s.as_bytes().to_vec();
        let len = bytes.len();
        NimString { data: bytes, len }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn cstring(&self) -> *const u8 {
        self.data.as_ptr()
    }
}

/// Sequence type for Nim compatibility
pub struct NimSeq<T> {
    data: Vec<T>,
    len: usize,
}

impl<T> NimSeq<T> {
    pub fn new() -> NimSeq<T> {
        NimSeq {
            data: Vec::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn add(&mut self, item: T) {
        self.data.push(item);
        self.len = self.data.len();
    }

    pub fn pop(&mut self) -> NimOption<T> {
        if self.len == 0 {
            NimOption::None
        } else {
            self.len -= 1;
            NimOption::Some(self.data.pop().unwrap())
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.len = 0;
    }
}

impl<T: Clone> Clone for NimSeq<T> {
    fn clone(&self) -> NimSeq<T> {
        NimSeq {
            data: self.data.clone(),
            len: self.len,
        }
    }
}

impl<T> Default for NimSeq<T> {
    fn default() -> NimSeq<T> {
        NimSeq::new()
    }
}

/// Built-in inc procedure
pub fn inc(x: &mut i64) {
    *x += 1;
}

/// Built-in dec procedure
pub fn dec(x: &mut i64) {
    *x -= 1;
}

/// Built-in abs procedure
pub fn abs(x: i64) -> i64 {
    if x < 0 {
        -x
    } else {
        x
    }
}

/// Built-in min procedure
pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        a
    } else {
        b
    }
}

/// Built-in max procedure
pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b {
        a
    } else {
        b
    }
}

/// Built-in swap procedure
pub fn swap<T: Clone>(a: &mut T, b: &mut T) {
    let temp = (*a).clone();
    *a = (*b).clone();
    *b = temp;
}

/// Built-in assert procedure
pub fn assert_impl(condition: bool, msg: &str, file: &str, line: u32, col: u32) {
    if !condition {
        unsafe {
            PANIC_HOOK(msg, file, line, col);
        }
    }
}

/// Built-in static assert
pub fn static_assert(condition: bool, msg: &str) {
    if !condition {
        panic!("static assert failed: {}", msg);
    }
}

/// Built-in debug printer
pub fn debugEcho(args: &[&str]) {
    eprintln!("DEBUG: {}", args.join(" "));
}

/// Magic marker for procedure call operators
pub const OP_CALL: &str = "__call";

/// Magic marker for indexing operators
pub const OP_IDX: &str = "__idx";

/// Magic marker for field access
pub const OP_FIELD: &str = "__field";

/// Magic marker for unary minus
pub const OP_NEG: &str = "__neg";

/// Magic marker for binary plus
pub const OP_PLUS: &str = "__plus";

/// Magic marker for binary minus
pub const OP_MINUS: &str = "__minus";

/// Magic marker for multiplication
pub const OP_MUL: &str = "__mul";

/// Magic marker for division
pub const OP_DIV: &str = "__div";

/// Internal type descriptor marker
pub const TYPE_DESC: &str = "__typedesc";

/// Object marker for inheritance
pub const OBJECT_MARKER: &str = "__object";

/// Marker for method calls
pub const METHOD_CALL: &str = "__method";

/// Marker for tuple construction
pub const TUPLE_CONS: &str = "__tuple";

/// Marker for closure conversion
pub const CLOSURE_CONS: &str = "__closure";

/// Built-in sizeof operator
pub const fn sizeof<T>(_: &T) -> usize {
    std::mem::size_of::<T>()
}

/// Built-in aligned sizeof operator
pub const fn alignof<T>(_: &T) -> usize {
    std::mem::align_of::<T>()
}

/// Built-in typeinfo
pub const TYPEINFO: &str = "__typeinfo";

/// Thread-local storage marker
pub const THREAD_LOCAL: &str = "__thread_local";

/// Compile-time known marker
pub const CTKNOWN: &str = "__ctknown";

/// No return marker
pub const NO_RETURN: &str = "__noreturn";

/// Compiler import marker
pub const COMPILER_MAGIC: &str = "__compiler";

/// FFI import marker
pub const FFI_MAGIC: &str = "__ffi";

/// Dynlib marker
pub const DYNLIB_MAGIC: &str = "__dynlib";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nim_string_new() {
        let s = NimString::new("hello");
        assert_eq!(s.len, 5);
    }

    #[test]
    fn test_nim_string_empty() {
        let s = NimString::new("");
        assert!(s.is_empty());
    }

    #[test]
    fn test_nim_seq_new() {
        let seq: NimSeq<i32> = NimSeq::new();
        assert!(seq.is_empty());
        assert_eq!(seq.len, 0);
    }

    #[test]
    fn test_nim_seq_add() {
        let mut seq: NimSeq<i32> = NimSeq::new();
        seq.add(42);
        assert_eq!(seq.len, 1);
        assert!(!seq.is_empty());
    }

    #[test]
    fn test_nim_seq_pop() {
        let mut seq: NimSeq<i32> = NimSeq::new();
        seq.add(42);
        let item = seq.pop();
        assert!(matches!(item, NimOption::Some(42)));
        assert_eq!(seq.len, 0);
    }

    #[test]
    fn test_nim_result_ok() {
        let r: NimResult<i32> = NimResult::Ok(42);
        assert!(r.is_ok());
        assert!(!r.is_err());
        assert_eq!(r.get(), 42);
    }

    #[test]
    fn test_nim_result_err() {
        let r: NimResult<i32> = NimResult::Error("error".to_string().into());
        assert!(!r.is_ok());
        assert!(r.is_err());
    }

    #[test]
    fn test_nim_option_some() {
        let o: NimOption<i32> = NimOption::Some(42);
        assert!(o.is_some());
        assert!(!o.is_none());
        assert_eq!(o.get(), 42);
    }

    #[test]
    fn test_nim_option_none() {
        let o: NimOption<i32> = NimOption::None;
        assert!(!o.is_some());
        assert!(o.is_none());
    }

    #[test]
    fn test_inc() {
        let mut x: i64 = 5;
        inc(&mut x);
        assert_eq!(x, 6);
    }

    #[test]
    fn test_dec() {
        let mut x: i64 = 5;
        dec(&mut x);
        assert_eq!(x, 4);
    }

    #[test]
    fn test_abs() {
        assert_eq!(abs(-5), 5);
        assert_eq!(abs(5), 5);
        assert_eq!(abs(0), 0);
    }

    #[test]
    fn test_min() {
        assert_eq!(min(3, 5), 3);
        assert_eq!(min(5, 3), 3);
        assert_eq!(min(5, 5), 5);
    }

    #[test]
    fn test_max() {
        assert_eq!(max(3, 5), 5);
        assert_eq!(max(5, 3), 5);
        assert_eq!(max(5, 5), 5);
    }

    #[test]
    fn test_swap() {
        let mut a = 1;
        let mut b = 2;
        swap(&mut a, &mut b);
        assert_eq!(a, 2);
        assert_eq!(b, 1);
    }

    #[test]
    fn test_nim_result_get_or() {
        let r: NimResult<i32> = NimResult::Ok(42);
        assert_eq!(r.get_or(0), 42);

        let r: NimResult<i32> = NimResult::Error("error".to_string().into());
        assert_eq!(r.get_or(0), 0);
    }

    #[test]
    fn test_nim_option_get_or() {
        let o: NimOption<i32> = NimOption::Some(42);
        assert_eq!(o.get_or(0), 42);

        let o: NimOption<i32> = NimOption::None;
        assert_eq!(o.get_or(0), 0);
    }

    #[test]
    fn test_nim_seq_clone() {
        let mut seq: NimSeq<i32> = NimSeq::new();
        seq.add(1);
        seq.add(2);
        let cloned = seq.clone();
        assert_eq!(cloned.len, 2);
    }
}
