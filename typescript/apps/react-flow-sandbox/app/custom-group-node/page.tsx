/*
This route illustrates a fully custom "group node" abstraction for readers unfamiliar with
graph rendering. React Flow normally renders simple rectangular nodes from the `data` field.
Here we replace the default node renderer for A2 and A3 with `GroupNode`, a bespoke component
that draws labeled containers and manually positions inner rows. Each row exposes React Flow
handles, so we can attach edges like `A1 -> B1`, `B1 -> B2`, `C1 -> D1`, or `D4 -> D5` even
though the visuals live inside a single React Flow node. The implementation feeds the entire
hierarchy through ELK (a layout engine) so that the top-level nodes (A1, A2, A3, A4), the
B-members inside A2, and the nested C/D groups inside A3 all receive sensible coordinates
without any hard-coded x/y pairs. We convert ELK's coordinates into pixel offsets that the
custom renderer understands, leaving React Flow to handle edges, panning, and hit-testing.
*/
'use client';

import { Fragment, useEffect, useState } from 'react';
import {
  Background,
  Controls,
  Handle,
  MiniMap,
  Position,
  ReactFlow,
  type Edge,
  type Node,
  type NodeProps,
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

const GROUP_WIDTH = 360;
const OUTER_CONTENT_OFFSET = 96;
const OUTER_PADDING = 48;
const NESTED_CONTENT_OFFSET = 52;
const NESTED_PADDING = 28;
const HANDLE_COLOR = '#7c3aed';

const BASE_NODE_STYLE = {
  width: 140,
  height: 60,
  borderRadius: 12,
  border: '2px solid #7c3aed',
  background: 'rgba(243, 232, 255, 0.8)',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  fontWeight: 600,
  color: '#581c87',
} as const;

type GroupMember = {
  id: string;
  label: string;
  top: number;
  height: number;
};

type NestedGroup = {
  id: string;
  label: string;
  top: number;
  height: number;
  members: GroupMember[];
};

type GroupNodeData = {
  label: string;
  members?: GroupMember[];
  groups?: NestedGroup[];
};

function computeContainerHeight(members: GroupMember[], groups: NestedGroup[]) {
  const bottoms = [
    ...members.map((member) => member.top + member.height),
    ...groups.map((group) => group.top + group.height),
  ];

  const tallest = bottoms.length > 0 ? Math.max(...bottoms) : OUTER_CONTENT_OFFSET + 60;
  return tallest + OUTER_PADDING;
}

function GroupNode({ data }: NodeProps<GroupNodeData>) {
  const members = data.members ?? [];
  const groups = data.groups ?? [];
  const containerHeight = computeContainerHeight(members, groups);

  return (
    <div
      style={{
        width: GROUP_WIDTH,
        height: containerHeight,
        borderRadius: 18,
        border: '2px solid #9333ea',
        background: 'rgba(233, 213, 255, 0.6)',
        position: 'relative',
        padding: '24px 20px 20px',
        boxShadow: '0 16px 40px rgba(147, 51, 234, 0.12)',
      }}
    >
      <div style={{ fontWeight: 700, color: '#581c87', fontSize: 18, marginBottom: 20 }}>{data.label}</div>
      <Handle id="group-in" type="target" position={Position.Left} style={{ top: 48, background: HANDLE_COLOR }} />
      <Handle id="group-out" type="source" position={Position.Right} style={{ top: 48, background: HANDLE_COLOR }} />

      {members.map((member) => (
        <div
          key={member.id}
          style={{
            position: 'absolute',
            left: 32,
            right: 32,
            top: member.top,
            height: member.height,
            borderRadius: 12,
            border: '1px solid rgba(59, 7, 100, 0.4)',
            background: '#fdf4ff',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontWeight: 600,
            color: '#3b0764',
            boxShadow: '0 8px 18px rgba(192, 132, 252, 0.25)',
          }}
        >
          {member.label}
        </div>
      ))}

      {members.map((member) => {
        const handleTop = member.top + member.height / 2;
        return (
          <Fragment key={member.id}>
            <Handle
              id={`${member.id}-in`}
              type="target"
              position={Position.Left}
              style={{ top: handleTop, background: HANDLE_COLOR }}
            />
            <Handle
              id={`${member.id}-out`}
              type="source"
              position={Position.Right}
              style={{ top: handleTop, background: HANDLE_COLOR }}
            />
          </Fragment>
        );
      })}

      {groups.map((group) => {
        const groupHandleTop = group.top + 26;

        return (
          <Fragment key={group.id}>
            <div
              style={{
                position: 'absolute',
                left: 28,
                right: 28,
                top: group.top,
                height: group.height,
                borderRadius: 14,
                border: '1px solid rgba(88, 28, 135, 0.3)',
                background: 'rgba(237, 233, 254, 0.55)',
                boxShadow: '0 10px 30px rgba(147, 51, 234, 0.18)',
                padding: '18px 16px 16px',
              }}
            >
              <div style={{ fontWeight: 700, color: '#5b21b6', marginBottom: 14 }}>{group.label}</div>

              {group.members.map((member) => (
                <div
                  key={member.id}
                  style={{
                    position: 'absolute',
                    left: 16,
                    right: 16,
                    top: member.top,
                    height: member.height,
                    borderRadius: 12,
                    border: '1px solid rgba(91, 33, 182, 0.35)',
                    background: '#faf5ff',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    fontWeight: 600,
                    color: '#4c1d95',
                    boxShadow: '0 6px 16px rgba(139, 92, 246, 0.25)',
                  }}
                >
                  {member.label}
                </div>
              ))}
            </div>

            <Handle
              id={`${group.id}-in`}
              type="target"
              position={Position.Left}
              style={{ top: groupHandleTop, background: HANDLE_COLOR }}
            />
            <Handle
              id={`${group.id}-out`}
              type="source"
              position={Position.Right}
              style={{ top: groupHandleTop, background: HANDLE_COLOR }}
            />

            {group.members.map((member) => {
              const handleTop = group.top + member.top + member.height / 2;
              return (
                <Fragment key={member.id}>
                  <Handle
                    id={`${member.id}-in`}
                    type="target"
                    position={Position.Left}
                    style={{ top: handleTop, background: HANDLE_COLOR }}
                  />
                  <Handle
                    id={`${member.id}-out`}
                    type="source"
                    position={Position.Right}
                    style={{ top: handleTop, background: HANDLE_COLOR }}
                  />
                </Fragment>
              );
            })}
          </Fragment>
        );
      })}
    </div>
  );
}

const layoutNodesDefinition: Node[] = [
  {
    id: 'A1',
    data: { label: 'A1' },
    position: { x: 0, y: 0 },
    style: { width: 140, height: 60 },
  },
  {
    id: 'A2',
    data: { label: 'A2' },
    position: { x: 0, y: 0 },
    style: { width: GROUP_WIDTH, height: 420 },
  },
  {
    id: 'A3',
    data: { label: 'A3' },
    position: { x: 0, y: 0 },
    style: { width: GROUP_WIDTH, height: 520 },
  },
  {
    id: 'A4',
    data: { label: 'A4' },
    position: { x: 0, y: 0 },
    style: { width: 140, height: 60 },
  },
  {
    id: 'B1',
    data: { label: 'B1' },
    position: { x: 0, y: 0 },
    parentId: 'A2',
    extent: 'parent',
    style: { width: 220, height: 54 },
  },
  {
    id: 'B2',
    data: { label: 'B2' },
    position: { x: 0, y: 0 },
    parentId: 'A2',
    extent: 'parent',
    style: { width: 220, height: 54 },
  },
  {
    id: 'B3',
    data: { label: 'B3' },
    position: { x: 0, y: 0 },
    parentId: 'A2',
    extent: 'parent',
    style: { width: 220, height: 54 },
  },
  {
    id: 'C1',
    data: { label: 'C1' },
    position: { x: 0, y: 0 },
    parentId: 'A3',
    extent: 'parent',
    style: { width: 300, height: 240 },
  },
  {
    id: 'C2',
    data: { label: 'C2' },
    position: { x: 0, y: 0 },
    parentId: 'A3',
    extent: 'parent',
    style: { width: 300, height: 260 },
  },
  {
    id: 'D1',
    data: { label: 'D1' },
    position: { x: 0, y: 0 },
    parentId: 'C1',
    extent: 'parent',
    style: { width: 220, height: 48 },
  },
  {
    id: 'D2',
    data: { label: 'D2' },
    position: { x: 0, y: 0 },
    parentId: 'C1',
    extent: 'parent',
    style: { width: 220, height: 48 },
  },
  {
    id: 'D3',
    data: { label: 'D3' },
    position: { x: 0, y: 0 },
    parentId: 'C2',
    extent: 'parent',
    style: { width: 220, height: 48 },
  },
  {
    id: 'D4',
    data: { label: 'D4' },
    position: { x: 0, y: 0 },
    parentId: 'C2',
    extent: 'parent',
    style: { width: 220, height: 48 },
  },
  {
    id: 'D5',
    data: { label: 'D5' },
    position: { x: 0, y: 0 },
    parentId: 'C2',
    extent: 'parent',
    style: { width: 220, height: 48 },
  },
];

const layoutEdges = [
  { id: 'layout-A1-A2', sources: ['A1'], targets: ['A2'] },
  { id: 'layout-A1-B1', sources: ['A1'], targets: ['B1'] },
  { id: 'layout-B1-B2', sources: ['B1'], targets: ['B2'] },
  { id: 'layout-B2-B3', sources: ['B2'], targets: ['B3'] },
  { id: 'layout-A2-A3', sources: ['A2'], targets: ['A3'] },
  { id: 'layout-A3-C1', sources: ['A3'], targets: ['C1'] },
  { id: 'layout-C1-D1', sources: ['C1'], targets: ['D1'] },
  { id: 'layout-D1-D2', sources: ['D1'], targets: ['D2'] },
  { id: 'layout-A3-C2', sources: ['A3'], targets: ['C2'] },
  { id: 'layout-C2-D3', sources: ['C2'], targets: ['D3'] },
  { id: 'layout-D3-D4', sources: ['D3'], targets: ['D4'] },
  { id: 'layout-D4-D5', sources: ['D4'], targets: ['D5'] },
  { id: 'layout-A3-A4', sources: ['A3'], targets: ['A4'] },
];

type ElkNodeResult = {
  x: number;
  y: number;
  width: number;
  height: number;
  parentId?: string;
};

function getNodeSize(node: Node) {
  const width = typeof node.style?.width === 'number' ? node.style.width : 180;
  const height = typeof node.style?.height === 'number' ? node.style.height : 60;
  return { width, height };
}

function buildChildrenMap(nodes: Node[]) {
  const map = new Map<string, Node[]>();

  nodes.forEach((node) => {
    if (!node.parentId) {
      return;
    }

    const siblings = map.get(node.parentId) ?? [];
    siblings.push(node);
    map.set(node.parentId, siblings);
  });

  return map;
}

const childrenMap = buildChildrenMap(layoutNodesDefinition);

function buildElkNode(node: Node, allNodes: Node[]): Record<string, unknown> {
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
}

async function createLayout(
  captureGraph?: (graph: Record<string, unknown>) => void,
) {
  const topLevelNodes = layoutNodesDefinition.filter((node) => !node.parentId);

  const elkGraph = {
    id: 'root',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': 'DOWN',
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'org.eclipse.elk.layered.spacing.nodeNodeBetweenLayers': '96',
      'org.eclipse.elk.spacing.componentComponent': '96',
    },
    children: topLevelNodes.map((node) => buildElkNode(node, layoutNodesDefinition)),
    edges: layoutEdges,
  } satisfies Parameters<typeof elk.layout>[0];

  if (captureGraph) {
    captureGraph(cloneGraph(elkGraph));
  }

  const layout = await elk.layout(elkGraph);
  const positions: Record<string, ElkNodeResult> = {};

  const assignPositions = (elkNode: any, parent?: string, offset = { x: 0, y: 0 }) => {
    if (!elkNode?.id) {
      return;
    }

    const x = (elkNode.x ?? 0) + offset.x;
    const y = (elkNode.y ?? 0) + offset.y;

    positions[elkNode.id] = {
      x,
      y,
      width: elkNode.width ?? 0,
      height: elkNode.height ?? 0,
      parentId: parent,
    };

    if (Array.isArray(elkNode.children)) {
      elkNode.children.forEach((child: any) => assignPositions(child, elkNode.id, { x, y }));
    }
  };

  layout.children?.forEach((child) => assignPositions(child));
  return positions;
}

