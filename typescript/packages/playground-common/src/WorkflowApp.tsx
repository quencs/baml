import '@xyflow/react/dist/style.css';

import {
  Background,
  BackgroundVariant,
  Controls,
  ReactFlow,
  ReactFlowProvider,
  SelectionMode,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from '@xyflow/react';
import { useEffect, useId, useState } from 'react';

import { DetailPanel } from './features/detail-panel';
import { WorkflowToolbar, WorkflowIndicator } from './features/workflow/components';
import { LLMOnlyPanel, LLMTestPanel } from './features/llm/components';
import { DebugPanel } from './features/debug';
import { kEdgeTypes, ColorfulMarkerDefinitions, kNodeTypes } from './graph-primitives';
import { ReactflowInstance } from './features/graph/components';
import { useAutoLayout } from './features/graph/layout/useAutoLayout';
import { useDetailPanel, useActiveWorkflow, useLayoutDirection, useSelectedNode, useLLMOnlyMode } from './sdk/hooks';
import { flowStore } from './states/reactflow';
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from '@baml/ui/resizable';
import type { Node } from '@xyflow/react';
import { useBAMLSDK } from './sdk/provider';
import { Loader as Spinner } from '@baml/ui/custom/loader';
import { useCodeNavigation } from './features/navigation/hooks';
import { useExecutionSync } from './features/execution/hooks';
import { useGraphSync } from './features/graph/hooks';

const EditWorkFlow = () => {
  const [nodes, _setNodes, onNodesChange] = useNodesState([]);
  const [edges, _setEdges, onEdgesChange] = useEdgesState([]);

  // Feature hooks
  useCodeNavigation();
  useExecutionSync();
  const { convertedGraph, isLayoutLoading } = useGraphSync();

  // SDK hooks
  const [, setSelectedNodeId] = useSelectedNode();
  const detailPanel = useDetailPanel();
  const { activeWorkflowId } = useActiveWorkflow();
  const [direction] = useLayoutDirection();
  const sdk = useBAMLSDK();
  const shouldShowLLMOnlyView = useLLMOnlyMode();

  const { getEdges, setNodes } = useReactFlow();
  const { layout } = useAutoLayout();

  // UI state
  const [isDarkMode, setIsDarkMode] = useState(false);
  const backgroundId = useId(); // Generate ID at top level to avoid hook ordering issues

  // Toggle dark mode
  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }

    // Force edges to re-render with new colors by creating new edge objects
    const currentEdges = flowStore.value.getEdges();
    if (currentEdges.length > 0) {
      // Create new edge objects to force React to re-render with new theme colors
      const updatedEdges = currentEdges.map(edge => ({
        ...edge,
        // Add a timestamp to force re-render
        data: {
          ...edge.data,
          _themeUpdate: Date.now(),
        }
      }));
      flowStore.value.setEdges(updatedEdges);
    }
  }, [isDarkMode]);

  // ✅ Clear node states when workflow changes
  useEffect(() => {
    // Clear all node states AND outputs in UI when switching workflows
    setNodes((currentNodes) =>
      currentNodes.map((node) => ({
        ...node,
        data: {
          ...node.data,
          executionState: 'not-started',
          isExecutionActive: false,
          outputs: undefined,
          error: undefined,
        },
      }))
    );
  }, [activeWorkflowId, sdk.store, setNodes]);

  const handleResetLayout = async () => {
    // Re-run the layout algorithm from scratch without resetting viewport
    if (!convertedGraph) return;

    await layout({
      nodes: convertedGraph.nodes,
      edges: convertedGraph.edges,
      direction,
      skipFitView: true
    });
  };

  // Recalculate edge routing when a node is being dragged or moved
  const handleNodeDrag = () => {
    const currentEdges = getEdges();

    // Clear ELK routing data so edges use dynamic routing with new node positions
    const edgesWithoutElkRouting = currentEdges.map((edge) => ({
      ...edge,
      data: {
        ...edge.data,
        layout: edge.data?.layout ? {
          ...edge.data.layout,
          inputPoints: undefined, // Clear ELK routing
        } : undefined,
      },
    }));

    // Update edges to trigger re-render with new positions
    flowStore.value.setEdges(edgesWithoutElkRouting);
  };

  // Handle node click - select the node and open detail panel
  const handleNodeClick = (_event: React.MouseEvent, node: Node) => {
    console.log('Node clicked:', node.id);
    setSelectedNodeId(node.id);
    detailPanel.open();
  };

  return (
    <div className="w-screen h-screen flex flex-col bg-background">
      {/* Top Toolbar */}
      <WorkflowToolbar
        isDarkMode={isDarkMode}
        onToggleDarkMode={() => setIsDarkMode(!isDarkMode)}
        onResetLayout={handleResetLayout}
      />
      <ColorfulMarkerDefinitions />

      {/* Workflow Indicator (centered) - hide in LLM-only mode */}
      {!shouldShowLLMOnlyView && <WorkflowIndicator />}

      {/* Debug Panel */}
      <DebugPanel />

      {/* Resizable Layout */}
      <ResizablePanelGroup direction="vertical" className="flex-1" id="workflow-layout">
        {/* Main Graph Panel */}
        <ResizablePanel defaultSize={detailPanel.isOpen ? 60 : 100} minSize={30}>
          <div className="relative w-full h-full">
            {/* Loading overlay */}
            {isLayoutLoading && (
              <div className="absolute inset-0 z-50 flex items-center justify-center bg-background">
                <div className="flex flex-col items-center gap-3">
                  <Spinner className="size-8" />
                  <p className="text-sm text-muted-foreground">Calculating layout...</p>
                </div>
              </div>
            )}

            {/* LLM-Only View or Graph */}
            {shouldShowLLMOnlyView ? (
              <LLMOnlyPanel />
            ) : (
              <ReactFlow
                edges={edges}
                edgeTypes={kEdgeTypes}
                nodes={nodes}
                nodeTypes={kNodeTypes}
                onEdgesChange={onEdgesChange}
                onNodesChange={onNodesChange}
                onNodeDrag={handleNodeDrag}
                onNodeClick={handleNodeClick}
                panOnScroll
                selectionOnDrag
                panOnDrag={[1,2]}
                selectionMode={SelectionMode.Partial}

                colorMode={isDarkMode ? 'dark' : 'light'}
              >
                <Background
                  className="bg-background"
                  color={isDarkMode ? 'hsl(var(--muted))' : 'hsl(var(--muted))'}
                  id={backgroundId}
                  variant={BackgroundVariant.Dots}
                />
                <ReactflowInstance />
                <Controls />
                {/* <MiniMap pannable zoomable  style={{width: 50, height: 50}} /> */}
              </ReactFlow>
            )}
          </div>
        </ResizablePanel>

        {/* Detail Panel (Resizable) */}
        {detailPanel.isOpen && (
          <>
            <ResizableHandle />
            <ResizablePanel defaultSize={40} minSize={20} maxSize={70}>
              {shouldShowLLMOnlyView ? <LLMTestPanel /> : <DetailPanel />}
            </ResizablePanel>
          </>
        )}
      </ResizablePanelGroup>
    </div>
  );
};

export const WorkFlow = () => {
  return (
    <ReactFlowProvider>
      <EditWorkFlow />
    </ReactFlowProvider>
  );
};

// Export with SDK Provider for use in main.tsx
export default WorkFlow;
