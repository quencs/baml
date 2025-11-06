/**
 * Mock Data Provider for BAML SDK
 *
 * Provides comprehensive sample workflows and simulates realistic execution
 * for development and testing. Covers all node types, execution states,
 * error scenarios, caching, branching, and loops.
 */

import type {
  MockDataProvider,
  WorkflowDefinition,
  ExecutionSnapshot,
  BAMLEvent,
  GraphNode,
  GraphEdge,
  LogEntry,
  TestCaseInput,
  BAMLFile,
} from './types';

/**
 * Configuration for mock execution behavior
 */
interface MockConfig {
  /** Probability of cache hits (0-1) */
  cacheHitRate: number;
  /** Probability of errors (0-1) */
  errorRate: number;
  /** Enable verbose logging */
  verboseLogging: boolean;
  /** Execution speed multiplier (higher = slower) */
  speedMultiplier: number;
}

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
 * Default Mock Data Provider with comprehensive testing scenarios
 */
export class DefaultMockProvider implements MockDataProvider {
  private workflows: WorkflowDefinition[];
  private executions: Map<string, ExecutionSnapshot[]> = new Map();
  private config: MockConfig;
  private executionCount = 0;

  constructor(config?: Partial<MockConfig>) {
    this.config = {
      cacheHitRate: config?.cacheHitRate ?? 0.3,
      errorRate: config?.errorRate ?? 0.1,
      verboseLogging: config?.verboseLogging ?? true,
      speedMultiplier: config?.speedMultiplier ?? 1,
    };
    this.workflows = this.createSampleWorkflows();

    // Validate mock data on initialization
    this.validateMockData();
  }

  /**
   * Validate that all tests reference valid nodes/functions
   * Throws an error if invalid data is found
   */
  private validateMockData(): void {
    const bamlFiles = this.getBAMLFiles();
    const workflows = this.workflows;

    // Build a map of all valid node IDs across all workflows
    const allNodeIds = new Set<string>();
    workflows.forEach(workflow => {
      workflow.nodes.forEach(node => allNodeIds.add(node.id));
    });

    // Collect all errors
    const errors: string[] = [];

    // Validate each test
    bamlFiles.forEach(file => {
      file.tests.forEach(test => {
        // Check if the test's functionName exists as a node in any workflow
        const foundInWorkflow = allNodeIds.has(test.functionName);

        // If not found in any workflow, check if it's a standalone function
        const isStandaloneFunction = file.functions.some(
          f => f.name === test.functionName && f.type !== 'workflow'
        );

        if (!foundInWorkflow && !isStandaloneFunction) {
          errors.push(
            `Test "${test.name}" in ${file.path} references function "${test.functionName}" which doesn't exist as a node in any workflow or as a standalone function`
          );
        }
      });
    });

    // Throw if any errors found
    if (errors.length > 0) {
      throw new Error(
        `❌ Mock data validation failed:\n${errors.map(e => `  - ${e}`).join('\n')}`
      );
    }

    console.log('✅ Mock data validation passed');
  }

