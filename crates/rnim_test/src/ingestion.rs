//! Test ingestion and provenance tracking for official Nim tests.
//!
//! This module provides infrastructure for importing test cases while tracking:
//! - Source provenance (official Nim, community, custom)
//! - Licensing status
//! - Supported/deferred status with reasons

use std::collections::HashMap;
use std::path::PathBuf;

/// Source of a test case
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestProvenance {
    /// Tests from official Nim compiler repository
    Official,
    /// Community-contributed tests
    Community,
    /// Custom tests written for chimera-nim
    Custom,
    /// Tests from third-party sources
    ThirdParty,
}

impl TestProvenance {
    /// Get the directory name for this provenance
    pub fn dir_name(&self) -> &'static str {
        match self {
            TestProvenance::Official => "official",
            TestProvenance::Community => "community",
            TestProvenance::Custom => "custom",
            TestProvenance::ThirdParty => "third_party",
        }
    }
}

/// Status of a test case
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestStatus {
    /// Test is supported and should pass
    Supported,
    /// Test is not yet supported but is being tracked
    Deferred,
    /// Test is explicitly not supported
    Unsupported,
}

impl TestStatus {
    /// Get the directory name for this status
    pub fn dir_name(&self) -> &'static str {
        match self {
            TestStatus::Supported => "supported",
            TestStatus::Deferred => "deferred",
            TestStatus::Unsupported => "unsupported",
        }
    }
}

/// Reason for deferral or non-support
#[derive(Debug, Clone)]
pub struct DeferralReason {
    /// Category of reason
    pub category: DeferralCategory,
    /// Human-readable description
    pub description: String,
    /// Issue or task tracking this
    pub tracking_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferralCategory {
    /// Feature not yet implemented
    FeatureNotImplemented,
    /// Known bug in chimera-nim
    KnownBug,
    /// Known incompatibility with official Nim
    Incompatibility,
    /// Licensing issue
    Licensing,
    /// Platform-specific issue
    PlatformSpecific,
    /// Backend limitation
    BackendLimitation,
}

/// Metadata for a test case
#[derive(Debug, Clone)]
pub struct TestMetadata {
    /// Test provenance
    pub provenance: TestProvenance,
    /// Test status
    pub status: TestStatus,
    /// Deferral reason if deferred or unsupported
    pub deferral_reason: Option<DeferralReason>,
    /// Original test file path (for imported tests)
    pub original_path: Option<PathBuf>,
    /// Notes about the test
    pub notes: Vec<String>,
}

impl TestMetadata {
    /// Create metadata for a custom test
    pub fn custom() -> Self {
        Self {
            provenance: TestProvenance::Custom,
            status: TestStatus::Supported,
            deferral_reason: None,
            original_path: None,
            notes: Vec::new(),
        }
    }

    /// Create metadata for an official test
    pub fn official(status: TestStatus, reason: Option<DeferralReason>) -> Self {
        Self {
            provenance: TestProvenance::Official,
            status,
            deferral_reason: reason,
            original_path: None,
            notes: Vec::new(),
        }
    }

    /// Mark test as deferred with a reason
    pub fn deferred(
        category: DeferralCategory,
        description: String,
        tracking_ref: Option<String>,
    ) -> Self {
        Self {
            provenance: TestProvenance::Official,
            status: TestStatus::Deferred,
            deferral_reason: Some(DeferralReason {
                category,
                description,
                tracking_ref,
            }),
            original_path: None,
            notes: Vec::new(),
        }
    }

