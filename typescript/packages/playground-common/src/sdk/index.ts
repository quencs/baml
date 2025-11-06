/**
 * BAML SDK - Main Interface
 *
 * This SDK provides the API for workflow management, execution, and real-time updates.
 * It uses Jotai atoms for reactive state management.
 */

import { getDefaultStore, type createStore } from 'jotai';
import type {
  BAMLSDKConfig,
  WorkflowDefinition,
  ExecutionSnapshot,
  BAMLEvent,
  GraphNode,
  GraphEdge,
  NodeExecution,
  CacheEntry,
  TestCaseInput,
} from './types';
import type { TestExecutionEvent } from './providers/base';
import {
  workflowsAtom,
  activeWorkflowIdAtom,
  activeWorkflowAtom,
  workflowExecutionsAtomFamily,
  addEventAtom,
  cacheAtom,
  getCacheKey,
  viewModeAtom,
  nodeStateAtomFamily,
  registerNodeAtom,
  clearAllNodeStatesAtom,
} from './atoms';

export * from './types';
export * from './atoms';
export type { TestExecutionEvent } from './providers/base';

/**
 * Main BAML SDK class
 */
export class BAMLSDK {
  private config: BAMLSDKConfig;
  public store: ReturnType<typeof createStore>;
  private activeExecutions = new Map<string, AbortController>();
  public mockData?: BAMLSDKConfig['mockData'];

  constructor(config: BAMLSDKConfig, store?: ReturnType<typeof createStore>) {
    this.config = config;
    this.store = store ?? getDefaultStore();
    // Support both 'mockData' and 'provider' for backward compatibility
    this.mockData = config.mockData || config.provider;
  }

  async initialize() {
    if (this.config.mode === 'mock' && this.config.mockData) {
      // Load mock workflows
      const workflows = this.config.mockData.getWorkflows();
      this.store.set(workflowsAtom, workflows);

      // Emit discovery events
      for (const workflow of workflows) {
        this.emitEvent({ type: 'workflow.discovered', workflow });
      }

      // Set first workflow as active
      if (workflows.length > 0 && workflows[0]) {
        this.workflows.setActive(workflows[0].id);
      } else {
        console.error('No workflows found');
      }
    }
  }

  // ============================================================================
  // Workflow API
  // ============================================================================

  workflows = {
    /**
     * Get all available workflows
     */
    getAll: (): WorkflowDefinition[] => {
      return this.store.get(workflowsAtom);
    },

    /**
     * Get a specific workflow by ID
     */
    getById: (workflowId: string): WorkflowDefinition | null => {
      const workflows = this.store.get(workflowsAtom);
      return workflows.find((w) => w.id === workflowId) ?? null;
    },

    /**
     * Get the currently active workflow
     */
    getActive: (): WorkflowDefinition | null => {
      return this.store.get(activeWorkflowAtom);
    },

    /**
     * Set the active workflow
     */
    setActive: (workflowId: string): void => {
      const workflow = this.workflows.getById(workflowId);
      if (!workflow) {
        throw new Error(`Workflow ${workflowId} not found`);
      }

      this.store.set(activeWorkflowIdAtom, workflowId);
      this.store.set(viewModeAtom, { mode: 'editor' });
      this.emitEvent({ type: 'workflow.selected', workflowId });
    },
  };

  // ============================================================================
  // Execution API
  // ============================================================================

