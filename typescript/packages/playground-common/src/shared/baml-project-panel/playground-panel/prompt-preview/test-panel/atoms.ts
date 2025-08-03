import { atom } from 'jotai';
import { atomWithStorage } from 'jotai/utils';
import type { TestState } from '../../atoms';
import type { WasmCancellationToken } from '@gloo-ai/baml-schema-wasm-web';

export interface TestHistoryEntry {
  timestamp: number;
  functionName: string;
  testName: string;
  response: TestState;
  input?: any;
}

export interface TestHistoryRun {
  timestamp: number;
  tests: TestHistoryEntry[];
}

export type TestStatusType = 'queued' | 'running' | 'done' | 'error' | 'idle' | 'cancelled';
export type DoneTestStatusType =
  | 'passed'
  | 'llm_failed'
  | 'parse_failed'
  | 'constraints_failed'
  | 'assert_failed'
  | 'cancelled'
  | 'error';

// TODO: make this persistent, but make sure to serialize the wasm objects properly
export const testHistoryAtom = atom<TestHistoryRun[]>([]);
export const selectedHistoryIndexAtom = atom<number>(0);
export const isParallelTestsEnabledAtom = atomWithStorage<boolean>(
  'runTestsInParallel',
  true,
);

// Cancellation state
export const isCancellingAtom = atom<boolean>(false);

// Atom to store the active cancellation token
export const activeCancellationTokenAtom = atom<WasmCancellationToken | null>(null);
