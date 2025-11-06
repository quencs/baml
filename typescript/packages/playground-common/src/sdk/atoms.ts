/**
 * Jotai Atoms for BAML SDK State Management
 *
 * This file re-exports all atoms from the organized atoms/ directory.
 * Atoms are now split into domain-specific files for better organization:
 *
 * - atoms/workflow.atoms.ts: Workflow definitions and navigation
 * - atoms/execution.atoms.ts: Execution state, node states, events, caching
 * - atoms/ui.atoms.ts: UI state, panels, selection, user interactions
 * - atoms/derived.atoms.ts: Computed/derived atoms
 *
 * All imports from './sdk/atoms' will continue to work unchanged.
 */

// Re-export workflow atoms
export {
  workflowsAtom,
  activeWorkflowIdAtom,
  activeWorkflowAtom,
  recentWorkflowsAtom,
} from './atoms/workflow.atoms';

// Re-export execution atoms
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
} from './atoms/execution.atoms';

// Re-export UI atoms
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
} from './atoms/ui.atoms';

// Re-export derived atoms
export {
  allFunctionsMapAtom,
  functionsByTypeAtom,
  workflowFunctionIdsAtom,
  standaloneFunctionsAtom,
  selectedFunctionAtom,
  isSelectedFunctionStandaloneAtom,
  isLLMOnlyModeAtom,
} from './atoms/derived.atoms';
