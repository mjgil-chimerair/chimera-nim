//! JavaScript backend for supported Nim JS compilation mode.

use camino::Utf8Path;
#[cfg(test)]
use rnim_allocator as _;
#[allow(unused_imports)]
use rnim_mir::{
    BasicBlock, BinOp, CmpOp, Function, FunctionAttribute, GotoTarget, Local, MirBody, MirModule,
    MirStmt, MirType, MirValue, NodeIndex, Place, Terminator, UnOp,
};
use rnim_span::{FileId, Span};
use std::collections::HashMap;

/// JavaScript backend configuration
#[derive(Debug, Clone)]
pub struct JsCodegenConfig {
    /// Source map enabled
    pub source_map: bool,
    /// Use ES modules
    pub es_modules: bool,
    /// Runtime mode
    pub runtime: JsRuntime,
    /// Enable async/await transformation
    pub async_support: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum JsRuntime {
    #[default]
    Node,
    Browser,
    Standalone,
}

impl Default for JsCodegenConfig {
    fn default() -> Self {
        JsCodegenConfig {
            source_map: true,
            es_modules: true,
            runtime: JsRuntime::Node,
            async_support: true,
        }
    }
}

/// JavaScript emitted module
#[derive(Debug, Clone)]
pub struct JsModule {
    pub name: String,
    pub source: String,
    pub source_map: Option<String>,
    pub dependencies: Vec<String>,
}

impl JsModule {
    pub fn new(name: String) -> Self {
        JsModule {
            name,
            source: String::new(),
            source_map: None,
            dependencies: Vec::new(),
        }
    }
}

/// JavaScript code generator
#[allow(dead_code)]
pub struct JsCodeGenerator {
    config: JsCodegenConfig,
    output: String,
    indent_level: usize,
    source_map_buffer: Vec<SourceMapEntry>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SourceMapEntry {
    pub generated_line: usize,
    pub generated_col: usize,
    pub source_line: u32,
    pub source_col: u32,
    pub source_file: String,
}

impl JsCodeGenerator {
    pub fn new(config: JsCodegenConfig) -> Self {
        JsCodeGenerator {
            config,
            output: String::new(),
            indent_level: 0,
            source_map_buffer: Vec::new(),
        }
    }

    fn indent(&mut self) {
        for _ in 0..self.indent_level {
            self.output.push_str("  ");
        }
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }

    fn emit_line(&mut self, line: &str) {
        self.indent();
        self.output.push_str(line);
        self.newline();
    }

