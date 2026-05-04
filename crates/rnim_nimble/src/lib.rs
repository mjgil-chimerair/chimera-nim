//! Nimble package manager integration.
//!
//! This module provides parsing and resolution for `.nimble` package files.

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{FileId, Span};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// A Nimble package manifest
#[derive(Debug, Clone, PartialEq)]
pub struct NimbleManifest {
    /// Package name
    pub name: String,
    /// Version string
    pub version: String,
    /// Author information
    pub author: Option<String>,
    /// Description
    pub description: Option<String>,
    /// License
    pub license: Option<String>,
    /// Package skip file
    pub skip_file: Option<String>,
    /// Skip dirs
    pub skip_dirs: Vec<String>,
    /// Install files
    pub install_files: Vec<String>,
    /// Install dirs
    pub install_dirs: Vec<String>,
    /// Requires dependencies
    pub requires: Vec<NimbleRequirement>,
    /// Development dependencies
    pub dev_depends: Vec<NimbleRequirement>,
    /// Bin section
    pub bin: Vec<BinSpec>,
    /// Binary name (alias for bin)
    pub bin_name: Option<String>,
    /// Backend
    pub backend: Option<String>,
    /// Flags
    pub flags: Vec<String>,
    /// Span for error reporting
    pub span: Span,
}

impl NimbleManifest {
    /// Create a new manifest
    pub fn new(name: &str, version: &str, span: Span) -> Self {
        NimbleManifest {
            name: name.to_string(),
            version: version.to_string(),
            author: None,
            description: None,
            license: None,
            skip_file: None,
            skip_dirs: Vec::new(),
            install_files: Vec::new(),
            install_dirs: Vec::new(),
            requires: Vec::new(),
            dev_depends: Vec::new(),
            bin: Vec::new(),
            bin_name: None,
            backend: None,
            flags: Vec::new(),
            span,
        }
    }

    /// Get the package ID
    pub fn package_id(&self) -> String {
        format!("{}#{}", self.name, self.version)
    }
}

/// A requirement specification
#[derive(Debug, Clone, PartialEq)]
pub struct NimbleRequirement {
    pub name: String,
    pub version_spec: Option<VersionSpec>,
    pub url: Option<String>,
}

impl NimbleRequirement {
    pub fn new(name: &str) -> Self {
        NimbleRequirement {
            name: name.to_string(),
            version_spec: None,
            url: None,
        }
    }

    pub fn with_version(mut self, spec: VersionSpec) -> Self {
        self.version_spec = Some(spec);
        self
    }

    pub fn with_url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }
}

/// Version specification
#[derive(Debug, Clone, PartialEq)]
pub enum VersionSpec {
    Exact(String),
    Minimum(String),
    Range(String, String),
    Any,
}

impl VersionSpec {
    pub fn matches(&self, version: &str) -> bool {
        match self {
            VersionSpec::Exact(v) => version == v,
            VersionSpec::Minimum(v) => version >= v.as_str(),
            VersionSpec::Range(min, max) => version >= min.as_str() && version <= max.as_str(),
            VersionSpec::Any => true,
        }
    }
}

/// Binary specification
#[derive(Debug, Clone, PartialEq)]
pub struct BinSpec {
    pub name: String,
    pub file: String,
    pub options: Vec<String>,
}

impl BinSpec {
    pub fn new(name: &str, file: &str) -> Self {
        BinSpec {
            name: name.to_string(),
            file: file.to_string(),
            options: Vec::new(),
        }
    }
}

/// Parse errors
#[derive(Debug, Clone, PartialEq)]
pub enum NimbleError {
    InvalidHeader(String, Span),
    InvalidField(String, Span),
    MissingRequired(String, Span),
    InvalidVersion(String, Span),
    ParseError(String, Span),
}

impl std::fmt::Display for NimbleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NimbleError::InvalidHeader(s, _) => write!(f, "invalid header: {}", s),
            NimbleError::InvalidField(s, _) => write!(f, "invalid field: {}", s),
            NimbleError::MissingRequired(s, _) => write!(f, "missing required field: {}", s),
            NimbleError::InvalidVersion(s, _) => write!(f, "invalid version: {}", s),
            NimbleError::ParseError(s, _) => write!(f, "parse error: {}", s),
        }
    }
}

/// Nimble parser
#[derive(Debug, Clone, Default)]
pub struct NimbleParser {
    /// Current package dir
    package_dir: Option<PathBuf>,
}

