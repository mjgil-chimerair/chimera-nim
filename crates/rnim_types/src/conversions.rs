//! Type conversions and converters.
//!
//! This module provides conversion ranking, implicit/explicit conversion
//! handling, and converter declaration support.

use rnim_span::Span;
use std::collections::HashMap;

/// Conversion rank - how preferred is this conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ConversionRank {
    /// Exact match, no conversion needed
    Exact,
    /// Identity conversion (e.g., int to int)
    Identity,
    /// Widening conversion (e.g., int8 to int32)
    Widening,
    /// Narrowing conversion (e.g., int32 to int8) - may lose data
    Narrowing,
    /// User-defined conversion via converter
    UserDefined,
    /// Conversion through converter chain
    UserDefinedChain,
    /// Invalid/illegal conversion
    #[default]
    Invalid,
}

/// A registered converter
#[derive(Debug, Clone)]
pub struct Converter {
    pub name: String,
    pub from_type: String,
    pub to_type: String,
    pub is_explicit: bool,
    pub span: Span,
}

impl Converter {
    pub fn new(name: &str, from_type: &str, to_type: &str, is_explicit: bool, span: Span) -> Self {
        Converter {
            name: name.to_string(),
            from_type: from_type.to_string(),
            to_type: to_type.to_string(),
            is_explicit,
            span,
        }
    }
}

/// Conversion graph for tracking legal conversions
#[derive(Debug, Clone, Default)]
pub struct ConversionGraph {
    /// Registered converters
    converters: HashMap<String, Converter>,
    /// Conversion cache: (from, to) -> rank
    cache: HashMap<(String, String), ConversionRank>,
    /// Supertype relationships: type -> supertypes
    supertypes: HashMap<String, Vec<String>>,
}

impl ConversionGraph {
    pub fn new() -> Self {
        ConversionGraph {
            converters: HashMap::new(),
            cache: HashMap::new(),
            supertypes: HashMap::new(),
        }
    }

    /// Register a converter
    pub fn register_converter(&mut self, converter: Converter) {
        let key = format!(
            "{}:{}->{}",
            converter.from_type, converter.to_type, converter.name
        );
        self.converters.insert(key, converter);
        // Invalidate cache
        self.cache.clear();
    }

    /// Add a supertype relationship
    pub fn add_supertype(&mut self, subtype: &str, supertype: &str) {
        self.supertypes
            .entry(subtype.to_string())
            .or_default()
            .push(supertype.to_string());
        self.cache.clear();
    }

    /// Get the conversion rank between two types
    pub fn get_conversion_rank(&mut self, from: &str, to: &str) -> ConversionRank {
        // Check cache first
        if let Some(&rank) = self.cache.get(&(from.to_string(), to.to_string())) {
            return rank;
        }

        let rank = self.compute_conversion_rank(from, to);

        // Cache the result
        self.cache.insert((from.to_string(), to.to_string()), rank);
        rank
    }

    /// Compute conversion rank between types
    fn compute_conversion_rank(&mut self, from: &str, to: &str) -> ConversionRank {
        // Same type - exact match
        if from == to {
            return ConversionRank::Exact;
        }

        // Check for registered converter
        let converter_key = format!("{}:{}->*", from, to);
        if self
            .converters
            .keys()
            .any(|k| k.starts_with(&converter_key))
        {
            // Check if it's explicit only
            let _full_key = format!("{}:{}->", from, to);
            for conv in self.converters.values() {
                if conv.from_type == from && conv.to_type == to {
                    return ConversionRank::UserDefined;
                }
            }
        }

        // Check numeric conversions
        if let (Some(from_info), Some(to_info)) = (parse_numeric_type(from), parse_numeric_type(to))
        {
            return rank_numeric_conversion(from_info, to_info);
        }

        // Check subtype relationships
        if self.is_subtype(from, to) {
            return ConversionRank::Widening;
        }

        // Check if there's a path through supertypes
        if let Some(path) = self.find_conversion_path(from, to) {
            if path.len() == 1 {
                ConversionRank::Widening
            } else {
                ConversionRank::UserDefinedChain
            }
        } else {
            ConversionRank::Invalid
        }
    }

    /// Check if type is a subtype of another
    fn is_subtype(&self, from: &str, to: &str) -> bool {
        if let Some(supers) = self.supertypes.get(from) {
            if supers.contains(&to.to_string()) {
                return true;
            }
            // Check transitive
            for super_type in supers {
                if self.is_subtype(super_type, to) {
                    return true;
                }
            }
        }
        false
    }

