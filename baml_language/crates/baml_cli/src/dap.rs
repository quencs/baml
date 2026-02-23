use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use baml_project::ProjectDatabase;
use bex_vm::{BexVm, DebugBreakpoint, DebugStop, DebugStopReason, VmExecState, types::ObjectTrait};
use bex_vm_types::{Object, Program, Value};
use clap::Args;
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};
use walkdir::WalkDir;

const THREAD_ID: i64 = 1;
const BUILTIN_PATH_PREFIX: &str = "<builtin>/";

#[derive(Args, Debug, Default)]
pub struct DebugAdapterArgs {}

impl DebugAdapterArgs {
    pub fn run(&self) -> Result<()> {
        run()
    }
}

pub fn run() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let writer = BufWriter::new(stdout.lock());
    let mut server = DapServer::new(writer);

    while let Some(message) = read_dap_message(&mut reader)? {
        let value: JsonValue =
            serde_json::from_str(&message).context("failed to parse incoming DAP message")?;

        if value.get("type").and_then(JsonValue::as_str) != Some("request") {
            continue;
        }

        let keep_running = server.handle_request(&value)?;
        if !keep_running {
            break;
        }
    }

    Ok(())
}

fn read_dap_message(reader: &mut impl BufRead) -> Result<Option<String>> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Ok(None);
        }

        let header = line.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            break;
        }

        if let Some(length) = header.strip_prefix("Content-Length:") {
            let parsed = length.trim().parse::<usize>()?;
            content_length = Some(parsed);
        }
    }

    let content_length =
        content_length.ok_or_else(|| anyhow!("missing Content-Length header in DAP message"))?;

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    let payload = String::from_utf8(body).context("DAP payload is not valid UTF-8")?;
    Ok(Some(payload))
}

struct DapServer<W: Write> {
    writer: W,
    next_seq: i64,
    pending_breakpoints: HashMap<PathBuf, Vec<usize>>,
    session: Option<DebugSession>,
}

fn build_sequence_points_by_file(program: &Program) -> HashMap<u32, Vec<usize>> {
    let mut by_file: HashMap<u32, Vec<usize>> = HashMap::new();

    for object in &program.objects {
        let Object::Function(function) = object else {
            continue;
        };

        for entry in &function.bytecode.line_table {
            if !entry.sequence_point {
                continue;
            }
            by_file
                .entry(entry.span.file_id.as_u32())
                .or_default()
                .push(entry.line);
        }
    }

    for lines in by_file.values_mut() {
        lines.sort_unstable();
        lines.dedup();
    }

    by_file
}

