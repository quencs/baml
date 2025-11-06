/**
 * Unified Atoms - Central Export
 *
 * This file exports all atoms from the unified atom structure.
 * Components should import from this file for consistency.
 *
 * Example:
 * ```typescript
 * import { workflowsAtom, activeWorkflowAtom, runtimeAtom } from '@/shared/atoms';
 * ```
 */

// ============================================================================
// Workflow Domain
// ============================================================================
export {
  workflowsAtom,
  activeWorkflowIdAtom,
  activeWorkflowAtom,
  recentWorkflowsAtom,
  runtimeStateAtom,
  selectedFunctionNameAtom,
  selectedTestcaseNameAtom,
  selectedItemAtom,
  functionObjectAtom,
  testcaseObjectAtom,
  updateCursorAtom,
  selectionAtom,
  selectedFunctionObjectAtom,
  // Backward-compatible aliases
  selectedFunctionAtom,
  selectedTestcaseAtom,
  type WorkflowDefinition,
} from './workflow.atoms';

// ============================================================================
// Execution Domain
// ============================================================================
export {
  workflowExecutionsAtomFamily,
  activeExecutionIdAtom,
  selectedExecutionIdAtom,
  selectedExecutionAtom,
  selectExecutionAtom,
  activeWorkflowExecutionsAtom,
  latestExecutionAtom,
  nodeStateAtomFamily,
  nodeExecutionAtomFamily,
  allNodeStatesAtom,
  registerNodeAtom,
  clearAllNodeStatesAtom,
  nodeExecutionsAtom,
  executionEventStreamAtom,
  addEventAtom,
  cacheEntriesAtomFamily,
  cacheAtom,
  getCacheKey,
  // Test execution atoms
  testCaseAtom,
  functionTestSnippetAtom,
  testCaseResponseAtom,
  areTestsRunningAtom,
  runningTestsAtom,
  currentAbortControllerAtom,
  type NodeExecutionState,
  type ExecutionStatus,
  type ExecutionSnapshot,
  type BAMLEvent,
  type CacheEntry,
  type TestStatusType,
  type DoneTestStatusType,
  type TestState,
  type WatchNotification,
} from './execution.atoms';

// ============================================================================
// Runtime Domain
// Re-exported from baml-project-panel/atoms.ts (single source of truth)
// ============================================================================
export {
  wasmPanicAtom,
  useWasmPanicHandler,
  useClearWasmPanic,
  betaFeatureEnabledAtom,
  wasmAtom,
  useWaitForWasm,
  filesAtom,
  sandboxFilesAtom,
  projectAtom,
  ctxAtom,
  runtimeAtom,
  diagnosticsAtom,
  numErrorsAtom,
  generatedFilesAtom,
  generatedFilesByLangAtom,
  isPanelVisibleAtom,
  vscodeSettingsAtom,
  proxyUrlAtom,
  type WasmPanicState,
} from '../baml-project-panel/atoms';

// ============================================================================
// UI Domain
// ============================================================================
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
  flashRangesAtom,
  type BAMLFile,
  type CodeClickEvent,
  type InputSource,
  type FlashRange,
} from './ui.atoms';

// ============================================================================
// Cursor Enrichment (Phase 6)
// ============================================================================
export {
  enrichCursorToCodeClick,
  calculateByteIndex,
  determineFunctionType,
  determineFunctionNodeType,
} from './cursor-enrichment';

// ============================================================================
// Derived Domain
// ============================================================================
export {
  allFunctionsMapAtom,
  functionsByTypeAtom,
  workflowFunctionIdsAtom,
  standaloneFunctionsAtom,
  selectedFunctionFromNodeAtom,
  isSelectedFunctionStandaloneAtom,
  isLLMOnlyModeAtom,
  type BAMLFunction,
} from './derived.atoms';
