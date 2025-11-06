/**
 * Execution Domain Atoms
 *
 * State related to workflow executions, node states, events, and caching.
 */

import { atom } from 'jotai';
import { atomFamily } from 'jotai/utils';
import type {
  ExecutionSnapshot,
  BAMLEvent,
  CacheEntry,
  NodeExecutionState,
} from '../types';
import { activeWorkflowIdAtom } from './workflow.atoms';

// ============================================================================
// Execution Atoms
// ============================================================================

/**
 * Per-workflow executions using atomFamily
 *
 * This allows components to subscribe to specific workflow executions,
 * avoiding re-renders when unrelated workflows update.
 */
export const workflowExecutionsAtomFamily = atomFamily((_workflowId: string) =>
  atom<ExecutionSnapshot[]>([])
);

/**
 * Currently selected execution ID (for viewing snapshots)
 */
export const selectedExecutionIdAtom = atom<string | null>(null);

/**
 * Track which workflow owns the selected execution (for lookup)
 */
const selectedExecutionWorkflowIdAtom = atom<string | null>(null);

/**
 * Derived atom for executions of the active workflow
 */
export const activeWorkflowExecutionsAtom = atom((get) => {
  const activeWorkflowId = get(activeWorkflowIdAtom);
  if (!activeWorkflowId) return [];

  return get(workflowExecutionsAtomFamily(activeWorkflowId));
});

/**
 * Derived atom for the currently selected execution
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
 */
export const nodeStateAtomFamily = atomFamily((_nodeId: string) =>
  atom<NodeExecutionState>('not-started')
);

/**
 * Registry of active node IDs
 * Used to track which nodes exist so we can batch-read their states
 */
const activeNodeIdsAtom = atom<Set<string>>(new Set<string>());

/**
 * Helper atom to get all node states as a Map
 * This provides compatibility for code that needs to iterate over all states
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
 */
export const eventStreamAtom = atom<BAMLEvent[]>([]);

/**
 * Writable atom to add events
 */
export const addEventAtom = atom(
  null,
  (get, set, event: BAMLEvent) => {
    const currentEvents = get(eventStreamAtom);
    set(eventStreamAtom, [...currentEvents, event]);

    // Keep only last 100 events to prevent memory leaks
    if (currentEvents.length > 100) {
      set(eventStreamAtom, currentEvents.slice(-100));
    }
  }
);

// ============================================================================
// Cache Atoms
// ============================================================================

/**
 * Cache storage
 * Maps `${nodeId}:${inputsHash}` -> CacheEntry
 */
export const cacheAtom = atom<Map<string, CacheEntry>>(new Map());

/**
 * Get cache key helper
 */
export const getCacheKey = (nodeId: string, inputsHash: string): string => {
  return `${nodeId}:${inputsHash}`;
};