const edges: Edge[] = [
  { id: 'edge-A1-A2', source: 'A1', target: 'A2', targetHandle: 'group-in', animated: true },
  { id: 'edge-A1-B1', source: 'A1', target: 'A2', targetHandle: 'B1-in' },
  { id: 'edge-B1-B2', source: 'A2', sourceHandle: 'B1-out', target: 'A2', targetHandle: 'B2-in' },
  { id: 'edge-B2-B3', source: 'A2', sourceHandle: 'B2-out', target: 'A2', targetHandle: 'B3-in' },
  { id: 'edge-A2-A3', source: 'A2', sourceHandle: 'group-out', target: 'A3', targetHandle: 'group-in' },
  { id: 'edge-A3-C1', source: 'A3', sourceHandle: 'group-out', target: 'A3', targetHandle: 'C1-in' },
  { id: 'edge-C1-D1', source: 'A3', sourceHandle: 'C1-out', target: 'A3', targetHandle: 'D1-in' },
  { id: 'edge-D1-D2', source: 'A3', sourceHandle: 'D1-out', target: 'A3', targetHandle: 'D2-in' },
  { id: 'edge-A3-C2', source: 'A3', sourceHandle: 'group-out', target: 'A3', targetHandle: 'C2-in' },
  { id: 'edge-C2-D3', source: 'A3', sourceHandle: 'C2-out', target: 'A3', targetHandle: 'D3-in' },
  { id: 'edge-D3-D4', source: 'A3', sourceHandle: 'D3-out', target: 'A3', targetHandle: 'D4-in' },
  { id: 'edge-D4-D5', source: 'A3', sourceHandle: 'D4-out', target: 'A3', targetHandle: 'D5-in' },
  { id: 'edge-A3-A4', source: 'A3', sourceHandle: 'group-out', target: 'A4' },
];

