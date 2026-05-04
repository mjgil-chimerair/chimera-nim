//! Conditional compilation support for `when` and `defined`.

use rnim_span::Span;
use std::collections::HashMap;

/// Target platform types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetPlatform {
    /// x86 32-bit
    X86,
    /// x86 64-bit
    X86_64,
    /// ARM 32-bit
    Arm,
    /// ARM 64-bit
    Arm64,
    /// Any UNIX-like platform
    Unix,
    /// Windows
    Windows,
    /// macOS
    MacOS,
    /// Linux
    Linux,
    /// JavaScript (via Nim JS backend)
    Js,
    /// Embedded system
    Embedded,
}

/// Target CPU architecture
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetCpu {
    I386,
    Amd64,
    Arm,
    Arm64,
    Js,
}

/// Feature flag state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureState {
    Enabled,
    Disabled,
    Undefined,
}

/// A defined symbol with its value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinedValue {
    Boolean(bool),
    Integer(i64),
    String(String),
}

/// Conditional compilation evaluator
#[derive(Debug, Clone)]
pub struct WhenCtx {
    /// User-defined symbols
    symbols: HashMap<String, DefinedValue>,
    /// Target platform
    target_platform: TargetPlatform,
    /// Target CPU
    target_cpu: TargetCpu,
    /// OS name (e.g., "windows", "linux")
    os_name: String,
    /// CPU architecture name (e.g., "i386", "amd64")
    cpu_name: String,
    /// Compile-time defined constants
    defined_constants: HashMap<String, bool>,
}

impl WhenCtx {
    pub fn new() -> Self {
        let mut symbols = HashMap::new();
        // Add default compile-time constants
        symbols.insert("nimvm".to_string(), DefinedValue::Boolean(false));
        symbols.insert("compiles".to_string(), DefinedValue::Boolean(false));

        WhenCtx {
            symbols,
            target_platform: TargetPlatform::Linux,
            target_cpu: TargetCpu::Amd64,
            os_name: "linux".to_string(),
            cpu_name: "amd64".to_string(),
            defined_constants: HashMap::new(),
        }
    }

    /// Set the target platform
    pub fn set_target(&mut self, platform: TargetPlatform, cpu: TargetCpu) {
        self.target_platform = platform;
        self.target_cpu = cpu;
    }

    /// Set a symbol value
    pub fn set_symbol(&mut self, name: &str, value: DefinedValue) {
        self.symbols.insert(name.to_string(), value);
    }

    /// Get a symbol value
    pub fn get_symbol(&self, name: &str) -> Option<&DefinedValue> {
        self.symbols.get(name)
    }

    /// Check if a symbol is defined (via `defined`)
    pub fn is_defined(&self, name: &str) -> bool {
        // Check built-in defines first
        match name {
            "nimvm" | "compiles" => true,
            "linux" => self.target_platform == TargetPlatform::Linux,
            "windows" => self.target_platform == TargetPlatform::Windows,
            "macosx" | "macos" => self.target_platform == TargetPlatform::MacOS,
            "unix" => matches!(
                self.target_platform,
                TargetPlatform::Unix | TargetPlatform::Linux | TargetPlatform::MacOS
            ),
            "posix" => matches!(
                self.target_platform,
                TargetPlatform::Unix | TargetPlatform::Linux | TargetPlatform::MacOS
            ),
            "x86" => self.target_cpu == TargetCpu::I386,
            "x86_64" | "amd64" => self.target_cpu == TargetCpu::Amd64,
            "arm" => self.target_cpu == TargetCpu::Arm,
            "arm64" => self.target_cpu == TargetCpu::Arm64,
            "js" => self.target_cpu == TargetCpu::Js || self.target_platform == TargetPlatform::Js,
            "embedded" => self.target_platform == TargetPlatform::Embedded,
            _ => self.symbols.contains_key(name),
        }
    }

