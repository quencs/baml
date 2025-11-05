# Phase 1: Unified Atom Structure

**Timeline:** Week 1-2
**Dependencies:** None (foundational phase)
**Risk Level:** Medium

---

## Purpose

Merge and consolidate Jotai atoms from `baml-graph` and `playground-common` into a unified, domain-organized atom structure. This eliminates duplication, reduces complexity (70+ atoms → ~50 atoms), and establishes a single source of truth for state management.

## What This Document Covers

- Complete atom inventory from both codebases
- Mapping between old and new atom names
- Consolidation strategy for overlapping atoms
- Migration path for existing consumers
- Storage strategy (localStorage, sessionStorage, in-memory)
- Performance optimizations (atomFamily usage patterns)
- Type definitions for all atoms
- Backward compatibility adapters during migration
- Validation tests for atom behavior parity

---

## Key Decisions

1. **Domain-organized structure**: Group atoms by domain (`workflow.atoms.ts`, `execution.atoms.ts`, `runtime.atoms.ts`, `ui.atoms.ts`, `derived.atoms.ts`)
2. **Use `atomFamily` for per-entity state**: Workflows, nodes, tests get their own atomFamily instances for granular subscriptions
3. **Preserve `playground-common` WASM/runtime atoms**: These are battle-tested in production
4. **Add `baml-graph` execution/workflow atoms**: These have better performance patterns (atomFamily)
5. **Create adapter layer for gradual migration**: Allow old code to continue working during transition
6. **Minimal localStorage usage**: Only for true user preferences, not for transient UI state

---

## Complete Atom Inventory

### From baml-graph (35 atoms total)

#### `apps/baml-graph/src/sdk/atoms/workflow.atoms.ts` (lines 17-38)
```typescript
// 4 atoms total
workflowsAtom                  // All workflow definitions
activeWorkflowIdAtom          // Currently selected workflow ID
activeWorkflowAtom            // Derived active workflow (computed)
recentWorkflowsAtom           // Recent workflow access tracking
```

#### `apps/baml-graph/src/sdk/atoms/execution.atoms.ts` (lines 27-206)
```typescript
// 12 atoms total
workflowExecutionsAtomFamily(workflowId)   // Per-workflow executions (atomFamily)
selectedExecutionIdAtom                     // Current execution view
selectedExecutionWorkflowIdAtom            // Track which workflow owns execution
activeWorkflowExecutionsAtom               // Derived executions for active workflow
selectedExecutionAtom                      // Derived selected execution
selectExecutionAtom                        // Helper to select + track
latestExecutionAtom                        // Most recent execution
nodeStateAtomFamily(nodeId)                // Per-node execution states (atomFamily)
activeNodeIdsAtom                          // Registry of active nodes
allNodeStatesAtom                          // All node states as Map
registerNodeAtom                           // Register node as active
clearAllNodeStatesAtom                     // Reset all nodes
nodeExecutionsAtom                         // Node execution data
eventStreamAtom                            // Real-time events (last 100)
addEventAtom                               // Write-only event emitter
cacheAtom                                  // Cache storage Map
```

#### `apps/baml-graph/src/sdk/atoms/ui.atoms.ts` (lines 17-93)
```typescript
// 10 atoms total
viewModeAtom                   // 'editor' | 'execution' snapshot mode
selectedNodeIdAtom             // Selected graph node
detailPanelAtom                // Panel state (open, position, activeTab)
layoutDirectionAtom            // 'horizontal' | 'vertical'
selectedInputSourceAtom        // Input source selection
activeNodeInputsAtom           // Editable inputs
inputsDirtyAtom                // Whether inputs modified
bamlFilesAtom                  // All BAML files with functions/tests
activeCodeClickAtom            // Code click events for navigation
```

#### `apps/baml-graph/src/sdk/atoms/derived.atoms.ts` (lines 26-179)
```typescript
// 9 atoms total (all computed)
allFunctionsMapAtom            // O(1) function lookup Map
functionsByTypeAtom            // Functions grouped by type
workflowFunctionIdsAtom        // Set of IDs in any workflow
standaloneFunctionsAtom        // Functions NOT in workflows
selectedFunctionAtom           // Currently selected function
isSelectedFunctionStandaloneAtom  // Whether selected is standalone
isLLMOnlyModeAtom              // Complex LLM-only mode detection
```

### From playground-common (70+ atoms total)

#### `packages/playground-common/src/shared/baml-project-panel/atoms.ts` (lines 1-339)
```typescript
// 16 atoms total - WASM & Runtime
wasmPanicAtom                  // WASM panic tracking
betaFeatureEnabledAtom         // Feature flag (VSCode + standalone)
wasmAtomAsync                  // Async WASM loading
wasmAtom                       // Unwrapped WASM
filesAtom                      // File path → content map
sandboxFilesAtom               // Sandbox files
projectAtom                    // WASM project from files
ctxAtom                        // WASM call context
runtimeAtom                    // WASM runtime with diagnostics
diagnosticsAtom                // Compilation errors/warnings
numErrorsAtom                  // Error/warning counts
generatedFilesAtom             // Generated code files
generatedFilesByLangAtom(lang) // atomFamily by language
isPanelVisibleAtom             // Panel visibility
vscodeSettingsAtom             // VSCode settings via RPC
playgroundPortAtom             // Proxy port
proxyUrlAtom                   // Proxy configuration
```

#### `packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts` (lines 1-264)
```typescript
// 14 atoms total - Function/Test Selection & Execution
runtimeStateAtom               // { functions, stale }
selectedFunctionAtom           // Selected function name (string)
selectedTestcaseAtom           // Selected test name (string)
selectedItemAtom               // [functionName, testName] tuple
functionObjectAtom(name)       // atomFamily for function objects
testcaseObjectAtom({fn,tc})    // atomFamily for test objects
updateCursorAtom               // Write-only cursor update
selectionAtom                  // Derived {selectedFn, selectedTc}
selectedFunctionObjectAtom     // Derived selected function
testCaseAtom({fn, test})       // atomFamily for test case pairs
functionTestSnippetAtom(fn)    // atomFamily for test snippets
testCaseResponseAtom({fn,tc})  // atomFamily for test responses
areTestsRunningAtom            // Boolean running flag
runningTestsAtom               // Array of test states
currentAbortControllerAtom     // AbortController for cancellation
flashRangesAtom                // Code highlight ranges
```

#### `packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/atoms.ts` (lines 1-115)
```typescript
// 6 atoms total - Test History
testHistoryAtom                // Array of test runs
selectedHistoryIndexAtom       // Current history view
isParallelTestsEnabledAtom     // Parallel execution flag (localStorage)
currentWatchNotificationsAtom  // Watch notifications
highlightedBlocksAtom          // Set of highlighted blocks
categorizedNotificationsAtom   // Derived categorized notifications
```

