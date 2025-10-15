/*
This variation keeps the underlying graph completely flat—every node (A1, A2, B1-B3, C1, C2,
D1-D5) is a peer—and instead relies on metadata to signal grouping. Each inner node records
the subgraphs it belongs to (e.g. B nodes list `A2`, D1 lists both `A3` and `C1`), and the
`GroupOverlay` helper inspects React Flow's internal store to collect nodes with a matching
tag, measure their on-screen positions, and draw dashed highlight boxes. Because we never
alter the edge list, connections like `A1 -> B1`, `B1 -> B2`, `C1 -> D1`, or `D4 -> D5` work
exactly as in any other flat graph. The visual affordance, however, gives users the same
mental model as nested Mermaid subgraphs—A2 wraps the B nodes, A3 wraps C1/C2, and the C
groups wrap their D descendants—without resorting to actual parent-child node relationships.
To avoid hand-tuned coordinates we run the graph through ELK, a layout engine that positions
all nodes based on the edges while we merely supply widths/heights.
*/
'use client';

import type { CSSProperties } from 'react';
import { useEffect, useState } from 'react';
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  useStore,
  type Edge,
  type Node,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import ELK from 'elkjs/lib/elk.bundled';

import { HeaderNav } from '../components/HeaderNav';

const elk = new ELK();

const cloneGraph = <T>(graph: T): T => {
  if (typeof structuredClone === 'function') {
    return structuredClone(graph);
  }

  return JSON.parse(JSON.stringify(graph)) as T;
};

type GroupedNodeData = {
  label: string;
  groups?: string[];
};

const baseNodes: Node<GroupedNodeData>[] = [
  {
    id: 'A1',
    data: { label: 'A1' },
    position: { x: 0, y: 0 },
    style: {
      width: 150,
      height: 60,
      borderRadius: 12,
      border: '2px solid #f59e0b',
      background: 'rgba(254, 240, 138, 0.6)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 600,
      color: '#92400e',
    },
  },
  {
    id: 'A2',
    data: { label: 'A2 (metadata group)', groups: ['A2'] },
    position: { x: 0, y: 0 },
    style: {
      width: 340,
      height: 360,
      borderRadius: 18,
      border: '2px dashed #f97316',
      background: 'rgba(254, 215, 170, 0.35)',
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      fontWeight: 700,
      color: '#b45309',
      padding: '16px 20px',
      boxSizing: 'border-box',
      zIndex: 0,
    },
  },
  {
    id: 'A3',
    data: { label: 'A3 (metadata group)', groups: ['A3'] },
    position: { x: 0, y: 0 },
    style: {
      width: 420,
      height: 440,
      borderRadius: 18,
      border: '2px dashed #f97316',
      background: 'rgba(254, 215, 170, 0.25)',
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      fontWeight: 700,
      color: '#b45309',
      padding: '16px 20px',
      boxSizing: 'border-box',
      zIndex: 0,
    },
  },
  {
    id: 'A4',
    data: { label: 'A4' },
    position: { x: 0, y: 0 },
    style: {
      width: 150,
      height: 60,
      borderRadius: 12,
      border: '2px solid #f59e0b',
      background: 'rgba(254, 240, 138, 0.6)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 600,
      color: '#92400e',
    },
  },
  {
    id: 'B1',
    data: { label: 'B1', groups: ['A2'] },
    position: { x: 0, y: 0 },
    style: {
      width: 190,
      height: 56,
      borderRadius: 12,
      border: '1px solid rgba(180, 83, 9, 0.6)',
      background: '#fff7ed',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 600,
      color: '#b45309',
      boxShadow: '0 10px 25px rgba(249, 115, 22, 0.18)',
      zIndex: 1,
    },
  },
  {
    id: 'B2',
    data: { label: 'B2', groups: ['A2'] },
    position: { x: 0, y: 0 },
    style: {
      width: 190,
      height: 56,
      borderRadius: 12,
      border: '1px solid rgba(180, 83, 9, 0.6)',
      background: '#fff7ed',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 600,
      color: '#b45309',
      boxShadow: '0 10px 25px rgba(249, 115, 22, 0.18)',
      zIndex: 1,
    },
  },
  {
    id: 'B3',
    data: { label: 'B3', groups: ['A2'] },
    position: { x: 0, y: 0 },
    style: {
      width: 190,
      height: 56,
      borderRadius: 12,
      border: '1px solid rgba(180, 83, 9, 0.6)',
      background: '#fff7ed',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 600,
      color: '#b45309',
      boxShadow: '0 10px 25px rgba(249, 115, 22, 0.18)',
      zIndex: 1,
    },
  },
  {
    id: 'C1',
    data: { label: 'C1', groups: ['A3', 'C1'] },
    position: { x: 0, y: 0 },
    style: {
      width: 260,
      height: 200,
      borderRadius: 16,
      border: '1px solid rgba(217, 119, 6, 0.55)',
      background: '#fffbeb',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 700,
      color: '#c2410c',
      boxShadow: '0 12px 26px rgba(217, 119, 6, 0.18)',
    },
  },
  {
    id: 'C2',
    data: { label: 'C2', groups: ['A3', 'C2'] },
    position: { x: 0, y: 0 },
    style: {
      width: 260,
      height: 220,
      borderRadius: 16,
      border: '1px solid rgba(217, 119, 6, 0.55)',
      background: '#fffbeb',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 700,
      color: '#c2410c',
      boxShadow: '0 12px 26px rgba(217, 119, 6, 0.18)',
    },
  },
  {
    id: 'D1',
    data: { label: 'D1', groups: ['A3', 'C1'] },
    position: { x: 0, y: 0 },
    style: {
      width: 180,
      height: 48,
      borderRadius: 12,
      border: '1px solid rgba(217, 119, 6, 0.45)',
      background: '#fff7ed',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 600,
      color: '#b45309',
      boxShadow: '0 10px 24px rgba(249, 115, 22, 0.18)',
    },
  },
  {
    id: 'D2',
    data: { label: 'D2', groups: ['A3', 'C1'] },
    position: { x: 0, y: 0 },
    style: {
      width: 180,
      height: 48,
      borderRadius: 12,
      border: '1px solid rgba(217, 119, 6, 0.45)',
      background: '#fff7ed',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 600,
      color: '#b45309',
      boxShadow: '0 10px 24px rgba(249, 115, 22, 0.18)',
    },
  },
  {
    id: 'D3',
    data: { label: 'D3', groups: ['A3', 'C2'] },
    position: { x: 0, y: 0 },
    style: {
      width: 180,
      height: 48,
      borderRadius: 12,
      border: '1px solid rgba(217, 119, 6, 0.45)',
      background: '#fff7ed',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 600,
      color: '#b45309',
      boxShadow: '0 10px 24px rgba(249, 115, 22, 0.18)',
    },
  },
  {
    id: 'D4',
    data: { label: 'D4', groups: ['A3', 'C2'] },
    position: { x: 0, y: 0 },
    style: {
      width: 180,
      height: 48,
      borderRadius: 12,
      border: '1px solid rgba(217, 119, 6, 0.45)',
      background: '#fff7ed',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 600,
      color: '#b45309',
      boxShadow: '0 10px 24px rgba(249, 115, 22, 0.18)',
    },
  },
  {
    id: 'D5',
    data: { label: 'D5', groups: ['A3', 'C2'] },
    position: { x: 0, y: 0 },
    style: {
      width: 180,
      height: 48,
      borderRadius: 12,
      border: '1px solid rgba(217, 119, 6, 0.45)',
      background: '#fff7ed',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontWeight: 600,
      color: '#b45309',
      boxShadow: '0 10px 24px rgba(249, 115, 22, 0.18)',
    },
  },
];

