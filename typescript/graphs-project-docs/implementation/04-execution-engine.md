# Phase 4: Unified Execution Engine

**Timeline:** Week 3-4
**Dependencies:** Phase 3 (Data Providers)
**Risk Level:** High

## Purpose

Create a unified execution system that handles both test execution (single function) and workflow execution (multiple nodes). This replaces the separate `runTests` and `executeWorkflow` implementations with a single, flexible execution engine that supports:

1. **Isolated Function Execution** - Run a single function with test inputs (current `runTest` behavior)
2. **Function-in-Workflow** - Run a single node within its workflow context
3. **Full Workflow Execution** - Execute entire workflow graph with dependency resolution
4. **Partial Workflow** - Start execution from a specific node

## What This Document Will Cover

- Unified `execute()` API design and semantics
- Three execution modes: isolated function, function-in-workflow, full workflow
- ExecutionEngine class implementation
- Graph traversal algorithms (topological sort, dependency resolution)
- Node execution with state management
- Input resolution (from test cases, previous nodes, user input)
- Error handling and recovery strategies
- Abort/cancellation support
- Watch notification collection
- Cache integration
- Progress tracking and event emission
- Backward compatibility wrappers (`runTest()`, `runWorkflow()`)

## Key Decisions

**Architecture**:
- Single `sdk.execute(options)` method with typed options
- ExecutionEngine as internal implementation detail
- Graph traversal happens in ExecutionEngine
- Node state updates happen incrementally (not batched)

**Execution**:
- Support for partial workflow execution (start from node)
- Abort via AbortController
- Events emitted at each execution milestone
- Cache lookup before node execution

**Backward Compatibility**:
- Keep `sdk.tests.run()` as wrapper around `execute()`
- Keep `sdk.executions.start()` as wrapper around `execute()`
- Old APIs delegate to new ExecutionEngine

## Source Files to Reference

### Current Implementation (Playground-Common)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/index.ts` (lines 222-598 - current execution implementation)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/providers/base.ts` (lines 1-109 - DataProvider interface)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/providers/mock-provider.ts` (lines 48-174 - mock execution)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/types.ts` (types and interfaces)

### From baml-graph (Reference)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/index.ts` (lines 120-179 - execution start)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/mock.ts` (lines 570-811 - graph traversal)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC_ANSWERS.md` (lines 413-832 - Question 7: Unifying runTests vs run workflow)
- `/Users/aaronvillalpando/Projects/baml/typescript/BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md` (lines 100-150 - SDK execution methods)

### Current State Notes
- SDK class already exists with basic workflow execution (via provider pattern)
- DataProvider interface is already defined with executeWorkflow method
- MockDataProvider has basic workflow simulation
- Need to refactor to use dedicated ExecutionEngine class
- Need to add function-isolated and function-in-workflow modes

---

## Part 1: Execution API Design

### 1.1 ExecutionOptions Type

```typescript
/**
 * Unified execution options
 * Supports three execution modes via discriminated union
 */
type ExecutionOptions =
  // Mode 1: Execute single function in isolation (test mode)
  | {
      mode: 'function-isolated';
      functionName: string;
      testName?: string; // Optional: use specific test case
      inputs?: Record<string, unknown>; // Optional: override test inputs
      cachePolicy?: CachePolicy;
    }
  // Mode 2: Execute single function within workflow context
  | {
      mode: 'function-in-workflow';
      workflowId: string;
      nodeId: string; // Which function to execute
      inputs?: Record<string, unknown>; // Optional: override inputs
      cachePolicy?: CachePolicy;
    }
  // Mode 3: Execute full workflow
  | {
      mode: 'workflow';
      workflowId: string;
      inputs: Record<string, unknown>; // Workflow entry inputs
      startFromNodeId?: string; // Optional: partial execution
      cachePolicy?: CachePolicy;
      clearCache?: boolean;
    };

/**
 * Cache policy for execution
 */
type CachePolicy = 'auto' | 'always-run' | 'always-cache';
```

### 1.2 ExecutionResult Type

