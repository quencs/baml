/**
 * Centralized Mock Data Configuration
 *
 * All mock workflows, test cases, and output generators in one place
 */

import type { WorkflowDefinition, GraphNode, GraphEdge, TestCaseInput, BAMLFile } from '../types';
import type { MockRuntimeConfig, NodeOutputGenerator } from './types';
import type { FunctionDefinition } from '../runtime/BamlRuntimeInterface';

/**
 * Helper to create a workflow
 */
function createWorkflow(
  id: string,
  nodes: Array<{
    id: string;
    label: string;
    type?: GraphNode['type'];
    parent?: string;
    llmClient?: string;
  }>,
  edges: Array<{ from: string; to: string; label?: string }>,
  options?: {
    parameters?: Array<{ name: string; type: string; optional: boolean }>;
    returnType?: string;
  }
): WorkflowDefinition {
  const graphNodes: GraphNode[] = nodes.map((n) => ({
    id: n.id,
    type: n.type || 'function',
    label: n.label,
    functionName: n.id,
    parent: n.parent,
    llmClient: n.llmClient,
    codeHash: `hash_${n.id}_${Date.now()}`,
    lastModified: Date.now(),
  }));

  const graphEdges: GraphEdge[] = edges.map((e, idx) => ({
    id: `edge_${idx}`,
    source: e.from,
    target: e.to,
    label: e.label,
  }));

  return {
    id,
    displayName: id.replace(/([A-Z])/g, ' $1').trim(),
    filePath: `/mock/${id}.baml`,
    startLine: 1,
    endLine: 100,
    nodes: graphNodes,
    edges: graphEdges,
    entryPoint: nodes[0]?.id || '',
    parameters: options?.parameters || [
      { name: 'input', type: 'any', optional: false },
    ],
    returnType: options?.returnType || 'any',
    childFunctions: nodes.map((n) => n.id),
    lastModified: Date.now(),
    codeHash: `hash_${id}`,
  };
}

/**
 * Create mock workflows
 */
function createMockWorkflows(): WorkflowDefinition[] {
  return [
    // 1. Simple Linear Workflow
    createWorkflow(
      'simpleWorkflow',
      [
        { id: 'fetchData', label: 'Fetch Data', type: 'function' },
        {
          id: 'processData',
          label: 'Process Data',
          type: 'llm_function',
          llmClient: 'GPT-4o',
        },
        { id: 'saveResult', label: 'Save Result', type: 'function' },
      ],
      [
        { from: 'fetchData', to: 'processData' },
        { from: 'processData', to: 'saveResult' },
      ],
      {
        parameters: [{ name: 'input', type: 'string', optional: false }],
        returnType: '{ result: string; processed: boolean }',
      }
    ),

    // 2. Conditional Workflow with Subgraph
    createWorkflow(
      'conditionalWorkflow',
      [
        { id: 'validateInput', label: 'Validate Input', type: 'function' },
        { id: 'checkCondition', label: 'Check Condition', type: 'conditional' },
        {
          id: 'handleSuccess',
          label: 'Analyze Result',
          type: 'llm_function',
          llmClient: 'Claude-3',
        },
        { id: 'handleFailure', label: 'Handle Failure', type: 'function' },

        // Subgraph: Nested processing (group node)
        { id: 'PROCESSING_SUBGRAPH', label: 'ProcessingWorkflow', type: 'group' },
        {
          id: 'subgraph_process',
          label: 'Process Data',
          type: 'function',
          parent: 'PROCESSING_SUBGRAPH',
        },
        {
          id: 'subgraph_validate',
          label: 'Validate Result',
          type: 'function',
          parent: 'PROCESSING_SUBGRAPH',
        },
      ],
      [
        { from: 'validateInput', to: 'checkCondition' },
        { from: 'checkCondition', to: 'handleSuccess', label: 'success' },
        { from: 'checkCondition', to: 'handleFailure', label: 'failure' },
        { from: 'handleSuccess', to: 'subgraph_process' },
        { from: 'subgraph_process', to: 'subgraph_validate' },
      ],
      {
        parameters: [
          { name: 'data', type: 'any', optional: false },
          { name: 'threshold', type: 'number', optional: true },
        ],
        returnType: '{ success: boolean; result?: any }',
      }
    ),

    // 3. Shared Workflow
    createWorkflow(
      'sharedWorkflow',
      [
        { id: 'aggregateData', label: 'Aggregate Data', type: 'function' },
        { id: 'fetchData', label: 'Fetch Data', type: 'function' },
      ],
      [{ from: 'aggregateData', to: 'fetchData' }],
      {
        parameters: [{ name: 'sources', type: 'string[]', optional: false }],
        returnType: '{ aggregated: any; count: number }',
      }
    ),
  ];
}

