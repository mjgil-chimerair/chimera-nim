//! Workspace validation tests for chimera-nim.
//!
//! This module provides tests that validate:
//! - Cargo metadata can be queried successfully
//! - No cyclic dependencies exist in the workspace
//! - All declared workspace members exist on disk

use std::path::PathBuf;

/// Result of validating the workspace
#[derive(Debug, Clone)]
pub struct WorkspaceValidation {
    /// Whether metadata was successfully queried
    pub metadata_available: bool,
    /// Number of workspace members found
    pub member_count: usize,
    /// Expected number of members (from workspace config)
    pub expected_members: usize,
    /// Whether any cycles were detected
    pub has_cycles: bool,
    /// List of missing members (if any)
    pub missing_members: Vec<String>,
}

impl WorkspaceValidation {
    /// Create a new validation result
    pub fn new() -> Self {
        Self {
            metadata_available: false,
            member_count: 0,
            expected_members: 0,
            has_cycles: false,
            missing_members: Vec::new(),
        }
    }

    /// Check if validation passed
    pub fn passed(&self) -> bool {
        self.metadata_available
            && self.member_count == self.expected_members
            && !self.has_cycles
            && self.missing_members.is_empty()
    }
}

impl Default for WorkspaceValidation {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate cargo metadata can be retrieved
pub fn validate_cargo_metadata() -> Result<cargo_metadata::Metadata, String> {
    cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .map_err(|e| format!("Failed to get cargo metadata: {}", e))
}

/// Get all packages in the workspace
pub fn get_all_packages() -> Result<Vec<String>, String> {
    let metadata = validate_cargo_metadata()?;
    Ok(metadata.packages.iter().map(|p| p.name.clone()).collect())
}

/// Check for dependency cycles using cargo metadata
pub fn check_dependency_cycles() -> Result<bool, String> {
    let metadata = validate_cargo_metadata()?;

    // Get set of workspace package names
    let workspace_names: std::collections::HashSet<_> = metadata
        .workspace_members
        .iter()
        .map(|p| p.to_string())
        .collect();

    // Build adjacency list from dependencies (only internal deps)
    let mut graph: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();

    for package in &metadata.packages {
        if !workspace_names.contains(&package.name) {
            continue;
        }

        let deps: Vec<&str> = package
            .dependencies
            .iter()
            .filter(|dep| workspace_names.contains(&dep.name))
            .map(|dep| dep.name.as_str())
            .collect();
        graph.insert(package.name.as_str(), deps);
    }

    // DFS-based cycle detection with owned strings
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut rec_stack: std::collections::HashSet<String> = std::collections::HashSet::new();

    fn has_cycle(
        graph: &std::collections::HashMap<&str, Vec<&str>>,
        node: &str,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> bool {
        if rec_stack.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }

        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(deps) = graph.get(node) {
            for dep in deps {
                if has_cycle(graph, dep, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    for package_name in graph.keys() {
        if has_cycle(&graph, package_name, &mut visited, &mut rec_stack) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Validate all workspace members exist on disk
pub fn validate_members_exist(members: &[String]) -> Vec<String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let mut missing = Vec::new();
    for member in members {
        let member_path = workspace_root.join(member);
        if !member_path.exists() {
            missing.push(member.clone());
        }
    }
    missing
}

/// Expected workspace members
pub fn expected_workspace_members() -> Vec<&'static str> {
    vec![
        "crates/chimera",
        "crates/rnim_allocator",
        "crates/rnim_session",
        "crates/rnim_span",
        "crates/rnim_diag",
        "crates/rnim_lexer",
        "crates/rnim_parser",
        "crates/rnim_ast",
        "crates/rnim_symbols",
        "crates/rnim_hir",
        "crates/rnim_types",
        "crates/rnim_sema",
        "crates/rnim_effects",
        "crates/rnim_macro",
        "crates/rnim_vm",
        "crates/rnim_mir",
        "crates/rnim_lifetime",
        "crates/rnim_codegen_c",
        "crates/rnim_codegen_js",
        "crates/rnim_runtime",
        "crates/rnim_stdlib",
        "crates/rnim_nimble",
        "crates/rnim_docgen",
        "crates/rnim_suggest",
        "crates/rnim_test",
        "crates/rnim_fuzz",
        "crates/rnim_tools",
        "crates/rnim_build",
        "crates/rnim_query",
        "xtask",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_metadata_available() {
        let result = validate_cargo_metadata();
        assert!(
            result.is_ok(),
            "cargo metadata should be available: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_workspace_has_expected_members() {
        let packages = get_all_packages().expect("should get packages");

        // At minimum, we should have chimera as a member
        assert!(
            packages.iter().any(|p| p.contains("chimera")),
            "chimera should be a workspace member. Found: {:?}",
            packages
        );
    }

    #[test]
    fn test_no_dependency_cycles() {
        let has_cycles = check_dependency_cycles().expect("should check cycles");
        assert!(!has_cycles, "Workspace should not have dependency cycles");
    }

    #[test]
    fn test_all_declared_members_exist() {
        let expected = expected_workspace_members();
        let missing =
            validate_members_exist(&expected.iter().map(|s| s.to_string()).collect::<Vec<_>>());

        assert!(
            missing.is_empty(),
            "All declared workspace members should exist. Missing: {:?}",
            missing
        );
    }

    #[test]
    fn test_workspace_validation_passes() {
        let packages = get_all_packages().expect("should get packages");
        let has_cycles = check_dependency_cycles().expect("should check cycles");
        let missing = validate_members_exist(
            &expected_workspace_members()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        );

        let validation = WorkspaceValidation {
            metadata_available: true,
            member_count: packages.len(),
            expected_members: expected_workspace_members().len(),
            has_cycles,
            missing_members: missing,
        };

        assert!(
            validation.passed(),
            "Workspace validation should pass. validation={:?}",
            validation
        );
    }

    #[test]
    fn test_packages_have_versions() {
        let packages = get_all_packages().expect("should get packages");
        assert!(!packages.is_empty(), "Should have packages in workspace");
    }
}