```typescript
/**
 * Result of execution
 */
interface ExecutionResult {
  executionId: string;
  status: 'success' | 'error' | 'cancelled';
  duration: number;

  // For function-isolated and function-in-workflow
  outputs?: Record<string, unknown>;
  error?: Error;

  // For workflow
  nodeResults?: Map<string, NodeExecutionResult>;

  // Watch notifications (for all modes)
  watchNotifications?: WatchNotification[];

  // Cache statistics
  cacheStats?: {
    hits: number;
    misses: number;
  };
}

interface NodeExecutionResult {
  nodeId: string;
  status: 'success' | 'error' | 'skipped' | 'cached';
  inputs: Record<string, unknown>;
  outputs?: Record<string, unknown>;
  error?: Error;
  duration: number;
  cached?: boolean;
}
```

### 1.3 SDK API Surface

```typescript
// New unified API
class BAMLSDK {
  /**
   * Unified execution method
   * Returns async iterator for real-time updates
   */
  async *execute(options: ExecutionOptions): AsyncGenerator<ExecutionEvent> {
    // Implementation delegates to ExecutionEngine
  }

  /**
   * Backward compatibility: Run test
   */
  async runTest(
    functionName: string,
    testName: string,
    options?: { inputs?: Record<string, unknown> }
  ): Promise<ExecutionResult> {
    // Wrapper around execute()
    const events: ExecutionEvent[] = [];
    for await (const event of this.execute({
      mode: 'function-isolated',
      functionName,
      testName,
      inputs: options?.inputs,
    })) {
      events.push(event);
    }
    return this.buildResultFromEvents(events);
  }

  /**
   * Backward compatibility: Start workflow execution
   */
  async executeWorkflow(
    workflowId: string,
    inputs: Record<string, unknown>,
    options?: { startFromNodeId?: string; clearCache?: boolean }
  ): Promise<string> {
    // Wrapper around execute()
    const executionId = `exec_${Date.now()}`;

    // Start async execution
    (async () => {
      for await (const event of this.execute({
        mode: 'workflow',
        workflowId,
        inputs,
        startFromNodeId: options?.startFromNodeId,
        clearCache: options?.clearCache,
      })) {
        this.emitEvent(event);
      }
    })();

    return executionId;
  }
}
```

---

## Part 2: ExecutionEngine Class

### 2.1 Class Structure

**Location:** `packages/playground-common/src/sdk/execution/engine.ts`

