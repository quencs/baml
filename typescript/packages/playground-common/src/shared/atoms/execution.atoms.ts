/**
 * Execution Domain Atoms
 *
 * State related to workflow executions, node states, events, and caching.
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts
 */

import { atom } from 'jotai';
import { atomFamily } from 'jotai/utils';
import { activeWorkflowIdAtom, runtimeStateAtom } from './workflow.atoms';
import type {
  WasmFunctionResponse,
  WasmTestResponse,
} from '@gloo-ai/baml-schema-wasm-web';

// Watch notification type (from test-panel/types.ts)
export interface WatchNotification {
  variable_name?: string;
  channel_name?: string;
  block_name?: string;
  function_name?: string;
  test_name?: string;
  is_stream: boolean;
  value: string;
}

// Temporary type definitions - will be replaced by SDK types
export type NodeExecutionState =
  | 'not-started'
  | 'pending'
  | 'running'
  | 'success'
  | 'error'
  | 'skipped'
  | 'cached';

export type ExecutionStatus =
  | 'pending'
  | 'running'
  | 'paused'
  | 'completed'
  | 'error'
  | 'cancelled';

export interface ExecutionSnapshot {
  id: string;
  workflowId: string;
  timestamp: number;
  graphSnapshot: {
    nodes: any[];
    edges: any[];
    codeHash: string;
  };
  status: ExecutionStatus;
  nodeExecutions: Map<string, any>;
  trigger: 'manual' | 'auto' | 'test';
  duration?: number;
  branchPath: string[];
  inputs: Record<string, any>;
  outputs?: Record<string, any>;
  error?: Error;
}

export type BAMLEvent = any; // Will be defined in SDK types

export interface CacheEntry {
  nodeId: string;
  codeHash: string;
  inputs: Record<string, any>;
  inputsHash: string;
  outputs: Record<string, any>;
  executionId: string;
  timestamp: number;
  duration: number;
}

// ============================================================================
// Execution Atoms
// ============================================================================

/**
 * Per-workflow executions using atomFamily
 *
 * This allows components to subscribe to specific workflow executions,
 * avoiding re-renders when unrelated workflows update.
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:27-29
 */
export const workflowExecutionsAtomFamily = atomFamily((_workflowId: string) =>
  atom<ExecutionSnapshot[]>([])
);

/**
 * Currently active execution ID (for running executions)
 * Source: Derived from baml-graph patterns
 */
export const activeExecutionIdAtom = atom<string | null>(null);

/**
 * Currently selected execution ID (for viewing snapshots)
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:34
 */
export const selectedExecutionIdAtom = atom<string | null>(null);

/**
 * Track which workflow owns the selected execution (for lookup)
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:39
 */
const selectedExecutionWorkflowIdAtom = atom<string | null>(null);

/**
 * Derived atom for executions of the active workflow
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:44-49
 */
export const activeWorkflowExecutionsAtom = atom((get) => {
  const activeWorkflowId = get(activeWorkflowIdAtom);
  if (!activeWorkflowId) return [];

  return get(workflowExecutionsAtomFamily(activeWorkflowId));
});

/**
 * Derived atom for the currently selected execution
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:54-63
 */
export const selectedExecutionAtom = atom((get) => {
  const selectedId = get(selectedExecutionIdAtom);
  if (!selectedId) return null;

  const workflowId = get(selectedExecutionWorkflowIdAtom);
  if (!workflowId) return null;

  const executions = get(workflowExecutionsAtomFamily(workflowId));
  return executions.find((e) => e.id === selectedId) ?? null;
});

/**
 * Helper atom to select an execution and track its workflow
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:68-74
 */
export const selectExecutionAtom = atom(
  null,
  (_get, set, { executionId, workflowId }: { executionId: string; workflowId: string }) => {
    set(selectedExecutionIdAtom, executionId);
    set(selectedExecutionWorkflowIdAtom, workflowId);
  }
);

/**
 * Latest execution for the active workflow
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:79-87
 */
export const latestExecutionAtom = atom((get) => {
  const executions = get(activeWorkflowExecutionsAtom);
  if (executions.length === 0) return null;

  // Return the most recent execution
  return executions.reduce((latest, current) =>
    current.timestamp > latest.timestamp ? current : latest
  );
});

// ============================================================================
// Node State Atoms
// ============================================================================

/**
 * Per-node execution state using atomFamily
 *
 * This allows components to subscribe to specific node states,
 * avoiding re-renders when unrelated nodes update.
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:99-101
 */
export const nodeStateAtomFamily = atomFamily((_nodeId: string) =>
  atom<NodeExecutionState>('not-started')
);

/**
 * Per-node per-execution data using atomFamily
 * Key: {executionId, nodeId}
 * Source: Derived from baml-graph patterns
 */
export const nodeExecutionAtomFamily = atomFamily(
  ({ executionId, nodeId }: { executionId: string; nodeId: string }) =>
    atom<any | null>(null)
);

/**
 * Registry of active node IDs
 * Used to track which nodes exist so we can batch-read their states
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:107
 */
const activeNodeIdsAtom = atom<Set<string>>(new Set<string>());

/**
 * Helper atom to get all node states as a Map
 * This provides compatibility for code that needs to iterate over all states
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:113-122
 */
export const allNodeStatesAtom = atom((get) => {
  const nodeIds = get(activeNodeIdsAtom);
  const statesMap = new Map<string, NodeExecutionState>();

  for (const nodeId of nodeIds) {
    statesMap.set(nodeId, get(nodeStateAtomFamily(nodeId)));
  }

  return statesMap;
});

