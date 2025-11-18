use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use baml_compiler::watch::{shared_handler, SharedWatchHandler, WatchBamlValue, WatchNotification};
use baml_compiler::watch::{VizExecDelta, VizExecEvent};
use baml_runtime::{FunctionResult, RuntimeContextManager, TripWire};
use internal_baml_core::feature_flags::FeatureFlags;
use serde::Serialize;
use serde_json::Value;

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
struct ReducerSnapshot {
    state_update: StateUpdate,
    state: BTreeMap<String, LexicalState>,
    emitted_events: Vec<VizExecEvent>,
}

#[derive(Debug, Clone, Serialize)]
struct EventRecord {
    kind: String,
    function: String,
    variable: Option<String>,
    channel: Option<String>,
    stream_id: Option<String>,
    header: Option<HeaderEvent>,
    viz_event: Option<VizExecEvent>,
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
    fn apply(&mut self, event: &EventRecord) -> ReducerSnapshot {
        let lexical_id = build_lexical_id(event);
        let next = if let Some(viz_event) = &event.viz_event {
            match viz_event.event {
                VizExecDelta::Enter => LexicalState::Running,
                VizExecDelta::Exit => LexicalState::Completed,
            }
        } else {
            let current = self
                .states
                .get(&lexical_id)
                .copied()
                .unwrap_or(LexicalState::NotRunning);
            match current {
                LexicalState::NotRunning => LexicalState::Running,
                LexicalState::Running => LexicalState::Completed,
                LexicalState::Completed => LexicalState::Completed,
            }
        };

        self.states.insert(lexical_id.clone(), next.clone());
        ReducerSnapshot {
            state_update: StateUpdate {
                lexical_id,
                new_state: next,
            },
            state: self
                .states
                .iter()
                .map(|(key, state)| (key.clone(), *state))
                .collect(),
            emitted_events: Vec::new(),
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

        if let Some(viz_event) = &event.viz_event {
            match viz_event.event {
                VizExecDelta::Enter => {
                    // HeaderContextEnter uses header_level to dedent before pushing.
                    if let Some(level) = viz_event.header_level {
                        while self.frames.len() > level as usize {
                            self.frames.pop();
                        }
                    }
                    self.frames
                        .push(format!("viz:{}", viz_event.lexical_id.clone()));
                }
                VizExecDelta::Exit => {
                    let expected = format!("viz:{}", viz_event.lexical_id);
                    if self.frames.last().is_some_and(|top| top == &expected) {
                        self.frames.pop();
                    }
                }
            }

            return self.frames.clone();
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
    watch_event: EventRecord,
    stack_after: Vec<String>,
    reducer: ReducerSnapshot,
}

#[derive(Debug, Serialize)]
struct FixtureSnapshot {
    fixture: String,
    snapshots: Vec<StreamSnapshot>,
}

#[test]
fn viz_runtime_snapshots() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
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

            let snapshot_name = format!("viz_runtime__{}", fixture_name.replace('-', "_"));
            insta::assert_yaml_snapshot!(snapshot_name, &snapshot);
        });
    });
}

async fn run_fixture(path: &Path) -> anyhow::Result<FixtureSnapshot> {
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
    let reducer_snapshots = Arc::new(Mutex::new(Vec::<ReducerSnapshot>::new()));

    let reducer = Arc::new(Mutex::new(VizStateReducer::default()));
    let stack_tracker = Arc::new(Mutex::new(ContextStack::default()));

    let handler = build_watch_handler(
        events.clone(),
        stacks.clone(),
        reducer_snapshots.clone(),
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
    let reducer_snapshots = reducer_snapshots.lock().unwrap();

    let snapshots: Vec<_> = events
        .iter()
        .enumerate()
        .map(|(idx, event)| StreamSnapshot {
            watch_event: event.clone(),
            stack_after: stacks.get(idx).cloned().unwrap_or_default(),
            reducer: reducer_snapshots.get(idx).cloned().unwrap(),
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
    reducer_snapshots: Arc<Mutex<Vec<ReducerSnapshot>>>,
    reducer: Arc<Mutex<VizStateReducer>>,
    stack_tracker: Arc<Mutex<ContextStack>>,
) -> SharedWatchHandler {
    shared_handler(move |notification: WatchNotification| {
        let event = to_event_record(&notification);

        let mut stack_guard = stack_tracker.lock().unwrap();
        let stack_after = stack_guard.apply(&event);
        drop(stack_guard);

        let mut reducer_guard = reducer.lock().unwrap();
        let reducer_snapshot = reducer_guard.apply(&event);
        drop(reducer_guard);

        events.lock().unwrap().push(event);
        stacks.lock().unwrap().push(stack_after);
        reducer_snapshots
            .lock()
            .unwrap()
            .push(reducer_snapshot);
    })
}

fn to_event_record(notification: &WatchNotification) -> EventRecord {
    let (kind, stream_id, viz_event) = match &notification.value {
        WatchBamlValue::Value(_) => ("value".to_string(), None, None),
        WatchBamlValue::Header(_) => ("header".to_string(), None, None),
        WatchBamlValue::VizExecState(event) => {
            ("viz_exec_state".to_string(), None, Some(event.clone()))
        }
        WatchBamlValue::StreamStart(id) => ("stream_start".to_string(), Some(id.clone()), None),
        WatchBamlValue::StreamUpdate(id, _) => {
            ("stream_update".to_string(), Some(id.clone()), None)
        }
        WatchBamlValue::StreamEnd(id) => ("stream_end".to_string(), Some(id.clone()), None),
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
        viz_event,
        value,
        is_stream: notification.is_stream,
    }
}

fn build_lexical_id(event: &EventRecord) -> String {
    if let Some(viz) = &event.viz_event {
        return viz.lexical_id.clone();
    }

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