```typescript
/**
 * ExecutionEngine - Internal execution orchestrator
 *
 * Responsibilities:
 * - Graph traversal for workflows
 * - Node execution with state management
 * - Input resolution from various sources
 * - Cache integration
 * - Event emission
 * - Abort handling
 */
export class ExecutionEngine {
  constructor(
    private provider: DataProvider,
    private store: Store
  ) {}

  /**
   * Main execution entry point
   */
  async *execute(options: ExecutionOptions): AsyncGenerator<ExecutionEvent> {
    switch (options.mode) {
      case 'function-isolated':
        yield* this.executeFunctionIsolated(options);
        break;
      case 'function-in-workflow':
        yield* this.executeFunctionInWorkflow(options);
        break;
      case 'workflow':
        yield* this.executeWorkflow(options);
        break;
    }
  }

  /**
   * Execute single function in isolation (test mode)
   */
  private async *executeFunctionIsolated(
    options: Extract<ExecutionOptions, { mode: 'function-isolated' }>
  ): AsyncGenerator<ExecutionEvent> {
    const { functionName, testName, inputs, cachePolicy } = options;

    // 1. Resolve inputs
    const resolvedInputs = await this.resolveInputs({
      functionName,
      testName,
      manualInputs: inputs,
    });

    // 2. Check cache
    if (cachePolicy !== 'always-run') {
      const cached = await this.checkCache(functionName, resolvedInputs);
      if (cached) {
        yield {
          type: 'node.cached',
          nodeId: functionName,
          fromExecutionId: cached.executionId,
        };
        yield {
          type: 'execution.completed',
          outputs: cached.outputs,
          duration: 0,
        };
        return;
      }
    }

    // 3. Execute via provider
    const startTime = Date.now();
    yield { type: 'node.started', nodeId: functionName, inputs: resolvedInputs };

    try {
      const result = await this.provider.runTest(functionName, testName || 'default');

      // Collect events from generator
      for await (const event of result) {
        yield event;
      }

      yield {
        type: 'execution.completed',
        duration: Date.now() - startTime,
      };
    } catch (error) {
      yield {
        type: 'execution.error',
        error: error as Error,
      };
    }
  }

  /**
   * Execute workflow with full graph traversal
   */
  private async *executeWorkflow(
    options: Extract<ExecutionOptions, { mode: 'workflow' }>
  ): AsyncGenerator<ExecutionEvent> {
    const { workflowId, inputs, startFromNodeId, clearCache } = options;

    // 1. Get workflow definition
    const workflow = await this.provider.getWorkflow(workflowId);
    if (!workflow) {
      throw new Error(`Workflow ${workflowId} not found`);
    }

    // 2. Create execution snapshot
    const executionId = `exec_${Date.now()}`;
    const execution: ExecutionSnapshot = {
      id: executionId,
      workflowId,
      timestamp: Date.now(),
      graphSnapshot: {
        nodes: workflow.nodes,
        edges: workflow.edges,
        codeHash: workflow.codeHash,
      },
      status: 'running',
      nodeExecutions: new Map(),
      trigger: 'manual',
      branchPath: [],
      inputs,
    };

    // 3. Clear cache if requested
    if (clearCache) {
      await this.provider.clearCache('workflow', workflowId);
    }

    // 4. Initialize node states
    for (const node of workflow.nodes) {
      this.store.set(nodeStateAtomFamily(node.id), 'not-started');
    }

    yield { type: 'execution.started', executionId, workflowId };

    // 5. Perform graph traversal
    const visited = new Set<string>();
    const context: Record<string, unknown> = { ...inputs };
    let currentNodes = [startFromNodeId || workflow.entryPoint];

    while (currentNodes.length > 0) {
      const nextNodes: string[] = [];

      // Execute all nodes in current level
      for (const nodeId of currentNodes) {
        const node = workflow.nodes.find((n) => n.id === nodeId);
        if (!node || visited.has(nodeId)) continue;

        visited.add(nodeId);

        // Execute node
        yield* this.executeNode({
          node,
          executionId,
          context,
          workflow,
          execution,
        });

        // Determine next nodes
        const outgoingEdges = workflow.edges.filter((e) => e.source === nodeId);
        nextNodes.push(...outgoingEdges.map((e) => e.target));
      }

      currentNodes = nextNodes;
    }

    // 6. Mark execution complete
    execution.status = 'completed';
    execution.duration = Date.now() - execution.timestamp;

    yield {
      type: 'execution.completed',
      executionId,
      duration: execution.duration,
      outputs: execution.outputs ?? {},
    };
  }

  /**
   * Execute a single node
   */
  private async *executeNode(params: {
    node: GraphNode;
    executionId: string;
    context: Record<string, unknown>;
    workflow: WorkflowDefinition;
    execution: ExecutionSnapshot;
  }): AsyncGenerator<ExecutionEvent> {
    const { node, executionId, context, workflow, execution } = params;

    // 1. Prepare inputs
    const nodeInputs = { ...context };

    // 2. Update state to running
    this.store.set(nodeStateAtomFamily(node.id), 'running');

    yield {
      type: 'node.started',
      executionId,
      nodeId: node.id,
      inputs: nodeInputs,
    };

    // 3. Check cache
    const inputsHash = this.hashInputs(nodeInputs);
    const cachedEntry = this.store.get(cacheAtom).get(
      getCacheKey(node.id, inputsHash)
    );

    if (cachedEntry && cachedEntry.codeHash === node.codeHash) {
      this.store.set(nodeStateAtomFamily(node.id), 'cached');

      yield {
        type: 'node.cached',
        executionId,
        nodeId: node.id,
        fromExecutionId: cachedEntry.executionId,
      };

      // Use cached outputs
      Object.assign(context, cachedEntry.outputs);
      return;
    }

    // 4. Execute via provider (delegate to appropriate method based on node type)
    const startTime = Date.now();

    try {
      // For now, simulate execution
      // TODO: Integrate with actual WASM execution
      const outputs = await this.executeNodeViaProvider(node, nodeInputs);
      const duration = Date.now() - startTime;

      // 5. Update state to success
      this.store.set(nodeStateAtomFamily(node.id), 'success');

      yield {
        type: 'node.completed',
        executionId,
        nodeId: node.id,
        inputs: nodeInputs,
        outputs,
        duration,
      };

      // 6. Store in cache
      const cacheEntry: CacheEntry = {
        nodeId: node.id,
        codeHash: node.codeHash,
        inputs: nodeInputs,
        inputsHash,
        outputs,
        executionId,
        timestamp: Date.now(),
        duration,
      };

      const cache = this.store.get(cacheAtom);
      cache.set(getCacheKey(node.id, inputsHash), cacheEntry);
      this.store.set(cacheAtom, new Map(cache));

      // 7. Update context with outputs
      Object.assign(context, outputs);

      // 8. Store node execution in snapshot
      const nodeExecution: NodeExecution = {
        nodeId: node.id,
        executionId,
        state: 'success',
        inputs: nodeInputs,
        outputs,
        logs: [],
        startTime,
        endTime: Date.now(),
        duration,
      };

      execution.nodeExecutions.set(node.id, nodeExecution);
    } catch (error) {
      // Error handling
      this.store.set(nodeStateAtomFamily(node.id), 'error');

      yield {
        type: 'node.error',
        executionId,
        nodeId: node.id,
        error: error as Error,
      };

      // Store error in execution
      const nodeExecution: NodeExecution = {
        nodeId: node.id,
        executionId,
        state: 'error',
        inputs: nodeInputs,
        logs: [],
        startTime,
        endTime: Date.now(),
        duration: Date.now() - startTime,
        error: error as Error,
      };

      execution.nodeExecutions.set(node.id, nodeExecution);

      // Don't propagate error, just mark node as failed
    }
  }

  /**
   * Execute node via provider (delegates to appropriate method)
   */
  private async executeNodeViaProvider(
    node: GraphNode,
    inputs: Record<string, unknown>
  ): Promise<Record<string, unknown>> {
    // For mock mode: provider handles this
    // For VSCode mode: need to integrate with WASM runtime

    // TODO: Call appropriate provider method based on node type
    // For now, return mock outputs
    return {
      result: `Output from ${node.id}`,
      timestamp: Date.now(),
    };
  }

  /**
   * Resolve inputs from various sources
   */
  private async resolveInputs(params: {
    functionName: string;
    testName?: string;
    manualInputs?: Record<string, unknown>;
  }): Promise<Record<string, unknown>> {
    const { functionName, testName, manualInputs } = params;

    // Priority 1: Manual inputs
    if (manualInputs) {
      return manualInputs;
    }

    // Priority 2: Test case inputs
    if (testName) {
      const testCases = await this.provider.getTestCases(functionName);
      const testCase = testCases.find((tc) => tc.name === testName);
      if (testCase) {
        return testCase.inputs;
      }
    }

    // Priority 3: Empty inputs
    return {};
  }

  /**
   * Check cache for node execution
   */
  private async checkCache(
    nodeId: string,
    inputs: Record<string, unknown>
  ): Promise<CacheEntry | null> {
    const inputsHash = this.hashInputs(inputs);
    const cache = this.store.get(cacheAtom);
    return cache.get(getCacheKey(nodeId, inputsHash)) ?? null;
  }

  /**
   * Hash inputs for cache key
   */
  private hashInputs(inputs: Record<string, unknown>): string {
    return JSON.stringify(inputs); // Simple hash, can be improved
  }
}
```