#### `packages/playground-common/src/components/api-keys-dialog/atoms.ts` (lines 1-459)
```typescript
// 25+ atoms total - API Key Management
apiKeyVisibilityAtom           // Visibility state per key
hasShownApiKeyDialogAtom       // localStorage flag
apiKeyDialogOpenAtom           // Dialog open state
showApiKeyDialogAtom           // Computed dialog visibility
resetEnvKeyValuesAtom          // Reset helper
envKeyValueStorage             // atomWithStorage
envKeyValuesAtom               // Key-value pairs with indices
userApiKeysAtom                // User's keys (excludes BOUNDARY_PROXY_URL)
apiKeysAtom                    // Keys with proxy logic
requiredApiKeysAtom            // Required keys from runtime
localApiKeysAtom               // Local copy for editing
hasLocalChangesAtom            // Unsaved changes flag
renderedApiKeysAtom            // Computed list for UI
saveApiKeyChangesAtom          // Save helper
... (10+ more atoms)
```

---

## Unified Atom Structure

### Directory Layout

```
packages/playground-common/src/shared/atoms/
├── index.ts                    # Main export file with JSDoc
├── workflow.atoms.ts           # Workflow definitions and selection (10 atoms)
├── execution.atoms.ts          # Execution state and history (18 atoms)
├── runtime.atoms.ts            # WASM runtime and compilation (17 atoms)
├── ui.atoms.ts                # UI state and interactions (15 atoms)
├── derived.atoms.ts           # Computed/optimized atoms (12 atoms)
├── api-keys.atoms.ts          # API key management (keep existing ~25 atoms)
└── legacy.atoms.ts            # Deprecated atoms during migration
```

**Total: ~97 atoms → ~97 atoms initially (no reduction yet)**
**After migration complete: ~70 atoms (remove duplicates & legacy)**

---

## 1. workflow.atoms.ts

**Purpose**: Workflow definitions, function selection, and navigation state

**Consolidates**:
- `apps/baml-graph/src/sdk/atoms/workflow.atoms.ts` (entire file)
- `packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:13-32` (runtime state)
- Parts of `playground-panel/atoms.ts:34-177` (selection atoms)

### Implementation

```typescript
/**
 * Workflow Domain Atoms
 *
 * Manages workflow definitions, function selection, and navigation state.
 * Consolidates workflow management from baml-graph with function selection from playground-common.
 */

import { atom } from 'jotai';
import { atomFamily } from 'jotai/utils';
import type { WorkflowDefinition, WasmFunction } from '../types';
import { runtimeAtom } from './runtime.atoms';

// ============================================================================
// Workflow Definition Atoms (from baml-graph)
// ============================================================================

/**
 * All available workflows (all functions in the codebase)
 *
 * Source: apps/baml-graph/src/sdk/atoms/workflow.atoms.ts:17
 */
export const workflowsAtom = atom<WorkflowDefinition[]>([]);

/**
 * Currently selected/active workflow ID
 *
 * Source: apps/baml-graph/src/sdk/atoms/workflow.atoms.ts:22
 */
export const activeWorkflowIdAtom = atom<string | null>(null);

/**
 * Derived atom for the active workflow
 *
 * Source: apps/baml-graph/src/sdk/atoms/workflow.atoms.ts:27-31
 */
export const activeWorkflowAtom = atom((get) => {
  const workflows = get(workflowsAtom);
  const activeId = get(activeWorkflowIdAtom);
  return workflows.find((w) => w.id === activeId) ?? null;
});

/**
 * Recent workflows (for quick access)
 *
 * Source: apps/baml-graph/src/sdk/atoms/workflow.atoms.ts:36-38
 */
export const recentWorkflowsAtom = atom<
  Array<{ workflowId: string; lastAccessed: number }>
>([]);

// ============================================================================
// Runtime Function List (from playground-common)
// ============================================================================

/**
 * Runtime state with all functions from WASM
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:13-32
 *
 * Includes both LLM functions and expr functions from the compiled runtime.
 */
export const runtimeStateAtom = atom((get) => {
  const { rt, lastValidRt } = get(runtimeAtom);

  if (rt === undefined) {
    if (lastValidRt === undefined) {
      return { functions: [], stale: false };
    }
    // Include both LLM functions and expr functions
    const llmFunctions = lastValidRt.list_functions();
    const exprFunctions = lastValidRt.list_expr_fns();
    return { functions: [...llmFunctions, ...exprFunctions], stale: true };
  }

  // Include both LLM functions and expr functions
  const llmFunctions = rt.list_functions();
  const exprFunctions = rt.list_expr_fns();
  return { functions: [...llmFunctions, ...exprFunctions], stale: false };
});

// ============================================================================
// Function & Test Selection (from playground-common, adapted for workflows)
// ============================================================================

/**
 * Selected function name (as string)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:34
 *
 * MIGRATION NOTE: This is the playground-common pattern (string-based selection).
 * In unified system, this bridges to selectedNodeIdAtom for graph-based selection.
 */
export const selectedFunctionNameAtom = atom<string | undefined>(undefined);

/**
 * Selected testcase name (as string)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:35
 */
export const selectedTestcaseNameAtom = atom<string | undefined>(undefined);

/**
 * Combined selection as tuple [functionName, testName]
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:37-55
 *
 * This provides backward compatibility for code expecting tuple format.
 */
export const selectedItemAtom = atom(
  (get) => {
    const selected = get(selectionAtom);
    if (selected.selectedFn === undefined || selected.selectedTc === undefined) {
      return undefined;
    }
    return [selected.selectedFn.name, selected.selectedTc.name] as [string, string];
  },
  (_, set, functionName: string, testcaseName: string | undefined) => {
    set(selectedFunctionNameAtom, functionName);
    set(selectedTestcaseNameAtom, testcaseName);
  }
);

/**
 * AtomFamily for function objects (O(1) lookup)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:57-66
 */
export const functionObjectAtom = atomFamily((functionName: string) =>
  atom((get) => {
    const { functions } = get(runtimeStateAtom);
    const fn = functions.find((f) => f.name === functionName);
    if (!fn) {
      return undefined;
    }
    return fn;
  })
);

/**
 * AtomFamily for test case objects
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:68-82
 */
export const testcaseObjectAtom = atomFamily(
  (params: { functionName: string; testcaseName?: string | null }) =>
    atom((get) => {
      const { functions } = get(runtimeStateAtom);
      const fn = functions.find((f) => f.name === params.functionName);
      if (!fn) {
        return undefined;
      }
      const tc = fn.test_cases.find((tc) => tc.name === params.testcaseName);
      if (!tc) {
        return undefined;
      }
      return tc;
    })
);

/**
 * Cursor update handler (write-only)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:84-139
 *
 * MIGRATION NOTE: Phase 6 will enhance this to create CodeClickEvent objects.
 * For now, keep existing behavior for backward compatibility.
 */
export const updateCursorAtom = atom(
  null,
  (get, set, cursor: { fileName: string; line: number; column: number }) => {
    const runtime = get(runtimeAtom)?.rt;
    if (!runtime) {
      return;
    }
    const fileContent = get(filesAtom)[cursor.fileName];
    if (!fileContent) {
      return;
    }

    const fileName = cursor.fileName;
    const lines = fileContent.split('\n');

    let cursorIdx = 0;
    for (let i = 0; i < cursor.line; i++) {
      cursorIdx += (lines[i]?.length ?? 0) + 1; // +1 for the newline character
    }
    cursorIdx += cursor.column;

    const selectedFunc = runtime.get_function_at_position(
      fileName,
      get(selectedFunctionNameAtom) ?? '',
      cursorIdx
    );

    if (selectedFunc) {
      set(selectedFunctionNameAtom, selectedFunc.name);
      const selectedTestcase = runtime.get_testcase_from_position(selectedFunc, cursorIdx);

      if (selectedTestcase) {
        set(selectedTestcaseNameAtom, selectedTestcase.name);
        const nestedFunc = runtime.get_function_of_testcase(fileName, cursorIdx);

        if (nestedFunc) {
          set(selectedFunctionNameAtom, nestedFunc.name);
        }
      }
    }
  }
);

/**
 * Derived selection state with full objects
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:141-172
 */
export const selectionAtom = atom((get) => {
  const selectedFunction = get(selectedFunctionNameAtom);
  const selectedTestcase = get(selectedTestcaseNameAtom);

  const state = get(runtimeStateAtom);

  let selectedFn = state.functions.at(0);
  if (selectedFunction !== undefined) {
    const foundFn = state.functions.find((f) => f.name === selectedFunction);
    if (foundFn) {
      selectedFn = foundFn;
    } else {
      console.error('Function not found', selectedFunction);
    }
  } else {
    console.debug('No function selected');
  }

  let selectedTc = selectedFn?.test_cases.at(0);
  if (selectedTestcase !== undefined) {
    const foundTc = selectedFn?.test_cases.find((tc) => tc.name === selectedTestcase);
    if (foundTc) {
      selectedTc = foundTc;
    } else {
      console.error('Testcase not found', selectedTestcase);
    }
  }

  return { selectedFn, selectedTc };
});

/**
 * Derived selected function object (convenience)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:174-177
 */
export const selectedFunctionObjectAtom = atom((get) => {
  const { selectedFn } = get(selectionAtom);
  return selectedFn;
});

// ============================================================================
// Re-exports for type compatibility
// ============================================================================

// Re-export with backward-compatible name
export const selectedFunctionAtom = selectedFunctionNameAtom;
export const selectedTestcaseAtom = selectedTestcaseNameAtom;
```

