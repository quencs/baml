/**
 * Central export for all Jotai atoms
 *
 * Atoms are now organized by domain:
 * - workflow.atoms.ts: Workflow definitions and navigation
 * - execution.atoms.ts: Execution state, node states, events, caching
 * - ui.atoms.ts: UI state, panels, selection, user interactions
 * - derived.atoms.ts: Computed/derived atoms based on base atoms
 */

// Re-export all workflow atoms
export {
  workflowsAtom,
  activeWorkflowIdAtom,
  activeWorkflowAtom,
  recentWorkflowsAtom,
} from './workflow.atoms';

// Re-export all execution atoms
export {
  workflowExecutionsAtomFamily,
  selectedExecutionIdAtom,
  activeWorkflowExecutionsAtom,
  selectedExecutionAtom,
  selectExecutionAtom,
  latestExecutionAtom,
  nodeStateAtomFamily,
  allNodeStatesAtom,
  registerNodeAtom,
  clearAllNodeStatesAtom,
  nodeExecutionsAtom,
  eventStreamAtom,
  addEventAtom,
  cacheAtom,
  getCacheKey,
} from './execution.atoms';

// Re-export all UI atoms
export {
  viewModeAtom,
  selectedNodeIdAtom,
  detailPanelAtom,
  layoutDirectionAtom,
  selectedInputSourceAtom,
  activeNodeInputsAtom,
  inputsDirtyAtom,
  bamlFilesAtom,
  activeCodeClickAtom,
} from './ui.atoms';

// Re-export all derived atoms
export {
  allFunctionsMapAtom,
  functionsByTypeAtom,
  workflowFunctionIdsAtom,
  standaloneFunctionsAtom,
  selectedFunctionAtom,
  isSelectedFunctionStandaloneAtom,
  isLLMOnlyModeAtom,
} from './derived.atoms';