impl<W: Write> DapServer<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            next_seq: 1,
            pending_breakpoints: HashMap::new(),
            session: None,
        }
    }

    fn handle_request(&mut self, request: &JsonValue) -> Result<bool> {
        let request_seq = request.get("seq").and_then(JsonValue::as_i64).unwrap_or(0);
        let command = request
            .get("command")
            .and_then(JsonValue::as_str)
            .unwrap_or("");
        let args = request.get("arguments").cloned().unwrap_or(JsonValue::Null);

        let result = match command {
            "initialize" => self.handle_initialize(request_seq, command),
            "launch" => self.handle_launch(request_seq, command, args),
            "setBreakpoints" => self.handle_set_breakpoints(request_seq, command, args),
            "configurationDone" => self.handle_configuration_done(request_seq, command),
            "threads" => self.handle_threads(request_seq, command),
            "stackTrace" => self.handle_stack_trace(request_seq, command, args),
            "scopes" => self.handle_scopes(request_seq, command, args),
            "variables" => self.handle_variables(request_seq, command, args),
            "continue" => self.handle_resume_command(request_seq, command, ResumeAction::Continue),
            "next" => self.handle_resume_command(request_seq, command, ResumeAction::StepOver),
            "stepIn" => self.handle_resume_command(request_seq, command, ResumeAction::StepIn),
            "stepOut" => self.handle_resume_command(request_seq, command, ResumeAction::StepOut),
            "pause" => self.handle_resume_command(request_seq, command, ResumeAction::Pause),
            "disconnect" => {
                self.respond_success(request_seq, command, Some(json!({})))?;
                self.send_terminated_event()?;
                self.session = None;
                return Ok(false);
            }
            _ => self.respond_error(
                request_seq,
                command,
                format!("unsupported DAP request: {command}"),
            ),
        };

        if let Err(error) = result {
            self.respond_error(request_seq, command, error.to_string())?;
        }

        Ok(true)
    }

    fn handle_initialize(&mut self, request_seq: i64, command: &str) -> Result<()> {
        let body = json!({
            "supportsConfigurationDoneRequest": true,
            "supportsRestartRequest": false,
            "supportsStepBack": false,
            "supportsTerminateRequest": true,
            "supportsEvaluateForHovers": false,
        });
        self.respond_success(request_seq, command, Some(body))?;
        self.send_event("initialized", json!({}))
    }

    fn handle_launch(&mut self, request_seq: i64, command: &str, args: JsonValue) -> Result<()> {
        let launch = serde_json::from_value::<LaunchArgs>(args)
            .context("invalid launch arguments for baml debugger")?;
        let project_path = launch
            .project_path
            .ok_or_else(|| anyhow!("launch.projectPath is required"))?;
        let function_name = launch
            .function_name
            .ok_or_else(|| anyhow!("launch.functionName is required"))?;
        let root = canonicalize_path(&project_path);

        if !root.is_dir() {
            bail!("projectPath is not a directory: {}", root.display());
        }

        let mut db = ProjectDatabase::new();
        db.set_project_root(&root);
        load_project_files(&mut db, &root)?;

        let bytecode = db
            .get_debug_bytecode()
            .map_err(|error| anyhow!("{error}"))?;
        let function_index = bytecode
            .function_index(&function_name)
            .ok_or_else(|| anyhow!("function not found: {function_name}"))?;

        let sequence_points_by_file = build_sequence_points_by_file(&bytecode);
        let mut vm = BexVm::from_program(bytecode)?;
        let function_ptr = vm.heap.compile_time_ptr(function_index);
        let args = parse_launch_args(&mut vm, function_ptr, launch.args)?;
        vm.set_entry_point(function_ptr, &args);

        let mut session = DebugSession::new(
            db,
            vm,
            launch.stop_on_entry.unwrap_or(false),
            sequence_points_by_file,
        );

        let pending = self.pending_breakpoints.clone();
        for (path, lines) in &pending {
            if let Some(file_id) = session.path_to_file_id.get(path).copied() {
                session.set_breakpoints(file_id, lines);
            }
        }

        self.session = Some(session);
        self.respond_success(request_seq, command, Some(json!({})))
    }

    fn handle_set_breakpoints(
        &mut self,
        request_seq: i64,
        command: &str,
        args: JsonValue,
    ) -> Result<()> {
        let args = serde_json::from_value::<SetBreakpointsArgs>(args)
            .context("invalid setBreakpoints arguments")?;
        let source = args
            .source
            .and_then(|source| source.path)
            .ok_or_else(|| anyhow!("setBreakpoints.source.path is required"))?;
        let requested_lines = if !args.breakpoints.is_empty() {
            args.breakpoints
                .iter()
                .filter_map(|bp| usize::try_from(bp.line).ok())
                .collect::<Vec<_>>()
        } else {
            args.lines
                .iter()
                .filter_map(|line| usize::try_from(*line).ok())
                .collect::<Vec<_>>()
        };

        let canonical_path = canonicalize_path(&source);
        self.pending_breakpoints
            .insert(canonical_path.clone(), requested_lines.clone());

        let mut resolved_lines = vec![None; requested_lines.len()];

        if !source.starts_with(BUILTIN_PATH_PREFIX)
            && let Some(session) = self.session.as_mut()
            && let Some(file_id) = session.path_to_file_id.get(&canonical_path).copied()
        {
            resolved_lines = session.set_breakpoints(file_id, &requested_lines);
        }

        let breakpoints = requested_lines
            .into_iter()
            .zip(resolved_lines)
            .map(|(requested_line, resolved_line)| {
                let resolved = resolved_line.unwrap_or(requested_line);
                json!({
                    "verified": resolved_line.is_some(),
                    "line": resolved,
                })
            })
            .collect::<Vec<_>>();

        self.respond_success(
            request_seq,
            command,
            Some(json!({ "breakpoints": breakpoints })),
        )
    }

    fn handle_configuration_done(&mut self, request_seq: i64, command: &str) -> Result<()> {
        let stop_on_entry = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("launch must be called before configurationDone"))?
            .stop_on_entry;
        self.respond_success(request_seq, command, Some(json!({})))?;
        let action = if stop_on_entry {
            ResumeAction::StepIn
        } else {
            ResumeAction::Continue
        };
        self.run_and_emit_or_terminate(action)
    }

    fn handle_threads(&mut self, request_seq: i64, command: &str) -> Result<()> {
        self.respond_success(
            request_seq,
            command,
            Some(json!({
                "threads": [{
                    "id": THREAD_ID,
                    "name": "main",
                }]
            })),
        )
    }

    fn handle_stack_trace(
        &mut self,
        request_seq: i64,
        command: &str,
        args: JsonValue,
    ) -> Result<()> {
        let args = serde_json::from_value::<StackTraceArgs>(args).unwrap_or_default();
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("no active debug session"))?;

        let frames = session.vm.debug_stack_frames();
        let total_frames = frames.len();
        let start = args.start_frame.unwrap_or(0);
        let levels = args.levels.unwrap_or(total_frames.saturating_sub(start));

        let stack_frames = frames
            .into_iter()
            .skip(start)
            .take(levels)
            .map(|frame| {
                let (line, source) = match frame.line_entry {
                    Some(entry) => {
                        let source = session
                            .file_id_to_path
                            .get(&entry.span.file_id.as_u32())
                            .map(|path| {
                                json!({
                                    "name": path.file_name().and_then(|name| name.to_str()).unwrap_or(""),
                                    "path": path.to_string_lossy().to_string(),
                                })
                            });
                        (entry.line, source)
                    }
                    None => (1, None),
                };

                json!({
                    "id": i64::try_from(frame.frame_depth).unwrap_or_default(),
                    "name": frame.function_name,
                    "line": line,
                    "column": 1,
                    "source": source.unwrap_or(json!(null)),
                })
            })
            .collect::<Vec<_>>();

        self.respond_success(
            request_seq,
            command,
            Some(json!({
                "stackFrames": stack_frames,
                "totalFrames": total_frames,
            })),
        )
    }

    fn handle_scopes(&mut self, request_seq: i64, command: &str, args: JsonValue) -> Result<()> {
        let args =
            serde_json::from_value::<ScopesArgs>(args).context("invalid scopes arguments")?;
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("no active debug session"))?;
        let frame_depth = usize::try_from(args.frame_id).context("invalid frameId")?;

        let variables_reference = session.new_handle(HandleValue::Locals { frame_depth });
        self.respond_success(
            request_seq,
            command,
            Some(json!({
                "scopes": [{
                    "name": "Locals",
                    "presentationHint": "locals",
                    "variablesReference": variables_reference,
                    "expensive": false,
                }]
            })),
        )
    }

    fn handle_variables(&mut self, request_seq: i64, command: &str, args: JsonValue) -> Result<()> {
        let args =
            serde_json::from_value::<VariablesArgs>(args).context("invalid variables arguments")?;
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("no active debug session"))?;
        let variables = session.variables_for_reference(args.variables_reference)?;

        self.respond_success(
            request_seq,
            command,
            Some(json!({
                "variables": variables,
            })),
        )
    }

    fn handle_resume_command(
        &mut self,
        request_seq: i64,
        command: &str,
        action: ResumeAction,
    ) -> Result<()> {
        if self.session.is_none() {
            bail!("no active debug session");
        }

        let body = if command == "continue" {
            Some(json!({ "allThreadsContinued": true }))
        } else {
            Some(json!({}))
        };
        self.respond_success(request_seq, command, body)?;
        self.send_event(
            "continued",
            json!({
                "threadId": THREAD_ID,
                "allThreadsContinued": true,
            }),
        )?;
        self.run_and_emit_or_terminate(action)
    }

    fn run_and_emit_or_terminate(&mut self, action: ResumeAction) -> Result<()> {
        if let Err(error) = self.run_and_emit(action) {
            self.send_event(
                "output",
                json!({
                    "category": "stderr",
                    "output": format!("{}\n", error),
                }),
            )?;
            self.send_terminated_event()?;
            self.session = None;
        }
        Ok(())
    }

    fn run_and_emit(&mut self, action: ResumeAction) -> Result<()> {
        let outcome = {
            let session = self
                .session
                .as_mut()
                .ok_or_else(|| anyhow!("no active debug session"))?;
            session.run(action)?
        };

        match outcome {
            RunOutcome::Stopped(stop) => self.send_stopped_event(stop.reason),
            RunOutcome::Complete => self.send_terminated_event(),
        }
    }

    fn send_stopped_event(&mut self, reason: DebugStopReason) -> Result<()> {
        let reason = match reason {
            DebugStopReason::Breakpoint => "breakpoint",
            DebugStopReason::Step => "step",
            DebugStopReason::Pause => "pause",
        };

        self.send_event(
            "stopped",
            json!({
                "reason": reason,
                "threadId": THREAD_ID,
                "allThreadsStopped": true,
            }),
        )
    }

    fn send_terminated_event(&mut self) -> Result<()> {
        self.send_event("terminated", json!({}))
    }

    fn respond_success(
        &mut self,
        request_seq: i64,
        command: &str,
        body: Option<JsonValue>,
    ) -> Result<()> {
        let mut response = json!({
            "seq": self.next_outgoing_seq(),
            "type": "response",
            "request_seq": request_seq,
            "success": true,
            "command": command,
        });
        if let Some(body) = body {
            response["body"] = body;
        }
        self.write_message(&response)
    }

    fn respond_error(&mut self, request_seq: i64, command: &str, message: String) -> Result<()> {
        let response = json!({
            "seq": self.next_outgoing_seq(),
            "type": "response",
            "request_seq": request_seq,
            "success": false,
            "command": command,
            "message": message,
        });
        self.write_message(&response)
    }

    fn send_event(&mut self, event: &str, body: JsonValue) -> Result<()> {
        let message = json!({
            "seq": self.next_outgoing_seq(),
            "type": "event",
            "event": event,
            "body": body,
        });
        self.write_message(&message)
    }

    fn write_message(&mut self, payload: &JsonValue) -> Result<()> {
        let serialized = serde_json::to_vec(payload)?;
        let header = format!("Content-Length: {}\r\n\r\n", serialized.len());
        self.writer.write_all(header.as_bytes())?;
        self.writer.write_all(&serialized)?;
        self.writer.flush()?;
        Ok(())
    }

    fn next_outgoing_seq(&mut self) -> i64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }
}

