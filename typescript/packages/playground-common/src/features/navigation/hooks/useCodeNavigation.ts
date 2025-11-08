/**
 * Code Navigation Hook
 *
 * Handles navigation when user clicks on functions/tests in the Debug Panel.
 * Uses the navigation heuristic to determine the appropriate action.
 */

import { useEffect } from 'react';
import { useAtomValue } from 'jotai';
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
        } else {
          console.error(`❌ Cannot switch to workflow: "${action.workflowId}" not found`);
          // Clear selection to avoid broken state
          setActiveWorkflow(null);
          setSelectedNodeId(null);
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
          break;
        }

        // First clear the selected node to exit LLM-only mode
        setSelectedNodeId(null);

        // Then switch to the new workflow
        setActiveWorkflow(action.workflowId);

        // Wait for workflow to load before selecting node
        timeouts.push(setTimeout(() => {
          console.log('🎯 Selecting node in workflow:', action.nodeId);
          setSelectedNodeId(action.nodeId);
          openDetailPanel();

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
  ]);
}
