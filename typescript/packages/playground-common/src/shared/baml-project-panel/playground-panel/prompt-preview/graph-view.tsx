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
import { useDetailPanel, useActiveWorkflow, useLayoutDirection } from '../../../../sdk/hooks';
import { flowStore } from '../../../../states/reactflow';
import { Loader as Spinner } from '@baml/ui/custom/loader';
import { useGraphSync } from '../../../../features/graph/hooks';
import { useAtom, useAtomValue, useSetAtom } from 'jotai';
import { detailPanelStateAtom, graphControlsTipDismissedAtom, unifiedSelectionAtom } from '../atoms';
import { MousePointer2, ZoomIn, X } from 'lucide-react';

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
  const detailPanel = useDetailPanel();
  const { activeWorkflowId } = useActiveWorkflow();
  const [direction] = useLayoutDirection();

  // Sync detail panel state with unified atoms
  const setDetailPanelState = useSetAtom(detailPanelStateAtom);
  const setUnifiedSelection = useSetAtom(unifiedSelectionAtom);
  const [graphTipDismissed, setGraphTipDismissed] = useAtom(
    graphControlsTipDismissedAtom
  );
  const selectedNodeId = useAtomValue(unifiedSelectionAtom).selectedNodeId;
  useEffect(() => {
    setDetailPanelState({ isOpen: detailPanel.isOpen });
  }, [detailPanel.isOpen, setDetailPanelState]);

  useEffect(() => {
    const nodes = flowStore.value.getNodes?.();
    if (!nodes || !nodes.length) return;
    const updated = nodes.map((node) =>
      node.selected === (node.id === selectedNodeId)
        ? node
        : { ...node, selected: node.id === selectedNodeId }
    );
    flowStore.value.setNodes?.(updated);
  }, [selectedNodeId]);

  const { getEdges, setNodes } = useReactFlow();

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
    setUnifiedSelection((prev) => ({
      ...prev,
      functionName: node.id,
      testName: null,
      selectedNodeId: node.id,
      activeWorkflowId: prev.activeWorkflowId ?? activeWorkflowId ?? null,
    }));
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

      {!graphTipDismissed && (
        <div className="absolute top-4 left-4 z-20 max-w-xs rounded-md border border-border bg-background/95 shadow-lg p-3 text-xs text-muted-foreground">
          <div className="flex items-center justify-between gap-2 mb-2 text-[11px] font-semibold text-foreground">
            <span>Navigate like Figma</span>
            <button
              type="button"
              className="text-muted-foreground hover:text-foreground"
              onClick={() => setGraphTipDismissed(true)}
            >
              <X className="w-3 h-3" />
            </button>
          </div>
          <div className="flex items-center gap-2 mb-1">
            <MousePointer2 className="w-3.5 h-3.5 text-primary" />
            <span>Right-click + drag to pan</span>
          </div>
          <div className="flex items-center gap-2">
            <ZoomIn className="w-3.5 h-3.5 text-primary" />
            <span>⌘ + scroll to zoom</span>
          </div>
        </div>
      )}
    </div>
  );
};