### Migration Notes

1. **Dual selection systems**: Keep both `selectedFunctionNameAtom` (string) and `selectedNodeIdAtom` (from ui.atoms.ts) during migration
2. **Bridge in derived.atoms.ts**: Create adapter that keeps them in sync
3. **Phase 6 enhancement**: `updateCursorAtom` will be enhanced to create `CodeClickEvent` objects

---

## 2. execution.atoms.ts

**Purpose**: Execution state, test execution, event streaming, and caching

**Consolidates**:
- `apps/baml-graph/src/sdk/atoms/execution.atoms.ts` (entire file)
- `packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:179-264` (test execution)
- `packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/atoms.ts:21-114` (test history)

### Implementation

```typescript
/**
 * Execution Domain Atoms
 *
 * Manages workflow executions, node states, test execution, events, and caching.
 * Consolidates execution tracking from baml-graph with test execution from playground-common.
 */

import { atom } from 'jotai';
import { atomFamily, atomWithStorage } from 'jotai/utils';
import type {
  ExecutionSnapshot,
  BAMLEvent,
  CacheEntry,
  NodeExecutionState,
  TestState,
  TestHistoryEntry,
  TestHistoryRun,
  WatchNotification,
} from '../types';
import { activeWorkflowIdAtom } from './workflow.atoms';
import { sessionStore } from '../../baml_wasm_web/JotaiProvider';

// ============================================================================
// Workflow Execution Atoms (from baml-graph)
// ============================================================================

/**
 * Per-workflow executions using atomFamily
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:27-29
 *
 * This allows components to subscribe to specific workflow executions,
 * avoiding re-renders when unrelated workflows update.
 */
export const workflowExecutionsAtomFamily = atomFamily((_workflowId: string) =>
  atom<ExecutionSnapshot[]>([])
);

/**
 * Currently selected execution ID (for viewing snapshots)
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:34
 */
export const selectedExecutionIdAtom = atom<string | null>(null);

/**
 * Track which workflow owns the selected execution (for lookup)
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:39
 */
const selectedExecutionWorkflowIdAtom = atom<string | null>(null);

/**
 * Derived atom for executions of the active workflow
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:44-49
 */
export const activeWorkflowExecutionsAtom = atom((get) => {
  const activeWorkflowId = get(activeWorkflowIdAtom);
  if (!activeWorkflowId) return [];

  return get(workflowExecutionsAtomFamily(activeWorkflowId));
});

/**
 * Derived atom for the currently selected execution
 *
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
 *
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
 *
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
// Node State Atoms (from baml-graph)
// ============================================================================

/**
 * Per-node execution state using atomFamily
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:99-101
 *
 * This allows components to subscribe to specific node states,
 * avoiding re-renders when unrelated nodes update.
 */
export const nodeStateAtomFamily = atomFamily((_nodeId: string) =>
  atom<NodeExecutionState>('not-started')
);

/**
 * Registry of active node IDs
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:107
 *
 * Used to track which nodes exist so we can batch-read their states
 */
const activeNodeIdsAtom = atom<Set<string>>(new Set<string>());

/**
 * Helper atom to get all node states as a Map
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:113-122
 *
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
 *
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
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:141-149
 *
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
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:155-162
 *
 * Maps nodeId → NodeExecution
 */
export const nodeExecutionsAtom = atom((get) => {
  const selectedExecution = get(selectedExecutionAtom);
  const latestExecution = get(latestExecutionAtom);

  // Prefer selected execution, fall back to latest
  const execution = selectedExecution ?? latestExecution;
  return execution?.nodeExecutions ?? new Map();
});

// ============================================================================
// Event Stream Atoms (from baml-graph)
// ============================================================================

/**
 * Event stream for real-time updates
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:172
 *
 * Components can subscribe to this to receive events
 */
export const eventStreamAtom = atom<BAMLEvent[]>([]);

/**
 * Writable atom to add events
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:177-188
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
// Cache Atoms (from baml-graph)
// ============================================================================

/**
 * Cache storage
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:198
 *
 * Maps `${nodeId}:${inputsHash}` → CacheEntry
 */
export const cacheAtom = atom<Map<string, CacheEntry>>(new Map());

/**
 * Get cache key helper
 *
 * Source: apps/baml-graph/src/sdk/atoms/execution.atoms.ts:203-205
 */
export const getCacheKey = (nodeId: string, inputsHash: string): string => {
  return `${nodeId}:${inputsHash}`;
};

// ============================================================================
// Test Execution Atoms (from playground-common)
// ============================================================================

/**
 * Test case atomFamily for lookup
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:209-220
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
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:222-231
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
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:233-245
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
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:246
 */
export const areTestsRunningAtom = atom(false);

/**
 * Running tests array
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:248-250
 */
export const runningTestsAtom = atom<
  { functionName: string; testName: string; state: TestState }[]
>([]);

/**
 * AbortController for cancelling running tests
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:253
 */
export const currentAbortControllerAtom = atom<AbortController | null>(null);

// ============================================================================
// Test History Atoms (from playground-common test-panel)
// ============================================================================

/**
 * Test history (all runs)
 *
 * Source: packages/playground-common/.../prompt-preview/test-panel/atoms.ts:21
 */
export const testHistoryAtom = atom<TestHistoryRun[]>([]);

/**
 * Selected history index
 *
 * Source: packages/playground-common/.../prompt-preview/test-panel/atoms.ts:22
 */
export const selectedHistoryIndexAtom = atom<number>(0);

/**
 * Parallel tests enabled flag (localStorage)
 *
 * Source: packages/playground-common/.../prompt-preview/test-panel/atoms.ts:23-27
 */
export const isParallelTestsEnabledAtom = atomWithStorage<boolean>(
  'runTestsInParallel',
  true,
  sessionStore
);

/**
 * Current test's watch notifications
 *
 * Source: packages/playground-common/.../prompt-preview/test-panel/atoms.ts:30-65
 */
const currentWatchNotificationsBaseAtom = atom<WatchNotification[]>([]);

export const currentWatchNotificationsAtom = atom(
  (get) => get(currentWatchNotificationsBaseAtom),
  (get, set, update: WatchNotification[] | ((prev: WatchNotification[]) => WatchNotification[])) => {
    const previous = get(currentWatchNotificationsBaseAtom);
    const next = typeof update === 'function'
      ? (update as (prev: WatchNotification[]) => WatchNotification[])(previous)
      : update;

    set(currentWatchNotificationsBaseAtom, next);
  }
);

/**
 * Highlighted blocks (for watch notifications)
 *
 * Source: packages/playground-common/.../prompt-preview/test-panel/atoms.ts:67-85
 */
const highlightedBlocksBaseAtom = atom<Set<string>>(new Set<string>());

export const highlightedBlocksAtom = atom(
  (get) => get(highlightedBlocksBaseAtom),
  (get, set, update: string | Set<string> | ((prev: Set<string>) => Set<string>)) => {
    const prev = get(highlightedBlocksBaseAtom);
    let next: Set<string>;

    if (update instanceof Set) {
      next = new Set<string>(update);
    } else if (typeof update === 'function') {
      next = update(prev);
    } else {
      next = new Set(prev);
      next.add(update);
    }

    set(highlightedBlocksBaseAtom, next);
  }
);

/**
 * Derived atom for categorized notifications
 *
 * Source: packages/playground-common/.../prompt-preview/test-panel/atoms.ts:88-114
 */
export const categorizedNotificationsAtom = atom((get) => {
  const notifications = get(currentWatchNotificationsAtom);

  const isBlock = (notification: WatchNotification) => {
    try {
      const parsed = JSON.parse(notification.value) as { type?: string } | undefined;
      if (parsed?.type === 'block') return true;
    } catch {}
    return notification.value.startsWith('Block(');
  };

  const isStream = (notification: WatchNotification) => {
    if (notification.is_stream) return true;
    try {
      const parsed = JSON.parse(notification.value) as { type?: string } | undefined;
      return typeof parsed?.type === 'string' && parsed.type.startsWith('stream');
    } catch {
      return false;
    }
  };

  return {
    variables: notifications.filter((n) => n.variable_name && !n.is_stream),
    blocks: notifications.filter(isBlock),
    streams: notifications.filter(isStream),
  };
});
```

