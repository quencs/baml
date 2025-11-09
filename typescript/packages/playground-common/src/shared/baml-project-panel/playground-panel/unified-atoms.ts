/**
 * Unified State Atoms
 *
 * This file contains the unified state atoms that merge the state management
 * between the original PromptPreview app and the WorkflowApp.
 */

import { atom } from 'jotai';
import { selectionAtom as originalSelectionAtom } from './atoms';
import {
  selectedFunctionNameAtom,
  selectedTestCaseNameAtom,
} from '../../../sdk/atoms/core.atoms';

// ============================================================================
// CORE UNIFIED STATE
// ============================================================================

/**
 * Unified selection state - single source of truth for all selection
 */
export interface UnifiedSelection {
  // Function-level selection (for the toolbar, running tests)
  functionName: string | null;
  testName: string | null;

  // Workflow context (null if viewing standalone function)
  activeWorkflowId: string | null;

  // Graph node selection (null if not in graph view or no node selected)
  selectedNodeId: string | null;
}

// Internal state atom
const unifiedSelectionStateAtom = atom<UnifiedSelection>({
  functionName: null,
  testName: null,
  activeWorkflowId: null,
  selectedNodeId: null,
});

/**
 * Unified selection atom - syncs with SDK atoms
 * When you update this atom, it also updates the SDK selectedFunctionNameAtom and selectedTestCaseNameAtom
 */
export const unifiedSelectionAtom = atom(
  (get) => get(unifiedSelectionStateAtom),
  (get, set, update: UnifiedSelection | ((prev: UnifiedSelection) => UnifiedSelection)) => {
    const newValue = typeof update === 'function' ? update(get(unifiedSelectionStateAtom)) : update;

    console.log('📝 Unified Selection Updated:', newValue);

    // Update internal state
    set(unifiedSelectionStateAtom, newValue);

    // Sync to SDK atoms
    set(selectedFunctionNameAtom, newValue.functionName);
    set(selectedTestCaseNameAtom, newValue.testName);
  }
);

/**
 * Active tab state
 */
export type TabValue = 'preview' | 'curl' | 'graph';
export const activeTabAtom = atom<TabValue>('preview');

/**
 * Detail panel state (for graph view)
 */
export const detailPanelStateAtom = atom({
  isOpen: false,
});

// ============================================================================
// DERIVED STATE
// ============================================================================

/**
 * View mode - determines what UI to show based on current selection
 */
export const viewModeAtom = atom((get) => {
  const selection = get(unifiedSelectionAtom);
  const { selectedFn } = get(originalSelectionAtom);

  // Is the selected function part of a workflow?
  const isInWorkflow = selection.activeWorkflowId !== null;

  // Is the selected function an LLM function?
  const isLLMFunction = selectedFn?.type === 'llm_function';

  return {
    showTabs: isLLMFunction,  // Only LLM functions get tabs
    showGraphTab: isInWorkflow,  // Only show Graph tab if in workflow
    defaultTab: (isInWorkflow ? 'graph' : 'preview') as TabValue,  // Smart default
    showTabBar: isLLMFunction || isInWorkflow,  // Hide tab bar only for non-LLM standalone
  };
});

/**
 * Bottom panel mode - determines whether to show TestPanel or DetailPanel
 */
export type BottomPanelMode = 'test-panel' | 'detail-panel';
export const bottomPanelModeAtom = atom<BottomPanelMode>((get) => {
  const activeTab = get(activeTabAtom);
  const selection = get(unifiedSelectionAtom);

  // Show DetailPanel when:
  // - On Graph tab, OR
  // - A graph node is selected (even if on other tabs)
  if (activeTab === 'graph' || selection.selectedNodeId !== null) {
    return 'detail-panel';
  }

  // Show TestPanel for Preview/cURL tabs
  return 'test-panel';
});

/**
 * Helper atom to determine if we should show the graph view
 */
export const shouldShowGraphAtom = atom((get) => {
  const selection = get(unifiedSelectionAtom);
  const activeTab = get(activeTabAtom);

  // Show graph if we're on the graph tab or if we're in a workflow
  return activeTab === 'graph' && selection.activeWorkflowId !== null;
});
