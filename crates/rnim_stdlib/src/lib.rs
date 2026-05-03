//! Minimal compatible `system`, core modules, and stdlib compatibility shims.

#[cfg(test)]
use rnim_allocator as _;
pub mod compat;
pub mod macros;
pub mod system;

// Re-export commonly used items
pub use system::{
    abort, abs, assert_impl, dec, echo, halt, inc, max, min, quit, swap, NimOption, NimResult,
    NimSeq, NimString, EMPTY, FALSE, NIL, TRUE, VERSION,
};

// Re-export stdlib compatibility items
pub use compat::{
    check_module_compatibility, Backend, CompatibilityPolicy, CompatibilityReport, CoverageStats,
    ModuleCategory, ModuleStatus, StdlibModule, StdlibRegistry,
};

// Re-export macro items
pub use macros::{MacroEnv, MacroError, MacroExpander, MacroResult};
