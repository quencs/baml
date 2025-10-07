import React, { useState, useCallback, useEffect } from 'react';
import { Play, PlayCircle } from 'lucide-react';
import {
  ReactFlow,
  ReactFlowProvider,
  Node,
  Edge,
  MarkerType,
  useNodesState,
  useEdgesState,
  Position,
  Handle,
  useReactFlow,
} from '@xyflow/react';
import { hierarchy, tree } from 'd3-hierarchy';

interface TraceSpan {
  id: string;
  name: string;
  depth?: number;
  children?: TraceSpan[];
  input?: any;
  output?: any;
  latency?: number;
  cost?: number;
  metadata?: Record<string, any>;
}

interface TraceRun {
  id: string;
  timestamp: string;
  status: 'success' | 'error' | 'partial';
  totalLatency: number;
  totalCost: number;
  trace: TraceSpan;
}

interface TraceNodeData {
  span: TraceSpan;
  isRunning: boolean;
  isCompleted: boolean;
  isDivergent: boolean;
  isOriginalPath: boolean;
  onPlay: (span: TraceSpan, mode: 'single' | 'subtree') => void;
}

const mockTraceRuns: TraceRun[] = [
  {
    id: 'run-1',
    timestamp: '2025-09-30 14:23:45',
    status: 'success',
    totalLatency: 1250,
    totalCost: 0.0042,
    trace: {
      id: 'root-1',
      name: 'Root',
      depth: 0,
      input: { query: 'What is the weather today?', context: { user_id: '123' } },
      output: { result: 'The weather is sunny with a high of 75°F', confidence: 0.95 },
      latency: 1250,
      cost: 0.0042,
      metadata: { model: 'gpt-4', temperature: 0.7 },
      children: [
        {
          id: 'llma-1',
          name: 'ExtractIntent',
          depth: 1,
          input: { prompt: 'Extract weather query intent', text: 'What is the weather today?' },
          output: { intent: 'weather_query', location: 'current' },
          latency: 450,
          cost: 0.0015,
          metadata: { model: 'gpt-3.5-turbo', tokens: 125 },
        },
        {
          id: 'llmb-1',
          name: 'GetWeather',
          depth: 1,
          input: { intent: 'weather_query', location: 'current' },
          output: { weather: 'sunny', temperature: 75, unit: 'F' },
          latency: 800,
          cost: 0.0027,
          metadata: { model: 'gpt-4', tokens: 450 },
        },
      ],
    },
  },
  {
    id: 'run-2',
    timestamp: '2025-09-30 14:18:32',
    status: 'success',
    totalLatency: 2340,
    totalCost: 0.0089,
    trace: {
      id: 'root-2',
      name: 'Root',
      depth: 0,
      input: { query: 'Analyze this code for bugs', context: { language: 'python' } },
      output: {
        result: 'Found 3 potential issues',
        suggestions: ['Add try-catch', 'Remove unused var', 'Fix memory leak'],
      },
      latency: 2340,
      cost: 0.0089,
      metadata: { model: 'gpt-4', temperature: 0.3 },
      children: [
        {
          id: 'parse-2',
          name: 'ParseCode',
          depth: 1,
          input: { code: 'def process_data()...', language: 'python' },
          output: { ast: '...', symbols: ['process_data', 'data', 'unused_var', 'result'] },
          latency: 320,
          cost: 0.0008,
          metadata: { model: 'gpt-3.5-turbo', tokens: 180 },
        },
        {
          id: 'analyze-2',
          name: 'AnalyzeBugs',
          depth: 1,
          input: { ast: '...', symbols: ['process_data', 'data', 'unused_var', 'result'] },
          output: { issues: ['memory_leak', 'unused_variable', 'no_error_handling'] },
          latency: 1120,
          cost: 0.0045,
          metadata: { model: 'gpt-4', tokens: 890 },
          children: [
            {
              id: 'check-memory-2',
              name: 'CheckMemory',
              depth: 2,
              input: { ast: '...', focus: 'memory' },
              output: { found: true, location: 'line 2', severity: 'medium' },
              latency: 450,
              cost: 0.0018,
              metadata: { model: 'gpt-4', tokens: 320 },
            },
            {
              id: 'check-unused-2',
              name: 'CheckUnused',
              depth: 2,
              input: { symbols: ['process_data', 'data', 'unused_var', 'result'] },
              output: { unused: ['unused_var'] },
              latency: 280,
              cost: 0.0011,
              metadata: { model: 'gpt-3.5-turbo', tokens: 150 },
            },
          ],
        },
        {
          id: 'suggest-2',
          name: 'GenSuggestions',
          depth: 1,
          input: { issues: ['memory_leak', 'unused_variable', 'no_error_handling'] },
          output: { suggestions: ['Add try-catch', 'Remove unused var', 'Fix memory leak'] },
          latency: 900,
          cost: 0.0036,
          metadata: { model: 'gpt-4', tokens: 720 },
        },
      ],
    },
  },
];

