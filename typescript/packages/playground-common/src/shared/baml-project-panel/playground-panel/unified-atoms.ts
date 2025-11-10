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
  activeWorkflowIdAtom,
  selectedNodeIdAtom,
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

/**
 * Unified selection atom - syncs with SDK atoms
 * When you update this atom, it also updates the SDK selectedFunctionNameAtom and selectedTestCaseNameAtom
 */
export const unifiedSelectionAtom = atom(
  (get): UnifiedSelection => ({
    functionName: get(selectedFunctionNameAtom),
    testName: get(selectedTestCaseNameAtom),
    activeWorkflowId: get(activeWorkflowIdAtom),
    selectedNodeId: get(selectedNodeIdAtom),
  }),
  (get, set, update: UnifiedSelection | ((prev: UnifiedSelection) => UnifiedSelection)) => {
    const current: UnifiedSelection = {
      functionName: get(selectedFunctionNameAtom),
      testName: get(selectedTestCaseNameAtom),
      activeWorkflowId: get(activeWorkflowIdAtom),
      selectedNodeId: get(selectedNodeIdAtom),
    };

    const next = typeof update === 'function' ? update(current) : update;
    const finalValue: UnifiedSelection = {
      ...current,
      ...next,
    };

    console.log('📝 Unified Selection Updated:', finalValue);

    set(selectedFunctionNameAtom, finalValue.functionName);
    set(selectedTestCaseNameAtom, finalValue.testName);
    set(activeWorkflowIdAtom, finalValue.activeWorkflowId);
    set(selectedNodeIdAtom, finalValue.selectedNodeId);
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

  const isLLMFunction = selectedFn?.type === 'llm_function';
  const isWorkflow = selectedFn?.type === 'workflow';
  const showGraph = isWorkflow || isInWorkflow;

  return {
    showTabs: isLLMFunction || showGraph,
    showGraphTab: showGraph,
    defaultTab: (showGraph ? 'graph' : 'preview') as TabValue,
    showTabBar: isLLMFunction || showGraph,
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
