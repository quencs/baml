export type LexicalState = "NotRunning" | "Running" | "Completed";

export interface EventRecord {
  kind: string;
  function: string;
  variable?: string | null;
  channel?: string | null;
  stream_id?: string | null;
  header?: {
    level: number;
    title: string;
  };
  viz_event?: VizExecEvent | null;
  value?: unknown;
  is_stream: boolean;
}

export interface StateUpdate {
  lexical_id: string;
  new_state: LexicalState;
}

export type VizExecDelta = "enter" | "exit";

export interface VizExecEvent {
  event: VizExecDelta;
  node_type: string;
  lexical_id: string;
  label: string;
  header_level?: number | null;
}

export interface ReducerSnapshot {
  state_update: StateUpdate;
  state: Record<string, LexicalState>;
  emitted_events: VizExecEvent[];
}

export interface SnapshotRow {
  watch_event: EventRecord;
  stack_after: string[];
  reducer: ReducerSnapshot;
}

export interface SnapshotEntry {
  fixture: string;
  rows: SnapshotRow[];
}

export interface CombinedRow {
  index: number;
  watchEvent: EventRecord;
  stackAfter: string[];
  reducer: ReducerSnapshot;
}