const NODE_WIDTH = 160;
const NODE_HEIGHT = 64;
const HORIZONTAL_GAP = 120;
const VERTICAL_GAP = 40;

const randomSuffix = () => Math.random().toString(36).slice(2, 8);

const collectIdsFromSpans = (spans: TraceSpan[]): string[] => {
  const ids: string[] = [];
  spans.forEach((child) => {
    ids.push(child.id);
    if (child.children?.length) {
      ids.push(...collectIdsFromSpans(child.children));
    }
  });
  return ids;
};

const collectBaseSpanIds = (span: TraceSpan): string[] => {
  const ids = [span.id];
  span.children?.forEach((child) => {
    ids.push(...collectBaseSpanIds(child));
  });
  return ids;
};

const generateAltBranch = (parent: TraceSpan): TraceSpan[] => {
  const depthBase = (parent.depth ?? 0) + 1;
  const suffix = randomSuffix();
  const mainId = `${parent.id}-alt-${suffix}`;
  const workerId = `${mainId}-worker`;
  const finalizeId = `${mainId}-finalize`;

  return [
    {
      id: mainId,
      name: 'AltPath1()',
      depth: depthBase,
      input: { reason: 'feature_flag_trigger', parent: parent.name },
      output: { status: 'rerouted', branch: 'alternate-primary' },
      latency: 350 + Math.floor(Math.random() * 220),
      cost: parseFloat((0.0015 + Math.random() * 0.001).toFixed(4)),
      metadata: { simulated: true, path: 'alt-primary' },
      children: [
        {
          id: workerId,
          name: 'AltWorker()',
          depth: depthBase + 1,
          input: { payload: 'alternate-data' },
          output: { processed: true, checksum: randomSuffix() },
          latency: 260 + Math.floor(Math.random() * 160),
          cost: parseFloat((0.001 + Math.random() * 0.0008).toFixed(4)),
          metadata: { simulated: true, path: 'alt-worker' },
          children: [
            {
              id: finalizeId,
              name: 'AltFinalize()',
              depth: depthBase + 2,
              input: { branch: 'alternate-primary' },
              output: { status: 'complete', result: 'alternate_success' },
              latency: 140 + Math.floor(Math.random() * 120),
              cost: parseFloat((0.0006 + Math.random() * 0.0005).toFixed(4)),
              metadata: { simulated: true, path: 'alt-finalize' },
            },
          ],
        },
      ],
    },
  ];
};

const TraceNode: React.FC<{ data: TraceNodeData; selected: boolean }> = ({ data, selected }) => {
  const { span, isRunning, isCompleted, isDivergent, isOriginalPath, onPlay } = data;
  const [isHovered, setIsHovered] = useState(false);
  const hasChildren = !!span.children?.length;

  return (
    <div
      className="relative group"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <Handle type="target" position={Position.Left} style={{ background: '#64748b', height: 8, width: 8 }} />
      <div
        style={{
          width: NODE_WIDTH,
          padding: '8px',
          borderRadius: '4px',
          border: `2px solid ${
            selected
              ? '#3b82f6'
              : isRunning
              ? '#60a5fa'
              : isCompleted
              ? '#4ade80'
              : isDivergent
              ? '#f97316'
              : '#e5e7eb'
          }`,
          background: isRunning
            ? 'rgba(59, 130, 246, 0.1)'
            : isCompleted
            ? 'rgba(74, 222, 128, 0.1)'
            : isDivergent
            ? 'rgba(249, 115, 22, 0.2)'
            : isOriginalPath
            ? 'rgba(156, 163, 175, 0.4)'
            : 'white',
          opacity: isOriginalPath ? 0.45 : 1,
          borderStyle: isOriginalPath ? 'dashed' : 'solid',
          boxShadow: selected ? '0 4px 6px rgba(0,0,0,0.1)' : '0 1px 3px rgba(0,0,0,0.05)',
          transition: 'all 0.2s',
          animation: isRunning ? 'pulse 2s infinite' : 'none',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <div style={{ flexShrink: 0 }}>
            <div style={{ fontFamily: 'monospace', fontSize: '12px', fontWeight: 600, whiteSpace: 'nowrap' }}>
              {span.name}
            </div>
            {span.latency && (
              <div style={{ fontSize: '10px', color: '#6b7280' }}>
                {span.latency}ms
              </div>
            )}
          </div>
          {isHovered && !isRunning && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '2px', flexShrink: 0 }}>
              <button
                onClick={() => onPlay(span, 'single')}
                style={{
                  padding: '2px',
                  borderRadius: '4px',
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                }}
                title="Replay this function only"
              >
                <Play size={12} fill="currentColor" />
              </button>
              {hasChildren && (
                <button
                  onClick={() => onPlay(span, 'subtree')}
                  style={{
                    padding: '2px',
                    borderRadius: '4px',
                    border: 'none',
                    background: 'transparent',
                    cursor: 'pointer',
                  }}
                  title="Replay with all children"
                >
                  <PlayCircle size={12} />
                </button>
              )}
            </div>
          )}
        </div>
      </div>
      <Handle type="source" position={Position.Right} style={{ background: '#64748b', height: 8, width: 8 }} />
    </div>
  );
};