const edges: Edge[] = [
  { id: 'A1-A2', source: 'A1', target: 'A2', animated: true },
  { id: 'A1-B1', source: 'A1', target: 'B1' },
  { id: 'B1-B2', source: 'B1', target: 'B2' },
  { id: 'B2-B3', source: 'B2', target: 'B3' },
  { id: 'A2-A3', source: 'A2', target: 'A3' },
  { id: 'A3-C1', source: 'A3', target: 'C1' },
  { id: 'C1-D1', source: 'C1', target: 'D1' },
  { id: 'D1-D2', source: 'D1', target: 'D2' },
  { id: 'A3-C2', source: 'A3', target: 'C2' },
  { id: 'C2-D3', source: 'C2', target: 'D3' },
  { id: 'D3-D4', source: 'D3', target: 'D4' },
  { id: 'D4-D5', source: 'D4', target: 'D5' },
  { id: 'A3-A4', source: 'A3', target: 'A4' },
];

function getNodeSize(node: Node<GroupedNodeData>) {
  const width = typeof node.style?.width === 'number' ? node.style.width : 180;
  const height = typeof node.style?.height === 'number' ? node.style.height : 60;
  return { width, height };
}

async function layoutNodes(
  nodes: Node<GroupedNodeData>[],
  graphEdges: Edge[],
  captureGraph?: (graph: Record<string, unknown>) => void,
) {
  const elkGraph = {
    id: 'root',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': 'DOWN',
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'org.eclipse.elk.layered.spacing.nodeNodeBetweenLayers': '80',
      'org.eclipse.elk.spacing.componentComponent': '80',
    },
    children: nodes.map((node) => {
      const { width, height } = getNodeSize(node);
      return {
        id: node.id,
        width,
        height,
      };
    }),
    edges: graphEdges.map((edge) => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  } satisfies Parameters<typeof elk.layout>[0];

  if (captureGraph) {
    captureGraph(cloneGraph(elkGraph));
  }

  const layout = await elk.layout(elkGraph);

  const positioned = nodes.map((node) => {
    const layoutNode = layout.children?.find((child) => child.id === node.id);

    if (!layoutNode) {
      return node;
    }

    return {
      ...node,
      position: {
        x: layoutNode.x ?? 0,
        y: layoutNode.y ?? 0,
      },
    };
  });

  return positioned;
}

