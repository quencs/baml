/**
 * Playground Panel Atoms
 *
 * This file bridges between the old WASM-based runtime and the new SDK.
 * It maintains backward compatibility while using SDK atoms where possible.
 */

import { type Atom, atom } from 'jotai';
import { filesAtom, runtimeAtom } from '../atoms';

// Related to test status
import type {
  WasmFunction,
  WasmFunctionResponse,
  WasmTestResponse,
} from '@gloo-ai/baml-schema-wasm-web';
import { atomFamily, atomWithStorage } from 'jotai/utils';

// ============================================================================
// SDK Atoms - Direct Re-exports
// ============================================================================

// Re-export SDK atoms directly without creating local copies
export {
  // Test execution state
  areTestsRunningAtom,
  currentAbortControllerAtom,
  flashRangesAtom,
  testHistoryAtom,
  selectedHistoryIndexAtom,
  selectedTestHistoryAtom,
  currentWatchNotificationsAtom,
  highlightedBlocksAtom,
  categorizedNotificationsAtom,

  // Types
  type TestState,
  type TestHistoryEntry,
  type TestHistoryRun,
  type WatchNotification,
  type FlashRange,
  type CategorizedNotifications,
} from '../../../sdk/atoms/test.atoms';

// Import for internal use
import {
  selectedTestHistoryAtom,
} from '../../../sdk/atoms/test.atoms';

// Re-export selection atoms from SDK
import {
  selectedFunctionNameAtom,
  selectedTestCaseNameAtom,
  updateSelectionAtom,
} from '../../../sdk/atoms/core.atoms';

// Import runtimeStateAtom for accessing WASM functions
import { runtimeStateAtom } from '../../atoms';

// ============================================================================
// Selection Atoms - Adapter to SDK atoms
// ============================================================================

/**
 * Adapter atoms to convert between SDK (null) and local (undefined) conventions
 * These write to SDK atoms to ensure consistent state
 */
export const selectedFunctionAtom = atom(
  (get) => get(selectedFunctionNameAtom) ?? undefined,
  (get, set, value: string | undefined) => {
    set(selectedFunctionNameAtom, value ?? null);
  }
);

export const selectedTestcaseAtom = atom(
  (get) => get(selectedTestCaseNameAtom) ?? undefined,
  (get, set, value: string | undefined) => {
    set(selectedTestCaseNameAtom, value ?? null);
  }
);

export const graphControlsTipDismissedAtom = atomWithStorage(
  'playground:graphControlsTipDismissed',
  false
);

// ============================================================================
// Derived Selection Atoms
// ============================================================================

export const selectedItemAtom = atom(
  (get) => {
    const selected = get(selectionAtom);
    if (
      selected.selectedFn === undefined ||
      selected.selectedTc === undefined
    ) {
      return undefined;
    }
    return [selected.selectedFn.name, selected.selectedTc.name] as [
      string,
      string,
    ];
  },
  (_, set, functionName: string, testcaseName: string | undefined) => {
    set(selectedFunctionAtom, functionName);
    set(selectedTestcaseAtom, testcaseName);
  },
);

// ============================================================================
// Function & Test Case Helpers
// ============================================================================

export const functionObjectAtom = atomFamily((functionName: string) =>
  atom((get) => {
    const { functions } = get(runtimeStateAtom);
    const fn = functions.find((f) => f.name === functionName);
    if (!fn) {
      return undefined;
    }
    return fn;
  }),
);

export const testcaseObjectAtom = atomFamily(
  (params: { functionName: string; testcaseName?: string | null }) =>
    atom((get) => {
      const { functions } = get(runtimeStateAtom);
      const fn = functions.find((f) => f.name === params.functionName);
      if (!fn) {
        return undefined;
      }
      const tc = fn.testCases?.find((tc) => tc.name === params.testcaseName);
      if (!tc) {
        return undefined;
      }
      return tc;
    }),
);

// ============================================================================
// Cursor Management
// ============================================================================

/**
 * Update cursor position - determines which function/test is at cursor and updates selection
 *
 * NOTE: This is a legacy atom for backward compatibility.
 * The logic has been abstracted into BamlRuntime.updateCursor() and SDK.cursor.update()
 * This atom now uses the runtime's method instead of calling WASM directly.
 */
