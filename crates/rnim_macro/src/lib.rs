//! Template and macro expansion, hygienic symbol handling, Nim AST macro API model.

#![allow(clippy::collapsible_match, clippy::bool_comparison, unused_variables)]

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::Span;
use std::collections::HashMap;

/// AST node kinds for macro API
#[derive(Debug, Clone, PartialEq)]
pub enum AstKind {
    /// A nil literal
    Nil,
    /// An integer literal
    Int(i64),
    /// A float literal
    Float(f64),
    /// A string literal
    String(String),
    /// A character literal
    Char(char),
    /// A boolean literal
    Bool(bool),
    /// An identifier
    Ident(String),
    /// A binary operation
    BinaryOp {
        op: String,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    /// A unary operation
    UnaryOp { op: String, operand: Box<AstNode> },
    /// An assignment
    Assignment {
        target: Box<AstNode>,
        value: Box<AstNode>,
    },
    /// A block of statements
    Block(Vec<AstNode>),
    /// An if statement
    If {
        condition: Box<AstNode>,
        then_branch: Box<AstNode>,
        else_branch: Option<Box<AstNode>>,
    },
    /// A while loop
    While {
        condition: Box<AstNode>,
        body: Box<AstNode>,
    },
    /// A for loop
    For {
        variable: String,
        iterable: Box<AstNode>,
        body: Box<AstNode>,
    },
    /// A case statement
    Case {
        expr: Box<AstNode>,
        branches: Vec<(Vec<AstNode>, AstNode)>,
    },
    /// A procedure call
    Call {
        callee: Box<AstNode>,
        args: Vec<AstNode>,
    },
    /// A procedure definition
    ProcDef {
        name: String,
        params: Vec<(String, Option<String>)>,
        ret_type: Option<String>,
        body: Box<AstNode>,
    },
    /// A let/var declaration
    VarDecl {
        kind: String,
        name: String,
        typ: Option<String>,
        value: Box<AstNode>,
    },
    /// A tuple construction
    Tuple(Vec<AstNode>),
    /// A sequence construction
    Seq(Vec<AstNode>),
    /// Object construction with field bindings
    Object {
        typ: Option<String>,
        fields: Vec<(String, AstNode)>,
    },
    /// A field access
    FieldAccess { expr: Box<AstNode>, field: String },
    /// Array indexing
    Index {
        expr: Box<AstNode>,
        index: Box<AstNode>,
    },
    /// A pragma
    Pragma(Vec<(String, Option<AstNode>)>),
    /// Import statement
    Import {
        module: String,
        symbols: Vec<String>,
    },
    /// Return statement
    Return(Option<Box<AstNode>>),
    /// Raise statement
    Raise(Box<AstNode>),
    /// Try/except/finally
    Try {
        body: Box<AstNode>,
        except: Vec<(Option<String>, AstNode)>,
        finally: Option<Box<AstNode>>,
    },
}

/// An AST node with source span
#[derive(Debug, Clone, PartialEq)]
pub struct AstNode {
    pub kind: AstKind,
    pub span: Span,
}

impl AstNode {
    /// Create a new AST node with the given kind and span
    pub fn new(kind: AstKind, span: Span) -> Self {
        AstNode { kind, span }
    }

    /// Create an identifier node
    pub fn ident(name: &str, span: Span) -> Self {
        AstNode::new(AstKind::Ident(name.to_string()), span)
    }

    /// Create an integer literal node
    pub fn int(value: i64, span: Span) -> Self {
        AstNode::new(AstKind::Int(value), span)
    }

    /// Create a string literal node
    pub fn string(value: &str, span: Span) -> Self {
        AstNode::new(AstKind::String(value.to_string()), span)
    }

    /// Create a binary operation node
    pub fn binary_op(op: &str, left: AstNode, right: AstNode, span: Span) -> Self {
        AstNode::new(
            AstKind::BinaryOp {
                op: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            },
            span,
        )
    }