impl NimbleParser {
    pub fn new() -> Self {
        NimbleParser { package_dir: None }
    }

    pub fn with_package_dir(mut self, dir: PathBuf) -> Self {
        self.package_dir = Some(dir);
        self
    }

    /// Parse a nimble file from content
    pub fn parse(&self, content: &str, span: Span) -> Result<NimbleManifest, NimbleError> {
        let mut manifest: Option<NimbleManifest> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check for header
            if trimmed.starts_with('[') {
                if let Some(end) = trimmed.find(']') {
                    let header = &trimmed[1..end];
                    if header == "Package" {
                        // Package header found, continue parsing
                    }
                }
                continue;
            }

            // Parse key = value
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim();
                let value = trimmed[eq_pos + 1..].trim();

                // Remove quotes from value
                let value = value.trim_matches('"').trim_matches('\'');

                if key == "name" && manifest.is_none() {
                    manifest = Some(NimbleManifest::new(value, "0.0.0", span));
                } else if let Some(ref mut m) = manifest {
                    self.set_field(m, key, value)?;
                }
            }
        }

        manifest.ok_or_else(|| NimbleError::ParseError("missing package header".to_string(), span))
    }

    fn set_field(
        &self,
        manifest: &mut NimbleManifest,
        key: &str,
        value: &str,
    ) -> Result<(), NimbleError> {
        match key {
            "name" => manifest.name = value.to_string(),
            "version" => manifest.version = value.to_string(),
            "author" => manifest.author = Some(value.to_string()),
            "description" => manifest.description = Some(value.to_string()),
            "license" => manifest.license = Some(value.to_string()),
            "skipFile" => manifest.skip_file = Some(value.to_string()),
            "skipDirs" => {
                manifest.skip_dirs = value.split(',').map(|s| s.trim().to_string()).collect()
            }
            "installFiles" => {
                manifest.install_files = value.split(',').map(|s| s.trim().to_string()).collect()
            }
            "installDirs" => {
                manifest.install_dirs = value.split(',').map(|s| s.trim().to_string()).collect()
            }
            "requires" => {
                for req in self.parse_requires(value) {
                    manifest.requires.push(req);
                }
            }
            "bin" => {
                for spec in self.parse_bin(value) {
                    manifest.bin.push(spec);
                }
            }
            "binName" => manifest.bin_name = Some(value.to_string()),
            "backend" => manifest.backend = Some(value.to_string()),
            "flags" => manifest.flags = value.split_whitespace().map(|s| s.to_string()).collect(),
            _ => {}
        }
        Ok(())
    }

    fn parse_requires(&self, value: &str) -> Vec<NimbleRequirement> {
        let mut reqs = Vec::new();
        for part in value.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let parts: Vec<&str> = part.split_whitespace().collect();
            if let Some(name) = parts.first() {
                let mut req = NimbleRequirement::new(name);
                if parts.len() > 1 {
                    let version = parts[1];
                    if version == "any" {
                        req.version_spec = Some(VersionSpec::Any);
                    } else {
                        req.version_spec = Some(VersionSpec::Minimum(version.to_string()));
                    }
                }
                reqs.push(req);
            }
        }
        reqs
    }

    fn parse_bin(&self, value: &str) -> Vec<BinSpec> {
        let mut bins = Vec::new();
        for part in value.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let parts: Vec<&str> = part.split_whitespace().collect();
            if let Some(file) = parts.first() {
                let name = if parts.len() > 1 {
                    parts[1].to_string()
                } else {
                    std::path::Path::new(file)
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| file.to_string())
                };
                bins.push(BinSpec::new(&name, file));
            }
        }
        bins
    }

    /// Get the package directory
    pub fn package_dir(&self) -> Option<&PathBuf> {
        self.package_dir.as_ref()
    }
}

/// Package resolution result
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub manifest: NimbleManifest,
    pub path: PathBuf,
    pub dependencies: Vec<ResolvedPackage>,
}

impl ResolvedPackage {
    pub fn new(manifest: NimbleManifest, path: PathBuf) -> Self {
        ResolvedPackage {
            manifest,
            path,
            dependencies: Vec::new(),
        }
    }
}

/// Package graph for dependency resolution
#[derive(Debug, Clone, Default)]
pub struct PackageGraph {
    /// Packages by name
    packages: HashMap<String, ResolvedPackage>,
    /// Packages by path
    packages_by_path: HashMap<PathBuf, String>,
}

impl PackageGraph {
    pub fn new() -> Self {
        PackageGraph {
            packages: HashMap::new(),
            packages_by_path: HashMap::new(),
        }
    }

