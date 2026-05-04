//! Differential testing infrastructure for comparing chimera-nim against official Nim.
//!
//! This module provides infrastructure for running the same source through both
//! chimera-nim and official Nim, then comparing the outputs.

use std::path::PathBuf;
use std::process::Command;

/// Result of a differential test comparison
#[derive(Debug, Clone)]
pub struct DifferentialResult {
    /// Whether the test passed (outputs match)
    pub passed: bool,
    /// chimera-nim output
    pub chimera_output: TestOutput,
    /// Official Nim output
    pub official_output: TestOutput,
    /// Description of the difference
    pub difference: Option<String>,
}

/// Output from a compiler run
#[derive(Debug, Clone, Default)]
pub struct TestOutput {
    /// stdout content
    pub stdout: String,
    /// stderr content
    pub stderr: String,
    /// Exit code
    pub exit_code: i32,
}

impl TestOutput {
    /// Create a new test output
    pub fn new(stdout: String, stderr: String, exit_code: i32) -> Self {
        Self {
            stdout,
            stderr,
            exit_code,
        }
    }

    /// Check if outputs are equal
    pub fn matches(&self, other: &TestOutput, compare_diagnostics: bool) -> bool {
        if self.exit_code != other.exit_code {
            return false;
        }
        if compare_diagnostics {
            // Compare stderr for diagnostic messages
            // but allow for different line numbers/file paths
            self.normalize_diagnostics(&other.stderr) == other.normalize_diagnostics(&self.stderr)
        } else {
            self.stdout == other.stdout
        }
    }

    /// Normalize diagnostic output for comparison
    /// Strips file paths and line numbers that may differ
    fn normalize_diagnostics(&self, _other_stderr: &str) -> String {
        // For now, just compare exit codes and stdout
        // A more sophisticated implementation would normalize error messages
        self.stdout.clone()
    }
}

/// Configuration for differential testing
#[derive(Debug, Clone)]
pub struct DifferentialConfig {
    /// Path to official Nim compiler (nim)
    pub nim_path: Option<PathBuf>,
    /// Whether to compare diagnostics
    pub compare_diagnostics: bool,
    /// Whether to run in CI mode (fail on missing Nim)
    pub ci_mode: bool,
}

impl Default for DifferentialConfig {
    fn default() -> Self {
        Self {
            nim_path: None,
            compare_diagnostics: true,
            ci_mode: false,
        }
    }
}

impl DifferentialConfig {
    /// Create a new config with auto-detected Nim
    pub fn auto_detect() -> Self {
        Self {
            nim_path: find_official_nim(),
            compare_diagnostics: true,
            ci_mode: false,
        }
    }

    /// Set custom Nim path
    pub fn with_nim_path(mut self, path: PathBuf) -> Self {
        self.nim_path = Some(path);
        self
    }

    /// Enable CI mode
    pub fn with_ci_mode(mut self) -> Self {
        self.ci_mode = true;
        self
    }
}

/// Find official Nim compiler in PATH
pub fn find_official_nim() -> Option<PathBuf> {
    // Try common names
    for name in &["nim", "nim-2.2", "nim-2.0", "nim-1.6"] {
        if let Ok(path) = std::process::Command::new(name)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
        {
            if path {
                // Verify it's not chimera
                if let Ok(version_output) =
                    std::process::Command::new(name).arg("--version").output()
                {
                    let version = String::from_utf8_lossy(&version_output.stdout);
                    if !version.contains("chimera") {
                        return Some(PathBuf::from(name));
                    }
                }
            }
        }
    }
    None
}

/// Get Nim version string
fn get_nim_version(nim_path: &PathBuf) -> Result<String, String> {
    let output = Command::new(nim_path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Differential test runner
#[derive(Debug, Clone)]
pub struct DifferentialRunner {
    config: DifferentialConfig,
}

impl DifferentialRunner {
    /// Create a new runner with auto-detected config
    pub fn new() -> Self {
        Self {
            config: DifferentialConfig::auto_detect(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: DifferentialConfig) -> Self {
        Self { config }
    }

    /// Check if official Nim is available
    pub fn is_nim_available(&self) -> bool {
        self.config.nim_path.is_some()
    }

    /// Get the configured Nim path
    pub fn nim_path(&self) -> Option<&PathBuf> {
        self.config.nim_path.as_ref()
    }

    /// Run a single differential test
    pub fn run_test(&self, source: &PathBuf) -> DifferentialResult {
        // For now, return a placeholder result
        // Full implementation would compile with both compilers and compare
        DifferentialResult {
            passed: false,
            chimera_output: TestOutput::default(),
            official_output: TestOutput::default(),
            difference: Some("Differential testing not yet fully implemented".to_string()),
        }
    }

    /// Compare two outputs
    pub fn compare(&self, chimera: &TestOutput, official: &TestOutput) -> bool {
        chimera.matches(official, self.config.compare_diagnostics)
    }
}

impl Default for DifferentialRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Tolerance configuration for comparison
#[derive(Debug, Clone)]
pub struct ComparisonTolerance {
    /// Ignore exit code differences
    pub ignore_exit_code: bool,
    /// Ignore whitespace differences
    pub ignore_whitespace: bool,
    /// Allowed exit code differences (e.g., 127 for command not found)
    pub allowed_exit_code_diffs: Vec<i32>,
}

impl Default for ComparisonTolerance {
    fn default() -> Self {
        Self {
            ignore_exit_code: false,
            ignore_whitespace: false,
            allowed_exit_code_diffs: vec![127], // command not found
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_output_equality() {
        let out1 = TestOutput::new("hello".to_string(), "".to_string(), 0);
        let out2 = TestOutput::new("hello".to_string(), "".to_string(), 0);
        assert!(out1.matches(&out2, false));
    }

    #[test]
    fn test_test_output_inequality() {
        let out1 = TestOutput::new("hello".to_string(), "".to_string(), 0);
        let out2 = TestOutput::new("world".to_string(), "".to_string(), 0);
        assert!(!out1.matches(&out2, false));
    }

    #[test]
    fn test_test_output_exit_code_mismatch() {
        let out1 = TestOutput::new("hello".to_string(), "".to_string(), 0);
        let out2 = TestOutput::new("hello".to_string(), "".to_string(), 1);
        assert!(!out1.matches(&out2, false));
    }

    #[test]
    fn test_differential_config_default() {
        let config = DifferentialConfig::default();
        assert!(config.nim_path.is_none());
        assert!(config.compare_diagnostics);
        assert!(!config.ci_mode);
    }

    #[test]
    fn test_differential_runner_no_nim() {
        let runner = DifferentialRunner::with_config(DifferentialConfig::default());
        assert!(!runner.is_nim_available());
    }

    #[test]
    fn test_differential_runner_with_path() {
        let config = DifferentialConfig::default().with_nim_path(PathBuf::from("/usr/bin/nim"));
        let runner = DifferentialRunner::with_config(config);
        assert!(runner.is_nim_available());
        assert_eq!(runner.nim_path().unwrap(), &PathBuf::from("/usr/bin/nim"));
    }

    #[test]
    fn test_comparison_tolerance_default() {
        let tolerance = ComparisonTolerance::default();
        assert!(!tolerance.ignore_exit_code);
        assert!(!tolerance.ignore_whitespace);
        assert_eq!(tolerance.allowed_exit_code_diffs, vec![127]);
    }
}