    /// Evaluate a `defined(name)` expression
    pub fn eval_defined(&self, name: &str) -> bool {
        self.is_defined(name)
    }

    /// Evaluate a condition expression
    pub fn eval_condition(&mut self, cond: &WhenCondition) -> Result<bool, String> {
        match cond {
            WhenCondition::True => Ok(true),
            WhenCondition::False => Ok(false),
            WhenCondition::Defined(name) => Ok(self.is_defined(name)),
            WhenCondition::NotDefined(name) => Ok(!self.is_defined(name)),
            WhenCondition::Eq(left, right) => {
                let left_val = self.get_symbol_value(left)?;
                let right_val = self.get_symbol_value(right)?;
                Ok(left_val == right_val)
            }
            WhenCondition::Ne(left, right) => {
                let left_val = self.get_symbol_value(left)?;
                let right_val = self.get_symbol_value(right)?;
                Ok(left_val != right_val)
            }
            WhenCondition::And(left, right) => {
                let left_val = self.eval_condition(left)?;
                let right_val = self.eval_condition(right)?;
                Ok(left_val && right_val)
            }
            WhenCondition::Or(left, right) => {
                let left_val = self.eval_condition(left)?;
                let right_val = self.eval_condition(right)?;
                Ok(left_val || right_val)
            }
            WhenCondition::Not(inner) => {
                let val = self.eval_condition(inner)?;
                Ok(!val)
            }
            WhenCondition::Parens(inner) => self.eval_condition(inner),
        }
    }

    fn get_symbol_value(&self, name: &str) -> Result<String, String> {
        if let Some(val) = self.symbols.get(name) {
            match val {
                DefinedValue::Boolean(b) => Ok(if *b { "1" } else { "0" }.to_string()),
                DefinedValue::Integer(i) => Ok(i.to_string()),
                DefinedValue::String(s) => Ok(s.clone()),
            }
        } else if self.is_defined(name) {
            Ok("1".to_string())
        } else {
            Err(format!("Undefined symbol: {}", name))
        }
    }

    /// Prune the inactive branch from a when statement
    /// Returns (then_branch, else_branch) where only one is kept
    pub fn prune_when(
        &mut self,
        cond: &WhenCondition,
        then_branch: &[WhenBranchItem],
        else_branch: Option<&[WhenBranchItem]>,
    ) -> Result<PrunedWhen, String> {
        let result = self.eval_condition(cond)?;
        if result {
            Ok(PrunedWhen {
                active_branch: WhenBranch::Then(then_branch.to_vec()),
                inactive_branch: else_branch.map(|b| WhenBranch::Else(b.to_vec())),
            })
        } else if let Some(else_items) = else_branch {
            Ok(PrunedWhen {
                active_branch: WhenBranch::Else(else_items.to_vec()),
                inactive_branch: Some(WhenBranch::Then(then_branch.to_vec())),
            })
        } else {
            Ok(PrunedWhen {
                active_branch: WhenBranch::Else(vec![]),
                inactive_branch: Some(WhenBranch::Then(then_branch.to_vec())),
            })
        }
    }
}

impl Default for WhenCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// Condition for `when` statements
#[derive(Debug, Clone, PartialEq)]
pub enum WhenCondition {
    /// Always true
    True,
    /// Always false
    False,
    /// `defined(symbol)` check
    Defined(String),
    /// `not defined(symbol)` check
    NotDefined(String),
    /// Equality comparison
    Eq(String, String),
    /// Inequality comparison
    Ne(String, String),
    /// Logical AND
    And(Box<WhenCondition>, Box<WhenCondition>),
    /// Logical OR
    Or(Box<WhenCondition>, Box<WhenCondition>),
    /// Logical NOT
    Not(Box<WhenCondition>),
    /// Parenthesized condition
    Parens(Box<WhenCondition>),
}

/// A single item in a when branch
#[derive(Debug, Clone, PartialEq)]
pub struct WhenBranchItem {
    pub span: Span,
    pub text: String,
}