    /// Add a package to the graph
    pub fn add_package(&mut self, package: ResolvedPackage) -> Result<(), NimbleError> {
        let name = package.manifest.name.clone();
        if self.packages.contains_key(&name) {
            return Err(NimbleError::InvalidField(
                format!("duplicate package: {}", name),
                package.manifest.span,
            ));
        }
        self.packages.insert(name.clone(), package);
        if let Some(pkg) = self.packages.get(&name) {
            self.packages_by_path.insert(pkg.path.clone(), name);
        }
        Ok(())
    }

    /// Get a package by name
    pub fn get(&self, name: &str) -> Option<&ResolvedPackage> {
        self.packages.get(name)
    }

    /// Get a package by path
    pub fn get_by_path(&self, path: &PathBuf) -> Option<&ResolvedPackage> {
        self.packages_by_path
            .get(path)
            .and_then(|n| self.packages.get(n))
    }

    /// Get all packages
    pub fn packages(&self) -> &HashMap<String, ResolvedPackage> {
        &self.packages
    }

    /// Check for cycles in dependencies
    pub fn has_cycles(&self) -> bool {
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();

        for name in self.packages.keys() {
            if self.detect_cycle(name, &mut visited, &mut in_stack) {
                return true;
            }
        }
        false
    }

    fn detect_cycle(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
    ) -> bool {
        if in_stack.contains(name) {
            return true;
        }
        if visited.contains(name) {
            return false;
        }

        visited.insert(name.to_string());
        in_stack.insert(name.to_string());

        if let Some(pkg) = self.packages.get(name) {
            for dep in &pkg.dependencies {
                if self.detect_cycle(&dep.manifest.name, visited, in_stack) {
                    return true;
                }
            }
        }

        in_stack.remove(name);
        false
    }

    /// Topological sort of packages
    pub fn topological_sort(&self) -> Result<Vec<String>, NimbleError> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp = HashSet::new();

        for name in self.packages.keys() {
            if self.tsort_visit(name, &mut visited, &mut temp, &mut result) {
                return Err(NimbleError::ParseError(
                    "dependency cycle detected".to_string(),
                    Span::new(FileId(0), 0, 0),
                ));
            }
        }

        Ok(result)
    }

    fn tsort_visit(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        temp: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> bool {
        if temp.contains(name) {
            return true; // Cycle detected
        }
        if visited.contains(name) {
            return false;
        }

        temp.insert(name.to_string());

        if let Some(pkg) = self.packages.get(name) {
            for dep in &pkg.dependencies {
                if self.tsort_visit(&dep.manifest.name, visited, temp, result) {
                    return true;
                }
            }
        }

        temp.remove(name);
        visited.insert(name.to_string());
        result.push(name.to_string());
        false
    }
}