export const updateCursorAtom = atom(
  null,
  (
    get,
    set,
    cursor: {
      fileName: string;
      line: number;
      column: number;
    },
  ) => {
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
      get(selectedFunctionAtom) ?? '',
      cursorIdx,
    );

    if (selectedFunc) {
      const selectedTestcase = runtime.get_testcase_from_position(
        selectedFunc,
        cursorIdx,
      );

      if (selectedTestcase) {
        // Check for nested function in test case
        const nestedFunc = runtime.get_function_of_testcase(
          fileName,
          cursorIdx,
        );

        // Use shared selection update logic
        set(updateSelectionAtom, {
          functionName: nestedFunc ? nestedFunc.name : selectedFunc.name,
          testCaseName: selectedTestcase.name,
        });
      } else {
        // Just a function, no test case
        set(updateSelectionAtom, {
          functionName: selectedFunc.name,
          testCaseName: null,
        });
      }
    }
  }
);

// ============================================================================
// Selection State
// ============================================================================

export const selectionAtom = atom((get) => {
  const selectedFunction = get(selectedFunctionAtom);
  const selectedTestcase = get(selectedTestcaseAtom);

  const { functions } = get(runtimeStateAtom);

  let selectedFn = functions.at(0);
  if (selectedFunction !== undefined) {
    const foundFn = functions.find((f) => f.name === selectedFunction);
    if (foundFn) {
      selectedFn = foundFn;
    } else {
      console.error('Function not found', selectedFunction);
    }
  } else {
    console.debug('No function selected');
  }

  let selectedTc = selectedFn?.testCases?.at(0);
  if (selectedTestcase !== undefined) {
    const foundTc = selectedFn?.testCases?.find(
      (tc) => tc.name === selectedTestcase,
    );
    if (foundTc) {
      selectedTc = foundTc;
    } else {
      console.error('Testcase not found', selectedTestcase);
      // Clear the invalid test selection from SDK atoms
      // This prevents the error from persisting
      selectedTc = undefined;
    }
  }

  return { selectedFn, selectedTc };
});

export const selectedFunctionObjectAtom = atom((get) => {
  const { selectedFn } = get(selectionAtom);
  return selectedFn;
});

// ============================================================================
// Test Status Types (for backward compatibility)
// ============================================================================

export type TestStatusType = 'queued' | 'running' | 'done' | 'error' | 'idle';
export type DoneTestStatusType =
  | 'passed'
  | 'llm_failed'
  | 'parse_failed'
  | 'constraints_failed'
  | 'assert_failed'
  | 'error';

// ============================================================================
// Test Case Helpers
// ============================================================================

export const testCaseAtom = atomFamily(
  (params: { functionName: string; testName: string }) =>
    atom((get) => {
      const { functions } = get(runtimeStateAtom);
      const fn = functions.find((f) => f.name === params.functionName);
      const tc = fn?.testCases?.find((tc) => tc.name === params.testName);
      if (!fn || !tc) {
        return undefined;
      }
      return { fn, tc };
    }),
);

export const functionTestSnippetAtom = atomFamily((functionName: string) =>
  atom((get) => {
    const { functions } = get(runtimeStateAtom);
    const fn = functions.find((f) => f.name === functionName);
    if (!fn) {
      return undefined;
    }
    return fn.testSnippet;
  }),
);

// ============================================================================
// Test Case Response (uses SDK test history)
// ============================================================================

/**
 * Get the test state for a specific function/test case from SDK test history
 * This replaces the old runningTestsAtom which was never set
 */
export const testCaseResponseAtom = atomFamily(
  (params: { functionName?: string; testName?: string }) =>
    atom((get) => {
      // Get the currently selected test history run (most recent)
      const historyRun = get(selectedTestHistoryAtom);
      if (!historyRun) {
        return undefined;
      }

      // Find the matching test in the history
      const testEntry = historyRun.tests.find(
        (t) =>
          t.functionName === params.functionName &&
          t.testName === params.testName
      );

      // Return the test state (response field contains the TestState)
      return testEntry?.response;
    }),
);

// ============================================================================
// UNIFIED STATE INTEGRATION
// ============================================================================

// Re-export unified atoms
export {
  unifiedSelectionAtom,
  activeTabAtom,
  detailPanelStateAtom,
  viewModeAtom,
  bottomPanelModeAtom,
  shouldShowGraphAtom,
  type UnifiedSelection,
  type TabValue,
  type BottomPanelMode,
} from './unified-atoms';