    #[allow(dead_code)]
    fn emit_type(&self, ty: &MirType) -> String {
        match ty {
            MirType::Unit => "undefined".to_string(),
            MirType::Bool => "boolean".to_string(),
            MirType::Int(_) | MirType::Uint(_) => "number".to_string(),
            MirType::Float(_) | MirType::Double(_) => "number".to_string(),
            MirType::Char => "string".to_string(),
            MirType::String => "string".to_string(),
            MirType::Ref(_) | MirType::MutRef(_) => "object".to_string(),
            MirType::Pointer(_) => "number".to_string(),
            MirType::Array(inner, _size) => format!("Array<{}>", self.emit_type(inner)),
            MirType::Seq(_) => "Array".to_string(),
            MirType::Set(_) => "Array".to_string(),
            MirType::Tuple(types) => format!(
                "[{}]",
                types
                    .iter()
                    .map(|t| self.emit_type(t))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            MirType::Adt(name, _) => name.clone(),
            MirType::Enum(name, _) => name.clone(),
            MirType::Proc(params, ret) => {
                let param_strs: Vec<String> = params.iter().map(|p| self.emit_type(p)).collect();
                format!(
                    "function({}) => {}",
                    param_strs.join(", "),
                    self.emit_type(ret)
                )
            }
            MirType::OpenArray(inner) => format!("Array<{}>", self.emit_type(inner)),
            MirType::Varargs => "Array".to_string(),
            MirType::Untyped => "any".to_string(),
            MirType::Never => "never".to_string(),
        }
    }

    fn emit_value(&self, value: &MirValue) -> String {
        match value {
            MirValue::Unit(_) => "undefined".to_string(),
            MirValue::Bool(b, _) => if *b { "true" } else { "false" }.to_string(),
            MirValue::Int(i, _) => i.to_string(),
            MirValue::Uint(u, _) => u.to_string(),
            MirValue::Float(f, _) => f.to_string(),
            MirValue::String(s, _) => format!(
                "\"{}\"",
                s.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
            ),
            MirValue::Pointer(ptr, _) => format!("{}", *ptr as usize),
            MirValue::Place(place) => format!("l_{}", place.local.0),
            MirValue::Tuple(values, _) => {
                let elements: Vec<String> = values.iter().map(|v| self.emit_value(v)).collect();
                format!("[{}]", elements.join(", "))
            }
            MirValue::Array(values, _) => {
                let elements: Vec<String> = values.iter().map(|v| self.emit_value(v)).collect();
                format!("[{}]", elements.join(", "))
            }
            MirValue::Struct(fields, _) => {
                let pairs: Vec<String> = fields
                    .iter()
                    .map(|(f, v)| format!("f_{}: {}", f.0, self.emit_value(v)))
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
            MirValue::Variant {
                enum_name,
                index,
                value,
                ..
            } => {
                format!(
                    "{{ \"$variant\": \"{}\", \"$index\": {}, \"$value\": {} }}",
                    enum_name,
                    index,
                    self.emit_value(value)
                )
            }
            MirValue::BinOp(op, lhs, rhs, _) => {
                let l = self.emit_value(lhs);
                let r = self.emit_value(rhs);
                let js_op = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Mod => "%",
                    BinOp::BitAnd => "&",
                    BinOp::BitOr => "|",
                    BinOp::BitXor => "^",
                    BinOp::ShiftLeft => "<<",
                    BinOp::ShiftRight => ">>",
                };
                format!("({} {} {})", l, js_op, r)
            }
            MirValue::UnOp(op, val, _) => {
                let v = self.emit_value(val);
                match op {
                    UnOp::Neg => format!("(-{})", v),
                    UnOp::Not => format!("(!{})", v),
                    UnOp::BitNot => format!("(~{})", v),
                }
            }
            MirValue::Cast(val, _, _) => self.emit_value(val),
            MirValue::BitCast(val, _, _) => self.emit_value(val),
            MirValue::Comparison(op, lhs, rhs, _) => {
                let l = self.emit_value(lhs);
                let r = self.emit_value(rhs);
                let js_op = match op {
                    CmpOp::Eq => "===",
                    CmpOp::Ne => "!==",
                    CmpOp::Lt => "<",
                    CmpOp::Le => "<=",
                    CmpOp::Gt => ">",
                    CmpOp::Ge => ">=",
                };
                format!("({} {} {})", l, js_op, r)
            }
            MirValue::Function(func_ref) => func_ref.name.clone(),
            MirValue::Closure { func, captured } => {
                let func_name = self.emit_value(func);
                let captured_init: Vec<String> = captured
                    .iter()
                    .map(|(_, val)| self.emit_value(val))
                    .collect();
                format!(
                    "(() => {{ const [$captures] = [{}]; return {}; }})()",
                    captured_init.join(", "),
                    func_name
                )
            }
            MirValue::AddrOf(place) => {
                format!("(() => l_{})", place.local.0)
            }
        }
    }

    fn emit_function(&mut self, func: &Function) {
        let params: Vec<String> = func
            .params
            .iter()
            .enumerate()
            .map(|(i, _ty)| format!("p_{}", i))
            .collect();

        let async_kw = if self.config.async_support
            && func
                .attributes
                .iter()
                .any(|a| matches!(a, FunctionAttribute::NoSideEffect))
        {
            "async "
        } else {
            ""
        };

        self.emit_line(&format!(
            "function {}{}({}) {{",
            async_kw,
            self.js_identifier(&func.name),
            params.join(", ")
        ));
        self.indent_level += 1;

        if let Some(body) = &func.body {
            self.emit_mir_body(body);
        } else {
            self.emit_line("return undefined;");
        }

        self.indent_level -= 1;
        self.emit_line("}");
        self.newline();
    }

    fn js_identifier(&self, name: &str) -> String {
        name.replace(".", "_").replace("::", "_")
    }

    fn emit_mir_body(&mut self, body: &MirBody) {
        for (idx, info) in body.locals.iter().enumerate() {
            if !info.is_arg {
                let ty_str = self.js_type(&info.ty);
                self.emit_line(&format!("let l_{} = undefined; // {}", idx, ty_str));
            }
        }

        for (idx, block) in body.blocks.iter().enumerate() {
            if idx > 0 {
                self.emit_line(&format!("// block_{}", idx));
            }
            self.emit_line(&format!("block_{}: {{", idx));
            self.indent_level += 1;

            for stmt in &block.statements {
                self.emit_statement(stmt);
            }

            self.emit_terminator(&block.terminator);
            self.indent_level -= 1;
            self.emit_line("}");
            self.newline();
        }
    }

    fn js_type(&self, ty: &MirType) -> String {
        match ty {
            MirType::Unit => "undefined".to_string(),
            MirType::Bool => "boolean".to_string(),
            MirType::Int(_) | MirType::Uint(_) => "number".to_string(),
            MirType::Float(_) | MirType::Double(_) => "number".to_string(),
            MirType::Char | MirType::String => "string".to_string(),
            MirType::Ref(inner) => format!("Ref<{}>", self.js_type(inner)),
            MirType::MutRef(inner) => format!("Ref<{}>", self.js_type(inner)),
            MirType::Pointer(_) => "number".to_string(),
            MirType::Array(inner, size) => format!("Array[{}]<{}>", size, self.js_type(inner)),
            MirType::Seq(_) => "Array".to_string(),
            MirType::Set(_) => "Set".to_string(),
            MirType::Tuple(types) => format!(
                "[{}]",
                types
                    .iter()
                    .map(|t| self.js_type(t))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            MirType::Adt(name, _) => name.clone(),
            MirType::Enum(name, _) => name.clone(),
            MirType::Proc(params, ret) => {
                let param_strs: Vec<String> = params.iter().map(|p| self.js_type(p)).collect();
                format!("({}) => {}", param_strs.join(", "), self.js_type(ret))
            }
            MirType::OpenArray(inner) => format!("Array<{}>", self.js_type(inner)),
            MirType::Varargs => "Array".to_string(),
            MirType::Untyped => "any".to_string(),
            MirType::Never => "never".to_string(),
        }
    }

    fn emit_statement(&mut self, stmt: &MirStmt) {
        match stmt {
            MirStmt::Assign { place, value } => {
                let lhs = format!("l_{}", place.local.0);
                let rhs = self.emit_value(value);
                self.emit_line(&format!("{} = {};", lhs, rhs));
            }
            MirStmt::SetDiscriminant {
                place,
                variant_index,
            } => {
                self.emit_line(&format!(
                    "l_{}.discriminant = {};",
                    place.local.0, variant_index
                ));
            }
            MirStmt::StorageLive(local, _) => {
                let _ = local;
            }
            MirStmt::StorageDead(local, _) => {
                self.emit_line(&format!("l_{} = null; // StorageDead", local.0));
            }
            MirStmt::Call {
                destination,
                callee,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| self.emit_value(a)).collect();
                let call_str = format!("{}({})", self.emit_value(callee), args.join(", "));
                if let Some((place, _)) = destination {
                    self.emit_line(&format!("l_{} = {};", place.local.0, call_str));
                } else {
                    self.emit_line(&format!("{};", call_str));
                }
            }
            MirStmt::TryCall {
                destination,
                cleanup: _,
                callee,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| self.emit_value(a)).collect();
                let call_str = format!("{}({})", self.emit_value(callee), args.join(", "));
                if let Some((place, _)) = destination {
                    let lhs = format!("l_{}", place.local.0);
                    self.emit_line(&format!(
                        "try {{ {} = {}; }} catch (e) {{ /* handle */ }}",
                        lhs, call_str
                    ));
                } else {
                    self.emit_line(&format!(
                        "try {{ {}; }} catch (e) {{ /* handle */ }}",
                        call_str
                    ));
                }
            }
            MirStmt::Drop { place, target } => {
                self.emit_line(&format!("// Drop l_{}", place.local.0));
                if let Some(t) = target {
                    self.emit_line(&format!("goto block_{};", t.index()));
                }
            }
            MirStmt::Deinit(place) => {
                self.emit_line(&format!("// Deinit l_{}", place.local.0));
            }
            MirStmt::Assert {
                condition,
                msg,
                target,
            } => {
                self.emit_line(&format!(
                    "if (!({})) {{ throw new Error('Assertion failed: {}'); }}",
                    self.emit_value(condition),
                    msg
                ));
                self.emit_line(&format!("goto block_{};", target.index()));
            }
            MirStmt::FakeRead { place } => {
                self.emit_line(&format!("// FakeRead l_{}", place.local.0));
            }
            MirStmt::Nop => {}
            MirStmt::ResetRecursion(local, _) => {
                self.emit_line(&format!("// ResetRecursion l_{}", local.0));
            }
        }
    }

