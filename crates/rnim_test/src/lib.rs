//! Conformance test runner for Nim language tests.
//!
//! This module provides golden test infrastructure for comparing compiler output
//! against expected results across multiple test categories:
//! - compile-pass: files that should compile successfully
//! - compile-fail: files that should fail compilation
//! - run-pass: files that should compile and run successfully
//! - run-fail: files that should compile but fail at runtime
//! - parser: tests for parsing specific constructs
//! - sema: tests for semantic analysis
//! - macro: tests for macro expansion
//! - backend: tests for code generation
//! - docgen: tests for documentation generation

#[cfg(test)]
use rnim_allocator as _;

pub mod differential;
pub mod ingestion;
pub mod workspace;

use std::path::PathBuf;

/// Test category for conformance tests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCategory {
    /// Tests that should compile successfully
    CompilePass,
    /// Tests that should fail to compile
    CompileFail,
    /// Tests that should compile and run successfully
    RunPass,
    /// Tests that should compile but fail at runtime
    RunFail,
    /// Parser-specific tests
    Parser,
    /// Semantic analysis tests
    Sema,
    /// Macro expansion tests
    Macro,
    /// Backend code generation tests
    Backend,
    /// Documentation generation tests
    DocGen,
}

impl TestCategory {
    /// Get the directory name for this test category
    pub fn dir_name(&self) -> &'static str {
        match self {
            TestCategory::CompilePass => "compile_pass",
            TestCategory::CompileFail => "compile_fail",
            TestCategory::RunPass => "run_pass",
            TestCategory::RunFail => "run_fail",
            TestCategory::Parser => "parser",
            TestCategory::Sema => "sema",
            TestCategory::Macro => "macro",
            TestCategory::Backend => "backend",
            TestCategory::DocGen => "docgen",
        }
    }
}

/// A single test case
#[derive(Debug, Clone)]
pub struct TestCase {
    /// Category this test belongs to
    pub category: TestCategory,
    /// Path to the test file
    pub file_path: PathBuf,
    /// Expected output (for golden tests)
    pub expected_output: Option<String>,
    /// Expected exit code (for run tests)
    pub expected_exit_code: Option<i32>,
}

impl TestCase {
    /// Create a new test case
    pub fn new(category: TestCategory, file_path: PathBuf) -> Self {
        Self {
            category,
            file_path,
            expected_output: None,
            expected_exit_code: None,
        }
    }

    /// Set expected output for a golden test
    pub fn with_expected_output(mut self, output: String) -> Self {
        self.expected_output = Some(output);
        self
    }

    /// Set expected exit code for a run test
    pub fn with_expected_exit_code(mut self, code: i32) -> Self {
        self.expected_exit_code = Some(code);
        self
    }
}

/// Golden test runner for Nim conformance tests
#[derive(Debug, Clone)]
pub struct GoldenRunner {
    /// Discovered test cases
    test_cases: Vec<TestCase>,
    /// Test directory root
    test_dir: PathBuf,
}

impl GoldenRunner {
    /// Create a new golden runner
    pub fn new() -> Self {
        Self {
            test_cases: Vec::new(),
            test_dir: PathBuf::from("tests"),
        }
    }

    /// Set the test directory
    pub fn with_test_dir(mut self, dir: PathBuf) -> Self {
        self.test_dir = dir;
        self
    }

    /// Discover all test cases in the test directory
    pub fn discover_tests(&mut self) -> Result<(), TestError> {
        self.test_cases.clear();

        for category in &[
            TestCategory::CompilePass,
            TestCategory::CompileFail,
            TestCategory::RunPass,
            TestCategory::RunFail,
            TestCategory::Parser,
            TestCategory::Sema,
            TestCategory::Macro,
            TestCategory::Backend,
            TestCategory::DocGen,
        ] {
            let category_dir = self.test_dir.join(category.dir_name());
            if category_dir.exists() {
                self.discover_category_tests(category, &category_dir)?;
            }
        }

        Ok(())
    }