const nodeTypes = {
  traceNode: TraceNode,
};

const buildFlowGraph = (
  rootSpan: TraceSpan,
  alternateBranches: Record<string, TraceSpan[]>,
  runningSpans: Set<string>,
  completedSpans: Set<string>,
  divergentSpans: Set<string>,
  originalSpans: Set<string>,
  onPlay: (span: TraceSpan, mode: 'single' | 'subtree') => void
): { nodes: Node[]; edges: Edge[] } => {
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  const spanToHierarchy = (span: TraceSpan): any => ({
    id: span.id,
    span,
    children: [
      ...(span.children?.map(spanToHierarchy) ?? []),
      ...(alternateBranches[span.id]?.map(spanToHierarchy) ?? []),
    ],
  });

  const root = hierarchy(spanToHierarchy(rootSpan));

  const treeLayout = tree<any>()
    .nodeSize([NODE_HEIGHT + VERTICAL_GAP, NODE_WIDTH + HORIZONTAL_GAP])
    .separation((a: any, b: any) => (a.parent === b.parent ? 1 : 1.1));

  treeLayout(root);

  root.descendants().forEach((d: any) => {
    const span = d.data.span;
    const nodeId = span.id;

    nodes.push({
      id: nodeId,
      type: 'traceNode',
      position: {
        x: d.y,
        y: d.x,
      },
      data: {
        span,
        isRunning: runningSpans.has(nodeId),
        isCompleted: completedSpans.has(nodeId),
        isDivergent: divergentSpans.has(nodeId),
        isOriginalPath: originalSpans.has(nodeId),
        onPlay,
      },
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
    });

    if (d.parent) {
      const parentId = d.parent.data.span.id;
      edges.push({
        id: `${parentId}-${nodeId}`,
        source: parentId,
        target: nodeId,
        type: 'smoothstep',
        animated: runningSpans.has(nodeId),
        style: {
          stroke: divergentSpans.has(nodeId)
            ? '#f97316'
            : originalSpans.has(nodeId)
            ? '#9ca3af'
            : '#64748b',
          strokeWidth: 2,
          strokeDasharray: originalSpans.has(nodeId) ? '5,5' : undefined,
        },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          color: divergentSpans.has(nodeId)
            ? '#f97316'
            : originalSpans.has(nodeId)
            ? '#9ca3af'
            : '#64748b',
        },
      });
    }
  });

  return { nodes, edges };
};