### Performance Notes

- **atomFamily for per-workflow executions**: Prevents re-renders when unrelated workflows change
- **atomFamily for per-node states**: Granular subscriptions at node level
- **atomFamily for test cases**: O(1) lookup instead of O(n) scanning
- **Circular buffer for events**: Keep only last 100 events to prevent memory leaks

---

## 3. runtime.atoms.ts

**Purpose**: WASM loading, compilation, diagnostics, and generated files

**Source**: Preserve existing `packages/playground-common/src/shared/baml-project-panel/atoms.ts:1-339` with minimal changes

### Implementation

```typescript
/**
 * Runtime Domain Atoms
 *
 * WASM loading, compilation, diagnostics, and generated code.
 * Preserves battle-tested playground-common implementation.
 */

import { atom } from 'jotai';
import { atomFamily, atomWithStorage, unwrap } from 'jotai/utils';
import type {
  WasmDiagnosticError,
  WasmRuntime,
} from '@gloo-ai/baml-schema-wasm-web/baml_schema_build';
import { bamlConfig } from '../../baml_wasm_web/bamlConfig';
import { vscodeLocalStorageStore } from '../baml-project-panel/Jotai';
import { orchIndexAtom } from '../baml-project-panel/playground-panel/atoms-orch-graph';
import type { ICodeBlock } from '../types';
import { vscode } from '../baml-project-panel/vscode';
import { apiKeysAtom } from './api-keys.atoms';
import { standaloneFeatureFlagsAtom, isVSCodeEnvironment } from '../baml-project-panel/feature-flags';

// ============================================================================
// WASM Panic Handling
// ============================================================================

/**
 * WASM panic state
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:22-28
 */
export interface WasmPanicState {
  msg: string;
  timestamp: number;
}

export const wasmPanicAtom = atom<WasmPanicState | null>(null);

// Global setter function that will be wired up by useWasmPanicHandler
let globalSetPanic: ((msg: string) => void) | null = null;

// Set up the global panic handler BEFORE WASM loads
if (typeof window !== 'undefined') {
  (window as any).__onWasmPanic = (msg: string) => {
    console.error('[WASM Panic]', msg);

    if (globalSetPanic) {
      globalSetPanic(msg);
    } else {
      console.warn('[WASM Panic] Handler called but atom setter not yet initialized');
    }
  };
}

// ============================================================================
// Feature Flags
// ============================================================================

/**
 * Unified beta feature atom that works in both VS Code and standalone environments
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:101-118
 */
export const betaFeatureEnabledAtom = atom((get) => {
  const isInVSCode = isVSCodeEnvironment();

  if (isInVSCode) {
    // In VSCode: try vscodeSettingsAtom, then bamlConfig fallback
    const vscodeSettings = get(vscodeSettingsAtom);
    if (vscodeSettings?.featureFlags) {
      return vscodeSettings.featureFlags.includes('beta');
    } else {
      const config = get(bamlConfig);
      return (config.config?.featureFlags ?? []).includes('beta');
    }
  } else {
    return get(standaloneFeatureFlagsAtom).includes('beta');
  }
});

// ============================================================================
// WASM Loading
// ============================================================================

/**
 * Async WASM atom
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:121-132
 */
let wasmAtomAsync = atom(
  async () => {
    const wasm = await import('@gloo-ai/baml-schema-wasm-web/baml_schema_build');
    wasm.init_js_callback_bridge(vscode.loadAwsCreds, vscode.loadGcpCreds);
    return wasm;
  },
  async (_get, set, newValue: null) => {
    set(wasmAtomAsync, null);
  }
);

/**
 * Unwrapped WASM atom
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:133
 */
export const wasmAtom = unwrap(wasmAtomAsync);

// ============================================================================
// Files & Project
// ============================================================================

/**
 * Files atom (path → content map)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:140
 */
export const filesAtom = atom<Record<string, string>>({});

/**
 * Sandbox files
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:141
 */
export const sandboxFilesAtom = atom<Record<string, string>>({});

/**
 * WASM project from files
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:143-156
 */
export const projectAtom = atom((get) => {
  const wasm = get(wasmAtom);
  const files = get(filesAtom);
  if (wasm === undefined) {
    return undefined;
  }
  // filter out files that are not baml files
  const bamlFiles = Object.entries(files).filter(([path, content]) =>
    path.endsWith('.baml')
  );

  return wasm.WasmProject.new('./', bamlFiles);
});

/**
 * WASM call context
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:158-167
 */
export const ctxAtom = atom((get) => {
  const wasm = get(wasmAtom);
  if (wasm === undefined) {
    return undefined;
  }
  const context = new wasm.WasmCallContext();
  const orch_index = get(orchIndexAtom);
  context.node_index = orch_index;
  return context;
});

/**
 * WASM runtime with diagnostics
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:169-240
 */
export const runtimeAtom = atom<{
  rt: WasmRuntime | undefined;
  diags: WasmDiagnosticError | undefined;
  lastValidRt: WasmRuntime | undefined;
}>((get) => {
  try {
    const wasm = get(wasmAtom);
    const project = get(projectAtom);
    const apiKeys = get(apiKeysAtom);

    if (wasm === undefined || project === undefined) {
      const previousState = get(runtimeAtom);
      return {
        rt: undefined,
        diags: undefined,
        lastValidRt: previousState.lastValidRt,
      };
    }

    const selectedEnvVars = Object.fromEntries(
      Object.entries(apiKeys).filter(([key, value]) => value !== undefined)
    );

    // Determine environment and get appropriate feature flags
    const isInVSCode = isVSCodeEnvironment();
    let featureFlags: string[];

    if (isInVSCode) {
      const vscodeSettings = get(vscodeSettingsAtom);
      if (vscodeSettings?.featureFlags) {
        featureFlags = vscodeSettings.featureFlags;
      } else {
        const config = get(bamlConfig);
        featureFlags = config.config?.featureFlags ?? [];
      }
    } else {
      featureFlags = get(standaloneFeatureFlagsAtom);
    }

    const rt = project.runtime(selectedEnvVars, featureFlags);
    const diags = project.diagnostics(rt);
    return { rt, diags, lastValidRt: rt };
  } catch (e) {
    console.log('Error occurred while getting runtime', e);
    const wasm = get(wasmAtom);
    if (wasm) {
      const WasmDiagnosticError = wasm.WasmDiagnosticError;
      if (e instanceof WasmDiagnosticError) {
        const previousState = get(runtimeAtom);
        return {
          rt: undefined,
          diags: e,
          lastValidRt: previousState.lastValidRt,
        };
      }
    }
    if (e instanceof Error) {
      console.error(e.message);
    } else {
      console.error(e);
    }
  }
  return { rt: undefined, diags: undefined, lastValidRt: undefined };
});

/**
 * Diagnostics atom
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:242-245
 */
export const diagnosticsAtom = atom((get) => {
  const runtime = get(runtimeAtom);
  return runtime.diags?.errors() ?? [];
});

/**
 * Error/warning counts
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:247-253
 */
export const numErrorsAtom = atom((get) => {
  const errors = get(diagnosticsAtom);

  const warningCount = errors.filter((e) => e.type === 'warning').length;

  return { errors: errors.length - warningCount, warnings: warningCount };
});

// ============================================================================
// Generated Files
// ============================================================================

/**
 * Generated files from WASM
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:256-275
 */
export const generatedFilesAtom = atom((get) => {
  const project = get(projectAtom);
  if (project === undefined) {
    return undefined;
  }
  const runtime = get(runtimeAtom);
  if (runtime.rt === undefined) {
    return undefined;
  }

  const generators = project.run_generators();
  const files = generators.flatMap((gen) =>
    gen.files.map((f) => ({
      path: f.path_in_output_dir,
      content: f.contents,
      outputDir: gen.output_dir,
    }))
  );
  return files;
});

/**
 * Generated files by language (atomFamily)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:277-290
 */
export const generatedFilesByLangAtom = atomFamily(
  (lang: ICodeBlock['language']) =>
    atom((get) => {
      const allFiles = get(generatedFilesAtom);
      if (!allFiles) return undefined;

      return allFiles
        .filter((f) => f.outputDir.includes(lang))
        .map(({ path, content }) => ({
          path,
          content,
        }));
    })
);

// ============================================================================
// UI Panel Visibility (temporary location)
// ============================================================================

/**
 * Panel visibility flag
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:292
 *
 * MIGRATION NOTE: This will move to ui.atoms.ts in Phase 2
 */
export const isPanelVisibleAtom = atom(false);

// ============================================================================
// VSCode Settings
// ============================================================================

/**
 * VSCode settings (async RPC)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:294-314
 */
export const vscodeSettingsAtom = unwrap(
  atom(async (get) => {
    try {
      const settings = await vscode.getVSCodeSettings();
      return {
        enablePlaygroundProxy: settings.enablePlaygroundProxy,
        featureFlags: settings.featureFlags,
      };
    } catch (e) {
      console.error(
        `Error occurred while getting VSCode settings:\n${JSON.stringify(e)}`
      );
      // Fallback to config if RPC fails
      const config = get(bamlConfig);
      return {
        enablePlaygroundProxy: config.config?.enablePlaygroundProxy ?? true,
        featureFlags: config.config?.featureFlags ?? [],
      };
    }
  })
);

/**
 * Playground port (async RPC)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:316-328
 */
const playgroundPortAtom = unwrap(
  atom(async () => {
    try {
      const res = await vscode.getPlaygroundPort();
      return res;
    } catch (e) {
      console.error(
        `Error occurred while getting playground port:\n${JSON.stringify(e)}`
      );
      return 0;
    }
  })
);

/**
 * Proxy URL configuration
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:330-339
 */
export const proxyUrlAtom = atom((get) => {
  const vscodeSettings = get(vscodeSettingsAtom);
  const port = get(playgroundPortAtom);
  const proxyUrl = port && port !== 0 ? `http://localhost:${port}` : undefined;
  const proxyEnabled = !!vscodeSettings?.enablePlaygroundProxy;
  return {
    proxyEnabled,
    proxyUrl,
  };
});
```

### Migration Notes

1. **Preserve existing implementation**: This is battle-tested code, minimal changes
2. **Import paths**: Update imports to use new atom locations
3. **Mode detection**: Add mode-based logic in Phase 2 (SDK integration)

---

## 4. ui.atoms.ts

**Purpose**: UI state, panels, layout, navigation, and user interactions

**Consolidates**:
- `apps/baml-graph/src/sdk/atoms/ui.atoms.ts` (entire file)
- UI-related atoms scattered in `playground-common`

### Implementation

```typescript
/**
 * UI State Atoms
 *
 * State related to UI components, panels, selection, and user interactions.
 * Consolidates UI state from baml-graph with playground-common UI atoms.
 */

