/**
 * ExecutionEngine - Unified execution orchestrator
 *
 * Responsibilities:
 * - Graph traversal for workflows
 * - Node execution with state management
 * - Input resolution from various sources
 * - Cache integration
 * - Event emission
 * - Abort handling
 *
 * Supports three execution modes:
 * 1. function-isolated: Run single function with test inputs (test mode)
 * 2. function-in-workflow: Run single node within workflow context
 * 3. workflow: Execute entire workflow graph
 */

import { createStore } from 'jotai';
import type { DataProvider } from '../providers/base';
import type {
  ExecutionOptions,
  ExecutionEvent,
  ExecutionResult,
  NodeExecutionResult,
  InputResolutionParams,
  WatchNotification,
} from './types';
import type {
  WorkflowDefinition,
  ExecutionSnapshot,
  GraphNode,
  NodeExecution,
  CacheEntry,
  TestCaseInput,
} from '../types';
import {
  nodeStateAtomFamily,
  cacheAtom,
  getCacheKey,
  clearAllNodeStatesAtom,
  registerNodeAtom,
  workflowExecutionsAtomFamily,
} from '../../shared/atoms';

/**
 * ExecutionEngine - Internal execution orchestrator
 */
export class ExecutionEngine {
  private abortControllers = new Map<string, AbortController>();

  constructor(
    private provider: DataProvider,
    private store: ReturnType<typeof createStore>
  ) {}

  /**
   * Main execution entry point
   * Returns async generator for real-time updates
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
    const executionId = `exec_test_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

    console.log(`[ExecutionEngine] Starting isolated function execution: ${functionName}`, { testName, executionId });

    // 1. Resolve inputs
    const resolvedInputs = await this.resolveInputs({
      functionName,
      testName,
      manualInputs: inputs,
    });

    // 2. Check cache
    if (cachePolicy !== 'always-run') {
      const inputsHash = this.hashInputs(resolvedInputs);
      const cached = this.checkCache(functionName, inputsHash);
      if (cached) {
        console.log(`[ExecutionEngine] Cache hit for ${functionName}`);
        yield {
          type: 'node.cached',
          nodeId: functionName,
          fromExecutionId: cached.executionId,
          executionId,
        };
        return;
      }
    }

    // 3. Execute via provider
    const startTime = Date.now();
    yield {
      type: 'node.started',
      nodeId: functionName,
      inputs: resolvedInputs,
      executionId,
    };

    try {
      // Use provider's test execution
      const testNameResolved = testName || 'default';
      for await (const event of this.provider.runTest(functionName, testNameResolved)) {
        // Convert test events to BAMLEvents
        if (event.type === 'test.completed') {
          yield {
            type: 'node.completed',
            executionId,
            nodeId: functionName,
            outputs: { result: 'success' }, // TODO: Extract actual outputs from test result
            duration: event.duration,
          };
        } else if (event.type === 'test.error') {
          yield {
            type: 'node.error',
            executionId,
            nodeId: functionName,
            error: typeof event.error === 'string' ? new Error(event.error) : event.error,
          };
        }
        // Other test events can be logged or converted as needed
      }

      yield {
        type: 'execution.completed',
        executionId,
        duration: Date.now() - startTime,
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

  /**
   * Execute single function within workflow context
   * This allows running a specific node while having access to upstream outputs
   */
  private async *executeFunctionInWorkflow(
    options: Extract<ExecutionOptions, { mode: 'function-in-workflow' }>
  ): AsyncGenerator<ExecutionEvent> {
    const { workflowId, nodeId, inputs, cachePolicy } = options;
    const executionId = `exec_node_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

    console.log(`[ExecutionEngine] Starting function-in-workflow: ${nodeId} in ${workflowId}`);

    // Get workflow
    const workflow = await this.provider.getWorkflow(workflowId);
    if (!workflow) {
      throw new Error(`Workflow ${workflowId} not found`);
    }

    // Find node
    const node = workflow.nodes.find((n) => n.id === nodeId);
    if (!node) {
      throw new Error(`Node ${nodeId} not found in workflow ${workflowId}`);
    }

    // For now, execute as isolated function
    // TODO: In future, resolve inputs from upstream nodes in workflow context
    yield* this.executeFunctionIsolated({
      mode: 'function-isolated',
      functionName: node.functionName || nodeId,
      inputs,
      cachePolicy,
    });
  }

  /**
   * Execute full workflow with graph traversal
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
    const executionId = `exec_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
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
    this.store.set(clearAllNodeStatesAtom);
    for (const node of workflow.nodes) {
      this.store.set(registerNodeAtom, node.id);
      this.store.set(nodeStateAtomFamily(node.id), 'not-started');
    }

    // 5. Create abort controller
    const controller = new AbortController();
    this.abortControllers.set(executionId, controller);

    yield {
      type: 'execution.started',
      executionId,
      workflowId,
    };

    console.log(`[ExecutionEngine] Starting workflow execution: ${workflowId}`, { executionId });

    try {
      // 6. Perform graph traversal using BFS
      const visited = new Set<string>();
      const context: Record<string, unknown> = { ...inputs };
      let currentNodes = [startFromNodeId || workflow.entryPoint || workflow.nodes[0]!.id];

      while (currentNodes.length > 0) {
        // Check for abort
        if (controller.signal.aborted) {
          console.log(`[ExecutionEngine] Workflow execution aborted: ${executionId}`);
          yield {
            type: 'execution.cancelled',
            executionId,
            reason: 'User cancelled',
          };
          return;
        }

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
            controller,
          });

          // Determine next nodes
          const outgoingEdges = workflow.edges.filter((e) => e.source === nodeId);
          for (const edge of outgoingEdges) {
            // TODO: Handle conditional edges
            if (edge.condition) {
              // For now, always take the edge (no condition evaluation)
              nextNodes.push(edge.target);
            } else {
              nextNodes.push(edge.target);
            }
          }
        }

