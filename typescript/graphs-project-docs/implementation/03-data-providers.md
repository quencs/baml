# Phase 3: Data Provider Abstraction

**Timeline:** Week 3
**Dependencies:** Phase 2 (SDK Integration)
**Risk Level:** Medium

## Purpose

Implement the DataProvider pattern that abstracts data sources (mock data, VSCode WASM runtime, future server API). This enables the SDK to work in different modes without changing business logic, making the system testable, portable, and extensible.

## What This Document Will Cover

- Complete `DataProvider` interface definition with all methods
- `MockDataProvider` implementation (from baml-graph, battle-tested)
- `VSCodeDataProvider` implementation (wraps playground-common WASM)
- Provider selection and initialization logic
- Mock data structure and management
- WASM runtime integration patterns
- Error handling and fallback strategies
- Provider testing strategies
- Extension pattern for future providers (server, standalone)

## Key Decisions

1. **Single Interface**: One `DataProvider` interface for all modes
2. **Async Methods**: All data operations return Promises for consistency
3. **MockProvider**: Uses hardcoded TypeScript data (from baml-graph)
4. **VSCodeProvider**: Wraps existing WASM runtime atoms (no duplication)
5. **Injection Pattern**: Provider injected into SDK at construction
6. **Stateless Providers**: Providers don't hold state, they access external sources
7. **Error Handling**: Providers throw errors, SDK handles them

## Source Files to Reference

### From baml-graph (Mock Provider)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/mock.ts` (lines 93-1094 - complete mock implementation)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/types.ts` (lines 305-318 - MockDataProvider interface)

### From playground-common (WASM Integration)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/atoms.ts` (lines 121-339 - WASM runtime)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/vscode.ts` (lines 63-535 - VSCode API)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/test-runner.ts` (lines 538-629 - test execution)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC.md` (lines 1380-1546 - Data Provider Interface section)
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC_ANSWERS.md` (lines 8-95 - WASM runtime support answer)

---

## Part 1: DataProvider Interface

### 1.1 Complete Interface Definition

The `DataProvider` interface abstracts all external data sources the SDK needs.

```typescript
/**
 * Data Provider Interface
 *
 * Abstracts data sources for the SDK:
 * - MockDataProvider: Hardcoded data for browser/testing
 * - VSCodeDataProvider: WASM runtime integration
 * - ServerDataProvider: Future remote API (not implemented yet)
 *
 * All methods are async to support various backends.
 */
export interface DataProvider {
  // ============================================================================
  // WORKFLOW DATA
  // ============================================================================

  /**
   * Get all available workflows
   *
   * Mock: Returns hardcoded sample workflows
   * VSCode: Parses workflows from WASM runtime
   */
  getWorkflows(): Promise<WorkflowDefinition[]>;

  /**
   * Get workflow by ID
   */
  getWorkflow(workflowId: string): Promise<WorkflowDefinition | null>;

  // ============================================================================
  // FILE SYSTEM & CODE
  // ============================================================================

  /**
   * Get all BAML files in the project
   *
   * Mock: Returns sample file structure
   * VSCode: Returns actual files from WASM runtime
   */
  getBAMLFiles(): Promise<BAMLFile[]>;

  /**
   * Get file content
   */
  getFileContent(filePath: string): Promise<string>;

  /**
   * Watch file changes
   * Returns unsubscribe function
   */
  watchFiles(callback: (files: Record<string, string>) => void): () => void;

  // ============================================================================
  // EXECUTION
  // ============================================================================

  /**
   * Get execution history for a workflow
   *
   * Mock: Returns empty array (no persistence)
   * VSCode: Could integrate with extension storage
   */
  getExecutions(workflowId: string): Promise<ExecutionSnapshot[]>;

  /**
   * Start workflow execution
   * Returns async generator that yields events
   *
   * Mock: Simulates execution with realistic timing
   * VSCode: Executes via WASM runtime
   */
  executeWorkflow(
    workflowId: string,
    inputs: Record<string, any>,
    options?: ExecutionOptions
  ): AsyncGenerator<BAMLEvent>;

  /**
   * Cancel running execution
   */
  cancelExecution(executionId: string): Promise<void>;

  // ============================================================================
  // TEST EXECUTION
  // ============================================================================

  /**
   * Get test cases for a function/node
   *
   * Mock: Returns sample test cases
   * VSCode: Parses tests from WASM runtime
   */
  getTestCases(functionName: string): Promise<TestCaseInput[]>;

  /**
   * Run a single test case
   * Returns async generator that yields test events
   */
  runTest(
    functionName: string,
    testName: string
  ): AsyncGenerator<TestExecutionEvent>;

