use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use baml_compiler::watch::{shared_handler, SharedWatchHandler, WatchBamlValue, WatchNotification};
use baml_runtime::{FunctionResult, RuntimeContextManager, TripWire};
use internal_baml_core::feature_flags::FeatureFlags;
use serde::Serialize;
use serde_json::Value;

type DynResult<T> = anyhow::Result<T>;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
enum LexicalState {
    NotRunning,
    Running,
    Completed,
}

#[derive(Debug, Clone, Serialize)]
struct StateUpdate {
    lexical_id: String,
    new_state: LexicalState,
}

#[derive(Debug, Clone, Serialize)]
struct EventRecord {
    kind: String,
    function: String,
    variable: Option<String>,
    channel: Option<String>,
    stream_id: Option<String>,
    header: Option<HeaderEvent>,
    value: Option<Value>,
    is_stream: bool,
}

#[derive(Debug, Clone, Serialize)]
struct HeaderEvent {
    level: u8,
    title: String,
}

#[derive(Default)]
struct VizStateReducer {
    states: HashMap<String, LexicalState>,
}

impl VizStateReducer {
    // Stub reducer: once lexical IDs are provided by the runtime, this should map
    // events to concrete node state transitions instead of this placeholder toggle.
    fn apply(&mut self, event: &EventRecord) -> StateUpdate {
        let lexical_id = build_lexical_id(event);
        let current = self
            .states
            .get(&lexical_id)
            .copied()
            .unwrap_or(LexicalState::NotRunning);
        let next = match current {
            LexicalState::NotRunning => LexicalState::Running,
            LexicalState::Running => LexicalState::Completed,
            LexicalState::Completed => LexicalState::Completed,
        };
        self.states.insert(lexical_id.clone(), next.clone());
        StateUpdate {
            lexical_id,
            new_state: next,
        }
    }
}

// NOTE: Intentionally stubbed; leave this simplistic until a later project
// fills out the real runtime-aware context tracking semantics.
#[derive(Default)]
struct ContextStack {
    frames: Vec<String>,
}

impl ContextStack {
    fn apply(&mut self, event: &EventRecord) -> Vec<String> {
        // Keep this minimal; a later pass will replace it with real call/header stack tracking.
        if self.frames.is_empty() {
            self.frames.push(format!("fn:{}", event.function));
        }

        if let Some(header) = &event.header {
            while self.frames.len() >= header.level as usize {
                self.frames.pop();
            }
            self.frames
                .push(format!("hdr:{}:{}", header.level, header.title));
        }

        self.frames.clone()
    }
}

#[derive(Debug, Clone, Serialize)]
struct StreamSnapshot {
    event: EventRecord,
    stack_after: Vec<String>,
    state_update: StateUpdate,
}

#[derive(Debug, Serialize)]
struct FixtureSnapshot {
    fixture: String,
    snapshots: Vec<StreamSnapshot>,
}

#[test]
fn viz_runtime_snapshots() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let write_artifacts = should_write_artifacts();
    let snapshot_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("viz-runtime")
        .join("snapshots");
    insta::with_settings!({snapshot_path => snapshot_root}, {
        insta::glob!("viz-runtime/testdata", "*.baml", |relative| {
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join(relative);
            let fixture_name = Path::new(relative)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();

            let snapshot = rt.block_on(run_fixture(&fixture)).expect("fixture run");

            if write_artifacts {
                write_fixture_artifacts(&snapshot, &fixture, &fixture_name)
                    .expect("write viz artifacts");
            }

            let snapshot_name = format!("viz_runtime__{}", fixture_name.replace('-', "_"));
            insta::assert_yaml_snapshot!(snapshot_name, &snapshot);
        });
    });
}

async fn run_fixture(path: &Path) -> DynResult<FixtureSnapshot> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let relative = path
        .strip_prefix(&root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let mut files = HashMap::new();
    let contents = fs::read_to_string(path)?;
    files.insert(relative, contents);

    let runtime = baml_runtime::BamlRuntime::from_file_content(
        root.to_str().expect("manifest dir to str"),
        &files,
        HashMap::<String, String>::new(),
        FeatureFlags::default(),
    )?;

    let events = Arc::new(Mutex::new(Vec::<EventRecord>::new()));
    let stacks = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let updates = Arc::new(Mutex::new(Vec::<StateUpdate>::new()));

    let reducer = Arc::new(Mutex::new(VizStateReducer::default()));
    let stack_tracker = Arc::new(Mutex::new(ContextStack::default()));

    let handler = build_watch_handler(
        events.clone(),
        stacks.clone(),
        updates.clone(),
        reducer.clone(),
        stack_tracker.clone(),
    );

    let ctx = RuntimeContextManager::new(None);
    let on_event: Option<fn(FunctionResult)> = None;
    let on_tick: Option<fn()> = None;

    let (result, _) = runtime
        .run_test_with_expr_events(
            "Main",
            "default",
            &ctx,
            on_event,
            None,
            None,
            HashMap::<String, String>::new(),
            None,
            TripWire::new(None),
            on_tick,
            Some(handler),
        )
        .await;

    result?;

    let events = events.lock().unwrap();
    let stacks = stacks.lock().unwrap();
    let updates = updates.lock().unwrap();

    let snapshots: Vec<_> = events
        .iter()
        .enumerate()
        .map(|(idx, event)| StreamSnapshot {
            event: event.clone(),
            stack_after: stacks.get(idx).cloned().unwrap_or_default(),
            state_update: updates.get(idx).cloned().unwrap(),
        })
        .collect();

    Ok(FixtureSnapshot {
        fixture: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string(),
        snapshots,
    })
}

