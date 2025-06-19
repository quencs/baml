/// Max call stack size.
///
/// Could be dynamic but we still need to set a limit and error with stack
/// overflow if reached.
const MAX_FRAMES: usize = 256;

/// Individual bytecode instruction.
///
/// For faster iteration we'll start with an in-memory data structure that
/// represents the bytecode instead of real binary instructions since getting
/// those to work correctly is much harder (unsafe Rust, pointer arithmetic).
///
/// We do need to respect some sort of "instruction format" however. In
/// stack-based VMs some instructions don't take any arguments (for example,
/// the `ADD` instruction would grab its operands from the evaluation stack),
/// but some others such as `LOAD_CONST` need to know which constant to load,
/// so they take an unsigned integer as an argument (the index of the constant
/// in the constant pool). Same goes for jump instructions, we need to know the
/// offset.
///
/// We are not limited to one single argument, we can have variable-length
/// instructions in the VM, but we do have to keep the arguments limited to
/// "bytes" (unsigned integers, signed integers, etc). Use the arguments to
/// index into runtime structures such as constant pools, object pools, etc.
/// Don't embed complex data structures in an instruction. Avoid this:
///
/// ```ignore
/// enum Instruction {
///     MySuperDuperInstruction(HashMap<String, Vec<Function>>)
/// }
/// ```
///
/// Instead store the state or complex structure in the [`Vm`] struct and find a
/// way to reference it with very simple instructions.
#[derive(Clone)]
enum Instruction {
    /// Loads a constant from the bytecode's constant pool.
    ///
    /// Format: `LOAD_CONST i` where `i` is the index of the constant in the
    /// [`Bytecode::constants`] pool.
    LoadConst(usize),

    /// Loads a variable from the frame's local variable slots.
    ///
    /// Format: `LOAD_VAR i` where `i` is the index of the variable in the
    /// [`Frame::locals`] array.
    LoadVar(usize),

    /// Stores a value in the frame's local variable slots.
    ///
    /// Format: `STORE_VAR i` where `i` is the index of the variable in the
    /// [`Frame::locals`] array.
    StoreVar(usize),

    /// Pop the top of [`Vm::stack`] (the evaluation stack).
    Pop,

    /// Jump to another instruction.
    ///
    /// Format: `JUMP o` where `o` is the offset from the current instruction
    /// to the target instruction (can be negative to jump backwards).
    Jump(isize),

    /// Jump to another instruction if the top of [`Vm::stack`] is false.
    ///
    /// Format: `JUMP_IF_FALSE o` where `o` is the offset from the current
    /// instruction to the target instruction (can be negative to jump
    /// backwards).
    JumpIfFalse(isize),

    /// Load a global variable from the [`Vm::globals`] array.
    ///
    /// Format: `LOAD_GLOBAL i` where `i` is the index of the global variable
    /// in the [`Vm::globals`] array.
    ///
    /// Note that functions are also globals and can be passed around and stored
    /// in local variables, so we need to load their name in the stack before we
    /// call the function.
    LoadGlobal(usize),

    /// Store a value in a global variable.
    ///
    /// Format: `STORE_GLOBAL i` where `i` is the index of the global variable
    /// in the [`Vm::globals`] array.
    StoreGlobal(usize),

    /// Call a function.
    ///
    /// Format: `CALL n` where `n` is the number of arguments passed to the
    /// function.
    ///
    /// Arguments are pushed onto the eval stack and the name of the function
    /// is right below them.
    Call(usize),

    /// Return from a function.
    ///
    /// No arguments needed, result is stored in the eval stack and the VM
    /// simply has to clean up the call stack and continue execution.
    Return,
}

/// Runtime values.
#[derive(Clone)]
enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Object(Object),
}

/// Executable bytecode.
#[derive(Clone)]
struct Bytecode {
    /// Sequence of instructions.
    instructions: Vec<Instruction>,

    /// Constant pool.
    constants: Vec<Value>,
}

/// Function type.
#[derive(Clone)]
enum FunctionKind {
    /// Regular executable function.
    ///
    /// The VM pushes a call frame onto the call stack and runs the bytecode.
    Exec,

    /// LLM function.
    ///
    /// The VM will handle control flow to the Baml runtime to produce the
    /// result and then push it on top of the eval stack.
    Llm,

    /// OS interfacing function.
    ///
    /// VM will handle control flow to a Rust wrapper that calls into the OS
    /// and returns a result. Needed for features like `fetch`.
    Native,
}

/// Represents any Baml function.
#[derive(Clone)]
struct Function {
    name: String,
    arity: usize,
    bytecode: Bytecode,
    kind: FunctionKind,
}

