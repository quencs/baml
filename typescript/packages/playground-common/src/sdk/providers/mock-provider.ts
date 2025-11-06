/**
 * Mock Data Provider
 *
 * Source: apps/baml-graph/src/sdk/mock.ts (adapted for DataProvider interface)
 * Provides realistic mock data for browser mode and testing
 *
 * Implements the full DataProvider interface with hardcoded sample data
 * and execution simulation.
 */

import type { DataProvider, TestExecutionEvent, Diagnostic } from './base';
import type {
  WorkflowDefinition,
  ExecutionSnapshot,
  BAMLEvent,
  TestCaseInput,
  GraphNode,
  GraphEdge,
  BAMLFile,
  BAMLFunction,
  CacheEntry,
  CodePosition,
  ExecutionOptions,
} from '../types';

/**
 * Configuration for mock behavior
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
 * Mock Data Provider Implementation
 *
 * Used for:
 * - Browser-based playground (no WASM)
 * - Unit testing
 * - Development without VSCode
 */
export class MockDataProvider implements DataProvider {
  private config: MockConfig;
  private workflows: WorkflowDefinition[];
  private executions: Map<string, ExecutionSnapshot[]> = new Map();
  private executionCount = 0;
  private abortControllers: Map<string, AbortController> = new Map();
  private settings: Record<string, any> = {};

  constructor(config?: Partial<MockConfig>) {
    this.config = {
      cacheHitRate: config?.cacheHitRate ?? 0.3,
      errorRate: config?.errorRate ?? 0.1,
      verboseLogging: config?.verboseLogging ?? true,
      speedMultiplier: config?.speedMultiplier ?? 1,
    };

    // Initialize mock workflows
    this.workflows = this.createSampleWorkflows();

    console.log('[MockProvider] Initialized with', this.workflows.length, 'workflows');
  }

  // ============================================================================
  // WORKFLOW DATA
  // ============================================================================

  async getWorkflows(): Promise<WorkflowDefinition[]> {
    await this.delay(50); // Simulate network latency
    return this.workflows;
  }

  async getWorkflow(workflowId: string): Promise<WorkflowDefinition | null> {
    await this.delay(30);
    return this.workflows.find((w) => w.id === workflowId) ?? null;
  }

  // ============================================================================
  // FILE SYSTEM & CODE
  // ============================================================================

  async getBAMLFiles(): Promise<BAMLFile[]> {
    await this.delay(50);
    return this.createSampleBAMLFiles();
  }

  async getFileContent(filePath: string): Promise<string> {
    await this.delay(30);
    // Return mock file content
    return `// Mock content for ${filePath}\nfunction example() {\n  return "mock";\n}`;
  }

  watchFiles(callback: (files: Record<string, string>) => void): () => void {
    // Mock file watching - no-op
    console.log('[MockProvider] watchFiles called (no-op in mock mode)');
    return () => {
      console.log('[MockProvider] File watch unsubscribed');
    };
  }

  // ============================================================================
  // EXECUTION
  // ============================================================================

  async getExecutions(workflowId: string): Promise<ExecutionSnapshot[]> {
    await this.delay(30);
    return this.executions.get(workflowId) ?? [];
  }