        currentNodes = nextNodes;
      }

      // 7. Mark execution complete
      execution.status = 'completed';
      execution.duration = Date.now() - execution.timestamp;

      // Update execution in store
      const workflowExecutionsAtom = workflowExecutionsAtomFamily(workflowId);
      const workflowExecutions = this.store.get(workflowExecutionsAtom);
      const index = workflowExecutions.findIndex((e: ExecutionSnapshot) => e.id === executionId);
      if (index !== -1) {
        const updatedExecutions = [...workflowExecutions];
        updatedExecutions[index] = { ...execution };
        this.store.set(workflowExecutionsAtom, updatedExecutions);
      }

      yield {
        type: 'execution.completed',
        executionId,
        duration: execution.duration,
        outputs: execution.outputs ?? {},
      };
    } catch (error) {
      execution.status = 'error';
      execution.error = error as Error;

      yield {
        type: 'execution.error',
        executionId,
        error: error as Error,
      };
    } finally {
      this.abortControllers.delete(executionId);
    }
  }

  /**
   * Execute a single node within a workflow
   */
  private async *executeNode(params: {
    node: GraphNode;
    executionId: string;
    context: Record<string, unknown>;
    workflow: WorkflowDefinition;
    execution: ExecutionSnapshot;
    controller: AbortController;
  }): AsyncGenerator<ExecutionEvent> {
    const { node, executionId, context, workflow, execution, controller } = params;

    console.log(`[ExecutionEngine] Executing node: ${node.id}`, { executionId });

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
    const cachedEntry = this.checkCache(node.id, inputsHash);

    if (cachedEntry && cachedEntry.codeHash === node.codeHash) {
      console.log(`[ExecutionEngine] Using cached result for node: ${node.id}`);
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

    // 4. Execute node
    const startTime = Date.now();

    try {
      // Delegate to provider for actual execution
      const outputs = await this.executeNodeViaProvider(node, nodeInputs, controller);
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
      console.error(`[ExecutionEngine] Node error: ${node.id}`, error);

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
      // Other nodes can still execute if they don't depend on this one
    }
  }

  /**
   * Execute node via provider (delegates to appropriate method)
   */
  private async executeNodeViaProvider(
    node: GraphNode,
    inputs: Record<string, unknown>,
    controller: AbortController
  ): Promise<Record<string, unknown>> {
    // For now, use provider's test execution
    // In future, this should call a more specific execution method based on node type

    const functionName = node.functionName || node.id;

    console.log(`[ExecutionEngine] Executing ${functionName} via provider`);

    // Get test cases for this function
    const testCases = await this.provider.getTestCases(functionName);
    const testName = testCases[0]?.name || 'default';

    // Run the test
    const outputs: Record<string, unknown> = {};
    for await (const event of this.provider.runTest(functionName, testName)) {
      if (controller.signal.aborted) {
        throw new Error('Execution aborted');
      }

      if (event.type === 'test.completed') {
        // Extract outputs from test result
        outputs.result = event.response;
        outputs.status = event.status;
      } else if (event.type === 'test.error') {
        throw typeof event.error === 'string' ? new Error(event.error) : event.error;
      }
    }

    return outputs;
  }

  /**
   * Resolve inputs from various sources
   * Priority: manual inputs > test case > context > empty
   */
  private async resolveInputs(params: InputResolutionParams): Promise<Record<string, unknown>> {
    const { functionName, testName, manualInputs, context } = params;

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

    // Priority 3: Workflow context (from previous nodes)
    if (context) {
      return context;
    }

    // Priority 4: Empty inputs
    return {};
  }

  /**
   * Check cache for node execution
   */
  private checkCache(nodeId: string, inputsHash: string): CacheEntry | null {
    const cache = this.store.get(cacheAtom);
    const key = getCacheKey(nodeId, inputsHash);
    return cache.get(key) ?? null;
  }

  /**
   * Hash inputs for cache key
   * Simple JSON stringify for now - can be improved with better hashing
   */
  private hashInputs(inputs: Record<string, unknown>): string {
    return JSON.stringify(inputs);
  }

  /**
   * Cancel execution
   */
  cancel(executionId: string): void {
    const controller = this.abortControllers.get(executionId);
    if (controller) {
      console.log(`[ExecutionEngine] Cancelling execution: ${executionId}`);
      controller.abort();
      this.abortControllers.delete(executionId);
    }
  }

  /**
   * Cleanup - cancel all active executions
   */
  dispose(): void {
    console.log('[ExecutionEngine] Disposing - cancelling all active executions');
    for (const controller of this.abortControllers.values()) {
      controller.abort();
    }
    this.abortControllers.clear();
  }
}
