/*
Hierarchical ELK demo: we model the mermaid-like structure
A1 -> A2 {B1 B2 B3} -> A3 {C1 {D3}, C2 {D4 D5}} -> A4 with compound parent nodes.
ELK computes coordinates for every level so that React Flow renders nested subgraphs
without manual x/y tuning.
*/
'use client';

import { useEffect, useState } from 'react';
import { Background, Controls, MiniMap, ReactFlow, type Edge, type Node } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import ELK from 'elkjs/lib/elk.bundled';

import { HeaderNav } from '../components/HeaderNav';

const elk = new ELK();

const baseNodes: Node[] = [
  {
    id: 'A1',
    data: { label: 'A1' },
    position: { x: 0, y: 0 },
    style: {
      width: 160,
      height: 64,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '2px solid #0284c7',
      background: 'rgba(2, 132, 199, 0.12)',
      fontWeight: 600,
    },
  },
  {
    id: 'A2',
    data: { label: 'A2 · subgraph' },
    position: { x: 0, y: 0 },
    style: {
      width: 360,
      height: 280,
      padding: 16,
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      borderRadius: 18,
      border: '2px solid #0369a1',
      background: 'rgba(191, 219, 254, 0.7)',
      fontWeight: 600,
    },
  },
  {
    id: 'B1',
    data: { label: 'B1' },
    position: { x: 0, y: 0 },
    parentId: 'A2',
    extent: 'parent',
    style: {
      width: 190,
      height: 56,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid #0369a1',
      background: '#fff',
      boxShadow: '0 4px 12px rgba(3, 105, 161, 0.15)',
      fontWeight: 600,
    },
  },
  {
    id: 'B2',
    data: { label: 'B2' },
    position: { x: 0, y: 0 },
    parentId: 'A2',
    extent: 'parent',
    style: {
      width: 190,
      height: 56,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid #0369a1',
      background: '#fff',
      boxShadow: '0 4px 12px rgba(3, 105, 161, 0.15)',
      fontWeight: 600,
    },
  },
  {
    id: 'B3',
    data: { label: 'B3' },
    position: { x: 0, y: 0 },
    parentId: 'A2',
    extent: 'parent',
    style: {
      width: 190,
      height: 56,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid #0369a1',
      background: '#fff',
      boxShadow: '0 4px 12px rgba(3, 105, 161, 0.15)',
      fontWeight: 600,
    },
  },
  {
    id: 'A3',
    data: { label: 'A3 · subgraph' },
    position: { x: 0, y: 0 },
    style: {
      width: 430,
      height: 340,
      padding: 20,
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      borderRadius: 20,
      border: '2px solid #0ea5e9',
      background: 'rgba(125, 211, 252, 0.55)',
      fontWeight: 600,
    },
  },
  {
    id: 'C1',
    data: { label: 'C1 · subgraph' },
    position: { x: 0, y: 0 },
    parentId: 'A3',
    extent: 'parent',
    style: {
      width: 240,
      height: 200,
      padding: 12,
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      borderRadius: 14,
      border: '1px solid #0ea5e9',
      background: '#fff',
      boxShadow: '0 6px 16px rgba(14, 165, 233, 0.18)',
      fontWeight: 600,
    },
  },
  {
    id: 'D3',
    data: { label: 'D3' },
    position: { x: 0, y: 0 },
    parentId: 'C1',
    extent: 'parent',
    style: {
      width: 160,
      height: 48,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid rgba(14, 165, 233, 0.8)',
      background: '#f8fafc',
      boxShadow: '0 4px 10px rgba(14, 165, 233, 0.18)',
      fontWeight: 600,
    },
  },
  {
    id: 'C2',
    data: { label: 'C2 · subgraph' },
    position: { x: 0, y: 0 },
    parentId: 'A3',
    extent: 'parent',
    style: {
      width: 260,
      height: 220,
      padding: 12,
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      borderRadius: 14,
      border: '1px solid #0ea5e9',
      background: '#fff',
      boxShadow: '0 6px 16px rgba(14, 165, 233, 0.18)',
      fontWeight: 600,
    },
  },
  {
    id: 'D4',
    data: { label: 'D4' },
    position: { x: 0, y: 0 },
    parentId: 'C2',
    extent: 'parent',
    style: {
      width: 160,
      height: 48,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid rgba(14, 165, 233, 0.8)',
      background: '#f8fafc',
      boxShadow: '0 4px 10px rgba(14, 165, 233, 0.18)',
      fontWeight: 600,
    },
  },
  {
    id: 'D5',
    data: { label: 'D5' },
    position: { x: 0, y: 0 },
    parentId: 'C2',
    extent: 'parent',
    style: {
      width: 160,
      height: 48,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid rgba(14, 165, 233, 0.8)',
      background: '#f8fafc',
      boxShadow: '0 4px 10px rgba(14, 165, 233, 0.18)',
      fontWeight: 600,
    },
  },
  {
    id: 'A4',
    data: { label: 'A4' },
    position: { x: 0, y: 0 },
    style: {
      width: 160,
      height: 64,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '2px solid #0284c7',
      background: 'rgba(2, 132, 199, 0.12)',
      fontWeight: 600,
    },
  },
];

