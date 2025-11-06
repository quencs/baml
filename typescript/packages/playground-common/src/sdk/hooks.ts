/**
 * React Hooks for BAML SDK
 *
 * Convenient hooks for React components to access SDK state and functionality
 */

import { useAtom, useAtomValue, useSetAtom } from 'jotai';
import { useCallback, useMemo } from 'react';

// Re-export useBAMLSDK from provider
export { useBAMLSDK } from './provider';
import {
  workflowsAtom,
  activeWorkflowIdAtom,
  activeWorkflowAtom,
  activeWorkflowExecutionsAtom,
  selectedExecutionIdAtom,
  selectedExecutionAtom,
  latestExecutionAtom,
  nodeExecutionsAtom,
  nodeStateAtomFamily,
  allNodeStatesAtom,
  viewModeAtom,
  selectedNodeIdAtom,
  detailPanelAtom,
  layoutDirectionAtom,
  eventStreamAtom,
  selectedInputSourceAtom,
  activeNodeInputsAtom,
  inputsDirtyAtom,
  allFunctionsMapAtom,
  functionsByTypeAtom,
  standaloneFunctionsAtom,
  selectedFunctionAtom,
  isLLMOnlyModeAtom,
} from './atoms';
import type { BAMLEvent, InputSource } from './types';

// ============================================================================
// Workflow Hooks
// ============================================================================

/**
 * Get all available workflows
 */
export function useWorkflows() {
  return useAtomValue(workflowsAtom);
}

/**
 * Get and set the active workflow
 */
export function useActiveWorkflow() {
  const [activeWorkflowId, setActiveWorkflowId] = useAtom(activeWorkflowIdAtom);
  const activeWorkflow = useAtomValue(activeWorkflowAtom);
  const workflows = useAtomValue(workflowsAtom);

  const setActive = useCallback(
    (workflowId: string | null) => {
      if (workflowId === null) {
        setActiveWorkflowId(null);
        return;
      }

      const workflow = workflows.find((w) => w.id === workflowId);
      if (workflow) {
        setActiveWorkflowId(workflowId);
      } else {
        console.warn(`⚠️ Cannot set active workflow: "${workflowId}" not found`);
      }
    },
    [workflows, setActiveWorkflowId]
  );

  return {
    activeWorkflow,
    activeWorkflowId,
    setActiveWorkflow: setActive,
  };
}

// ============================================================================
// Execution Hooks
// ============================================================================

/**
 * Get executions for the active workflow
 */
export function useActiveWorkflowExecutions() {
  return useAtomValue(activeWorkflowExecutionsAtom);
}

/**
 * Get the selected execution (for viewing snapshots)
 */
export function useSelectedExecution() {
  const [selectedExecutionId, setSelectedExecutionId] = useAtom(
    selectedExecutionIdAtom
  );
  const selectedExecution = useAtomValue(selectedExecutionAtom);

  return {
    selectedExecution,
    selectedExecutionId,
    setSelectedExecutionId,
  };
}

/**
 * Get the latest execution for the active workflow
 */
export function useLatestExecution() {
  return useAtomValue(latestExecutionAtom);
}

// ============================================================================
// Node Hooks
// ============================================================================

/**
 * Get node execution data
 */
export function useNodeExecutions() {
  return useAtomValue(nodeExecutionsAtom);
}

/**
 * Get execution data for a specific node
 */
export function useNodeExecution(nodeId: string) {
  const nodeExecutions = useNodeExecutions();
  return nodeExecutions.get(nodeId) ?? null;
}

/**
 * Get state for a specific node using atomFamily
 *
 * This provides granular subscriptions - components only re-render
 * when their specific node's state changes.
 */
export function useNodeState(nodeId: string) {
  return useAtomValue(nodeStateAtomFamily(nodeId));
}

/**
 * Set state for a specific node
 */
export function useSetNodeState(nodeId: string) {
  return useSetAtom(nodeStateAtomFamily(nodeId));
}

/**
 * Get all node states as a Map
 *
 * Use this only when you need to iterate over all nodes.
 * For single-node subscriptions, use useNodeState() instead for better performance.
 */
