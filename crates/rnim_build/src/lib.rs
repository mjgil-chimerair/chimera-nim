//! Build and link orchestration.
//!
//! This module provides compile/link plan generation, external compiler
//! invocation, object file management, and cache handling.

use camino::{Utf8Path, Utf8PathBuf};
#[cfg(test)]
use rnim_allocator as _;
use std::collections::HashMap;
use std::process::Command;

/// Link target format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkFormat {
    /// Executable
    Executable,
    /// Shared library
    SharedLib,
    /// Static library
    StaticLib,
    /// Object file
    ObjectFile,
}

impl Default for LinkFormat {
    fn default() -> Self {
        LinkFormat::Executable
    }
}

/// Compile/link plan for a single file
#[derive(Debug, Clone)]
pub struct CompilePlan {
    /// Input file
    pub input: Utf8PathBuf,
    /// Output file
    pub output: Utf8PathBuf,
    /// Format to produce
    pub format: LinkFormat,
    /// Compiler to use
    pub compiler: Compiler,
    /// Compiler flags
    pub flags: Vec<String>,
    /// Link flags
    pub link_flags: Vec<String>,
    /// Dependencies (for rebuild detection)
    pub dependencies: Vec<Utf8PathBuf>,
    /// Working directory
    pub working_dir: Utf8PathBuf,
}

impl CompilePlan {
    pub fn new(input: Utf8PathBuf, output: Utf8PathBuf) -> Self {
        CompilePlan {
            input,
            output,
            format: LinkFormat::Executable,
            compiler: Compiler::default(),
            flags: Vec::new(),
            link_flags: Vec::new(),
            dependencies: Vec::new(),
            working_dir: Utf8PathBuf::from("."),
        }
    }

    /// Set the output format
    pub fn with_format(mut self, format: LinkFormat) -> Self {
        self.format = format;
        self
    }

    /// Set the compiler
    pub fn with_compiler(mut self, compiler: Compiler) -> Self {
        self.compiler = compiler;
        self
    }

    /// Add a compiler flag
    pub fn add_flag(&mut self, flag: &str) {
        self.flags.push(flag.to_string());
    }

    /// Add a link flag
    pub fn add_link_flag(&mut self, flag: &str) {
        self.link_flags.push(flag.to_string());
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dep: Utf8PathBuf) {
        self.dependencies.push(dep);
    }

    /// Get the compile command as a string
    pub fn compile_command(&self) -> String {
        let mut cmd = format!("{} {}", self.compiler.exe(), self.input);
        for flag in &self.flags {
            cmd.push_str(&format!(" {}", flag));
        }
        cmd.push_str(&format!(" -o {}", self.output));
        cmd
    }

    /// Get the link command as a string
    pub fn link_command(&self) -> String {
        let mut cmd = format!("{} ", self.compiler.linker_exe());
        for flag in &self.link_flags {
            cmd.push_str(&format!(" {}", flag));
        }
        cmd.push_str(&format!(" -o {}", self.output));
        cmd
    }
}

/// External compiler selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compiler {
    /// GCC
    Gcc,
    /// Clang
    Clang,
    /// MSVC
    Msvc,
    /// Custom command
    Custom(&'static str),
}

impl Default for Compiler {
    fn default() -> Self {
        Compiler::Gcc
    }
}

impl Compiler {
    /// Get the compiler executable name
    pub fn exe(&self) -> &str {
        match self {
            Compiler::Gcc => "gcc",
            Compiler::Clang => "clang",
            Compiler::Msvc => "cl",
            Compiler::Custom(cmd) => cmd,
        }
    }

    /// Get the linker executable name
    pub fn linker_exe(&self) -> &str {
        match self {
            Compiler::Gcc => "ld",
            Compiler::Clang => "ld.lld",
            Compiler::Msvc => "link",
            Compiler::Custom(cmd) => cmd,
        }
    }

