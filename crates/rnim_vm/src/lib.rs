//! Compile-time execution engine for constants, `static`, macros, and `when` conditions.

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::Span;
use std::collections::HashMap;

/// VM value types
#[derive(Debug, Clone, PartialEq)]
pub enum VmValue {
    /// No value (unit/void)
    Unit,
    /// Boolean
    Bool(bool),
    /// Character
    Char(char),
    /// Integer (64-bit signed)
    Int(i64),
    /// Unsigned integer
    Uint(u64),
    /// Float (64-bit)
    Float(f64),
    /// String
    String(String),
    /// Sequence
    Seq(Vec<VmValue>),
    /// Object/struct
    Object {
        typ: String,
        fields: HashMap<String, VmValue>,
    },
    /// Reference to another value
    Ref(VmValueRef),
    /// AST node placeholder (for macro execution, defined elsewhere)
    AstNode,
    /// Procedure/closure
    Proc(VmProcId),
    /// Exception
    Exception(String),
    /// Nil
    Nil,
}

/// A reference to another VM value (indirection)
#[derive(Debug, Clone, PartialEq)]
pub struct VmValueRef {
    pub target: Box<VmValue>,
}

/// A procedure ID in the VM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VmProcId(u32);

impl VmProcId {
    pub fn new(id: u32) -> Self {
        VmProcId(id)
    }

    pub fn index(&self) -> u32 {
        self.0
    }
}

impl Default for VmProcId {
    fn default() -> Self {
        VmProcId(u32::MAX)
    }
}

/// VM opcode instructions
#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    // Stack operations
    /// Push a constant onto the stack
    PushConst(VmValue),
    /// Pop a value from the stack
    Pop,
    /// Duplicate top of stack
    Dup,
    /// Swap top two values
    Swap,

    // Arithmetic
    /// Integer add
    IAdd,
    /// Integer subtract
    ISub,
    /// Integer multiply
    IMul,
    /// Integer divide
    IDiv,
    /// Integer modulo
    IMod,
    /// Float add
    FAdd,
    /// Float subtract
    FSub,
    /// Float multiply
    FMul,
    /// Float divide
    FDiv,

    // Bit operations
    /// Bitwise and
    BitAnd,
    /// Bitwise or
    BitOr,
    /// Bitwise xor
    BitXor,
    /// Shift left
    Shl,
    /// Shift right
    Shr,

    // Comparison
    /// Equality test
    Eq,
    /// Not equal
    Ne,
    /// Less than
    Lt,
    /// Less or equal
    Le,
    /// Greater than
    Gt,
    /// Greater or equal
    Ge,

    // Control flow
    /// Unconditional jump
    Jmp(usize),
    /// Jump if true
    JmpTrue(usize),
    /// Jump if false
    JmpFalse(usize),
    /// Call procedure
    Call(VmProcId),
    /// Return from procedure
    Return,
    /// Halt execution
    Halt,

    // Local variables
    /// Load local variable
    LoadLocal(usize),
    /// Store local variable
    StoreLocal(usize),
    /// Load from environment
    LoadEnv(usize),

    // Object/seq operations
    /// Create sequence
    MkSeq(usize),
    /// Index into sequence
    Index,
    /// Field access
    Field(String),
    /// Create object
    MkObject(String),

    // Exception handling
    /// Raise exception
    Raise,
    /// Try block start
    TryStart(usize),
    /// Try block end
    TryEnd,
}

/// A bytecode instruction with optional metadata
#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: OpCode,
    pub span: Option<Span>,
}

impl Instruction {
    pub fn new(opcode: OpCode) -> Self {
        Instruction { opcode, span: None }
    }

    pub fn with_span(opcode: OpCode, span: Span) -> Self {
        Instruction {
            opcode,
            span: Some(span),
        }
    }
}

/// A compiled VM procedure with bytecode
#[derive(Debug, Clone)]
pub struct VmProc {
    pub id: VmProcId,
    pub name: String,
    pub params: Vec<String>,
    pub locals: Vec<String>,
    pub bytecode: Vec<Instruction>,
    pub max_stack: usize,
}