  /**
   * Run multiple tests
   * Can run in parallel or sequential
   */
  runTests(
    tests: Array<{ functionName: string; testName: string }>,
    options?: { parallel?: boolean }
  ): AsyncGenerator<TestExecutionEvent>;

  /**
   * Cancel running tests
   */
  cancelTests(): Promise<void>;

  // ============================================================================
  // GRAPH & STRUCTURE
  // ============================================================================

  /**
   * Get graph structure for a workflow
   * Returns nodes and edges
   */
  getGraph(workflowId: string): Promise<{
    nodes: GraphNode[];
    edges: GraphEdge[];
  }>;

  /**
   * Get all functions (for function map)
   */
  getFunctions(): Promise<BAMLFunction[]>;

  // ============================================================================
  // CACHE MANAGEMENT
  // ============================================================================

  /**
   * Get cache entries for a node
   *
   * Mock: Returns empty (no caching)
   * VSCode: Could integrate with extension storage
   */
  getCacheEntries(nodeId: string): Promise<CacheEntry[]>;

  /**
   * Save cache entry
   */
  saveCacheEntry(entry: CacheEntry): Promise<void>;

  /**
   * Clear cache
   */
  clearCache(scope: 'all' | 'workflow' | 'node', id?: string): Promise<void>;

  // ============================================================================
  // NAVIGATION & CODE SYNC
  // ============================================================================

  /**
   * Navigate to code location
   *
   * Mock: Logs to console
   * VSCode: Jumps to file in editor
   */
  navigateToCode(position: CodePosition): Promise<void>;

  /**
   * Highlight code ranges
   */
  highlightCode(ranges: CodePosition[]): Promise<void>;

  // ============================================================================
  // SETTINGS & CONFIGURATION
  // ============================================================================

  /**
   * Get settings
   *
   * Mock: Returns default settings
   * VSCode: Reads from VSCode settings
   */
  getSettings(): Promise<Record<string, any>>;

  /**
   * Update setting
   */
  updateSetting(key: string, value: any): Promise<void>;

  // ============================================================================
  // RUNTIME & COMPILATION
  // ============================================================================

  /**
   * Get WASM runtime version
   */
  getRuntimeVersion(): Promise<string>;

  /**
   * Get compilation diagnostics
   */
  getDiagnostics(): Promise<Diagnostic[]>;

  /**
   * Trigger compilation
   */
  compile(): Promise<void>;

  // ============================================================================
  // LIFECYCLE
  // ============================================================================

  /**
   * Initialize provider
   * Called once when SDK starts
   */
  initialize(): Promise<void>;

  /**
   * Cleanup provider
   * Called when SDK is disposed
   */
  dispose(): Promise<void>;
}
```

### 1.2 Provider Types

```typescript
/**
 * Test execution event
 */
export type TestExecutionEvent =
  | { type: 'test.started'; functionName: string; testName: string }
  | { type: 'test.completed'; functionName: string; testName: string; duration: number; passed: boolean }
  | { type: 'test.error'; functionName: string; testName: string; error: Error }
  | { type: 'test.log'; functionName: string; testName: string; message: string };

/**
 * Diagnostic from compilation
 */
export interface Diagnostic {
  level: 'error' | 'warning' | 'info';
  message: string;
  filePath: string;
  line: number;
  column: number;
}

/**
 * Execution options
 */
export interface ExecutionOptions {
  startFromNodeId?: string;
  cachePolicy?: 'auto' | 'always-run' | 'always-cache';
  timeout?: number;
}
```

---

## Part 2: MockDataProvider Implementation

### 2.1 mock-provider.ts - Complete Implementation

```typescript
/**
 * Mock Data Provider
 *
 * Source: apps/baml-graph/src/sdk/mock.ts:93-1094
 * Provides realistic mock data for browser mode and testing
 */