    /// Get the standard flags for this compiler
    pub fn standard_flags(&self) -> Vec<&'static str> {
        match self {
            Compiler::Gcc => vec!["-Wall", "-Wextra", "-pedantic"],
            Compiler::Clang => vec!["-Wall", "-Wextra", "-pedantic"],
            Compiler::Msvc => vec!["/W4"],
            Compiler::Custom(_) => vec![],
        }
    }

    /// Detect if a compiler is available on the system
    pub fn detect() -> Option<Compiler> {
        if Command::new("clang").output().is_ok() {
            Some(Compiler::Clang)
        } else if Command::new("gcc").output().is_ok() {
            Some(Compiler::Gcc)
        } else if Command::new("cl").output().is_ok() {
            Some(Compiler::Msvc)
        } else {
            None
        }
    }
}

/// Build cache key computation
#[derive(Debug, Clone)]
pub struct CacheKey {
    /// Hash of the input file content
    pub input_hash: u64,
    /// Hash of compiler flags
    pub flags_hash: u64,
    /// Hash of compiler version
    pub compiler_version_hash: u64,
    /// Full hash
    pub full_hash: u64,
}

impl CacheKey {
    /// Create a new cache key from components
    pub fn new(input_hash: u64, flags_hash: u64, compiler_version_hash: u64) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        input_hash.hash(&mut hasher);
        flags_hash.hash(&mut hasher);
        compiler_version_hash.hash(&mut hasher);
        let full_hash = hasher.finish();

        CacheKey {
            input_hash,
            flags_hash,
            compiler_version_hash,
            full_hash,
        }
    }

    /// Get the cache file name
    pub fn cache_file_name(&self) -> String {
        format!("{:016x}.cache", self.full_hash)
    }
}

/// Build cache manager
#[derive(Debug, Clone, Default)]
pub struct BuildCache {
    /// Cache directory
    cache_dir: Utf8PathBuf,
    /// Known cache entries
    entries: HashMap<Utf8PathBuf, CacheEntry>,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub output: Utf8PathBuf,
    pub timestamp: std::time::SystemTime,
}

impl BuildCache {
    /// Create a new build cache
    pub fn new(cache_dir: Utf8PathBuf) -> Self {
        BuildCache {
            cache_dir,
            entries: HashMap::new(),
        }
    }

    /// Check if a cached output exists and is valid
    pub fn get_cached(&self, input: &Utf8Path, key: &CacheKey) -> Option<Utf8PathBuf> {
        // Check if cache file exists and is valid
        let cache_file = self.cache_dir.join(key.cache_file_name());
        if cache_file.exists() {
            // Check if input file hasn't been modified since cache was created
            if let Ok(input_meta) = std::fs::metadata(input) {
                if let Ok(cache_meta) = std::fs::metadata(&cache_file) {
                    if let (Ok(input_modified), Ok(cache_modified)) =
                        (input_meta.modified(), cache_meta.modified())
                    {
                        // If input is older than cache, use cached output
                        if input_modified <= cache_modified {
                            return Some(self.cache_dir.join("out").join(key.cache_file_name()));
                        }
                    }
                }
            }
        }
        None
    }

    /// Store a compiled output in the cache
    pub fn store(&mut self, input: Utf8PathBuf, key: CacheKey, output: Utf8PathBuf) {
        let entry = CacheEntry {
            key,
            output,
            timestamp: std::time::SystemTime::now(),
        };
        self.entries.insert(input, entry);
    }

    /// Get the cache directory
    pub fn cache_dir(&self) -> &Utf8Path {
        &self.cache_dir
    }
}

/// Build orchestrator
#[derive(Debug, Clone, Default)]
pub struct BuildOrchestrator {
    /// Compile plans
    plans: Vec<CompilePlan>,
    /// Build cache
    cache: BuildCache,
    /// Compiler to use
    compiler: Compiler,
}

impl BuildOrchestrator {
    /// Create a new build orchestrator
    pub fn new(cache_dir: Utf8PathBuf) -> Self {
        BuildOrchestrator {
            plans: Vec::new(),
            cache: BuildCache::new(cache_dir),
            compiler: Compiler::default(),
        }
    }

    /// Set the compiler
    pub fn with_compiler(mut self, compiler: Compiler) -> Self {
        self.compiler = compiler;
        self
    }

