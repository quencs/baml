import { atom, useAtom } from 'jotai';
import { atomFamily } from 'jotai/utils';

// Base types
export type TestState = {
  status: 'idle' | 'running' | 'done' | 'error';
  response?: any; // Using any for now to avoid WASM dependency
  message?: string;
  latency_ms: number;
  timestamp: number;
};

export type DoneTestStatusType = 'passed' | 'failed' | 'error';

// Atoms
export const selectedItemAtom = atom<[string, string] | null>(null);

export const testcaseObjectAtom = atomFamily((params: { functionName: string; testcaseName: string }) =>
  atom<{ tc: any; fn: any; span?: any } | null>(null)
);

// Re-export wasmAtom from shared directory for compatibility
export { wasmAtom } from './shared/baml-project-panel/atoms';

// Utility setter for selectedItemAtom  
export const setSelectedItemAtom = atom(
  null,
  (get, set, functionName: string, testName: string) => {
    set(selectedItemAtom, [functionName, testName]);
  }
);

// Export the setter function for use in components
export const useSetSelectedItem = () => {
  const [, setSelectedItem] = useAtom(setSelectedItemAtom);
  return (functionName: string, testName: string) => setSelectedItem(functionName, testName);
};

// Re-export types for convenience
export type { TestCase, TestResult, TestHistoryRun } from './types';