    /// Find a conversion path between types
    fn find_conversion_path(&mut self, from: &str, to: &str) -> Option<Vec<String>> {
        // Simple BFS for conversion path
        use std::collections::VecDeque;
        let mut queue: VecDeque<Vec<String>> = VecDeque::new();
        queue.push_back(vec![from.to_string()]);
        let mut visited = std::collections::HashSet::new();
        visited.insert(from.to_string());

        while let Some(path) = queue.pop_front() {
            let current = path.last().unwrap();
            if current == to {
                return Some(path);
            }

            // Try all supertypes
            if let Some(supers) = self.supertypes.get(current) {
                for super_type in supers {
                    if !visited.contains(super_type) {
                        visited.insert(super_type.clone());
                        let mut new_path = path.clone();
                        new_path.push(super_type.clone());
                        queue.push_back(new_path);
                    }
                }
            }

            // Try converters
            for conv in self.converters.values() {
                if conv.from_type == *current && !visited.contains(&conv.to_type) {
                    visited.insert(conv.to_type.clone());
                    let mut new_path = path.clone();
                    new_path.push(conv.to_type.clone());
                    queue.push_back(new_path);
                }
            }
        }

        None
    }

    /// Get all converters from a type
    pub fn get_converters_from(&self, from_type: &str) -> Vec<&Converter> {
        self.converters
            .values()
            .filter(|c| c.from_type == from_type)
            .collect()
    }

    /// Get all converters to a type
    pub fn get_converters_to(&self, to_type: &str) -> Vec<&Converter> {
        self.converters
            .values()
            .filter(|c| c.to_type == to_type)
            .collect()
    }

    /// Check if a conversion is legal (not Invalid rank)
    pub fn is_conversion_legal(&mut self, from: &str, to: &str) -> bool {
        let rank = self.get_conversion_rank(from, to);
        rank != ConversionRank::Invalid
    }

    /// Check if a conversion is implicit (can be done without explicit cast)
    pub fn is_implicit_conversion(&mut self, from: &str, to: &str) -> bool {
        let rank = self.get_conversion_rank(from, to);
        matches!(
            rank,
            ConversionRank::Exact | ConversionRank::Identity | ConversionRank::Widening
        )
    }
}

/// Information about a numeric type
#[derive(Debug, Clone, Copy)]
struct NumericTypeInfo {
    kind: NumericKind,
    size: u32,
    is_signed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumericKind {
    Int,
    Float,
}

fn parse_numeric_type(s: &str) -> Option<NumericTypeInfo> {
    match s {
        "int8" => Some(NumericTypeInfo {
            kind: NumericKind::Int,
            size: 8,
            is_signed: true,
        }),
        "int16" => Some(NumericTypeInfo {
            kind: NumericKind::Int,
            size: 16,
            is_signed: true,
        }),
        "int32" => Some(NumericTypeInfo {
            kind: NumericKind::Int,
            size: 32,
            is_signed: true,
        }),
        "int64" => Some(NumericTypeInfo {
            kind: NumericKind::Int,
            size: 64,
            is_signed: true,
        }),
        "int" | "cint" => Some(NumericTypeInfo {
            kind: NumericKind::Int,
            size: 32,
            is_signed: true,
        }),
        "uint8" => Some(NumericTypeInfo {
            kind: NumericKind::Int,
            size: 8,
            is_signed: false,
        }),
        "uint16" => Some(NumericTypeInfo {
            kind: NumericKind::Int,
            size: 16,
            is_signed: false,
        }),
        "uint32" => Some(NumericTypeInfo {
            kind: NumericKind::Int,
            size: 32,
            is_signed: false,
        }),
        "uint64" => Some(NumericTypeInfo {
            kind: NumericKind::Int,
            size: 64,
            is_signed: false,
        }),
        "uint" | "cuint" => Some(NumericTypeInfo {
            kind: NumericKind::Int,
            size: 32,
            is_signed: false,
        }),
        "float32" => Some(NumericTypeInfo {
            kind: NumericKind::Float,
            size: 32,
            is_signed: true,
        }),
        "float64" | "float" => Some(NumericTypeInfo {
            kind: NumericKind::Float,
            size: 64,
            is_signed: true,
        }),
        "cfloat" => Some(NumericTypeInfo {
            kind: NumericKind::Float,
            size: 32,
            is_signed: true,
        }),
        _ => None,
    }
}

fn rank_numeric_conversion(from: NumericTypeInfo, to: NumericTypeInfo) -> ConversionRank {
    // Can't convert between int and float directly without user-defined converter
    if from.kind != to.kind {
        return ConversionRank::Invalid;
    }

    // Same type
    if from.size == to.size && from.is_signed == to.is_signed {
        return ConversionRank::Exact;
    }

    // Widening conversion (smaller to larger, same signedness)
    if from.is_signed == to.is_signed && from.size < to.size {
        return ConversionRank::Widening;
    }

    // Widening from unsigned to signed of same or larger size
    if !from.is_signed && to.is_signed && from.size <= to.size {
        return ConversionRank::Widening;
    }

    // Narrowing conversion
    ConversionRank::Narrowing
}

/// Conversion result with rank and path
#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub rank: ConversionRank,
    pub path: Vec<String>,
    pub requires_explicit: bool,
}

impl ConversionResult {
    pub fn new(rank: ConversionRank, path: Vec<String>) -> Self {
        let requires_explicit = matches!(
            rank,
            ConversionRank::Narrowing
                | ConversionRank::UserDefined
                | ConversionRank::UserDefinedChain
        );
        ConversionResult {
            rank,
            path,
            requires_explicit,
        }
    }

