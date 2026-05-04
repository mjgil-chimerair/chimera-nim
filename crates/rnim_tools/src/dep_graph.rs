//! Dependency graph validation for the rnim workspace.
//!
//! This module validates that crate dependencies follow the allowed edges
//! defined in docs/design/workspace.md.

use std::collections::HashMap;
use std::path::Path;

const ALLOWED_EDGES: &[(&str, &str)] = &[
    // rnim can depend on many crates
    ("rnim", "rnim_session"),
    ("rnim", "rnim_lexer"),
    ("rnim", "rnim_parser"),
    ("rnim", "rnim_ast"),
    ("rnim", "rnim_diag"),
    ("rnim", "rnim_symbols"),
    ("rnim", "rnim_hir"),
    ("rnim", "rnim_types"),
    ("rnim", "rnim_sema"),
    ("rnim", "rnim_build"),
    ("rnim", "rnim_docgen"),
    ("rnim", "rnim_suggest"),
    ("rnim", "rnim_test"),
    ("rnim", "rnim_tools"),
    // rnim_span is foundational - no workspace deps
    // rnim_diag -> rnim_span
    ("rnim_diag", "rnim_span"),
    // rnim_lexer -> rnim_span
    ("rnim_lexer", "rnim_span"),
    // rnim_parser -> rnim_span, rnim_lexer, rnim_diag
    ("rnim_parser", "rnim_span"),
    ("rnim_parser", "rnim_lexer"),
    ("rnim_parser", "rnim_diag"),
    // rnim_ast -> rnim_span, rnim_parser
    ("rnim_ast", "rnim_span"),
    ("rnim_ast", "rnim_parser"),
    // rnim_symbols -> rnim_span, rnim_ast
    ("rnim_symbols", "rnim_span"),
    ("rnim_symbols", "rnim_ast"),
    // rnim_hir -> rnim_span, rnim_ast, rnim_symbols
    ("rnim_hir", "rnim_span"),
    ("rnim_hir", "rnim_ast"),
    ("rnim_hir", "rnim_symbols"),
    // rnim_types -> rnim_span, rnim_ast
    ("rnim_types", "rnim_span"),
    ("rnim_types", "rnim_ast"),
    // rnim_sema -> rnim_span, rnim_ast, rnim_hir, rnim_types, rnim_effects
    ("rnim_sema", "rnim_span"),
    ("rnim_sema", "rnim_ast"),
    ("rnim_sema", "rnim_hir"),
    ("rnim_sema", "rnim_types"),
    ("rnim_sema", "rnim_effects"),
    // rnim_effects -> rnim_span, rnim_hir
    ("rnim_effects", "rnim_span"),
    ("rnim_effects", "rnim_hir"),
    // rnim_macro -> rnim_ast, rnim_vm, rnim_span
    ("rnim_macro", "rnim_ast"),
    ("rnim_macro", "rnim_vm"),
    ("rnim_macro", "rnim_span"),
    // rnim_vm -> rnim_span, rnim_ast
    ("rnim_vm", "rnim_span"),
    ("rnim_vm", "rnim_ast"),
    // rnim_mir -> rnim_span, rnim_hir
    ("rnim_mir", "rnim_span"),
    ("rnim_mir", "rnim_hir"),
    // rnim_lifetime -> rnim_span, rnim_hir, rnim_mir
    ("rnim_lifetime", "rnim_span"),
    ("rnim_lifetime", "rnim_hir"),
    ("rnim_lifetime", "rnim_mir"),
    // rnim_codegen_c -> rnim_span, rnim_mir
    ("rnim_codegen_c", "rnim_span"),
    ("rnim_codegen_c", "rnim_mir"),
    // rnim_codegen_js -> rnim_span, rnim_mir
    ("rnim_codegen_js", "rnim_span"),
    ("rnim_codegen_js", "rnim_mir"),
    // rnim_build -> rnim_span
    ("rnim_build", "rnim_span"),
    // rnim_docgen -> rnim_span, rnim_ast
    ("rnim_docgen", "rnim_span"),
    ("rnim_docgen", "rnim_ast"),
    // rnim_suggest -> rnim_session, rnim_lexer, rnim_parser, rnim_ast, rnim_sema
    ("rnim_suggest", "rnim_session"),
    ("rnim_suggest", "rnim_lexer"),
    ("rnim_suggest", "rnim_parser"),
    ("rnim_suggest", "rnim_ast"),
    ("rnim_suggest", "rnim_sema"),
    // rnim_test -> rnim_session, rnim_parser
    ("rnim_test", "rnim_session"),
    ("rnim_test", "rnim_parser"),
    // rnim_fuzz -> rnim_lexer, rnim_parser
    ("rnim_fuzz", "rnim_lexer"),
    ("rnim_fuzz", "rnim_parser"),
    // rnim_tools -> rnim_span, rnim_parser
    ("rnim_tools", "rnim_span"),
    ("rnim_tools", "rnim_parser"),
];