/**
 * Create mock test cases
 */
function createMockTestCases(): Record<string, TestCaseInput[]> {
  return {
    fetchData: [
      {
        id: 'test_fetchData_success',
        name: 'success_case',
        source: 'test',
        nodeId: 'fetchData',
        filePath: 'tests/fetchData.test.ts',
        inputs: { url: 'https://api.example.com/data', timeout: 5000 },
        expectedOutput: { data: { id: 1, name: 'Test' }, status: 200 },
        status: 'passing',
        lastRun: Date.now() - 3600000,
      },
      {
        id: 'test_fetchData_timeout',
        name: 'timeout_case',
        source: 'test',
        nodeId: 'fetchData',
        filePath: 'tests/fetchData.test.ts',
        inputs: { url: 'https://slow-api.example.com/data', timeout: 100 },
        expectedOutput: { error: 'Timeout' },
        status: 'passing',
        lastRun: Date.now() - 7200000,
      },
    ],
    processData: [
      {
        id: 'test_processData_valid',
        name: 'valid_input',
        source: 'test',
        nodeId: 'processData',
        filePath: 'tests/processData.test.ts',
        inputs: { data: { id: 1, value: 'test' }, format: 'json' },
        expectedOutput: { processed: true, result: 'formatted data' },
        status: 'passing',
        lastRun: Date.now() - 1800000,
      },
    ],
    validateInput: [
      {
        id: 'test_validateInput_valid_email',
        name: 'valid_email',
        source: 'test',
        nodeId: 'validateInput',
        filePath: 'tests/validateInput.test.ts',
        inputs: { data: { email: 'test@example.com', age: 25 } },
        expectedOutput: { valid: true },
        status: 'passing',
        lastRun: Date.now() - 600000,
      },
    ],
    aggregateData: [
      {
        id: 'test_aggregateData_multiple',
        name: 'multiple_sources',
        source: 'test',
        nodeId: 'aggregateData',
        filePath: 'tests/aggregateData.test.ts',
        inputs: { sources: ['api1', 'api2', 'api3'] },
        expectedOutput: { aggregated: { total: 3 }, count: 3 },
        status: 'passing',
        lastRun: Date.now() - 1200000,
      },
    ],
  };
}

/**
 * Create mock output generators
 */