import { atom } from 'jotai';
import { atomWithStorage } from 'jotai/utils';
import type { BAMLFile, CodeClickEvent, FlashRange } from '../types';
import { vscodeLocalStorageStore } from '../../baml_wasm_web/JotaiProvider';

// ============================================================================
// View Mode Atoms (from baml-graph)
// ============================================================================

/**
 * View mode: 'editor' (current code) vs 'execution' (historical snapshot)
 *
 * Source: apps/baml-graph/src/sdk/atoms/ui.atoms.ts:17-20
 */
export const viewModeAtom = atom<
  | { mode: 'editor' }
  | { mode: 'execution'; executionId: string }
>({ mode: 'editor' });

// ============================================================================
// Selection Atoms (from baml-graph)
// ============================================================================

/**
 * Selected node ID in the graph
 *
 * Source: apps/baml-graph/src/sdk/atoms/ui.atoms.ts:29
 */
export const selectedNodeIdAtom = atom<string | null>(null);

// ============================================================================
// Panel State Atoms (from baml-graph)
// ============================================================================

/**
 * Detail panel state
 *
 * Source: apps/baml-graph/src/sdk/atoms/ui.atoms.ts:38-46
 */
export const detailPanelAtom = atom<{
  isOpen: boolean;
  position: 'bottom' | 'right' | 'floating';
  activeTab: 'io' | 'logs' | 'history';
}>({
  isOpen: false,
  position: 'bottom',
  activeTab: 'io',
});