/// The result of pruning a when statement
#[derive(Debug, Clone)]
pub enum WhenBranch {
    Then(Vec<WhenBranchItem>),
    Else(Vec<WhenBranchItem>),
}

/// Result of pruning showing which branch is active
#[derive(Debug, Clone)]
pub struct PrunedWhen {
    pub active_branch: WhenBranch,
    pub inactive_branch: Option<WhenBranch>,
}

/// Evaluate a compile-time constant expression
pub fn eval_ct_expr(expr: &CtfeExpr) -> Result<CtfeValue, String> {
    match expr {
        CtfeExpr::Literal(lit) => Ok(CtfeValue::Integer(*lit)),
        CtfeExpr::Add(a, b) => {
            let a_val = eval_ct_expr(a)?;
            let b_val = eval_ct_expr(b)?;
            match (a_val, b_val) {
                (CtfeValue::Integer(ai), CtfeValue::Integer(bi)) => Ok(CtfeValue::Integer(ai + bi)),
                _ => Err("Invalid types for add".to_string()),
            }
        }
        CtfeExpr::Sub(a, b) => {
            let a_val = eval_ct_expr(a)?;
            let b_val = eval_ct_expr(b)?;
            match (a_val, b_val) {
                (CtfeValue::Integer(ai), CtfeValue::Integer(bi)) => Ok(CtfeValue::Integer(ai - bi)),
                _ => Err("Invalid types for sub".to_string()),
            }
        }
        CtfeExpr::Mul(a, b) => {
            let a_val = eval_ct_expr(a)?;
            let b_val = eval_ct_expr(b)?;
            match (a_val, b_val) {
                (CtfeValue::Integer(ai), CtfeValue::Integer(bi)) => Ok(CtfeValue::Integer(ai * bi)),
                _ => Err("Invalid types for mul".to_string()),
            }
        }
        CtfeExpr::Div(a, b) => {
            let a_val = eval_ct_expr(a)?;
            let b_val = eval_ct_expr(b)?;
            match (a_val, b_val) {
                (CtfeValue::Integer(ai), CtfeValue::Integer(bi)) if bi != 0 => {
                    Ok(CtfeValue::Integer(ai / bi))
                }
                _ => Err("Division by zero or invalid types".to_string()),
            }
        }
        CtfeExpr::Mod(a, b) => {
            let a_val = eval_ct_expr(a)?;
            let b_val = eval_ct_expr(b)?;
            match (a_val, b_val) {
                (CtfeValue::Integer(ai), CtfeValue::Integer(bi)) if bi != 0 => {
                    Ok(CtfeValue::Integer(ai % bi))
                }
                _ => Err("Modulo by zero or invalid types".to_string()),
            }
        }
        CtfeExpr::Neg(a) => {
            let a_val = eval_ct_expr(a)?;
            match a_val {
                CtfeValue::Integer(i) => Ok(CtfeValue::Integer(-i)),
                _ => Err("Invalid type for neg".to_string()),
            }
        }
        CtfeExpr::Ident(name) => {
            // For now, treat identifiers as integers for compile-time constants
            // A full implementation would look up symbol tables
            Ok(CtfeValue::Integer(0))
        }
    }
}

/// Compile-time evaluable expression
#[derive(Debug, Clone, PartialEq)]
pub enum CtfeExpr {
    /// Integer literal
    Literal(i64),
    /// Addition
    Add(Box<CtfeExpr>, Box<CtfeExpr>),
    /// Subtraction
    Sub(Box<CtfeExpr>, Box<CtfeExpr>),
    /// Multiplication
    Mul(Box<CtfeExpr>, Box<CtfeExpr>),
    /// Division
    Div(Box<CtfeExpr>, Box<CtfeExpr>),
    /// Modulo
    Mod(Box<CtfeExpr>, Box<CtfeExpr>),
    /// Negation
    Neg(Box<CtfeExpr>),
    /// Identifier reference
    Ident(String),
}