  executions = {
    /**
     * Start a new execution of a workflow
     */
    start: async (
      workflowId: string,
      inputs: Record<string, unknown>,
      options?: { clearCache?: boolean; startFromNodeId?: string }
    ): Promise<string> => {
      const workflow = this.workflows.getById(workflowId);
      if (!workflow) {
        throw new Error(`Workflow ${workflowId} not found`);
      }

      // Clear cache if requested
      if (options?.clearCache) {
        this.cache.clear({ workflowId });
      }

      // Clear all node states before starting new execution
      this.store.set(clearAllNodeStatesAtom);

      // Wait 200ms to give users visual feedback that a new execution is starting
      await new Promise(resolve => setTimeout(resolve, 200));

      // Generate execution ID
      const executionId = `exec_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

      // Create execution snapshot
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

      // Add to executions using atomFamily
      const workflowExecutionsAtom = workflowExecutionsAtomFamily(workflowId);
      const workflowExecutions = this.store.get(workflowExecutionsAtom);
      this.store.set(workflowExecutionsAtom, [execution, ...workflowExecutions]);

      // Emit start event
      this.emitEvent({
        type: 'execution.started',
        executionId,
        workflowId,
      });

      // Run execution simulation (if mock mode)
      if (this.config.mode === 'mock' && this.config.mockData) {
        this.runMockExecution(execution, workflow, inputs, options?.startFromNodeId);
      }

      return executionId;
    },

    /**
     * Get all executions for a workflow
     */
    getExecutions: (workflowId: string): ExecutionSnapshot[] => {
      const workflowExecutionsAtom = workflowExecutionsAtomFamily(workflowId);
      return this.store.get(workflowExecutionsAtom);
    },

    /**
     * Get a specific execution by ID
     * Note: This requires searching through all workflows
     */
    getExecution: (executionId: string): ExecutionSnapshot | null => {
      // Get all workflows to search through their executions
      const workflows = this.store.get(workflowsAtom);
      for (const workflow of workflows) {
        const workflowExecutionsAtom = workflowExecutionsAtomFamily(workflow.id);
        const executions = this.store.get(workflowExecutionsAtom);
        const found = executions.find((e) => e.id === executionId);
        if (found) return found;
      }
      return null;
    },

    /**
     * Cancel a running execution
     */
    cancel: (executionId: string): void => {
      const controller = this.activeExecutions.get(executionId);
      if (controller) {
        controller.abort();
        this.activeExecutions.delete(executionId);
        this.emitEvent({
          type: 'execution.cancelled',
          executionId,
          reason: 'User cancelled',
        });
      }
    },
  };

  // ============================================================================
  // Graph API
  // ============================================================================

  graph = {
    /**
     * Get graph structure for a workflow
     */
    getGraph: (workflowId: string): { nodes: GraphNode[]; edges: GraphEdge[] } | null => {
      const workflow = this.workflows.getById(workflowId);
      if (!workflow) return null;

      return {
        nodes: workflow.nodes,
        edges: workflow.edges,
      };
    },

    /**
     * Update node positions (for layout persistence)
     */
    updateNodePositions: (
      workflowId: string,
      positions: Map<string, { x: number; y: number }>
    ): void => {
      const workflows = this.store.get(workflowsAtom);
      const updatedWorkflows = workflows.map((w) => {
        if (w.id !== workflowId) return w;

        return {
          ...w,
          nodes: w.nodes.map((node) => {
            const pos = positions.get(node.id);
            if (!pos) return node;

            return { ...node, position: pos };
          }),
        };
      });

      this.store.set(workflowsAtom, updatedWorkflows);
      this.emitEvent({
        type: 'graph.layout.updated',
        workflowId,
        positions,
      });
    },
  };

  // ============================================================================
  // Cache API
  // ============================================================================

  cache = {
    /**
     * Get cached result for a node
     */
    get: (nodeId: string, inputsHash: string): CacheEntry | null => {
      const cache = this.store.get(cacheAtom);
      const key = getCacheKey(nodeId, inputsHash);
      return cache.get(key) ?? null;
    },

    /**
     * Set cache for a node
     */
    set: (entry: CacheEntry): void => {
      const cache = this.store.get(cacheAtom);
      const key = getCacheKey(entry.nodeId, entry.inputsHash);
      cache.set(key, entry);
      this.store.set(cacheAtom, new Map(cache));
    },

    /**
     * Clear cache
     */
    clear: (scope?: { workflowId?: string; nodeId?: string }): void => {
      if (!scope) {
        // Clear all cache
        this.store.set(cacheAtom, new Map());
        this.emitEvent({ type: 'cache.cleared', scope: 'all' });
      } else if (scope.workflowId) {
        // Clear cache for workflow (simplified - clear all for now)
        this.store.set(cacheAtom, new Map());
        this.emitEvent({
          type: 'cache.cleared',
          scope: 'workflow',
          workflowId: scope.workflowId,
        });
      }
    },
  };

  // ============================================================================
  // Test Cases API (Input Library - Phase 2)
  // ============================================================================

  testCases = {
    /**
     * Get test cases for a specific node
     */
    get: (workflowId: string, nodeId: string): TestCaseInput[] => {
      if (this.config.mode === 'mock' && this.config.mockData) {
        return this.config.mockData.getTestCases(workflowId, nodeId);
      }
      return [];
    },
  };

  // ============================================================================
  // Files API
  // ============================================================================

  files = {
    /**
     * Update BAML files (triggers recompilation)
     */
    update: (files: Record<string, string>): void => {
      // TODO: Implement file update logic
      console.debug('[SDK] Files updated:', Object.keys(files));
    },
  };

  // ============================================================================
  // Navigation API
  // ============================================================================

  navigation = {
    /**
     * Update cursor position from IDE
     */
    updateCursor: (content: any): void => {
      // TODO: Implement cursor update logic
      console.debug('[SDK] Cursor updated:', content);
    },

    /**
     * Update cursor position from range
     */
    updateCursorFromRange: (params: {
      fileName: string;
      start: { line: number; character: number };
      end: { line: number; character: number };
    }): void => {
      // TODO: Implement cursor range update logic
      console.debug('[SDK] Cursor updated from range:', params);
    },

    /**
     * Select a function (navigate to it in the UI)
     */
    selectFunction: (functionName: string): void => {
      // TODO: Implement function selection logic
      console.debug('[SDK] Function selected:', functionName);
    },
  };

  // ============================================================================
  // Tests API
  // ============================================================================

  tests = {
    /**
     * Run a test case
     */
    run: async (functionName: string, testCaseName: string): Promise<{
      executionId: string;
      status: 'success' | 'error';
      duration: number;
      outputs?: Record<string, any>;
      error?: Error;
    }> => {
      // TODO: Implement test execution logic
      console.debug('[SDK] Running test:', { functionName, testCaseName });

      // For now, return a mock result
      return {
        executionId: `test_${Date.now()}`,
        status: 'success',
        duration: 100,
      };
    },

    /**
     * Run all tests for a function
     */
    runAll: (
      tests: Array<{ functionName: string; testName: string }>,
      options?: { parallel?: boolean; abortSignal?: AbortSignal }
    ): AsyncGenerator<TestExecutionEvent> => {
      const self = this;
      async function* gen(): AsyncGenerator<TestExecutionEvent> {
        if (self.config.mode === 'mock' && self.config.mockData && 'runTests' in (self.config.mockData as any)) {
          for await (const event of (self.config.mockData as any).runTests(tests, options)) {
            yield event as TestExecutionEvent;
          }
          return;
        }
        console.debug('[SDK] runAll fallback: no provider available');
      }
      return gen();
    },
  };

  // ============================================================================
  // Unified Execution API
  // ============================================================================

  /**
   * Unified execute API that yields events
   * This is the modern API for executing workflows and tests
   */
  async *execute(params: {
    mode: 'workflow' | 'test';
    workflowId: string;
    inputs: Record<string, unknown>;
    testCaseName?: string;
    options?: { clearCache?: boolean; startFromNodeId?: string };
  }): AsyncGenerator<BAMLEvent> {
    const { workflowId, inputs, options } = params;

    // Start execution
    const executionId = await this.executions.start(workflowId, inputs, options);

    // Yield events from mock data provider
    if (this.config.mode === 'mock' && this.config.mockData) {
      for await (const event of this.config.mockData.simulateExecution(
        workflowId,
        inputs,
        options?.startFromNodeId
      )) {
        yield event;
      }
    }
  }

  // ============================================================================
  // Event System
  // ============================================================================

  private emitEvent(event: BAMLEvent): void {
    this.store.set(addEventAtom, event);
  }

  /**
   * Subscribe to events
   * Returns unsubscribe function
   */
  onEvent(_callback: (event: BAMLEvent) => void): () => void {
    // Use Jotai's store subscription
    return this.store.sub(addEventAtom, () => {
      // Not ideal but works for now
      // In a real implementation, we'd use a proper event emitter
    });
  }

  // ============================================================================
  // Mock Execution Simulation
  // ============================================================================

  private async runMockExecution(
    execution: ExecutionSnapshot,
    workflow: WorkflowDefinition,
    inputs: Record<string, unknown>,
    startFromNodeId?: string
  ): Promise<void> {
    if (!this.config.mockData) return;

    const controller = new AbortController();
    this.activeExecutions.set(execution.id, controller);

    console.log(`🎬 Starting mock execution for workflow: ${workflow.id}`, { executionId: execution.id });

    try {
      // Simulate execution through the generator
      for await (const event of this.config.mockData.simulateExecution(
        workflow.id,
        inputs,
        startFromNodeId
      )) {
        if (controller.signal.aborted) {
          console.log(`⏹️ Execution aborted: ${execution.id}`);
          break;
        }

        // Update node states based on events
        if (event.type === 'node.started') {
          console.log(`▶️ Node started: ${event.nodeId}`);
          // Register node and update its state using atomFamily
          this.store.set(registerNodeAtom, event.nodeId);
          this.store.set(nodeStateAtomFamily(event.nodeId), 'running');

          // Create preliminary NodeExecution entry to store inputs and start time
          const nodeExec: NodeExecution = {
            nodeId: event.nodeId,
            executionId: execution.id,
            state: 'running',
            inputs: event.inputs,
            outputs: undefined,
            logs: [],
            startTime: Date.now(),
            endTime: undefined,
            duration: undefined,
          };

          execution.nodeExecutions.set(event.nodeId, nodeExec);
        } else if (event.type === 'node.completed') {
          console.log(`✅ Node completed: ${event.nodeId}`, event.outputs);
          // Update node state using atomFamily
          this.store.set(nodeStateAtomFamily(event.nodeId), 'success');

          // Update execution with node execution data
          const existingNodeExec = execution.nodeExecutions.get(event.nodeId);
          const endTime = Date.now();
          const nodeExec: NodeExecution = {
            nodeId: event.nodeId,
            executionId: execution.id,
            state: 'success',
            inputs: existingNodeExec?.inputs || event.inputs || {},  // Use inputs from started event, fallback to completed event
            outputs: event.outputs,
            logs: existingNodeExec?.logs || [],
            startTime: existingNodeExec?.startTime || (endTime - event.duration),
            endTime,
            duration: event.duration,
          };

          execution.nodeExecutions.set(event.nodeId, nodeExec);
        } else if (event.type === 'node.error') {
          console.error(`❌ Node error: ${event.nodeId}`, event.error);
          // Update node state using atomFamily
          this.store.set(nodeStateAtomFamily(event.nodeId), 'error');

          // Create NodeExecution entry for the failed node
          const existingNodeExec = execution.nodeExecutions.get(event.nodeId);
          const nodeExec: NodeExecution = {
            nodeId: event.nodeId,
            executionId: execution.id,
            state: 'error',
            inputs: existingNodeExec?.inputs || {},  // Use inputs from started event if available
            outputs: undefined,
            logs: [],
            startTime: existingNodeExec?.startTime || Date.now(),
            endTime: Date.now(),
            duration: existingNodeExec?.startTime ? Date.now() - existingNodeExec.startTime : 0,
            error: event.error,
          };

          execution.nodeExecutions.set(event.nodeId, nodeExec);
        } else if (event.type === 'node.cached') {
          console.log(`💾 Node cached: ${event.nodeId}`);
          // Update node state using atomFamily
          this.store.set(nodeStateAtomFamily(event.nodeId), 'cached');
        }

        this.emitEvent(event);
      }

      // Mark execution as completed
      execution.status = 'completed';
      execution.duration = Date.now() - execution.timestamp;

      // Update execution in map with a new object to ensure reactivity
      const workflowExecutionsAtom = workflowExecutionsAtomFamily(workflow.id);
      const workflowExecutions = this.store.get(workflowExecutionsAtom);
      const index = workflowExecutions.findIndex((e) => e.id === execution.id);
      if (index !== -1) {
        // Create new execution object to ensure reactivity
        const updatedExecutions = [...workflowExecutions];
        updatedExecutions[index] = { ...execution };
        this.store.set(workflowExecutionsAtom, updatedExecutions);
      }

      this.emitEvent({
        type: 'execution.completed',
        executionId: execution.id,
        duration: execution.duration,
        outputs: execution.outputs ?? {},
      });
    } catch (error) {
      execution.status = 'error';
      execution.error = error as Error;

      // Update execution in map
      const workflowExecutionsAtom = workflowExecutionsAtomFamily(workflow.id);
      const workflowExecutions = this.store.get(workflowExecutionsAtom);
      const index = workflowExecutions.findIndex((e) => e.id === execution.id);
      if (index !== -1) {
        const updatedExecutions = [...workflowExecutions];
        updatedExecutions[index] = { ...execution };
        this.store.set(workflowExecutionsAtom, updatedExecutions);
      }

      this.emitEvent({
        type: 'execution.error',
        executionId: execution.id,
        error: error as Error,
      });
    } finally {
      this.activeExecutions.delete(execution.id);
    }
  }

  // ============================================================================
  // Cleanup
  // ============================================================================

  dispose(): void {
    // Cancel all running executions
    for (const controller of this.activeExecutions.values()) {
      controller.abort();
    }
    this.activeExecutions.clear();
  }
}

/**
 * Create a new BAML SDK instance
 */
export function createBAMLSDK(config: BAMLSDKConfig, store?: ReturnType<typeof createStore>): BAMLSDK {
  return new BAMLSDK(config, store);
}