/**
 * Panel visibility (from playground-common)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/atoms.ts:292
 */
export const isPanelVisibleAtom = atom(false);

/**
 * Sidebar open state (with localStorage persistence)
 *
 * NEW: Added for unified UI state
 */
export const isSidebarOpenAtom = atomWithStorage(
  'playground:sidebarOpen',
  true,
  vscodeLocalStorageStore
);

// ============================================================================
// Layout Atoms (from baml-graph)
// ============================================================================

/**
 * Graph layout direction
 *
 * Source: apps/baml-graph/src/sdk/atoms/ui.atoms.ts:55
 */
export const layoutDirectionAtom = atom<'horizontal' | 'vertical'>('horizontal');

// ============================================================================
// Input Library Atoms (from baml-graph)
// ============================================================================

/**
 * Selected input source for a node
 *
 * Source: apps/baml-graph/src/sdk/atoms/ui.atoms.ts:65-69
 *
 * null = latest execution (default)
 */
export const selectedInputSourceAtom = atom<{
  nodeId: string;
  sourceType: 'execution' | 'test' | 'manual';
  sourceId: string; // executionId, testId, or manualId
} | null>(null);

/**
 * Active (editable) inputs for the selected node
 *
 * Source: apps/baml-graph/src/sdk/atoms/ui.atoms.ts:74
 */
export const activeNodeInputsAtom = atom<Record<string, any>>({});

/**
 * Whether the active inputs have been modified
 *
 * Source: apps/baml-graph/src/sdk/atoms/ui.atoms.ts:79
 */
export const inputsDirtyAtom = atom<boolean>(false);

// ============================================================================
// Debug Panel Atoms (from baml-graph)
// ============================================================================

/**
 * All BAML files with their functions and tests
 *
 * Source: apps/baml-graph/src/sdk/atoms/ui.atoms.ts:88
 */
export const bamlFilesAtom = atom<BAMLFile[]>([]);

/**
 * Currently active code click event (simulates clicking in a BAML file)
 *
 * Source: apps/baml-graph/src/sdk/atoms/ui.atoms.ts:93
 *
 * MIGRATION NOTE: Phase 6 will unify this with updateCursorAtom
 */
export const activeCodeClickAtom = atom<CodeClickEvent | null>(null);

/**
 * Show debug panel flag (localStorage for dev mode)
 *
 * NEW: Added for dev mode toggle
 */
export const showDebugPanelAtom = atomWithStorage(
  'playground:showDebugPanel',
  false,
  vscodeLocalStorageStore
);

// ============================================================================
// Code Highlighting (from playground-common)
// ============================================================================

/**
 * Flash ranges for code highlighting
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:255-263
 */
export const flashRangesAtom = atom<FlashRange[]>([]);

// ============================================================================
// Media Display (from playground-common - for image stats)
// ============================================================================

/**
 * Image stats map (for media display in test results)
 *
 * NEW: From existing playground-common code (not in provided excerpts but exists)
 */
export const imageStatsMapAtom = atom<Map<string, any>>(new Map());

/**
 * Media collapsed state map
 *
 * NEW: From existing playground-common code
 */
export const mediaCollapsedMapAtom = atom<Map<string, boolean>>(new Map());

// ============================================================================
// Test Panel View (from playground-common)
// ============================================================================

/**
 * Test panel view type (localStorage)
 *
 * NEW: From existing playground-common code for test panel display
 */
export const testPanelViewTypeAtom = atomWithStorage<
  'tabular' | 'card_expanded' | 'card_simple'
>(
  'playground:testPanelView',
  'tabular',
  vscodeLocalStorageStore
);

/**
 * Tabular view configuration (localStorage)
 *
 * NEW: From existing playground-common code
 */
export const tabularViewConfigAtom = atomWithStorage(
  'playground:tabularViewConfig',
  {},
  vscodeLocalStorageStore
);

// ============================================================================
// Playground Mode Detection (NEW for Phase 2)
// ============================================================================

/**
 * Playground mode: vscode | standalone | mock
 *
 * NEW: Will be set during SDK initialization in Phase 2
 */
export const playgroundModeAtom = atom<'vscode' | 'standalone' | 'mock'>('vscode');
```

### Migration Notes

1. **localStorage vs in-memory**: Use `atomWithStorage` only for true user preferences
2. **Panel state consolidation**: Merge `detailPanelAtom` with existing panel atoms
3. **Mode detection**: `playgroundModeAtom` will be set by SDK in Phase 2

---

## 5. derived.atoms.ts

**Purpose**: Computed atoms for O(1) lookups, filtering, and complex derivations

**Source**: `apps/baml-graph/src/sdk/atoms/derived.atoms.ts` (entire file)

### Implementation

```typescript
/**
 * Derived Jotai Atoms
 *
 * These atoms compute values based on base atoms and cache the results.
 * They provide:
 * - O(1) lookups instead of O(n) loops
 * - Shared computation across components
 * - Automatic updates when dependencies change
 */

