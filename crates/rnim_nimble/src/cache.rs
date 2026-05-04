//! Package lockfile and cache management.
//!
//! This module provides deterministic package cache layout, lockfile
//! schema, source hashing, offline mode support, and path override
//! capabilities.

use crate::{NimbleError, NimbleManifest, PackageGraph, ResolvedPackage};
use rnim_span::Span;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Lockfile schema version
pub const LOCKFILE_VERSION: &str = "1.0";

/// A lockfile entry for a single package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileEntry {
    /// Package name
    pub name: String,
    /// Resolved version
    pub version: String,
    /// Resolved source URL or path
    pub url: Option<String>,
    /// SHA256 hash of the package source
    pub hash: String,
    /// Checksum of the nimble file
    pub nimble_hash: String,
    /// Dependencies (resolved versions)
    pub dependencies: Vec<LockfileDep>,
    /// When this entry was locked
    pub timestamp: u64,
    /// Original requirement that led to this resolution
    pub requirement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileDep {
    pub name: String,
    pub version: String,
    pub url: Option<String>,
}

/// A complete lockfile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: String,
    pub packages: HashMap<String, LockfileEntry>,
    pub metadata: LockfileMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileMetadata {
    pub created_at: u64,
    pub updated_at: u64,
    pub nim_version: String,
    pub cache_path: PathBuf,
}

impl Lockfile {
    pub fn new(nim_version: &str, cache_path: PathBuf) -> Self {
        let now = current_timestamp();
        Lockfile {
            version: LOCKFILE_VERSION.to_string(),
            packages: HashMap::new(),
            metadata: LockfileMetadata {
                created_at: now,
                updated_at: now,
                nim_version: nim_version.to_string(),
                cache_path,
            },
        }
    }

    pub fn add_package(&mut self, entry: LockfileEntry) {
        let key = format!("{}#{}", entry.name, entry.version);
        self.packages.insert(key, entry);
        self.metadata.updated_at = current_timestamp();
    }

    pub fn get_package(&self, name: &str, version: &str) -> Option<&LockfileEntry> {
        let key = format!("{}#{}", name, version);
        self.packages.get(&key)
    }

    pub fn remove_package(&mut self, name: &str, version: &str) -> Option<LockfileEntry> {
        let key = format!("{}#{}", name, version);
        self.metadata.updated_at = current_timestamp();
        self.packages.remove(&key)
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn package_keys(&self) -> Vec<String> {
        self.packages.keys().cloned().collect()
    }
}

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub path: PathBuf,
    pub manifest_hash: String,
    pub source_hash: String,
    pub created_at: u64,
    pub last_accessed: u64,
    pub size_bytes: u64,
}

/// A cache key based on package identity and version
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub name: String,
    pub version: String,
    pub source_hash: String,
}

impl CacheKey {
    pub fn new(name: &str, version: &str, source_hash: &str) -> Self {
        CacheKey {
            name: name.to_string(),
            version: version.to_string(),
            source_hash: source_hash.to_string(),
        }
    }

    pub fn cache_path(&self, base: &Path) -> PathBuf {
        // Deterministic cache layout: cache/name/version/hash/
        base.join(&self.name)
            .join(&self.version)
            .join(&self.source_hash[..8].to_string())
    }
}

/// Package cache manager
#[derive(Debug, Clone)]
pub struct PackageCache {
    pub root: PathBuf,
    pub entries: HashMap<CacheKey, CacheEntry>,
    pub offline: bool,
    pub overrides: HashMap<String, PathBuf>,
}

impl PackageCache {
    pub fn new(root: PathBuf) -> Self {
        PackageCache {
            root,
            entries: HashMap::new(),
            offline: false,
            overrides: HashMap::new(),
        }
    }

    pub fn with_offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    pub fn with_override(mut self, name: &str, path: PathBuf) -> Self {
        self.overrides.insert(name.to_string(), path);
        self
    }

    /// Get the cache path for a package
    pub fn get_cache_path(&self, key: &CacheKey) -> PathBuf {
        if let Some(override_path) = self.overrides.get(&key.name) {
            return override_path.clone();
        }
        key.cache_path(&self.root)
    }

    /// Check if a package is cached
    pub fn is_cached(&self, key: &CacheKey) -> bool {
        let path = self.get_cache_path(key);
        path.exists() && path.is_dir()
    }