/// Compile-time evaluable value
#[derive(Debug, Clone, PartialEq)]
pub enum CtfeValue {
    Integer(i64),
    String(String),
    Boolean(bool),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_when_ctx_new() {
        let ctx = WhenCtx::new();
        assert!(ctx.is_defined("nimvm"));
        assert!(ctx.is_defined("compiles"));
    }

    #[test]
    fn test_is_defined_linux() {
        let ctx = WhenCtx::new();
        assert!(ctx.is_defined("linux"));
    }

    #[test]
    fn test_is_defined_windows() {
        let mut ctx = WhenCtx::new();
        ctx.set_target(TargetPlatform::Windows, TargetCpu::I386);
        assert!(ctx.is_defined("windows"));
    }

    #[test]
    fn test_is_defined_x86() {
        let ctx = WhenCtx::new();
        assert!(ctx.is_defined("x86_64"));
    }

    #[test]
    fn test_set_symbol() {
        let mut ctx = WhenCtx::new();
        ctx.set_symbol("myFlag", DefinedValue::Boolean(true));
        assert!(ctx.is_defined("myFlag"));
    }

    #[test]
    fn test_eval_condition_defined() {
        let mut ctx = WhenCtx::new();
        ctx.set_symbol("testFlag", DefinedValue::Boolean(true));
        let cond = WhenCondition::Defined("testFlag".to_string());
        assert!(ctx.eval_condition(&cond).unwrap());
    }

    #[test]
    fn test_eval_condition_not_defined() {
        let mut ctx = WhenCtx::new();
        let cond = WhenCondition::NotDefined("nonexistent".to_string());
        assert!(ctx.eval_condition(&cond).unwrap());
    }

    #[test]
    fn test_eval_condition_and() {
        let mut ctx = WhenCtx::new();
        ctx.set_symbol("a", DefinedValue::Boolean(true));
        ctx.set_symbol("b", DefinedValue::Boolean(true));
        let cond = WhenCondition::And(
            Box::new(WhenCondition::Defined("a".to_string())),
            Box::new(WhenCondition::Defined("b".to_string())),
        );
        assert!(ctx.eval_condition(&cond).unwrap());
    }

    #[test]
    fn test_eval_condition_or() {
        let mut ctx = WhenCtx::new();
        let cond = WhenCondition::Or(
            Box::new(WhenCondition::Defined("linux".to_string())),
            Box::new(WhenCondition::Defined("windows".to_string())),
        );
        assert!(ctx.eval_condition(&cond).unwrap());
    }

    #[test]
    fn test_eval_condition_not() {
        let mut ctx = WhenCtx::new();
        let cond = WhenCondition::Not(Box::new(WhenCondition::Defined("nonexistent".to_string())));
        assert!(ctx.eval_condition(&cond).unwrap());
    }

    #[test]
    fn test_prune_when_then_branch() {
        let mut ctx = WhenCtx::new();
        ctx.set_symbol("testFlag", DefinedValue::Boolean(true));
        let cond = WhenCondition::Defined("testFlag".to_string());
        let then_items = vec![WhenBranchItem {
            span: Span::new(FileId(0), 0, 0),
            text: "echo 1".to_string(),
        }];
        let else_items = vec![WhenBranchItem {
            span: Span::new(FileId(0), 0, 0),
            text: "echo 2".to_string(),
        }];
        let result = ctx.prune_when(&cond, &then_items, Some(&else_items));
        assert!(result.is_ok());
        let pruned = result.unwrap();
        match pruned.active_branch {
            WhenBranch::Then(_) => {}
            _ => panic!("Expected Then branch"),
        }
    }

