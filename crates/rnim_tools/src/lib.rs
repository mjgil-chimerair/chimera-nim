//! Shared utilities for formatter, minimizer, snapshot updater, coverage utilities.

#[cfg(test)]
use rnim_allocator as _;
pub mod formatter;

pub use formatter::{
    format_default, format_source, FormatToken, FormatTokenKind, Formatter, FormatterConfig,
    FormatterState, PrettyPrinted,
};

pub struct PrettyPrinter {}

impl PrettyPrinter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn format(&mut self, _source: &str) -> String {
        _source.to_string()
    }
}

impl Default for PrettyPrinter {
    fn default() -> Self {
        Self::new()
    }
}
pub mod dep_graph;

pub use dep_graph::{is_allowed_edge, is_frontend_backend_violation};

pub mod design_doc;

pub use design_doc::{generate_status_report, generate_task_matrix};
