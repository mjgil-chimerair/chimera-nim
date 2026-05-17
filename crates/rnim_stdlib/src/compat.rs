//! Stdlib compatibility module.
//!
//! This module provides stdlib module status tracking, compatibility policy,
//! and module registry for the Rust Nim compiler.

#![allow(clippy::new_without_default)]

use std::collections::HashMap;

/// Stdlib module support status
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum ModuleStatus {
    /// Full support - module compiles and works correctly
    Supported,
    /// Partial support - module has known limitations or deviations
    Partial,
    /// Shim implementation - provides stubs/type definitions but no full implementation
    Shim,
    /// Not yet implemented - module exists but not yet ported
    NotImplemented,
    /// Not applicable for this backend
    NotApplicable,
    /// Explicitly not supported
    Unsupported,
}

/// Stdlib module information
#[derive(Debug, Clone)]
pub struct StdlibModule {
    pub name: String,
    pub status: ModuleStatus,
    pub category: ModuleCategory,
    pub dependencies: Vec<String>,
    pub backends: Vec<Backend>,
    pub limitations: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum ModuleCategory {
    Core,
    Text,
    Collections,
    OS,
    Time,
    Testing,
    Docs,
    Threading,
    Math,
    Net,
    Database,
    GUI,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Backend {
    C,
    Js,
    Native,
    All,
}

#[derive(Debug, Clone)]
pub struct CompatibilityPolicy {
    pub prefer_official_source: bool,
    pub compile_nim_source: bool,
    pub use_runtime_backed: bool,
    pub minimum_coverage: f32,
}

impl Default for CompatibilityPolicy {
    fn default() -> Self {
        CompatibilityPolicy {
            prefer_official_source: false,
            compile_nim_source: true,
            use_runtime_backed: true,
            minimum_coverage: 0.8,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StdlibRegistry {
    modules: HashMap<String, StdlibModule>,
    policy: CompatibilityPolicy,
}

impl StdlibRegistry {
    pub fn new() -> Self {
        let mut registry = StdlibRegistry {
            modules: HashMap::new(),
            policy: CompatibilityPolicy::default(),
        };
        registry.init_core_modules();
        registry
    }

    fn init_core_modules(&mut self) {
        self.register(StdlibModule {
            name: "system".to_string(),
            status: ModuleStatus::Supported,
            category: ModuleCategory::Core,
            dependencies: vec![],
            backends: vec![Backend::All],
            limitations: vec![],
            notes: "Core types, operators, assertions, basic IO".to_string(),
        });

        self.register(StdlibModule {
            name: "strutils".to_string(),
            status: ModuleStatus::Partial,
            category: ModuleCategory::Text,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::C, Backend::Js],
            limitations: vec!["Some format procs not fully compatible".to_string()],
            notes: "String utilities - partial implementation".to_string(),
        });

        self.register(StdlibModule {
            name: "math".to_string(),
            status: ModuleStatus::Supported,
            category: ModuleCategory::Math,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::All],
            limitations: vec![],
            notes: "Mathematical functions and constants".to_string(),
        });

        self.register(StdlibModule {
            name: "times".to_string(),
            status: ModuleStatus::Partial,
            category: ModuleCategory::Time,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::C],
            limitations: vec!["Timezone support limited".to_string()],
            notes: "Date and time utilities".to_string(),
        });

        self.register(StdlibModule {
            name: "os".to_string(),
            status: ModuleStatus::Partial,
            category: ModuleCategory::OS,
            dependencies: vec!["system".to_string(), "strutils".to_string()],
            backends: vec![Backend::C],
            limitations: vec!["Some path operations platform-specific".to_string()],
            notes: "Operating system interface".to_string(),
        });

        self.register(StdlibModule {
            name: "sequtils".to_string(),
            status: ModuleStatus::Supported,
            category: ModuleCategory::Collections,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::All],
            limitations: vec![],
            notes: "Sequence utilities".to_string(),
        });

        self.register(StdlibModule {
            name: "tables".to_string(),
            status: ModuleStatus::Partial,
            category: ModuleCategory::Collections,
            dependencies: vec!["system".to_string(), "hashes".to_string()],
            backends: vec![Backend::C, Backend::Js],
            limitations: vec!["Ordered table variant not complete".to_string()],
            notes: "Hash table implementation".to_string(),
        });

        self.register(StdlibModule {
            name: "sets".to_string(),
            status: ModuleStatus::Supported,
            category: ModuleCategory::Collections,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::All],
            limitations: vec![],
            notes: "Hash set implementation".to_string(),
        });

        self.register(StdlibModule {
            name: "hashes".to_string(),
            status: ModuleStatus::Supported,
            category: ModuleCategory::Core,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::All],
            limitations: vec![],
            notes: "Hash function support".to_string(),
        });