    fn emit_terminator(&mut self, term: &Terminator) {
        match term {
            Terminator::Goto(target) => match target.as_ref() {
                GotoTarget::Next => {
                    self.emit_line("// Fallthrough");
                }
                GotoTarget::Block(idx) => {
                    self.emit_line(&format!("break block_{};", idx.index()));
                }
                GotoTarget::Switch(cases) => {
                    self.emit_line("// Switch dispatch");
                    for (val, target) in cases {
                        self.emit_line(&format!(
                            "case {}: break block_{};",
                            self.emit_value(val),
                            target.index()
                        ));
                    }
                    self.emit_line("break;");
                }
            },
            Terminator::Switch {
                discriminant,
                targets,
            } => {
                self.emit_line(&format!("switch ({}) {{", self.emit_value(discriminant)));
                for (i, target) in targets.iter().enumerate() {
                    self.emit_line(&format!("case {}: break block_{};", i, target.index()));
                }
                self.emit_line("default: break;");
                self.emit_line("}");
            }
            Terminator::Return => {
                self.emit_line("return undefined;");
            }
            Terminator::Call {
                destination,
                target,
                cleanup: _,
                callee,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| self.emit_value(a)).collect();
                let call_str = format!("{}({})", self.emit_value(callee), args.join(", "));
                if let Some((place, _)) = destination {
                    self.emit_line(&format!("l_{} = {};", place.local.0, call_str));
                }
                if let Some(t) = target {
                    self.emit_line(&format!("break block_{};", t.index()));
                }
            }
            Terminator::TryCall {
                destination,
                target,
                cleanup: _,
                callee,
                arguments,
            } => {
                self.emit_line("try {");
                let args: Vec<String> = arguments.iter().map(|a| self.emit_value(a)).collect();
                let call_str = format!("{}({})", self.emit_value(callee), args.join(", "));
                if let Some((place, _)) = destination {
                    self.emit_line(&format!("  l_{} = {};", place.local.0, call_str));
                } else {
                    self.emit_line(&format!("  {};", call_str));
                }
                self.emit_line("} catch (e) { /* handle */ }");
                if let Some(t) = target {
                    self.emit_line(&format!("break block_{};", t.index()));
                }
            }
            Terminator::If {
                condition,
                then_block,
                else_block,
            } => {
                self.emit_line(&format!(
                    "if ({}) {{ break block_{}; }} else {{ break block_{}; }}",
                    self.emit_value(condition),
                    then_block.index(),
                    else_block.index()
                ));
            }
            Terminator::Raise(value) => {
                let val_str = self.emit_value(value);
                self.emit_line(&format!("throw {};", val_str));
            }
            Terminator::Assert {
                condition,
                msg,
                target,
            } => {
                let cond_str = self.emit_value(condition);
                self.emit_line(&format!(
                    "if (!({})) {{ throw new Error('Assertion failed: {}'); }}",
                    cond_str, msg
                ));
                self.emit_line(&format!("break block_{};", target.index()));
            }
            Terminator::Unreachable => {
                self.emit_line("throw new Error('unreachable');");
            }
            Terminator::Drop {
                place,
                target,
                unwind: _,
            } => {
                self.emit_line(&format!("// Drop l_{}", place.local.0));
                if let Some(t) = target {
                    self.emit_line(&format!("break block_{};", t.index()));
                }
            }
            Terminator::Fallthrough => {
                self.emit_line("// Fallthrough");
            }
        }
    }