function buildGroupData(positions: Record<string, ElkNodeResult>) {
  const membersA2 = (childrenMap.get('A2') ?? []).map((node) => {
    const parent = positions['A2'];
    const layoutNode = positions[node.id];

    if (!parent || !layoutNode) {
      return { id: node.id, label: node.data?.label ?? node.id, top: OUTER_CONTENT_OFFSET, height: 54 };
    }

    return {
      id: node.id,
      label: node.data?.label ?? node.id,
      top: layoutNode.y - parent.y + OUTER_CONTENT_OFFSET,
      height: layoutNode.height || 54,
    };
  });

  const groupsA3: NestedGroup[] = (childrenMap.get('A3') ?? [])
    .filter((node) => node.id === 'C1' || node.id === 'C2')
    .map((node) => {
      const parent = positions['A3'];
      const layoutNode = positions[node.id];

      if (!parent || !layoutNode) {
        return {
          id: node.id,
          label: node.data?.label ?? node.id,
          top: OUTER_CONTENT_OFFSET,
          height: 220,
          members: [],
        } satisfies NestedGroup;
      }

      const nestedMembers = (childrenMap.get(node.id) ?? []).map((child) => {
        const childLayout = positions[child.id];

        if (!childLayout) {
          return {
            id: child.id,
            label: child.data?.label ?? child.id,
            top: NESTED_CONTENT_OFFSET,
            height: 48,
          } satisfies GroupMember;
        }

        return {
          id: child.id,
          label: child.data?.label ?? child.id,
          top: childLayout.y - layoutNode.y + NESTED_CONTENT_OFFSET,
          height: childLayout.height || 48,
        } satisfies GroupMember;
      });

      const memberBottoms = nestedMembers.map((member) => member.top + member.height);
      const nestedHeight = (memberBottoms.length > 0 ? Math.max(...memberBottoms) : NESTED_CONTENT_OFFSET + 48) + NESTED_PADDING;

      return {
        id: node.id,
        label: node.data?.label ?? node.id,
        top: layoutNode.y - parent.y + OUTER_CONTENT_OFFSET,
        height: nestedHeight,
        members: nestedMembers,
      } satisfies NestedGroup;
    });

  return { membersA2, groupsA3 };
}