export function useAllNodeStates() {
  return useAtomValue(allNodeStatesAtom);
}

// ============================================================================
// UI State Hooks
// ============================================================================

/**
 * Get and set view mode (editor vs execution snapshot)
 */
export function useViewMode() {
  return useAtom(viewModeAtom);
}

/**
 * Get and set selected node
 */
export function useSelectedNode() {
  return useAtom(selectedNodeIdAtom);
}

/**
 * Get and set detail panel state
 */
export function useDetailPanel() {
  const panel = useAtomValue(detailPanelAtom);
  const setPanel = useSetAtom(detailPanelAtom);

  const open = useCallback(() => {
    setPanel((prev) => ({ ...prev, isOpen: true }));
  }, [setPanel]);

  const close = useCallback(() => {
    setPanel((prev) => ({ ...prev, isOpen: false }));
  }, [setPanel]);

  const setPosition = useCallback(
    (position: 'bottom' | 'right' | 'floating') => {
      setPanel((prev) => ({ ...prev, position }));
    },
    [setPanel]
  );

  const setActiveTab = useCallback(
    (tab: 'io' | 'logs' | 'history') => {
      setPanel((prev) => ({ ...prev, activeTab: tab }));
    },
    [setPanel]
  );

  return {
    ...panel,
    open,
    close,
    setPosition,
    setActiveTab,
  };
}

/**
 * Get and set layout direction
 */
export function useLayoutDirection() {
  return useAtom(layoutDirectionAtom);
}

// ============================================================================
// Event Hooks
// ============================================================================

/**
 * Get recent events
 */
export function useEvents(limit = 10) {
  const events = useAtomValue(eventStreamAtom);
  return events.slice(-limit);
}

/**
 * Subscribe to specific event types
 */
export function useEventSubscription(
  eventType: BAMLEvent['type'],
  callback: (event: BAMLEvent) => void
) {
  const events = useAtomValue(eventStreamAtom);

  // Only call callback for new events of the specified type
  useMemo(() => {
    const latestEvent = events[events.length - 1];
    if (latestEvent && latestEvent.type === eventType) {
      callback(latestEvent);
    }
  }, [events, eventType, callback]);
}

// ============================================================================
// Combined/Derived Hooks
// ============================================================================

/**
 * Get the current graph to display (editor or snapshot)
 */
export function useCurrentGraph() {
  const [viewMode] = useViewMode();
  const activeWorkflow = useAtomValue(activeWorkflowAtom);
  const selectedExecution = useAtomValue(selectedExecutionAtom);

  return useMemo(() => {
    if (viewMode.mode === 'execution' && selectedExecution) {
      // Return snapshot graph
      return {
        nodes: selectedExecution.graphSnapshot.nodes,
        edges: selectedExecution.graphSnapshot.edges,
        isSnapshot: true,
        execution: selectedExecution,
      };
    }

    // Return live graph from current workflow
    return {
      nodes: activeWorkflow?.nodes ?? [],
      edges: activeWorkflow?.edges ?? [],
      isSnapshot: false,
      workflow: activeWorkflow,
    };
  }, [viewMode, activeWorkflow, selectedExecution]);
}

/**
 * Get the active node (selected node with its execution data)
 */
export function useActiveNode() {
  const [selectedNodeId] = useSelectedNode();
  const nodeExecutions = useNodeExecutions();
  const currentGraph = useCurrentGraph();

  // IMPORTANT: Call all hooks before any conditional returns (Rules of Hooks)
  // Use empty string as fallback to maintain hook call order even when no node is selected
  const state = useNodeState(selectedNodeId ?? '');

  if (!selectedNodeId) return null;

  const node = currentGraph.nodes.find((n) => n.id === selectedNodeId);
  if (!node) return null;

  const execution = nodeExecutions.get(selectedNodeId);

  return {
    node,
    execution,
    state,
  };
}

// ============================================================================
// Input Library Hooks (Phase 1: Previous Executions)
// ============================================================================

/**
 * Get available input sources for a node
 * Includes both previous executions and test cases
 */