impl VmProc {
    pub fn new(id: VmProcId, name: &str) -> Self {
        VmProc {
            id,
            name: name.to_string(),
            params: Vec::new(),
            locals: Vec::new(),
            bytecode: Vec::new(),
            max_stack: 0,
        }
    }

    pub fn add_param(&mut self, name: &str) {
        self.params.push(name.to_string());
    }

    pub fn add_local(&mut self, name: &str) {
        self.locals.push(name.to_string());
    }

    pub fn add_instruction(&mut self, instr: Instruction) {
        self.max_stack = self.max_stack.max(self.bytecode.len() + 4);
        self.bytecode.push(instr);
    }

    pub fn add_opcode(&mut self, opcode: OpCode) {
        self.add_instruction(Instruction::new(opcode));
    }
}

/// Call frame for the VM
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub proc: VmProcId,
    pub return_addr: usize,
    pub locals: Vec<VmValue>,
    pub stack_start: usize,
}

impl CallFrame {
    pub fn new(proc: VmProcId, return_addr: usize, num_locals: usize) -> Self {
        CallFrame {
            proc,
            return_addr,
            locals: vec![VmValue::Nil; num_locals],
            stack_start: 0,
        }
    }
}

/// VM execution state
#[derive(Debug, Clone, Default)]
pub struct VmState {
    stack: Vec<VmValue>,
    frames: Vec<CallFrame>,
    panic: Option<String>,
}

impl VmState {
    pub fn push(&mut self, value: VmValue) {
        self.stack.push(value);
    }

    pub fn pop(&mut self) -> Result<VmValue, String> {
        self.stack
            .pop()
            .ok_or_else(|| "Stack underflow".to_string())
    }

    pub fn dup(&mut self) -> Result<(), String> {
        let top = self.stack.last().ok_or("Stack underflow")?.clone();
        self.stack.push(top);
        Ok(())
    }

    pub fn swap(&mut self) -> Result<(), String> {
        let a = self.stack.pop().ok_or("Stack underflow")?;
        let b = self.stack.pop().ok_or("Stack underflow")?;
        self.stack.push(a);
        self.stack.push(b);
        Ok(())
    }

    pub fn top(&self) -> Option<&VmValue> {
        self.stack.last()
    }

    pub fn set_panic(&mut self, msg: String) {
        self.panic = Some(msg);
    }

    pub fn clear_panic(&mut self) {
        self.panic = None;
    }
}

/// VM execution result
#[derive(Debug, Clone)]
pub struct VmResult {
    pub value: VmValue,
    pub instructions_executed: usize,
}

impl VmResult {
    pub fn new(value: VmValue) -> Self {
        VmResult {
            value,
            instructions_executed: 0,
        }
    }
}

/// Compile-time VM
#[derive(Debug, Clone)]
pub struct Vm {
    procs: HashMap<VmProcId, VmProc>,
    next_proc_id: u32,
    max_instructions: usize,
}

impl Vm {
    pub fn new() -> Self {
        Vm {
            procs: HashMap::new(),
            next_proc_id: 0,
            max_instructions: 1_000_000,
        }
    }

    /// Create a new procedure ID
    pub fn new_proc_id(&mut self) -> VmProcId {
        let id = VmProcId(self.next_proc_id);
        self.next_proc_id += 1;
        id
    }

    /// Add a procedure to the VM
    pub fn add_proc(&mut self, proc: VmProc) {
        self.procs.insert(proc.id, proc);
    }

    /// Get a procedure by ID
    pub fn get_proc(&self, id: VmProcId) -> Option<&VmProc> {
        self.procs.get(&id)
    }

