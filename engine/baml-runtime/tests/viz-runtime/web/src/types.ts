export type LexicalState = "not_running" | "running" | "completed";

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
  path_segment: {
    kind: string;
    slug?: string;
    ordinal: number;
  };
  label: string;
  header_level?: number | null;
}

export interface SnapshotRow {
  watch_event: EventRecord;
  stack_after?: string[] | null;
  emitted_events?: StateUpdate[] | null;
  state?: {
    nodes: Record<string, LexicalState>;
    frames: Frame[];
  } | null;
}

export interface SnapshotEntry {
  fixture: string;
  inputFile?: string;
  rows: SnapshotRow[];
}

export interface CombinedRow {
  index: number;
  watchEvent: EventRecord;
  stackAfter: string[];
  emittedEvents: StateUpdate[];
  state: {
    nodes: Record<string, LexicalState>;
    frames: Frame[];
  };
}

export interface Frame {
  lexical_id: string;
  node_type: string;
  label: string;
  header_level?: number | null;
}