type Bounds = { x: number; y: number; width: number; height: number };

type GroupOverlayProps = {
  groupId: string;
  label: string;
  accent: string;
  zIndex?: number;
};

function GroupOverlay({ groupId, label, accent, zIndex = 0 }: GroupOverlayProps) {
  const bounds = useStore((state) => {
    const map = state.nodeInternals;

    if (!map || map.size === 0) {
      return null;
    }

    const internalNodes = Array.from(map.values()).filter((node) => {
      if (!node.data) {
        return false;
      }

      const data = node.data as GroupedNodeData;
      const groups = Array.isArray(data.groups)
        ? data.groups
        : 'groupId' in data && typeof (data as any).groupId === 'string'
          ? [(data as any).groupId as string]
          : [];

      return groups.includes(groupId);
    });

    if (internalNodes.length === 0) {
      return null;
    }

    const dimensionsReady = internalNodes.every(
      (node) => typeof node.width === 'number' && typeof node.height === 'number',
    );

    if (!dimensionsReady) {
      return null;
    }

    const minX = Math.min(
      ...internalNodes.map((node) => (node.positionAbsolute?.x ?? node.position.x)),
    );
    const minY = Math.min(
      ...internalNodes.map((node) => (node.positionAbsolute?.y ?? node.position.y)),
    );
    const maxX = Math.max(
      ...internalNodes.map(
        (node) => (node.positionAbsolute?.x ?? node.position.x) + (node.width ?? 0),
      ),
    );
    const maxY = Math.max(
      ...internalNodes.map(
        (node) => (node.positionAbsolute?.y ?? node.position.y) + (node.height ?? 0),
      ),
    );

    const padding = 28;

    const rawBounds: Bounds = {
      x: minX - padding,
      y: minY - padding,
      width: maxX - minX + padding * 2,
      height: maxY - minY + padding * 2,
    };

    return rawBounds;
  });

  const transform = useStore((state) => state.transform);

  if (!bounds) {
    return null;
  }

  const [translateX, translateY, zoom] = transform;

  const style: CSSProperties = {
    left: bounds.x * zoom + translateX,
    top: bounds.y * zoom + translateY,
    width: bounds.width * zoom,
    height: bounds.height * zoom,
  };

  return (
    <div
      style={{
        position: 'absolute',
        pointerEvents: 'none',
        border: `2px dashed ${accent}`,
        borderRadius: 18,
        background: `${accent}14`,
        boxShadow: `0 12px 32px ${accent}29`,
        zIndex,
        ...style,
      }}
    >
      <div
        style={{
          position: 'absolute',
          top: -28,
          left: 24,
          padding: '4px 12px',
          borderRadius: 9999,
          border: `1px solid ${accent}66`,
          background: '#fff7ed',
          fontWeight: 700,
          color: accent,
        }}
      >
        Subgraph {label}
      </div>
    </div>
  );
}

export default function MetadataOverlayDemo() {
  const [nodes, setNodes] = useState<Node<GroupedNodeData>[]>(baseNodes);
  const [elkInput, setElkInput] = useState<Record<string, unknown> | null>(null);

  useEffect(() => {
    void (async () => {
      const layouted = await layoutNodes(baseNodes, edges, setElkInput);
      setNodes(layouted);
    })();
  }, []);

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <HeaderNav />
      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        <aside
          style={{
            width: '33%',
            minWidth: 280,
            maxWidth: 420,
            borderRight: '1px solid rgba(148, 163, 184, 0.35)',
            padding: '16px',
            overflowY: 'auto',
            background: 'rgba(249, 115, 22, 0.04)',
            fontFamily: 'var(--font-geist-mono, ui-monospace)',
            fontSize: 12,
            lineHeight: 1.5,
          }}
        >
          <h2 style={{ margin: '0 0 12px', fontSize: 14, fontWeight: 600 }}>ELK Input Graph</h2>
          <pre style={{ margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
            {elkInput ? JSON.stringify(elkInput, null, 2) : 'computing layout…'}
          </pre>
        </aside>
        <div style={{ flex: 1 }}>
          <ReactFlow nodes={nodes} edges={edges} fitView>
            <GroupOverlay groupId="A2" label="A2" accent="#f97316" zIndex={1} />
            <GroupOverlay groupId="A3" label="A3" accent="#d97706" zIndex={0} />
            <GroupOverlay groupId="C1" label="C1" accent="#ea580c" zIndex={2} />
            <GroupOverlay groupId="C2" label="C2" accent="#ea580c" zIndex={2} />
            <MiniMap />
            <Controls />
            <Background gap={18} />
          </ReactFlow>
        </div>
      </div>
    </div>
  );
}