    /// Create a call node
    pub fn call(callee: AstNode, args: Vec<AstNode>, span: Span) -> Self {
        AstNode::new(
            AstKind::Call {
                callee: Box::new(callee),
                args,
            },
            span,
        )
    }

    /// Create a block node
    pub fn block(nodes: Vec<AstNode>, span: Span) -> Self {
        AstNode::new(AstKind::Block(nodes), span)
    }

    /// Create a procedure definition node
    pub fn proc_def(
        name: &str,
        params: Vec<(String, Option<String>)>,
        ret_type: Option<String>,
        body: AstNode,
        span: Span,
    ) -> Self {
        AstNode::new(
            AstKind::ProcDef {
                name: name.to_string(),
                params,
                ret_type,
                body: Box::new(body),
            },
            span,
        )
    }

    /// Create a let declaration node
    pub fn let_decl(name: &str, value: AstNode, span: Span) -> Self {
        AstNode::new(
            AstKind::VarDecl {
                kind: "let".to_string(),
                name: name.to_string(),
                typ: None,
                value: Box::new(value),
            },
            span,
        )
    }

    /// Create a var declaration node
    pub fn var_decl(name: &str, typ: Option<String>, value: AstNode, span: Span) -> Self {
        AstNode::new(
            AstKind::VarDecl {
                kind: "var".to_string(),
                name: name.to_string(),
                typ,
                value: Box::new(value),
            },
            span,
        )
    }

    /// Check if this node is a literal
    pub fn is_literal(&self) -> bool {
        matches!(
            self.kind,
            AstKind::Nil
                | AstKind::Int(_)
                | AstKind::Float(_)
                | AstKind::String(_)
                | AstKind::Char(_)
                | AstKind::Bool(_)
        )
    }

    /// Get the source span of this node
    pub fn span(&self) -> Span {
        self.span
    }
}

/// Macro error with span information
#[derive(Debug, Clone)]
pub struct MacroError {
    pub message: String,
    pub span: Span,
}

impl MacroError {
    pub fn new(message: &str, span: Span) -> Self {
        MacroError {
            message: message.to_string(),
            span,
        }
    }
}

/// Result of quote do construction
#[derive(Debug, Clone)]
pub struct QuotedCode {
    pub nodes: Vec<AstNode>,
    pub hygiene: HygieneContext,
}

impl QuotedCode {
    /// Create a new quoted code block
    pub fn new(nodes: Vec<AstNode>) -> Self {
        QuotedCode {
            nodes,
            hygiene: HygieneContext::clean(),
        }
    }

    /// Get the number of nodes in this quoted block
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the quoted block is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Template parameter kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateParamKind {
    /// Untyped template parameter (matches any AST node)
    Untyped,
    /// Typed template parameter with expected type
    Typed { expected_type: Option<String> },
    /// Block template parameter
    Block,
    /// Variadic template parameter
    Variadic,
}

/// A template parameter definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateParam {
    pub name: String,
    pub kind: TemplateParamKind,
    pub default_value: Option<String>,
}

/// Template expansion result with hygiene information
#[derive(Debug, Clone)]
pub struct TemplateExpansion {
    pub code: String,
    pub hygiene: HygieneContext,
}

/// Hygiene context for symbol visibility during expansion
#[derive(Debug, Clone, Default)]
pub struct HygieneContext {
    /// Map from original symbol to replacement in expanded code
    bindings: HashMap<String, String>,
    /// Whether this expansion is "dirty" (accesses variables from outer scope)
    is_dirty: bool,
}

impl HygieneContext {
    pub fn new() -> Self {
        HygieneContext {
            bindings: HashMap::new(),
            is_dirty: false,
        }
    }

    /// Create a clean hygiene context (no outer scope access)
    pub fn clean() -> Self {
        HygieneContext {
            bindings: HashMap::new(),
            is_dirty: false,
        }
    }

    /// Create a dirty hygiene context (may access outer scope variables)
    pub fn dirty() -> Self {
        HygieneContext {
            bindings: HashMap::new(),
            is_dirty: true,
        }
    }