---

## Part 3: Graph Traversal Algorithm

### 3.1 Topological Sort (for future use)

```typescript
/**
 * Topological sort for workflow nodes
 * Returns nodes in dependency order
 */
function topologicalSort(
  nodes: GraphNode[],
  edges: GraphEdge[]
): GraphNode[] {
  const inDegree = new Map<string, number>();
  const adjacency = new Map<string, string[]>();

  // Initialize
  for (const node of nodes) {
    inDegree.set(node.id, 0);
    adjacency.set(node.id, []);
  }

  // Build adjacency list and in-degree
  for (const edge of edges) {
    adjacency.get(edge.source)?.push(edge.target);
    inDegree.set(edge.target, (inDegree.get(edge.target) ?? 0) + 1);
  }

  // Kahn's algorithm
  const queue: string[] = [];
  const sorted: GraphNode[] = [];

  // Start with nodes that have no dependencies
  for (const [nodeId, degree] of inDegree.entries()) {
    if (degree === 0) {
      queue.push(nodeId);
    }
  }

  while (queue.length > 0) {
    const nodeId = queue.shift()!;
    const node = nodes.find((n) => n.id === nodeId)!;
    sorted.push(node);

    // Reduce in-degree of neighbors
    for (const neighbor of adjacency.get(nodeId) ?? []) {
      const newDegree = (inDegree.get(neighbor) ?? 0) - 1;
      inDegree.set(neighbor, newDegree);

      if (newDegree === 0) {
        queue.push(neighbor);
      }
    }
  }

  // Check for cycles
  if (sorted.length !== nodes.length) {
    throw new Error('Workflow has cycles');
  }

  return sorted;
}
```

