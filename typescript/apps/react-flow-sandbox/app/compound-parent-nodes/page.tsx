/*
This example shows the shortest path to mimicking a Mermaid "subgraph" by using the
compound-node support that ships with React Flow. We model each graph element as a node
object, then declare directed edges between node ids. The special trick is that node A2
acts as a container: B1, B2, and B3 set `parentId: 'A2'` and `extent: 'parent'`, so they
render inside the bounds of A2 and move together when the subgraph is dragged. We extend
the same idea one layer deeper: A3 wraps C1 and C2, and each C node wraps its own D nodes,
building a stack A1 → A2 {B} → A3 {C {D}} → A4. Because the children still expose their own
ids, we can mix connections such as `A1 -> A2`, `A1 -> B1`, `B1 -> B2`, or `C1 -> D1`, which
mirrors how Mermaid allows edges both to a subgraph and to the nodes inside it. In this
version the positions are not hard-coded; ELK (a layered graph layout engine) computes
coordinates for the entire hierarchy, so you can scale the structure without touching any
x/y numbers.
*/
'use client';

import { useEffect, useState } from 'react';
import { ReactFlow, Background, Controls, MiniMap, type Edge, type Node } from '@xyflow/react';
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

const baseNodes: Node[] = [
  {
    id: 'A1',
    data: { label: 'A1' },
    position: { x: 0, y: 0 },
    style: {
      width: 140,
      height: 60,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '2px solid #2563eb',
      background: 'rgba(59, 130, 246, 0.1)',
    },
  },
  {
    id: 'A2',
    data: { label: 'A2 (subgraph)' },
    position: { x: 0, y: 0 },
    style: {
      width: 340,
      height: 360,
      padding: 12,
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      borderRadius: 16,
      border: '2px solid #1d4ed8',
      background: 'rgba(219, 234, 254, 0.7)',
      fontWeight: 600,
    },
  },
  {
    id: 'A3',
    data: { label: 'A3 (subgraph)' },
    position: { x: 0, y: 0 },
    style: {
      width: 420,
      height: 460,
      padding: 16,
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      borderRadius: 18,
      border: '2px solid #1d4ed8',
      background: 'rgba(191, 219, 254, 0.6)',
      fontWeight: 600,
    },
  },
  {
    id: 'A4',
    data: { label: 'A4' },
    position: { x: 0, y: 0 },
    style: {
      width: 140,
      height: 60,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '2px solid #2563eb',
      background: 'rgba(59, 130, 246, 0.1)',
    },
  },
  {
    id: 'B1',
    data: { label: 'B1' },
    position: { x: 0, y: 0 },
    parentId: 'A2',
    extent: 'parent',
    style: {
      width: 200,
      height: 56,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid #1d4ed8',
      background: '#fff',
      boxShadow: '0 4px 10px rgba(59, 130, 246, 0.12)',
    },
  },
  {
    id: 'B2',
    data: { label: 'B2' },
    position: { x: 0, y: 0 },
    parentId: 'A2',
    extent: 'parent',
    style: {
      width: 200,
      height: 56,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid #1d4ed8',
      background: '#fff',
      boxShadow: '0 4px 10px rgba(59, 130, 246, 0.12)',
    },
  },
  {
    id: 'B3',
    data: { label: 'B3' },
    position: { x: 0, y: 0 },
    parentId: 'A2',
    extent: 'parent',
    style: {
      width: 200,
      height: 56,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid #1d4ed8',
      background: '#fff',
      boxShadow: '0 4px 10px rgba(59, 130, 246, 0.12)',
    },
  },
  {
    id: 'C1',
    data: { label: 'C1' },
    position: { x: 0, y: 0 },
    parentId: 'A3',
    extent: 'parent',
    style: {
      width: 280,
      height: 240,
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      borderRadius: 14,
      border: '1px solid #1d4ed8',
      background: '#fff',
      padding: 10,
      boxShadow: '0 6px 14px rgba(59, 130, 246, 0.12)',
      fontWeight: 600,
    },
  },
  {
    id: 'C2',
    data: { label: 'C2' },
    position: { x: 0, y: 0 },
    parentId: 'A3',
    extent: 'parent',
    style: {
      width: 280,
      height: 260,
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      borderRadius: 14,
      border: '1px solid #1d4ed8',
      background: '#fff',
      padding: 10,
      boxShadow: '0 6px 14px rgba(59, 130, 246, 0.12)',
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
      width: 200,
      height: 48,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid rgba(37, 99, 235, 0.6)',
      background: '#f8fafc',
      boxShadow: '0 4px 12px rgba(59, 130, 246, 0.12)',
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
      width: 200,
      height: 48,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid rgba(37, 99, 235, 0.6)',
      background: '#f8fafc',
      boxShadow: '0 4px 12px rgba(59, 130, 246, 0.12)',
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
      width: 200,
      height: 48,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid rgba(37, 99, 235, 0.6)',
      background: '#f8fafc',
      boxShadow: '0 4px 12px rgba(59, 130, 246, 0.12)',
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
      width: 200,
      height: 48,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid rgba(37, 99, 235, 0.6)',
      background: '#f8fafc',
      boxShadow: '0 4px 12px rgba(59, 130, 246, 0.12)',
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
      width: 200,
      height: 48,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 12,
      border: '1px solid rgba(37, 99, 235, 0.6)',
      background: '#f8fafc',
      boxShadow: '0 4px 12px rgba(59, 130, 246, 0.12)',
      fontWeight: 600,
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

function getNodeSize(node: Node) {
  const width = typeof node.style?.width === 'number' ? node.style.width : 180;
  const height = typeof node.style?.height === 'number' ? node.style.height : 60;
  return { width, height };
}

async function layoutNodes(
  nodes: Node[],
  graphEdges: Edge[],
  captureGraph?: (graph: Record<string, unknown>) => void,
) {
  const topLevelNodes = nodes.filter((node) => !node.parentId);

  const buildElkNode = (node: Node, allNodes: Node[]): Record<string, unknown> => {
    const { width, height } = getNodeSize(node);
    const childNodes = allNodes.filter((candidate) => candidate.parentId === node.id);

    const elkNode: Record<string, unknown> = {
      id: node.id,
      width,
      height,
    };

    if (childNodes.length > 0) {
      elkNode.layoutOptions = {
        'elk.direction': 'DOWN',
        'org.eclipse.elk.layered.spacing.nodeNodeBetweenLayers': '48',
      };
      elkNode.children = childNodes.map((child) => buildElkNode(child, allNodes));
    }

    return elkNode;
  };

  const elkGraph = {
    id: 'root',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': 'RIGHT',
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'org.eclipse.elk.layered.spacing.nodeNodeBetweenLayers': '80',
      'org.eclipse.elk.spacing.componentComponent': '80',
    },
    children: topLevelNodes.map((node) => buildElkNode(node, nodes)),
    edges: graphEdges.map((edge) => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  };

  if (captureGraph) {
    captureGraph(cloneGraph(elkGraph));
  }

  const layout = await elk.layout(elkGraph);
  const positions: Record<string, { x: number; y: number }> = {};

  function assignPositions(node: Record<string, any>, parentOffset = { x: 0, y: 0 }) {
    if (!node.id) {
      return;
    }

    const x = (node.x ?? 0) + parentOffset.x;
    const y = (node.y ?? 0) + parentOffset.y;
    positions[node.id] = { x, y };

    if (Array.isArray(node.children)) {
      node.children.forEach((child) => assignPositions(child, { x, y }));
    }
  }

  layout.children?.forEach((child) => assignPositions(child));

  return nodes.map((node) => {
    const layoutPosition = positions[node.id];

    if (!layoutPosition) {
      return node;
    }

    if (node.parentId) {
      const parentPosition = positions[node.parentId];
      if (parentPosition) {
        return {
          ...node,
          position: {
            x: layoutPosition.x - parentPosition.x,
            y: layoutPosition.y - parentPosition.y,
          },
        };
      }
    }

    return {
      ...node,
      position: layoutPosition,
    };
  });
}

export default function CompoundSubgraphDemo() {
  const [nodes, setNodes] = useState<Node[]>(baseNodes);
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
            background: 'rgba(15, 23, 42, 0.03)',
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
            <MiniMap />
            <Controls />
            <Background gap={18} />
          </ReactFlow>
        </div>
      </div>
    </div>
  );
}