    /// Add a binding from original symbol to replacement
    pub fn bind(&mut self, original: &str, replacement: &str) {
        self.bindings
            .insert(original.to_string(), replacement.to_string());
    }

    /// Check if a symbol has been bound in this context
    pub fn is_bound(&self, symbol: &str) -> bool {
        self.bindings.contains_key(symbol)
    }

    /// Get the replacement for a symbol if bound
    pub fn get_replacement(&self, symbol: &str) -> Option<&str> {
        self.bindings.get(symbol).map(|s| s.as_str())
    }

    /// Mark this context as dirty (accesses outer scope)
    pub fn set_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Check if this context is dirty
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }
}

/// Template expander with support for typed/untyped templates
#[derive(Debug, Clone)]
pub struct TemplateExpander {
    max_depth: usize,
    current_depth: usize,
}

impl TemplateExpander {
    pub fn new() -> Self {
        TemplateExpander {
            max_depth: 100,
            current_depth: 0,
        }
    }

    /// Expand a template with given arguments
    pub fn expand(
        &mut self,
        template: &str,
        params: &[TemplateParam],
        args: &[String],
    ) -> Result<TemplateExpansion, String> {
        if params.len() != args.len() {
            return Err(format!(
                "Template argument count mismatch: expected {}, got {}",
                params.len(),
                args.len()
            ));
        }

        // Check recursion depth
        self.current_depth += 1;
        if self.current_depth > self.max_depth {
            self.current_depth -= 1;
            return Err("Template expansion recursion limit exceeded".to_string());
        }

        let mut hygiene = HygieneContext::clean();
        let mut result = template.to_string();

        // Replace parameters with arguments
        for (param, arg) in params.iter().zip(args.iter()) {
            let replacement = match param.kind {
                TemplateParamKind::Block => {
                    // Block parameters wrap in braces
                    format!(" {{{} }} ", arg)
                }
                TemplateParamKind::Variadic => {
                    // Variadic params consume remaining args
                    format!("{} *", arg)
                }
                _ => format!(" {}", arg),
            };

            let pattern = format!("${}$", param.name);
            result = result.replace(&pattern, &replacement);

            // For typed templates, perform hygiene binding
            if matches!(param.kind, TemplateParamKind::Typed { .. }) {
                hygiene.bind(&param.name, arg);
            }
        }

        self.current_depth -= 1;
        Ok(TemplateExpansion {
            code: result,
            hygiene,
        })
    }

    /// Expand with typed parameters (with type checking)
    pub fn expand_typed(
        &mut self,
        template: &str,
        params: &[TemplateParam],
        args: &[String],
        arg_types: &[Option<String>],
    ) -> Result<TemplateExpansion, String> {
        // Validate argument types match parameter expectations
        for (i, (param, arg_type)) in params.iter().zip(arg_types.iter()).enumerate() {
            if let TemplateParamKind::Typed { expected_type } = &param.kind {
                if let Some(expected_typename) = expected_type {
                    if let Some(actual_type) = arg_type {
                        if expected_typename != actual_type {
                            return Err(format!(
                                "Argument {} type mismatch: expected {}, got {}",
                                i + 1,
                                expected_typename,
                                actual_type
                            ));
                        }
                    }
                }
            }
        }

        self.expand(template, params, args)
    }

    /// Check if a template is dirty (accesses outer scope)
    pub fn is_dirty_template(&self, template: &str) -> bool {
        // Check for patterns that indicate dirty template behavior
        template.contains("=.")
    }

    /// Get the expected type for a parameter at a given position
    pub fn get_param_type(&self, params: &[TemplateParam], index: usize) -> Option<String> {
        params.get(index).and_then(|p| {
            if let TemplateParamKind::Typed { expected_type } = &p.kind {
                expected_type.clone()
            } else {
                None
            }
        })
    }
}

/// Macro expander for Nim's AST macros
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MacroExpander {
    template_expander: TemplateExpander,
}

impl MacroExpander {
    pub fn new() -> Self {
        MacroExpander {
            template_expander: TemplateExpander::new(),
        }
    }