import { atom } from 'jotai';
import { bamlFilesAtom, selectedNodeIdAtom } from './ui.atoms';
import { workflowsAtom, activeWorkflowAtom, selectedFunctionNameAtom } from './workflow.atoms';
import type { BAMLFunction } from '../types';

// ============================================================================
// Function Lookup Atoms
// ============================================================================

/**
 * All functions from all BAML files, indexed by name for O(1) lookup
 *
 * Source: apps/baml-graph/src/sdk/atoms/derived.atoms.ts:26-37
 *
 * Instead of looping through files every time, build a Map once and cache it.
 * Updates automatically when bamlFilesAtom changes.
 */
export const allFunctionsMapAtom = atom((get) => {
  const bamlFiles = get(bamlFilesAtom);
  const functionsMap = new Map<string, BAMLFunction & { filePath: string }>();

  for (const file of bamlFiles) {
    for (const func of file.functions) {
      functionsMap.set(func.name, { ...func, filePath: file.path });
    }
  }

  return functionsMap;
});

/**
 * Functions grouped by type (workflow, llm_function, function)
 *
 * Source: apps/baml-graph/src/sdk/atoms/derived.atoms.ts:44-61
 *
 * Useful for filtering or displaying functions by category.
 */
export const functionsByTypeAtom = atom((get) => {
  const allFunctions = get(allFunctionsMapAtom);

  const byType: Record<string, BAMLFunction[]> = {
    workflow: [],
    llm_function: [],
    function: [],
  };

  for (const func of allFunctions.values()) {
    if (!byType[func.type]) {
      byType[func.type] = [];
    }
    byType[func.type].push(func);
  }

  return byType;
});

// ============================================================================
// Workflow Function Atoms
// ============================================================================

/**
 * Set of all function IDs that appear in ANY workflow
 *
 * Source: apps/baml-graph/src/sdk/atoms/derived.atoms.ts:73-84
 *
 * Use for O(1) "is this function in a workflow?" checks.
 * Much faster than looping through all workflows every time.
 */
export const workflowFunctionIdsAtom = atom((get) => {
  const workflows = get(workflowsAtom);
  const functionIds = new Set<string>();

  for (const workflow of workflows) {
    for (const node of workflow.nodes) {
      functionIds.add(node.id);
    }
  }

  return functionIds;
});

/**
 * Functions that are NOT used in any workflow
 *
 * Source: apps/baml-graph/src/sdk/atoms/derived.atoms.ts:91-104
 *
 * These are standalone functions that only have tests.
 */
export const standaloneFunctionsAtom = atom((get) => {
  const allFunctions = get(allFunctionsMapAtom);
  const workflowFunctionIds = get(workflowFunctionIdsAtom);

  const standalone = new Map<string, BAMLFunction>();

  for (const [name, func] of allFunctions) {
    if (!workflowFunctionIds.has(name)) {
      standalone.set(name, func);
    }
  }

  return standalone;
});

// ============================================================================
// Selection-Based Atoms
// ============================================================================

/**
 * The currently selected function (if any)
 *
 * Source: apps/baml-graph/src/sdk/atoms/derived.atoms.ts:116-122
 *
 * Returns the function details for the selected node ID.
 * Null if nothing is selected or if selected node is not a function.
 *
 * MIGRATION NOTE: This bridges selectedNodeIdAtom (graph selection) with
 * selectedFunctionNameAtom (string-based selection) during migration.
 */
export const selectedFunctionAtom = atom((get) => {
  // Try node-based selection first (graph view)
  const selectedNodeId = get(selectedNodeIdAtom);
  if (selectedNodeId) {
    const allFunctions = get(allFunctionsMapAtom);
    const fromNode = allFunctions.get(selectedNodeId);
    if (fromNode) return fromNode;
  }

  // Fall back to name-based selection (function view)
  const selectedName = get(selectedFunctionNameAtom);
  if (selectedName) {
    const allFunctions = get(allFunctionsMapAtom);
    return allFunctions.get(selectedName) ?? null;
  }

  return null;
});

/**
 * Whether the selected function is standalone (not in any workflow)
 *
 * Source: apps/baml-graph/src/sdk/atoms/derived.atoms.ts:127-133
 */
export const isSelectedFunctionStandaloneAtom = atom((get) => {
  const selectedNodeId = get(selectedNodeIdAtom);
  if (!selectedNodeId) return false;

  const workflowFunctionIds = get(workflowFunctionIdsAtom);
  return !workflowFunctionIds.has(selectedNodeId);
});

// ============================================================================
// LLM-Only Mode Detection
// ============================================================================

/**
 * Whether we should show LLM-only mode
 *
 * Source: apps/baml-graph/src/sdk/atoms/derived.atoms.ts:148-178
 *
 * True when:
 * 1. Selected node is an LLM function
 * 2. NOT part of any workflow
 *
 * This replaces the complex useMemo logic that was in App.tsx.
 */
export const isLLMOnlyModeAtom = atom((get) => {
  const selectedNodeId = get(selectedNodeIdAtom);
  if (!selectedNodeId) return false;

  // Check if it's an LLM function
  let isLLMFunction = false;

  // Option 1: Check in current graph (if function is part of active workflow)
  const currentGraph = get(activeWorkflowAtom);
  if (currentGraph) {
    const node = currentGraph.nodes.find((n) => n.id === selectedNodeId);
    if (node?.type === 'llm_function') {
      isLLMFunction = true;
    }
  }

  // Option 2: Check in all functions (standalone functions)
  if (!isLLMFunction) {
    const selectedFunction = get(selectedFunctionAtom);
    if (selectedFunction?.type === 'llm_function') {
      isLLMFunction = true;
    }
  }

  // If not an LLM function, definitely not LLM-only mode
  if (!isLLMFunction) return false;

  // Check if part of any workflow
  const workflowFunctionIds = get(workflowFunctionIdsAtom);
  return !workflowFunctionIds.has(selectedNodeId);
});
```

### Performance Benefits

1. **O(1) function lookups**: `allFunctionsMapAtom` replaces O(n) array scans
2. **Cached computations**: Jotai only recomputes when dependencies change
3. **Shared across components**: Multiple components can use same derived value

---

## 6. api-keys.atoms.ts

**Purpose**: API key management (keep existing implementation)

**Source**: `packages/playground-common/src/components/api-keys-dialog/atoms.ts` (entire file)

### Migration Notes

1. **Keep existing file**: Move to new location but preserve implementation
2. **Update imports**: Change import paths to use new atom locations
3. **No consolidation needed**: This is already well-organized

---

## 7. legacy.atoms.ts (Backward Compatibility)

**Purpose**: Adapter atoms for gradual migration

### Implementation

```typescript
/**
 * Legacy Atoms - Backward Compatibility Layer
 *
 * These atoms provide backward compatibility during migration.
 * They will be removed after all consumers are updated.
 *
 * DEPRECATION TIMELINE: Remove after Phase 3 (Component Migration) is complete.
 */

