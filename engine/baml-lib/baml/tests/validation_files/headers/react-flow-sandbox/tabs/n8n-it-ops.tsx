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

type NodeKind = 'action' | 'decision' | 'outcome';

type PipelineNodeConfig = {
  label: string;
  subtitle?: string;
  kind: NodeKind;
};

const NODE_HORIZONTAL_SPACING = 220;
const NODE_VERTICAL_SPACING = 180;

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
  action: {
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
  outcome: {
    ...BASE_STYLE,
    background: '#ecfdf5',
    borderColor: '#34d399',
    color: '#065f46',
  },
};

const PIPELINE_NODE_CONFIG: Record<string, PipelineNodeConfig> = {
  getUser: {
    label: 'GetUserFromMicrosoftEntra()',
    subtitle: 'Action',
    kind: 'action',
  },
  createJira: {
    label: 'CreateJiraUser()',
    subtitle: 'Action',
    kind: 'action',
  },
  decision: {
    label: 'If / Else',
    subtitle: 'IsManager()',
    kind: 'decision',
  },
  addSlack: {
    label: 'AddToSlackChannel()',
    subtitle: 'Then',
    kind: 'outcome',
  },
  updateProfile: {
    label: 'UpdateSlackProfile()',
    subtitle: 'Else',
    kind: 'outcome',
  },
};

const NODE_LAYOUT: Record<string, { row: number; col: number }> = {
  getUser: { row: 0, col: 0 },
  createJira: { row: 1, col: 0 },
  decision: { row: 2, col: 0 },
  addSlack: { row: 3, col: -1 },
  updateProfile: { row: 3, col: 1 },
};

const N8nItOpsInner: React.FC = () => {
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
        type: 'default',
        position: { x, y },
        style: { ...STYLE_MAP[config.kind] },
      } satisfies Node;
    });
  }, []);

  const initialEdges = useMemo<Edge[]>(() => [
    {
      id: 'getUser-createJira',
      source: 'getUser',
      target: 'createJira',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#6366f1' },
      style: { stroke: '#6366f1', strokeWidth: 2 },
    },
    {
      id: 'createJira-decision',
      source: 'createJira',
      target: 'decision',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#f59e0b' },
      style: { stroke: '#f59e0b', strokeWidth: 2 },
    },
    {
      id: 'decision-addSlack',
      source: 'decision',
      target: 'addSlack',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#22c55e' },
      style: { stroke: '#22c55e', strokeWidth: 2 },
      label: 'Then',
      labelStyle: { fill: '#15803d', fontWeight: 600 },
      labelBgStyle: { fill: '#dcfce7', fillOpacity: 0.85, stroke: '#bbf7d0' },
    },
    {
      id: 'decision-updateProfile',
      source: 'decision',
      target: 'updateProfile',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#60a5fa' },
      style: { stroke: '#60a5fa', strokeWidth: 2, strokeDasharray: '6 3' },
      label: 'Else',
      labelStyle: { fill: '#1d4ed8', fontWeight: 600 },
      labelBgStyle: { fill: '#eff6ff', fillOpacity: 0.85, stroke: '#bfdbfe' },
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
        fitViewOptions={{ padding: 0.25 }}
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

const N8nItOpsTab: React.FC = () => (
  <ReactFlowProvider>
    <N8nItOpsInner />
  </ReactFlowProvider>
);

export default N8nItOpsTab;
