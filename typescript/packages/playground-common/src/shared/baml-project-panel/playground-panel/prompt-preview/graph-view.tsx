'use client';

import '@xyflow/react/dist/style.css';

import {
  Background,
  BackgroundVariant,
  Controls,
  ReactFlow,
  SelectionMode,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from '@xyflow/react';
import { useEffect, useId } from 'react';
import type { Node } from '@xyflow/react';

// Import graph primitives and components from WorkflowApp
import { kEdgeTypes, ColorfulMarkerDefinitions, kNodeTypes } from '../../../../graph-primitives';
import { ReactflowInstance } from '../../../../features/graph/components';
import { useAutoLayout } from '../../../../features/graph/layout/useAutoLayout';
import { useDetailPanel, useActiveWorkflow, useLayoutDirection, useSelectedNode } from '../../../../sdk/hooks';
import { flowStore } from '../../../../states/reactflow';
import { useBAMLSDK } from '../../../../sdk/provider';
import { Loader as Spinner } from '@baml/ui/custom/loader';
import { useGraphSync } from '../../../../features/graph/hooks';
import { useSetAtom } from 'jotai';
import { detailPanelStateAtom } from '../atoms';

/**
 * GraphView - ReactFlow graph component for the Graph tab
 *
 * This component renders the workflow graph and handles:
 * - Auto-layout
 * - Node selection
 * - Detail panel integration
 */
export const GraphView = () => {
  const [nodes, _setNodes, onNodesChange] = useNodesState([]);
  const [edges, _setEdges, onEdgesChange] = useEdgesState([]);

  // Feature hooks
  const { convertedGraph, isLayoutLoading } = useGraphSync();

  // SDK hooks
  const [, setSelectedNodeId] = useSelectedNode();
  const detailPanel = useDetailPanel();
  const { activeWorkflowId } = useActiveWorkflow();
  const [direction] = useLayoutDirection();
  const sdk = useBAMLSDK();

  // Sync detail panel state with unified atoms
  const setDetailPanelState = useSetAtom(detailPanelStateAtom);
  useEffect(() => {
    setDetailPanelState({ isOpen: detailPanel.isOpen });
  }, [detailPanel.isOpen, setDetailPanelState]);

  const { getEdges, setNodes } = useReactFlow();
  const { layout } = useAutoLayout();

  // UI state
  const backgroundId = useId();

  // Clear node states when workflow changes
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
  }, [activeWorkflowId, setNodes]);

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
    <div className="relative w-full h-full">
      <ColorfulMarkerDefinitions />

      {/* Loading overlay */}
      {isLayoutLoading && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-background">
          <div className="flex flex-col items-center gap-3">
            <Spinner className="size-8" />
            <p className="text-sm text-muted-foreground">Calculating layout...</p>
          </div>
        </div>
      )}

      {/* ReactFlow Graph */}
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
        panOnDrag={[1, 2]}
        selectionMode={SelectionMode.Partial}
        colorMode="light"
      >
        <Background
          className="bg-background"
          color="hsl(var(--muted))"
          id={backgroundId}
          variant={BackgroundVariant.Dots}
        />
        <ReactflowInstance />
        <Controls />
      </ReactFlow>
    </div>
  );
};