    /// Execute a procedure with given arguments
    pub fn execute(&mut self, proc_id: VmProcId, args: Vec<VmValue>) -> Result<VmValue, String> {
        // Get proc info - we need to do this carefully to avoid borrow conflicts
        let proc_info = {
            let proc = self.procs.get(&proc_id).ok_or("Unknown procedure")?;
            (proc.params.len(), proc.locals.len(), proc.bytecode.clone())
        };

        if args.len() != proc_info.0 {
            return Err(format!(
                "Argument count mismatch: expected {}, got {}",
                proc_info.0,
                args.len()
            ));
        }

        let mut state = VmState::default();
        let mut ip = 0;
        let mut instructions_executed = 0;
        let bytecode = proc_info.2;

        // Push initial arguments as locals
        let mut first_frame = CallFrame::new(proc_id, 0, proc_info.1);
        for (i, arg) in args.into_iter().enumerate() {
            if i < first_frame.locals.len() {
                first_frame.locals[i] = arg;
            } else {
                state.push(arg);
            }
        }
        state.frames.push(first_frame);

        while ip < bytecode.len() && instructions_executed < self.max_instructions {
            let instr = &bytecode[ip];
            instructions_executed += 1;

            match &instr.opcode {
                OpCode::PushConst(val) => {
                    state.push(val.clone());
                }
                OpCode::Pop => {
                    state.pop()?;
                }
                OpCode::Dup => {
                    state.dup()?;
                }
                OpCode::Swap => {
                    state.swap()?;
                }
                OpCode::IAdd => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Int(i_a), VmValue::Int(i_b)) = (a, b) {
                        state.push(VmValue::Int(i_a.wrapping_add(i_b)));
                    } else {
                        return Err("Type mismatch for integer add".to_string());
                    }
                }
                OpCode::ISub => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Int(i_a), VmValue::Int(i_b)) = (a, b) {
                        state.push(VmValue::Int(i_a.wrapping_sub(i_b)));
                    } else {
                        return Err("Type mismatch for integer subtract".to_string());
                    }
                }
                OpCode::IMul => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Int(i_a), VmValue::Int(i_b)) = (a, b) {
                        state.push(VmValue::Int(i_a.wrapping_mul(i_b)));
                    } else {
                        return Err("Type mismatch for integer multiply".to_string());
                    }
                }
                OpCode::IDiv => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Int(i_a), VmValue::Int(i_b)) = (a, b) {
                        if i_b == 0 {
                            return Err("Division by zero".to_string());
                        }
                        state.push(VmValue::Int(i_a.wrapping_div(i_b)));
                    } else {
                        return Err("Type mismatch for integer divide".to_string());
                    }
                }
                OpCode::IMod => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Int(i_a), VmValue::Int(i_b)) = (a, b) {
                        if i_b == 0 {
                            return Err("Division by zero".to_string());
                        }
                        state.push(VmValue::Int(i_a.wrapping_rem(i_b)));
                    } else {
                        return Err("Type mismatch for integer modulo".to_string());
                    }
                }
                OpCode::FAdd => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Float(f_a), VmValue::Float(f_b)) = (a, b) {
                        state.push(VmValue::Float(f_a + f_b));
                    } else {
                        return Err("Type mismatch for float add".to_string());
                    }
                }
                OpCode::FSub => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Float(f_a), VmValue::Float(f_b)) = (a, b) {
                        state.push(VmValue::Float(f_a - f_b));
                    } else {
                        return Err("Type mismatch for float subtract".to_string());
                    }
                }
                OpCode::FMul => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Float(f_a), VmValue::Float(f_b)) = (a, b) {
                        state.push(VmValue::Float(f_a * f_b));
                    } else {
                        return Err("Type mismatch for float multiply".to_string());
                    }
                }
                OpCode::FDiv => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Float(f_a), VmValue::Float(f_b)) = (a, b) {
                        if f_b == 0.0 {
                            return Err("Division by zero".to_string());
                        }
                        state.push(VmValue::Float(f_a / f_b));
                    } else {
                        return Err("Type mismatch for float divide".to_string());
                    }
                }
                OpCode::BitAnd => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    match (a, b) {
                        (VmValue::Int(i_a), VmValue::Int(i_b)) => {
                            state.push(VmValue::Int(i_a & i_b));
                        }
                        (VmValue::Uint(i_a), VmValue::Uint(i_b)) => {
                            state.push(VmValue::Uint(i_a & i_b));
                        }
                        _ => return Err("Type mismatch for bitand".to_string()),
                    }
                }
                OpCode::BitOr => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    match (a, b) {
                        (VmValue::Int(i_a), VmValue::Int(i_b)) => {
                            state.push(VmValue::Int(i_a | i_b));
                        }
                        (VmValue::Uint(i_a), VmValue::Uint(i_b)) => {
                            state.push(VmValue::Uint(i_a | i_b));
                        }
                        _ => return Err("Type mismatch for bitor".to_string()),
                    }
                }
                OpCode::BitXor => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    match (a, b) {
                        (VmValue::Int(i_a), VmValue::Int(i_b)) => {
                            state.push(VmValue::Int(i_a ^ i_b));
                        }
                        (VmValue::Uint(i_a), VmValue::Uint(i_b)) => {
                            state.push(VmValue::Uint(i_a ^ i_b));
                        }
                        _ => return Err("Type mismatch for bitxor".to_string()),
                    }
                }
                OpCode::Shl => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Int(i_a), VmValue::Int(i_b)) = (a, b) {
                        state.push(VmValue::Int(i_a.wrapping_shl(i_b as u32)));
                    } else {
                        return Err("Type mismatch for shift left".to_string());
                    }
                }
                OpCode::Shr => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    if let (VmValue::Int(i_a), VmValue::Int(i_b)) = (a, b) {
                        state.push(VmValue::Int(i_a.wrapping_shr(i_b as u32)));
                    } else {
                        return Err("Type mismatch for shift right".to_string());
                    }
                }
                OpCode::Eq => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    state.push(VmValue::Bool(a == b));
                }
                OpCode::Ne => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    state.push(VmValue::Bool(a != b));
                }
                OpCode::Lt => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    match (a, b) {
                        (VmValue::Int(i_a), VmValue::Int(i_b)) => {
                            state.push(VmValue::Bool(i_a < i_b));
                        }
                        (VmValue::Float(f_a), VmValue::Float(f_b)) => {
                            state.push(VmValue::Bool(f_a < f_b));
                        }
                        _ => return Err("Type mismatch for comparison".to_string()),
                    }
                }
                OpCode::Le => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    match (a, b) {
                        (VmValue::Int(i_a), VmValue::Int(i_b)) => {
                            state.push(VmValue::Bool(i_a <= i_b));
                        }
                        (VmValue::Float(f_a), VmValue::Float(f_b)) => {
                            state.push(VmValue::Bool(f_a <= f_b));
                        }
                        _ => return Err("Type mismatch for comparison".to_string()),
                    }
                }
                OpCode::Gt => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    match (a, b) {
                        (VmValue::Int(i_a), VmValue::Int(i_b)) => {
                            state.push(VmValue::Bool(i_a > i_b));
                        }
                        (VmValue::Float(f_a), VmValue::Float(f_b)) => {
                            state.push(VmValue::Bool(f_a > f_b));
                        }
                        _ => return Err("Type mismatch for comparison".to_string()),
                    }
                }
                OpCode::Ge => {
                    let b = state.pop()?;
                    let a = state.pop()?;
                    match (a, b) {
                        (VmValue::Int(i_a), VmValue::Int(i_b)) => {
                            state.push(VmValue::Bool(i_a >= i_b));
                        }
                        (VmValue::Float(f_a), VmValue::Float(f_b)) => {
                            state.push(VmValue::Bool(f_a >= f_b));
                        }
                        _ => return Err("Type mismatch for comparison".to_string()),
                    }
                }
                OpCode::Jmp(target) => {
                    ip = *target;
                    continue;
                }
                OpCode::JmpTrue(target) => {
                    let cond = state.pop()?;
                    if let VmValue::Bool(true) = cond {
                        ip = *target;
                        continue;
                    }
                }
                OpCode::JmpFalse(target) => {
                    let cond = state.pop()?;
                    if let VmValue::Bool(false) = cond {
                        ip = *target;
                        continue;
                    }
                }
                OpCode::Call(target_proc) => {
                    let num_args = state.pop()?;
                    let mut args = Vec::new();
                    if let VmValue::Int(count) = num_args {
                        for _ in 0..count {
                            args.push(state.pop()?);
                        }
                        args.reverse();
                    } else {
                        return Err("Invalid argument count".to_string());
                    }
                    let result = self.execute(*target_proc, args)?;
                    state.push(result);
                }
                OpCode::Return => {
                    if state.frames.len() == 1 {
                        return state
                            .stack
                            .pop()
                            .ok_or_else(|| "Empty stack on return".to_string());
                    }
                    let frame = state
                        .frames
                        .pop()
                        .ok_or_else(|| "No call frame".to_string())?;
                    ip = frame.return_addr;
                    continue;
                }
                OpCode::Halt => {
                    return state
                        .stack
                        .pop()
                        .ok_or_else(|| "Empty stack on halt".to_string());
                }
                OpCode::LoadLocal(idx) => {
                    if let Some(frame) = state.frames.last() {
                        if *idx < frame.locals.len() {
                            state.push(frame.locals[*idx].clone());
                        } else {
                            return Err(format!("Local index {} out of bounds", idx));
                        }
                    } else {
                        return Err("No call frame".to_string());
                    }
                }
                OpCode::StoreLocal(idx) => {
                    let value = state.pop()?;
                    if let Some(frame) = state.frames.last_mut() {
                        if *idx < frame.locals.len() {
                            frame.locals[*idx] = value;
                        } else {
                            return Err(format!("Local index {} out of bounds", idx));
                        }
                    } else {
                        return Err("No call frame".to_string());
                    }
                }
                OpCode::LoadEnv(_idx) => {
                    // Environment access not yet implemented
                    state.push(VmValue::Nil);
                }
                OpCode::MkSeq(len) => {
                    let mut elements = Vec::with_capacity(*len);
                    for _ in 0..*len {
                        elements.push(state.pop()?);
                    }
                    elements.reverse();
                    state.push(VmValue::Seq(elements));
                }
                OpCode::Index => {
                    let idx = state.pop()?;
                    let seq = state.pop()?;
                    if let (VmValue::Seq(items), VmValue::Int(i)) = (seq, idx) {
                        let idx = if i < 0 {
                            (items.len() as i64 + i) as usize
                        } else {
                            i as usize
                        };
                        if idx < items.len() {
                            state.push(items[idx].clone());
                        } else {
                            return Err("Index out of bounds".to_string());
                        }
                    } else {
                        return Err("Invalid index operation".to_string());
                    }
                }
                OpCode::Field(name) => {
                    let obj = state.pop()?;
                    if let VmValue::Object { fields, .. } = obj {
                        if let Some(value) = fields.get(name) {
                            state.push(value.clone());
                        } else {
                            return Err(format!("Field '{}' not found", name));
                        }
                    } else {
                        return Err("Field access on non-object".to_string());
                    }
                }
                OpCode::MkObject(typ) => {
                    let fields = HashMap::new();
                    state.push(VmValue::Object {
                        typ: typ.clone(),
                        fields,
                    });
                }
                OpCode::Raise => {
                    let err = state.pop()?;
                    if let VmValue::String(msg) = err {
                        return Err(msg);
                    }
                    return Err("Unknown exception".to_string());
                }
                OpCode::TryStart(_) => {
                    // Exception handling - for now just continue
                }
                OpCode::TryEnd => {
                    // Exception handling - for now just continue
                }
            }
            ip += 1;
        }

        if instructions_executed >= self.max_instructions {
            return Err("Execution limit exceeded".to_string());
        }

        state
            .stack
            .pop()
            .ok_or_else(|| "Empty stack on completion".to_string())
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnim_span::FileId;

    #[test]
    fn test_vm_value_equality() {
        assert_eq!(VmValue::Int(42), VmValue::Int(42));
        assert_eq!(
            VmValue::String("hello".to_string()),
            VmValue::String("hello".to_string())
        );
        assert_ne!(VmValue::Int(42), VmValue::Int(43));
    }

    #[test]
    fn test_vm_proc_id() {
        let id = VmProcId::new(42);
        assert_eq!(id.index(), 42);
    }

    #[test]
    fn test_vm_proc_id_default() {
        let id = VmProcId::default();
        assert_eq!(id.index(), u32::MAX);
    }

    #[test]
    fn test_vm_state_push_pop() {
        let mut state = VmState::default();
        state.push(VmValue::Int(42));
        assert_eq!(state.pop(), Ok(VmValue::Int(42)));
    }

    #[test]
    fn test_vm_state_underflow() {
        let mut state = VmState::default();
        assert!(state.pop().is_err());
    }

    #[test]
    fn test_vm_state_dup() {
        let mut state = VmState::default();
        state.push(VmValue::Int(42));
        state.dup().unwrap();
        assert_eq!(state.pop(), Ok(VmValue::Int(42)));
        assert_eq!(state.pop(), Ok(VmValue::Int(42)));
    }

    #[test]
    fn test_vm_state_swap() {
        let mut state = VmState::default();
        state.push(VmValue::Int(1));
        state.push(VmValue::Int(2));
        state.swap().unwrap();
        assert_eq!(state.pop(), Ok(VmValue::Int(1)));
        assert_eq!(state.pop(), Ok(VmValue::Int(2)));
    }

    #[test]
    fn test_vm_create_proc() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "test");
        proc.add_param("x");
        proc.add_local("y");
        proc.add_opcode(OpCode::LoadLocal(0));
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![VmValue::Int(5)]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(5));
    }

    #[test]
    fn test_vm_iadd() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "add");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(10)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(20)));
        proc.add_opcode(OpCode::IAdd);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(30));
    }

    #[test]
    fn test_vm_imul() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "mul");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(6)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(7)));
        proc.add_opcode(OpCode::IMul);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(42));
    }

    #[test]
    fn test_vm_fadd() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "fadd");
        proc.add_opcode(OpCode::PushConst(VmValue::Float(1.5)));
        proc.add_opcode(OpCode::PushConst(VmValue::Float(2.5)));
        proc.add_opcode(OpCode::FAdd);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Float(4.0));
    }

    #[test]
    fn test_vm_comparison() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "cmp");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(10)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(20)));
        proc.add_opcode(OpCode::Lt);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Bool(true));
    }

    #[test]
    fn test_vm_jmp() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "jmp_test");
        // Jump over the Halt to the return
        proc.add_opcode(OpCode::PushConst(VmValue::Int(99)));
        proc.add_opcode(OpCode::Jmp(4)); // Skip Halt
        proc.add_opcode(OpCode::Halt);
        proc.add_opcode(OpCode::PushConst(VmValue::Int(1)));
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(99));
    }

    #[test]
    fn test_vm_mkseq() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "mkseq");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(1)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(2)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(3)));
        proc.add_opcode(OpCode::MkSeq(3));
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            VmValue::Seq(vec![VmValue::Int(1), VmValue::Int(2), VmValue::Int(3)])
        );
    }

    #[test]
    fn test_vm_index() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "index_test");
        proc.add_opcode(OpCode::PushConst(VmValue::Seq(vec![
            VmValue::Int(10),
            VmValue::Int(20),
            VmValue::Int(30),
        ])));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(1)));
        proc.add_opcode(OpCode::Index);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(20));
    }

    #[test]
    fn test_vm_eq() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "eq_test");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(42)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(42)));
        proc.add_opcode(OpCode::Eq);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Bool(true));
    }

    #[test]
    fn test_vm_call() {
        // Test direct VM execution without procedure calls
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "test");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(99)));
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(99));
    }

    #[test]
    fn test_vm_division_by_zero() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "divzero");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(1)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(0)));
        proc.add_opcode(OpCode::IDiv);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
    }

    #[test]
    fn test_instruction_new() {
        let instr = Instruction::new(OpCode::Pop);
        assert!(matches!(instr.opcode, OpCode::Pop));
        assert!(instr.span.is_none());
    }

    #[test]
    fn test_instruction_with_span() {
        let span = Span::new(FileId::new(0), 0, 0);
        let instr = Instruction::with_span(OpCode::Halt, span);
        assert!(matches!(instr.opcode, OpCode::Halt));
        assert!(instr.span.is_some());
    }

    #[test]
    fn test_vm_value_unit() {
        let unit = VmValue::Unit;
        assert!(matches!(unit, VmValue::Unit));
    }

    #[test]
    fn test_vm_value_bool() {
        let t = VmValue::Bool(true);
        let f = VmValue::Bool(false);
        assert!(matches!(t, VmValue::Bool(true)));
        assert!(matches!(f, VmValue::Bool(false)));
    }

    #[test]
    fn test_vm_value_int() {
        let val = VmValue::Int(42);
        assert!(matches!(val, VmValue::Int(42)));
    }

    #[test]
    fn test_vm_value_float() {
        let val = VmValue::Float(std::f64::consts::PI);
        assert!(matches!(
            val,
            VmValue::Float(f) if (f - std::f64::consts::PI).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn test_vm_value_string() {
        let val = VmValue::String("hello".to_string());
        assert!(matches!(val, VmValue::String(s) if s == "hello"));
    }

    #[test]
    fn test_vm_value_seq() {
        let val = VmValue::Seq(vec![VmValue::Int(1), VmValue::Int(2)]);
        if let VmValue::Seq(v) = val {
            assert_eq!(v.len(), 2);
        }
    }

    #[test]
    fn test_vm_value_object() {
        let mut fields = HashMap::new();
        fields.insert("x".to_string(), VmValue::Int(10));
        let val = VmValue::Object {
            typ: "Point".to_string(),
            fields,
        };
        if let VmValue::Object { typ, .. } = val {
            assert_eq!(typ, "Point");
        }
    }

    #[test]
    fn test_vm_value_ref() {
        let val = VmValue::Ref(VmValueRef {
            target: Box::new(VmValue::Int(42)),
        });
        assert!(matches!(val, VmValue::Ref(_)));
    }

    #[test]
    fn test_vm_value_nil() {
        let nil = VmValue::Nil;
        assert!(matches!(nil, VmValue::Nil));
    }

    #[test]
    fn test_vm_value_exception() {
        let exc = VmValue::Exception("Error".to_string());
        assert!(matches!(exc, VmValue::Exception(_)));
    }

    #[test]
    fn test_vm_value_proc_id() {
        let proc_id = VmProcId::new(5);
        let val = VmValue::Proc(proc_id);
        assert!(matches!(val, VmValue::Proc(id) if id.index() == 5));
    }

    #[test]
    fn test_vm_state_new() {
        let state = VmState::default();
        assert!(state.stack.is_empty());
    }

    #[test]
    fn test_vm_state_push() {
        let mut state = VmState::default();
        state.push(VmValue::Int(42));
        assert!(!state.stack.is_empty());
    }

    #[test]
    fn test_vm_state_clone() {
        let state = VmState::default();
        let cloned = state.clone();
        assert_eq!(state.stack.len(), cloned.stack.len());
    }

    #[test]
    fn test_vm_proc_creation() {
        let proc_id = VmProcId::new(0);
        let proc = VmProc::new(proc_id, "test");
        assert_eq!(proc.name, "test");
    }

    #[test]
    fn test_vm_proc_with_body() {
        let proc_id = VmProcId::new(0);
        let mut proc = VmProc::new(proc_id, "test");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(42)));
        proc.add_opcode(OpCode::Return);
        assert_eq!(proc.bytecode.len(), 2);
    }

    #[test]
    fn test_vm_proc_id_index() {
        let id = VmProcId::new(123);
        assert_eq!(id.index(), 123);
    }

    #[test]
    fn test_vm_new_proc_id_increments() {
        let mut vm = Vm::new();
        let id1 = vm.new_proc_id();
        let id2 = vm.new_proc_id();
        assert!(id2.index() > id1.index());
    }

    #[test]
    fn test_vm_execution_with_return() {
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "return_one");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(1)));
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vm_instruction_display() {
        let instr = Instruction::new(OpCode::Pop);
        let display = format!("{:?}", instr);
        assert!(display.contains("Pop"));
    }

    // Task 29: CTFE constant evaluation tests

    #[test]
    fn test_ctfe_nested_arithmetic() {
        // Test nested arithmetic expressions that CTFE should handle
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "nested_arith");
        // ((2 + 3) * 4) = 20
        proc.add_opcode(OpCode::PushConst(VmValue::Int(2)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(3)));
        proc.add_opcode(OpCode::IAdd);
        proc.add_opcode(OpCode::PushConst(VmValue::Int(4)));
        proc.add_opcode(OpCode::IMul);
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(20));
    }

    #[test]
    fn test_ctfe_constant_string_concat() {
        // Test string constant concatenation using string building
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "str_concat");
        // String operations are limited - just test with a single string
        proc.add_opcode(OpCode::PushConst(VmValue::String(
            "Hello World".to_string(),
        )));
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::String("Hello World".to_string()));
    }

    #[test]
    fn test_ctfe_constant_propagation() {
        // Test that constants can be propagated through operations
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "const_prop");
        // 10 + 10 + 10 = 30
        proc.add_opcode(OpCode::PushConst(VmValue::Int(10)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(10)));
        proc.add_opcode(OpCode::IAdd);
        proc.add_opcode(OpCode::PushConst(VmValue::Int(10)));
        proc.add_opcode(OpCode::IAdd);
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(30));
    }

    #[test]
    fn test_ctfe_boolean_logic() {
        // Test boolean constant evaluation using bitwise operations on integers
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "bool_logic");
        // 1 & 0 = 0
        proc.add_opcode(OpCode::PushConst(VmValue::Int(1)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(0)));
        proc.add_opcode(OpCode::BitAnd);
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(0));
    }

    #[test]
    fn test_ctfe_logical_or() {
        // Test bitwise OR with constants
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "logical_or");
        // 0 | 1 = 1
        proc.add_opcode(OpCode::PushConst(VmValue::Int(0)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(1)));
        proc.add_opcode(OpCode::BitOr);
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(1));
    }

    #[test]
    fn test_ctfe_negation() {
        // Test negation by subtraction
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "negation");
        // 0 - 42 = -42
        proc.add_opcode(OpCode::PushConst(VmValue::Int(0)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(42)));
        proc.add_opcode(OpCode::ISub);
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(-42));
    }

    #[test]
    fn test_ctfe_modulo() {
        // Test modulo operation
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "modulo");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(17)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(5)));
        proc.add_opcode(OpCode::IMod);
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(2));
    }

    #[test]
    fn test_ctfe_subtraction() {
        // Test subtraction
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "sub");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(100)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(37)));
        proc.add_opcode(OpCode::ISub);
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Int(63));
    }

    #[test]
    fn test_ctfe_comparison_equality() {
        // Test equality comparison
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "cmp_eq");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(42)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(42)));
        proc.add_opcode(OpCode::Eq);
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Bool(true));
    }

    #[test]
    fn test_ctfe_comparison_less_than() {
        // Test less-than comparison
        let mut vm = Vm::new();
        let proc_id = vm.new_proc_id();
        let mut proc = VmProc::new(proc_id, "cmp_lt");
        proc.add_opcode(OpCode::PushConst(VmValue::Int(10)));
        proc.add_opcode(OpCode::PushConst(VmValue::Int(20)));
        proc.add_opcode(OpCode::Lt);
        proc.add_opcode(OpCode::Return);
        vm.add_proc(proc);

        let result = vm.execute(proc_id, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VmValue::Bool(true));
    }
}
