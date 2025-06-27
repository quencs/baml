use crate::bytecode::{Bytecode, Instruction};

/// Max call stack size.
const MAX_FRAMES: usize = 256;

/// Function type.
#[derive(Clone, Debug)]
pub enum FunctionKind {
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
#[derive(Clone, Debug)]
pub struct Function {
    /// Function name.
    pub name: String,

    /// Number of arguments the function accepts.
    pub arity: usize,

    /// Bytecode to execute.
    pub bytecode: Bytecode,

    /// Type of function.
    pub kind: FunctionKind,
}

impl std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<fn {}>", self.name)
    }
}

/// Any data that the Baml program can reference and is allocated on heap.
///
/// [`Vm`] should own objects and give references to them to the running Baml
/// program. Internally, in the [`Vm`] code, note that by reference I don't mean
/// a Rust reference (& or &mut), but rather a [`usize`] that is used to index
/// into the [`Vm::objects`] pool.
///
/// Read [`Vm::objects`] for more information.
#[derive(Clone, Debug)]
pub enum Object {
    Function(Function),
    String(String),
    // TODO: Classes, instances, etc.
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Object::Function(function) => function.fmt(f),
            Object::String(string) => string.fmt(f),
        }
    }
}

/// Runtime values.
///
/// This struct should not contain allocated objects and should be [`Copy`].
/// Read the documentation of [`Vm::objects`] to understand how allocated
/// objects work in the virtual machine.
#[derive(Clone, Copy, Debug)]
pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    /// Index into the [`Vm::objects`] vec.
    ///
    /// Strings are also objects, don't add `Value::String`.
    Object(usize),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Int(int) => write!(f, "{int}"),
            Value::Float(float) => write!(f, "{float}"),
            Value::Bool(bool) => write!(f, "{bool}"),
            Value::Object(object) => write!(f, "{object}"),
        }
    }
}

/// Call frame.
///
/// This is what gets pushed onto the call stack every time we call a function.
///
/// As with [`Value`], this struct should not own allocated objects (like
/// functions) but instead use references to index into [`Vm::objects`]. Should
/// be [`Copy`].
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    /// The running function.
    pub function: usize,

    /// Instruction pointer (IP) or program counter (PC).
    ///
    /// Points to the next instruction that the VM will execute. It is of type
    /// [`isize`] because some jumps can create negative offsets (for loops)
    /// and it's easier to operate on an [`isize`] and cast it to [`usize`]
    /// only once (when we index into [`Bytecode::instructions`]). However,
    /// this number should never be negative, otherwise indexing into the
    /// instruction vec will panic.
    pub instruction_ptr: isize,

    /// Local variables offset in the eval stack.
    pub locals_offset: usize,
}

