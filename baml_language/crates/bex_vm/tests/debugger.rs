//! VM debugger behavior tests with explicit sequence points.

use bex_vm::{BexVm, DebugBreakpoint, DebugStopReason, VmExecState};
use bex_vm_types::{
    Bytecode, ConstValue, Function, FunctionKind, GlobalIndex, Instruction, Object, ObjectIndex,
    Program as VmProgram, Value,
    bytecode::{InstructionMeta, LineTableEntry},
};

const FILE_ID: u32 = u32::MAX;
const MAIN_CALL_LINE: usize = 10;
const MAIN_AFTER_CALL_LINE: usize = 11;
const CALLEE_LINE: usize = 20;

fn line(pc: usize, line: usize, discriminator: u32) -> LineTableEntry {
    let span = baml_type::Span::fake();
    LineTableEntry {
        pc,
        span,
        line,
        sequence_point: true,
        discriminator,
    }
}

fn make_function(
    name: &str,
    instructions: Vec<Instruction>,
    constants: Vec<ConstValue>,
    line_table: Vec<LineTableEntry>,
) -> Function {
    let meta = vec![InstructionMeta { operand: None }; instructions.len()];
    Function {
        name: name.to_string(),
        arity: 0,
        real_local_count: 0,
        bytecode: Bytecode {
            instructions,
            constants,
            resolved_constants: Vec::new(),
            jump_tables: Vec::new(),
            line_table,
            meta,
        },
        kind: FunctionKind::Bytecode,
        local_names: Vec::new(),
        debug_locals: Vec::new(),
        span: baml_type::Span::fake(),
        block_notifications: Vec::new(),
        viz_nodes: Vec::new(),
        return_type: baml_type::Ty::Int,
        param_names: Vec::new(),
        param_types: Vec::new(),
        body_meta: None,
        trace: false,
    }
}

fn setup_vm() -> anyhow::Result<BexVm> {
    let callee = make_function(
        "callee",
        vec![Instruction::LoadConst(0), Instruction::Return],
        vec![ConstValue::Int(7)],
        vec![line(0, CALLEE_LINE, 0), line(1, CALLEE_LINE + 1, 0)],
    );

    let main = make_function(
        "main",
        vec![
            Instruction::Call(GlobalIndex::from_raw(0)),
            Instruction::Return,
        ],
        vec![],
        vec![line(0, MAIN_CALL_LINE, 0), line(1, MAIN_AFTER_CALL_LINE, 0)],
    );

    let mut program = VmProgram::new();
    let callee_idx = program.add_object(Object::Function(Box::new(callee)));
    let main_idx = program.add_object(Object::Function(Box::new(main)));
    program.add_global(ConstValue::Object(ObjectIndex::from_raw(callee_idx)));
    program.add_global(ConstValue::Object(ObjectIndex::from_raw(main_idx)));
    program
        .function_indices
        .insert("callee".to_string(), callee_idx);
    program
        .function_indices
        .insert("main".to_string(), main_idx);
    program
        .function_global_indices
        .insert("callee".to_string(), 0);
    program
        .function_global_indices
        .insert("main".to_string(), 1);

    let mut vm = BexVm::from_program(program)?;
    let main_ptr = vm.heap.compile_time_ptr(main_idx);
    vm.set_entry_point(main_ptr, &[]);
    Ok(vm)
}

fn exec_until_stop_or_complete(vm: &mut BexVm) -> anyhow::Result<VmExecState> {
    loop {
        let state = vm.exec()?;
        if matches!(state, VmExecState::SpanNotify(_)) {
            continue;
        }
        return Ok(state);
    }
}

#[test]
fn hits_line_breakpoint() -> anyhow::Result<()> {
    let mut vm = setup_vm()?;
    vm.debug_set_breakpoints([DebugBreakpoint {
        file_id: FILE_ID,
        line: MAIN_CALL_LINE,
    }]);

    let state = exec_until_stop_or_complete(&mut vm)?;
    let VmExecState::DebugStop(stop) = state else {
        anyhow::bail!("expected DebugStop");
    };
    assert_eq!(stop.reason, DebugStopReason::Breakpoint);
    assert_eq!(stop.line_entry.line, MAIN_CALL_LINE);
    Ok(())
}

#[test]
fn step_over_stays_in_current_frame() -> anyhow::Result<()> {
    let mut vm = setup_vm()?;
    vm.debug_set_breakpoints([DebugBreakpoint {
        file_id: FILE_ID,
        line: MAIN_CALL_LINE,
    }]);

    let first = exec_until_stop_or_complete(&mut vm)?;
    let VmExecState::DebugStop(first_stop) = first else {
        anyhow::bail!("expected initial DebugStop");
    };
    assert_eq!(first_stop.function_name, "main");
    assert_eq!(first_stop.frame_depth, 0);

    vm.debug_step_over();
    let second = exec_until_stop_or_complete(&mut vm)?;
    let VmExecState::DebugStop(second_stop) = second else {
        anyhow::bail!("expected step DebugStop");
    };
    assert_eq!(second_stop.reason, DebugStopReason::Step);
    assert_eq!(second_stop.function_name, "main");
    assert_eq!(second_stop.frame_depth, 0);
    assert_eq!(second_stop.line_entry.line, MAIN_AFTER_CALL_LINE);
    Ok(())
}

#[test]
fn step_out_unwinds_to_caller() -> anyhow::Result<()> {
    let mut vm = setup_vm()?;
    vm.debug_set_breakpoints([DebugBreakpoint {
        file_id: FILE_ID,
        line: CALLEE_LINE,
    }]);

    let first = exec_until_stop_or_complete(&mut vm)?;
    let VmExecState::DebugStop(first_stop) = first else {
        anyhow::bail!("expected initial DebugStop");
    };
    assert_eq!(first_stop.function_name, "callee");
    assert_eq!(first_stop.frame_depth, 1);

    vm.debug_step_out();
    let second = exec_until_stop_or_complete(&mut vm)?;
    let VmExecState::DebugStop(second_stop) = second else {
        anyhow::bail!("expected step-out DebugStop");
    };
    assert_eq!(second_stop.reason, DebugStopReason::Step);
    assert_eq!(second_stop.function_name, "main");
    assert_eq!(second_stop.frame_depth, 0);
    assert_eq!(second_stop.line_entry.line, MAIN_AFTER_CALL_LINE);
    Ok(())
}

#[test]
fn pause_stops_on_next_sequence_point() -> anyhow::Result<()> {
    let mut vm = setup_vm()?;
    vm.debug_pause();

    let state = exec_until_stop_or_complete(&mut vm)?;
    let VmExecState::DebugStop(stop) = state else {
        anyhow::bail!("expected DebugStop");
    };
    assert_eq!(stop.reason, DebugStopReason::Pause);
    assert_eq!(stop.line_entry.line, MAIN_CALL_LINE);
    Ok(())
}

#[test]
fn resume_does_not_repeat_same_stop() -> anyhow::Result<()> {
    let mut vm = setup_vm()?;
    vm.debug_set_breakpoints([DebugBreakpoint {
        file_id: FILE_ID,
        line: MAIN_CALL_LINE,
    }]);

    let first = exec_until_stop_or_complete(&mut vm)?;
    let VmExecState::DebugStop(first_stop) = first else {
        anyhow::bail!("expected initial stop");
    };
    assert_eq!(first_stop.line_entry.line, MAIN_CALL_LINE);

    vm.debug_continue();
    let second = exec_until_stop_or_complete(&mut vm)?;
    assert_eq!(second, VmExecState::Complete(Value::Int(7)));
    Ok(())
}
