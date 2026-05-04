//! Overload resolution for routine calls.
//!
//! This module provides ranking and selection of overloaded routines
//! based on argument types, conversion costs, and other factors.

use rnim_span::Span;
use std::collections::{HashMap, HashSet};

/// Score for a candidate match
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchScore {
    /// Exact match, no conversion needed
    Exact,
    /// Match with safe conversion (widening)
    SafeConversion(i32),
    /// Match requiring user-defined conversion
    UserConversion(i32),
    /// Match with potentially lossy conversion (narrowing)
    UnsafeConversion(i32),
    /// Generic match awaiting type resolution
    GenericMatch,
    /// Match via varargs
    VarargsMatch,
    /// No match possible
    NoMatch,
}

impl MatchScore {
    /// Check if this score represents a viable match
    pub fn is_viable(&self) -> bool {
        !matches!(self, MatchScore::NoMatch)
    }

    /// Check if this is a better match than another
    pub fn is_better_than(&self, other: &MatchScore) -> bool {
        match (self, other) {
            (MatchScore::Exact, MatchScore::Exact) => true,
            (MatchScore::Exact, _) => true,
            (_, MatchScore::Exact) => false,
            (MatchScore::GenericMatch, MatchScore::GenericMatch) => true,
            (MatchScore::GenericMatch, _) => true,
            (_, MatchScore::GenericMatch) => false,
            (MatchScore::VarargsMatch, MatchScore::VarargsMatch) => true,
            (MatchScore::VarargsMatch, _) => true,
            (_, MatchScore::VarargsMatch) => false,
            (MatchScore::SafeConversion(a), MatchScore::SafeConversion(b)) => a < b,
            (MatchScore::UserConversion(a), MatchScore::UserConversion(b)) => a < b,
            (MatchScore::UnsafeConversion(a), MatchScore::UnsafeConversion(b)) => a < b,
            (MatchScore::SafeConversion(_), MatchScore::UserConversion(_)) => true,
            (MatchScore::SafeConversion(_), MatchScore::UnsafeConversion(_)) => true,
            (MatchScore::UserConversion(_), MatchScore::UnsafeConversion(_)) => true,
            _ => false,
        }
    }
}

/// A candidate in an overload set
#[derive(Debug, Clone)]
pub struct Candidate {
    pub name: String,
    pub param_types: Vec<String>,
    pub return_type: Option<String>,
    pub defaults: usize,
    pub is_variadic: bool,
    pub span: Span,
}

impl Candidate {
    pub fn new(
        name: &str,
        param_types: Vec<String>,
        return_type: Option<String>,
        span: Span,
    ) -> Self {
        Candidate {
            name: name.to_string(),
            param_types,
            return_type,
            defaults: 0,
            is_variadic: false,
            span,
        }
    }

    pub fn with_defaults(mut self, defaults: usize) -> Self {
        self.defaults = defaults;
        self
    }

    pub fn variadic(mut self) -> Self {
        self.is_variadic = true;
        self
    }
}

/// Result of resolving overloads
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub candidates: Vec<CandidateMatch>,
    pub best_candidate: Option<usize>,
}

impl ResolutionResult {
    pub fn new() -> Self {
        ResolutionResult {
            candidates: Vec::new(),
            best_candidate: None,
        }
    }

    pub fn with_candidates(candidates: Vec<CandidateMatch>) -> Self {
        let mut best_candidate: Option<usize> = None;
        let mut best_score: Option<&MatchScore> = None;

        // Find best candidate
        for (i, c) in candidates.iter().enumerate() {
            if c.score.is_viable() {
                match &best_score {
                    None => {
                        best_score = Some(&c.score);
                        best_candidate = Some(i);
                    }
                    Some(best) => {
                        if c.score.is_better_than(best) {
                            best_score = Some(&c.score);
                            best_candidate = Some(i);
                        }
                    }
                }
            }
        }

        ResolutionResult {
            candidates,
            best_candidate,
        }
    }

    pub fn is_ambiguous(&self) -> bool {
        let viable: Vec<_> = self
            .candidates
            .iter()
            .filter(|c| c.score.is_viable())
            .collect();
        if viable.len() <= 1 {
            return false;
        }
        // Check if top two candidates have the same score
        if viable.len() >= 2 {
            let scores: Vec<_> = viable.iter().map(|c| &c.score).collect();
            if scores[0] == scores[1] {
                return true;
            }
        }
        false
    }
}

impl Default for ResolutionResult {
    fn default() -> Self {
        Self::new()
    }
}

/// A matched candidate with its score
#[derive(Debug, Clone)]
pub struct CandidateMatch {
    pub candidate: Candidate,
    pub score: MatchScore,
    pub conversion_costs: Vec<i32>,
}

/// Overload resolver
#[derive(Debug, Clone, Default)]
pub struct OverloadResolver {
    /// Registered candidates
    candidates: Vec<Candidate>,
}

