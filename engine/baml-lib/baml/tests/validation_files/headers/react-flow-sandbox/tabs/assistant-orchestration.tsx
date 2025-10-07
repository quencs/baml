import React, { useMemo } from 'react';
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

const AssistantFlowInner: React.FC = () => {
  const initialNodes = useMemo<Node[]>(
    () => [
      {
        id: 'user-message',
        data: { label: 'User Message' },
        position: { x: 0, y: 80 },
        type: 'input',
        style: {
          padding: 12,
          borderRadius: 12,
          background: '#fef3c7',
          border: '1px solid #f59e0b',
          fontWeight: 600,
        },
      },
      {
        id: 'intent-detect',
        data: { label: 'Intent Detection' },
        position: { x: 220, y: 0 },
        style: {
          padding: 12,
          borderRadius: 12,
          background: '#ede9fe',
          border: '1px solid #8b5cf6',
          fontWeight: 600,
        },
      },
      {
        id: 'planner',
        data: { label: 'Planner' },
        position: { x: 220, y: 160 },
        style: {
          padding: 12,
          borderRadius: 12,
          background: '#e0f2fe',
          border: '1px solid #38bdf8',
          fontWeight: 600,
        },
      },
      {
        id: 'tool-router',
        data: { label: 'Tool Router' },
        position: { x: 440, y: 80 },
        style: {
          padding: 12,
          borderRadius: 12,
          background: '#fef9c3',
          border: '1px solid #eab308',
          fontWeight: 600,
        },
      },
      {
        id: 'tool-exec',
        data: { label: 'Tool Execution' },
        position: { x: 660, y: 0 },
        style: {
          padding: 12,
          borderRadius: 12,
          background: '#dcfce7',
          border: '1px solid #22c55e',
          fontWeight: 600,
        },
      },
      {
        id: 'response',
        data: { label: 'Response Synthesizer' },
        position: { x: 880, y: 60 },
        type: 'output',
        style: {
          padding: 12,
          borderRadius: 12,
          background: '#f1f5f9',
          border: '1px solid #64748b',
          fontWeight: 600,
        },
      },
      {
        id: 'reflection',
        data: { label: 'Reflection Loop' },
        position: { x: 660, y: 160 },
        style: {
          padding: 12,
          borderRadius: 12,
          background: '#fee2e2',
          border: '1px solid #f87171',
          fontWeight: 600,
        },
      },
      {
        id: 'evaluation',
        data: { label: 'Policy Evaluation' },
        position: { x: 440, y: 220 },
        style: {
          padding: 12,
          borderRadius: 12,
          background: '#cffafe',
          border: '1px solid #06b6d4',
          fontWeight: 600,
        },
      },
    ],
    []
  );

  const initialEdges = useMemo<Edge[]>(
    () => [
      {
        id: 'e-user-intent',
        source: 'user-message',
        target: 'intent-detect',
        type: 'smoothstep',
        animated: true,
        style: { stroke: '#f59e0b' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#f59e0b' },
      },
      {
        id: 'e-user-planner',
        source: 'user-message',
        target: 'planner',
        type: 'smoothstep',
        style: { stroke: '#38bdf8', strokeDasharray: '6 3' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#38bdf8' },
      },
      {
        id: 'e-intent-router',
        source: 'intent-detect',
        target: 'tool-router',
        type: 'smoothstep',
        style: { stroke: '#8b5cf6' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#8b5cf6' },
      },
      {
        id: 'e-planner-router',
        source: 'planner',
        target: 'tool-router',
        type: 'smoothstep',
        style: { stroke: '#38bdf8' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#38bdf8' },
      },
      {
        id: 'e-router-exec',
        source: 'tool-router',
        target: 'tool-exec',
        type: 'smoothstep',
        animated: true,
        style: { stroke: '#eab308' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#eab308' },
      },
      {
        id: 'e-exec-response',
        source: 'tool-exec',
        target: 'response',
        type: 'smoothstep',
        style: { stroke: '#22c55e' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#22c55e' },
      },
      {
        id: 'e-exec-reflection',
        source: 'tool-exec',
        target: 'reflection',
        type: 'smoothstep',
        style: { stroke: '#f87171' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#f87171' },
      },
      {
        id: 'e-reflection-planner',
        source: 'reflection',
        target: 'planner',
        type: 'smoothstep',
        style: { stroke: '#f87171', strokeDasharray: '6 3' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#f87171' },
      },
      {
        id: 'e-planner-evaluation',
        source: 'planner',
        target: 'evaluation',
        type: 'smoothstep',
        style: { stroke: '#06b6d4' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#06b6d4' },
      },
      {
        id: 'e-evaluation-response',
        source: 'evaluation',
        target: 'response',
        type: 'smoothstep',
        style: { stroke: '#06b6d4', strokeDasharray: '6 3' },
        markerEnd: { type: MarkerType.ArrowClosed, color: '#06b6d4' },
      },
    ],
    []
  );

  const [nodes, , onNodesChange] = useNodesState(initialNodes);
  const [edges, , onEdgesChange] = useEdgesState(initialEdges);

  return (
    <div style={{ height: '100%', width: '100%', background: '#f9fafb' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
        fitViewOptions={{ padding: 0.18 }}
        panOnScroll
        selectionOnDrag
        nodeOrigin={[0.5, 0.5]}
        minZoom={0.4}
        maxZoom={1.4}
      >
        <Background gap={24} size={1} color="#e2e8f0" />
        <Controls position="top-left" />
      </ReactFlow>
    </div>
  );
};

const AssistantOrchestrationTab: React.FC = () => (
  <ReactFlowProvider>
    <AssistantFlowInner />
  </ReactFlowProvider>
);

export default AssistantOrchestrationTab;