const edges: Edge[] = [
  { id: 'A1-A2', source: 'A1', target: 'A2', animated: true },
  { id: 'A2-B1', source: 'A2', target: 'B1' },
  { id: 'B1-B2', source: 'B1', target: 'B2' },
  { id: 'B2-B3', source: 'B2', target: 'B3' },
  { id: 'B3-A3', source: 'B3', target: 'A3' },
  { id: 'A3-C1', source: 'A3', target: 'C1' },
  { id: 'C1-D3', source: 'C1', target: 'D3' },
  { id: 'A3-C2', source: 'A3', target: 'C2' },
  { id: 'C2-D4', source: 'C2', target: 'D4' },
  { id: 'D4-D5', source: 'D4', target: 'D5' },
  { id: 'C2-A4', source: 'C2', target: 'A4' },
];

type ElkNode = {
  id: string;
  width: number;
  height: number;
  children?: ElkNode[];
  layoutOptions?: Record<string, string>;
};

type ElkGraph = {
  id: string;
  layoutOptions: Record<string, string>;
  children: ElkNode[];
  edges: { id: string; sources: string[]; targets: string[] }[];
};

function getNodeSize(node: Node) {
  const width = typeof node.style?.width === 'number' ? node.style.width : 180;
  const height = typeof node.style?.height === 'number' ? node.style.height : 60;
  return { width, height };
}

async function layoutWithElk(nodes: Node[], graphEdges: Edge[]) {
  const childrenByParent = new Map<string | null, Node[]>();
  nodes.forEach((node) => {
    const key = node.parentId ?? null;
    if (!childrenByParent.has(key)) {
      childrenByParent.set(key, []);
    }
    childrenByParent.get(key)!.push(node);
  });

  const buildElkNode = (node: Node): ElkNode => {
    const { width, height } = getNodeSize(node);
    const elkNode: ElkNode = { id: node.id, width, height };
    const childNodes = childrenByParent.get(node.id) ?? [];

    if (childNodes.length > 0) {
      elkNode.children = childNodes.map(buildElkNode);
      elkNode.layoutOptions = {
        'elk.direction': 'DOWN',
        'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
        'org.eclipse.elk.layered.spacing.nodeNodeBetweenLayers': '40',
      };
    }

    return elkNode;
  };

  const elkGraph: ElkGraph = {
    id: 'root',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': 'RIGHT',
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'org.eclipse.elk.spacing.componentComponent': '100',
      'org.eclipse.elk.layered.spacing.nodeNodeBetweenLayers': '70',
    },
    children: (childrenByParent.get(null) ?? []).map(buildElkNode),
    edges: graphEdges.map((edge) => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  };

  const layout = await elk.layout(elkGraph);

  const absolutePositions = new Map<string, { x: number; y: number }>();

  const assignPositions = (
    node: { id?: string; x?: number; y?: number; children?: unknown[] },
    offset: { x: number; y: number },
  ) => {
    if (!node.id) return;

    const x = (node.x ?? 0) + offset.x;
    const y = (node.y ?? 0) + offset.y;
    absolutePositions.set(node.id, { x, y });

    if (Array.isArray(node.children)) {
      node.children.forEach((child) => {
        assignPositions(child as { id?: string; x?: number; y?: number; children?: unknown[] }, { x, y });
      });
    }
  };

  layout.children?.forEach((child) => assignPositions(child, { x: 0, y: 0 }));

  const layoutedNodes = nodes.map((node) => {
    const absolute = absolutePositions.get(node.id);
    if (!absolute) {
      return node;
    }

    if (node.parentId) {
      const parentPosition = absolutePositions.get(node.parentId);
      if (parentPosition) {
        return {
          ...node,
          position: {
            x: absolute.x - parentPosition.x,
            y: absolute.y - parentPosition.y,
          },
        };
      }
    }

    return {
      ...node,
      position: absolute,
    };
  });

  return { layoutedNodes, elkGraph };
}

export default function HierarchicalElkLayout() {
  const [nodes, setNodes] = useState<Node[]>(baseNodes);
  const [elkGraph, setElkGraph] = useState<ElkGraph | null>(null);

  useEffect(() => {
    void (async () => {
      const { layoutedNodes, elkGraph } = await layoutWithElk(baseNodes, edges);
      setNodes(layoutedNodes);
      setElkGraph(elkGraph);
    })();
  }, []);

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <HeaderNav />
      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        <aside
          style={{
            width: '32%',
            minWidth: 260,
            maxWidth: 420,
            borderRight: '1px solid rgba(148, 163, 184, 0.35)',
            padding: '18px',
            overflowY: 'auto',
            background: 'rgba(15, 23, 42, 0.03)',
            fontFamily: 'var(--font-geist-mono, ui-monospace)',
            fontSize: 12,
            lineHeight: 1.5,
          }}
        >
          <h2 style={{ margin: '0 0 12px', fontSize: 14, fontWeight: 600 }}>ELK hierarchy</h2>
          <p style={{ margin: '0 0 16px', fontFamily: 'var(--font-geist-sans, system-ui)', fontSize: 13 }}>
            The layered layout flows left-to-right. Containers A2 and A3 host their child subgraphs, and ELK keeps
            nested offsets, so React Flow only needs the relative positions it emits.
          </p>
          <pre style={{ margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
            {elkGraph ? JSON.stringify(elkGraph, null, 2) : 'computing layout…'}
          </pre>
        </aside>
        <div style={{ flex: 1 }}>
          <ReactFlow nodes={nodes} edges={edges} fitView minZoom={0.2} defaultEdgeOptions={{ type: 'smoothstep' }}>
            <MiniMap maskColor="rgba(15, 23, 42, 0.08)" nodeStrokeColor="rgba(14, 165, 233, 0.8)" />
            <Controls showInteractive={false} />
            <Background gap={24} size={1} color="rgba(148, 163, 184, 0.35)" />
          </ReactFlow>
        </div>
      </div>
    </div>
  );
}
