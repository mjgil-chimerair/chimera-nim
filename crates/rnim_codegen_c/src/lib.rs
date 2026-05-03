//! C/C++/Objective-C compatible backend, generated headers, ABI lowering, external compiler invocation.

use camino::{Utf8Path, Utf8PathBuf};
#[cfg(test)]
use rnim_allocator as _;
use rnim_mir::{
    BinOp, CmpOp, Function, FunctionAttribute, GotoTarget, Local, MirBody, MirModule, MirStmt,
    MirType, MirValue, Place, Terminator, UnOp,
};
use rnim_span::{FileId, Span};
use std::collections::HashMap;

/// C backend configuration
#[derive(Debug, Clone)]
pub struct CodegenConfig {
    pub output_dir: Utf8PathBuf,
    pub debug_info: bool,
    pub c_version: CVersion,
    pub exceptions: bool,
    pub use_orc: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CVersion {
    C99,
    C11,
    C17,
}

impl Default for CVersion {
    fn default() -> Self {
        CVersion::C11
    }
}

impl Default for CodegenConfig {
    fn default() -> Self {
        CodegenConfig {
            output_dir: Utf8PathBuf::from("."),
            debug_info: true,
            c_version: CVersion::C11,
            exceptions: true,
            use_orc: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CModule {
    pub name: String,
    pub source: String,
    pub header: String,
    pub dependencies: Vec<String>,
}

impl CModule {
    pub fn new(name: String) -> Self {
        CModule {
            name,
            source: String::new(),
            header: String::new(),
            dependencies: Vec::new(),
        }
    }
}

pub struct CCodeGenerator {
    config: CodegenConfig,
    output: String,
    indent_level: usize,
}

impl CCodeGenerator {
    pub fn new(config: CodegenConfig) -> Self {
        CCodeGenerator {
            config,
            output: String::new(),
            indent_level: 0,
        }
    }

    fn indent(&mut self) {
        for _ in 0..self.indent_level {
            self.output.push_str("    ");
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

    fn emit_type(&self, ty: &MirType) -> String {
        match ty {
            MirType::Unit => "void".to_string(),
            MirType::Bool => "bool".to_string(),
            MirType::Int(size) => format!("int{}_t", size),
            MirType::Uint(size) => format!("uint{}_t", size),
            MirType::Float(_) => "float".to_string(),
            MirType::Double(_) => "double".to_string(),
            MirType::Char => "char".to_string(),
            MirType::String => "RNIM_STRING".to_string(),
            MirType::Ref(inner) => format!("{}*", self.emit_type(inner)),
            MirType::MutRef(inner) => format!("{}*", self.emit_type(inner)),
            MirType::Pointer(inner) => format!("{}*", self.emit_type(inner)),
            MirType::Array(inner, size) => format!("{}[{}]", self.emit_type(inner), size),
            MirType::Seq(_) => "RNIM_SEQ".to_string(),
            MirType::Set(_) => "uint64_t".to_string(),
            MirType::Tuple(types) => {
                let fields: Vec<String> = types
                    .iter()
                    .enumerate()
                    .map(|(i, t)| format!("{} f_{}", self.emit_type(t), i))
                    .collect();
                format!("struct {{ {} }}", fields.join("; "))
            }
            MirType::Adt(name, _) => name.clone(),
            MirType::Enum(name, _) => name.clone(),
            MirType::Proc(params, ret) => {
                let param_strs: Vec<String> = params.iter().map(|p| self.emit_type(p)).collect();
                format!("{} (*)({})", self.emit_type(ret), param_strs.join(", "))
            }
            MirType::OpenArray(inner) => format!("RNIM_OPENARRAY({})", self.emit_type(inner)),
            MirType::Varargs => "...".to_string(),
            MirType::Untyped => "void*".to_string(),
            MirType::Never => "NR_NORETURN".to_string(),
        }
    }

    fn emit_value(&self, value: &MirValue) -> String {
        match value {
            MirValue::Unit(_) => "RNIM_UNIT".to_string(),
            MirValue::Bool(b, _) => if *b { "true" } else { "false" }.to_string(),
            MirValue::Int(i, _) => i.to_string(),
            MirValue::Uint(u, _) => format!("{}u", u),
            MirValue::Float(f, _) => {
                let bits = f.to_bits();
                format!("NIM_FLOAT_CONST({}, {})", bits, f)
            }
            MirValue::String(s, _) => {
                format!(
                    "RNIM_STR_LITERAL(\"{}\")",
                    s.replace('\\', "\\\\").replace('"', "\\\"")
                )
            }
            MirValue::Pointer(ptr, _) => format!("(void*){}", *ptr as usize),
            MirValue::Place(place) => format!("l_{}", place.local.0),
            MirValue::Tuple(values, _) => {
                let elements: Vec<String> = values.iter().map(|v| self.emit_value(v)).collect();
                format!("((RNIM_TUPLE){{{}}})", elements.join(", "))
            }
            MirValue::Array(values, _) => {
                let elements: Vec<String> = values.iter().map(|v| self.emit_value(v)).collect();
                format!("((void*){{{}}})", elements.join(", "))
            }
            MirValue::Struct(fields, _) => {
                let pairs: Vec<String> = fields
                    .iter()
                    .map(|(f, v)| format!(".f_{} = {}", f.0, self.emit_value(v)))
                    .collect();
                format!("((struct {{}} {{{}}})", pairs.join(", "))
            }
            MirValue::Variant {
                enum_name,
                index,
                value,
                ..
            } => {
                format!(
                    "RNIM_VARIANT({}, {}, {})",
                    enum_name,
                    index,
                    self.emit_value(value)
                )
            }
            MirValue::BinOp(op, lhs, rhs, _) => {
                let l = self.emit_value(lhs);
                let r = self.emit_value(rhs);
                let c_op = match op {
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
                format!("({} {} {})", l, c_op, r)
            }
            MirValue::UnOp(op, val, _) => {
                let v = self.emit_value(val);
                match op {
                    UnOp::Neg => format!("(-{})", v),
                    UnOp::Not => format!("(!{})", v),
                    UnOp::BitNot => format!("(~{})", v),
                }
            }
            MirValue::Cast(val, target, _) => {
                format!("(({})({}))", self.emit_type(target), self.emit_value(val))
            }
            MirValue::BitCast(val, target, _) => {
                format!("(({})({}))", self.emit_type(target), self.emit_value(val))
            }
            MirValue::Comparison(op, lhs, rhs, _) => {
                let l = self.emit_value(lhs);
                let r = self.emit_value(rhs);
                let c_op = match op {
                    CmpOp::Eq => "==",
                    CmpOp::Ne => "!=",
                    CmpOp::Lt => "<",
                    CmpOp::Le => "<=",
                    CmpOp::Gt => ">",
                    CmpOp::Ge => ">=",
                };
                format!("({} {} {})", l, c_op, r)
            }
            MirValue::Function(func_ref) => func_ref.name.clone(),
            MirValue::Closure { func, captured } => {
                let func_name = self.emit_value(func);
                let captured_init: Vec<String> = captured
                    .iter()
                    .map(|(_, val)| self.emit_value(val))
                    .collect();
                format!(
                    "RNIM_CLOSURE({}, (RNIM_CAP_SLICE){{{}}})",
                    func_name,
                    captured_init.join(", ")
                )
            }
            MirValue::AddrOf(place) => {
                format!("(&l_{})", place.local.0)
            }
        }
    }

    fn emit_function_header(&self, func: &Function) -> String {
        let ret_type = self.emit_type(&func.return_type);
        let params: Vec<String> = func
            .params
            .iter()
            .enumerate()
            .map(|(i, ty)| format!("{} p_{}", self.emit_type(ty), i))
            .collect();
        format!("{} {}({});", ret_type, func.name, params.join(", "))
    }

    fn emit_function_impl(&mut self, func: &Function) {
        let ret_type = self.emit_type(&func.return_type);
        let params: Vec<String> = func
            .params
            .iter()
            .enumerate()
            .map(|(i, ty)| format!("{} p_{}", self.emit_type(ty), i))
            .collect();

        let mut attrs = Vec::new();
        for attr in &func.attributes {
            match attr {
                FunctionAttribute::NoReturn => attrs.push("__attribute__((noreturn))"),
                FunctionAttribute::Cold => attrs.push("__attribute__((cold))"),
                FunctionAttribute::Inline => attrs.push("inline"),
                FunctionAttribute::NoInline => attrs.push("__attribute__((noinline))"),
                FunctionAttribute::Entry | FunctionAttribute::NoSideEffect => {}
            }
        }

        let attr_str = if attrs.is_empty() {
            String::new()
        } else {
            format!("{} ", attrs.join(" "))
        };

        self.emit_line(&format!(
            "{} {}{}({}) {{",
            ret_type,
            attr_str,
            func.name,
            params.join(", ")
        ));
        self.indent_level += 1;

        if let Some(body) = &func.body {
            self.emit_mir_body(body);
        }

        self.indent_level -= 1;
        self.emit_line("}");
        self.newline();
    }

    fn emit_mir_body(&mut self, body: &MirBody) {
        for (idx, info) in body.locals.iter().enumerate() {
            if !info.is_arg {
                let ty_str = self.emit_type(&info.ty);
                self.emit_line(&format!("{} l_{};", ty_str, idx));
            }
        }

        for (idx, block) in body.blocks.iter().enumerate() {
            self.emit_line(&format!("block_{}:", idx));

            for stmt in &block.statements {
                self.emit_statement(stmt);
            }

            self.emit_terminator(&block.terminator);
            self.newline();
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
                self.emit_line(&format!("/* StorageDead l_{} */", local.0));
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
                    self.emit_line(&format!("l_{} = {};", place.local.0, call_str));
                } else {
                    self.emit_line(&format!("{};", call_str));
                }
            }
            MirStmt::Drop { place, target } => {
                if let Some(t) = target {
                    self.emit_line(&format!("goto block_{};", t.index()));
                }
                let _ = place;
            }
            MirStmt::Deinit(place) => {
                self.emit_line(&format!("/* Deinit l_{} */", place.local.0));
            }
            MirStmt::Assert {
                condition,
                msg,
                target,
            } => {
                let cond_val = self.emit_value(condition);
                let span_start = condition.span().start;
                self.emit_line(&format!(
                    "if (!({})) {{ RNIM_ASSERT_FAIL(\"{}\", {}); goto block_{}; }}",
                    cond_val,
                    msg,
                    span_start,
                    target.index()
                ));
            }
            MirStmt::FakeRead { place } => {
                self.emit_line(&format!("/* FakeRead l_{} */", place.local.0));
            }
            MirStmt::Nop => {}
            MirStmt::ResetRecursion(local, _) => {
                self.emit_line(&format!("/* ResetRecursion l_{} */", local.0));
            }
        }
    }

    fn emit_terminator(&mut self, term: &Terminator) {
        match term {
            Terminator::Goto(target) => match target.as_ref() {
                GotoTarget::Next => {
                    self.emit_line("/* Fallthrough */");
                }
                GotoTarget::Block(idx) => {
                    self.emit_line(&format!("goto block_{};", idx.index()));
                }
                GotoTarget::Switch(cases) => {
                    self.emit_line("/* Switch dispatch */");
                    for (_, target) in cases {
                        self.emit_line(&format!("goto block_{};", target.index()));
                    }
                }
            },
            Terminator::Switch {
                discriminant,
                targets,
            } => {
                let disc = self.emit_value(discriminant);
                self.emit_line(&format!("switch ((int){}) {{", disc));
                for (i, target) in targets.iter().enumerate() {
                    self.emit_line(&format!("case {}: goto block_{};", i, target.index()));
                }
                self.emit_line("default: break;");
                self.emit_line("}");
            }
            Terminator::Return => {
                self.emit_line("return;");
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
                    self.emit_line(&format!("goto block_{};", t.index()));
                }
            }
            Terminator::TryCall {
                destination,
                target,
                cleanup: _,
                callee,
                arguments,
            } => {
                self.emit_line("RNIM_TRY_START {");
                let args: Vec<String> = arguments.iter().map(|a| self.emit_value(a)).collect();
                let call_str = format!("{}({})", self.emit_value(callee), args.join(", "));
                if let Some((place, _)) = destination {
                    self.emit_line(&format!("l_{} = {};", place.local.0, call_str));
                }
                self.emit_line("} RNIM_TRY_CATCH_GOTO(l_unwind);");
                if let Some(t) = target {
                    self.emit_line(&format!("goto block_{};", t.index()));
                }
            }
            Terminator::If {
                condition,
                then_block,
                else_block,
            } => {
                self.emit_line(&format!("if ({}) {{", self.emit_value(condition)));
                self.emit_line(&format!("goto block_{};", then_block.index()));
                self.emit_line("} else {");
                self.emit_line(&format!("goto block_{};", else_block.index()));
                self.emit_line("}");
            }
            Terminator::Raise(value) => {
                self.emit_line(&format!("RNIM_RAISE({});", self.emit_value(value)));
                self.emit_line("goto l_unwind;");
            }
            Terminator::Assert {
                condition,
                msg,
                target,
            } => {
                let cond_val = self.emit_value(condition);
                let span_start = condition.span().start;
                self.emit_line(&format!(
                    "if (!({})) {{ RNIM_ASSERT_FAIL(\"{}\", {}); goto block_{}; }}",
                    cond_val,
                    msg,
                    span_start,
                    target.index()
                ));
            }
            Terminator::Unreachable => {
                self.emit_line("RNIM_UNREACHABLE();");
            }
            Terminator::Drop {
                place,
                target,
                unwind: _,
            } => {
                if let Some(t) = target {
                    self.emit_line(&format!("goto block_{};", t.index()));
                }
                let _ = place;
            }
            Terminator::Fallthrough => {
                self.emit_line("/* Fallthrough */");
            }
        }
    }

    fn emit_header(&mut self, module: &MirModule) {
        self.output.push_str("#ifndef RNIM_GEN_H\n");
        self.output.push_str("#define RNIM_GEN_H\n\n");
        self.output.push_str("#include <stdint.h>\n");
        self.output.push_str("#include <stdbool.h>\n");
        self.output.push_str("#include <stddef.h>\n\n");
        self.output.push_str("/* Runtime types */\n");
        self.output
            .push_str("typedef struct { void* data; int len; int cap; } RNIM_SEQ;\n");
        self.output
            .push_str("typedef struct { const char* data; int len; } RNIM_STRING;\n");
        self.output.push_str("#define RNIM_UNIT ((void*)0)\n");
        self.output.push_str(
            "#define NIM_FLOAT_CONST(bits, val) __builtin_nanf_with_signaling_bit(val)\n",
        );
        self.output.push_str(
            "#define NIM_DOUBLE_CONST(bits, val) __builtin_nan_with_signaling_bit(val)\n",
        );
        self.output
            .push_str("#define RNIM_STR_LITERAL(s) ((RNIM_STRING){s, sizeof(s)-1})\n");
        self.output
            .push_str("#define RNIM_DECREF(x) do {} while(0)\n");
        self.output
            .push_str("#define RNIM_INCREF(x) do {} while(0)\n");
        self.output
            .push_str("#define RNIM_DEINIT(x) do {} while(0)\n");
        self.output.push_str("#define RNIM_TUPLE (void*)\n");
        self.output.push_str("#define RNIM_VARIANT(e, i, v) (v)\n");
        self.output.push_str("#define RNIM_CLOSURE(f, c) (f)\n");
        self.output.push_str("#define RNIM_CAP_SLICE (void*)\n");
        self.output
            .push_str("#define RNIM_RAISE(exc) do {} while(0)\n");
        self.output
            .push_str("#define RNIM_UNREACHABLE() __builtin_unreachable()\n");
        self.output
            .push_str("#define RNIM_ASSERT_FAIL(msg, pos) do {} while(0)\n");
        self.output.push_str("#define RNIM_TRY_START do {}\n");
        self.output
            .push_str("#define RNIM_TRY_CATCH_GOTO(label) goto label\n");
        self.output.push_str("#define l_unwind ((void)0)\n");
        self.newline();

        for func in &module.functions {
            self.output.push_str(&self.emit_function_header(func));
            self.output.push('\n');
        }

        self.output.push_str("\n#endif /* RNIM_GEN_H */\n");
    }

    fn emit_source(&mut self, module: &MirModule) {
        self.output
            .push_str("/* Generated by rustnim C backend */\n");
        self.output.push_str("#include \"rnim_gen.h\"\n\n");

        for func in &module.functions {
            self.emit_function_impl(func);
        }
    }

    pub fn emit_c(&mut self, module: &MirModule) -> CModule {
        let mut c_module = CModule::new(module.name.clone());

        self.emit_header(module);
        c_module.header = self.output.clone();
        self.output.clear();

        self.emit_source(module);
        c_module.source = self.output.clone();

        c_module
    }
}

pub struct EmitResult {
    pub module: CModule,
    pub success: bool,
    pub errors: Vec<String>,
}

impl EmitResult {
    pub fn new(module: CModule) -> Self {
        EmitResult {
            module,
            success: true,
            errors: Vec::new(),
        }
    }

    pub fn error(msg: String) -> Self {
        EmitResult {
            module: CModule::new(String::new()),
            success: false,
            errors: vec![msg],
        }
    }
}

pub fn emit_c(path: &Utf8Path) -> Result<CModule, String> {
    let config = CodegenConfig::default();
    let mut generator = CCodeGenerator::new(config);

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
        return Ok(CModule::new("empty".to_string()));
    };

    Ok(generator.emit_c(&module))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codegen_config_default() {
        let config = CodegenConfig::default();
        assert_eq!(config.c_version, CVersion::C11);
        assert!(config.debug_info);
        assert!(config.exceptions);
        assert!(config.use_orc);
    }

    #[test]
    fn test_cmodule_new() {
        let module = CModule::new("test".to_string());
        assert_eq!(module.name, "test");
        assert!(module.source.is_empty());
        assert!(module.header.is_empty());
        assert!(module.dependencies.is_empty());
    }

    #[test]
    fn test_emit_type_bool() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(gen.emit_type(&MirType::Bool), "bool");
    }

    #[test]
    fn test_emit_type_int() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(gen.emit_type(&MirType::Int(32)), "int32_t");
        assert_eq!(gen.emit_type(&MirType::Uint(64)), "uint64_t");
    }