    pub fn invalid() -> Self {
        ConversionResult {
            rank: ConversionRank::Invalid,
            path: Vec::new(),
            requires_explicit: true,
        }
    }
}

/// Find the best conversion between two types
pub fn find_best_conversion(graph: &mut ConversionGraph, from: &str, to: &str) -> ConversionResult {
    let rank = graph.get_conversion_rank(from, to);
    let path = graph.find_conversion_path(from, to).unwrap_or_default();
    ConversionResult::new(rank, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_conversion_rank_ordering() {
        assert!(ConversionRank::Exact < ConversionRank::Identity);
        assert!(ConversionRank::Identity < ConversionRank::Widening);
        assert!(ConversionRank::Widening < ConversionRank::Narrowing);
        assert!(ConversionRank::Narrowing < ConversionRank::UserDefined);
        assert!(ConversionRank::UserDefined < ConversionRank::Invalid);
    }

    #[test]
    fn test_conversion_graph_new() {
        let graph = ConversionGraph::new();
        assert!(graph.converters.is_empty());
        assert!(graph.cache.is_empty());
    }

    #[test]
    fn test_conversion_graph_register_converter() {
        let mut graph = ConversionGraph::new();
        let conv = Converter::new("MyConv", "int", "MyType", false, Span::new(FileId(0), 0, 0));
        graph.register_converter(conv);
        assert_eq!(graph.converters.len(), 1);
    }

    #[test]
    fn test_conversion_graph_same_type() {
        let mut graph = ConversionGraph::new();
        let rank = graph.get_conversion_rank("int32", "int32");
        assert_eq!(rank, ConversionRank::Exact);
    }

    #[test]
    fn test_conversion_graph_int_widening() {
        let mut graph = ConversionGraph::new();
        let rank = graph.get_conversion_rank("int8", "int16");
        assert_eq!(rank, ConversionRank::Widening);
    }

    #[test]
    fn test_conversion_graph_int_narrowing() {
        let mut graph = ConversionGraph::new();
        let rank = graph.get_conversion_rank("int32", "int8");
        assert_eq!(rank, ConversionRank::Narrowing);
    }

    #[test]
    fn test_conversion_graph_supertype() {
        let mut graph = ConversionGraph::new();
        graph.add_supertype("Child", "Parent");
        let rank = graph.get_conversion_rank("Child", "Parent");
        assert_eq!(rank, ConversionRank::Widening);
    }

    #[test]
    fn test_conversion_graph_int_to_float_invalid() {
        let mut graph = ConversionGraph::new();
        let rank = graph.get_conversion_rank("int32", "float32");
        assert_eq!(rank, ConversionRank::Invalid);
    }

    #[test]
    fn test_is_conversion_legal() {
        let mut graph = ConversionGraph::new();
        assert!(!graph.is_conversion_legal("int32", "float32"));
        assert!(graph.is_conversion_legal("int8", "int16"));
    }

    #[test]
    fn test_is_implicit_conversion() {
        let mut graph = ConversionGraph::new();
        assert!(graph.is_implicit_conversion("int8", "int16"));
        assert!(!graph.is_implicit_conversion("int32", "int8"));
    }

    #[test]
    fn test_get_converters_from() {
        let mut graph = ConversionGraph::new();
        let conv = Converter::new(
            "ToMyType",
            "int",
            "MyType",
            false,
            Span::new(FileId(0), 0, 0),
        );
        graph.register_converter(conv);
        let converters = graph.get_converters_from("int");
        assert_eq!(converters.len(), 1);
        assert_eq!(converters[0].to_type, "MyType");
    }

    #[test]
    fn test_get_converters_to() {
        let mut graph = ConversionGraph::new();
        let conv = Converter::new(
            "FromMyType",
            "MyType",
            "int",
            false,
            Span::new(FileId(0), 0, 0),
        );
        graph.register_converter(conv);
        let converters = graph.get_converters_to("int");
        assert_eq!(converters.len(), 1);
        assert_eq!(converters[0].from_type, "MyType");
    }

    #[test]
    fn test_conversion_result_requires_explicit() {
        let result = ConversionResult::new(ConversionRank::Narrowing, vec![]);
        assert!(result.requires_explicit);

        let result = ConversionResult::new(ConversionRank::Widening, vec![]);
        assert!(!result.requires_explicit);
    }

    #[test]
    fn test_find_best_conversion() {
        let mut graph = ConversionGraph::new();
        graph.add_supertype("Child", "Parent");
        let result = find_best_conversion(&mut graph, "Child", "Parent");
        assert_eq!(result.rank, ConversionRank::Widening);
    }

    #[test]
    fn test_conversion_result_invalid() {
        let result = ConversionResult::invalid();
        assert_eq!(result.rank, ConversionRank::Invalid);
        assert!(result.requires_explicit);
    }

    #[test]
    fn test_numeric_type_parsing() {
        assert!(parse_numeric_type("int8").is_some());
        assert!(parse_numeric_type("int32").is_some());
        assert!(parse_numeric_type("float64").is_some());
        assert!(parse_numeric_type("string").is_none());
        assert!(parse_numeric_type("MyType").is_none());
    }
}