const FunctionDetailsPanel: React.FC<{ span: TraceSpan | null }> = ({ span }) => {
  if (!span) {
    return (
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100%',
          color: '#6b7280',
          fontSize: '14px',
        }}
      >
        Select a span to view details
      </div>
    );
  }

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: '16px' }}>
      <div>
        <h3 style={{ fontWeight: 600, fontSize: '18px', marginBottom: '8px' }}>{span.name}</h3>
        <div style={{ display: 'flex', gap: '16px', fontSize: '14px', color: '#6b7280' }}>
          {span.latency && <span>Latency: {span.latency}ms</span>}
          {span.cost && <span>Cost: ${span.cost.toFixed(4)}</span>}
        </div>
      </div>

      {span.metadata && (
        <div style={{ marginTop: '16px' }}>
          <h4 style={{ fontWeight: 500, fontSize: '14px', marginBottom: '8px' }}>Metadata</h4>
          <div
            style={{
              background: '#f3f4f6',
              borderRadius: '4px',
              padding: '12px',
              fontFamily: 'monospace',
              fontSize: '12px',
            }}
          >
            <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word', margin: 0 }}>
              {JSON.stringify(span.metadata, null, 2)}
            </pre>
          </div>
        </div>
      )}

      <div style={{ marginTop: '16px' }}>
        <h4 style={{ fontWeight: 500, fontSize: '14px', marginBottom: '8px' }}>Input</h4>
        <div
          style={{
            background: '#f3f4f6',
            borderRadius: '4px',
            padding: '12px',
            fontFamily: 'monospace',
            fontSize: '12px',
            maxHeight: '192px',
            overflow: 'auto',
          }}
        >
          <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word', margin: 0 }}>
            {JSON.stringify(span.input, null, 2)}
          </pre>
        </div>
      </div>

      <div style={{ marginTop: '16px' }}>
        <h4 style={{ fontWeight: 500, fontSize: '14px', marginBottom: '8px' }}>Output</h4>
        <div
          style={{
            background: '#f3f4f6',
            borderRadius: '4px',
            padding: '12px',
            fontFamily: 'monospace',
            fontSize: '12px',
            maxHeight: '192px',
            overflow: 'auto',
          }}
        >
          <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word', margin: 0 }}>
            {JSON.stringify(span.output, null, 2)}
          </pre>
        </div>
      </div>
    </div>
  );
};

const TerminalView: React.FC<{ logs: string[] }> = ({ logs }) => {
  const getLogColor = (log: string) => {
    if (log.startsWith('>')) return '#60a5fa';
    if (log.includes('started')) return '#facc15';
    if (log.includes('✓') || log.includes('completed')) return '#4ade80';
    if (log.includes('Input:')) return '#22d3ee';
    if (log.includes('Output:')) return '#c084fc';
    if (log.includes('error') || log.includes('Error')) return '#f87171';
    return '#9ca3af';
  };

  return (
    <div
      style={{
        height: '100%',
        background: '#0a0a0a',
        fontFamily: 'monospace',
        fontSize: '12px',
        padding: '16px',
        overflow: 'auto',
      }}
    >
      {logs.map((log, idx) => (
        <div
          key={idx}
          style={{ marginBottom: '4px', whiteSpace: 'pre-wrap', wordBreak: 'break-word', color: getLogColor(log) }}
        >
          {log}
        </div>
      ))}
    </div>
  );
};