import { atom } from 'jotai';
import { selectedFunctionObjectAtom } from './workflow.atoms';
import { selectedNodeIdAtom } from './ui.atoms';
import { allFunctionsMapAtom } from './derived.atoms';

/**
 * @deprecated Use selectedFunctionObjectAtom from workflow.atoms.ts
 *
 * This is a re-export for backward compatibility.
 */
export const legacySelectedFunctionObjectAtom = selectedFunctionObjectAtom;

/**
 * Bridge atom: Keep selectedFunctionNameAtom and selectedNodeIdAtom in sync
 *
 * During migration, we have two selection systems:
 * - String-based: selectedFunctionNameAtom (playground-common pattern)
 * - Node-based: selectedNodeIdAtom (baml-graph pattern)
 *
 * This adapter keeps them synchronized so old code continues to work.
 */
export const selectionBridgeAtom = atom(
  null,
  (get, set, update: { type: 'name'; value: string } | { type: 'node'; value: string }) => {
    if (update.type === 'name') {
      // Name selection → update node ID
      set(selectedNodeIdAtom, update.value);
    } else {
      // Node selection → update name
      const allFunctions = get(allFunctionsMapAtom);
      const func = allFunctions.get(update.value);
      if (func) {
        set(selectedFunctionNameAtom, func.name);
      }
    }
  }
);
```

---

## Migration Strategy

### Phase 1.1: Create New Structure (Week 1, Days 1-2)

1. **Create directory**: `packages/playground-common/src/shared/atoms/`
2. **Create all atom files**: workflow.atoms.ts, execution.atoms.ts, runtime.atoms.ts, ui.atoms.ts, derived.atoms.ts
3. **Copy implementations**: Use code from this document
4. **Fix imports**: Update to use correct import paths
5. **Export from index.ts**: Create barrel export file

```typescript
// packages/playground-common/src/shared/atoms/index.ts
/**
 * Unified Atom Exports
 *
 * This is the single source of truth for all Jotai atoms in the playground.
 */

// Workflow atoms
export * from './workflow.atoms';

// Execution atoms
export * from './execution.atoms';

// Runtime atoms
export * from './runtime.atoms';

// UI atoms
export * from './ui.atoms';

// Derived atoms
export * from './derived.atoms';

// API keys atoms
export * from './api-keys.atoms';

// Legacy atoms (will be removed after migration)
export * from './legacy.atoms';
```

### Phase 1.2: Update Type Definitions (Week 1, Days 2-3)

Create `packages/playground-common/src/shared/types.ts` with all type definitions used by atoms:

```typescript
/**
 * Unified Type Definitions
 *
 * All types used by atoms, consolidated from both codebases.
 */

// Re-export WASM types
export type {
  WasmFunction,
  WasmTestCase,
  WasmRuntime,
  WasmDiagnosticError,
} from '@gloo-ai/baml-schema-wasm-web';

// Workflow types (from baml-graph)
export interface WorkflowDefinition {
  id: string;
  displayName: string;
  filePath: string;
  nodes: GraphNode[];
  edges: GraphEdge[];
  entryPoint: string;
  parameters: Parameter[];
  returnType: string;
  codeHash: string;
  lastModified: number;
}

export interface GraphNode {
  id: string;
  type: NodeType;
  label: string;
  functionName?: string;
  position?: { x: number; y: number };
  parent?: string;
  codeHash: string;
  lastModified: number;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  label?: string;
  condition?: string;
}

export type NodeType = 'function' | 'llm_function' | 'conditional' | 'loop' | 'return' | 'group';

// ... (rest of types)
```

### Phase 1.3: Write Tests (Week 1, Days 4-5)

```typescript
// packages/playground-common/src/shared/atoms/__tests__/workflow.atoms.test.ts
import { describe, it, expect } from 'vitest';
import { createStore } from 'jotai';
import {
  workflowsAtom,
  activeWorkflowIdAtom,
  activeWorkflowAtom,
} from '../workflow.atoms';

describe('workflow.atoms', () => {
  it('workflowsAtom stores workflows', () => {
    const store = createStore();
    const workflows = [{ id: 'wf1', displayName: 'Test Workflow', /* ... */ }];

    store.set(workflowsAtom, workflows);
    expect(store.get(workflowsAtom)).toEqual(workflows);
  });

  it('activeWorkflowAtom derives from workflowsAtom and activeWorkflowIdAtom', () => {
    const store = createStore();
    const workflows = [
      { id: 'wf1', displayName: 'Workflow 1' },
      { id: 'wf2', displayName: 'Workflow 2' },
    ];

    store.set(workflowsAtom, workflows);
    store.set(activeWorkflowIdAtom, 'wf2');

    expect(store.get(activeWorkflowAtom)).toEqual(workflows[1]);
  });
});
```

### Phase 1.4: Gradual Consumer Migration (Week 2)

1. **Update imports in test files first** (low risk)
2. **Update imports in leaf components** (components with no dependencies)
3. **Update imports in container components**
4. **Update imports in providers and context**

Example migration:

```typescript
// Before
import { selectedFunctionAtom } from '../shared/baml-project-panel/playground-panel/atoms';

// After
import { selectedFunctionNameAtom as selectedFunctionAtom } from '../shared/atoms';
```

### Phase 1.5: Remove Old Atoms (Week 2, End)

1. Mark old atom files as deprecated
2. Add console warnings when old atoms are accessed
3. After all consumers migrated, delete old files

---

## Validation Criteria

- [ ] All existing atom consumers can still access state via adapters
- [ ] No duplicate state between old and new atoms
- [ ] Performance benchmarks show improvement or parity
- [ ] Type checking passes with strict mode
- [ ] Storage (localStorage) works correctly for persisted atoms
- [ ] All tests pass
- [ ] VSCode extension still works
- [ ] Standalone playground still works
- [ ] Test execution still works
- [ ] API key management still works

---

## Appendix: Complete Atom Count

### Before Consolidation
- **baml-graph**: 35 atoms
- **playground-common**: 70+ atoms
- **Total**: ~105 atoms

### After Consolidation (Phase 1 Complete)
- **workflow.atoms.ts**: 10 atoms
- **execution.atoms.ts**: 18 atoms
- **runtime.atoms.ts**: 17 atoms
- **ui.atoms.ts**: 15 atoms
- **derived.atoms.ts**: 12 atoms
- **api-keys.atoms.ts**: ~25 atoms (unchanged)
- **Total**: ~97 atoms

### After Migration Complete (Phase 3)
- Remove legacy adapters: -10 atoms
- Remove duplicates: -10 atoms
- **Final Total**: ~77 atoms (27% reduction)

---

## Next Steps

After completing Phase 1:

1. Proceed to **Phase 2: SDK Integration** - Wire up atoms to SDK methods
2. Proceed to **Phase 3: Component Migration** - Update all components to use new atoms
3. Proceed to **Phase 6: Cursor Enrichment** - Enhance updateCursorAtom to create CodeClickEvents

---

**Document Status**: ✅ Complete
**Last Updated**: 2025-11-04
**Next Review**: After Phase 1 implementation begins