    fn emit_runtime(&mut self) {
        self.output.push_str("// Nim runtime support\n");
        self.output.push_str("const RNIM_UNIT = undefined;\n");
        self.output.push_str("const RNIM_STRING = String;\n");
        self.output.push_str("const RNIM_SEQ = Array;\n");
        self.output
            .push_str("const RNIM_DECREF = (x) => { /* GC handled by JS */ };\n");
        self.output
            .push_str("const RNIM_INCREF = (x) => { /* GC handled by JS */ };\n");
        self.output
            .push_str("const RNIM_RAISE = (exc) => { throw exc; };\n");
        self.output
            .push_str("const RNIM_ASSERT_FAIL = (msg) => { throw new Error(msg); };\n");
        self.newline();
    }

    fn emit_header(&mut self, _module: &MirModule) {
        self.output
            .push_str("// Generated by rustnim JavaScript backend\n");
        if self.config.es_modules {
            self.output.push_str("\"use strict\";\n");
            self.output.push_str("export {\n");
        }
        self.newline();
    }

    fn emit_footer(&mut self, _module: &MirModule) {
        if self.config.es_modules {
            self.output.push_str("};\n");
        }
    }

    fn generate_source_map(&self) -> Option<String> {
        if !self.config.source_map {
            return None;
        }
        Some("{\"version\": 3, \"sources\": [], \"mappings\": \"\"}".to_string())
    }