#[derive(Clone, Copy)]
enum HandleValue {
    Locals { frame_depth: usize },
    Value(Value),
}

#[derive(Clone, Copy)]
enum ResumeAction {
    Continue,
    StepIn,
    StepOver,
    StepOut,
    Pause,
}

enum RunOutcome {
    Stopped(DebugStop),
    Complete,
}

#[derive(Clone, Copy)]
struct SourceStopSite {
    frame_depth: usize,
    file_id: u32,
    line: usize,
    pc: usize,
}

struct DebugSession {
    db: ProjectDatabase,
    vm: BexVm,
    stop_on_entry: bool,
    file_id_to_path: HashMap<u32, PathBuf>,
    path_to_file_id: HashMap<PathBuf, u32>,
    sequence_points_by_file: HashMap<u32, Vec<usize>>,
    breakpoints_by_file: HashMap<u32, HashSet<usize>>,
    handles: HashMap<i64, HandleValue>,
    next_handle: i64,
}

impl DebugSession {
    fn new(
        db: ProjectDatabase,
        vm: BexVm,
        stop_on_entry: bool,
        sequence_points_by_file: HashMap<u32, Vec<usize>>,
    ) -> Self {
        let mut session = Self {
            db,
            vm,
            stop_on_entry,
            file_id_to_path: HashMap::new(),
            path_to_file_id: HashMap::new(),
            sequence_points_by_file,
            breakpoints_by_file: HashMap::new(),
            handles: HashMap::new(),
            next_handle: 1,
        };
        session.rebuild_file_maps();
        session
    }