        self.register(StdlibModule {
            name: "typetraits".to_string(),
            status: ModuleStatus::Supported,
            category: ModuleCategory::Core,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::All],
            limitations: vec![],
            notes: "Type reflection utilities".to_string(),
        });

        self.register(StdlibModule {
            name: "macros".to_string(),
            status: ModuleStatus::Partial,
            category: ModuleCategory::Core,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::All],
            limitations: vec!["Some macro constructs not supported".to_string()],
            notes: "Macro system support".to_string(),
        });

        self.register(StdlibModule {
            name: "unittest".to_string(),
            status: ModuleStatus::Partial,
            category: ModuleCategory::Testing,
            dependencies: vec!["system".to_string(), "strutils".to_string()],
            backends: vec![Backend::C],
            limitations: vec!["Async tests not fully supported".to_string()],
            notes: "Unit testing framework".to_string(),
        });

        self.register(StdlibModule {
            name: "json".to_string(),
            status: ModuleStatus::Partial,
            category: ModuleCategory::Text,
            dependencies: vec!["system".to_string(), "strutils".to_string()],
            backends: vec![Backend::C, Backend::Js],
            limitations: vec!["JSON schema support limited".to_string()],
            notes: "JSON parsing and generation".to_string(),
        });

        self.register(StdlibModule {
            name: "streams".to_string(),
            status: ModuleStatus::Supported,
            category: ModuleCategory::Core,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::All],
            limitations: vec![],
            notes: "Stream I/O utilities".to_string(),
        });

        self.register(StdlibModule {
            name: "cpuinfo".to_string(),
            status: ModuleStatus::Shim,
            category: ModuleCategory::Core,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::C],
            limitations: vec!["Only basic CPU feature detection".to_string()],
            notes: "CPU feature detection - stub implementation".to_string(),
        });

        self.register(StdlibModule {
            name: "colors".to_string(),
            status: ModuleStatus::Shim,
            category: ModuleCategory::Other,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::All],
            limitations: vec!["Limited color space support".to_string()],
            notes: "Color utilities - stub implementation".to_string(),
        });

        self.register(StdlibModule {
            name: "async".to_string(),
            status: ModuleStatus::Partial,
            category: ModuleCategory::Threading,
            dependencies: vec!["system".to_string()],
            backends: vec![Backend::C],
            limitations: vec!["Only basic async operations supported".to_string()],
            notes: "Async primitives support".to_string(),
        });

        self.register(StdlibModule {
            name: "asyncdispatch".to_string(),
            status: ModuleStatus::Partial,
            category: ModuleCategory::Threading,
            dependencies: vec!["system".to_string(), "async".to_string()],
            backends: vec![Backend::C],
            limitations: vec!["Timer resolution limited".to_string()],
            notes: "Async dispatcher".to_string(),
        });
    }

    pub fn register(&mut self, module: StdlibModule) {
        self.modules.insert(module.name.clone(), module);
    }

    pub fn get(&self, name: &str) -> Option<&StdlibModule> {
        self.modules.get(name)
    }

    pub fn by_category(&self, category: ModuleCategory) -> Vec<&StdlibModule> {
        self.modules
            .values()
            .filter(|m| m.category == category)
            .collect()
    }

    pub fn by_status(&self, status: ModuleStatus) -> Vec<&StdlibModule> {
        self.modules
            .values()
            .filter(|m| m.status == status)
            .collect()
    }

    pub fn for_backend(&self, backend: Backend) -> Vec<&StdlibModule> {
        self.modules
            .values()
            .filter(|m| m.backends.contains(&Backend::All) || m.backends.contains(&backend))
            .collect()
    }

    pub fn coverage_stats(&self) -> CoverageStats {
        let total = self.modules.len();
        let supported = self
            .modules
            .values()
            .filter(|m| m.status == ModuleStatus::Supported)
            .count();
        let partial = self
            .modules
            .values()
            .filter(|m| m.status == ModuleStatus::Partial)
            .count();
        let shim = self
            .modules
            .values()
            .filter(|m| m.status == ModuleStatus::Shim)
            .count();
        let not_implemented = self
            .modules
            .values()
            .filter(|m| m.status == ModuleStatus::NotImplemented)
            .count();

        CoverageStats {
            total,
            supported,
            partial,
            shim,
            not_implemented,
            supported_pct: if total > 0 {
                supported as f32 / total as f32
            } else {
                0.0
            },
            covered_pct: if total > 0 {
                (supported + partial + shim) as f32 / total as f32
            } else {
                0.0
            },
        }
    }

    pub fn policy(&self) -> &CompatibilityPolicy {
        &self.policy
    }

    pub fn status_matrix(&self) -> String {
        let mut md = String::from("# Stdlib Module Status Matrix\n\n");
        md.push_str("| Module | Status | Category | Limitations |\n");
        md.push_str("|--------|--------|----------|-------------|\n");

        let mut sorted: Vec<_> = self.modules.values().collect();
        sorted.sort_by_key(|m| m.name.clone());

        for module in sorted {
            let limitations = if module.limitations.is_empty() {
                "None".to_string()
            } else {
                module.limitations.join("; ")
            };

            md.push_str(&format!(
                "| {} | {:?} | {:?} | {} |\n",
                module.name, module.status, module.category, limitations
            ));
        }

        let stats = self.coverage_stats();
        md.push_str(&format!(
            "\n**Coverage**: {}/{} modules supported ({:.0}%)\n",
            stats.supported,
            stats.total,
            stats.supported_pct * 100.0
        ));

        md
    }
}