/// The beast.
///
/// This is a stack based virtual machine. Stack based machines work by pushing
/// and popping values from an "evaluation stack". Picture this example from
/// [Crafting Interpreters](https://craftinginterpreters.com/a-virtual-machine.html):
///
/// ```ignore
/// fn echo(n) {
///     print(n)
///     return n
/// }
///
/// print(echo(echo(1) + echo(2)) + echo(echo(4) + echo(5)))
/// ```
///
/// Output should be:
///
/// ```text
/// 1
/// 2
/// 3
/// 4
/// 5
/// 9
/// 12
/// ```
///
/// The code above would create an AST similar to this:
///
/// ```text
///                 +-------+
///                 | print |
///                 +-------+
///                     |
///                   +---+
///          +--------| + |--------+
///          |        +---+        |
///      +------+               +------+
///      | echo |               | echo |
///      +------+               +------+
///          |                     |
///        +---+                 +---+
///        | + |                 | + |
///        +---+                 +---+
///          |                     |
///     +---------+           +----------+
///     |         |           |          |
/// +------+   +------+   +------+   +------+
/// | echo |   | echo |   | echo |   | echo |
/// +------+   +------+   +------+   +------+
///     |         |           |          |
///   +---+     +---+       +---+      +---+
///   | 1 |     | 2 |       | 4 |      | 5 |
///   +---+     +---+       +---+      +---+
/// ```
///
/// If we "flatten" the AST considering the "lifetime" of each value, we get
/// this structure:
///
/// ```text
///                   +---+
/// constant 1 ...... | 1 |
/// echo(1) ......... |   |---+
/// constant 2 ...... |   | 2 |
/// echo(2) ......... |   |   |
///                   +---+---+
/// add 1+2 ......... | 3 |
/// echo(3) ......... |   |---+
/// constant 4 ...... |   | 4 |
/// echo(4) ......... |   |   |---+
/// constant 5 ...... |   |   | 5 |
/// echo(5) ......... |   |   |   |
///                   |   |---+---+
/// add 4+5 ......... |   | 9 |
/// echo(9) ......... |   |   |
///                   +---+---+
/// add 3+9 ......... |12 |
/// print(12) ....... |   |
///                   +---+
/// ```
///
/// Looks like a stack doesn't it? That's the evaluation stack. All values in
/// the program flow through that stack, eliminating the need for instructions
/// with registers. Instead of `ADD r2, r0, r1` we just have `ADD`, which pops
/// two values from the stack, produces the result and pushes it back on top.
/// Simple, right? The drawback is that we need to execute more instructions to
/// achieve the same result as a register based VM. If we want to add two
/// variables, a register VM would run a single instruction:
///
/// ```text
/// ADD r2, r0, r1  // Add the contents of r0 and r1 and store the result in r2
///                 // r2 = r0 + r1
/// ```
///
/// Meanwhile a stack VM would run 4 instructions:
///
/// ```text
/// LOAD_VAR 0   // Push the contents of variable 0 on top of the stack
/// LOAD_VAR 1   // Push the contents of variable 1 on top of the stack
/// ADD          // Pop two values, add and push the result on top of the stack
/// STORE_VAR 2  // Store the top of the stack in variable 2
/// ```
///
/// Basically it's slower because it needs more cycles to do the same thing.
/// Other than that, pretty much everything is better in a stack VM, especially
/// simplicity (we don't even need to figure out which registers to use and when
/// to use them).
///
/// TODO: Explain frames and objects.
pub struct Vm {
    /// Call stack.
    ///
    /// On each function call we create a new [`Frame`] and push it on this
    /// stack. On each return, we destroy the frame and pop it from the stack
    /// to resume the execution of the previous frame.
    pub frames: Vec<Frame>,

    /// Evaluation stack.
    ///
    /// This stack only stores values.
    pub stack: Vec<Value>,

    /// Object pool.
    ///
    /// For now, since we don't have a garbage collector yet, this is basically
    /// an arena of objects. **Every object** is allocated here and will be
    /// destroyed when the lifetime of the [`Vm`] ends. Do not allocate objects
    /// elsewhere since that will make adding a garbage collector harder.
    /// Only allocate objects here and use indices to reference them, don't
    /// bother with Rust references because they will introduce lifetime issues.
    pub objects: Vec<Object>,

    /// Global variables.
    ///
    /// This stores the functions and globally declared variables.
    pub globals: Vec<Value>,
}

#[derive(Debug)]
pub enum RuntimeError {
    StackOverflow,
    InvalidArgumentCount { expected: usize, got: usize },
    InternalError,
}

#[derive(Debug)]
pub enum VmError {
    RuntimeError(RuntimeError),
}

impl From<RuntimeError> for VmError {
    fn from(error: RuntimeError) -> Self {
        VmError::RuntimeError(error)
    }
}