    #[test]
    fn test_prune_when_condition_false() {
        let mut ctx = WhenCtx::new();
        // Condition is false (nonexistent is NOT defined, so NotDefined is false)
        let cond = WhenCondition::Defined("nonexistent".to_string());
        let then_items = vec![WhenBranchItem {
            span: Span::new(FileId(0), 0, 0),
            text: "echo 1".to_string(),
        }];
        let else_items = vec![WhenBranchItem {
            span: Span::new(FileId(0), 0, 0),
            text: "echo 2".to_string(),
        }];
        let result = ctx.prune_when(&cond, &then_items, Some(&else_items));
        assert!(result.is_ok());
        let pruned = result.unwrap();
        // Defined("nonexistent") is false, so Else branch should be active
        match pruned.active_branch {
            WhenBranch::Else(_) => {}
            _ => panic!("Expected Else branch when condition is false"),
        }
    }

    #[test]
    fn test_eval_ct_expr_literal() {
        let expr = CtfeExpr::Literal(42);
        let result = eval_ct_expr(&expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CtfeValue::Integer(42));
    }

    #[test]
    fn test_eval_ct_expr_add() {
        let expr = CtfeExpr::Add(
            Box::new(CtfeExpr::Literal(10)),
            Box::new(CtfeExpr::Literal(20)),
        );
        let result = eval_ct_expr(&expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CtfeValue::Integer(30));
    }

    #[test]
    fn test_eval_ct_expr_mul() {
        let expr = CtfeExpr::Mul(
            Box::new(CtfeExpr::Literal(6)),
            Box::new(CtfeExpr::Literal(7)),
        );
        let result = eval_ct_expr(&expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CtfeValue::Integer(42));
    }

    #[test]
    fn test_eval_ct_expr_div() {
        let expr = CtfeExpr::Div(
            Box::new(CtfeExpr::Literal(20)),
            Box::new(CtfeExpr::Literal(4)),
        );
        let result = eval_ct_expr(&expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CtfeValue::Integer(5));
    }

    #[test]
    fn test_eval_ct_expr_mod() {
        let expr = CtfeExpr::Mod(
            Box::new(CtfeExpr::Literal(17)),
            Box::new(CtfeExpr::Literal(5)),
        );
        let result = eval_ct_expr(&expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CtfeValue::Integer(2));
    }

    #[test]
    fn test_eval_ct_expr_neg() {
        let expr = CtfeExpr::Neg(Box::new(CtfeExpr::Literal(42)));
        let result = eval_ct_expr(&expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CtfeValue::Integer(-42));
    }

    #[test]
    fn test_eval_ct_expr_complex() {
        // ((10 + 20) * 2) / 3 = 20
        let expr = CtfeExpr::Div(
            Box::new(CtfeExpr::Mul(
                Box::new(CtfeExpr::Add(
                    Box::new(CtfeExpr::Literal(10)),
                    Box::new(CtfeExpr::Literal(20)),
                )),
                Box::new(CtfeExpr::Literal(2)),
            )),
            Box::new(CtfeExpr::Literal(3)),
        );
        let result = eval_ct_expr(&expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CtfeValue::Integer(20));
    }

    #[test]
    fn test_eval_ct_expr_div_by_zero() {
        let expr = CtfeExpr::Div(
            Box::new(CtfeExpr::Literal(1)),
            Box::new(CtfeExpr::Literal(0)),
        );
        let result = eval_ct_expr(&expr);
        assert!(result.is_err());
    }

    // Task 34: Conditional compilation tests

    #[test]
    fn test_user_defined_symbol_int() {
        // Test user-defined integer symbol evaluation
        let mut ctx = WhenCtx::new();
        ctx.set_symbol("MY_VERSION", DefinedValue::Integer(42));
        // Just test that the symbol is defined with correct value
        assert!(ctx.is_defined("MY_VERSION"));
        assert_eq!(ctx.get_symbol("MY_VERSION"), Some(&DefinedValue::Integer(42)));
    }

    #[test]
    fn test_user_defined_symbol_string() {
        // Test user-defined string symbol
        let mut ctx = WhenCtx::new();
        ctx.set_symbol("MY_OS", DefinedValue::String("linux".to_string()));
        assert!(ctx.is_defined("MY_OS"));
    }

