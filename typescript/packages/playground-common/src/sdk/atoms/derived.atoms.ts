/**
 * Derived Jotai Atoms for BAML SDK
 *
 * These atoms compute values based on base atoms and cache the results.
 * They provide:
 * - O(1) lookups instead of O(n) loops
 * - Shared computation across components
 * - Automatic updates when dependencies change
 */

import { atom } from 'jotai';
import { bamlFilesAtom, selectedNodeIdAtom } from './ui.atoms';
import { workflowsAtom, activeWorkflowAtom } from './workflow.atoms';
import type { BAMLFunction } from '../types';

// ============================================================================
// Function Lookup Atoms
// ============================================================================

/**
 * All functions from all BAML files, indexed by name for O(1) lookup
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
    byType[func.type]?.push(func);
  }

  return byType;
});

// ============================================================================
// Workflow Function Atoms
// ============================================================================

/**
 * Set of all function IDs that appear in ANY workflow
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
 * Returns the function details for the selected node ID.
 * Null if nothing is selected or if selected node is not a function.
 */
export const selectedFunctionAtom = atom((get) => {
  const selectedNodeId = get(selectedNodeIdAtom);
  if (!selectedNodeId) return null;

  const allFunctions = get(allFunctionsMapAtom);
  return allFunctions.get(selectedNodeId) ?? null;
});

/**
 * Whether the selected function is standalone (not in any workflow)
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
    const node = currentGraph.nodes.find(n => n.id === selectedNodeId);
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