import type {
  DataProvider,
  WorkflowDefinition,
  ExecutionSnapshot,
  BAMLEvent,
  TestCaseInput,
  TestExecutionEvent,
  GraphNode,
  GraphEdge,
  BAMLFile,
  BAMLFunction,
  CacheEntry,
  CodePosition,
  Diagnostic,
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
    yield { type: 'test.started', functionName, testName };

    await this.delay(500 * this.config.speedMultiplier);

    const shouldError = Math.random() < this.config.errorRate;

    if (shouldError) {
      yield {
        type: 'test.error',
        functionName,
        testName,
        error: new Error('Test failed: Mock error'),
      };
    } else {
      yield {
        type: 'test.completed',
        functionName,
        testName,
        duration: 500,
        passed: true,
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
    let currentNodes = [options?.startFromNodeId || workflow.entryPoint];
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
```

---

## Part 3: VSCodeDataProvider Implementation

### 3.1 vscode-provider.ts - Complete Implementation

```typescript
/**
 * VSCode Data Provider
 *
 * Wraps playground-common WASM runtime for SDK access
 * Integrates with existing EventListener and atoms
 */

import type { Store } from 'jotai/vanilla';
import type {
  DataProvider,
  WorkflowDefinition,
  ExecutionSnapshot,
  BAMLEvent,
  TestCaseInput,
  TestExecutionEvent,
  GraphNode,
  GraphEdge,
  BAMLFile,
  BAMLFunction,
  CacheEntry,
  CodePosition,
  Diagnostic,
  ExecutionOptions,
} from '../types';
import {
  wasmAtom,
  runtimeAtom,
  diagnosticsAtom,
  filesAtom,
} from '../../shared/atoms';
import { vscode } from '../../shared/baml-project-panel/vscode';

/**
 * VSCode Data Provider Implementation
 *
 * Wraps existing WASM runtime and VSCode API
 * Does NOT duplicate functionality - delegates to existing systems
 */
export class VSCodeDataProvider implements DataProvider {
  private store: Store;
  private abortController: AbortController | null = null;

  constructor(store: Store) {
    this.store = store;
    console.log('[VSCodeProvider] Created');
  }

  // ============================================================================
  // WORKFLOW DATA
  // ============================================================================

  async getWorkflows(): Promise<WorkflowDefinition[]> {
    const runtime = this.store.get(runtimeAtom);
    if (!runtime) {
      console.warn('[VSCodeProvider] Runtime not available');
      return [];
    }

    try {
      // Get functions from runtime
      const functions = runtime.getFunctions?.() ?? [];

      // Convert to workflow definitions
      const workflows = functions
        .filter((fn: any) => fn.type === 'workflow')
        .map((fn: any) => this.convertToWorkflowDefinition(fn));

      return workflows;
    } catch (error) {
      console.error('[VSCodeProvider] Failed to get workflows', error);
      return [];
    }
  }

  async getWorkflow(workflowId: string): Promise<WorkflowDefinition | null> {
    const workflows = await this.getWorkflows();
    return workflows.find((w) => w.id === workflowId) ?? null;
  }

  // ============================================================================
  // FILE SYSTEM & CODE
  // ============================================================================

  async getBAMLFiles(): Promise<BAMLFile[]> {
    const runtime = this.store.get(runtimeAtom);
    if (!runtime) return [];

    try {
      // Get files from runtime
      const files = runtime.getFiles?.() ?? [];
      return files.map((file: any) => this.convertToBAMLFile(file));
    } catch (error) {
      console.error('[VSCodeProvider] Failed to get BAML files', error);
      return [];
    }
  }

  async getFileContent(filePath: string): Promise<string> {
    const files = this.store.get(filesAtom);
    return files[filePath] ?? '';
  }

  watchFiles(callback: (files: Record<string, string>) => void): () => void {
    // Subscribe to files atom
    const unsubscribe = this.store.sub(filesAtom, () => {
      const files = this.store.get(filesAtom);
      callback(files);
    });

    return unsubscribe;
  }

  // ============================================================================
  // EXECUTION
  // ============================================================================

  async getExecutions(workflowId: string): Promise<ExecutionSnapshot[]> {
    // TODO: Could integrate with extension storage
    return [];
  }

  async *executeWorkflow(
    workflowId: string,
    inputs: Record<string, any>,
    options?: ExecutionOptions
  ): AsyncGenerator<BAMLEvent> {
    const runtime = this.store.get(runtimeAtom);
    if (!runtime) {
      throw new Error('WASM runtime not available');
    }

    const executionId = `exec_${Date.now()}`;

    try {
      yield {
        type: 'execution.started',
        executionId,
        workflowId,
      };

      // Execute via WASM runtime
      // TODO: Integrate with actual WASM execution
      // For now, delegate to existing test runner pattern
      console.log('[VSCodeProvider] Executing workflow', workflowId, 'with inputs', inputs);

      // Simulate completion
      yield {
        type: 'execution.completed',
        executionId,
        duration: 1000,
        outputs: {},
      };
    } catch (error) {
      yield {
        type: 'execution.error',
        executionId,
        error: error as Error,
      };
    }
  }

  async cancelExecution(executionId: string): Promise<void> {
    if (this.abortController) {
      this.abortController.abort();
      this.abortController = null;
    }
  }

  // ============================================================================
  // TEST EXECUTION
  // ============================================================================

  async getTestCases(functionName: string): Promise<TestCaseInput[]> {
    const runtime = this.store.get(runtimeAtom);
    if (!runtime) return [];

    try {
      // Get tests from runtime
      const tests = runtime.getTests?.(functionName) ?? [];
      return tests.map((test: any) => this.convertToTestCase(test, functionName));
    } catch (error) {
      console.error('[VSCodeProvider] Failed to get test cases', error);
      return [];
    }
  }

  async *runTest(
    functionName: string,
    testName: string
  ): AsyncGenerator<TestExecutionEvent> {
    const runtime = this.store.get(runtimeAtom);
    if (!runtime) {
      throw new Error('WASM runtime not available');
    }

    yield { type: 'test.started', functionName, testName };

    try {
      // Run test via WASM runtime
      // Integration point: playground-common/.../test-runner.ts:595-612
      const result = await runtime.runTest?.(functionName, testName);

      yield {
        type: 'test.completed',
        functionName,
        testName,
        duration: result?.duration ?? 0,
        passed: result?.passed ?? false,
      };
    } catch (error) {
      yield {
        type: 'test.error',
        functionName,
        testName,
        error: error as Error,
      };
    }
  }

  async *runTests(
    tests: Array<{ functionName: string; testName: string }>,
    options?: { parallel?: boolean }
  ): AsyncGenerator<TestExecutionEvent> {
    // Delegate to existing test runner
    // Integration point: playground-common/.../test-runner.ts:538-629
    for (const test of tests) {
      yield* this.runTest(test.functionName, test.testName);
    }
  }

  async cancelTests(): Promise<void> {
    // Integrate with existing test cancellation
    // Integration point: playground-common/.../test-runner.ts:614-626
    console.log('[VSCodeProvider] Cancelling tests');
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
    // TODO: Could integrate with extension storage
    return [];
  }

  async saveCacheEntry(entry: CacheEntry): Promise<void> {
    console.log('[VSCodeProvider] saveCacheEntry (not implemented)');
  }

  async clearCache(scope: 'all' | 'workflow' | 'node', id?: string): Promise<void> {
    console.log('[VSCodeProvider] clearCache', { scope, id });
  }

  // ============================================================================
  // NAVIGATION & CODE SYNC
  // ============================================================================

  async navigateToCode(position: CodePosition): Promise<void> {
    // Integration point: playground-common/.../vscode.ts:154-183
    await vscode.jumpToFile({
      file_path: position.filePath,
      start_line: position.line,
      start_column: position.column,
      end_line: position.line,
      end_column: position.column,
    });
  }

  async highlightCode(ranges: CodePosition[]): Promise<void> {
    // Integration point: playground-common/.../vscode.ts:331-343
    const spans = ranges.map((range) => ({
      file_path: range.filePath,
      start_line: range.line,
      start: range.column,
      end_line: range.line,
      end: range.column,
    }));

    await vscode.setFlashingRegions(spans);
  }

  // ============================================================================
  // SETTINGS & CONFIGURATION
  // ============================================================================

  async getSettings(): Promise<Record<string, any>> {
    // Integration point: playground-common/.../vscode.ts:257-274
    const settings = await vscode.getVSCodeSettings();
    return settings;
  }

  async updateSetting(key: string, value: any): Promise<void> {
    // TODO: Integrate with VSCode settings update
    console.log('[VSCodeProvider] updateSetting', key, value);
  }

  // ============================================================================
  // RUNTIME & COMPILATION
  // ============================================================================

  async getRuntimeVersion(): Promise<string> {
    const wasm = this.store.get(wasmAtom);
    return wasm?.version() ?? 'unknown';
  }

  async getDiagnostics(): Promise<Diagnostic[]> {
    const diagnostics = this.store.get(diagnosticsAtom);
    return diagnostics.map((d: any) => ({
      level: d.type === 'error' ? 'error' : d.type === 'warning' ? 'warning' : 'info',
      message: d.message,
      filePath: d.file,
      line: d.line,
      column: d.column,
    }));
  }

  async compile(): Promise<void> {
    // Trigger recompilation via WASM
    const wasm = this.store.get(wasmAtom);
    if (!wasm) {
      throw new Error('WASM not loaded');
    }

    // TODO: Trigger actual compilation
    console.log('[VSCodeProvider] Compilation triggered');
  }

  // ============================================================================
  // LIFECYCLE
  // ============================================================================

  async initialize(): Promise<void> {
    console.log('[VSCodeProvider] Initializing...');

    // Wait for WASM to be ready
    const wasm = this.store.get(wasmAtom);
    if (!wasm) {
      console.warn('[VSCodeProvider] WASM not loaded yet');
    }

    console.log('[VSCodeProvider] Initialized');
  }

  async dispose(): Promise<void> {
    console.log('[VSCodeProvider] Disposing...');
    if (this.abortController) {
      this.abortController.abort();
    }
  }

  // ============================================================================
  // PRIVATE HELPERS
  // ============================================================================

  /**
   * Convert WASM function to workflow definition
   */
  private convertToWorkflowDefinition(fn: any): WorkflowDefinition {
    return {
      id: fn.name,
      displayName: fn.name,
      filePath: fn.filePath ?? '',
      startLine: fn.span?.start_line ?? 0,
      endLine: fn.span?.end_line ?? 0,
      nodes: [], // TODO: Parse from function body
      edges: [], // TODO: Parse from function body
      entryPoint: '',
      parameters: fn.parameters ?? [],
      returnType: fn.returnType ?? 'any',
      childFunctions: [],
      lastModified: Date.now(),
      codeHash: this.computeHash(fn),
    };
  }

  /**
   * Convert WASM file to BAMLFile
   */
  private convertToBAMLFile(file: any): BAMLFile {
    return {
      path: file.path,
      functions: file.functions ?? [],
      tests: file.tests ?? [],
    };
  }

  /**
   * Convert WASM test to TestCaseInput
   */
  private convertToTestCase(test: any, functionName: string): TestCaseInput {
    return {
      id: test.id ?? test.name,
      name: test.name,
      source: 'test',
      nodeId: functionName,
      filePath: test.filePath ?? '',
      inputs: test.inputs ?? {},
      expectedOutput: test.expectedOutput,
      status: 'unknown',
    };
  }

  /**
   * Compute hash for cache invalidation
   */
  private computeHash(data: any): string {
    return `hash_${JSON.stringify(data)}_${Date.now()}`;
  }
}

/**
 * Create VSCode provider
 */
export function createVSCodeProvider(store: Store): DataProvider {
  return new VSCodeDataProvider(store);
}
```

---

## Part 4: Provider Factory and Selection

### 4.1 provider-factory.ts

```typescript
/**
 * Provider Factory
 *
 * Selects and creates the appropriate data provider based on environment
 */

import type { Store } from 'jotai/vanilla';
import type { DataProvider } from '../types';
import { createMockProvider, type MockConfig } from './mock-provider';
import { createVSCodeProvider } from './vscode-provider';

/**
 * Provider mode
 */
export type ProviderMode = 'mock' | 'vscode' | 'server';

/**
 * Provider configuration
 */
export interface ProviderConfig {
  mode: ProviderMode;
  mockConfig?: Partial<MockConfig>;
  serverUrl?: string;
}

/**
 * Auto-detect provider mode based on environment
 */
export function detectProviderMode(): ProviderMode {
  // Check if running in VSCode webview
  if (typeof acquireVsCodeApi === 'function') {
    return 'vscode';
  }

  // Check if server is available
  // TODO: Ping server to check availability

  // Default to mock mode
  return 'mock';
}

/**
 * Create data provider based on config
 */
export function createDataProvider(
  config: ProviderConfig,
  store?: Store
): DataProvider {
  console.log('[ProviderFactory] Creating provider:', config.mode);

  switch (config.mode) {
    case 'mock':
      return createMockProvider(config.mockConfig);

    case 'vscode':
      if (!store) {
        throw new Error('Store required for VSCode provider');
      }
      return createVSCodeProvider(store);

    case 'server':
      // TODO: Implement server provider
      throw new Error('Server provider not implemented yet');

    default:
      throw new Error(`Unknown provider mode: ${config.mode}`);
  }
}

/**
 * Create provider with auto-detection
 */
export function createAutoProvider(store?: Store): DataProvider {
  const mode = detectProviderMode();
  return createDataProvider({ mode }, store);
}
```

### 4.2 Integration with SDK

Update SDK to accept provider:

```typescript
/**
 * Updated SDK constructor to accept provider
 */
export class BAMLSDK {
  private store: Store;
  private provider: DataProvider;
  private eventEmitter: EventEmitter;
  private isInitialized = false;

  constructor(provider: DataProvider, store: Store) {
    this.provider = provider;
    this.store = store;
    this.eventEmitter = new EventEmitter();

    console.log('[BAMLSDK] Created with provider');
  }

  // All SDK methods now delegate to provider
  async initialize(): Promise<void> {
    if (this.isInitialized) return;

    console.log('[BAMLSDK] Initializing...');

    // Initialize provider
    await this.provider.initialize();

    // Load initial data
    const workflows = await this.provider.getWorkflows();
    this.store.set(workflowsAtom, workflows);

    this.isInitialized = true;
    console.log('[BAMLSDK] Initialized with', workflows.length, 'workflows');
  }

  // Workflow methods delegate to provider
  workflows = {
    getAll: async (): Promise<WorkflowDefinition[]> => {
      return await this.provider.getWorkflows();
    },

    // ... other methods
  };

  // Execution methods delegate to provider
  executions = {
    start: async (
      workflowId: string,
      inputs: Record<string, any>,
      options?: ExecutionOptions
    ): Promise<string> => {
      const executionId = `exec_${Date.now()}`;

      // Start execution via provider
      const generator = this.provider.executeWorkflow(workflowId, inputs, options);

      // Process events
      (async () => {
        for await (const event of generator) {
          this.emit(event);
        }
      })();

      return executionId;
    },

    // ... other methods
  };
}

/**
 * Updated factory function
 */
export function createBAMLSDK(
  provider: DataProvider,
  store: Store
): BAMLSDK {
  return new BAMLSDK(provider, store);
}
```

### 4.3 Updated Provider Component

```typescript
/**
 * Updated BAMLSDKProvider with provider injection
 */
export function BAMLSDKProvider({ children, config }: BAMLSDKProviderProps) {
  const storeRef = useRef<ReturnType<typeof createStore>>();
  const providerRef = useRef<DataProvider>();
  const sdkRef = useRef<BAMLSDK>();

  if (!storeRef.current) {
    storeRef.current = createStore();
  }

  if (!providerRef.current) {
    // Create provider based on config
    const providerConfig: ProviderConfig = config?.providerConfig ?? {
      mode: detectProviderMode(),
    };

    providerRef.current = createDataProvider(providerConfig, storeRef.current);
  }

  if (!sdkRef.current) {
    sdkRef.current = createBAMLSDK(providerRef.current, storeRef.current);
  }

  // ... rest of provider code
}
```

---

## Part 5: Testing Strategy

### 5.1 Provider Unit Tests

```typescript
/**
 * Mock Provider Tests
 */
describe('MockDataProvider', () => {
  let provider: DataProvider;

  beforeEach(() => {
    provider = createMockProvider({
      speedMultiplier: 0.1, // Fast for testing
      errorRate: 0, // No errors
      cacheHitRate: 0,
    });
  });

  afterEach(async () => {
    await provider.dispose();
  });

  describe('getWorkflows', () => {
    it('should return sample workflows', async () => {
      const workflows = await provider.getWorkflows();
      expect(workflows).toHaveLength(1);
      expect(workflows[0].id).toBe('simpleWorkflow');
    });
  });

  describe('executeWorkflow', () => {
    it('should execute workflow with events', async () => {
      const events: BAMLEvent[] = [];

      const generator = provider.executeWorkflow('simpleWorkflow', { input: 'test' });

      for await (const event of generator) {
        events.push(event);
      }

      expect(events[0].type).toBe('execution.started');
      expect(events[events.length - 1].type).toBe('execution.completed');
    });

    it('should support cancellation', async () => {
      const generator = provider.executeWorkflow('simpleWorkflow', { input: 'test' });

      // Start execution
      const firstEvent = await generator.next();
      expect(firstEvent.value.type).toBe('execution.started');

      // Cancel
      const executionId = firstEvent.value.executionId;
      await provider.cancelExecution(executionId);

      // Should be cancelled
      const events: BAMLEvent[] = [];
      for await (const event of generator) {
        events.push(event);
      }

      expect(events.some(e => e.type === 'execution.cancelled')).toBe(true);
    });
  });

  describe('runTest', () => {
    it('should run test and emit events', async () => {
      const events: TestExecutionEvent[] = [];

      const generator = provider.runTest('fetchData', 'test_success');

      for await (const event of generator) {
        events.push(event);
      }

      expect(events[0].type).toBe('test.started');
      expect(events[events.length - 1].type).toBe('test.completed');
    });
  });
});

/**
 * VSCode Provider Tests
 */
describe('VSCodeDataProvider', () => {
  let store: Store;
  let provider: DataProvider;

  beforeEach(() => {
    store = createStore();
    provider = createVSCodeProvider(store);
  });

  afterEach(async () => {
    await provider.dispose();
  });

  describe('getWorkflows', () => {
    it('should get workflows from WASM runtime', async () => {
      // Mock WASM runtime
      const mockRuntime = {
        getFunctions: () => [
          {
            name: 'testWorkflow',
            type: 'workflow',
            filePath: 'test.baml',
          },
        ],
      };

      store.set(runtimeAtom, mockRuntime);

      const workflows = await provider.getWorkflows();
      expect(workflows).toHaveLength(1);
      expect(workflows[0].id).toBe('testWorkflow');
    });

    it('should handle missing runtime gracefully', async () => {
      store.set(runtimeAtom, null);

      const workflows = await provider.getWorkflows();
      expect(workflows).toEqual([]);
    });
  });

  describe('watchFiles', () => {
    it('should subscribe to file changes', async () => {
      const callback = jest.fn();
      const unsubscribe = provider.watchFiles(callback);

      // Update files atom
      store.set(filesAtom, { 'test.baml': 'content' });

      expect(callback).toHaveBeenCalledWith({ 'test.baml': 'content' });

      unsubscribe();
    });
  });
});
```

### 5.2 Provider Integration Tests

```typescript
/**
 * Provider Integration with SDK
 */
describe('Provider Integration', () => {
  it('should work with SDK', async () => {
    const store = createStore();
    const provider = createMockProvider({ speedMultiplier: 0.1 });
    const sdk = createBAMLSDK(provider, store);

    await sdk.initialize();

    const workflows = sdk.workflows.getAll();
    expect(workflows).toHaveLength(1);
  });

  it('should switch providers without breaking SDK', async () => {
    const store = createStore();

    // Start with mock
    let provider: DataProvider = createMockProvider({ speedMultiplier: 0.1 });
    let sdk = createBAMLSDK(provider, store);
    await sdk.initialize();

    let workflows = sdk.workflows.getAll();
    expect(workflows).toHaveLength(1);

    // Switch to VSCode provider
    provider = createVSCodeProvider(store);
    sdk = createBAMLSDK(provider, store);
    await sdk.initialize();

    // Should still work
    workflows = sdk.workflows.getAll();
    expect(workflows).toBeDefined();
  });
});
```

---

## Part 6: Migration Guide

### 6.1 Migration Steps

**Step 1: Create Provider Directory Structure**

```bash
packages/playground-common/src/sdk/providers/
├── base.ts              # DataProvider interface
├── mock-provider.ts     # MockDataProvider implementation
├── vscode-provider.ts   # VSCodeDataProvider implementation
├── provider-factory.ts  # Provider selection logic
└── __tests__/
    ├── mock-provider.test.ts
    └── vscode-provider.test.ts
```

**Step 2: Extract Interface**

Copy the DataProvider interface from this document into `base.ts`.

**Step 3: Implement MockProvider**

Copy `MockDataProvider` from baml-graph `mock.ts` and adapt to the interface.

**Step 4: Implement VSCodeProvider**

Create `VSCodeDataProvider` that wraps existing WASM runtime.

**Step 5: Update SDK**

Update SDK constructor to accept `DataProvider` instead of config.

**Step 6: Update Provider Component**

Update `BAMLSDKProvider` to create and inject provider.

**Step 7: Test**

Run all provider tests to ensure implementations are correct.

### 6.2 Backward Compatibility

The provider abstraction is **fully backward compatible**:

- Existing EventListener continues to work
- Existing atoms unchanged
- WASM runtime integration unchanged
- Only SDK internals change

### 6.3 Testing Checklist

- [ ] MockProvider returns realistic data
- [ ] MockProvider execution simulation works
- [ ] VSCodeProvider wraps WASM correctly
- [ ] VSCodeProvider handles missing runtime
- [ ] Provider factory selects correct mode
- [ ] SDK works with both providers
- [ ] File watching works
- [ ] Test execution works
- [ ] Navigation works
- [ ] Settings work

---

## Part 7: Implementation Checklist

### 7.1 Core Provider Implementation

- [ ] Create `sdk/providers/` directory
- [ ] **base.ts**: Define `DataProvider` interface (~150 lines)
  - [ ] Workflow methods (2)
  - [ ] File system methods (3)
  - [ ] Execution methods (3)
  - [ ] Test methods (4)
  - [ ] Graph methods (2)
  - [ ] Cache methods (3)
  - [ ] Navigation methods (2)
  - [ ] Settings methods (2)
  - [ ] Runtime methods (3)
  - [ ] Lifecycle methods (2)
- [ ] **mock-provider.ts**: Implement `MockDataProvider` (~800 lines)
  - [ ] Copy from baml-graph mock.ts
  - [ ] Adapt to DataProvider interface
  - [ ] Add all required methods
  - [ ] Add mock data generation
  - [ ] Add execution simulation
- [ ] **vscode-provider.ts**: Implement `VSCodeDataProvider` (~400 lines)
  - [ ] Wrap WASM runtime
  - [ ] Integrate with VSCode API
  - [ ] Implement all interface methods
  - [ ] Handle missing runtime gracefully
- [ ] **provider-factory.ts**: Provider selection logic (~100 lines)
  - [ ] Mode detection
  - [ ] Provider creation
  - [ ] Auto-detection

### 7.2 SDK Integration

- [ ] Update SDK constructor to accept `DataProvider`
- [ ] Update SDK methods to delegate to provider
- [ ] Update SDK initialization to use provider
- [ ] Remove direct WASM access from SDK
- [ ] Update BAMLSDKProvider to inject provider

### 7.3 Testing

- [ ] **mock-provider.test.ts**: Mock provider tests
  - [ ] Test workflow methods
  - [ ] Test execution simulation
  - [ ] Test cancellation
  - [ ] Test error scenarios
- [ ] **vscode-provider.test.ts**: VSCode provider tests
  - [ ] Test WASM integration
  - [ ] Test file watching
  - [ ] Test missing runtime handling
- [ ] **provider-factory.test.ts**: Factory tests
  - [ ] Test mode detection
  - [ ] Test provider creation
- [ ] Integration tests with SDK

### 7.4 Documentation

- [ ] API documentation for DataProvider interface
- [ ] Provider implementation guide
- [ ] Migration guide
- [ ] Extension guide for future providers

---

## Part 8: Validation Criteria

### 8.1 Functional Requirements

- [ ] MockProvider returns realistic data
- [ ] MockProvider execution simulation works smoothly
- [ ] MockProvider supports cancellation
- [ ] VSCodeProvider wraps WASM runtime correctly
- [ ] VSCodeProvider handles missing runtime gracefully
- [ ] VSCodeProvider integrates with VSCode API
- [ ] Provider factory detects mode correctly
- [ ] Provider factory creates correct provider
- [ ] All DataProvider methods implemented
- [ ] Provider can be swapped without breaking SDK

### 8.2 Integration

- [ ] SDK works with MockProvider
- [ ] SDK works with VSCodeProvider
- [ ] Existing EventListener continues to work
- [ ] Existing atoms unchanged
- [ ] WASM runtime integration unchanged
- [ ] No circular dependencies
- [ ] Type checking passes

### 8.3 Testing

- [ ] All provider unit tests pass
- [ ] All integration tests pass
- [ ] Manual testing in browser (mock mode)
- [ ] Manual testing in VSCode (vscode mode)
- [ ] Test coverage > 80%

### 8.4 Performance

- [ ] Mock execution performs smoothly
- [ ] No memory leaks in providers
- [ ] File watching doesn't cause excessive updates
- [ ] Provider switching is fast

---

## Part 9: Risk Mitigation

### 9.1 High-Risk Areas

**1. WASM Runtime Coupling**

Risk: VSCodeProvider might be too tightly coupled to WASM runtime

Mitigation:
- Use defensive programming (check for null)
- Graceful degradation when runtime unavailable
- Comprehensive error handling

**2. Provider Switching**

Risk: Switching providers might break SDK state

Mitigation:
- Providers are stateless
- All state lives in atoms
- Provider just accesses data sources

**3. Mock Data Realism**

Risk: Mock data might not match real WASM behavior

Mitigation:
- Copy mock.ts from baml-graph (battle-tested)
- Regular updates to match WASM changes
- Integration tests with real WASM

### 9.2 Rollback Plan

If provider abstraction causes issues:

1. Keep SDK working with inline mock data
2. Revert provider abstraction
3. Continue with Phase 2 SDK (direct WASM access)

Rollback is safe because:
- Phase 1 atoms independent
- Phase 2 SDK can work without providers
- EventListener unchanged

---

## Part 10: Success Metrics

- [ ] DataProvider interface defined (150 lines)
- [ ] MockProvider implemented (800 lines)
- [ ] VSCodeProvider implemented (400 lines)
- [ ] Provider factory implemented (100 lines)
- [ ] SDK integrated with providers
- [ ] All 26 provider methods implemented
- [ ] All tests pass (>80% coverage)
- [ ] No regressions in existing features
- [ ] Provider switching works
- [ ] Documentation complete

---

## Appendix A: Provider Method Count

### By Category

1. **Workflow Data**: 2 methods
2. **File System**: 3 methods
3. **Execution**: 3 methods
4. **Test Execution**: 4 methods
5. **Graph & Structure**: 2 methods
6. **Cache Management**: 3 methods
7. **Navigation**: 2 methods
8. **Settings**: 2 methods
9. **Runtime**: 3 methods
10. **Lifecycle**: 2 methods

**Total: 26 methods**

---

## Appendix B: File Size Estimates

| File | Lines of Code | Complexity |
|------|--------------|------------|
| base.ts (interface) | ~150 | Low (types only) |
| mock-provider.ts | ~800 | High (complex simulation) |
| vscode-provider.ts | ~400 | Medium (WASM wrapping) |
| provider-factory.ts | ~100 | Low (simple factory) |
| **Total** | ~1450 | **Medium-High** |

---

## Appendix C: Dependencies

### No New Dependencies

Provider abstraction uses only existing dependencies:
- Jotai (already used)
- TypeScript (already used)
- Existing WASM types

### Internal Dependencies

- Phase 1 unified atoms
- Phase 2 SDK
- Existing WASM runtime
- VSCode API wrapper

---

**Last Updated**: 2025-11-04
**Status**: Phase 3 implementation document complete - Ready for implementation
**Next Phase**: Phase 4 (Execution Engine)