    #[test]
    fn test_multiple_and_conditions() {
        // Test multiple AND conditions
        let mut ctx = WhenCtx::new();
        ctx.set_symbol("a", DefinedValue::Boolean(true));
        ctx.set_symbol("b", DefinedValue::Boolean(true));
        ctx.set_symbol("c", DefinedValue::Boolean(true));
        let cond = WhenCondition::And(
            Box::new(WhenCondition::And(
                Box::new(WhenCondition::Defined("a".to_string())),
                Box::new(WhenCondition::Defined("b".to_string())),
            )),
            Box::new(WhenCondition::Defined("c".to_string())),
        );
        assert!(ctx.eval_condition(&cond).unwrap());
    }

    #[test]
    fn test_multiple_or_conditions() {
        // Test multiple OR conditions
        let mut ctx = WhenCtx::new();
        ctx.set_symbol("a", DefinedValue::Boolean(false));
        ctx.set_symbol("b", DefinedValue::Boolean(false));
        ctx.set_symbol("c", DefinedValue::Boolean(true));
        let cond = WhenCondition::Or(
            Box::new(WhenCondition::Or(
                Box::new(WhenCondition::Defined("a".to_string())),
                Box::new(WhenCondition::Defined("b".to_string())),
            )),
            Box::new(WhenCondition::Defined("c".to_string())),
        );
        assert!(ctx.eval_condition(&cond).unwrap());
    }

    #[test]
    fn test_nested_not_and_or() {
        // Test complex nested conditions: not(a and b) or c
        let mut ctx = WhenCtx::new();
        ctx.set_symbol("a", DefinedValue::Boolean(true));
        ctx.set_symbol("b", DefinedValue::Boolean(false));
        ctx.set_symbol("c", DefinedValue::Boolean(true));
        // not(true and false) or true = true or true = true
        let cond = WhenCondition::Or(
            Box::new(WhenCondition::Not(Box::new(WhenCondition::And(
                Box::new(WhenCondition::Defined("a".to_string())),
                Box::new(WhenCondition::Defined("b".to_string())),
            )))),
            Box::new(WhenCondition::Defined("c".to_string())),
        );
        assert!(ctx.eval_condition(&cond).unwrap());
    }

    #[test]
    fn test_target_platform_detection() {
        // Test that target platform symbols are defined
        let ctx = WhenCtx::new();
        // Default target should have some platform defined
        assert!(ctx.is_defined("linux") || ctx.is_defined("windows") || ctx.is_defined("macos"));
    }

    #[test]
    fn test_prune_when_only_then_branch() {
        // Test pruning when there's no else branch
        let mut ctx = WhenCtx::new();
        ctx.set_symbol("testFlag", DefinedValue::Boolean(true));
        let cond = WhenCondition::Defined("testFlag".to_string());
        let then_items = vec![WhenBranchItem {
            span: Span::new(FileId(0), 0, 0),
            text: "echo 1".to_string(),
        }];
        let result = ctx.prune_when(&cond, &then_items, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_ct_expr_sub() {
        // Test subtraction in CTFE
        let expr = CtfeExpr::Sub(
            Box::new(CtfeExpr::Literal(100)),
            Box::new(CtfeExpr::Literal(37)),
        );
        let result = eval_ct_expr(&expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CtfeValue::Integer(63));
    }

    #[test]
    fn test_when_ctx_clone() {
        // Test that WhenCtx can be cloned
        let ctx = WhenCtx::new();
        let cloned = ctx.clone();
        assert!(cloned.is_defined("nimvm"));
    }

    #[test]
    fn test_when_ctx_target_info() {
        // Test getting target information
        let ctx = WhenCtx::new();
        assert!(matches!(ctx.target_platform, TargetPlatform::Linux));
        assert!(matches!(ctx.target_cpu, TargetCpu::Amd64));
    }
}
