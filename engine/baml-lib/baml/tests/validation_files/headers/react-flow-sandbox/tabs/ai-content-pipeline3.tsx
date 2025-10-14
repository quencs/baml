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
  MiniMap,
  Controls,
} from '@xyflow/react';

type NodeStyleKey = 'source' | 'logic' | 'generation' | 'outcome';

type PipelineNodeConfig = {
  label: string;
  type?: Node['type'];
  description?: string;
  styleKey: NodeStyleKey;
};

const NODE_HORIZONTAL_SPACING = 210;
const NODE_VERTICAL_SPACING = 120;

const BASE_STYLE: CSSProperties = {
  padding: 12,
  borderRadius: 10,
  fontWeight: 600,
  boxShadow: '0 6px 18px rgba(15, 23, 42, 0.08)',
};

const STYLE_SOURCE: CSSProperties = {
  ...BASE_STYLE,
  background: '#eff6ff',
  border: '1px solid #3b82f6',
};

const STYLE_LOGIC: CSSProperties = {
  ...BASE_STYLE,
  background: '#fef3c7',
  border: '1px solid #f59e0b',
};

const STYLE_GENERATION: CSSProperties = {
  ...BASE_STYLE,
  background: '#ede9fe',
  border: '1px solid #8b5cf6',
};

const STYLE_OUTCOME: CSSProperties = {
  ...BASE_STYLE,
  background: '#dcfce7',
  border: '1px solid #22c55e',
};

const STYLE_MAP: Record<NodeStyleKey, CSSProperties> = {
  source: STYLE_SOURCE,
  logic: STYLE_LOGIC,
  generation: STYLE_GENERATION,
  outcome: STYLE_OUTCOME,
};

const PIPELINE_NODE_CONFIG: Record<string, PipelineNodeConfig> = {
  'ai-content-pipeline': {
    label: 'AIContentPipeline()',
    type: 'input',
    styleKey: 'source',
  },
  'get-emails': {
    label: 'GetEmails()',
    styleKey: 'source',
  },
  'get-posts': {
    label: 'GetPosts()',
    styleKey: 'source',
  },
  'process-video': {
    label: 'ProcessVideo()',
    styleKey: 'source',
  },
  'get-video': {
    label: 'GetVideo()',
    styleKey: 'source',
  },
  'summary-decision': {
    label: 'Has Transcript?',
    styleKey: 'logic',
    description: 'Branch depending on transcript availability',
  },
  'no-transcript': {
    label: '"no-transcript"',
    styleKey: 'outcome',
    description: 'Fallback summary string when transcript missing',
  },
  'summarize-video': {
    label: 'Generate Content (click to expand)',
    styleKey: 'generation',
  },
  'label-summary': {
    label: 'LabelSummaryCategory()',
    styleKey: 'generation',
  },
  'save-summary': {
    label: 'SaveSummary()',
    styleKey: 'generation',
  },
  'content-loop': {
    label: 'ForEach Content Type',
    styleKey: 'logic',
    description: 'Iterate email/x/linkedin drafts',
  },
  'get-structure': {
    label: 'GetStructure()',
    styleKey: 'generation',
  },
  'generate-content': {
    label: 'GenerateContent()',
    styleKey: 'generation',
  },
  'save-draft': {
    label: 'SaveDraft()',
    styleKey: 'generation',
  },
  'pr-decision': {
    label: 'Summary Exists?',
    styleKey: 'logic',
    description: 'Gate for PR creation',
  },
  'create-pr': {
    label: 'CreatePR()',
    styleKey: 'generation',
  },
  'no-pr': {
    label: '"no-pr"',
    styleKey: 'outcome',
    description: 'Status when PR is skipped',
  },
  done: {
    label: '"done"',
    type: 'output',
    styleKey: 'outcome',
  },
};

const NODE_LAYOUT: Record<string, { row: number; col: number }> = {
  'ai-content-pipeline': { row: 0, col: 0 },
  'get-emails': { row: 1, col: 0 },
  'get-posts': { row: 2, col: 0 },
  'process-video': { row: 3, col: 0 },
  'get-video': { row: 4, col: 0 },
  'summarize-video': { row: 6, col: 0 },
  'content-loop': { row: 9, col: 0 },
  'get-structure': { row: 10, col: 0 },
  'generate-content': { row: 11, col: 0 },
  'save-draft': { row: 12, col: 0 },
  'pr-decision': { row: 13, col: 0 },
  'no-pr': { row: 13, col: 1 },
  'create-pr': { row: 14, col: 0 },
  done: { row: 15, col: 0 },
};

