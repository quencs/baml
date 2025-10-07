import React, { CSSProperties, useEffect, useMemo } from 'react';
import {
  ReactFlow,
  ReactFlowProvider,
  Node,
  Edge,
  MarkerType,
  useNodesState,
  useEdgesState,
  Background,
  MiniMap,
  Controls,
} from '@xyflow/react';
import { hierarchy, tree } from 'd3-hierarchy';

type NodeStyleKey = 'source' | 'process' | 'consumer';

type PipelineNodeConfig = {
  label: string;
  type?: Node['type'];
  styleKey: NodeStyleKey;
};

type LayoutSpec = {
  id: string;
  children?: LayoutSpec[];
};

const NODE_HORIZONTAL_SPACING = 220;
const NODE_VERTICAL_SPACING = 120;

const NODE_STYLE_BASE: CSSProperties = {
  padding: 12,
  borderRadius: 8,
  fontWeight: 600,
};

const NODE_STYLE_SOURCE: CSSProperties = {
  ...NODE_STYLE_BASE,
  background: '#ecfeff',
  border: '1px solid #38bdf8',
};

const NODE_STYLE_PROCESS: CSSProperties = {
  ...NODE_STYLE_BASE,
  background: '#ede9fe',
  border: '1px solid #8b5cf6',
};

const NODE_STYLE_CONSUMER: CSSProperties = {
  ...NODE_STYLE_BASE,
  background: '#dcfce7',
  border: '1px solid #22c55e',
};

const NODE_STYLE_MAP: Record<NodeStyleKey, CSSProperties> = {
  source: NODE_STYLE_SOURCE,
  process: NODE_STYLE_PROCESS,
  consumer: NODE_STYLE_CONSUMER,
};

const PIPELINE_NODE_CONFIG: Record<string, PipelineNodeConfig> = {
  'source-db': {
    label: 'Source DB',
    type: 'input',
    styleKey: 'source',
  },
  'stream-ingest': {
    label: 'Streaming Ingest',
    styleKey: 'process',
  },
  'batch-ingest': {
    label: 'Batch Loader',
    styleKey: 'process',
  },
  'quality-gate': {
    label: 'Data Quality Gate',
    styleKey: 'process',
  },
  warehouse: {
    label: 'Analytics Warehouse',
    styleKey: 'process',
  },
  dashboards: {
    label: 'BI Dashboards',
    type: 'output',
    styleKey: 'consumer',
  },
  alerts: {
    label: 'Operational Alerts',
    type: 'output',
    styleKey: 'consumer',
  },
  catalog: {
    label: 'Data Catalog',
    styleKey: 'consumer',
  },
};

const LAYOUT_SPEC: LayoutSpec = {
  id: 'source-db',
  children: [
    {
      id: 'stream-ingest',
      children: [
        {
          id: 'quality-gate',
          children: [
            {
              id: 'warehouse',
              children: [
                { id: 'dashboards' },
                { id: 'alerts' },
                { id: 'catalog' },
              ],
            },
          ],
        },
      ],
    },
    {
      id: 'batch-ingest',
      children: [
        {
          id: 'quality-gate',
          children: [
            {
              id: 'warehouse',
              children: [
                { id: 'dashboards' },
                { id: 'alerts' },
                { id: 'catalog' },
              ],
            },
          ],
        },
      ],
    },
  ],
};