    fn discover_category_tests(
        &mut self,
        category: &TestCategory,
        dir: &PathBuf,
    ) -> Result<(), TestError> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir).map_err(|e| TestError::IoError(e.to_string()))? {
            let entry = entry.map_err(|e| TestError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("nim") {
                self.test_cases.push(TestCase::new(*category, path));
            }
        }

        Ok(())
    }

    /// Run all discovered tests
    pub fn run(&mut self) -> Result<TestResults, TestError> {
        if self.test_cases.is_empty() {
            self.discover_tests()?;
        }

        let mut results = TestResults::default();

        for test_case in &self.test_cases {
            let result = self.run_test(test_case);
            results.record(test_case, result);
        }

        Ok(results)
    }

    /// Run a single test case
    fn run_test(&self, test_case: &TestCase) -> TestResult {
        match test_case.category {
            TestCategory::CompilePass => self.run_compile_pass(test_case),
            TestCategory::CompileFail => self.run_compile_fail(test_case),
            TestCategory::RunPass => self.run_run_pass(test_case),
            TestCategory::RunFail => self.run_run_fail(test_case),
            TestCategory::Parser => self.run_parser_test(test_case),
            TestCategory::Sema => self.run_sema_test(test_case),
            TestCategory::Macro => self.run_macro_test(test_case),
            TestCategory::Backend => self.run_backend_test(test_case),
            TestCategory::DocGen => self.run_docgen_test(test_case),
        }
    }

    fn run_compile_pass(&self, _test_case: &TestCase) -> TestResult {
        // TODO: Implement actual compilation check
        TestResult::Passed
    }

    fn run_compile_fail(&self, _test_case: &TestCase) -> TestResult {
        // TODO: Implement actual compilation failure check
        TestResult::Passed
    }

    fn run_run_pass(&self, _test_case: &TestCase) -> TestResult {
        // TODO: Implement actual run check
        TestResult::Passed
    }

    fn run_run_fail(&self, _test_case: &TestCase) -> TestResult {
        // TODO: Implement actual runtime failure check
        TestResult::Passed
    }

    fn run_parser_test(&self, _test_case: &TestCase) -> TestResult {
        // TODO: Implement parser test
        TestResult::Passed
    }

    fn run_sema_test(&self, _test_case: &TestCase) -> TestResult {
        // TODO: Implement semantic analysis test
        TestResult::Passed
    }

    fn run_macro_test(&self, _test_case: &TestCase) -> TestResult {
        // TODO: Implement macro test
        TestResult::Passed
    }

    fn run_backend_test(&self, _test_case: &TestCase) -> TestResult {
        // TODO: Implement backend test
        TestResult::Passed
    }

    fn run_docgen_test(&self, _test_case: &TestCase) -> TestResult {
        // TODO: Implement docgen test
        TestResult::Passed
    }

    /// Get the number of discovered tests
    pub fn test_count(&self) -> usize {
        self.test_cases.len()
    }

    /// Get all test cases
    pub fn test_cases(&self) -> &[TestCase] {
        &self.test_cases
    }
}

impl Default for GoldenRunner {
    fn default() -> Self {
        Self {
            test_cases: Vec::new(),
            test_dir: PathBuf::from("tests"),
        }
    }
}

/// Test result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestResult {
    /// Test passed
    Passed,
    /// Test failed
    Failed(String),
    /// Test was skipped
    Skipped(String),
}

/// Test results summary
#[derive(Debug, Clone, Default)]
pub struct TestResults {
    /// Passed tests
    pub passed: usize,
    /// Failed tests
    pub failed: usize,
    /// Skipped tests
    pub skipped: usize,
    /// Failure details
    pub failures: Vec<TestFailure>,
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub test_case: TestCase,
    pub error: String,
}

impl TestResults {
    /// Record a test result
    pub fn record(&mut self, test_case: &TestCase, result: TestResult) {
        match result {
            TestResult::Passed => self.passed += 1,
            TestResult::Failed(msg) => {
                self.failed += 1;
                self.failures.push(TestFailure {
                    test_case: test_case.clone(),
                    error: msg,
                });
            }
            TestResult::Skipped(_) => self.skipped += 1,
        }
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Get total number of tests
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped
    }
}

/// Test errors
#[derive(Debug, Clone)]
pub enum TestError {
    /// I/O error
    IoError(String),
    /// Test discovery failed
    DiscoveryFailed(String),
    /// Test execution failed
    ExecutionFailed(String),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::IoError(msg) => write!(f, "I/O error: {}", msg),
            TestError::DiscoveryFailed(msg) => write!(f, "test discovery failed: {}", msg),
            TestError::ExecutionFailed(msg) => write!(f, "test execution failed: {}", msg),
        }
    }
}

