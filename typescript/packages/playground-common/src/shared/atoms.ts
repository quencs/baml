/**
 * Shared atoms used across the application
 *
 * Re-exports commonly used atoms from the SDK for convenience.
 */

import { atom } from 'jotai';

// Selection state
export {
  selectionAtom,
  selectedFunctionObjectAtom,
  selectedFunctionNameAtom,
  selectedTestCaseNameAtom,
  selectedTestCaseAtom,
  functionTestSnippetAtom,
  runtimeAtom,
} from '../sdk/atoms/core.atoms';

// Test execution state
export {
  areTestsRunningAtom,
  testHistoryAtom,
  selectedHistoryIndexAtom,
  selectedTestHistoryAtom,
  currentWatchNotificationsAtom,
  highlightedBlocksAtom,
  flashRangesAtom,
  categorizedNotificationsAtom,
  currentAbortControllerAtom,
} from '../sdk/atoms/test.atoms';

// Re-export types
export type {
  TestState,
  TestHistoryEntry,
  TestHistoryRun,
  WatchNotification,
  FlashRange,
  CategorizedNotifications,
} from '../sdk/atoms/test.atoms';

// Import atoms for aliases
import {
  selectionAtom,
  selectedFunctionObjectAtom,
  selectedTestCaseAtom,
  runtimeAtom,
} from '../sdk/atoms/core.atoms';
import { selectedTestHistoryAtom } from '../sdk/atoms/test.atoms';

// ============================================================================
// Compatibility aliases for old code
// ============================================================================

/**
 * @deprecated Use selectionAtom instead
 */
export const selectedItemAtom = selectionAtom;

/**
 * @deprecated Use selectedFunctionObjectAtom instead
 */
export const functionObjectAtom = selectedFunctionObjectAtom;

/**
 * @deprecated Use selectedTestCaseAtom instead
 */
export const testcaseObjectAtom = selectedTestCaseAtom;

/**
 * @deprecated Use selectionAtom and check selectedTc instead
 */
export const testCaseAtom = selectedTestCaseAtom;

/**
 * Type alias for backward compatibility
 * @deprecated Use TestState['response_status'] instead
 */
export type DoneTestStatusType = 'passed' | 'llm_failed' | 'parse_failed' | 'constraints_failed' | 'assert_failed' | 'error';

/**
 * Derived atom for test case response (from test history)
 * @deprecated Access test state from testHistoryAtom instead
 */
export const testCaseResponseAtom = atom((get) => {
  const history = get(selectedTestHistoryAtom);
  if (!history || !history.tests.length) return null;
  return history.tests[0]?.response || null;
});

/**
 * @deprecated Use runtimeAtom instead
 */
export const runtimeStateAtom = runtimeAtom;

/**
 * Cursor update atom for CodeMirror navigation
 * @deprecated This should be handled through SDK navigation API
 */
export const updateCursorAtom = atom(null, (_get, _set, _update: any) => {
  console.debug('[updateCursorAtom] Cursor update (deprecated)');
  // This is a no-op for compatibility - actual cursor updates should use SDK navigation API
});