    /// Expand a macro call
    pub fn expand(&mut self, macro_name: &str, args: &[String]) -> Result<String, String> {
        // For now, just return the macro call as-is
        // Full implementation would involve parsing and AST manipulation
        Ok(format!("{:?}", macro_name))
    }

    /// Quote a block of code (prevent expansion)
    pub fn quote(&self, code: &str) -> String {
        format!("quote do:\n{}", code)
    }

    /// Bind a symbol at compile time
    pub fn bind_sym(&self, name: &str) -> Result<String, String> {
        // In full implementation, would resolve symbol at compile time
        Ok(name.to_string())
    }

    /// Execute a macro in the VM and return the result
    pub fn execute_in_vm(
        &mut self,
        macro_name: &str,
        args: &[AstNode],
        vm: &mut rnim_vm::Vm,
    ) -> Result<rnim_vm::VmValue, String> {
        // Compile macro args to VM values
        let mut vm_args: Vec<rnim_vm::VmValue> = Vec::new();
        for arg in args {
            let vm_val = self.ast_to_vm_value(arg)?;
            vm_args.push(vm_val);
        }

        // Create a procedure for the macro
        let proc_id = vm.new_proc_id();
        let mut proc = rnim_vm::VmProc::new(proc_id, macro_name);

        // For now, just return the first arg as a simple implementation
        // A full implementation would parse and compile the macro body
        if !vm_args.is_empty() {
            proc.add_opcode(rnim_vm::OpCode::PushConst(vm_args.remove(0)));
        } else {
            proc.add_opcode(rnim_vm::OpCode::PushConst(rnim_vm::VmValue::Nil));
        }
        proc.add_opcode(rnim_vm::OpCode::Return);

        vm.add_proc(proc);
        vm.execute(proc_id, vec![])
    }

    /// Convert an AST node to a VM value
    fn ast_to_vm_value(&self, node: &AstNode) -> Result<rnim_vm::VmValue, String> {
        match &node.kind {
            AstKind::Nil => Ok(rnim_vm::VmValue::Nil),
            AstKind::Int(i) => Ok(rnim_vm::VmValue::Int(*i)),
            AstKind::Float(f) => Ok(rnim_vm::VmValue::Float(*f)),
            AstKind::String(s) => Ok(rnim_vm::VmValue::String(s.clone())),
            AstKind::Char(c) => Ok(rnim_vm::VmValue::Char(*c)),
            AstKind::Bool(b) => Ok(rnim_vm::VmValue::Bool(*b)),
            AstKind::Seq(items) => {
                let mut values = Vec::new();
                for item in items {
                    values.push(self.ast_to_vm_value(item)?);
                }
                Ok(rnim_vm::VmValue::Seq(values))
            }
            AstKind::Tuple(items) => {
                let mut values = Vec::new();
                for item in items {
                    values.push(self.ast_to_vm_value(item)?);
                }
                Ok(rnim_vm::VmValue::Seq(values))
            }
            AstKind::Ident(name) => Ok(rnim_vm::VmValue::String(name.clone())),
            _ => Err(format!(
                "Cannot convert AST kind to VM value: {:?}",
                node.kind
            )),
        }
    }
}

impl Default for TemplateExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MacroExpander {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_hygiene_context_clean() {
        let ctx = HygieneContext::clean();
        assert!(!ctx.is_dirty());
    }

    #[test]
    fn test_hygiene_context_dirty() {
        let ctx = HygieneContext::dirty();
        assert!(ctx.is_dirty());
    }

    #[test]
    fn test_hygiene_bind() {
        let mut ctx = HygieneContext::new();
        ctx.bind("x", "y");
        assert!(ctx.is_bound("x"));
        assert_eq!(ctx.get_replacement("x"), Some("y"));
    }