### 3.2 Breadth-First Traversal (current implementation)

```typescript
/**
 * BFS traversal - executes nodes level by level
 * Used in current implementation
 */
async function* traverseWorkflow(
  workflow: WorkflowDefinition,
  startNodeId: string,
  context: Record<string, unknown>
): AsyncGenerator<string> {
  const visited = new Set<string>();
  let currentLevel = [startNodeId];

  while (currentLevel.length > 0) {
    const nextLevel: string[] = [];

    // Process all nodes in current level
    for (const nodeId of currentLevel) {
      if (visited.has(nodeId)) continue;
      visited.add(nodeId);

      yield nodeId; // Execute this node

      // Find outgoing edges
      const outgoing = workflow.edges.filter((e) => e.source === nodeId);
      for (const edge of outgoing) {
        // Check if conditional edge
        if (edge.condition) {
          const conditionMet = evaluateCondition(edge.condition, context);
          if (conditionMet) {
            nextLevel.push(edge.target);
          }
        } else {
          nextLevel.push(edge.target);
        }
      }
    }

    currentLevel = nextLevel;
  }
}
```

---

## Part 4: Input Resolution Logic

### 4.1 Input Resolution Strategy

```typescript
/**
 * Input resolution priority:
 * 1. Manual inputs (user-provided)
 * 2. Test case inputs (from BAML test definitions)
 * 3. Previous node outputs (workflow context)
 * 4. Default values (from function signature)
 * 5. Empty object (fallback)
 */
class InputResolver {
  constructor(private provider: DataProvider) {}

  async resolve(params: InputResolutionParams): Promise<Record<string, unknown>> {
    // Priority 1: Manual inputs
    if (params.manualInputs) {
      return params.manualInputs;
    }

    // Priority 2: Test case inputs
    if (params.testName) {
      const inputs = await this.resolveFromTestCase(
        params.functionName,
        params.testName
      );
      if (inputs) return inputs;
    }

    // Priority 3: Workflow context (previous nodes)
    if (params.context) {
      const inputs = this.resolveFromContext(params.functionSignature, params.context);
      if (inputs) return inputs;
    }

    // Priority 4: Default values
    const defaults = this.resolveDefaults(params.functionSignature);
    if (defaults) return defaults;

    // Priority 5: Empty
    return {};
  }

  private async resolveFromTestCase(
    functionName: string,
    testName: string
  ): Promise<Record<string, unknown> | null> {
    const testCases = await this.provider.getTestCases(functionName);
    const testCase = testCases.find((tc) => tc.name === testName);
    return testCase?.inputs ?? null;
  }

  private resolveFromContext(
    signature: FunctionSignature,
    context: Record<string, unknown>
  ): Record<string, unknown> {
    const inputs: Record<string, unknown> = {};

    // Map context values to function parameters
    for (const param of signature.parameters) {
      if (context[param.name] !== undefined) {
        inputs[param.name] = context[param.name];
      }
    }

    return Object.keys(inputs).length > 0 ? inputs : {};
  }

  private resolveDefaults(
    signature: FunctionSignature
  ): Record<string, unknown> | null {
    const defaults: Record<string, unknown> = {};

    for (const param of signature.parameters) {
      if (param.defaultValue !== undefined) {
        defaults[param.name] = param.defaultValue;
      }
    }

    return Object.keys(defaults).length > 0 ? defaults : null;
  }
}
```

---

## Part 5: Integration with SDK

### 5.1 Update SDK Constructor