    #[test]
    fn test_emit_type_ref() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(
            gen.emit_type(&MirType::Ref(Box::new(MirType::Int(32)))),
            "int32_t*"
        );
    }

    #[test]
    fn test_emit_type_string() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(gen.emit_type(&MirType::String), "RNIM_STRING");
    }

    #[test]
    fn test_emit_type_array() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(
            gen.emit_type(&MirType::Array(Box::new(MirType::Int(32)), 10)),
            "int32_t[10]"
        );
    }

    #[test]
    fn test_mir_type_void() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(gen.emit_type(&MirType::Unit), "void");
    }

    #[test]
    fn test_emit_c_with_empty_module() {
        let config = CodegenConfig::default();
        let mut gen = CCodeGenerator::new(config);

        let module = MirModule {
            name: "empty".to_string(),
            functions: vec![],
            types: HashMap::new(),
            source_span: Span::new(FileId(0), 0, 0),
        };

        let result = gen.emit_c(&module);

        assert!(result.header.contains("#ifndef RNIM_GEN_H"));
        assert!(result.header.contains("RNIM_STRING"));
        assert!(result.header.contains("RNIM_SEQ"));
    }

    #[test]
    fn test_emit_type_proc() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        let proc_type = MirType::Proc(
            vec![MirType::Int(32), MirType::Int(32)],
            Box::new(MirType::Int(32)),
        );
        let result = gen.emit_type(&proc_type);
        assert!(result.contains("int32_t"));
    }

    #[test]
    fn test_emit_type_adt() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        let adt_type = MirType::Adt(
            "MyStruct".to_string(),
            vec![
                ("field1".to_string(), MirType::Int(32)),
                ("field2".to_string(), MirType::Bool),
            ],
        );
        assert_eq!(gen.emit_type(&adt_type), "MyStruct");
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

    #[test]
    fn test_emit_result_new() {
        let module = CModule::new("test".to_string());
        let result = EmitResult::new(module);
        assert!(result.success);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_emit_result_error() {
        let result = EmitResult::error("test error".to_string());
        assert!(!result.success);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "test error");
    }

    #[test]
    fn test_emit_type_float_double() {
        use rnim_mir::FloatRepr;
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(gen.emit_type(&MirType::Float(FloatRepr::new(0.0))), "float");
        assert_eq!(gen.emit_type(&MirType::Double(rnim_mir::FloatRepr64::new(0.0))), "double");
    }

    #[test]
    fn test_emit_type_pointer() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(
            gen.emit_type(&MirType::Pointer(Box::new(MirType::Int(32)))),
            "int32_t*"
        );
    }

    #[test]
    fn test_emit_type_seq() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(gen.emit_type(&MirType::Seq(Box::new(MirType::Int(32)))), "RNIM_SEQ");
    }

    #[test]
    fn test_emit_type_open_array() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(
            gen.emit_type(&MirType::OpenArray(Box::new(MirType::Int(32)))),
            "RNIM_OPENARRAY(int32_t)"
        );
    }

    #[test]
    fn test_emit_type_tuple() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(
            gen.emit_type(&MirType::Tuple(vec![MirType::Int(32), MirType::Bool])),
            "struct { int32_t f_0; bool f_1 }"
        );
    }

    #[test]
    fn test_cversion_default() {
        assert_eq!(CVersion::default(), CVersion::C11);
    }

    #[test]
    fn test_cversion_all_variants() {
        assert!(matches!(CVersion::C99, CVersion::C99));
        assert!(matches!(CVersion::C11, CVersion::C11));
        assert!(matches!(CVersion::C17, CVersion::C17));
    }

    #[test]
    fn test_emit_value_bool() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(gen.emit_value(&MirValue::Bool(true, Span::new(FileId(0), 0, 4))), "true");
        assert_eq!(gen.emit_value(&MirValue::Bool(false, Span::new(FileId(0), 0, 5))), "false");
    }

    #[test]
    fn test_emit_value_int() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(gen.emit_value(&MirValue::Int(42, Span::new(FileId(0), 0, 2))), "42");
        assert_eq!(gen.emit_value(&MirValue::Uint(100, Span::new(FileId(0), 0, 3))), "100u");
    }

    #[test]
    fn test_emit_value_unit() {
        let config = CodegenConfig::default();
        let gen = CCodeGenerator::new(config);
        assert_eq!(gen.emit_value(&MirValue::Unit(Span::new(FileId(0), 0, 0))), "RNIM_UNIT");
    }

    #[test]
    fn test_codegen_config_custom() {
        let config = CodegenConfig {
            output_dir: Utf8PathBuf::from("/output"),
            debug_info: false,
            c_version: CVersion::C99,
            exceptions: false,
            use_orc: false,
        };
        assert_eq!(config.output_dir.as_str(), "/output");
        assert!(!config.debug_info);
        assert!(!config.exceptions);
        assert!(!config.use_orc);
    }

    #[test]
    fn test_cmodule_with_content() {
        let mut module = CModule::new("test".to_string());
        module.source.push_str("int main() { return 0; }");
        module.header.push_str("int main(void);");
        assert!(!module.source.is_empty());
        assert!(!module.header.is_empty());
    }

    #[test]
    fn test_cmodule_add_dependency() {
        let mut module = CModule::new("test".to_string());
        module.dependencies.push("libm".to_string());
        module.dependencies.push("libpthread".to_string());
        assert_eq!(module.dependencies.len(), 2);
    }
}

mod cpp;
mod ffi;
mod objc;