const DataLineageFlowInner: React.FC = () => {
  const layoutPositions = useMemo(() => {
    const root = hierarchy(LAYOUT_SPEC, (d: LayoutSpec) => d.children);
    tree<LayoutSpec>()
      .nodeSize([NODE_VERTICAL_SPACING, NODE_HORIZONTAL_SPACING])
      .separation((a: any, b: any) => (a.parent === b.parent ? 1 : 1.2))(root);

    const aggregated = new Map<string, { x: number; y: number; count: number }>();
    root.descendants().forEach((node: any) => {
      const { id } = node.data;
      const current = aggregated.get(id);
      if (current) {
        current.x += node.x;
        current.y += node.y;
        current.count += 1;
      } else {
        aggregated.set(id, { x: node.x, y: node.y, count: 1 });
      }
    });

    if (aggregated.size === 0) return new Map<string, { x: number; y: number }>();

    let minX = Infinity;
    let minY = Infinity;
    aggregated.forEach(({ x, y, count }) => {
      const avgX = x / count;
      const avgY = y / count;
      if (avgX < minX) minX = avgX;
      if (avgY < minY) minY = avgY;
    });

    const normalized = new Map<string, { x: number; y: number }>();
    aggregated.forEach(({ x, y, count }, id) => {
      const avgX = x / count;
      const avgY = y / count;
      normalized.set(id, {
        x: avgX - minX,
        y: avgY - minY,
      });
    });

    return normalized;
  }, []);

  const initialNodes = useMemo<Node[]>(() => {
    let fallbackIndex = 0;
    return Object.entries(PIPELINE_NODE_CONFIG).map(([id, config]) => {
      const coords = layoutPositions.get(id);
      const x = coords ? coords.y : fallbackIndex * NODE_HORIZONTAL_SPACING;
      const y = coords ? coords.x : fallbackIndex * NODE_VERTICAL_SPACING;
      fallbackIndex += coords ? 0 : 1;

      return {
        id,
        data: { label: config.label },
        type: config.type,
        position: { x, y },
        style: { ...NODE_STYLE_MAP[config.styleKey] },
      } satisfies Node;
    });
  }, [layoutPositions]);

  const initialEdges = useMemo<Edge[]>(
    () => [
      {
        id: 'e-source-stream',
        source: 'source-db',
        target: 'stream-ingest',
        type: 'smoothstep',
        animated: true,
        style: { stroke: '#0ea5e9' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#0ea5e9' },
      },
      {
        id: 'e-source-batch',
        source: 'source-db',
        target: 'batch-ingest',
        type: 'smoothstep',
        style: { stroke: '#eab308' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#eab308' },
      },
      {
        id: 'e-stream-quality',
        source: 'stream-ingest',
        target: 'quality-gate',
        type: 'smoothstep',
        style: { stroke: '#3b82f6' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#3b82f6' },
      },
      {
        id: 'e-batch-quality',
        source: 'batch-ingest',
        target: 'quality-gate',
        type: 'smoothstep',
        style: { stroke: '#eab308' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#eab308' },
      },
      {
        id: 'e-quality-warehouse',
        source: 'quality-gate',
        target: 'warehouse',
        type: 'smoothstep',
        animated: true,
        style: { stroke: '#f87171' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#f87171' },
      },
      {
        id: 'e-warehouse-dashboards',
        source: 'warehouse',
        target: 'dashboards',
        type: 'smoothstep',
        style: { stroke: '#6366f1' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#6366f1' },
      },
      {
        id: 'e-warehouse-alerts',
        source: 'warehouse',
        target: 'alerts',
        type: 'smoothstep',
        style: { stroke: '#22c55e' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#22c55e' },
      },
      {
        id: 'e-warehouse-catalog',
        source: 'warehouse',
        target: 'catalog',
        type: 'smoothstep',
        style: { stroke: '#06b6d4' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#06b6d4' },
      },
      {
        id: 'e-catalog-quality',
        source: 'catalog',
        target: 'quality-gate',
        type: 'smoothstep',
        style: { stroke: '#06b6d4', strokeDasharray: '6 3' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#06b6d4' },
      },
    ],
    []
  );

  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, , onEdgesChange] = useEdgesState(initialEdges);

  useEffect(() => {
    setNodes(initialNodes);
  }, [initialNodes, setNodes]);

  return (
    <div style={{ height: '100%', width: '100%', background: '#f8fafc' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        panOnScroll
        zoomOnScroll
        nodeOrigin={[0.5, 0.5]}
        minZoom={0.4}
        maxZoom={1.4}
      >
        <Background gap={32} color="#e2e8f0" />
        <MiniMap
          pannable
          zoomable
          nodeBorderRadius={6}
          nodeStrokeColor={(node) => (node.type === 'input' ? '#0ea5e9' : node.type === 'output' ? '#6366f1' : '#64748b')}
          nodeColor={(node) => (node.type === 'input' ? '#bae6fd' : node.type === 'output' ? '#ddd6fe' : '#e2e8f0')}
        />
        <Controls position="top-left" showInteractive={false} />
      </ReactFlow>
    </div>
  );
};

const DataLineageTab: React.FC = () => (
  <ReactFlowProvider>
    <DataLineageFlowInner />
  </ReactFlowProvider>
);

export default DataLineageTab;
