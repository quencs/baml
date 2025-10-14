import React, { CSSProperties, useMemo } from 'react';
import {
  ReactFlow,
  ReactFlowProvider,
  Node,
  Edge,
  MarkerType,
  useNodesState,
  useEdgesState,
  Background,
  Controls,
} from '@xyflow/react';

interface NodeLayout {
  row: number;
  col: number;
}

type NodeKind = 'gather' | 'build' | 'decision' | 'operation';

type PipelineNodeConfig = {
  label: string;
  subtitle?: string;
  kind: NodeKind;
};

const NODE_HORIZONTAL_SPACING = 220;
const NODE_VERTICAL_SPACING = 170;

const BASE_STYLE: CSSProperties = {
  padding: 16,
  borderRadius: 16,
  fontWeight: 600,
  display: 'flex',
  flexDirection: 'column',
  gap: 4,
  boxShadow: '0 12px 28px rgba(15, 23, 42, 0.12)',
  border: '1px solid transparent',
};

const STYLE_MAP: Record<NodeKind, CSSProperties> = {
  gather: {
    ...BASE_STYLE,
    background: '#ecfdf5',
    borderColor: '#34d399',
    color: '#065f46',
  },
  build: {
    ...BASE_STYLE,
    background: '#eef2ff',
    borderColor: '#6366f1',
    color: '#312e81',
  },
  decision: {
    ...BASE_STYLE,
    background: '#fef3c7',
    borderColor: '#f59e0b',
    color: '#92400e',
  },
  operation: {
    ...BASE_STYLE,
    background: '#e0f2fe',
    borderColor: '#0ea5e9',
    color: '#075985',
  },
};

const PIPELINE_NODE_CONFIG: Record<string, PipelineNodeConfig> = {
  fetchDocs: {
    label: 'FetchProxmoxApiDocs()',
    subtitle: 'Gather docs',
    kind: 'gather',
  },
  fetchWiki: {
    label: 'FetchProxmoxWiki()',
    subtitle: 'Gather wiki',
    kind: 'gather',
  },
  fetchReference: {
    label: 'FetchProxmoxApiReference()',
    subtitle: 'Gather reference',
    kind: 'gather',
  },
  buildRequest: {
    label: 'BuildProxmoxHttpRequest()',
    subtitle: 'Assemble HTTP request',
    kind: 'build',
  },
  switchVerb: {
    label: 'Switch req.verb',
    subtitle: 'GET | POST | DELETE',
    kind: 'decision',
  },
  fetchMetadata: {
    label: 'FetchProxmoxResourceMetadata()',
    subtitle: 'GET',
    kind: 'operation',
  },
  modifyResource: {
    label: 'ModifyProxmoxResource()',
    subtitle: 'POST',
    kind: 'operation',
  },
  deleteResource: {
    label: 'DeleteProxmoxResource()',
    subtitle: 'DELETE',
    kind: 'operation',
  },
};

const NODE_LAYOUT: Record<string, NodeLayout> = {
  fetchDocs: { row: 0, col: 0 },
  fetchWiki: { row: 1, col: 0 },
  fetchReference: { row: 2, col: 0 },
  buildRequest: { row: 3, col: 0 },
  switchVerb: { row: 4, col: 0 },
  fetchMetadata: { row: 5, col: -1 },
  modifyResource: { row: 5, col: 0 },
  deleteResource: { row: 5, col: 1 },
};

const N8nDevopsProxmoxInner: React.FC = () => {
  const initialNodes = useMemo<Node[]>(() => {
    return Object.entries(PIPELINE_NODE_CONFIG).map(([id, config]) => {
      const layout = NODE_LAYOUT[id] ?? { row: 0, col: 0 };
      const x = layout.col * NODE_HORIZONTAL_SPACING;
      const y = layout.row * NODE_VERTICAL_SPACING;

      return {
        id,
        data: {
          label: (
            <div>
              <div style={{ fontSize: 14, fontWeight: 700 }}>{config.label}</div>
              {config.subtitle && (
                <div style={{ fontSize: 12, fontWeight: 500, opacity: 0.8 }}>{config.subtitle}</div>
              )}
            </div>
          ),
        },
        position: { x, y },
        style: { ...STYLE_MAP[config.kind] },
      } satisfies Node;
    });
  }, []);

  const initialEdges = useMemo<Edge[]>(() => [
    {
      id: 'docs-wiki',
      source: 'fetchDocs',
      target: 'fetchWiki',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#34d399' },
      style: { stroke: '#34d399', strokeWidth: 2 },
    },
    {
      id: 'wiki-reference',
      source: 'fetchWiki',
      target: 'fetchReference',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#22c55e' },
      style: { stroke: '#22c55e', strokeWidth: 2 },
    },
    {
      id: 'reference-build',
      source: 'fetchReference',
      target: 'buildRequest',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#6366f1' },
      style: { stroke: '#6366f1', strokeWidth: 2 },
    },
    {
      id: 'build-switch',
      source: 'buildRequest',
      target: 'switchVerb',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#f59e0b' },
      style: { stroke: '#f59e0b', strokeWidth: 2 },
    },
    {
      id: 'switch-get',
      source: 'switchVerb',
      target: 'fetchMetadata',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#0ea5e9' },
      style: { stroke: '#0ea5e9', strokeWidth: 2 },
      label: 'GET',
      labelStyle: { fill: '#0369a1', fontWeight: 600 },
      labelBgStyle: { fill: '#e0f2fe', fillOpacity: 0.85, stroke: '#bae6fd' },
    },
    {
      id: 'switch-post',
      source: 'switchVerb',
      target: 'modifyResource',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#22c55e' },
      style: { stroke: '#22c55e', strokeWidth: 2, strokeDasharray: '6 3' },
      label: 'POST',
      labelStyle: { fill: '#15803d', fontWeight: 600 },
      labelBgStyle: { fill: '#dcfce7', fillOpacity: 0.85, stroke: '#bbf7d0' },
    },
    {
      id: 'switch-delete',
      source: 'switchVerb',
      target: 'deleteResource',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#f87171' },
      style: { stroke: '#f87171', strokeWidth: 2, strokeDasharray: '6 3' },
      label: 'DELETE',
      labelStyle: { fill: '#b91c1c', fontWeight: 600 },
      labelBgStyle: { fill: '#fee2e2', fillOpacity: 0.85, stroke: '#fecaca' },
    },
  ], []);

  const [nodes, , onNodesChange] = useNodesState<Node>(initialNodes);
  const [edges, , onEdgesChange] = useEdgesState(initialEdges);

  return (
    <div style={{ height: '100%', width: '100%', background: '#f8fafc' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
        fitViewOptions={{ padding: 0.28 }}
        panOnScroll
        selectionOnDrag
        nodeOrigin={[0.5, 0.5]}
        minZoom={0.5}
        maxZoom={1.5}
      >
        <Background gap={24} size={1} color="#e2e8f0" />
        <Controls position="top-left" />
      </ReactFlow>
    </div>
  );
};

const N8nDevopsProxmoxTab: React.FC = () => (
  <ReactFlowProvider>
    <N8nDevopsProxmoxInner />
  </ReactFlowProvider>
);

export default N8nDevopsProxmoxTab;