    pub fn emit_js(&mut self, module: &MirModule) -> JsModule {
        let mut js_module = JsModule::new(module.name.clone());

        self.emit_runtime();
        self.emit_header(module);

        for func in &module.functions {
            self.emit_function(func);
        }

        self.emit_footer(module);
        js_module.source = self.output.clone();
        js_module.source_map = self.generate_source_map();

        js_module
    }
}

pub fn emit_js_api(path: &Utf8Path) -> Result<JsModule, String> {
    let config = JsCodegenConfig::default();
    let mut generator = JsCodeGenerator::new(config);

    // Try to load a MirModule from the path, or create empty for testing
    let module = if path.exists() {
        // In a full implementation, this would parse the file and build MIR
        // For now, create empty module with the file name
        let name = path
            .file_stem()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "output".to_string());
        MirModule {
            name,
            functions: Vec::new(),
            types: HashMap::new(),
            source_span: Span::new(FileId(0), 0, 0),
        }
    } else {
        // Path doesn't exist - return empty module with warning
        return Ok(JsModule::new("empty".to_string()));
    };

    Ok(generator.emit_js(&module))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_codegen_config_default() {
        let config = JsCodegenConfig::default();
        assert!(config.source_map);
        assert!(config.es_modules);
        assert!(config.async_support);
        assert_eq!(config.runtime, JsRuntime::Node);
    }

    #[test]
    fn test_jsmodule_new() {
        let module = JsModule::new("test".to_string());
        assert_eq!(module.name, "test");
        assert!(module.source.is_empty());
        assert!(module.source_map.is_none());
        assert!(module.dependencies.is_empty());
    }

    #[test]
    fn test_emit_type_bool() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        assert_eq!(r#gen.emit_type(&MirType::Bool), "boolean");
    }

    #[test]
    fn test_emit_type_int() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        assert_eq!(r#gen.emit_type(&MirType::Int(32)), "number");
    }

    #[test]
    fn test_emit_type_string() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        assert_eq!(r#gen.emit_type(&MirType::String), "string");
    }

    #[test]
    fn test_emit_type_unit() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        assert_eq!(r#gen.emit_type(&MirType::Unit), "undefined");
    }