    fn rebuild_file_maps(&mut self) {
        self.file_id_to_path.clear();
        self.path_to_file_id.clear();

        for path in self.db.non_builtin_file_paths() {
            if let Some(file_id) = self.db.path_to_file_id(&path) {
                let id = file_id.as_u32();
                self.path_to_file_id.insert(path.clone(), id);
                self.file_id_to_path.insert(id, path);
            }
        }
    }

    fn set_breakpoints(&mut self, file_id: u32, requested_lines: &[usize]) -> Vec<Option<usize>> {
        let mut verified_lines = HashSet::new();
        let mut resolved_lines = Vec::with_capacity(requested_lines.len());

        for &line in requested_lines {
            let resolved = self.resolve_breakpoint_line(file_id, line);
            if let Some(resolved_line) = resolved {
                verified_lines.insert(resolved_line);
            }
            resolved_lines.push(resolved);
        }

        if verified_lines.is_empty() {
            self.breakpoints_by_file.remove(&file_id);
        } else {
            self.breakpoints_by_file
                .insert(file_id, verified_lines.clone());
        }
        self.sync_breakpoints_to_vm();

        resolved_lines
    }

    fn resolve_breakpoint_line(&self, file_id: u32, requested_line: usize) -> Option<usize> {
        let lines = self.sequence_points_by_file.get(&file_id)?;
        if lines.is_empty() {
            return None;
        }

        // Snap to the first executable line at/after the requested line.
        // If the request is past the last executable line, snap to the last one.
        let idx = lines.partition_point(|line| *line < requested_line);
        if idx < lines.len() {
            Some(lines[idx])
        } else {
            lines.last().copied()
        }
    }