    /// Mark test as unsupported
    pub fn unsupported(category: DeferralCategory, description: String) -> Self {
        Self {
            provenance: TestProvenance::Official,
            status: TestStatus::Unsupported,
            deferral_reason: Some(DeferralReason {
                category,
                description,
                tracking_ref: None,
            }),
            original_path: None,
            notes: Vec::new(),
        }
    }
}

/// Test ingestion result
#[derive(Debug, Clone)]
pub struct IngestedTest {
    /// Path to the test file
    pub path: PathBuf,
    /// Test metadata
    pub metadata: TestMetadata,
}

impl IngestedTest {
    /// Create a new ingested test
    pub fn new(path: PathBuf, metadata: TestMetadata) -> Self {
        Self { path, metadata }
    }
}

/// Test ingestion report
#[derive(Debug, Clone, Default)]
pub struct IngestionReport {
    /// Total tests ingested
    pub total: usize,
    /// Supported tests
    pub supported: usize,
    /// Deferred tests
    pub deferred: usize,
    /// Unsupported tests
    pub unsupported: usize,
    /// Tests by provenance
    pub by_provenance: std::collections::HashMap<TestProvenance, usize>,
}

impl IngestionReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an ingested test
    pub fn record(&mut self, test: &IngestedTest) {
        self.total += 1;
        match test.metadata.status {
            TestStatus::Supported => self.supported += 1,
            TestStatus::Deferred => self.deferred += 1,
            TestStatus::Unsupported => self.unsupported += 1,
        }
        *self
            .by_provenance
            .entry(test.metadata.provenance)
            .or_insert(0) += 1;
    }

    /// Get the coverage percentage
    pub fn coverage_percent(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        (self.supported as f64 / self.total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_dir_names() {
        assert_eq!(TestProvenance::Official.dir_name(), "official");
        assert_eq!(TestProvenance::Community.dir_name(), "community");
        assert_eq!(TestProvenance::Custom.dir_name(), "custom");
        assert_eq!(TestProvenance::ThirdParty.dir_name(), "third_party");
    }

    #[test]
    fn test_status_dir_names() {
        assert_eq!(TestStatus::Supported.dir_name(), "supported");
        assert_eq!(TestStatus::Deferred.dir_name(), "deferred");
        assert_eq!(TestStatus::Unsupported.dir_name(), "unsupported");
    }

    #[test]
    fn test_metadata_custom() {
        let meta = TestMetadata::custom();
        assert_eq!(meta.provenance, TestProvenance::Custom);
        assert_eq!(meta.status, TestStatus::Supported);
        assert!(meta.deferral_reason.is_none());
    }

    #[test]
    fn test_metadata_official_supported() {
        let meta = TestMetadata::official(TestStatus::Supported, None);
        assert_eq!(meta.provenance, TestProvenance::Official);
        assert_eq!(meta.status, TestStatus::Supported);
    }

    #[test]
    fn test_metadata_deferred() {
        let meta = TestMetadata::deferred(
            DeferralCategory::FeatureNotImplemented,
            "Async/await not yet supported".to_string(),
            Some("issue #42".to_string()),
        );
        assert_eq!(meta.provenance, TestProvenance::Official);
        assert_eq!(meta.status, TestStatus::Deferred);
        assert!(meta.deferral_reason.is_some());
        let reason = meta.deferral_reason.unwrap();
        assert_eq!(reason.category, DeferralCategory::FeatureNotImplemented);
        assert_eq!(reason.tracking_ref, Some("issue #42".to_string()));
    }

    #[test]
    fn test_metadata_unsupported() {
        let meta = TestMetadata::unsupported(
            DeferralCategory::BackendLimitation,
            "JavaScript backend does not support FFI".to_string(),
        );
        assert_eq!(meta.status, TestStatus::Unsupported);
        assert!(meta.deferral_reason.is_some());
    }

    #[test]
    fn test_ingestion_report() {
        let mut report = IngestionReport::new();

        let test1 = IngestedTest::new(PathBuf::from("test1.nim"), TestMetadata::custom());
        let test2 = IngestedTest::new(
            PathBuf::from("test2.nim"),
            TestMetadata::official(TestStatus::Supported, None),
        );
        let test3 = IngestedTest::new(
            PathBuf::from("test3.nim"),
            TestMetadata::deferred(
                DeferralCategory::KnownBug,
                "Bug in overload resolution".to_string(),
                None,
            ),
        );

        report.record(&test1);
        report.record(&test2);
        report.record(&test3);

        assert_eq!(report.total, 3);
        assert_eq!(report.supported, 2);
        assert_eq!(report.deferred, 1);
        assert_eq!(report.unsupported, 0);
        assert_eq!(report.coverage_percent(), 66.66666666666666);
    }

    #[test]
    fn test_deferral_categories() {
        assert_eq!(
            format!("{:?}", DeferralCategory::FeatureNotImplemented),
            "FeatureNotImplemented"
        );
        assert_eq!(
            format!("{:?}", DeferralCategory::BackendLimitation),
            "BackendLimitation"
        );
    }
}