  private createSampleWorkflows(): WorkflowDefinition[] {
    return [
      // 1. Simple Linear Workflow - 3 consecutive nodes
      createWorkflow(
        'simpleWorkflow',
        [
          { id: 'fetchData', label: 'Fetch Data', type: 'function' },
          { id: 'processData', label: 'Process Data', type: 'llm_function', llmClient: 'GPT-4o' },
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
      // Structure follows data-workflow.ts pattern
      createWorkflow(
        'conditionalWorkflow',
        [
          { id: 'validateInput', label: 'Validate Input', type: 'function' },
          { id: 'checkCondition', label: 'Check Condition', type: 'conditional' },
          { id: 'handleSuccess', label: 'Analyze Result', type: 'llm_function', llmClient: 'Claude-3' },
          { id: 'handleFailure', label: 'Handle Failure', type: 'function' },

          // Subgraph: Nested processing (group node) - USE 'group' type!
          { id: 'PROCESSING_SUBGRAPH', label: 'ProcessingWorkflow', type: 'group' },
          // Nodes inside the subgraph
          { id: 'subgraph_process', label: 'Process Data', type: 'function', parent: 'PROCESSING_SUBGRAPH' },
          { id: 'subgraph_validate', label: 'Validate Result', type: 'function', parent: 'PROCESSING_SUBGRAPH' },
        ],
        [
          { from: 'validateInput', to: 'checkCondition' },
          { from: 'checkCondition', to: 'handleSuccess', label: 'success' },
          { from: 'checkCondition', to: 'handleFailure', label: 'failure' },

          // Success path goes to first child of subgraph
          { from: 'handleSuccess', to: 'subgraph_process' },

          // Inside subgraph: children connected to each other
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

      // 3. Shared Workflow - demonstrates function reuse across workflows
      // Uses fetchData which is also used in simpleWorkflow
      createWorkflow(
        'sharedWorkflow',
        [
          { id: 'aggregateData', label: 'Aggregate Data', type: 'function' },
          { id: 'fetchData', label: 'Fetch Data', type: 'function' },
        ],
        [
          { from: 'aggregateData', to: 'fetchData' },
        ],
        {
          parameters: [{ name: 'sources', type: 'string[]', optional: false }],
          returnType: '{ aggregated: any; count: number }',
        }
      ),
    ];
  }

  getWorkflows(): WorkflowDefinition[] {
    return this.workflows;
  }

  getExecutions(workflowId: string): ExecutionSnapshot[] {
    return this.executions.get(workflowId) ?? [];
  }

  /**
   * Get sample test cases for nodes in a workflow
   */
  getTestCases(_workflowId: string, nodeId: string): TestCaseInput[] {
    // Sample test cases for demonstration
    // NOTE: Tests are organized by the function/node they test, not by workflow
    const testCases: Record<string, TestCaseInput[]> = {
      'fetchData': [
        {
          id: 'test_fetchData_success',
          name: 'success_case',
          source: 'test',
          nodeId: 'fetchData',
          filePath: 'tests/fetchData.test.ts',
          inputs: { url: 'https://api.example.com/data', timeout: 5000 },
          expectedOutput: { data: { id: 1, name: 'Test' }, status: 200 },
          status: 'passing',
          lastRun: Date.now() - 3600000, // 1 hour ago
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
          lastRun: Date.now() - 7200000, // 2 hours ago
        },
        {
          id: 'test_fetchData_in_shared',
          name: 'in_shared_workflow',
          source: 'test',
          nodeId: 'fetchData',
          filePath: 'shared/workflows/shared.baml',
          inputs: { url: 'https://shared-api.example.com/aggregate', timeout: 3000 },
          expectedOutput: { data: { aggregated: true, sources: 3 }, status: 200 },
          status: 'passing',
          lastRun: Date.now() - 1000000, // ~16 minutes ago
        },
      ],
      'processData': [
        {
          id: 'test_processData_valid',
          name: 'valid_input',
          source: 'test',
          nodeId: 'processData',
          filePath: 'tests/processData.test.ts',
          inputs: { data: { id: 1, value: 'test' }, format: 'json' },
          expectedOutput: { processed: true, result: 'formatted data' },
          status: 'passing',
          lastRun: Date.now() - 1800000, // 30 minutes ago
        },
        {
          id: 'test_processData_empty',
          name: 'empty_input',
          source: 'test',
          nodeId: 'processData',
          filePath: 'tests/processData.test.ts',
          inputs: { data: {}, format: 'json' },
          expectedOutput: { processed: false, error: 'Empty data' },
          status: 'failing',
          lastRun: Date.now() - 900000, // 15 minutes ago
        },
      ],
      'validateInput': [
        {
          id: 'test_validateInput_valid_email',
          name: 'valid_email',
          source: 'test',
          nodeId: 'validateInput',
          filePath: 'tests/validateInput.test.ts',
          inputs: { data: { email: 'test@example.com', age: 25 } },
          expectedOutput: { valid: true },
          status: 'passing',
          lastRun: Date.now() - 600000, // 10 minutes ago
        },
        {
          id: 'test_validateInput_invalid_email',
          name: 'invalid_email',
          source: 'test',
          nodeId: 'validateInput',
          filePath: 'tests/validateInput.test.ts',
          inputs: { data: { email: 'invalid', age: 25 } },
          expectedOutput: { valid: false, error: 'Invalid email' },
          status: 'passing',
          lastRun: Date.now() - 500000, // 8 minutes ago
        },
      ],
      'handleSuccess': [
        {
          id: 'test_handleSuccess_normal',
          name: 'normal_case',
          source: 'test',
          nodeId: 'handleSuccess',
          filePath: 'tests/handleSuccess.test.ts',
          inputs: { data: { score: 0.9 } },
          expectedOutput: { analysis: 'Good result', confidence: 'high' },
          status: 'passing',
          lastRun: Date.now() - 400000, // 6.6 minutes ago
        },
      ],
      'handleFailure': [
        {
          id: 'test_handleFailure_error',
          name: 'error_case',
          source: 'test',
          nodeId: 'handleFailure',
          filePath: 'tests/handleFailure.test.ts',
          inputs: { error: { code: 'VALIDATION_FAILED', message: 'Invalid data' } },
          expectedOutput: { handled: true, fallback: 'default' },
          status: 'passing',
          lastRun: Date.now() - 300000, // 5 minutes ago
        },
      ],
      'aggregateData': [
        {
          id: 'test_aggregateData_multiple',
          name: 'multiple_sources',
          source: 'test',
          nodeId: 'aggregateData',
          filePath: 'tests/aggregateData.test.ts',
          inputs: { sources: ['api1', 'api2', 'api3'] },
          expectedOutput: { aggregated: { total: 3 }, count: 3 },
          status: 'passing',
          lastRun: Date.now() - 1200000, // 20 minutes ago
        },
        {
          id: 'test_aggregateData_single',
          name: 'single_source',
          source: 'test',
          nodeId: 'aggregateData',
          filePath: 'tests/aggregateData.test.ts',
          inputs: { sources: ['api1'] },
          expectedOutput: { aggregated: { total: 1 }, count: 1 },
          status: 'passing',
          lastRun: Date.now() - 2400000, // 40 minutes ago
        },
      ],
    };

    return testCases[nodeId] || [];
  }

  /**
   * Get all BAML files with their functions and tests for debug panel
   * Matches the actual workflows defined in createSampleWorkflows()
   */
  getBAMLFiles(): BAMLFile[] {
    const files: BAMLFile[] = [
      // File 1: Simple linear workflow
      {
        path: 'workflows/simple.baml',
        functions: [
          {
            name: 'simpleWorkflow',
            type: 'workflow',
            filePath: 'workflows/simple.baml',
          },
          {
            name: 'fetchData',
            type: 'function',
            filePath: 'workflows/simple.baml',
          },
          {
            name: 'processData',
            type: 'llm_function',
            filePath: 'workflows/simple.baml',
          },
          {
            name: 'saveResult',
            type: 'function',
            filePath: 'workflows/simple.baml',
          },
        ],
        tests: [
          // Function-level tests
          {
            name: 'test_fetchData_success',
            functionName: 'fetchData',
            filePath: 'workflows/simple.baml',
            nodeType: 'function',
          },
          {
            name: 'test_fetchData_timeout',
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
          {
            name: 'test_processData_empty',
            functionName: 'processData',
            filePath: 'workflows/simple.baml',
            nodeType: 'llm_function',
          },
        ],
      },
      // File 2: Conditional workflow with branching
      {
        path: 'workflows/conditional.baml',
        functions: [
          {
            name: 'conditionalWorkflow',
            type: 'workflow',
            filePath: 'workflows/conditional.baml',
          },
          {
            name: 'validateInput',
            type: 'function',
            filePath: 'workflows/conditional.baml',
          },
          {
            name: 'handleSuccess',
            type: 'llm_function',
            filePath: 'workflows/conditional.baml',
          },
          {
            name: 'handleFailure',
            type: 'function',
            filePath: 'workflows/conditional.baml',
          },
        ],
        tests: [
          // Function-level tests
          {
            name: 'test_validateInput_valid_email',
            functionName: 'validateInput',
            filePath: 'workflows/conditional.baml',
            nodeType: 'function',
          },
          {
            name: 'test_validateInput_invalid_email',
            functionName: 'validateInput',
            filePath: 'workflows/conditional.baml',
            nodeType: 'function',
          },
          {
            name: 'test_handleSuccess_normal',
            functionName: 'handleSuccess',
            filePath: 'workflows/conditional.baml',
            nodeType: 'llm_function',
          },
          // {
          //   name: 'test_handleFailure_error',
          //   functionName: 'handleFailure',
          //   filePath: 'workflows/conditional.baml',
          //   nodeType: 'function',
          // },
        ],
      },
      // File 3: Shared workflow - demonstrates function reuse
      // Note: fetchData is shared with simple.baml but only aggregateData is defined here
      {
        path: 'shared/workflows/shared.baml',
        functions: [
          {
            name: 'sharedWorkflow',
            type: 'workflow',
            filePath: 'shared/workflows/shared.baml',
          },
          {
            name: 'aggregateData',
            type: 'function',
            filePath: 'shared/workflows/shared.baml',
          },
        ],
        tests: [
          // Function-level tests for aggregateData
          {
            name: 'test_aggregateData_multiple',
            functionName: 'aggregateData',
            filePath: 'shared/workflows/shared.baml',
            nodeType: 'function',
          },
          {
            name: 'test_aggregateData_single',
            functionName: 'aggregateData',
            filePath: 'shared/workflows/shared.baml',
            nodeType: 'function',
          },
          // Test for shared function fetchData (demonstrates priority staying in current workflow)
          {
            name: 'test_fetchData_in_shared',
            functionName: 'fetchData',
            filePath: 'shared/workflows/shared.baml',
            nodeType: 'function',
          },
        ],
      },
      // File 4: Standalone LLM function not part of any workflow
      {
        path: 'llm_only.baml',
        functions: [
          {
            name: 'StandaloneLLMFunction',
            type: 'llm_function',
            filePath: 'llm_only.baml',
          },
        ],
        tests: [
          {
            name: 'test_standalone_basic',
            functionName: 'StandaloneLLMFunction',
            filePath: 'llm_only.baml',
            nodeType: 'llm_function',
          },
          {
            name: 'test_standalone_complex',
            functionName: 'StandaloneLLMFunction',
            filePath: 'llm_only.baml',
            nodeType: 'llm_function',
          },
        ],
      },
    ];

    return files;
  }

  /**
   * Comprehensive workflow execution simulation with realistic behavior
   */
  async *simulateExecution(
    workflowId: string,
    inputs: Record<string, unknown>,
    startFromNodeId?: string
  ): AsyncGenerator<BAMLEvent> {
    const workflow = this.workflows.find((w) => w.id === workflowId);
    if (!workflow) {
      throw new Error(`Workflow ${workflowId} not found`);
    }

    this.executionCount++;
    const executionId = `exec_${Date.now()}_${this.executionCount}`;

    // Determine execution path based on workflow type
    yield* this.simulateWorkflowPath(workflow, executionId, inputs, startFromNodeId);
  }

  /**
   * Simulate execution path with branching, loops, and realistic behavior
   */
  private async *simulateWorkflowPath(
    workflow: WorkflowDefinition,
    executionId: string,
    inputs: Record<string, unknown>,
    startFromNodeId?: string
  ): AsyncGenerator<BAMLEvent> {
    const visited = new Set<string>();
    // Start from specified node or use workflow entry point
    let currentNodes = [startFromNodeId || workflow.entryPoint];
    let iterationCount = 0;
    const maxIterations = 20; // Prevent infinite loops

    // Context that accumulates throughout execution
    const context: Record<string, unknown> = { ...inputs };

    // If starting from a specific node, log it
    if (startFromNodeId && startFromNodeId !== workflow.entryPoint) {
      console.log(`⏩ Starting execution from node: ${startFromNodeId}`);
    }

    while (currentNodes.length > 0 && iterationCount < maxIterations) {
      iterationCount++;
      const nextNodes: string[] = [];

      for (const nodeId of currentNodes) {
        const node = workflow.nodes.find((n) => n.id === nodeId);
        if (!node) continue;

        // Skip if already visited (unless it's a loop)
        if (visited.has(nodeId) && node.type !== 'loop') {
          continue;
        }

        visited.add(nodeId);

        // Execute node
        const result = yield* this.executeNode(
          node,
          executionId,
          context,
          workflow
        );

        if (result.error) {
          // Stop execution on error
          yield {
            type: 'execution.error',
            executionId,
            error: result.error,
            nodeId,
          };
          return;
        }

        // Merge outputs into context
        if (result.outputs) {
          Object.assign(context, result.outputs);
        }

        // Determine next nodes based on node type and outputs
        const outgoingEdges = workflow.edges.filter((e) => e.source === nodeId);

        if (node.type === 'conditional' && result.outputs?.condition) {
          // Follow the branch that matches the condition
          const chosenEdge = outgoingEdges.find(
            (e) => e.label === result.outputs?.condition
          );
          if (chosenEdge) {
            nextNodes.push(chosenEdge.target);

            if (this.config.verboseLogging) {
              yield {
                type: 'node.log',
                executionId,
                nodeId,
                log: this.createLog(
                  executionId,
                  'info',
                  `Branch: ${result.outputs.condition} → ${chosenEdge.target}`
                ),
              };
            }
          }
        } else if (node.type === 'loop' && result.outputs?.continue) {
          // Loop logic - continue iterating or exit
          const continueEdge = outgoingEdges.find((e) => e.label?.includes('continue'));
          const doneEdge = outgoingEdges.find((e) => e.label?.includes('done'));

          if (
            (result.outputs as any).currentIndex < (result.outputs as any).totalItems &&
            continueEdge
          ) {
            nextNodes.push(continueEdge.target);
            visited.delete(nodeId); // Allow re-visiting loop node
          } else if (doneEdge) {
            nextNodes.push(doneEdge.target);
          }
        } else if (node.type === 'return') {
          // End execution
          break;
        } else {
          // Follow all outgoing edges (typically just one for regular nodes)
          nextNodes.push(...outgoingEdges.map((e) => e.target));
        }
      }

      currentNodes = nextNodes;

      // Small delay between execution steps
      await this.delay(100 * this.config.speedMultiplier);
    }
  }

  /**
   * Execute a single node with realistic simulation
   */
  private async *executeNode(
    node: GraphNode,
    executionId: string,
    context: Record<string, unknown>,
    workflow: WorkflowDefinition
  ): AsyncGenerator<
    BAMLEvent,
    { outputs?: Record<string, unknown>; error?: Error },
    undefined
  > {
    // Capture inputs at the start
    const nodeInputs = { ...context };

    // Emit start event
    yield {
      type: 'node.started',
      executionId,
      nodeId: node.id,
      inputs: nodeInputs,
    };

    // Check for cache hit
    const shouldUseCache = Math.random() < this.config.cacheHitRate;
    if (shouldUseCache && this.executionCount > 1) {
      yield {
        type: 'node.cached',
        executionId,
        nodeId: node.id,
        fromExecutionId: `exec_${Date.now() - 60000}_1`,
      };

      const cachedOutputs = this.generateMockOutputs(node, workflow, context);
      yield {
        type: 'node.completed',
        executionId,
        nodeId: node.id,
        inputs: nodeInputs,  // Include inputs
        outputs: cachedOutputs,
        duration: 50, // Cached is fast
      };

      return { outputs: cachedOutputs };
    }

    // Simulate processing with logs
    const duration = this.getNodeDuration(node.type);
    const startTime = Date.now();

    // Generate realistic logs during execution
    const logCount = node.type === 'llm_function' ? 3 : 1;
    for (let i = 0; i < logCount; i++) {
      await this.delay((duration / logCount) * this.config.speedMultiplier);

      if (this.config.verboseLogging) {
        yield {
          type: 'node.log',
          executionId,
          nodeId: node.id,
          log: this.createLog(
            executionId,
            'info',
            this.getLogMessage(node, i, logCount)
          ),
        };
      }

      // Emit progress for long-running nodes
      if (node.type === 'llm_function' && i < logCount - 1) {
        yield {
          type: 'node.progress',
          executionId,
          nodeId: node.id,
          progress: ((i + 1) / logCount) * 100,
        };
      }
    }

    // Simulate errors (based on configured error rate)
    const shouldError = Math.random() < this.config.errorRate;

    if (shouldError) {
      const error = new Error(this.getErrorMessage(node));
      yield {
        type: 'node.error',
        executionId,
        nodeId: node.id,
        error,
      };
      return { error };
    }

    // Generate outputs
    const outputs = this.generateMockOutputs(node, workflow, context);
    const actualDuration = Date.now() - startTime;

    yield {
      type: 'node.completed',
      executionId,
      nodeId: node.id,
      inputs: nodeInputs,  // Include inputs
      outputs,
      duration: actualDuration,
    };

    return { outputs };
  }

  private getNodeDuration(nodeType: GraphNode['type']): number {
    switch (nodeType) {
      case 'llm_function':
        return 1500 + Math.random() * 1000; // 1.5-2.5s
      case 'conditional':
        return 300 + Math.random() * 200; // 0.3-0.5s
      case 'loop':
        return 200 + Math.random() * 100; // 0.2-0.3s
      case 'function':
        return 400 + Math.random() * 300; // 0.4-0.7s
      default:
        return 500 + Math.random() * 500; // 0.5-1s
    }
  }

  /**
   * Generate realistic mock outputs based on node type and context
   */
  private generateMockOutputs(
    node: GraphNode,
    workflow: WorkflowDefinition,
    context: Record<string, unknown>
  ): Record<string, unknown> {
    const nodeId = node.id.toLowerCase();

    // Simple Workflow outputs
    if (workflow.id === 'simpleWorkflow') {
      if (nodeId.includes('fetch')) {
        return {
          data: {
            id: Math.floor(Math.random() * 1000),
            value: (context.input as any) || 'sample data',
            timestamp: Date.now(),
          },
          records: Math.floor(Math.random() * 100) + 10,
        };
      }
      if (nodeId.includes('process')) {
        return {
          result: `Processed: ${(context.data as any)?.value || 'data'}`,
          tokens: Math.floor(Math.random() * 500) + 200,
          model: 'gpt-4',
          confidence: (Math.random() * 0.3 + 0.7).toFixed(2), // 0.7-1.0
        };
      }
      if (nodeId.includes('save')) {
        return {
          saved: true,
          id: `RESULT-${Date.now()}`,
          location: '/storage/results',
        };
      }
      if (nodeId === 'return') {
        return {
          result: context.result || 'completed',
          processed: true,
          totalRecords: context.records || 0,
        };
      }
    }

    // Conditional Workflow outputs
    if (workflow.id === 'conditionalWorkflow') {
      if (nodeId.includes('validate')) {
        return {
          valid: true,
          data: context.data || { value: 'validated' },
          checks: ['format', 'schema', 'range'],
        };
      }
      if (nodeId.includes('condition') || nodeId.includes('check')) {
        // 60% success, 40% failure for testing
        const isSuccess = Math.random() > 0.4;
        return {
          condition: isSuccess ? 'success' : 'failure',
          passed: isSuccess,
          score: Math.random().toFixed(2),
          threshold: context.threshold || 0.5,
        };
      }
      if (nodeId.includes('success')) {
        return {
          status: 'success',
          message: 'Condition met, proceeding to subgraph',
          nextStep: 'subgraph',
        };
      }
      if (nodeId.includes('failure')) {
        return {
          status: 'failure',
          message: 'Condition not met, skipping subgraph',
          reason: 'Below threshold',
        };
      }
      if (nodeId.includes('subgraph_process')) {
        return {
          subgraphResult: 'processed in subgraph',
          operations: ['transform', 'validate', 'enrich'],
          processingTime: Math.floor(Math.random() * 500) + 100,
        };
      }
      if (nodeId.includes('subgraph_validate')) {
        return {
          validated: true,
          subgraphComplete: true,
          finalOutput: {
            ...context,
            subgraphProcessed: true,
          },
        };
      }
      if (nodeId === 'return') {
        return {
          success: context.passed !== false,
          result: context.finalOutput || context,
          branchTaken: context.condition || 'unknown',
        };
      }
    }

    // Shared Workflow outputs
    if (workflow.id === 'sharedWorkflow') {
      if (nodeId.includes('aggregate')) {
        const sources = (context.sources as string[]) || ['default'];
        return {
          aggregated: {
            sources: sources,
            total: sources.length,
            timestamp: Date.now(),
          },
          count: sources.length,
          dataSize: Math.floor(Math.random() * 5000) + 1000,
        };
      }
      if (nodeId.includes('fetch')) {
        // fetchData when used in shared workflow
        return {
          data: {
            id: Math.floor(Math.random() * 1000),
            value: (context.aggregated as any) || 'shared fetch data',
            timestamp: Date.now(),
          },
          records: Math.floor(Math.random() * 100) + 10,
        };
      }
    }

    // Generic outputs by type
    switch (node.type) {
      case 'llm_function':
        return {
          result: `Generated AI response for ${node.label}`,
          tokens: Math.floor(Math.random() * 800) + 100,
          model: 'gpt-4',
          latency: Math.floor(Math.random() * 2000) + 500,
        };
      case 'conditional':
        return {
          condition: Math.random() > 0.5 ? 'success' : 'failure',
          evaluated: true,
        };
      case 'function':
        return {
          success: true,
          data: { timestamp: Date.now() },
          duration: Math.floor(Math.random() * 100) + 10,
        };
      default:
        return { completed: true };
    }
  }

  private getLogMessage(
    node: GraphNode,
    step: number,
    totalSteps: number
  ): string {
    if (node.type === 'llm_function') {
      const messages = [
        `Preparing prompt for ${node.label}`,
        `Calling LLM API (model: gpt-4)`,
        `Received and processing response`,
      ];
      return messages[step] || `Processing ${node.label}...`;
    }

    if (node.type === 'conditional') {
      return `Evaluating condition: ${node.label}`;
    }

    if (node.type === 'loop') {
      return `Iteration ${step + 1} of ${totalSteps}`;
    }

    return `Executing ${node.label}`;
  }

  private getErrorMessage(node: GraphNode): string {
    const errors = [
      `Timeout while executing ${node.label}`,
      `Invalid response from ${node.label}`,
      `Resource not found in ${node.label}`,
      `Rate limit exceeded in ${node.label}`,
      `Authentication failed in ${node.label}`,
      `Network error in ${node.label}`,
    ];
    return errors[Math.floor(Math.random() * errors.length)] ?? '';
  }

  private createLog(
    executionId: string,
    level: 'debug' | 'info' | 'warn' | 'error',
    message: string
  ): LogEntry {
    return {
      timestamp: Date.now(),
      level,
      message,
      executionId,
    };
  }

  private delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

/**
 * Create a mock SDK configuration with default settings
 */
export function createMockSDKConfig(config?: Partial<MockConfig>) {
  return {
    mode: 'mock' as const,
    mockData: new DefaultMockProvider(config),
  };
}

/**
 * Create a fast mock SDK for quick testing (no delays)
 */
export function createFastMockSDKConfig() {
  return {
    mode: 'mock' as const,
    mockData: new DefaultMockProvider({
      speedMultiplier: 0.1, // 10x faster
      verboseLogging: false,
      cacheHitRate: 0,
      errorRate: 0,
    }),
  };
}

/**
 * Create an error-prone mock SDK for testing error handling
 */
export function createErrorProneSDKConfig() {
  return {
    mode: 'mock' as const,
    mockData: new DefaultMockProvider({
      speedMultiplier: 1,
      verboseLogging: true,
      cacheHitRate: 0,
      errorRate: 0.5, // 50% error rate
    }),
  };
}

/**
 * Create a cache-heavy mock SDK for testing cache behavior
 */
export function createCacheHeavySDKConfig() {
  return {
    mode: 'mock' as const,
    mockData: new DefaultMockProvider({
      speedMultiplier: 1,
      verboseLogging: true,
      cacheHitRate: 0.8, // 80% cache hit rate
      errorRate: 0,
    }),
  };
}