function createOutputGenerators(): Record<string, NodeOutputGenerator> {
  return {
    // Simple workflow outputs
    fetchData: (ctx, inputs) => ({
      data: {
        id: Math.floor(Math.random() * 1000),
        value: (ctx.input as any) || 'sample data',
        timestamp: Date.now(),
      },
      records: Math.floor(Math.random() * 100) + 10,
    }),

    processData: (ctx) => ({
      result: `Processed: ${(ctx.data as any)?.value || 'data'}`,
      tokens: Math.floor(Math.random() * 500) + 200,
      model: 'gpt-4',
      confidence: (Math.random() * 0.3 + 0.7).toFixed(2),
    }),

    saveResult: () => ({
      saved: true,
      id: `RESULT-${Date.now()}`,
      location: '/storage/results',
    }),

    // Conditional workflow outputs
    validateInput: (ctx) => ({
      valid: true,
      data: ctx.data || { value: 'validated' },
      checks: ['format', 'schema', 'range'],
    }),

    checkCondition: () => ({
      condition: Math.random() > 0.4 ? 'success' : 'failure',
      passed: Math.random() > 0.4,
      score: Math.random().toFixed(2),
    }),

    handleSuccess: () => ({
      status: 'success',
      message: 'Condition met, proceeding to subgraph',
      nextStep: 'subgraph',
    }),

    handleFailure: () => ({
      status: 'failure',
      message: 'Condition not met, skipping subgraph',
      reason: 'Below threshold',
    }),

    subgraph_process: () => ({
      subgraphResult: 'processed in subgraph',
      operations: ['transform', 'validate', 'enrich'],
      processingTime: Math.floor(Math.random() * 500) + 100,
    }),

    subgraph_validate: (ctx) => ({
      validated: true,
      subgraphComplete: true,
      finalOutput: {
        ...ctx,
        subgraphProcessed: true,
      },
    }),

    // Shared workflow outputs
    aggregateData: (ctx) => {
      const sources = (ctx.sources as string[]) || ['default'];
      return {
        aggregated: {
          sources: sources,
          total: sources.length,
          timestamp: Date.now(),
        },
        count: sources.length,
        dataSize: Math.floor(Math.random() * 5000) + 1000,
      };
    },
  };
}

/**
 * Create mock BAML files
 */
function createBAMLFiles(): BAMLFile[] {
  return [
    {
      path: 'workflows/simple.baml',
      functions: [
        { name: 'simpleWorkflow', type: 'workflow', filePath: 'workflows/simple.baml' },
        { name: 'fetchData', type: 'function', filePath: 'workflows/simple.baml' },
        { name: 'processData', type: 'llm_function', filePath: 'workflows/simple.baml' },
        { name: 'saveResult', type: 'function', filePath: 'workflows/simple.baml' },
      ],
      tests: [
        {
          name: 'test_fetchData_success',
          functionName: 'fetchData',
          filePath: 'workflows/simple.baml',
          nodeType: 'function',
        },
        {
          name: 'test_processData_valid',
          functionName: 'processData',
          filePath: 'workflows/simple.baml',
          nodeType: 'llm_function',
        },
      ],
    },
    {
      path: 'workflows/conditional.baml',
      functions: [
        { name: 'conditionalWorkflow', type: 'workflow', filePath: 'workflows/conditional.baml' },
        { name: 'validateInput', type: 'function', filePath: 'workflows/conditional.baml' },
        { name: 'handleSuccess', type: 'llm_function', filePath: 'workflows/conditional.baml' },
        { name: 'handleFailure', type: 'function', filePath: 'workflows/conditional.baml' },
      ],
      tests: [
        {
          name: 'test_validateInput_valid_email',
          functionName: 'validateInput',
          filePath: 'workflows/conditional.baml',
          nodeType: 'function',
        },
      ],
    },
  ];
}

/**
 * Create default mock runtime configuration
 */
export function createMockRuntimeConfig(
  options?: {
    cacheHitRate?: number;
    errorRate?: number;
    verboseLogging?: boolean;
    speedMultiplier?: number;
  }
): MockRuntimeConfig {
  return {
    workflows: createMockWorkflows(),
    functions: [], // No standalone functions in this config
    testCases: createMockTestCases(),
    nodeOutputs: createOutputGenerators(),
    executionBehavior: {
      cacheHitRate: options?.cacheHitRate ?? 0.3,
      errorRate: options?.errorRate ?? 0.1,
      verboseLogging: options?.verboseLogging ?? true,
      speedMultiplier: options?.speedMultiplier ?? 1,
      nodeDelays: {
        llm_function: () => 1500 + Math.random() * 1000,
        function: () => 400 + Math.random() * 300,
        conditional: () => 300 + Math.random() * 200,
      },
    },
    bamlFiles: createBAMLFiles(),
  };
}
