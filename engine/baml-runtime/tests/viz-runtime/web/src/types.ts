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
  value?: unknown;
  is_stream: boolean;
}

export interface StateUpdate {
  lexical_id: string;
  new_state: LexicalState;
}

export interface SnapshotEntry {
  fixture: string;
  eventsText: string;
  stackText: string;
  updatesText: string;
}

export interface CombinedRow {
  index: number;
  event: EventRecord;
  stack: string[];
  update: StateUpdate;
}
