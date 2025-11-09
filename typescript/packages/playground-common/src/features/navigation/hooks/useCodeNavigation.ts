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
import { useDetailPanel, useSelectedInputSource } from '../../../sdk/hooks';
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
      case 'switch-workflow': {
        console.log('🔄 Switching to workflow:', action.workflowId);
        const targetWorkflow = sdk.workflows.getById(action.workflowId);
        if (targetWorkflow) {
          setUnifiedSelection((prev) => ({
            ...prev,
            functionName: action.workflowId,
            testName: null,
            activeWorkflowId: action.workflowId,
            selectedNodeId: null,
          }));
          setActiveTab('graph');
        } else {
          console.error(`❌ Cannot switch to workflow: "${action.workflowId}" not found`);
          setUnifiedSelection({
            functionName: null,
            testName: null,
            activeWorkflowId: null,
            selectedNodeId: null,
          });
        }
        break;
      }

      case 'select-node':
        console.log(
          '🎯 Selecting node in current workflow:',
          action.nodeId,
          action.testId ? `with test: ${action.testId}` : ''
        );
        setUnifiedSelection((prev) => ({
          ...prev,
          selectedNodeId: action.nodeId,
          functionName: action.nodeId,
          testName: action.testId ?? null,
          activeWorkflowId: action.workflowId,
        }));
        openDetailPanel();

        // Update unified state
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

      case 'switch-and-select': {
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
          setUnifiedSelection({
            functionName: null,
            testName: null,
            activeWorkflowId: null,
            selectedNodeId: null,
          });
          break;
        }

        // Clear node to exit LLM-only mode but remember pending target
        setUnifiedSelection({
          functionName: action.nodeId,
          testName: action.testId ?? null,
          activeWorkflowId: action.workflowId,
          selectedNodeId: null,
        });
        setActiveTab('graph');

        // Wait for workflow to load before selecting node
        timeouts.push(setTimeout(() => {
          console.log('🎯 Selecting node in workflow:', action.nodeId);
          setUnifiedSelection((prev) => ({
            ...prev,
            selectedNodeId: action.nodeId,
          }));
          openDetailPanel();

          // Update unified state with selected node
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
      }

      case 'show-function-tests':
        console.log('📝 Showing function with tests:', action.functionName);
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
          setUnifiedSelection((prev) => ({
            ...prev,
            selectedNodeId: action.functionName,
          }));
          openDetailPanel();
        }, 100));
        break;

      case 'empty-state':
        console.log('📭 Empty state:', action.reason, action.functionName);
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
    selectSource,
    openDetailPanel,
    setUnifiedSelection,
    setActiveTab,
    setDetailPanelState,
  ]);
}
