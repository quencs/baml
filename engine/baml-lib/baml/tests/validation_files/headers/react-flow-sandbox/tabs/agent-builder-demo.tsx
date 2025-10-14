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

type NodeKind = 'entry' | 'agent' | 'decision';

type PipelineNodeConfig = {
  label: string;
  subtitle?: string;
  kind: NodeKind;
};

const NODE_HORIZONTAL_SPACING = 200;
const NODE_VERTICAL_SPACING = 170;

const BASE_STYLE: CSSProperties = {
  padding: 14,
  borderRadius: 18,
  fontWeight: 600,
  display: 'flex',
  flexDirection: 'column',
  gap: 4,
  boxShadow: '0 12px 24px rgba(15, 23, 42, 0.12)',
  border: '1px solid transparent',
};

const STYLE_MAP: Record<NodeKind, CSSProperties> = {
  entry: {
    ...BASE_STYLE,
    background: '#ecfdf5',
    borderColor: '#34d399',
    color: '#047857',
  },
  agent: {
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
};

const PIPELINE_NODE_CONFIG: Record<string, PipelineNodeConfig> = {
  classifier: {
    label: 'ClassifyQueryType()',
    subtitle: 'Agent',
    kind: 'agent',
  },
  decision: {
    label: 'If / Else',
    subtitle: 'classification == QueryType.GetFlightInfo',
    kind: 'decision',
  },
  flightAgent: {
    label: 'FetchFlightInfo()',
    kind: 'agent',
  },
  itineraryAgent: {
    label: 'GenerateVacationItinerary()',
    kind: 'agent',
  },
};

const NODE_LAYOUT: Record<string, { row: number; col: number }> = {
  classifier: { row: 1, col: 0 },
  decision: { row: 2, col: 0 },
  flightAgent: { row: 3, col: -1 },
  itineraryAgent: { row: 3, col: 1 },
};

const AgentBuilderDemoInner: React.FC = () => {
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
      id: 'start-classifier',
      source: 'start',
      target: 'classifier',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#10b981' },
      style: { stroke: '#10b981', strokeWidth: 2 },
    },
    {
      id: 'classifier-decision',
      source: 'classifier',
      target: 'decision',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#6366f1' },
      style: { stroke: '#6366f1', strokeWidth: 2 },
    },
    {
      id: 'decision-flight',
      source: 'decision',
      target: 'flightAgent',
      type: 'smoothstep',
      markerEnd: { type: MarkerType.ArrowClosed, color: '#f59e0b' },
      style: { stroke: '#f59e0b', strokeWidth: 2 },
      label: 'Then',
      labelStyle: { fill: '#b45309', fontWeight: 600 },
      labelBgStyle: { fill: '#fff7ed', fillOpacity: 0.8, stroke: '#fde68a' },
    },
    {
      id: 'decision-itinerary',
      source: 'decision',
      target: 'itineraryAgent',
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
    <div style={{ height: '100%', width: '100%', background: '#f9fafb' }}>
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

const AgentBuilderDemoTab: React.FC = () => (
  <ReactFlowProvider>
    <AgentBuilderDemoInner />
  </ReactFlowProvider>
);

export default AgentBuilderDemoTab;