    /// Record cache access
    pub fn record_access(&mut self, key: &CacheKey) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed = current_timestamp();
        }
    }

    /// Add an override for a package name
    pub fn add_override(&mut self, name: &str, path: PathBuf) {
        self.overrides.insert(name.to_string(), path);
    }

    /// Remove an override
    pub fn remove_override(&mut self, name: &str) -> Option<PathBuf> {
        self.overrides.remove(name)
    }

    /// Get all overrides
    pub fn overrides(&self) -> &HashMap<String, PathBuf> {
        &self.overrides
    }

    /// Set offline mode
    pub fn set_offline(&mut self, offline: bool) {
        self.offline = offline;
    }

    /// Check if in offline mode
    pub fn is_offline(&self) -> bool {
        self.offline
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get cache size in bytes (estimated from entries)
    pub fn estimated_size(&self) -> u64 {
        self.entries.values().map(|e| e.size_bytes).sum()
    }

    /// Clean up old entries (simple LRU-ish cleanup)
    pub fn cleanup(&mut self, max_entries: usize) {
        if self.entries.len() <= max_entries {
            return;
        }

        // Collect keys to remove (oldest entries by last_accessed)
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by_key(|e| e.1.last_accessed);

        let keys_to_remove: Vec<_> = entries
            .into_iter()
            .take(self.entries.len() - max_entries)
            .map(|(k, _)| k.clone())
            .collect();

        for key in keys_to_remove {
            self.entries.remove(&key);
        }
    }
}

// Note: compute_source_hash requires sha2 crate - uncomment when needed
// pub fn compute_source_hash(path: &Path) -> Result<String, NimbleError> { ... }

/// Hash computation for nimble files
pub fn compute_nimble_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Verify a package against its lockfile entry
pub fn verify_package(path: &Path, entry: &LockfileEntry) -> Result<bool, NimbleError> {
    let nimble_path = path.join(format!("{}.nimble", entry.name));
    let computed_nimble_hash = if nimble_path.exists() {
        let content = std::fs::read_to_string(&nimble_path).map_err(|e| {
            NimbleError::ParseError(
                format!("failed to read nimble file: {}", e),
                Span::new(rnim_span::FileId(0), 0, 0),
            )
        })?;
        compute_nimble_hash(&content)
    } else {
        return Ok(false);
    };

    Ok(computed_nimble_hash == entry.nimble_hash)
}

/// Create a lockfile entry from a resolved package
pub fn create_lockfile_entry(
    pkg: &ResolvedPackage,
    source_hash: &str,
    requirement: &str,
) -> LockfileEntry {
    LockfileEntry {
        name: pkg.manifest.name.clone(),
        version: pkg.manifest.version.clone(),
        url: None,
        hash: source_hash.to_string(),
        nimble_hash: String::new(),
        dependencies: pkg
            .dependencies
            .iter()
            .map(|dep| LockfileDep {
                name: dep.manifest.name.clone(),
                version: dep.manifest.version.clone(),
                url: None,
            })
            .collect(),
        timestamp: current_timestamp(),
        requirement: requirement.to_string(),
    }
}

/// Generate deterministic cache key from package info
pub fn generate_cache_key(manifest: &NimbleManifest, source_hash: &str) -> CacheKey {
    CacheKey::new(&manifest.name, &manifest.version, source_hash)
}

/// Build a lockfile from a package graph
pub fn build_lockfile(graph: &PackageGraph, nim_version: &str, cache_path: PathBuf) -> Lockfile {
    let mut lockfile = Lockfile::new(nim_version, cache_path);

    for (_name, pkg) in graph.packages() {
        let entry = create_lockfile_entry(pkg, "", "");
        let key = format!("{}#{}", entry.name, entry.version);
        lockfile.packages.insert(key, entry);
    }

    lockfile.metadata.updated_at = current_timestamp();
    lockfile
}

/// Read a lockfile from disk
pub fn read_lockfile(path: &Path) -> Result<Lockfile, NimbleError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        NimbleError::ParseError(
            format!("failed to read lockfile: {}", e),
            Span::new(rnim_span::FileId(0), 0, 0),
        )
    })?;

    toml::from_str(&content).map_err(|e| {
        NimbleError::ParseError(
            format!("failed to parse lockfile: {}", e),
            Span::new(rnim_span::FileId(0), 0, 0),
        )
    })
}