/**
 * Helper atom to register a node as active
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:127-135
 */
export const registerNodeAtom = atom(
  null,
  (get, set, nodeId: string) => {
    const current = get(activeNodeIdsAtom);
    const updated = new Set(current);
    updated.add(nodeId);
    set(activeNodeIdsAtom, updated);
  }
);

/**
 * Helper atom to clear all node states (reset to 'not-started')
 * Call this before starting a new execution
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:141-149
 */
export const clearAllNodeStatesAtom = atom(
  null,
  (get, set, _update: void) => {
    const nodeIds = get(activeNodeIdsAtom);
    for (const nodeId of nodeIds) {
      set(nodeStateAtomFamily(nodeId), 'not-started');
    }
  }
);

/**
 * Node execution data for the selected/latest execution
 * Maps nodeId -> NodeExecution
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:155-162
 */
export const nodeExecutionsAtom = atom((get) => {
  const selectedExecution = get(selectedExecutionAtom);
  const latestExecution = get(latestExecutionAtom);

  // Prefer selected execution, fall back to latest
  const execution = selectedExecution ?? latestExecution;
  return execution?.nodeExecutions ?? new Map();
});

// ============================================================================
// Event Stream Atoms
// ============================================================================

/**
 * Event stream for real-time updates
 * Components can subscribe to this to receive events
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:172
 */
export const executionEventStreamAtom = atom<BAMLEvent[]>([]);

/**
 * Writable atom to add events
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:177-188
 */
export const addEventAtom = atom(
  null,
  (get, set, event: BAMLEvent) => {
    const currentEvents = get(executionEventStreamAtom);
    const updatedEvents = [...currentEvents, event];

    // Keep only last 100 events to prevent memory leaks
    set(executionEventStreamAtom, updatedEvents.slice(-100));
  }
);

// ============================================================================
// Cache Atoms
// ============================================================================

/**
 * Per-node cache entries using atomFamily
 * Source: Derived from baml-graph patterns
 */
export const cacheEntriesAtomFamily = atomFamily((_nodeId: string) =>
  atom<CacheEntry[]>([])
);

/**
 * Cache storage
 * Maps `${nodeId}:${inputsHash}` -> CacheEntry
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:198
 */
export const cacheAtom = atom<Map<string, CacheEntry>>(new Map());

/**
 * Get cache key helper
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:203-205
 */
export const getCacheKey = (nodeId: string, inputsHash: string): string => {
  return `${nodeId}:${inputsHash}`;
};

// ============================================================================
// Test Execution Atoms (from playground-common)
// ============================================================================

/**
 * Test status types
 * Source: playground-panel/atoms.ts:210-238
 */
export type TestStatusType = 'queued' | 'running' | 'done' | 'error' | 'idle';
export type DoneTestStatusType =
  | 'passed'
  | 'llm_failed'
  | 'parse_failed'
  | 'constraints_failed'
  | 'assert_failed'
  | 'error';
export type TestState =
  | {
      status: 'queued' | 'idle';
    }
  | {
      status: 'running';
      response?: WasmFunctionResponse;
      watchNotifications?: WatchNotification[];
    }
  | {
      status: 'done';
      response_status: DoneTestStatusType;
      response: WasmTestResponse;
      latency_ms: number;
      watchNotifications?: WatchNotification[];
    }
  | {
      status: 'error';
      message: string;
      watchNotifications?: WatchNotification[];
    };

/**
 * Test case atomFamily for lookup
 * Source: playground-panel/atoms.ts:240-251
 */
export const testCaseAtom = atomFamily(
  (params: { functionName: string; testName: string }) =>
    atom((get) => {
      const { functions } = get(runtimeStateAtom);
      const fn = functions.find((f) => f.name === params.functionName);
      const tc = fn?.test_cases.find((tc) => tc.name === params.testName);
      if (!fn || !tc) {
        return undefined;
      }
      return { fn, tc };
    })
);

/**
 * Function test snippet atomFamily
 * Source: playground-panel/atoms.ts:253-262
 */
export const functionTestSnippetAtom = atomFamily((functionName: string) =>
  atom((get) => {
    const { functions } = get(runtimeStateAtom);
    const fn = functions.find((f) => f.name === functionName);
    if (!fn) {
      return undefined;
    }
    return fn.test_snippet;
  })
);

/**
 * Test case response atomFamily
 * Source: playground-panel/atoms.ts:264-276
 */
export const testCaseResponseAtom = atomFamily(
  (params: { functionName?: string; testName?: string }) =>
    atom((get) => {
      const allTestCaseResponse = get(runningTestsAtom);
      const testCaseResponse = allTestCaseResponse.find(
        (t) =>
          t.functionName === params.functionName && t.testName === params.testName,
        undefined
      );
      return testCaseResponse?.state;
    })
);

/**
 * Are tests running flag
 * Source: playground-panel/atoms.ts:277
 */
export const areTestsRunningAtom = atom(false);

/**
 * Running tests array
 * Source: playground-panel/atoms.ts:279-281
 */
export const runningTestsAtom = atom<
  { functionName: string; testName: string; state: TestState }[]
>([]);

/**
 * AbortController for cancelling running tests
 * Source: playground-panel/atoms.ts:284
 */
export const currentAbortControllerAtom = atom<AbortController | null>(null);