fn build_watch_handler(
    events: Arc<Mutex<Vec<EventRecord>>>,
    stacks: Arc<Mutex<Vec<Vec<String>>>>,
    updates: Arc<Mutex<Vec<StateUpdate>>>,
    reducer: Arc<Mutex<VizStateReducer>>,
    stack_tracker: Arc<Mutex<ContextStack>>,
) -> SharedWatchHandler {
    shared_handler(move |notification: WatchNotification| {
        let event = to_event_record(&notification);

        let mut stack_guard = stack_tracker.lock().unwrap();
        let stack_after = stack_guard.apply(&event);
        drop(stack_guard);

        let mut reducer_guard = reducer.lock().unwrap();
        let update = reducer_guard.apply(&event);
        drop(reducer_guard);

        events.lock().unwrap().push(event);
        stacks.lock().unwrap().push(stack_after);
        updates.lock().unwrap().push(update);
    })
}

fn to_event_record(notification: &WatchNotification) -> EventRecord {
    let (kind, stream_id) = match &notification.value {
        WatchBamlValue::Value(_) => ("value".to_string(), None),
        WatchBamlValue::Header(_) => ("header".to_string(), None),
        WatchBamlValue::StreamStart(id) => ("stream_start".to_string(), Some(id.clone())),
        WatchBamlValue::StreamUpdate(id, _) => ("stream_update".to_string(), Some(id.clone())),
        WatchBamlValue::StreamEnd(id) => ("stream_end".to_string(), Some(id.clone())),
    };

    let header = match &notification.value {
        WatchBamlValue::Header(h) => Some(HeaderEvent {
            level: h.level,
            title: h.title.clone(),
        }),
        _ => None,
    };

    let value = match &notification.value {
        WatchBamlValue::Value(v) => serde_json::to_value(v.clone().value()).ok(),
        WatchBamlValue::StreamUpdate(_, v) => serde_json::to_value(v.clone().value()).ok(),
        _ => None,
    };

    EventRecord {
        kind,
        function: notification.function_name.clone(),
        variable: notification.variable_name.clone(),
        channel: notification.channel_name.clone(),
        stream_id,
        header,
        value,
        is_stream: notification.is_stream,
    }
}

fn write_jsonl<'a, T, I>(path: &Path, rows: I) -> DynResult<()>
where
    T: Serialize + 'a,
    I: IntoIterator<Item = &'a T>,
{
    let mut lines = String::new();
    for row in rows {
        let serialized = serde_json::to_string(row)?;
        lines.push_str(&serialized);
        lines.push('\n');
    }
    fs::write(path, lines)?;
    Ok(())
}

fn build_lexical_id(event: &EventRecord) -> String {
    // Placeholder heuristic until runtime watch notifications expose lexical IDs directly.
    if let Some(header) = &event.header {
        format!("{}|hdr:{}:{}", event.function, header.level, header.title)
    } else if let Some(chan) = &event.channel {
        format!("{}|chan:{}", event.function, chan)
    } else if let Some(var) = &event.variable {
        format!("{}|var:{}", event.function, var)
    } else {
        format!("{}|event", event.function)
    }
}

fn should_write_artifacts() -> bool {
    std::env::var_os("BAML_VIZ_WRITE_ARTIFACTS").is_some()
        || std::env::var_os("INSTA_UPDATE").is_some()
}

fn write_fixture_artifacts(
    snapshot: &FixtureSnapshot,
    fixture_path: &Path,
    fixture_name: &str,
) -> DynResult<()> {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("viz-runtime")
        .join("snapshots");
    fs::create_dir_all(&base_dir)?;
    let events_path = base_dir.join(format!("{fixture_name}.events.jsonl"));
    let stack_path = base_dir.join(format!("{fixture_name}.stack.jsonl"));
    let updates_path = base_dir.join(format!("{fixture_name}.updates.jsonl"));

    write_jsonl(
        &events_path,
        snapshot.snapshots.iter().map(|row| &row.event),
    )?;
    write_jsonl(
        &stack_path,
        snapshot.snapshots.iter().map(|row| &row.stack_after),
    )?;
    write_jsonl(
        &updates_path,
        snapshot.snapshots.iter().map(|row| &row.state_update),
    )?;

    Ok(())
}