const WorkflowViewInner: React.FC = () => {
  const [selectedRunIndex, setSelectedRunIndex] = useState(0);
  const [selectedSpan, setSelectedSpan] = useState<TraceSpan | null>(mockTraceRuns[0]?.trace ?? null);
  const [runningSpans, setRunningSpans] = useState<Set<string>>(new Set());
  const [completedSpans, setCompletedSpans] = useState<Set<string>>(new Set());
  const [divergentSpans, setDivergentSpans] = useState<Set<string>>(new Set());
  const [originalSpans, setOriginalSpans] = useState<Set<string>>(new Set());
  const [altBranches, setAltBranches] = useState<Record<string, TraceSpan[]>>({});
  const [logs, setLogs] = useState<string[]>(['> Workflow initialized', '> Ready to execute trace']);
  const { fitView } = useReactFlow();

  const currentRun = mockTraceRuns[selectedRunIndex] || mockTraceRuns[0]!;
  const currentTrace = currentRun.trace;

  const findSpanById = useCallback(
    (span: TraceSpan, id: string): TraceSpan | null => {
      if (span.id === id) return span;
      if (span.children) {
        for (const child of span.children) {
          const found = findSpanById(child, id);
          if (found) return found;
        }
      }
      const alternates = altBranches[span.id] ?? [];
      for (const altChild of alternates) {
        const found = findSpanById(altChild, id);
        if (found) return found;
      }
      return null;
    },
    [altBranches]
  );

  const simulateRun = useCallback(
    async (span: TraceSpan, replayMode: 'single' | 'subtree') => {
      const targetId = span.id;
      const modeLabel = replayMode === 'single' ? '(single)' : '(with children)';
      const hasChildren = !!span.children?.length;
      const shouldAttemptDivergence = replayMode === 'subtree' && hasChildren;
      const willDiverge = shouldAttemptDivergence && Math.random() < 0.5;

      const previousAltNodes = altBranches[targetId] ?? [];
      const previousAltIds = collectIdsFromSpans(previousAltNodes);
      const baseChildIds = hasChildren ? span.children!.flatMap(collectBaseSpanIds) : [];

      const executionIds = new Set<string>();

      if (replayMode === 'single') {
        executionIds.add(span.id);
      } else {
        collectBaseSpanIds(span).forEach((id) => executionIds.add(id));
      }

      let newAltNodes: TraceSpan[] = [];

      if (willDiverge) {
        newAltNodes = generateAltBranch(span);
        setAltBranches((prev) => ({ ...prev, [targetId]: newAltNodes }));
        collectIdsFromSpans(newAltNodes).forEach((id) => executionIds.add(id));
        setDivergentSpans((prev) => new Set([...prev, ...collectIdsFromSpans(newAltNodes)]));

        const baseIds = collectBaseSpanIds(span);
        setOriginalSpans((prev) => new Set([...prev, ...baseIds]));
      } else {
        if (previousAltIds.length) {
          setAltBranches((prev) => {
            const next = { ...prev };
            delete next[targetId];
            return next;
          });
          setDivergentSpans((prev) => {
            const next = new Set(prev);
            previousAltIds.forEach((id) => next.delete(id));
            return next;
          });
          setOriginalSpans((prev) => {
            const next = new Set(prev);
            collectBaseSpanIds(span).forEach((id) => next.delete(id));
            return next;
          });
        }
      }

      setLogs((prev) => [
        ...prev,
        `> Replaying ${span.name} ${modeLabel}`,
        `  • Target span contains ${hasChildren ? `${span.children!.length} child nodes` : 'no child nodes'}`,
      ]);

      const altLookup = new Map<string, TraceSpan>();
      Object.values(altBranches).forEach((branchNodes) => {
        branchNodes.forEach((node) => {
          collectIdsFromSpans([node]).forEach((id) => {
            altLookup.set(id, node);
          });
        });
      });

      for (const baseId of baseChildIds) {
        executionIds.add(baseId);
      }

      await new Promise((resolve) => setTimeout(resolve, 400));

      for (const spanId of executionIds) {
        setRunningSpans((prev) => new Set([...prev, spanId]));

        const currentSpan = findSpanById(currentTrace, spanId) ?? altLookup.get(spanId) ?? null;
        if (currentSpan) {
          setLogs((prev) => [
            ...prev,
            `  → ${currentSpan.name} started`,
            `  Input: ${JSON.stringify(currentSpan.input, null, 2)}`,
          ]);
        }

        await new Promise((resolve) => setTimeout(resolve, 800));

        setRunningSpans((prev) => {
          const next = new Set(prev);
          next.delete(spanId);
          return next;
        });
        setCompletedSpans((prev) => new Set([...prev, spanId]));

        if (currentSpan) {
          setLogs((prev) => [
            ...prev,
            `  Output: ${JSON.stringify(currentSpan.output, null, 2)}`,
            `  ✓ ${currentSpan.name} completed in ${currentSpan.latency ?? '—'}ms`,
          ]);
        }
      }

      if (willDiverge) {
        const altNames = newAltNodes.map((node) => node.name).join(', ');
        setLogs((prev) => [
          ...prev,
          `  → Divergent branch executed (${altNames || 'alternate nodes'})`,
          `> ${span.name} execution complete ${modeLabel}\n`,
        ]);
      } else {
        setLogs((prev) => [
          ...prev,
          `> ${span.name} execution complete ${modeLabel}`,
          '✓ Execution complete',
        ]);
      }
    },
    [altBranches, currentTrace, findSpanById]
  );

  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  useEffect(() => {
    const { nodes: newNodes, edges: newEdges } = buildFlowGraph(
      currentTrace,
      altBranches,
      runningSpans,
      completedSpans,
      divergentSpans,
      originalSpans,
      simulateRun
    );
    setNodes(newNodes);
    setEdges(newEdges);
  }, [
    currentTrace,
    altBranches,
    runningSpans,
    completedSpans,
    divergentSpans,
    originalSpans,
    simulateRun,
    setNodes,
    setEdges,
  ]);

  useEffect(() => {
    if (!nodes.length) return;
    requestAnimationFrame(() => {
      fitView({ padding: 0.24, duration: 400 });
    });
  }, [fitView, nodes.length]);

  const onNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    const span = node.data.span as TraceSpan;
    setSelectedSpan(span);
  }, []);

  return (
    <div
      style={{
        display: 'flex',
        height: '100%',
        width: '100%',
        flexDirection: 'column',
        overflow: 'hidden',
        border: '1px solid #e5e7eb',
        background: 'white',
      }}
    >
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
        <div style={{ flex: 3, display: 'flex', minHeight: 0 }}>
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
            <div style={{ padding: '8px 16px', borderBottom: '1px solid #e5e7eb' }}>
              <h3 style={{ fontWeight: 600, fontSize: '14px', marginBottom: '8px' }}>Trace History</h3>
              <div style={{ display: 'flex', gap: '4px', overflowX: 'auto' }}>
                {mockTraceRuns.map((run, idx) => (
                  <button
                    key={run.id}
                    onClick={() => {
                      setSelectedRunIndex(idx);
                      setSelectedSpan(run.trace);
                      setRunningSpans(new Set());
                      setCompletedSpans(new Set());
                      setDivergentSpans(new Set());
                      setOriginalSpans(new Set());
                      setAltBranches({});
                      setLogs([`> Loaded trace from ${run.timestamp}`, '> Ready to execute trace']);
                    }}
                    style={{
                      padding: '6px 12px',
                      borderRadius: '4px',
                      fontSize: '12px',
                      fontFamily: 'monospace',
                      whiteSpace: 'nowrap',
                      border: '1px solid',
                      borderColor: selectedRunIndex === idx ? '#e5e7eb' : 'transparent',
                      background: selectedRunIndex === idx ? '#f3f4f6' : '#f9fafb',
                      cursor: 'pointer',
                      color: run.status === 'error' ? '#ef4444' : 'inherit',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                      <span
                        style={{
                          width: '6px',
                          height: '6px',
                          borderRadius: '50%',
                          background:
                            run.status === 'success'
                              ? '#22c55e'
                              : run.status === 'error'
                              ? '#ef4444'
                              : '#eab308',
                        }}
                      />
                      <span>{run.timestamp}</span>
                      <span style={{ color: '#6b7280' }}>•</span>
                      <span>{run.totalLatency}ms</span>
                      <span style={{ color: '#6b7280' }}>•</span>
                      <span>${run.totalCost.toFixed(4)}</span>
                    </div>
                  </button>
                ))}
              </div>
            </div>
            <div style={{ flex: 1, minHeight: 0 }}>
              <ReactFlow
                nodes={nodes}
                edges={edges}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onNodeClick={onNodeClick}
                nodeTypes={nodeTypes}
                nodeOrigin={[0.5, 0.5]}
                fitView
                fitViewOptions={{ padding: 0.24 }}
                minZoom={0.5}
                maxZoom={1.5}
                panOnScroll
                selectionOnDrag
                panOnDrag={false}
              />
            </div>
          </div>

          <div style={{ width: '2px', background: '#e5e7eb', cursor: 'col-resize' }} />

          <div
            style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0, borderLeft: '1px solid #e5e7eb' }}
          >
            <div style={{ padding: '8px 16px', borderBottom: '1px solid #e5e7eb' }}>
              <h3 style={{ fontWeight: 600, fontSize: '14px' }}>Function Details</h3>
            </div>
            <div style={{ flex: 1, overflow: 'auto' }}>
              <FunctionDetailsPanel span={selectedSpan} />
            </div>
          </div>
        </div>

        <div style={{ height: '2px', background: '#e5e7eb', cursor: 'row-resize' }} />

        <div style={{ flex: 2, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          <div style={{ padding: '8px 16px', borderTop: '1px solid #e5e7eb', background: '#f9fafb' }}>
            <h3 style={{ fontWeight: 600, fontSize: '14px' }}>Terminal</h3>
          </div>
          <div style={{ flex: 1, overflow: 'auto' }}>
            <TerminalView logs={logs} />
          </div>
        </div>
      </div>

      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.7; }
        }
      `}</style>
    </div>
  );
};

const WorkflowReplayerTab: React.FC = () => (
  <ReactFlowProvider>
    <div style={{ height: '100%', minHeight: 0 }}>
      <WorkflowViewInner />
    </div>
  </ReactFlowProvider>
);

export default WorkflowReplayerTab;