```typescript
// File: src/sdk/index.ts

import { ExecutionEngine } from './execution/engine';

export class BAMLSDK {
  private executionEngine: ExecutionEngine;

  constructor(config: BAMLSDKConfig, store: Store) {
    // ... existing initialization

    // Create execution engine
    this.executionEngine = new ExecutionEngine(this.provider, store);
  }

  // New unified execute API
  async *execute(options: ExecutionOptions): AsyncGenerator<ExecutionEvent> {
    yield* this.executionEngine.execute(options);
  }

  // Backward compatibility wrappers
  executions = {
    start: async (
      workflowId: string,
      inputs: Record<string, unknown>,
      options?: { startFromNodeId?: string; clearCache?: boolean }
    ): Promise<string> => {
      const executionId = `exec_${Date.now()}`;

      // Start execution asynchronously
      (async () => {
        for await (const event of this.execute({
          mode: 'workflow',
          workflowId,
          inputs,
          startFromNodeId: options?.startFromNodeId,
          clearCache: options?.clearCache,
        })) {
          this.emitEvent(event);
        }
      })();

      return executionId;
    },

    // ... other methods
  };

  tests = {
    run: async (
      functionName: string,
      testName: string,
      options?: { inputs?: Record<string, unknown> }
    ): Promise<ExecutionResult> => {
      const events: ExecutionEvent[] = [];

      for await (const event of this.execute({
        mode: 'function-isolated',
        functionName,
        testName,
        inputs: options?.inputs,
      })) {
        events.push(event);
      }

      return this.buildResultFromEvents(events);
    },

    // ... other methods
  };
}
```

---

## Part 6: Error Handling & Recovery

### 6.1 Error Handling Strategy

```typescript
/**
 * Error handling at different levels
 */
class ExecutionEngine {
  private async *executeNode(/* ... */): AsyncGenerator<ExecutionEvent> {
    try {
      // Execute node
      const outputs = await this.executeNodeViaProvider(node, inputs);

      yield {
        type: 'node.completed',
        executionId,
        nodeId: node.id,
        outputs,
      };
    } catch (error) {
      // Node-level error
      yield {
        type: 'node.error',
        executionId,
        nodeId: node.id,
        error: error as Error,
      };

      // Update state
      this.store.set(nodeStateAtomFamily(node.id), 'error');

      // Strategy 1: Continue execution (don't propagate)
      // Other nodes can still execute if they don't depend on this one

      // Strategy 2: Abort workflow (optional)
      if (this.shouldAbortOnError(node)) {
        throw error;
      }
    }
  }

  private shouldAbortOnError(node: GraphNode): boolean {
    // Critical nodes should abort workflow
    return node.metadata?.critical === true;
  }
}
```

### 6.2 Cancellation Support

```typescript
/**
 * Abort execution via AbortController
 */
class ExecutionEngine {
  private abortControllers = new Map<string, AbortController>();

  async *executeWorkflow(options: ExecutionOptions): AsyncGenerator<ExecutionEvent> {
    const executionId = `exec_${Date.now()}`;
    const controller = new AbortController();
    this.abortControllers.set(executionId, controller);

    try {
      // Check abort signal during traversal
      while (currentNodes.length > 0) {
        if (controller.signal.aborted) {
          yield {
            type: 'execution.cancelled',
            executionId,
            reason: 'User cancelled',
          };
          return;
        }

        // Execute nodes...
      }
    } finally {
      this.abortControllers.delete(executionId);
    }
  }

  /**
   * Cancel execution
   */
  cancel(executionId: string): void {
    const controller = this.abortControllers.get(executionId);
    if (controller) {
      controller.abort();
    }
  }
}
```

---

## Part 7: Watch Notification Collection

### 7.1 Watch Notification Handling

```typescript
/**
 * Collect watch notifications during execution
 * Source: test-runner.ts:136-170
 */
interface WatchNotification {
  key: string;
  value: string;
  block_name?: string;
}

class ExecutionEngine {
  private watchNotifications: WatchNotification[] = [];

  private async executeNodeViaProvider(
    node: GraphNode,
    inputs: Record<string, unknown>
  ): Promise<{ outputs: Record<string, unknown>; watchNotifications: WatchNotification[] }> {
    this.watchNotifications = [];

    // Execute with watch notification callback
    const result = await this.provider.runTest(node.functionName!, 'default', {
      onWatchNotification: (notification: WatchNotification) => {
        this.watchNotifications.push(this.enrichNotification(notification));
      },
    });

    return {
      outputs: result.outputs,
      watchNotifications: this.watchNotifications,
    };
  }

  private enrichNotification(notification: WatchNotification): WatchNotification {
    // Enrich with block name if JSON value contains it
    if (!notification.block_name) {
      try {
        const parsed = JSON.parse(notification.value) as {
          type?: string;
          label?: string;
        };
        if (parsed?.type === 'block' && typeof parsed.label === 'string') {
          notification.block_name = parsed.label;
        }
      } catch {
        // Ignore parse errors
      }
    }
    return notification;
  }
}
```

---

## Implementation Checklist