export function useNodeInputSources(nodeId: string): InputSource[] {
  const workflowExecutions = useAtomValue(activeWorkflowExecutionsAtom);

  return useMemo(() => {
    if (!workflowExecutions.length) return [];
    const inputSources: InputSource[] = [];

    // Build input sources from executions where this node ran
    workflowExecutions.forEach((execution, index) => {
      const nodeExec = execution.nodeExecutions.get(nodeId);
      if (nodeExec && nodeExec.inputs) {
        inputSources.push({
          id: execution.id,
          name: `Execution #${workflowExecutions.length - index}`,
          source: 'execution',
          nodeId,
          executionId: execution.id,
          timestamp: execution.timestamp,
          inputs: nodeExec.inputs,
          outputs: nodeExec.outputs,
          status: nodeExec.state === 'success' ? 'success' : nodeExec.state === 'error' ? 'error' : 'running',
        });
      }
    });

    return inputSources;
  }, [workflowExecutions, nodeId]);
}

/**
 * Get and set the selected input source
 */
export function useSelectedInputSource() {
  const [selectedSource, setSelectedSource] = useAtom(selectedInputSourceAtom);

  const selectSource = useCallback(
    (nodeId: string, sourceType: 'execution' | 'test' | 'manual', sourceId: string) => {
      setSelectedSource({ nodeId, sourceType, sourceId });
    },
    [setSelectedSource]
  );

  const clearSource = useCallback(() => {
    setSelectedSource(null);
  }, [setSelectedSource]);

  return {
    selectedSource,
    selectSource,
    clearSource,
  };
}

/**
 * Get active inputs for a node
 * Returns inputs from selected source or latest execution
 */
export function useNodeActiveInputs(nodeId: string): Record<string, any> {
  const selectedSource = useAtomValue(selectedInputSourceAtom);
  const inputSources = useNodeInputSources(nodeId);
  const latestExecution = useLatestExecution();

  return useMemo(() => {
    // If a source is selected for this node, use it
    if (selectedSource && selectedSource.nodeId === nodeId) {
      const source = inputSources.find((s) => s.id === selectedSource.sourceId);
      if (source) {
        return source.inputs;
      }
    }

    // Otherwise, fall back to latest execution
    if (latestExecution) {
      const nodeExec = latestExecution.nodeExecutions.get(nodeId);
      if (nodeExec?.inputs) {
        return nodeExec.inputs;
      }
    }

    return {};
  }, [selectedSource, inputSources, latestExecution, nodeId]);
}

/**
 * Get/set active node inputs (editable)
 */
export function useActiveNodeInputs() {
  const [inputs, setInputs] = useAtom(activeNodeInputsAtom);
  const [isDirty, setIsDirty] = useAtom(inputsDirtyAtom);

  const updateInputs = useCallback(
    (newInputs: Record<string, any>) => {
      setInputs(newInputs);
      setIsDirty(true);
    },
    [setInputs, setIsDirty]
  );

  const resetInputs = useCallback(() => {
    setInputs({});
    setIsDirty(false);
  }, [setInputs, setIsDirty]);

  return {
    inputs,
    updateInputs,
    resetInputs,
    isDirty,
  };
}

// ============================================================================
// Derived Atoms Hooks (Function Lookup & LLM Mode)
// ============================================================================

/**
 * Get all functions as a Map for O(1) lookup
 */
export function useAllFunctions() {
  return useAtomValue(allFunctionsMapAtom);
}

/**
 * Get functions grouped by type
 */
export function useFunctionsByType() {
  return useAtomValue(functionsByTypeAtom);
}

/**
 * Get standalone functions (not in any workflow)
 */
export function useStandaloneFunctions() {
  return useAtomValue(standaloneFunctionsAtom);
}

/**
 * Get the currently selected function details
 */
export function useSelectedFunction() {
  return useAtomValue(selectedFunctionAtom);
}

/**
 * Check if we're in LLM-only mode
 */
export function useLLMOnlyMode() {
  return useAtomValue(isLLMOnlyModeAtom);
}