mod cache;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nimble_parser_new() {
        let parser = NimbleParser::new();
        assert!(parser.package_dir.is_none());
    }

    #[test]
    fn test_nimble_parser_with_package_dir() {
        let parser = NimbleParser::new().with_package_dir(PathBuf::from("/path/to/pkg"));
        assert!(parser.package_dir.is_some());
    }

    #[test]
    fn test_nimble_manifest_new() {
        let manifest = NimbleManifest::new("mypackage", "1.0.0", Span::new(FileId(0), 0, 0));
        assert_eq!(manifest.name, "mypackage");
        assert_eq!(manifest.version, "1.0.0");
    }

    #[test]
    fn test_nimble_manifest_package_id() {
        let manifest = NimbleManifest::new("mypackage", "1.0.0", Span::new(FileId(0), 0, 0));
        assert_eq!(manifest.package_id(), "mypackage#1.0.0");
    }

    #[test]
    fn test_parse_simple_manifest() {
        let parser = NimbleParser::new();
        let content = r#"
[Package]
name = "testpkg"
version = "0.1.0"
"#;
        let result = parser.parse(content, Span::new(FileId(0), 0, 0));
        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.name, "testpkg");
        assert_eq!(manifest.version, "0.1.0");
    }

    #[test]
    fn test_parse_manifest_with_deps() {
        let parser = NimbleParser::new();
        let content = r#"
[Package]
name = "testpkg"
version = "0.1.0"
requires = "nim > 1.0"
"#;
        let result = parser.parse(content, Span::new(FileId(0), 0, 0));
        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.requires.len(), 1);
        assert_eq!(manifest.requires[0].name, "nim");
    }

    #[test]
    fn test_parse_manifest_with_bin() {
        let parser = NimbleParser::new();
        let content = r#"
[Package]
name = "testpkg"
version = "0.1.0"
bin = "src/main.nim"
"#;
        let result = parser.parse(content, Span::new(FileId(0), 0, 0));
        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.bin.len(), 1);
        assert_eq!(manifest.bin[0].file, "src/main.nim");
    }

    #[test]
    fn test_nimble_requirement_new() {
        let req = NimbleRequirement::new("nim");
        assert_eq!(req.name, "nim");
        assert!(req.version_spec.is_none());
        assert!(req.url.is_none());
    }

    #[test]
    fn test_nimble_requirement_with_version() {
        let req =
            NimbleRequirement::new("nim").with_version(VersionSpec::Minimum("1.0".to_string()));
        assert_eq!(req.name, "nim");
        assert!(req.version_spec.is_some());
    }

    #[test]
    fn test_version_spec_exact() {
        let spec = VersionSpec::Exact("1.0.0".to_string());
        assert!(spec.matches("1.0.0"));
        assert!(!spec.matches("1.0.1"));
    }

    #[test]
    fn test_version_spec_minimum() {
        let spec = VersionSpec::Minimum("1.0.0".to_string());
        assert!(spec.matches("1.0.0"));
        assert!(spec.matches("1.0.1"));
        assert!(!spec.matches("0.9.0"));
    }

    #[test]
    fn test_version_spec_range() {
        let spec = VersionSpec::Range("1.0.0".to_string(), "2.0.0".to_string());
        assert!(spec.matches("1.0.0"));
        assert!(spec.matches("1.5.0"));
        assert!(!spec.matches("0.9.0"));
        assert!(!spec.matches("2.0.1"));
    }

    #[test]
    fn test_version_spec_any() {
        let spec = VersionSpec::Any;
        assert!(spec.matches("0.0.1"));
        assert!(spec.matches("999.999.999"));
    }

    #[test]
    fn test_bin_spec_new() {
        let spec = BinSpec::new("mybin", "src/main.nim");
        assert_eq!(spec.name, "mybin");
        assert_eq!(spec.file, "src/main.nim");
    }

    #[test]
    fn test_package_graph_new() {
        let graph = PackageGraph::new();
        assert!(graph.packages.is_empty());
    }

    #[test]
    fn test_package_graph_add_package() {
        let mut graph = PackageGraph::new();
        let manifest = NimbleManifest::new("pkg1", "1.0.0", Span::new(FileId(0), 0, 0));
        let pkg = ResolvedPackage::new(manifest, PathBuf::from("/path/to/pkg1"));
        let result = graph.add_package(pkg);
        assert!(result.is_ok());
        assert_eq!(graph.packages.len(), 1);
    }

    #[test]
    fn test_package_graph_get() {
        let mut graph = PackageGraph::new();
        let manifest = NimbleManifest::new("pkg1", "1.0.0", Span::new(FileId(0), 0, 0));
        let pkg = ResolvedPackage::new(manifest, PathBuf::from("/path/to/pkg1"));
        graph.add_package(pkg).unwrap();
        assert!(graph.get("pkg1").is_some());
        assert!(graph.get("nonexistent").is_none());
    }

    #[test]
    fn test_package_graph_no_cycles() {
        let mut graph = PackageGraph::new();
        let manifest1 = NimbleManifest::new("pkg1", "1.0.0", Span::new(FileId(0), 0, 0));
        let pkg1 = ResolvedPackage::new(manifest1, PathBuf::from("/path/to/pkg1"));
        graph.add_package(pkg1).unwrap();
        assert!(!graph.has_cycles());
    }

    #[test]
    fn test_package_graph_topological_sort() {
        let mut graph = PackageGraph::new();
        let manifest1 = NimbleManifest::new("pkg1", "1.0.0", Span::new(FileId(0), 0, 0));
        let pkg1 = ResolvedPackage::new(manifest1, PathBuf::from("/path/to/pkg1"));
        graph.add_package(pkg1).unwrap();
        let sorted = graph.topological_sort();
        assert!(sorted.is_ok());
        assert_eq!(sorted.unwrap(), vec!["pkg1"]);
    }

    #[test]
    fn test_nimble_error_display() {
        let err = NimbleError::MissingRequired("name".to_string(), Span::new(FileId(0), 0, 0));
        let s = format!("{}", err);
        assert!(s.contains("missing required field"));
    }
}