impl OverloadResolver {
    pub fn new() -> Self {
        OverloadResolver {
            candidates: Vec::new(),
        }
    }

    /// Add a candidate
    pub fn add_candidate(&mut self, candidate: Candidate) {
        self.candidates.push(candidate);
    }

    /// Add multiple candidates
    pub fn add_candidates(&mut self, candidates: Vec<Candidate>) {
        self.candidates.extend(candidates);
    }

    /// Get all candidates
    pub fn get_candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    /// Resolve overload for given argument types
    pub fn resolve(&self, arg_types: &[String]) -> ResolutionResult {
        let mut matches = Vec::new();

        for candidate in &self.candidates {
            let score = self.score_candidate(candidate, arg_types);
            matches.push(CandidateMatch {
                candidate: candidate.clone(),
                score,
                conversion_costs: Vec::new(),
            });
        }

        ResolutionResult::with_candidates(matches)
    }

    /// Score how well a candidate matches the argument types
    fn score_candidate(&self, candidate: &Candidate, arg_types: &[String]) -> MatchScore {
        let params = &candidate.param_types;

        // Check for varargs
        if candidate.is_variadic {
            if arg_types.len() >= params.len() - 1 {
                return MatchScore::VarargsMatch;
            }
        }

        // Check argument count
        if arg_types.len() < params.len() {
            // Check if we have enough defaults
            let needed = params.len() - arg_types.len();
            if needed > candidate.defaults {
                return MatchScore::NoMatch;
            }
        }

        if arg_types.len() > params.len() && !candidate.is_variadic {
            return MatchScore::NoMatch;
        }

        // Score each argument
        let mut total_cost = 0;
        let mut has_generic = false;
        let mut has_user_conversion = false;
        let mut has_unsafe_conversion = false;

        for (i, arg_type) in arg_types.iter().enumerate() {
            if i >= params.len() {
                break;
            }

            let param_type = &params[i];
            let cost = self.score_argument(arg_type, param_type);

            // Invalid conversion (cost >= 1000) is not viable
            if cost >= 1000 {
                return MatchScore::NoMatch;
            }

            match cost {
                0 => {}
                1..=10 => total_cost += cost,
                11..=100 => {
                    total_cost += cost;
                    has_user_conversion = true;
                }
                101.. => {
                    total_cost += cost;
                    has_unsafe_conversion = true;
                }
                _ => return MatchScore::NoMatch,
            }
        }

        // Determine final score
        if has_unsafe_conversion {
            MatchScore::UnsafeConversion(total_cost)
        } else if has_user_conversion {
            MatchScore::UserConversion(total_cost)
        } else if has_generic {
            MatchScore::GenericMatch
        } else if total_cost == 0 {
            MatchScore::Exact
        } else {
            MatchScore::SafeConversion(total_cost)
        }
    }

