/**
 * Code Navigation Hook
 *
 * Handles navigation when user clicks on functions/tests in the Debug Panel.
 * Uses the navigation heuristic to determine the appropriate action.
 */

import { useEffect } from 'react';
import { useAtomValue, useSetAtom } from 'jotai';
import { activeCodeClickAtom } from '../../../sdk/atoms/core.atoms';
import { useBAMLSDK } from '../../../sdk/provider';
import {
  determineNavigationAction,
  getCurrentNavigationState,
} from '../../../sdk/navigationHeuristic';
import {
  useActiveWorkflow,
  useSelectedNode,
  useDetailPanel,
  useSelectedInputSource,
} from '../../../sdk/hooks';
import { flowStore } from '../../../states/reactflow';
import { panToNodeIfNeeded } from '../../../utils/cameraPan';
import {
  unifiedSelectionAtom,
  activeTabAtom,
  detailPanelStateAtom,
} from '../../../shared/baml-project-panel/playground-panel/atoms';

/**
 * Hook that listens to code click events and performs navigation
 */
export function useCodeNavigation() {
  const activeCodeClick = useAtomValue(activeCodeClickAtom);
  const sdk = useBAMLSDK();
  const { setActiveWorkflow } = useActiveWorkflow();
  const [, setSelectedNodeId] = useSelectedNode();
  const { open: openDetailPanel } = useDetailPanel();
  const { selectSource } = useSelectedInputSource();

  // Unified state setters
  const setUnifiedSelection = useSetAtom(unifiedSelectionAtom);
  const setActiveTab = useSetAtom(activeTabAtom);
  const setDetailPanelState = useSetAtom(detailPanelStateAtom);

  useEffect(() => {
    if (!activeCodeClick) return;

    console.log('📍 Code click event:', activeCodeClick);

    // Get current navigation state
    const navState = getCurrentNavigationState(sdk);

    // Determine what action to take
    const action = determineNavigationAction(activeCodeClick, navState);
    console.log('🧭 Navigation action:', action);

    // Track timeout IDs for cleanup
    const timeouts: ReturnType<typeof setTimeout>[] = [];

    // Execute the action
    switch (action.type) {
      case 'switch-workflow':
        console.log('🔄 Switching to workflow:', action.workflowId);
        // Check if workflow exists before switching
        const targetWorkflow = sdk.workflows.getById(action.workflowId);
        if (targetWorkflow) {
          setActiveWorkflow(action.workflowId);

          // Update unified state
          setUnifiedSelection({
            functionName: action.workflowId,
            testName: null,
            activeWorkflowId: action.workflowId,
            selectedNodeId: null,
          });
          setActiveTab('graph');
        } else {
          console.error(`❌ Cannot switch to workflow: "${action.workflowId}" not found`);
          // Clear selection to avoid broken state
          setActiveWorkflow(null);
          setSelectedNodeId(null);
          setUnifiedSelection({
            functionName: null,
            testName: null,
            activeWorkflowId: null,
            selectedNodeId: null,
          });
        }
        break;

      case 'select-node':
        console.log(
          '🎯 Selecting node in current workflow:',
          action.nodeId,
          action.testId ? `with test: ${action.testId}` : ''
        );
        setSelectedNodeId(action.nodeId);
        openDetailPanel();

        // Update unified state
        // Only set testName if we're explicitly selecting a test (not just a function)
        setUnifiedSelection((prev) => ({
          ...prev,
          selectedNodeId: action.nodeId,
          functionName: action.nodeId,
          testName: action.testId ?? null, // Clear test if not provided
        }));
        setActiveTab('graph');
        setDetailPanelState({ isOpen: true });

        // If a testId is provided, select that test case in the details panel
        if (action.testId) {
          console.log('🎯 Selecting test case:', action.testId);
          selectSource(action.nodeId, 'test', action.testId);
        }

        // Pan to node after a brief delay to ensure node is rendered
        timeouts.push(setTimeout(() => {
          const node = flowStore.value.getNode(action.nodeId);
          if (node) {
            panToNodeIfNeeded(node, flowStore.value);
          }
        }, 100));
        break;

      case 'switch-and-select':
        console.log(
          '🔄 Switching to workflow and selecting node:',
          action.workflowId,
          action.nodeId,
          action.testId ? `with test: ${action.testId}` : ''
        );

        // Check if workflow exists before switching
        const workflowToSwitch = sdk.workflows.getById(action.workflowId);
        if (!workflowToSwitch) {
          console.error(`❌ Cannot switch to workflow: "${action.workflowId}" not found`);
          // Clear selection to avoid broken state
          setActiveWorkflow(null);
          setSelectedNodeId(null);
          setUnifiedSelection({
            functionName: null,
            testName: null,
            activeWorkflowId: null,
            selectedNodeId: null,
          });
          break;
        }

        // First clear the selected node to exit LLM-only mode
        setSelectedNodeId(null);

        // Then switch to the new workflow
        setActiveWorkflow(action.workflowId);

        // Update unified state for workflow switch
        setUnifiedSelection({
          functionName: action.nodeId,
          testName: action.testId ?? null,
          activeWorkflowId: action.workflowId,
          selectedNodeId: null, // Will be set after workflow loads
        });
        setActiveTab('graph');

        // Wait for workflow to load before selecting node
        timeouts.push(setTimeout(() => {
          console.log('🎯 Selecting node in workflow:', action.nodeId);
          setSelectedNodeId(action.nodeId);
          openDetailPanel();

          // Update unified state with selected node
          setUnifiedSelection((prev) => ({
            ...prev,
            selectedNodeId: action.nodeId,
          }));
          setDetailPanelState({ isOpen: true });

          // If a testId is provided, select that test case in the details panel
          if (action.testId) {
            console.log('🎯 Selecting test case:', action.testId);
            selectSource(action.nodeId, 'test', action.testId);
          }

          timeouts.push(setTimeout(() => {
            const node = flowStore.value.getNode(action.nodeId);
            if (node) {
              panToNodeIfNeeded(node, flowStore.value);
            } else {
              console.warn('⚠️ Node not found in ReactFlow after switching:', action.nodeId);
            }
          }, 100));
        }, 400));
        break;

      case 'show-function-tests':
        console.log('📝 Showing function with tests:', action.functionName);
        // Clear active workflow to trigger LLM-only view for standalone function
        setActiveWorkflow(null);

        // Update unified state for standalone function
        setUnifiedSelection({
          functionName: action.functionName,
          testName: action.tests[0] ?? null,
          activeWorkflowId: null,
          selectedNodeId: null,
        });
        setActiveTab('preview'); // Show prompt preview for standalone LLM function
        setDetailPanelState({ isOpen: false }); // Close detail panel for standalone view

        // Then select the function and open detail panel to show tests
        timeouts.push(setTimeout(() => {
          setSelectedNodeId(action.functionName);
          openDetailPanel();
        }, 100));
        break;

      case 'empty-state':
        console.log('📭 Empty state:', action.reason, action.functionName);
        // Clear workflow and selection to show empty state
        setActiveWorkflow(null);
        setSelectedNodeId(null);

        // Clear unified state
        setUnifiedSelection({
          functionName: null,
          testName: null,
          activeWorkflowId: null,
          selectedNodeId: null,
        });
        setActiveTab('preview');
        setDetailPanelState({ isOpen: false });
        break;
    }

    // Cleanup function to cancel pending timeouts
    return () => {
      timeouts.forEach(clearTimeout);
    };
  }, [
    activeCodeClick,
    sdk,
    setActiveWorkflow,
    setSelectedNodeId,
    selectSource,
    openDetailPanel,
    setUnifiedSelection,
    setActiveTab,
    setDetailPanelState,
  ]);
}