    /// Add a compile plan
    pub fn add_plan(&mut self, plan: CompilePlan) {
        self.plans.push(plan);
    }

    /// Create a plan from input/output files
    pub fn plan(&mut self, input: Utf8PathBuf, output: Utf8PathBuf) -> &mut CompilePlan {
        let mut plan = CompilePlan::new(input, output);
        plan.compiler = self.compiler;
        self.plans.push(plan);
        self.plans.last_mut().unwrap()
    }

    /// Execute all compile plans
    pub fn compile_all(&self) -> Result<(), BuildError> {
        for plan in &self.plans {
            self.compile_plan(plan)?;
        }
        Ok(())
    }

    /// Execute a single compile plan
    fn compile_plan(&self, plan: &CompilePlan) -> Result<(), BuildError> {
        let output = Command::new(plan.compiler.exe())
            .arg(&plan.input)
            .args(&plan.flags)
            .arg("-o")
            .arg(&plan.output)
            .output()
            .map_err(|e| BuildError::CompileFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(BuildError::CompileFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Check if any inputs have changed since last build
    pub fn needs_rebuild(&self) -> bool {
        // Check if any cached output is older than its input
        for plan in &self.plans {
            let cache_key = CacheKey::new(0, 0, 0); // Placeholder
            if let Some(cached) = self.cache.get_cached(&plan.input, &cache_key) {
                // If cache exists and is valid, no rebuild needed
                if cached.exists() {
                    continue;
                }
            }
            // Cache miss or invalid means we need to rebuild
            return true;
        }
        false
    }
}

/// Build error types
#[derive(Debug, Clone)]
pub enum BuildError {
    /// Compile command failed
    CompileFailed(String),
    /// Link command failed
    LinkFailed(String),
    /// Missing compiler
    MissingCompiler(String),
    /// Invalid path
    InvalidPath(String),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::CompileFailed(msg) => write!(f, "compile failed: {}", msg),
            BuildError::LinkFailed(msg) => write!(f, "link failed: {}", msg),
            BuildError::MissingCompiler(name) => write!(f, "missing compiler: {}", name),
            BuildError::InvalidPath(msg) => write!(f, "invalid path: {}", msg),
        }
    }
}

impl std::error::Error for BuildError {}

/// Linker invocation helper
pub fn link_executable(
    objects: &[&str],
    output: &str,
    compiler: Compiler,
) -> Result<(), BuildError> {
    let status = Command::new(compiler.linker_exe())
        .args(objects)
        .arg("-o")
        .arg(output)
        .status()
        .map_err(|e| BuildError::LinkFailed(e.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(BuildError::LinkFailed(format!(
            "linker exited with status: {}",
            status
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_format_default() {
        assert_eq!(LinkFormat::default(), LinkFormat::Executable);
    }

    #[test]
    fn test_compiler_exe() {
        assert_eq!(Compiler::Gcc.exe(), "gcc");
        assert_eq!(Compiler::Clang.exe(), "clang");
        assert_eq!(Compiler::Msvc.exe(), "cl");
    }

    #[test]
    fn test_compiler_linker_exe() {
        assert_eq!(Compiler::Gcc.linker_exe(), "ld");
        assert_eq!(Compiler::Clang.linker_exe(), "ld.lld");
        assert_eq!(Compiler::Msvc.linker_exe(), "link");
    }

    #[test]
    fn test_compiler_standard_flags() {
        assert!(Compiler::Gcc.standard_flags().contains(&"-Wall"));
        assert!(Compiler::Clang.standard_flags().contains(&"-Wall"));
        assert!(Compiler::Msvc.standard_flags().contains(&"/W4"));
    }

    #[test]
    fn test_cache_key_new() {
        let key = CacheKey::new(123, 456, 789);
        assert_eq!(key.input_hash, 123);
        assert_eq!(key.flags_hash, 456);
        assert_eq!(key.compiler_version_hash, 789);
    }

    #[test]
    fn test_cache_key_file_name() {
        let key = CacheKey::new(0, 0, 0);
        let name = key.cache_file_name();
        assert!(name.ends_with(".cache"));
        assert!(name.contains("0"));
    }

    #[test]
    fn test_build_cache_new() {
        let cache = BuildCache::new(Utf8PathBuf::from("/tmp/cache"));
        assert_eq!(cache.cache_dir().as_str(), "/tmp/cache");
    }

    #[test]
    fn test_build_cache_get_cached() {
        let cache = BuildCache::new(Utf8PathBuf::from("/tmp/cache"));
        let key = CacheKey::new(1, 2, 3);
        let result = cache.get_cached(Utf8Path::new("input.c"), &key);
        assert!(result.is_none());
    }

    #[test]
    fn test_build_cache_store() {
        let mut cache = BuildCache::new(Utf8PathBuf::from("/tmp/cache"));
        let key = CacheKey::new(1, 2, 3);
        cache.store(
            Utf8PathBuf::from("input.c"),
            key,
            Utf8PathBuf::from("output.o"),
        );
        assert_eq!(cache.entries.len(), 1);
    }

    #[test]
    fn test_compile_plan_new() {
        let plan = CompilePlan::new(Utf8PathBuf::from("input.c"), Utf8PathBuf::from("output.o"));
        assert_eq!(plan.input.as_str(), "input.c");
        assert_eq!(plan.output.as_str(), "output.o");
        assert_eq!(plan.format, LinkFormat::Executable);
        assert!(plan.flags.is_empty());
    }

    #[test]
    fn test_compile_plan_with_format() {
        let plan = CompilePlan::new(Utf8PathBuf::from("input.c"), Utf8PathBuf::from("output.o"))
            .with_format(LinkFormat::SharedLib);
        assert_eq!(plan.format, LinkFormat::SharedLib);
    }

    #[test]
    fn test_compile_plan_add_flag() {
        let mut plan =
            CompilePlan::new(Utf8PathBuf::from("input.c"), Utf8PathBuf::from("output.o"));
        plan.add_flag("-O2");
        assert!(plan.flags.contains(&"-O2".to_string()));
    }

    #[test]
    fn test_compile_plan_compile_command() {
        let plan = CompilePlan::new(Utf8PathBuf::from("input.c"), Utf8PathBuf::from("output.o"));
        let cmd = plan.compile_command();
        assert!(cmd.contains("input.c"));
        assert!(cmd.contains("output.o"));
    }

    #[test]
    fn test_build_orchestrator_new() {
        let orch = BuildOrchestrator::new(Utf8PathBuf::from("/tmp/build"));
        assert!(orch.plans.is_empty());
    }

    #[test]
    fn test_build_orchestrator_with_compiler() {
        let orch =
            BuildOrchestrator::new(Utf8PathBuf::from("/tmp/build")).with_compiler(Compiler::Clang);
        assert_eq!(orch.compiler, Compiler::Clang);
    }

    #[test]
    fn test_build_orchestrator_add_plan() {
        let mut orch = BuildOrchestrator::new(Utf8PathBuf::from("/tmp/build"));
        let plan = CompilePlan::new(Utf8PathBuf::from("input.c"), Utf8PathBuf::from("output.o"));
        orch.add_plan(plan);
        assert_eq!(orch.plans.len(), 1);
    }

    #[test]
    fn test_build_orchestrator_plan() {
        let mut orch = BuildOrchestrator::new(Utf8PathBuf::from("/tmp/build"));
        orch.plan(Utf8PathBuf::from("input.c"), Utf8PathBuf::from("output.o"));
        assert_eq!(orch.plans.len(), 1);
    }

    #[test]
    fn test_build_error_display() {
        let err = BuildError::CompileFailed("gcc failed".to_string());
        assert!(format!("{}", err).contains("compile failed"));

        let err = BuildError::LinkFailed("link failed".to_string());
        assert!(format!("{}", err).contains("link failed"));

        let err = BuildError::MissingCompiler("gcc".to_string());
        assert!(format!("{}", err).contains("missing compiler"));

        let err = BuildError::InvalidPath("/nonexistent".to_string());
        assert!(format!("{}", err).contains("invalid path"));
    }
}