/// Any data that the Baml program can reference.
///
/// VM should own objects and give references to them to the running Baml
/// program.
#[derive(Clone)]
enum Object {
    Function(Function),
    String(String),
    // TODO: Classes, instances, etc.
}

/// Call frame.
///
/// This is what gets pushed onto the call stack every time we call a function.
struct Frame {
    /// The running function.
    function: Function,

    /// Instruction pointer (IP) or program counter (PC).
    ///
    /// Points to the next instruction that the VM will execute.
    instruction_ptr: usize,

    /// Local variables offset in the eval stack.
    locals_offset: usize,
}

/// The beast.
struct Vm {
    /// Call stack.
    frames: Vec<Frame>,

    /// Evaluation stack.
    stack: Vec<Value>,

    /// Objects.
    objects: Vec<Object>,

    /// Global variables.
    globals: Vec<Value>,
}

enum RuntimeError {
    StackOverflow,
    InvalidArgumentCount { expected: usize, got: usize },
    InternalError,
}

enum VmError {
    RuntimeError(RuntimeError),
}

impl From<RuntimeError> for VmError {
    fn from(error: RuntimeError) -> Self {
        VmError::RuntimeError(error)
    }
}

impl Vm {
    fn exec(&mut self) -> Result<(), VmError> {
        let Some(mut frame) = self.frames.last_mut() else {
            return Ok(());
        };

        loop {
            // Set to one by default and increment if we're executing a jump
            // instruction.
            let mut instruction_offset = 1;

            match frame.function.bytecode.instructions[frame.instruction_ptr] {
                Instruction::LoadConst(index) => {
                    let value = &frame.function.bytecode.constants[index];
                    self.stack.push(value.clone());
                }

                Instruction::LoadVar(index) => {
                    let value = &self.stack[frame.locals_offset + index];
                    self.stack.push(value.clone());
                }

                Instruction::StoreVar(index) => {
                    let value = self.stack.last().unwrap();
                    self.stack[frame.locals_offset + index] = value.clone();
                }

                Instruction::Pop => {
                    self.stack.pop();
                }

                Instruction::Jump(offset) => {
                    instruction_offset = offset;
                }

                Instruction::JumpIfFalse(offset) => match self.stack.last() {
                    Some(Value::Bool(value)) => {
                        if !value {
                            instruction_offset = offset;
                        }
                    }

                    // Type error, we don't have "falsey" values in the language
                    // so we should always check booleans.
                    _ => return Err(VmError::RuntimeError(RuntimeError::InternalError)),
                },

                Instruction::LoadGlobal(index) => {
                    let value = &self.globals[index];
                    self.stack.push(value.clone());
                }

                Instruction::StoreGlobal(index) => {
                    let value = self.stack.last().unwrap();
                    self.globals[index] = value.clone();
                }

                Instruction::Call(arg_count) => {
                    let Value::Object(Object::Function(function)) =
                        &self.stack[self.stack.len() - arg_count - 1]
                    else {
                        return Err(VmError::RuntimeError(RuntimeError::InternalError));
                    };

                    if arg_count != function.arity {
                        return Err(VmError::RuntimeError(RuntimeError::InvalidArgumentCount {
                            expected: function.arity,
                            got: arg_count,
                        }));
                    }

                    if self.frames.len() >= MAX_FRAMES {
                        return Err(VmError::RuntimeError(RuntimeError::StackOverflow));
                    }

                    let locals_offset = if self.stack.is_empty() {
                        0
                    } else {
                        self.stack.len() - arg_count - 1
                    };

                    self.frames.push(Frame {
                        function: function.clone(),
                        instruction_ptr: 0,
                        locals_offset,
                    });

                    frame = self.frames.last_mut().unwrap();
                }

                Instruction::Return => {
                    // Pop the result from the eval stack.
                    let Some(result) = self.stack.pop() else {
                        return Err(VmError::RuntimeError(RuntimeError::InternalError));
                    };

                    // Restore the eval stack to the state before the function
                    // was called and leave the result on top.
                    self.stack.drain(frame.locals_offset..);
                    self.stack.push(result);

                    // Pop from the call stack.
                    self.frames.pop();

                    // If there are no more frames, we're done.
                    let Some(previous_frame) = self.frames.last_mut() else {
                        return Ok(());
                    };

                    // Resume previous frame execution.
                    frame = previous_frame;
                }
            }

            // Move to next instruction.
            frame.instruction_ptr = (frame.instruction_ptr as isize + instruction_offset) as usize;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bytecode() {
        assert_eq!(1, 1);
    }
}