### Core Engine
- [ ] **Create execution engine file** - `src/sdk/execution/engine.ts`
- [ ] **Define ExecutionOptions type** - All three modes
- [ ] **Define ExecutionResult type** - With all metadata
- [ ] **Create ExecutionEngine class** - Main orchestrator
- [ ] **Implement execute() entry point** - Mode dispatch

### Execution Modes
- [ ] **Implement executeFunctionIsolated()** - Test mode
- [ ] **Implement executeFunctionInWorkflow()** - Single node in context
- [ ] **Implement executeWorkflow()** - Full graph traversal

### Graph Traversal
- [ ] **Implement BFS traversal** - Level-by-level execution
- [ ] **Implement topological sort** - Dependency-ordered execution (optional)
- [ ] **Add conditional edge support** - Evaluate conditions

### Node Execution
- [ ] **Implement executeNode()** - Single node execution
- [ ] **Add state management** - Update nodeStateAtomFamily
- [ ] **Add cache integration** - Check before execute, store after
- [ ] **Add event emission** - node.started, node.completed, etc.

### Input Resolution
- [ ] **Create InputResolver class** - Priority-based resolution
- [ ] **Implement resolveFromTestCase()** - Load test inputs
- [ ] **Implement resolveFromContext()** - Map from previous nodes
- [ ] **Implement resolveDefaults()** - Use parameter defaults

### Error Handling
- [ ] **Add node-level error handling** - Catch and emit node.error
- [ ] **Add workflow-level error handling** - Optional abort on critical errors
- [ ] **Add cancellation support** - AbortController integration

### Integration
- [ ] **Update SDK class** - Add execute() method
- [ ] **Add backward compatibility** - Wrap old APIs
- [ ] **Update provider interface** - Add execution support if needed

### Testing
- [ ] **Write unit tests** - ExecutionEngine methods
- [ ] **Write integration tests** - End-to-end execution flows
- [ ] **Test all three modes** - Isolated, in-workflow, full workflow
- [ ] **Test error scenarios** - Node errors, cancellation
- [ ] **Test cache behavior** - Hits, misses, invalidation

---

## Validation Criteria

### Functional Requirements
- [ ] Single function execution works (isolated mode)
- [ ] Single function execution works within workflow (in-workflow mode)
- [ ] Full workflow execution works with multiple nodes
- [ ] Partial workflow execution works (start from node)
- [ ] Node states update correctly during execution
- [ ] Execution can be cancelled via abort
- [ ] Cache hits/misses work correctly
- [ ] Watch notifications collected during execution
- [ ] Events emitted at correct times with correct data

### Backward Compatibility
- [ ] Old `sdk.tests.run()` API works
- [ ] Old `sdk.executions.start()` API works
- [ ] Existing components using old APIs continue to work
- [ ] No breaking changes to public API

### Performance
- [ ] Execution completes in reasonable time
- [ ] No memory leaks during long workflows
- [ ] Cache lookup is O(1)
- [ ] Graph traversal is efficient (linear in nodes + edges)

### Error Handling
- [ ] Node errors don't crash execution engine
- [ ] Cancellation works reliably
- [ ] Error messages are helpful
- [ ] Failed nodes marked with error state

---

## Risk Mitigation

### High-Risk Areas

**1. Graph Traversal Complexity**
- Risk: Bugs in traversal algorithm
- Mitigation: Comprehensive unit tests, start with simple BFS

**2. State Synchronization**
- Risk: Node states out of sync with execution
- Mitigation: Update states synchronously, no batching

**3. Backward Compatibility**
- Risk: Breaking existing test execution
- Mitigation: Keep old APIs as wrappers, test extensively

### Testing Strategy

1. **Unit Tests** - Test each method in isolation
2. **Integration Tests** - Test full execution flows
3. **Regression Tests** - Ensure old APIs still work
4. **Manual Testing** - Test in UI with real workflows

---

## Success Metrics

- [ ] ExecutionEngine class implemented (~500 lines)
- [ ] All three execution modes working
- [ ] Graph traversal algorithm implemented
- [ ] Input resolution working from all sources
- [ ] Cache integration complete
- [ ] Error handling robust
- [ ] Backward compatibility maintained
- [ ] Test coverage > 80%
- [ ] Zero regressions in existing tests

---

**Last Updated**: 2025-11-04
**Status**: Ready for implementation
**Estimated Effort**: 3-4 days for experienced developer