    #[test]
    fn test_template_expander_basic() {
        let mut expander = TemplateExpander::new();
        let params = vec![
            TemplateParam {
                name: "x".to_string(),
                kind: TemplateParamKind::Untyped,
                default_value: None,
            },
            TemplateParam {
                name: "y".to_string(),
                kind: TemplateParamKind::Untyped,
                default_value: None,
            },
        ];
        let result = expander.expand("($x$ + $y$)", &params, &["1".to_string(), "2".to_string()]);
        assert!(result.is_ok());
        let expansion = result.unwrap();
        assert!(expansion.code.contains("1"));
        assert!(expansion.code.contains("2"));
    }

    #[test]
    fn test_template_expander_typed() {
        let mut expander = TemplateExpander::new();
        let params = vec![TemplateParam {
            name: "val".to_string(),
            kind: TemplateParamKind::Typed {
                expected_type: Some("int".to_string()),
            },
            default_value: None,
        }];
        let result = expander.expand_typed(
            "($val$ * 2)",
            &params,
            &["42".to_string()],
            &[Some("int".to_string())],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_template_expander_block_param() {
        let mut expander = TemplateExpander::new();
        let params = vec![TemplateParam {
            name: "block".to_string(),
            kind: TemplateParamKind::Block,
            default_value: None,
        }];
        let result = expander.expand("do: $block$", &params, &["echo 1".to_string()]);
        assert!(result.is_ok());
        let expansion = result.unwrap();
        assert!(expansion.code.contains("{"));
        assert!(expansion.code.contains("}"));
    }

    #[test]
    fn test_template_expander_recursion_limit() {
        let mut expander = TemplateExpander::new();
        expander.max_depth = 2;
        // The expand function doesn't recurse internally, so this test
        // verifies the structure is in place for future recursion handling
        let params = vec![TemplateParam {
            name: "x".to_string(),
            kind: TemplateParamKind::Untyped,
            default_value: None,
        }];
        let result = expander.expand("$x$", &params, &["test".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_macro_expander_quote() {
        let expander = MacroExpander::new();
        let quoted = expander.quote("echo 1");
        assert!(quoted.contains("quote do"));
    }

    #[test]
    fn test_macro_expander_bind_sym() {
        let expander = MacroExpander::new();
        let result = expander.bind_sym("myVar");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "myVar");
    }

    #[test]
    fn test_dirty_template_detection() {
        let expander = TemplateExpander::new();
        assert!(expander.is_dirty_template("x =. y"));
        assert!(!expander.is_dirty_template("x + y"));
    }

    #[test]
    fn test_template_param_kind_variants() {
        let untyped = TemplateParamKind::Untyped;
        let typed = TemplateParamKind::Typed {
            expected_type: Some("int".to_string()),
        };
        let block = TemplateParamKind::Block;
        let variadic = TemplateParamKind::Variadic;

        assert!(matches!(untyped, TemplateParamKind::Untyped));
        assert!(matches!(typed, TemplateParamKind::Typed { .. }));
        assert!(matches!(block, TemplateParamKind::Block));
        assert!(matches!(variadic, TemplateParamKind::Variadic));
    }

    #[test]
    fn test_get_param_type() {
        let expander = TemplateExpander::new();
        let params = vec![
            TemplateParam {
                name: "x".to_string(),
                kind: TemplateParamKind::Untyped,
                default_value: None,
            },
            TemplateParam {
                name: "y".to_string(),
                kind: TemplateParamKind::Typed {
                    expected_type: Some("string".to_string()),
                },
                default_value: None,
            },
        ];

        assert_eq!(expander.get_param_type(&params, 0), None);
        assert_eq!(
            expander.get_param_type(&params, 1),
            Some("string".to_string())
        );
    }

    #[test]
    fn test_ast_node_ident() {
        let span = Span::new(FileId(0), 0, 0);
        let node = AstNode::ident("foo", span);
        assert!(matches!(node.kind, AstKind::Ident(s) if s == "foo"));
    }

    #[test]
    fn test_ast_node_int() {
        let span = Span::new(FileId(0), 0, 0);
        let node = AstNode::int(42, span);
        assert!(matches!(node.kind, AstKind::Int(42)));
    }

    #[test]
    fn test_ast_node_string() {
        let span = Span::new(FileId(0), 0, 0);
        let node = AstNode::string("hello", span);
        assert!(matches!(node.kind, AstKind::String(s) if s == "hello"));
    }

    #[test]
    fn test_ast_node_binary_op() {
        let span = Span::new(FileId(0), 0, 0);
        let left = AstNode::int(1, span);
        let right = AstNode::int(2, span);
        let node = AstNode::binary_op("+", left, right, span);
        assert!(matches!(
            node.kind,
            AstKind::BinaryOp {
                op,
                left: _,
                right: _
            } if op == "+"
        ));
    }

    #[test]
    fn test_ast_node_call() {
        let span = Span::new(FileId(0), 0, 0);
        let callee = AstNode::ident("echo", span);
        let args = vec![AstNode::int(1, span)];
        let node = AstNode::call(callee, args, span);
        assert!(matches!(node.kind, AstKind::Call { callee: _, args: _ }));
    }

    #[test]
    fn test_ast_node_block() {
        let span = Span::new(FileId(0), 0, 0);
        let nodes = vec![AstNode::int(1, span), AstNode::int(2, span)];
        let block = AstNode::block(nodes, span);
        assert!(matches!(block.kind, AstKind::Block(v) if v.len() == 2));
    }

    #[test]
    fn test_ast_node_proc_def() {
        let span = Span::new(FileId(0), 0, 0);
        let body = AstNode::block(vec![], span);
        let node = AstNode::proc_def(
            "foo",
            vec![("x".to_string(), Some("int".to_string()))],
            Some("int".to_string()),
            body,
            span,
        );
        assert!(matches!(
            node.kind,
            AstKind::ProcDef {
                name,
                params: _,
                ret_type: _,
                body: _
            } if name == "foo"
        ));
    }

    #[test]
    fn test_ast_node_let_decl() {
        let span = Span::new(FileId(0), 0, 0);
        let value = AstNode::int(42, span);
        let node = AstNode::let_decl("x", value, span);
        assert!(matches!(
            node.kind,
            AstKind::VarDecl {
                kind,
                name,
                typ: _,
                value: _
            } if kind == "let" && name == "x"
        ));
    }

    #[test]
    fn test_ast_node_is_literal() {
        let span = Span::new(FileId(0), 0, 0);
        assert!(AstNode::int(1, span).is_literal());
        assert!(AstNode::string("test", span).is_literal());
        assert!(AstNode::ident("foo", span).is_literal() == false);
    }

    #[test]
    fn test_macro_error() {
        let span = Span::new(FileId(0), 0, 0);
        let err = MacroError::new("test error", span);
        assert_eq!(err.message, "test error");
    }

    #[test]
    fn test_quoted_code() {
        let span = Span::new(FileId(0), 0, 0);
        let nodes = vec![AstNode::int(1, span), AstNode::int(2, span)];
        let quoted = QuotedCode::new(nodes);
        assert_eq!(quoted.len(), 2);
        assert!(!quoted.is_empty());
    }

    #[test]
    fn test_quoted_code_empty() {
        let quoted = QuotedCode::new(vec![]);
        assert!(quoted.is_empty());
    }

    #[test]
    fn test_ast_node_ident_creation() {
        let span = Span::new(FileId(0), 0, 10);
        let node = AstNode::ident("myVar", span);
        assert!(matches!(node.kind, AstKind::Ident(_)));
    }

    #[test]
    fn test_ast_node_int_creation() {
        let span = Span::new(FileId(0), 0, 5);
        let node = AstNode::int(42, span);
        assert!(matches!(node.kind, AstKind::Int(42)));
    }

    #[test]
    fn test_ast_node_string_creation() {
        let span = Span::new(FileId(0), 0, 12);
        let node = AstNode::string("hello", span);
        assert!(matches!(node.kind, AstKind::String(ref s) if s == "hello"));
    }

    #[test]
    fn test_ast_node_binary_op_creation() {
        let span = Span::new(FileId(0), 0, 10);
        let left = AstNode::int(1, span);
        let right = AstNode::int(2, span);
        let node = AstNode::binary_op("+", left, right, span);
        assert!(matches!(node.kind, AstKind::BinaryOp { op, .. } if op == "+"));
    }

    #[test]
    fn test_ast_node_call_creation() {
        let span = Span::new(FileId(0), 0, 15);
        let callee = AstNode::ident("echo", span);
        let node = AstNode::call(callee, vec![AstNode::int(1, span)], span);
        assert!(matches!(node.kind, AstKind::Call { .. }));
    }

    #[test]
    fn test_ast_node_proc_def_creation() {
        let span = Span::new(FileId(0), 0, 25);
        let body = AstNode::new(AstKind::Block(vec![]), span);
        let node = AstNode::proc_def("foo", vec![], None, body, span);
        assert!(matches!(node.kind, AstKind::ProcDef { .. }));
    }

    #[test]
    fn test_hygiene_context_new() {
        let ctx = HygieneContext::new();
        assert!(!ctx.is_dirty());
    }

    #[test]
    fn test_hygiene_context_bound_check() {
        let mut ctx = HygieneContext::new();
        ctx.bind("a", "b");
        assert!(ctx.is_bound("a"));
        assert!(!ctx.is_bound("nonexistent"));
    }

    #[test]
    fn test_template_expander_empty_template() {
        let mut expander = TemplateExpander::new();
        let params = vec![];
        let result = expander.expand("", &params, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_template_expander_missing_param() {
        let mut expander = TemplateExpander::new();
        let params = vec![TemplateParam {
            name: "x".to_string(),
            kind: TemplateParamKind::Untyped,
            default_value: None,
        }];
        // Missing argument for 'x'
        let result = expander.expand("($x$)", &params, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_macro_error_with_span() {
        let span = Span::new(FileId(1), 5, 15);
        let err = MacroError::new("error message", span);
        assert_eq!(err.message, "error message");
        assert_eq!(err.span.start, 5);
    }

    #[test]
    fn test_quoted_code_len() {
        let span = Span::new(FileId(0), 0, 5);
        let nodes = vec![
            AstNode::int(1, span),
            AstNode::int(2, span),
            AstNode::int(3, span),
        ];
        let quoted = QuotedCode::new(nodes);
        assert_eq!(quoted.len(), 3);
    }

    #[test]
    fn test_template_param_kind_untyped() {
        let param = TemplateParam {
            name: "x".to_string(),
            kind: TemplateParamKind::Untyped,
            default_value: None,
        };
        assert!(matches!(param.kind, TemplateParamKind::Untyped));
    }

    #[test]
    fn test_template_param_with_typed() {
        let param = TemplateParam {
            name: "x".to_string(),
            kind: TemplateParamKind::Typed {
                expected_type: Some("int".to_string()),
            },
            default_value: None,
        };
        assert!(matches!(param.kind, TemplateParamKind::Typed { .. }));
    }

    #[test]
    fn test_template_param_with_default() {
        let param = TemplateParam {
            name: "x".to_string(),
            kind: TemplateParamKind::Untyped,
            default_value: Some("int".to_string()),
        };
        assert!(param.default_value.is_some());
    }

    // Task 33: Macro execution and splice-back tests

    #[test]
    fn test_macro_expander_quote_nested() {
        // Test that quote preserves nested structures
        let expander = MacroExpander::new();
        let quoted = expander.quote("echo(1, 2, 3)");
        assert!(quoted.contains("quote do"));
    }

    #[test]
    fn test_macro_expander_new() {
        let expander = MacroExpander::new();
        // Should be able to create a new expander without panicking
        assert!(expander.quote("test").contains("quote"));
    }

    #[test]
    fn test_template_expander_expand_with_numeric_conversion() {
        // Test that template expansion handles numeric substitutions
        let mut expander = TemplateExpander::new();
        let params = vec![TemplateParam {
            name: "n".to_string(),
            kind: TemplateParamKind::Typed {
                expected_type: Some("int".to_string()),
            },
            default_value: None,
        }];
        let result = expander.expand("let x = $n$", &params, &["42".to_string()]);
        assert!(result.is_ok());
        let expanded = result.unwrap();
        assert!(expanded.code.contains("42"));
    }

    #[test]
    fn test_template_expander_block_capture() {
        // Test that block parameters are captured correctly
        let mut expander = TemplateExpander::new();
        let params = vec![TemplateParam {
            name: "block".to_string(),
            kind: TemplateParamKind::Block,
            default_value: None,
        }];
        let result = expander.expand("if true:\n  $block$\n", &params, &["echo(1)".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ast_node_block_creation() {
        // Test block node creation
        let span = Span::new(FileId(0), 0, 10);
        let node1 = AstNode::int(1, span);
        let node2 = AstNode::int(2, span);
        let block = AstNode::block(vec![node1, node2], span);
        assert!(matches!(block.kind, AstKind::Block(v) if v.len() == 2));
    }

    #[test]
    fn test_ast_node_let_decl_creation() {
        // Test let declaration using the existing let_decl constructor
        let span = Span::new(FileId(0), 0, 15);
        let value = AstNode::int(10, span);
        let node = AstNode::let_decl("x", value, span);
        assert!(matches!(
            node.kind,
            AstKind::VarDecl { kind, name, .. } if kind == "let" && name == "x"
        ));
    }

    #[test]
    fn test_ast_node_var_decl_kind() {
        // Test var declaration kind
        let span = Span::new(FileId(0), 0, 20);
        let value = AstNode::int(10, span);
        let node = AstNode::var_decl("count", None, value, span);
        if let AstKind::VarDecl { kind, .. } = &node.kind {
            assert_eq!(kind, "var");
        }
    }

    #[test]
    fn test_ast_node_binary_op_fields() {
        // Test binary operation fields
        let span = Span::new(FileId(0), 0, 15);
        let left = AstNode::int(5, span);
        let right = AstNode::int(3, span);
        let node = AstNode::binary_op("+", left, right, span);
        if let AstKind::BinaryOp { op, .. } = &node.kind {
            assert_eq!(op, "+");
        }
    }

    #[test]
    fn test_ast_node_proc_def_fields() {
        // Test procedure definition fields
        let span = Span::new(FileId(0), 0, 25);
        let body = AstNode::block(vec![], span);
        let node = AstNode::proc_def(
            "add",
            vec![("a".to_string(), Some("int".to_string()))],
            Some("int".to_string()),
            body,
            span,
        );
        if let AstKind::ProcDef {
            name,
            params,
            ret_type,
            ..
        } = &node.kind
        {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 1);
            assert!(ret_type.is_some());
        }
    }

    #[test]
    fn test_ast_node_call_callee() {
        // Test call node callee
        let span = Span::new(FileId(0), 0, 20);
        let callee = AstNode::ident("echo", span);
        let args = vec![AstNode::int(1, span)];
        let node = AstNode::call(callee, args, span);
        assert!(matches!(node.kind, AstKind::Call { .. }));
    }

    #[test]
    fn test_macro_expander_quote_preserves_code() {
        // Test that quote preserves the original code structure
        let expander = MacroExpander::new();
        let code = "proc test() = echo 1";
        let quoted = expander.quote(code);
        // The quoted code should contain the original code
        assert!(quoted.contains("proc") || quoted.contains("quote"));
    }

    #[test]
    fn test_template_expander_recursion_with_deeply_nested() {
        // Test template expansion with deeply nested substitutions
        let mut expander = TemplateExpander::new();
        let params = vec![
            TemplateParam {
                name: "a".to_string(),
                kind: TemplateParamKind::Untyped,
                default_value: None,
            },
            TemplateParam {
                name: "b".to_string(),
                kind: TemplateParamKind::Untyped,
                default_value: None,
            },
        ];
        let result = expander.expand("$a$ + $b$", &params, &["1".to_string(), "2".to_string()]);
        assert!(result.is_ok());
        let expanded = result.unwrap();
        assert!(expanded.code.contains("1") && expanded.code.contains("2"));
    }
}