#[derive(Debug, Clone)]
pub struct CoverageStats {
    pub total: usize,
    pub supported: usize,
    pub partial: usize,
    pub shim: usize,
    pub not_implemented: usize,
    pub supported_pct: f32,
    pub covered_pct: f32,
}

pub fn check_module_compatibility(name: &str, registry: &StdlibRegistry) -> CompatibilityReport {
    match registry.get(name) {
        Some(module) => CompatibilityReport {
            module_name: name.to_string(),
            status: module.status,
            is_compatible: module.status != ModuleStatus::Unsupported
                && module.status != ModuleStatus::NotApplicable,
            limitations: module.limitations.clone(),
            recommendations: generate_recommendations(module),
        },
        None => CompatibilityReport {
            module_name: name.to_string(),
            status: ModuleStatus::NotImplemented,
            is_compatible: false,
            limitations: vec!["Module not found in registry".to_string()],
            recommendations: vec!["Consider implementing or adding shim".to_string()],
        },
    }
}

#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    pub module_name: String,
    pub status: ModuleStatus,
    pub is_compatible: bool,
    pub limitations: Vec<String>,
    pub recommendations: Vec<String>,
}

fn generate_recommendations(module: &StdlibModule) -> Vec<String> {
    let mut recs = Vec::new();
    match module.status {
        ModuleStatus::Supported => recs.push("Module is fully supported".to_string()),
        ModuleStatus::Partial => {
            recs.push("Module has partial support - see limitations".to_string());
            recs.push("Consider testing thoroughly before production use".to_string());
        }
        ModuleStatus::Shim => {
            recs.push(
                "Module is stub implementation - full functionality not available".to_string(),
            );
        }
        ModuleStatus::NotImplemented => {
            recs.push("Module not yet implemented".to_string());
            recs.push("Consider contributing implementation".to_string());
        }
        ModuleStatus::Unsupported => {
            recs.push("Module is explicitly unsupported".to_string());
        }
        ModuleStatus::NotApplicable => {
            recs.push("Module not applicable for this backend".to_string());
        }
    }
    recs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdlib_registry_new() {
        let registry = StdlibRegistry::new();
        assert!(!registry.modules.is_empty());
    }

    #[test]
    fn test_stdlib_registry_get_module() {
        let registry = StdlibRegistry::new();
        let system = registry.get("system");
        assert!(system.is_some());
        assert_eq!(system.unwrap().name, "system");
    }

    #[test]
    fn test_stdlib_registry_by_status() {
        let registry = StdlibRegistry::new();
        let supported = registry.by_status(ModuleStatus::Supported);
        assert!(!supported.is_empty());
    }

    #[test]
    fn test_stdlib_registry_by_category() {
        let registry = StdlibRegistry::new();
        let core = registry.by_category(ModuleCategory::Core);
        assert!(!core.is_empty());
    }

    #[test]
    fn test_stdlib_registry_for_backend() {
        let registry = StdlibRegistry::new();
        let c_modules = registry.for_backend(Backend::C);
        assert!(!c_modules.is_empty());
    }

    #[test]
    fn test_coverage_stats() {
        let registry = StdlibRegistry::new();
        let stats = registry.coverage_stats();
        assert!(stats.total > 0);
        assert!(stats.supported_pct >= 0.0);
    }

    #[test]
    fn test_check_module_compatibility() {
        let registry = StdlibRegistry::new();
        let report = check_module_compatibility("system", &registry);
        assert_eq!(report.module_name, "system");
        assert!(report.is_compatible);
    }

    #[test]
    fn test_check_module_compatibility_not_found() {
        let registry = StdlibRegistry::new();
        let report = check_module_compatibility("nonexistent", &registry);
        assert!(!report.is_compatible);
    }

    #[test]
    fn test_module_status_ordering() {
        assert!(ModuleStatus::Supported < ModuleStatus::Partial);
        assert!(ModuleStatus::Partial < ModuleStatus::Shim);
        assert!(ModuleStatus::Shim < ModuleStatus::NotImplemented);
    }

    #[test]
    fn test_status_matrix_generation() {
        let registry = StdlibRegistry::new();
        let matrix = registry.status_matrix();
        assert!(matrix.contains("Stdlib Module Status Matrix"));
        assert!(matrix.contains("| Module |"));
    }

    #[test]
    fn test_compatibility_report_recommendations() {
        let registry = StdlibRegistry::new();
        let report = check_module_compatibility("system", &registry);
        assert!(!report.recommendations.is_empty());
    }
}