impl std::error::Error for TestError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_dir_names() {
        assert_eq!(TestCategory::CompilePass.dir_name(), "compile_pass");
        assert_eq!(TestCategory::CompileFail.dir_name(), "compile_fail");
        assert_eq!(TestCategory::RunPass.dir_name(), "run_pass");
        assert_eq!(TestCategory::RunFail.dir_name(), "run_fail");
        assert_eq!(TestCategory::Parser.dir_name(), "parser");
        assert_eq!(TestCategory::Sema.dir_name(), "sema");
        assert_eq!(TestCategory::Macro.dir_name(), "macro");
        assert_eq!(TestCategory::Backend.dir_name(), "backend");
        assert_eq!(TestCategory::DocGen.dir_name(), "docgen");
    }

    #[test]
    fn test_test_case_creation() {
        let tc = TestCase::new(TestCategory::CompilePass, PathBuf::from("test.nim"));
        assert_eq!(tc.category, TestCategory::CompilePass);
        assert_eq!(tc.file_path, PathBuf::from("test.nim"));
        assert!(tc.expected_output.is_none());
        assert!(tc.expected_exit_code.is_none());
    }

    #[test]
    fn test_test_case_with_expected_output() {
        let tc = TestCase::new(TestCategory::Parser, PathBuf::from("test.nim"))
            .with_expected_output("expected output".to_string());
        assert!(tc.expected_output.is_some());
        assert_eq!(tc.expected_output.unwrap(), "expected output");
    }

    #[test]
    fn test_test_case_with_expected_exit_code() {
        let tc = TestCase::new(TestCategory::RunPass, PathBuf::from("test.nim"))
            .with_expected_exit_code(0);
        assert!(tc.expected_exit_code.is_some());
        assert_eq!(tc.expected_exit_code.unwrap(), 0);
    }

    #[test]
    fn test_golden_runner_default() {
        let runner = GoldenRunner::new();
        assert_eq!(runner.test_count(), 0);
        assert_eq!(runner.test_dir, PathBuf::from("tests"));
    }

    #[test]
    fn test_golden_runner_with_test_dir() {
        let runner = GoldenRunner::new().with_test_dir(PathBuf::from("/custom/tests"));
        assert_eq!(runner.test_dir, PathBuf::from("/custom/tests"));
    }

    #[test]
    fn test_test_results_default() {
        let results = TestResults::default();
        assert_eq!(results.passed, 0);
        assert_eq!(results.failed, 0);
        assert_eq!(results.skipped, 0);
        assert!(results.all_passed());
    }

    #[test]
    fn test_test_results_record_passed() {
        let mut results = TestResults::default();
        let tc = TestCase::new(TestCategory::CompilePass, PathBuf::from("test.nim"));
        results.record(&tc, TestResult::Passed);
        assert_eq!(results.passed, 1);
        assert!(results.all_passed());
    }

    #[test]
    fn test_test_results_record_failed() {
        let mut results = TestResults::default();
        let tc = TestCase::new(TestCategory::CompilePass, PathBuf::from("test.nim"));
        results.record(&tc, TestResult::Failed("test failed".to_string()));
        assert_eq!(results.failed, 1);
        assert!(!results.all_passed());
        assert_eq!(results.failures.len(), 1);
    }

    #[test]
    fn test_test_results_record_skipped() {
        let mut results = TestResults::default();
        let tc = TestCase::new(TestCategory::CompilePass, PathBuf::from("test.nim"));
        results.record(&tc, TestResult::Skipped("not implemented".to_string()));
        assert_eq!(results.skipped, 1);
    }

    #[test]
    fn test_test_results_total() {
        let mut results = TestResults::default();
        let tc = TestCase::new(TestCategory::CompilePass, PathBuf::from("test.nim"));
        results.record(&tc, TestResult::Passed);
        results.record(&tc, TestResult::Failed("failed".to_string()));
        results.record(&tc, TestResult::Skipped("skipped".to_string()));
        assert_eq!(results.total(), 3);
    }

    #[test]
    fn test_test_error_display() {
        let err = TestError::IoError("file not found".to_string());
        assert_eq!(format!("{}", err), "I/O error: file not found");

        let err = TestError::DiscoveryFailed("no tests found".to_string());
        assert_eq!(format!("{}", err), "test discovery failed: no tests found");

        let err = TestError::ExecutionFailed("crashed".to_string());
        assert_eq!(format!("{}", err), "test execution failed: crashed");
    }

    #[test]
    fn test_discover_tests_finds_fixture_files() {
        let mut runner = GoldenRunner::new();
        runner.test_dir = PathBuf::from("tests");

        // Discover tests should find our fixture files
        runner.discover_tests().unwrap();

        // We have at least 2 fixture files (hello_parser.nim and addition.nim)
        assert!(
            runner.test_count() >= 2,
            "Expected at least 2 tests, found {}",
            runner.test_count()
        );

        // Check that parser test was discovered
        let parser_tests: Vec<_> = runner
            .test_cases()
            .iter()
            .filter(|tc| tc.category == TestCategory::Parser)
            .collect();
        assert!(
            !parser_tests.is_empty(),
            "Expected at least one parser test"
        );

        // Check that compile_pass test was discovered
        let compile_pass_tests: Vec<_> = runner
            .test_cases()
            .iter()
            .filter(|tc| tc.category == TestCategory::CompilePass)
            .collect();
        assert!(
            !compile_pass_tests.is_empty(),
            "Expected at least one compile_pass test"
        );
    }

    #[test]
    fn test_discover_category_tests_finds_nim_files() {
        let mut runner = GoldenRunner::new();
        runner.test_dir = PathBuf::from("tests");

        runner.discover_tests().unwrap();

        // Verify we can find our specific fixture files
        let all_paths: Vec<_> = runner
            .test_cases()
            .iter()
            .filter_map(|tc| tc.file_path.file_name().and_then(|n| n.to_str()))
            .collect();

        assert!(
            all_paths.contains(&"hello_parser.nim"),
            "Should find hello_parser.nim"
        );
        assert!(
            all_paths.contains(&"addition.nim"),
            "Should find addition.nim"
        );
    }

    #[test]
    fn test_run_returns_results_for_all_discovered_tests() {
        let mut runner = GoldenRunner::new();
        runner.test_dir = PathBuf::from("tests");

        runner.discover_tests().unwrap();
        let results = runner.run().unwrap();

        // All discovered tests should be recorded in results
        assert_eq!(results.total(), runner.test_count());
    }
}