const nodeTypes = {
  groupNode: GroupNode,
};

export default function CustomGroupNodeDemo() {
  const [nodes, setNodes] = useState<Node<GroupNodeData | { label: string }>[]>([]);
  const [elkInput, setElkInput] = useState<Record<string, unknown> | null>(null);

  useEffect(() => {
    void (async () => {
      const positions = await createLayout(setElkInput);
      const { membersA2, groupsA3 } = buildGroupData(positions);

      const a2Height = computeContainerHeight(membersA2, []);
      const a3Height = computeContainerHeight([], groupsA3);

      const nextNodes: Node<GroupNodeData | { label: string }>[] = [
        {
          id: 'A1',
          data: { label: 'A1' },
          position: { x: positions['A1']?.x ?? 0, y: positions['A1']?.y ?? 0 },
          style: BASE_NODE_STYLE,
        },
        {
          id: 'A2',
          type: 'groupNode',
          data: { label: 'A2 (custom group)', members: membersA2 },
          position: { x: positions['A2']?.x ?? 0, y: positions['A2']?.y ?? 0 },
          style: { width: GROUP_WIDTH, height: a2Height },
        },
        {
          id: 'A3',
          type: 'groupNode',
          data: { label: 'A3 (custom group)', groups: groupsA3 },
          position: { x: positions['A3']?.x ?? 0, y: positions['A3']?.y ?? 0 },
          style: { width: GROUP_WIDTH, height: a3Height },
        },
        {
          id: 'A4',
          data: { label: 'A4' },
          position: { x: positions['A4']?.x ?? 0, y: positions['A4']?.y ?? 0 },
          style: BASE_NODE_STYLE,
        },
      ];

      setNodes(nextNodes);
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
            background: 'rgba(76, 29, 149, 0.05)',
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
          <ReactFlow nodes={nodes} edges={edges} nodeTypes={nodeTypes} fitView>
            <MiniMap />
            <Controls />
            <Background gap={18} />
          </ReactFlow>
        </div>
      </div>
    </div>
  );
}