    fn sync_breakpoints_to_vm(&mut self) {
        let breakpoints = self
            .breakpoints_by_file
            .iter()
            .flat_map(|(file_id, lines)| {
                lines.iter().map(move |line| DebugBreakpoint {
                    file_id: *file_id,
                    line: *line,
                })
            })
            .collect::<Vec<_>>();
        self.vm.debug_set_breakpoints(breakpoints);
    }

    fn apply_resume_action(&mut self, action: ResumeAction) {
        match action {
            ResumeAction::Continue => self.vm.debug_continue(),
            ResumeAction::StepIn => self.vm.debug_step_in(),
            ResumeAction::StepOver => self.vm.debug_step_over(),
            ResumeAction::StepOut => self.vm.debug_step_out(),
            ResumeAction::Pause => self.vm.debug_pause(),
        }
    }

    fn current_source_stop_site(&self) -> Option<SourceStopSite> {
        let frame = self.vm.debug_stack_frames().into_iter().next()?;
        let line_entry = frame.line_entry?;
        Some(SourceStopSite {
            frame_depth: frame.frame_depth,
            file_id: line_entry.span.file_id.as_u32(),
            line: line_entry.line,
            pc: frame.pc,
        })
    }

    fn run(&mut self, action: ResumeAction) -> Result<RunOutcome> {
        self.clear_handles();

        let step_origin = match action {
            ResumeAction::StepIn | ResumeAction::StepOver => self.current_source_stop_site(),
            ResumeAction::Continue | ResumeAction::StepOut | ResumeAction::Pause => None,
        };
        let mut forward_same_line_cursor = step_origin.map(|origin| origin.pc);
        let mut skipped_same_site_breakpoints = 0usize;

        self.apply_resume_action(action);

        loop {
            match self.vm.exec()? {
                VmExecState::Complete(_) => return Ok(RunOutcome::Complete),
                VmExecState::DebugStop(stop) => {
                    if let Some(origin) = step_origin {
                        let same_source_site = stop.frame_depth == origin.frame_depth
                            && stop.line_entry.span.file_id.as_u32() == origin.file_id
                            && stop.line_entry.line == origin.line;
                        // While stepping, don't get stuck on a breakpoint bound to the
                        // current source site. Continue until execution reaches a
                        // different site (or completion).
                        if same_source_site
                            && stop.reason == DebugStopReason::Breakpoint
                            && skipped_same_site_breakpoints < 32
                        {
                            skipped_same_site_breakpoints += 1;
                            self.apply_resume_action(action);
                            continue;
                        }
                        // Skip only forward same-line stops (multiple sequence points on one
                        // source line). Keep same-line stops when control flow loops back.
                        if same_source_site
                            && stop.pc > forward_same_line_cursor.unwrap_or(origin.pc)
                        {
                            forward_same_line_cursor = Some(stop.pc);
                            self.apply_resume_action(action);
                            continue;
                        }
                    }
                    return Ok(RunOutcome::Stopped(stop));
                }
                VmExecState::Notify(_) | VmExecState::SpanNotify(_) => continue,
                VmExecState::ScheduleFuture(_) | VmExecState::Await(_) => {
                    bail!("debugger does not support async/sys-op execution yet")
                }
            }
        }
    }

    fn new_handle(&mut self, value: HandleValue) -> i64 {
        let id = self.next_handle;
        self.next_handle += 1;
        self.handles.insert(id, value);
        id
    }

    fn clear_handles(&mut self) {
        self.handles.clear();
        self.next_handle = 1;
    }

