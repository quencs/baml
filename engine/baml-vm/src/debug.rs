//! VM debugging utilities & helpers.
//!
//! NOTE: Functions here should not take an entire reference to the
//! [`crate::Vm`] because then it will be hard to circumvent the borrow checker
//! in the [`crate::Vm::exec`] loop (which is where we want to use this).
//!
//! Instead, they take read only references to the parts of the [`crate::Vm`]
//! that they need, that way inside the loop we can "destructure" the
//! [`crate::Vm`] and let the compiler know exactly which properties we're
//! using as mutable and which properties we're using as immutable.
//!
//! Reference structs can be created if needed:
//!
//! ```ignore
//! struct InstructionContext<'vm> {
//!     instruction_ptr: isize,
//!     function: &'vm Function,
//!     objects: &'vm [Object],
//!     globals: &'vm [Value],
//! }
//!
//! ```

use crate::{Function, Instruction, Object, Value};

/// Context aware instruction display.
///
/// Instructions themselves are kinda "bare". For example, `LOAD_VAR 1`
/// means load the local variable at index 1, but what's the name of that
/// variable in the user's code? Same with `LOAD_CONST 1`, what's the value
/// of the constant at index 1?
///
/// This function returns a tuple `(instruction, metadata)` where `metadata`
/// is important debug information associated with the `instruction`. In
/// the case of `LOAD_VAR` it's the name of the variable, in the case of
/// `LOAD_CONST` it's the value of the constant, and so on.
///
/// If there's no relevant metadata to attach to the instruction, then this
/// function returns an empty string.
pub fn display_instruction(
    instruction_ptr: isize,
    function: &Function,
    objects: &[Object],
    globals: &[Value],
) -> (String, String) {
    let instruction = &function.bytecode.instructions[instruction_ptr as usize];

    let metadata = match instruction {
        // TODO: For this one we need to add some logic to check if it's
        // a function or a global variable. In the case of variables, we
        // have to store the names (potentially in the [`Vm`] struct) and
        // print it.
        Instruction::LoadGlobal(index) => {
            format!("({})", display_value(&globals[*index], objects))
        }

        // TODO: Same as above.
        Instruction::StoreGlobal(_) => todo!(),

        Instruction::LoadConst(index) => format!(
            "({})",
            display_value(&function.bytecode.constants[*index], objects)
        ),

        Instruction::LoadVar(index) | Instruction::StoreVar(index) => {
            format!("({})", function.local_var_names[*index])
        }

        Instruction::Jump(offset) | Instruction::JumpIfFalse(offset) => {
            format!("(to {})", instruction_ptr + offset)
        }

        Instruction::Pop | Instruction::Call(_) | Instruction::Return => String::new(),
    };

    (instruction.to_string(), metadata)
}

/// Context aware value display.
///
/// The default display for objects is just a reference number. If we want
/// all the information, we have to dereference the object and call it's
/// `to_string` implementation.
pub fn display_value(value: &Value, objects: &[Object]) -> String {
    match value {
        Value::Object(index) => objects[*index].to_string(),
        other => other.to_string(),
    }
}

/// The number of whitespaces that separate each column.
///
/// See [`display_bytecode`] for more information.
const COLUMN_MARGIN: usize = 3;

/// Print the bytecode of a function in a readable table format.
///
/// Format is [IP, INSTRUCTION, METADATA]. Something like this:
///
/// ```text
/// 0   LOAD_VAR 1        (b)
/// 1   JUMP_IF_FALSE 4   (to 5)
/// 2   POP
/// 3   LOAD_CONST 0      (1)
/// 4   JUMP 3            (to 7)
/// 5   POP
/// 6   LOAD_CONST 1      (2)
/// 7   RETURN
/// ```
///
/// Basically tries to mimic CPython's bytecode disassembly function.
///
/// Takes care of calculating how many whitespaces we need to make the table
/// symmetric and returns the entire table.
pub fn display_bytecode(function: &Function, objects: &[Object], globals: &[Value]) -> String {
    if function.bytecode.instructions.is_empty() {
        return String::new();
    }

    let mut rows = Vec::new();
    let mut widths = [0; 3];

    // Populate all the rows.
    for instruction_ptr in 0..function.bytecode.instructions.len() {
        let (instruction, metadata) =
            display_instruction(instruction_ptr as isize, function, objects, globals);

        // Table format is [IP, INSTR, META].
        let row = [instruction_ptr.to_string(), instruction, metadata];

        // Now calculate the max width of each column.
        for (i, col) in row.iter().enumerate() {
            let width = col.chars().count();

            if width > widths[i] {
                widths[i] = width;
            }
        }

        rows.push(row);
    }

    let mut table = String::new();

    // Print the table.
    for row in &rows {
        for (i, col) in row.iter().enumerate() {
            let mut width = widths[i];
            if i < row.len() - 1 {
                width += COLUMN_MARGIN;
            }

            table.push_str(&format!("{col:<width$}"));
        }

        table.push('\n');
    }

    table
}