/// Write a lockfile to disk
pub fn write_lockfile(lockfile: &Lockfile, path: &Path) -> Result<(), NimbleError> {
    let content = toml::to_string_pretty(lockfile).map_err(|e| {
        NimbleError::ParseError(
            format!("failed to serialize lockfile: {}", e),
            Span::new(rnim_span::FileId(0), 0, 0),
        )
    })?;

    std::fs::write(path, content).map_err(|e| {
        NimbleError::ParseError(
            format!("failed to write lockfile: {}", e),
            Span::new(rnim_span::FileId(0), 0, 0),
        )
    })
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NimbleManifest, ResolvedPackage};
    use std::path::PathBuf;

    #[test]
    fn test_lockfile_new() {
        let lockfile = Lockfile::new("1.0.0", PathBuf::from("/cache"));
        assert_eq!(lockfile.version, "1.0");
        assert!(lockfile.packages.is_empty());
        assert_eq!(lockfile.metadata.nim_version, "1.0.0");
    }

    #[test]
    fn test_lockfile_add_package() {
        let mut lockfile = Lockfile::new("1.0.0", PathBuf::from("/cache"));
        let entry = LockfileEntry {
            name: "testpkg".to_string(),
            version: "1.0.0".to_string(),
            url: None,
            hash: "abc123".to_string(),
            nimble_hash: "def456".to_string(),
            dependencies: vec![],
            timestamp: current_timestamp(),
            requirement: "testpkg >= 1.0".to_string(),
        };
        lockfile.add_package(entry);
        assert_eq!(lockfile.packages.len(), 1);
        assert!(lockfile.get_package("testpkg", "1.0.0").is_some());
    }

    #[test]
    fn test_lockfile_get_package() {
        let mut lockfile = Lockfile::new("1.0.0", PathBuf::from("/cache"));
        let entry = LockfileEntry {
            name: "testpkg".to_string(),
            version: "1.0.0".to_string(),
            url: None,
            hash: "abc123".to_string(),
            nimble_hash: "def456".to_string(),
            dependencies: vec![],
            timestamp: current_timestamp(),
            requirement: "testpkg >= 1.0".to_string(),
        };
        lockfile.add_package(entry);
        assert!(lockfile.get_package("testpkg", "1.0.0").is_some());
        assert!(lockfile.get_package("nonexistent", "1.0.0").is_none());
    }

    #[test]
    fn test_lockfile_remove_package() {
        let mut lockfile = Lockfile::new("1.0.0", PathBuf::from("/cache"));
        let entry = LockfileEntry {
            name: "testpkg".to_string(),
            version: "1.0.0".to_string(),
            url: None,
            hash: "abc123".to_string(),
            nimble_hash: "def456".to_string(),
            dependencies: vec![],
            timestamp: current_timestamp(),
            requirement: "testpkg >= 1.0".to_string(),
        };
        lockfile.add_package(entry);
        let removed = lockfile.remove_package("testpkg", "1.0.0");
        assert!(removed.is_some());
        assert!(lockfile.get_package("testpkg", "1.0.0").is_none());
    }

    #[test]
    fn test_cache_key_new() {
        let key = CacheKey::new("testpkg", "1.0.0", "hash123");
        assert_eq!(key.name, "testpkg");
        assert_eq!(key.version, "1.0.0");
        assert_eq!(key.source_hash, "hash123");
    }

    #[test]
    fn test_cache_key_cache_path() {
        let key = CacheKey::new("testpkg", "1.0.0", "abc123def");
        let path = key.cache_path(Path::new("/cache"));
        assert_eq!(path, Path::new("/cache/testpkg/1.0.0/abc123de"));
    }

    #[test]
    fn test_package_cache_new() {
        let cache = PackageCache::new(PathBuf::from("/cache"));
        assert_eq!(cache.root, PathBuf::from("/cache"));
        assert!(cache.entries.is_empty());
        assert!(!cache.offline);
    }

    #[test]
    fn test_package_cache_with_offline() {
        let cache = PackageCache::new(PathBuf::from("/cache")).with_offline(true);
        assert!(cache.offline);
    }

    #[test]
    fn test_package_cache_with_override() {
        let cache = PackageCache::new(PathBuf::from("/cache"))
            .with_override("testpkg", PathBuf::from("/override"));
        assert_eq!(
            cache.overrides.get("testpkg"),
            Some(&PathBuf::from("/override"))
        );
    }

    #[test]
    fn test_package_cache_get_cache_path_override() {
        let mut cache = PackageCache::new(PathBuf::from("/cache"));
        cache.add_override("testpkg", PathBuf::from("/override"));
        let key = CacheKey::new("testpkg", "1.0.0", "hash123");
        let path = cache.get_cache_path(&key);
        assert_eq!(path, PathBuf::from("/override"));
    }

    #[test]
    fn test_package_cache_add_remove_override() {
        let mut cache = PackageCache::new(PathBuf::from("/cache"));
        cache.add_override("testpkg", PathBuf::from("/override"));
        assert!(cache.overrides.contains_key("testpkg"));
        let removed = cache.remove_override("testpkg");
        assert!(removed.is_some());
        assert!(!cache.overrides.contains_key("testpkg"));
    }

    #[test]
    fn test_package_cache_offline_mode() {
        let mut cache = PackageCache::new(PathBuf::from("/cache"));
        cache.set_offline(true);
        assert!(cache.is_offline());
        cache.set_offline(false);
        assert!(!cache.is_offline());
    }

    #[test]
    fn test_package_cache_cleanup() {
        let mut cache = PackageCache::new(PathBuf::from("/cache"));
        for i in 0..10 {
            let key = CacheKey::new(&format!("pkg{}", i), "1.0.0", "hash");
            let entry = CacheEntry {
                key,
                path: PathBuf::from(format!("/cache/{}", i)),
                manifest_hash: "hash".to_string(),
                source_hash: "hash".to_string(),
                created_at: i as u64,
                last_accessed: i as u64,
                size_bytes: 100,
            };
            cache.entries.insert(entry.key.clone(), entry);
        }
        cache.cleanup(5);
        assert!(cache.entries.len() <= 5);
    }

    #[test]
    fn test_compute_nimble_hash() {
        let hash1 = compute_nimble_hash("name = \"test\"\nversion = \"1.0\"");
        let hash2 = compute_nimble_hash("name = \"test\"\nversion = \"1.0\"");
        let hash3 = compute_nimble_hash("name = \"other\"\nversion = \"1.0\"");
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_generate_cache_key() {
        let manifest = NimbleManifest::new(
            "testpkg",
            "1.0.0",
            rnim_span::Span::new(rnim_span::FileId(0), 0, 0),
        );
        let key = generate_cache_key(&manifest, "hash123");
        assert_eq!(key.name, "testpkg");
        assert_eq!(key.version, "1.0.0");
        assert_eq!(key.source_hash, "hash123");
    }

    #[test]
    fn test_build_lockfile() {
        let mut graph = PackageGraph::new();
        let manifest = NimbleManifest::new(
            "testpkg",
            "1.0.0",
            rnim_span::Span::new(rnim_span::FileId(0), 0, 0),
        );
        let pkg = ResolvedPackage::new(manifest, PathBuf::from("/path/to/pkg"));
        graph.add_package(pkg).unwrap();

        let lockfile = build_lockfile(&graph, "1.0.0", PathBuf::from("/cache"));
        assert!(!lockfile.is_empty());
        assert!(lockfile.get_package("testpkg", "1.0.0").is_some());
    }

    #[test]
    fn test_lockfile_serialization() {
        let mut lockfile = Lockfile::new("1.0.0", PathBuf::from("/cache"));
        let entry = LockfileEntry {
            name: "testpkg".to_string(),
            version: "1.0.0".to_string(),
            url: None,
            hash: "abc123".to_string(),
            nimble_hash: "def456".to_string(),
            dependencies: vec![],
            timestamp: current_timestamp(),
            requirement: "testpkg >= 1.0".to_string(),
        };
        lockfile.add_package(entry);

        let serialized = toml::to_string(&lockfile).unwrap();
        let deserialized: Lockfile = toml::from_str(&serialized).unwrap();
        assert!(deserialized.get_package("testpkg", "1.0.0").is_some());
    }
}