impl Vm {
    /// Main VM execution loop.
    ///
    /// Each "cycle" (loop iteration) executes a single instruction.
    pub fn exec(&mut self) -> Result<Value, VmError> {
        // Grab the last frame from the call stack.
        let Some(mut frame) = self.frames.last_mut() else {
            return Ok(Value::Null);
        };

        // Grab a reference to the function object. We do this before the loop
        // because there's no need to run this on every single iteration. Read
        // the implementations of `Instruction::Call` and `Instruction::Return`
        // below.
        let mut function = match &self.objects[frame.function] {
            Object::Function(function) => function,
            _ => return Err(VmError::RuntimeError(RuntimeError::InternalError)),
        };

        loop {
            // Current instruction pointer.
            let instruction_ptr = frame.instruction_ptr;

            // Move the frame's IP to the next instruction. We'll deal with
            // jump offsets later.
            frame.instruction_ptr += 1;

            eprintln!(
                "{}",
                function.bytecode.instructions[instruction_ptr as usize]
            );
            eprintln!(
                "[{}]",
                self.stack
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            match function.bytecode.instructions[instruction_ptr as usize] {
                Instruction::LoadConst(index) => {
                    let value = &function.bytecode.constants[index];
                    self.stack.push(*value);
                }

                Instruction::LoadVar(index) => {
                    let value = &self.stack[frame.locals_offset + index];
                    self.stack.push(*value);
                }

                Instruction::StoreVar(index) => {
                    let Some(value) = self.stack.last() else {
                        return Err(VmError::RuntimeError(RuntimeError::InternalError));
                    };

                    self.stack[frame.locals_offset + index] = *value;
                }

                Instruction::Pop => {
                    self.stack.pop();
                }

                Instruction::Jump(offset) => {
                    // Reassign the frame's IP to the new instruction.
                    // Remember that offset can be negative here, so even though
                    // we're adding it can still jump backwards.
                    frame.instruction_ptr = instruction_ptr + offset;
                }

                Instruction::JumpIfFalse(offset) => match self.stack.last() {
                    // Reassign only if the top of the stack is false.
                    Some(Value::Bool(value)) => {
                        if !value {
                            frame.instruction_ptr = instruction_ptr + offset;
                        }
                    }

                    // Empty stack, can't execute instruction.
                    None => return Err(VmError::RuntimeError(RuntimeError::InternalError)),

                    // Type error, we don't have "falsey" values in the language
                    // so we should always check booleans.
                    _ => return Err(VmError::RuntimeError(RuntimeError::InternalError)),
                },

                Instruction::LoadGlobal(index) => {
                    let value = &self.globals[index];
                    self.stack.push(*value);
                }

                Instruction::StoreGlobal(index) => {
                    let value = self.stack.last().unwrap();
                    self.globals[index] = *value;
                }

                Instruction::Call(arg_count) => {
                    // Function calls are pushed onto the stack like this:
                    //
                    // [callee, arg1, arg2, ..., argN]
                    //
                    // The callee is a ref to the function object, and after
                    // that we have all the function arguments. `arg_count` is
                    // the number of arguments pushed after the callee.
                    //
                    // That's how we compute the relative offset of the callee
                    // and it's local args in the stack.
                    let locals_offset = if self.stack.is_empty() {
                        0
                    } else {
                        self.stack.len() - arg_count - 1
                    };

                    // Get the function object from the stack.
                    let Value::Object(index) = &self.stack[locals_offset] else {
                        return Err(VmError::RuntimeError(RuntimeError::InternalError));
                    };

                    // Can't call a function if it's not a function ¯\_(ツ)_/¯
                    let Object::Function(callee) = &self.objects[*index] else {
                        return Err(VmError::RuntimeError(RuntimeError::InternalError));
                    };

                    // Compiler should have already checked this so we could
                    // skip it but it's an easy and fast check.
                    if arg_count != callee.arity {
                        return Err(VmError::RuntimeError(RuntimeError::InvalidArgumentCount {
                            expected: callee.arity,
                            got: arg_count,
                        }));
                    }

                    // Check if we've reached the max call stack size.
                    if self.frames.len() >= MAX_FRAMES {
                        return Err(VmError::RuntimeError(RuntimeError::StackOverflow));
                    }

                    // Otherwise push the new frame.
                    self.frames.push(Frame {
                        function: *index,
                        instruction_ptr: 0,
                        locals_offset,
                    });

                    // Point to next frame.
                    frame = self.frames.last_mut().expect("last_mut() was pushed above");

                    // Grab function ref. We do this to avoid running this
                    // code at the beginning of each iteration since it's
                    // totaly unnecessary. The function only changes when the
                    // frame changes.
                    match &self.objects[frame.function] {
                        Object::Function(f) => function = f,
                        _ => return Err(VmError::RuntimeError(RuntimeError::InternalError)),
                    }
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
                        return self
                            .stack
                            .pop()
                            .ok_or(VmError::RuntimeError(RuntimeError::InternalError));
                    };

                    // Resume previous frame execution.
                    frame = previous_frame;

                    // Point to the previous frame's function. Read the
                    // implementation of `Instruction::Call` above this one for
                    // more information about this piece.
                    match &self.objects[frame.function] {
                        Object::Function(f) => function = f,
                        _ => return Err(VmError::RuntimeError(RuntimeError::InternalError)),
                    }
                }
            }
        }
    }
}