    fn variables_for_reference(&mut self, reference: i64) -> Result<Vec<JsonValue>> {
        let handle = self
            .handles
            .get(&reference)
            .copied()
            .ok_or_else(|| anyhow!("invalid variablesReference: {reference}"))?;

        let entries = match handle {
            HandleValue::Locals { frame_depth } => self
                .vm
                .debug_frame_locals(frame_depth)
                .into_iter()
                .map(|local| (local.name, local.value))
                .collect::<Vec<_>>(),
            HandleValue::Value(value) => self.child_entries_for_value(value),
        };

        Ok(entries
            .into_iter()
            .map(|(name, value)| self.variable_json(name, value))
            .collect())
    }

    fn child_entries_for_value(&self, value: Value) -> Vec<(String, Value)> {
        let Value::Object(ptr) = value else {
            return Vec::new();
        };

        match self.vm.get_object(ptr) {
            Object::Array(items) => items
                .iter()
                .enumerate()
                .map(|(index, value)| (format!("[{index}]"), *value))
                .collect(),
            Object::Map(map) => map
                .iter()
                .map(|(key, value)| (key.to_string(), *value))
                .collect(),
            Object::Instance(instance) => {
                let field_names = match self.vm.get_object(instance.class) {
                    Object::Class(class) => class
                        .fields
                        .iter()
                        .map(|field| field.name.clone())
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                };

                instance
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, field_value)| {
                        let name = field_names
                            .get(index)
                            .cloned()
                            .unwrap_or_else(|| format!("field_{index}"));
                        (name, *field_value)
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn variable_json(&mut self, name: String, value: Value) -> JsonValue {
        let (display, type_name, expandable) = self.describe_value(value);
        let variables_reference = if expandable {
            self.new_handle(HandleValue::Value(value))
        } else {
            0
        };

        json!({
            "name": name,
            "value": display,
            "type": type_name,
            "variablesReference": variables_reference,
        })
    }

    fn describe_value(&self, value: Value) -> (String, String, bool) {
        match value {
            Value::Null => ("null".to_string(), "null".to_string(), false),
            Value::Int(value) => (value.to_string(), "int".to_string(), false),
            Value::Float(value) => (value.to_string(), "float".to_string(), false),
            Value::Bool(value) => (value.to_string(), "bool".to_string(), false),
            Value::Object(ptr) =>
            {
                #[allow(unreachable_patterns)]
                match self.vm.get_object(ptr) {
                    Object::String(value) => (format!("{value:?}"), "string".to_string(), false),
                    Object::Array(values) => (
                        format!("Array({})", values.len()),
                        "array".to_string(),
                        true,
                    ),
                    Object::Map(values) => {
                        (format!("Map({})", values.len()), "map".to_string(), true)
                    }
                    Object::Instance(instance) => {
                        let class_name = match self.vm.get_object(instance.class) {
                            Object::Class(class) => class.name.clone(),
                            _ => "instance".to_string(),
                        };
                        (format!("{class_name} {{...}}"), class_name, true)
                    }
                    Object::Variant(variant) => {
                        let variant_name = match self.vm.get_object(variant.enm) {
                            Object::Enum(enm) => enm
                                .variants
                                .get(variant.index)
                                .map(|entry| format!("{}::{}", enm.name, entry.name))
                                .unwrap_or_else(|| format!("{}::<{}>", enm.name, variant.index)),
                            _ => format!("<variant {}>", variant.index),
                        };
                        (variant_name, "variant".to_string(), false)
                    }
                    Object::Function(function) => (
                        format!("<fn {}>", function.name),
                        "function".to_string(),
                        false,
                    ),
                    Object::Class(class) => (
                        format!("<class {}>", class.name),
                        "class".to_string(),
                        false,
                    ),
                    Object::Enum(enm) => {
                        (format!("<enum {}>", enm.name), "enum".to_string(), false)
                    }
                    Object::Future(_) => ("<future>".to_string(), "future".to_string(), false),
                    Object::Resource(resource) => (
                        format!("<resource {resource:?}>"),
                        "resource".to_string(),
                        false,
                    ),
                    Object::Media(media) => (
                        format!("<media {:?}>", media.kind),
                        "media".to_string(),
                        false,
                    ),
                    Object::PromptAst(_) => {
                        ("<prompt_ast>".to_string(), "prompt_ast".to_string(), false)
                    }
                    Object::Collector(_) => {
                        ("<collector>".to_string(), "collector".to_string(), false)
                    }
                    Object::Type(ty) => (ty.to_string(), "type".to_string(), false),
                    _ => ("<object>".to_string(), "object".to_string(), false),
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchArgs {
    project_path: Option<String>,
    function_name: Option<String>,
    args: Option<JsonValue>,
    stop_on_entry: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetBreakpointsArgs {
    source: Option<DapSource>,
    #[serde(default)]
    breakpoints: Vec<SourceBreakpoint>,
    #[serde(default)]
    lines: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct SourceBreakpoint {
    line: i64,
}

#[derive(Debug, Deserialize)]
struct DapSource {
    path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StackTraceArgs {
    start_frame: Option<usize>,
    levels: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScopesArgs {
    frame_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VariablesArgs {
    variables_reference: i64,
}

fn canonicalize_path(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref()
        .canonicalize()
        .unwrap_or_else(|_| path.as_ref().to_path_buf())
}

fn load_project_files(db: &mut ProjectDatabase, root: &Path) -> Result<()> {
    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.into_path();
        let is_baml = path.extension().and_then(|ext| ext.to_str()) == Some("baml");
        let is_baml_jinja = path
            .to_string_lossy()
            .to_ascii_lowercase()
            .ends_with(".baml.jinja");
        if !is_baml && !is_baml_jinja {
            continue;
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read source file {}", path.display()))?;
        db.add_or_update_file(&path, &content);
    }

    Ok(())
}

fn parse_launch_args(
    vm: &mut BexVm,
    function_ptr: bex_vm_types::HeapPtr,
    raw_args: Option<JsonValue>,
) -> Result<Vec<Value>> {
    let function = vm
        .get_object(function_ptr)
        .as_function()
        .context("launch target is not a function")?
        .clone();

    let raw_args = raw_args.unwrap_or(JsonValue::Null);
    let arity = function.arity;

    if raw_args.is_null() {
        if arity == 0 {
            return Ok(Vec::new());
        }
        bail!(
            "function '{}' requires {} argument(s)",
            function.name,
            arity
        );
    }

    if let Some(values) = raw_args.as_array() {
        if values.len() != arity {
            bail!(
                "function '{}' expects {} argument(s), got {}",
                function.name,
                arity,
                values.len()
            );
        }

        return values
            .iter()
            .map(|value| json_to_vm_value(vm, value))
            .collect();
    }

    if let Some(map) = raw_args.as_object() {
        let mut ordered = Vec::with_capacity(arity);
        for param in &function.param_names {
            let value = map
                .get(param)
                .ok_or_else(|| anyhow!("missing argument '{param}'"))?;
            ordered.push(json_to_vm_value(vm, value)?);
        }

        let extras = map
            .keys()
            .filter(|key| !function.param_names.iter().any(|param| param == *key))
            .cloned()
            .collect::<Vec<_>>();
        if !extras.is_empty() {
            bail!("unexpected argument(s): {}", extras.join(", "));
        }

        return Ok(ordered);
    }

    if arity == 1 {
        return Ok(vec![json_to_vm_value(vm, &raw_args)?]);
    }

    bail!(
        "function '{}' expects {} argument(s), launch.args must be an object or array",
        function.name,
        arity
    )
}

fn json_to_vm_value(vm: &mut BexVm, value: &JsonValue) -> Result<Value> {
    match value {
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Bool(value) => Ok(Value::Bool(*value)),
        JsonValue::Number(number) => {
            if let Some(value) = number.as_i64() {
                return Ok(Value::Int(value));
            }
            if let Some(value) = number.as_u64() {
                let value = i64::try_from(value).context("integer is too large for i64")?;
                return Ok(Value::Int(value));
            }
            if let Some(value) = number.as_f64() {
                return Ok(Value::Float(value));
            }
            bail!("unsupported JSON number representation")
        }
        JsonValue::String(value) => Ok(vm.alloc_string(value.clone())),
        JsonValue::Array(values) => {
            let values = values
                .iter()
                .map(|value| json_to_vm_value(vm, value))
                .collect::<Result<Vec<_>>>()?;
            Ok(vm.alloc_array(values))
        }
        JsonValue::Object(values) => {
            let mut map = indexmap::IndexMap::new();
            for (key, value) in values {
                map.insert(key.clone(), json_to_vm_value(vm, value)?);
            }
            Ok(vm.alloc_map(map))
        }
    }
}