  async *executeWorkflow(
    workflowId: string,
    inputs: Record<string, any>,
    options?: ExecutionOptions
  ): AsyncGenerator<BAMLEvent> {
    const workflow = await this.getWorkflow(workflowId);
    if (!workflow) {
      throw new Error(`Workflow ${workflowId} not found`);
    }

    this.executionCount++;
    const executionId = `exec_${Date.now()}_${this.executionCount}`;

    // Create abort controller for this execution
    const abortController = new AbortController();
    this.abortControllers.set(executionId, abortController);

    try {
      // Emit start event
      yield {
        type: 'execution.started',
        executionId,
        workflowId,
      };

      // Simulate execution with event streaming
      yield* this.simulateWorkflowExecution(
        workflow,
        executionId,
        inputs,
        options,
        abortController.signal
      );

      // Emit completion event
      yield {
        type: 'execution.completed',
        executionId,
        duration: 1000,
        outputs: { result: 'completed' },
      };
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        yield {
          type: 'execution.cancelled',
          executionId,
          reason: 'User cancelled',
        };
      } else {
        yield {
          type: 'execution.error',
          executionId,
          error: error as Error,
        };
      }
    } finally {
      this.abortControllers.delete(executionId);
    }
  }

  async cancelExecution(executionId: string): Promise<void> {
    const controller = this.abortControllers.get(executionId);
    if (controller) {
      console.log('[MockProvider] Cancelling execution', executionId);
      controller.abort();
      this.abortControllers.delete(executionId);
    }
  }

  // ============================================================================
  // TEST EXECUTION
  // ============================================================================

  async getTestCases(functionName: string): Promise<TestCaseInput[]> {
    await this.delay(30);

    // Return mock test cases based on function
    const testCases: Record<string, TestCaseInput[]> = {
      fetchData: [
        {
          id: 'test_fetchData_success',
          name: 'success_case',
          source: 'test',
          nodeId: 'fetchData',
          filePath: 'tests/fetchData.test.ts',
          inputs: { url: 'https://api.example.com/data' },
          expectedOutput: { status: 200, data: {} },
          status: 'passing',
          lastRun: Date.now() - 3600000,
        },
      ],
      processData: [
        {
          id: 'test_processData_valid',
          name: 'valid_input',
          source: 'test',
          nodeId: 'processData',
          filePath: 'tests/processData.test.ts',
          inputs: { data: { id: 1 } },
          expectedOutput: { processed: true },
          status: 'passing',
          lastRun: Date.now() - 1800000,
        },
      ],
    };

    return testCases[functionName] ?? [];
  }

  async *runTest(
    functionName: string,
    testName: string
  ): AsyncGenerator<TestExecutionEvent> {
    yield { type: 'test.started', functionName, testName, timestamp: Date.now() };

    await this.delay(500 * this.config.speedMultiplier);

    const shouldError = Math.random() < this.config.errorRate;

    if (shouldError) {
      yield {
        type: 'test.error',
        functionName,
        testName,
        error: 'Test failed: Mock error',
      };
    } else {
      yield {
        type: 'test.completed',
        functionName,
        testName,
        duration: 500,
        response: { mockResult: 'success' },
        status: 'passed',
      };
    }
  }

  async *runTests(
    tests: Array<{ functionName: string; testName: string }>,
    options?: { parallel?: boolean }
  ): AsyncGenerator<TestExecutionEvent> {
    if (options?.parallel) {
      // Simulate parallel execution
      for (const test of tests) {
        yield* this.runTest(test.functionName, test.testName);
      }
    } else {
      // Sequential execution
      for (const test of tests) {
        yield* this.runTest(test.functionName, test.testName);
      }
    }
  }

  async cancelTests(): Promise<void> {
    console.log('[MockProvider] cancelTests called (no-op in mock mode)');
  }

  // ============================================================================
  // GRAPH & STRUCTURE
  // ============================================================================

  async getGraph(workflowId: string): Promise<{
    nodes: GraphNode[];
    edges: GraphEdge[];
  }> {
    const workflow = await this.getWorkflow(workflowId);
    return {
      nodes: workflow?.nodes ?? [],
      edges: workflow?.edges ?? [],
    };
  }

  async getFunctions(): Promise<BAMLFunction[]> {
    const files = await this.getBAMLFiles();
    return files.flatMap((file) => file.functions);
  }

  // ============================================================================
  // CACHE MANAGEMENT
  // ============================================================================

  async getCacheEntries(nodeId: string): Promise<CacheEntry[]> {
    // Mock: no cache persistence
    return [];
  }

  async saveCacheEntry(entry: CacheEntry): Promise<void> {
    // Mock: no-op
    console.log('[MockProvider] saveCacheEntry called (no-op)');
  }

  async clearCache(scope: 'all' | 'workflow' | 'node', id?: string): Promise<void> {
    console.log('[MockProvider] clearCache called', { scope, id });
  }

  // ============================================================================
  // NAVIGATION & CODE SYNC
  // ============================================================================

  async navigateToCode(position: CodePosition): Promise<void> {
    console.log('[MockProvider] Navigate to:', position);
  }

  async highlightCode(ranges: CodePosition[]): Promise<void> {
    console.log('[MockProvider] Highlight:', ranges.length, 'ranges');
  }

  // ============================================================================
  // SETTINGS & CONFIGURATION
  // ============================================================================

  async getSettings(): Promise<Record<string, any>> {
    return {
      ...this.settings,
      // Default settings
      theme: 'dark',
      autoSave: true,
      parallelTests: false,
    };
  }

  async updateSetting(key: string, value: any): Promise<void> {
    this.settings[key] = value;
    console.log('[MockProvider] Setting updated:', key, '=', value);
  }

  // ============================================================================
  // RUNTIME & COMPILATION
  // ============================================================================

  async getRuntimeVersion(): Promise<string> {
    return 'mock-runtime-v1.0.0';
  }

  async getDiagnostics(): Promise<Diagnostic[]> {
    // Return empty diagnostics (no errors in mock mode)
    return [];
  }

  async compile(): Promise<void> {
    await this.delay(200);
    console.log('[MockProvider] Compilation complete (mock)');
  }

  // ============================================================================
  // LIFECYCLE
  // ============================================================================

  async initialize(): Promise<void> {
    console.log('[MockProvider] Initializing...');
    await this.delay(100);
    console.log('[MockProvider] Initialized');
  }

  async dispose(): Promise<void> {
    console.log('[MockProvider] Disposing...');
    // Cancel all running executions
    for (const [executionId, controller] of this.abortControllers.entries()) {
      controller.abort();
    }
    this.abortControllers.clear();
  }

  // ============================================================================
  // PRIVATE HELPERS
  // ============================================================================

  /**
   * Simulate workflow execution with realistic event streaming
   * Source: apps/baml-graph/src/sdk/mock.ts:590-701
   */
  private async *simulateWorkflowExecution(
    workflow: WorkflowDefinition,
    executionId: string,
    inputs: Record<string, any>,
    options?: ExecutionOptions,
    signal?: AbortSignal
  ): AsyncGenerator<BAMLEvent> {
    const visited = new Set<string>();
    const startFromNode = options && 'startFromNodeId' in options ? options.startFromNodeId : undefined;
    let currentNodes = [startFromNode || workflow.entryPoint];
    const context: Record<string, unknown> = { ...inputs };

    while (currentNodes.length > 0) {
      // Check for cancellation
      if (signal?.aborted) {
        throw new Error('Execution cancelled');
      }

      const nextNodes: string[] = [];

      for (const nodeId of currentNodes) {
        const node = workflow.nodes.find((n) => n.id === nodeId);
        if (!node || visited.has(nodeId)) continue;

        visited.add(nodeId);

        // Execute node
        yield* this.executeNode(node, executionId, context, workflow);

        // Determine next nodes
        const outgoingEdges = workflow.edges.filter((e) => e.source === nodeId);
        nextNodes.push(...outgoingEdges.map((e) => e.target));
      }

      currentNodes = nextNodes;
      await this.delay(100 * this.config.speedMultiplier);
    }
  }

  /**
   * Execute a single node
   * Source: apps/baml-graph/src/sdk/mock.ts:706-811
   */
  private async *executeNode(
    node: GraphNode,
    executionId: string,
    context: Record<string, unknown>,
    workflow: WorkflowDefinition
  ): AsyncGenerator<BAMLEvent> {
    const nodeInputs = { ...context };

    // Emit start
    yield {
      type: 'node.started',
      executionId,
      nodeId: node.id,
      inputs: nodeInputs,
    };

    // Simulate processing
    const duration = this.getNodeDuration(node.type);
    await this.delay(duration * this.config.speedMultiplier);

    // Check for cache hit
    if (Math.random() < this.config.cacheHitRate) {
      yield {
        type: 'node.cached',
        executionId,
        nodeId: node.id,
        fromExecutionId: 'exec_cached',
      };
    }

    // Check for errors
    const shouldError = Math.random() < this.config.errorRate;
    if (shouldError) {
      yield {
        type: 'node.error',
        executionId,
        nodeId: node.id,
        error: new Error(`Node ${node.id} failed`),
      };
      return;
    }

    // Emit completion
    const outputs = this.generateMockOutputs(node, workflow, context);
    yield {
      type: 'node.completed',
      executionId,
      nodeId: node.id,
      inputs: nodeInputs,
      outputs,
      duration,
    };

    // Update context
    Object.assign(context, outputs);
  }

  /**
   * Get node execution duration based on type
   */
  private getNodeDuration(nodeType: GraphNode['type']): number {
    switch (nodeType) {
      case 'llm_function':
        return 1500 + Math.random() * 1000;
      case 'function':
        return 400 + Math.random() * 300;
      case 'conditional':
        return 300 + Math.random() * 200;
      default:
        return 500 + Math.random() * 500;
    }
  }

  /**
   * Generate mock outputs for a node
   */
  private generateMockOutputs(
    node: GraphNode,
    workflow: WorkflowDefinition,
    context: Record<string, unknown>
  ): Record<string, unknown> {
    switch (node.type) {
      case 'llm_function':
        return {
          result: `AI response for ${node.label}`,
          tokens: Math.floor(Math.random() * 500) + 100,
          model: 'gpt-4',
        };
      case 'function':
        return {
          success: true,
          data: { timestamp: Date.now() },
        };
      case 'conditional':
        return {
          condition: Math.random() > 0.5 ? 'success' : 'failure',
        };
      default:
        return { completed: true };
    }
  }

  /**
   * Create sample workflows
   * Source: apps/baml-graph/src/sdk/mock.ts:158-231
   */
  private createSampleWorkflows(): WorkflowDefinition[] {
    return [
      {
        id: 'simpleWorkflow',
        displayName: 'Simple Workflow',
        filePath: '/mock/simpleWorkflow.baml',
        startLine: 1,
        endLine: 100,
        nodes: [
          {
            id: 'fetchData',
            type: 'function',
            label: 'Fetch Data',
            functionName: 'fetchData',
            codeHash: 'hash_fetchData',
            lastModified: Date.now(),
          },
          {
            id: 'processData',
            type: 'llm_function',
            label: 'Process Data',
            functionName: 'processData',
            llmClient: 'GPT-4o',
            codeHash: 'hash_processData',
            lastModified: Date.now(),
          },
          {
            id: 'saveResult',
            type: 'function',
            label: 'Save Result',
            functionName: 'saveResult',
            codeHash: 'hash_saveResult',
            lastModified: Date.now(),
          },
        ],
        edges: [
          { id: 'edge_0', source: 'fetchData', target: 'processData' },
          { id: 'edge_1', source: 'processData', target: 'saveResult' },
        ],
        entryPoint: 'fetchData',
        parameters: [{ name: 'input', type: 'string', optional: false }],
        returnType: 'any',
        childFunctions: ['fetchData', 'processData', 'saveResult'],
        lastModified: Date.now(),
        codeHash: 'hash_simpleWorkflow',
      },
      // Add more sample workflows as needed
    ];
  }

  /**
   * Create sample BAML files
   */
  private createSampleBAMLFiles(): BAMLFile[] {
    return [
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
        ],
        tests: [
          {
            name: 'test_fetchData_success',
            functionName: 'fetchData',
            filePath: 'workflows/simple.baml',
            nodeType: 'function',
          },
        ],
      },
    ];
  }

  /**
   * Simulate workflow execution (required by MockDataProvider interface)
   * This is a simple wrapper around executeWorkflow for compatibility
   */
  async *simulateExecution(
    workflowId: string,
    inputs: Record<string, any>,
    startFromNodeId?: string
  ): AsyncGenerator<BAMLEvent> {
    // Simply delegate to executeWorkflow
    yield* this.executeWorkflow(workflowId, inputs);
  }

  /**
   * Delay helper
   */
  private delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

/**
 * Create mock provider with default config
 */
export function createMockProvider(config?: Partial<MockConfig>): DataProvider {
  return new MockDataProvider(config);
}

/**
 * Create fast mock provider (for testing)
 */
export function createFastMockProvider(): DataProvider {
  return new MockDataProvider({
    speedMultiplier: 0.1,
    verboseLogging: false,
    cacheHitRate: 0,
    errorRate: 0,
  });
}