const AIContentPipelineInner: React.FC = () => {
  const initialNodes = useMemo<Node[]>(() => {
    return Object.entries(PIPELINE_NODE_CONFIG).map(([id, config]) => {
      const layout = NODE_LAYOUT[id] ?? { row: 0, col: 0 };
      const x = layout.col * NODE_HORIZONTAL_SPACING;
      const y = layout.row * NODE_VERTICAL_SPACING;

      const nodeData = config.description
        ? { label: config.label, description: config.description }
        : { label: config.label };

      return {
        id,
        data: nodeData,
        type: config.type,
        position: { x, y },
        style: { ...STYLE_MAP[config.styleKey] },
      } satisfies Node;
    });
  }, []);

  const initialEdges = useMemo<Edge[]>(() => {
    const MAIN_SEQUENCE: string[] = [
      'ai-content-pipeline',
      'get-emails',
      'get-posts',
      'process-video',
      'get-video',
      'summarize-video',
      'content-loop',
      'get-structure',
      'generate-content',
      'save-draft',
      'pr-decision',
      'create-pr',
      'done',
    ];

    const sequenceEdges: Edge[] = MAIN_SEQUENCE.slice(0, -1).map((sourceId, index) => {
      const targetId = MAIN_SEQUENCE[index + 1];
      return {
        id: `seq-${sourceId}-${targetId}`,
        source: sourceId,
        target: targetId,
        type: 'smoothstep',
        markerEnd: { type: MarkerType.ArrowClosed, color: '#2563eb' },
        style: { stroke: '#2563eb', strokeWidth: 2 },
      } satisfies Edge;
    });

    const branchEdges: Edge[] = [
      {
        id: 'branch-summary-no-transcript',
        source: 'summary-decision',
        target: 'no-transcript',
        type: 'smoothstep',
        markerEnd: { type: MarkerType.ArrowClosed, color: '#f97316' },
        style: { stroke: '#f97316', strokeDasharray: '6 3' },
      },
      {
        id: 'branch-no-transcript-loop',
        source: 'no-transcript',
        target: 'content-loop',
        type: 'smoothstep',
        markerEnd: { type: MarkerType.ArrowClosed, color: '#f59e0b' },
        style: { stroke: '#f59e0b', strokeDasharray: '6 3' },
      },
      {
        id: 'branch-pr-no-pr',
        source: 'pr-decision',
        target: 'no-pr',
        type: 'smoothstep',
        markerEnd: { type: MarkerType.ArrowClosed, color: '#f97316' },
        style: { stroke: '#f97316', strokeDasharray: '6 3' },
      },
      {
        id: 'branch-no-pr-done',
        source: 'no-pr',
        target: 'done',
        type: 'smoothstep',
        markerEnd: { type: MarkerType.ArrowClosed, color: '#10b981' },
        style: { stroke: '#10b981', strokeDasharray: '6 3' },
      },
      {
        id: 'loop-save-draft-content-loop',
        source: 'save-draft',
        target: 'content-loop',
        type: 'smoothstep',
        animated: true,
        markerEnd: { type: MarkerType.ArrowClosed, color: '#f59e0b' },
        style: { stroke: '#f59e0b', strokeDasharray: '4 3' },
      },
    ];

    return [...sequenceEdges, ...branchEdges];
  }, []);

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
        fitViewOptions={{ padding: 0.24 }}
        panOnScroll
        selectionOnDrag
        nodeOrigin={[0.5, 0.5]}
        minZoom={0.4}
        maxZoom={1.4}
      >
        <Background gap={28} size={1} color="#e2e8f0" />
        <MiniMap
          pannable
          zoomable
          nodeBorderRadius={8}
          nodeStrokeColor={(node) => {
            if (node.type === 'input') return '#2563eb';
            if (node.type === 'output') return '#16a34a';
            return '#7c3aed';
          }}
          nodeColor={(node) => {
            if (node.type === 'input') return '#bfdbfe';
            if (node.type === 'output') return '#bbf7d0';
            return '#ddd6fe';
          }}
        />
        <Controls position="top-left" />
      </ReactFlow>
    </div>
  );
};

const AIContentPipelineTab: React.FC = () => (
  <ReactFlowProvider>
    <AIContentPipelineInner />
  </ReactFlowProvider>
);

export default AIContentPipelineTab;