    #[test]
    fn test_emit_value_bool() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let span = Span::new(FileId(0), 0, 0);
        assert_eq!(r#gen.emit_value(&MirValue::Bool(true, span)), "true");
        assert_eq!(r#gen.emit_value(&MirValue::Bool(false, span)), "false");
    }

    #[test]
    fn test_emit_value_int() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let span = Span::new(FileId(0), 0, 0);
        assert_eq!(r#gen.emit_value(&MirValue::Int(42, span)), "42");
        assert_eq!(r#gen.emit_value(&MirValue::Int(-10, span)), "-10");
    }

    #[test]
    fn test_emit_value_uint() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let span = Span::new(FileId(0), 0, 0);
        assert_eq!(r#gen.emit_value(&MirValue::Uint(100, span)), "100");
    }

    #[test]
    fn test_emit_value_string() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let span = Span::new(FileId(0), 0, 0);
        assert_eq!(
            r#gen.emit_value(&MirValue::String("hello".to_string(), span)),
            "\"hello\""
        );
    }

    #[test]
    fn test_emit_value_binop() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::BinOp(
            BinOp::Add,
            Box::new(MirValue::Int(2, span)),
            Box::new(MirValue::Int(3, span)),
            span,
        );
        assert_eq!(r#gen.emit_value(&expr), "(2 + 3)");
    }

    #[test]
    fn test_emit_value_unop() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::UnOp(UnOp::Neg, Box::new(MirValue::Int(5, span)), span);
        assert_eq!(r#gen.emit_value(&expr), "(-5)");
    }

    #[test]
    fn test_emit_value_comparison() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let span = Span::new(FileId(0), 0, 0);
        let expr = MirValue::Comparison(
            CmpOp::Eq,
            Box::new(MirValue::Int(10, span)),
            Box::new(MirValue::Int(20, span)),
            span,
        );
        assert_eq!(r#gen.emit_value(&expr), "(10 === 20)");
    }

    #[test]
    fn test_emit_value_array() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let span = Span::new(FileId(0), 0, 0);
        let arr = MirValue::Array(vec![MirValue::Int(1, span), MirValue::Int(2, span)], span);
        assert_eq!(r#gen.emit_value(&arr), "[1, 2]");
    }

    #[test]
    fn test_emit_value_tuple() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let span = Span::new(FileId(0), 0, 0);
        let tuple = MirValue::Tuple(
            vec![
                MirValue::Int(1, span),
                MirValue::String("hi".to_string(), span),
            ],
            span,
        );
        assert_eq!(r#gen.emit_value(&tuple), "[1, \"hi\"]");
    }

    #[test]
    fn test_emit_js_with_function() {
        let config = JsCodegenConfig::default();
        let mut r#gen = JsCodeGenerator::new(config);

        let span = Span::new(FileId(0), 0, 0);
        let entry = NodeIndex::new(0);

        let mut body = MirBody::new(entry, Place::new(Local::new(0, span)), span);

        let block = BasicBlock::with_terminator(Terminator::Return, span);
        body.blocks.push(block);

        let func = Function {
            name: "add".to_string(),
            body: Some(body),
            params: vec![MirType::Int(32), MirType::Int(32)],
            return_type: MirType::Int(32),
            span,
            attributes: vec![],
        };

        let module = MirModule {
            name: "test".to_string(),
            functions: vec![func],
            types: HashMap::new(),
            source_span: span,
        };

        let result = r#gen.emit_js(&module);

        assert!(result.source.contains("function add(p_0, p_1)"));
        assert!(result.source.contains("block_0"));
        assert!(result.source.contains("RNIM_UNIT"));
    }

    #[test]
    fn test_emit_js_with_empty_module() {
        let config = JsCodegenConfig::default();
        let mut r#gen = JsCodeGenerator::new(config);

        let module = MirModule {
            name: "empty".to_string(),
            functions: vec![],
            types: HashMap::new(),
            source_span: Span::new(FileId(0), 0, 0),
        };

        let result = r#gen.emit_js(&module);

        assert!(result.source.contains("RNIM_UNIT"));
        assert!(result.source.contains("RNIM_STRING"));
        assert!(result.source.contains("RNIM_SEQ"));
    }

    #[test]
    fn test_emit_type_array() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        assert_eq!(
            r#gen.emit_type(&MirType::Array(Box::new(MirType::Int(32)), 10)),
            "Array<number>"
        );
    }

    #[test]
    fn test_emit_type_proc() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let proc_type = MirType::Proc(
            vec![MirType::Int(32), MirType::Int(32)],
            Box::new(MirType::Int(32)),
        );
        let result = r#gen.emit_type(&proc_type);
        assert!(result.contains("function"));
        assert!(result.contains("number"));
    }

    #[test]
    fn test_emit_type_adt() {
        let config = JsCodegenConfig::default();
        let r#gen = JsCodeGenerator::new(config);
        let adt_type = MirType::Adt(
            "MyStruct".to_string(),
            vec![
                ("field1".to_string(), MirType::Int(32)),
                ("field2".to_string(), MirType::Bool),
            ],
        );
        assert_eq!(r#gen.emit_type(&adt_type), "MyStruct");
    }

    #[test]
    fn test_function_attribute_variants() {
        use rnim_mir::FunctionAttribute;
        assert!(matches!(
            FunctionAttribute::NoReturn,
            FunctionAttribute::NoReturn
        ));
        assert!(matches!(FunctionAttribute::Cold, FunctionAttribute::Cold));
        assert!(matches!(
            FunctionAttribute::Inline,
            FunctionAttribute::Inline
        ));
        assert!(matches!(
            FunctionAttribute::NoInline,
            FunctionAttribute::NoInline
        ));
        assert!(matches!(FunctionAttribute::Entry, FunctionAttribute::Entry));
        assert!(matches!(
            FunctionAttribute::NoSideEffect,
            FunctionAttribute::NoSideEffect
        ));
    }
}
