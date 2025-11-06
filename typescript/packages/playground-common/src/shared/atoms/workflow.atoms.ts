/**
 * Workflow Domain Atoms
 *
 * State related to workflows, their definitions, and navigation.
 * Consolidates workflow management from baml-graph with function selection from playground-common.
 *
 * Source: apps/baml-graph/src/sdk/atoms/workflow.atoms.ts
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:34-177
 */

import { atom } from 'jotai';
import { atomFamily } from 'jotai/utils';
import { runtimeAtom, filesAtom } from '../baml-project-panel/atoms';
import { enrichCursorToCodeClick } from './cursor-enrichment';
import { activeCodeClickAtom } from './ui.atoms';

// Import types - will be defined in SDK types
export interface WorkflowDefinition {
  id: string;
  displayName: string;
  filePath: string;
  startLine: number;
  endLine: number;
  nodes: any[];
  edges: any[];
  entryPoint: string;
  parameters: any[];
  returnType: string;
  childFunctions: string[];
  lastModified: number;
  codeHash: string;
}

// ============================================================================
// Workflow Definition Atoms (from baml-graph)
// ============================================================================

/**
 * All available workflows (all functions in the codebase)
 * Source: apps/baml-graph/src/sdk/atoms/workflow.atoms.ts:17
 */
export const workflowsAtom = atom<WorkflowDefinition[]>([]);

/**
 * Currently selected/active workflow
 * Source: apps/baml-graph/src/sdk/atoms/workflow.atoms.ts:22
 */
export const activeWorkflowIdAtom = atom<string | null>(null);

/**
 * Derived atom for the active workflow
 * Source: apps/baml-graph/src/sdk/atoms/workflow.atoms.ts:27-31
 */
export const activeWorkflowAtom = atom((get) => {
  const workflows = get(workflowsAtom);
  const activeId = get(activeWorkflowIdAtom);
  return workflows.find((w) => w.id === activeId) ?? null;
});

/**
 * Recent workflows (for quick access)
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
// Function & Test Selection (from playground-common)
// ============================================================================

/**
 * Selected function name (writable primitive atom)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:34
 */
export const selectedFunctionNameAtom = atom<string | undefined>(undefined);

/**
 * Selected testcase name (writable primitive atom)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:35
 */
export const selectedTestcaseNameAtom = atom<string | undefined>(undefined);

/**
 * Combined selection as tuple [functionName, testName]
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:37-55
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
 * Cursor update handler (write-only atom)
 *
 * Phase 6: Enhanced with cursor-to-CodeClick enrichment
 *
 * This atom now:
 * 1. Enriches cursor position into CodeClickEvent using WASM introspection
 * 2. Emits CodeClickEvent via activeCodeClickAtom
 * 3. Updates backward-compatibility atoms (selectedFunctionNameAtom, selectedTestcaseNameAtom)
 *
 * Source: packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:84-139
 * Enhanced: graphs-project-docs/implementation/06-cursor-enrichment.md
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

    // Enrich cursor position into CodeClickEvent
    const codeClickEvent = enrichCursorToCodeClick(
      runtime,
      cursor.fileName,
      cursor.line,
      cursor.column,
      fileContent,
      get(selectedFunctionNameAtom)
    );

    // Emit CodeClickEvent
    set(activeCodeClickAtom, codeClickEvent);

    // Update backward-compatibility atoms based on enriched event
    if (codeClickEvent) {
      if (codeClickEvent.type === 'function') {
        set(selectedFunctionNameAtom, codeClickEvent.functionName);
        set(selectedTestcaseNameAtom, undefined);
      } else if (codeClickEvent.type === 'test') {
        set(selectedFunctionNameAtom, codeClickEvent.functionName);
        set(selectedTestcaseNameAtom, codeClickEvent.testName);
      }
    } else {
      // Cursor is not on a semantic element, clear selection
      // (Optionally keep current selection - TBD based on UX preference)
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
// Backward Compatibility Exports
// ============================================================================

/**
 * Backward-compatible alias for selectedFunctionNameAtom
 * @deprecated Import selectedFunctionNameAtom directly
 */
export const selectedFunctionAtom = selectedFunctionNameAtom;

/**
 * Backward-compatible alias for selectedTestcaseNameAtom
 * @deprecated Import selectedTestcaseNameAtom directly
 */
export const selectedTestcaseAtom = selectedTestcaseNameAtom;