const FRONTEND_CRATES: &[&str] = &[
    "rnim_lexer",
    "rnim_parser",
    "rnim_ast",
    "rnim_symbols",
    "rnim_hir",
    "rnim_types",
    "rnim_sema",
];

const BACKEND_CRATES: &[&str] = &[
    "rnim_mir",
    "rnim_codegen_c",
    "rnim_codegen_js",
    "rnim_lifetime",
    "rnim_build",
];

/// Check if a dependency edge is allowed
pub fn is_allowed_edge(from: &str, to: &str) -> bool {
    ALLOWED_EDGES.iter().any(|(f, t)| *f == from && *t == to)
}

/// Check if from is a frontend crate and to is a backend crate
pub fn is_frontend_backend_violation(from: &str, to: &str) -> bool {
    FRONTEND_CRATES.contains(&from) && BACKEND_CRATES.contains(&to)
}

/// Parse Cargo.toml to extract dependencies
fn parse_deps(cargo_toml: &str) -> Vec<String> {
    let mut deps = Vec::new();

    // Simple regex-like parsing for [dependencies] section
    let in_deps = cargo_toml.contains("[dependencies]");
    if !in_deps {
        return deps;
    }

    // Extract deps between [dependencies] and next [section]
    let deps_section = cargo_toml
        .split("[dependencies]")
        .nth(1)
        .unwrap_or("")
        .split(|c| c == '[')
        .next()
        .unwrap_or("");

    for line in deps_section.lines() {
        let line = line.trim();
        // Match patterns like: crate_name = { ... } or crate_name = "version"
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Extract crate name before = or {
        if let Some(eq_pos) = line.find('=') {
            let name = line[..eq_pos].trim();
            // Remove workspace prefix if present
            let name = name.trim_start_matches("workspace.");
            // Filter out path deps, features, etc.
            if !name.contains('{')
                && !name.contains(']')
                && name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                deps.push(name.to_string());
            }
        }
    }

    deps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_edges_coverage() {
        // Verify all edges are unique
        let mut edge_set: HashMap<(&str, &str), usize> = HashMap::new();
        for &(from, to) in ALLOWED_EDGES {
            *edge_set.entry((from, to)).or_insert(0) += 1;
        }
        for ((from, to), count) in edge_set.iter() {
            assert_eq!(*count, 1, "Duplicate edge: {} -> {}", from, to);
        }
    }

    #[test]
    fn test_is_allowed_edge() {
        // Valid edges
        assert!(is_allowed_edge("rnim", "rnim_session"));
        assert!(is_allowed_edge("rnim_parser", "rnim_span"));
        assert!(is_allowed_edge("rnim_types", "rnim_ast"));

        // Invalid edges
        assert!(!is_allowed_edge("rnim_span", "rnim_parser")); // reverse
        assert!(!is_allowed_edge("rnim", "rnim")); // self
    }

    #[test]
    fn test_frontend_backend_violation() {
        // Frontend to backend is violation
        assert!(is_frontend_backend_violation("rnim_parser", "rnim_mir"));
        assert!(is_frontend_backend_violation(
            "rnim_types",
            "rnim_codegen_c"
        ));

        // Same category is not violation
        assert!(!is_frontend_backend_violation("rnim_parser", "rnim_lexer"));
        assert!(!is_frontend_backend_violation("rnim_mir", "rnim_codegen_c"));

        // Backend to frontend is not violation (allowed)
        assert!(!is_frontend_backend_violation("rnim_mir", "rnim_parser"));
    }

    #[test]
    fn test_parse_deps() {
        let cargo_toml = r#"
[package]
name = "test"

[dependencies]
rnim_span = { path = "../rnim_span" }
some_crate = "1.0"
"#;
        let deps = parse_deps(cargo_toml);
        assert!(deps.contains(&"some_crate".to_string()));
    }
}