    /// Score a single argument against a parameter type
    fn score_argument(&self, arg_type: &str, param_type: &str) -> i32 {
        // Exact match
        if arg_type == param_type {
            return 0;
        }

        // Check conversion rank - use a temporary graph for scoring
        let mut temp_graph = super::conversions::ConversionGraph::new();
        let rank = temp_graph.get_conversion_rank(arg_type, param_type);
        match rank {
            super::conversions::ConversionRank::Exact => 0,
            super::conversions::ConversionRank::Identity => 1,
            super::conversions::ConversionRank::Widening => 5,
            super::conversions::ConversionRank::Narrowing => 150,
            super::conversions::ConversionRank::UserDefined => 50,
            super::conversions::ConversionRank::UserDefinedChain => 75,
            super::conversions::ConversionRank::Invalid => 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_match_score_is_viable() {
        assert!(MatchScore::Exact.is_viable());
        assert!(MatchScore::SafeConversion(1).is_viable());
        assert!(!MatchScore::NoMatch.is_viable());
    }

    #[test]
    fn test_match_score_is_better_than() {
        assert!(MatchScore::Exact.is_better_than(&MatchScore::SafeConversion(1)));
        assert!(MatchScore::SafeConversion(1).is_better_than(&MatchScore::UnsafeConversion(50)));
        assert!(MatchScore::UnsafeConversion(1).is_better_than(&MatchScore::UnsafeConversion(50)));
    }

    #[test]
    fn test_candidate_new() {
        let candidate = Candidate::new(
            "foo",
            vec!["int".to_string(), "string".to_string()],
            Some("void".to_string()),
            Span::new(FileId(0), 0, 0),
        );
        assert_eq!(candidate.name, "foo");
        assert_eq!(candidate.param_types.len(), 2);
        assert!(candidate.return_type.is_some());
    }

    #[test]
    fn test_candidate_with_defaults() {
        let candidate = Candidate::new(
            "bar",
            vec!["int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        )
        .with_defaults(2);
        assert_eq!(candidate.defaults, 2);
    }

    #[test]
    fn test_candidate_variadic() {
        let candidate = Candidate::new(
            "baz",
            vec!["int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        )
        .variadic();
        assert!(candidate.is_variadic);
    }

    #[test]
    fn test_resolution_result_new() {
        let result = ResolutionResult::new();
        assert!(result.candidates.is_empty());
        assert!(result.best_candidate.is_none());
    }

    #[test]
    fn test_resolution_result_with_candidates() {
        let candidate = Candidate::new(
            "foo",
            vec!["int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        );
        let matches = vec![CandidateMatch {
            candidate,
            score: MatchScore::Exact,
            conversion_costs: vec![],
        }];
        let result = ResolutionResult::with_candidates(matches);
        assert!(result.best_candidate.is_some());
    }

    #[test]
    fn test_resolution_result_is_ambiguous() {
        let candidate1 = Candidate::new(
            "foo",
            vec!["int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        );
        let candidate2 = Candidate::new(
            "foo",
            vec!["int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        );
        let matches = vec![
            CandidateMatch {
                candidate: candidate1,
                score: MatchScore::Exact,
                conversion_costs: vec![],
            },
            CandidateMatch {
                candidate: candidate2,
                score: MatchScore::Exact,
                conversion_costs: vec![],
            },
        ];
        let result = ResolutionResult::with_candidates(matches);
        assert!(result.is_ambiguous());
    }

    #[test]
    fn test_overload_resolver_new() {
        let resolver = OverloadResolver::new();
        assert!(resolver.get_candidates().is_empty());
    }

    #[test]
    fn test_overload_resolver_add_candidate() {
        let mut resolver = OverloadResolver::new();
        let candidate = Candidate::new("foo", vec![], None, Span::new(FileId(0), 0, 0));
        resolver.add_candidate(candidate);
        assert_eq!(resolver.get_candidates().len(), 1);
    }

    #[test]
    fn test_overload_resolver_exact_match() {
        let mut resolver = OverloadResolver::new();
        resolver.add_candidate(Candidate::new(
            "foo",
            vec!["int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        ));
        let result = resolver.resolve(&["int".to_string()]);
        assert!(result.best_candidate.is_some());
    }

    #[test]
    fn test_overload_resolver_no_match() {
        let mut resolver = OverloadResolver::new();
        resolver.add_candidate(Candidate::new(
            "foo",
            vec!["int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        ));
        let result = resolver.resolve(&["string".to_string()]);
        assert!(result.best_candidate.is_none());
    }

    #[test]
    fn test_overload_resolver_selects_best() {
        let mut resolver = OverloadResolver::new();
        resolver.add_candidate(Candidate::new(
            "exact",
            vec!["int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        ));
        resolver.add_candidate(Candidate::new(
            "widened",
            vec!["int64".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        ));
        let result = resolver.resolve(&["int".to_string()]);
        assert!(result.best_candidate.is_some());
        let idx = result.best_candidate.unwrap();
        assert_eq!(result.candidates[idx].candidate.name, "exact");
    }

    #[test]
    fn test_match_score_tie_breaking() {
        let score1 = MatchScore::Exact;
        let score2 = MatchScore::Exact;
        // Same exact match is considered better (not tie-breaking in this impl)
        assert!(score1.is_better_than(&score2));
    }

    #[test]
    fn test_match_score_exact_vs_conversion() {
        let exact = MatchScore::Exact;
        let converted = MatchScore::SafeConversion(1);
        assert!(exact.is_better_than(&converted));
    }

    #[test]
    fn test_candidate_with_defaults_count() {
        let candidate = Candidate::new(
            "with_defaults",
            vec!["x: int".to_string(), "y: int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        );
        // defaults stores count, initialized to 0
        assert_eq!(candidate.defaults, 0);
    }

    #[test]
    fn test_candidate_variadic_marking() {
        let mut candidate = Candidate::new(
            "variadic",
            vec!["x: int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        );
        candidate.is_variadic = true;
        assert!(candidate.is_variadic);
    }

    #[test]
    fn test_candidate_exact_match() {
        let mut resolver = OverloadResolver::new();
        resolver.add_candidate(Candidate::new(
            "exact",
            vec!["int".to_string()],
            None,
            Span::new(FileId(0), 0, 0),
        ));
        let result = resolver.resolve(&["int".to_string()]);
        assert!(result.best_candidate.is_some());
    }

    #[test]
    fn test_resolution_result_no_ambiguity() {
        let result = ResolutionResult::new();
        assert!(!result.is_ambiguous());
    }

    #[test]
    fn test_overload_resolver_empty() {
        let resolver = OverloadResolver::new();
        let result = resolver.resolve(&["int".to_string()]);
        assert!(result.best_candidate.is_none());
    }
}
