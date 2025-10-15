/*
Sibling-only edges demo: this variant keeps the hierarchical layout from the ELK example,
but restricts edges so that every connection links nodes (or subgraphs) that share the
same parent. That lets us compare how the graph reads when cross-level edges are removed.
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
      border: '2px solid #2563eb',
      background: 'rgba(37, 99, 235, 0.1)',
      fontWeight: 600,
    },
  },
  {
    id: 'A2',
    data: { label: 'A2 (subgraph)' },
    position: { x: 0, y: 0 },
    style: {
      width: 360,
      height: 280,
      padding: 16,
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      borderRadius: 18,
      border: '2px solid #1d4ed8',
      background: 'rgba(191, 219, 254, 0.65)',
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
      border: '1px solid #1d4ed8',
      background: '#fff',
      boxShadow: '0 4px 12px rgba(37, 99, 235, 0.15)',
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
      border: '1px solid #1d4ed8',
      background: '#fff',
      boxShadow: '0 4px 12px rgba(37, 99, 235, 0.15)',
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
      border: '1px solid #1d4ed8',
      background: '#fff',
      boxShadow: '0 4px 12px rgba(37, 99, 235, 0.15)',
      fontWeight: 600,
    },
  },
  {
    id: 'A3',
    data: { label: 'A3 (subgraph)' },
    position: { x: 0, y: 0 },
    style: {
      width: 440,
      height: 360,
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
    data: { label: 'C1 (subgraph)' },
    position: { x: 0, y: 0 },
    parentId: 'A3',
    extent: 'parent',
    style: {
      width: 240,
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
    id: 'D1',
    data: { label: 'D1' },
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
      border: '1px solid rgba(14, 165, 233, 0.75)',
      background: '#f1f5f9',
      boxShadow: '0 4px 10px rgba(14, 165, 233, 0.18)',
      fontWeight: 600,
    },
  },
  {
    id: 'D2',
    data: { label: 'D2' },
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
      border: '1px solid rgba(14, 165, 233, 0.75)',
      background: '#f1f5f9',
      boxShadow: '0 4px 10px rgba(14, 165, 233, 0.18)',
      fontWeight: 600,
    },
  },
  {
    id: 'C2',
    data: { label: 'C2 (subgraph)' },
    position: { x: 0, y: 0 },
    parentId: 'A3',
    extent: 'parent',
    style: {
      width: 260,
      height: 240,
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
    parentId: 'C2',
    extent: 'parent',
    style: {
      width: 170,
      height: 52,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid rgba(14, 165, 233, 0.75)',
      background: '#f8fafc',
      boxShadow: '0 4px 10px rgba(14, 165, 233, 0.18)',
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
      width: 170,
      height: 52,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid rgba(14, 165, 233, 0.75)',
      background: '#f8fafc',
      boxShadow: '0 4px 10px rgba(14, 165, 233, 0.18)',
      fontWeight: 600,
    },
  },
  {
    id: 'C3',
    data: { label: 'C3 (leaf node)' },
    position: { x: 0, y: 0 },
    parentId: 'A3',
    extent: 'parent',
    style: {
      width: 200,
      height: 72,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 16,
      border: '1px dashed rgba(14, 165, 233, 0.7)',
      background: 'rgba(224, 242, 254, 0.6)',
      boxShadow: '0 6px 16px rgba(14, 165, 233, 0.18)',
      fontWeight: 600,
      fontStyle: 'italic',
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
      border: '2px solid #2563eb',
      background: 'rgba(37, 99, 235, 0.1)',
      fontWeight: 600,
    },
  },
];

const edges: Edge[] = [
  { id: 'A1-A2', source: 'A1', target: 'A2', animated: true },
  { id: 'A2-A3', source: 'A2', target: 'A3', animated: true },
  { id: 'A3-A4', source: 'A3', target: 'A4', animated: true },
  { id: 'B1-B2', source: 'B1', target: 'B2' },
  { id: 'B2-B3', source: 'B2', target: 'B3' },
  { id: 'C1-C3', source: 'C1', target: 'C3' },
  { id: 'C3-C2', source: 'C3', target: 'C2' },
  { id: 'D1-D2', source: 'D1', target: 'D2' },
  { id: 'D3-D4', source: 'D3', target: 'D4' },
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

export default function CustomGraphSiblingEdges() {
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
            fontFamily: 'var(--font-geist-sans, system-ui)',
            fontSize: 13,
            lineHeight: 1.6,
          }}
        >
          <h2 style={{ margin: '0 0 10px', fontSize: 16, fontWeight: 600 }}>Sibling-only wiring</h2>
          <p style={{ margin: '0 0 12px' }}>
            Compare this view to the hierarchical ELK demo: top-level nodes still chain together, but every nested edge
            now links siblings. No child bubbles connect directly to their grandparents, so the flow within each
            container is isolated.
          </p>
          <div style={{ fontSize: 12, color: 'rgba(15, 23, 42, 0.75)' }}>
            <strong>Same-level sequences</strong>
            <ul style={{ margin: '8px 0 0 16px', padding: 0 }}>
              <li>
                A: A1 {'->'} A2 {'->'} A3 {'->'} A4
              </li>
              <li>
                B (inside A2): B1 {'->'} B2 {'->'} B3
              </li>
              <li>
                C (inside A3): C1 {'->'} C3 {'->'} C2
              </li>
              <li>D (inside C1): D1 {'->'} D2</li>
              <li>D (inside C2): D3 {'->'} D4</li>
            </ul>
          </div>
          <details style={{ marginTop: 16 }}>
            <summary style={{ cursor: 'pointer', userSelect: 'none' }}>ELK input graph</summary>
            <pre style={{ margin: '8px 0 0', whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
              {elkGraph ? JSON.stringify(elkGraph, null, 2) : 'computing layout...'}
            </pre>
          </details>
        </aside>
        <div style={{ flex: 1 }}>
          <ReactFlow nodes={nodes} edges={edges} fitView minZoom={0.2} defaultEdgeOptions={{ type: 'smoothstep' }}>
            <MiniMap maskColor="rgba(15, 23, 42, 0.08)" nodeStrokeColor="rgba(37, 99, 235, 0.9)" />
            <Controls showInteractive={false} />
            <Background gap={24} size={1} color="rgba(148, 163, 184, 0.35)" />
          </ReactFlow>
        </div>
      </div>
    </div>
  );
}
