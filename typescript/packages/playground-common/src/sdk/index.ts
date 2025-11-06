/**
 * BAML SDK - Refactored Architecture
 *
 * Follows the immutable runtime pattern:
 * - Runtime is recreated on file changes (like wasmAtom)
 * - SDK orchestrates runtime and storage
 * - Storage abstraction allows swapping state management
 */

import type { SDKStorage } from './storage/SDKStorage';
import type { BamlRuntimeInterface, BamlRuntimeFactory } from './runtime/BamlRuntimeInterface';
import type {
  WorkflowDefinition,
  ExecutionSnapshot,
  NodeExecution,
  CacheEntry,
  TestCaseInput,
  NodeExecutionState,
} from './types';

// Import all atoms to expose via sdk.atoms
import * as coreAtoms from './atoms/core.atoms';

// Re-export types
export * from './types';
export * from './runtime/BamlRuntimeInterface';
export * from './storage/SDKStorage';
export * from './mock-config/types';

/**
 * BAML SDK - orchestrates runtime and storage
 * Follows wasmAtom pattern: creates new runtime instances on file changes
 */
export class BAMLSDK {
  private runtime: BamlRuntimeInterface | null = null;
  private storage: SDKStorage;
  private activeExecutions = new Map<string, AbortController>();
  private runtimeFactory: BamlRuntimeFactory;
  private currentFiles: Record<string, string> = {};

  /**
   * Expose all atoms directly via sdk.atoms
   * Components can access state via: sdk.atoms.workflows, sdk.atoms.diagnostics, etc.
   */
  atoms = coreAtoms;

  constructor(runtimeFactory: BamlRuntimeFactory, storage: SDKStorage) {
    this.runtimeFactory = runtimeFactory;
    this.storage = storage;
  }

  /**
   * Initialize SDK with initial files
   * Creates the first runtime instance
   */
  async initialize(
    initialFiles: Record<string, string>,
    options?: {
      envVars?: Record<string, string>;
      featureFlags?: string[];
    }
  ) {
    console.log('SDK: Initializing with', Object.keys(initialFiles).length, 'files');

    // Store files
    this.currentFiles = initialFiles;
    this.storage.setBAMLFiles(initialFiles);

    // Store env vars and feature flags
    if (options?.envVars) {
      this.storage.setEnvVars(options.envVars);
    }
    if (options?.featureFlags) {
      this.storage.setFeatureFlags(options.featureFlags);
    }

    // Create runtime from files (like wasmAtom creating WasmProject)
    this.runtime = await this.runtimeFactory(
      initialFiles,
      options?.envVars,
      options?.featureFlags
    );

    // Extract and store diagnostics
    const diagnostics = this.runtime.getDiagnostics();
    this.storage.setDiagnostics(diagnostics);

    // Check if runtime is valid (no compilation errors)
    const hasErrors = diagnostics.some((d) => d.type === 'error');
    this.storage.setLastValidRuntime(!hasErrors);

    // Extract and store generated files (only if runtime is valid)
    if (!hasErrors) {
      const generatedFiles = this.runtime.getGeneratedFiles();
      this.storage.setGeneratedFiles(generatedFiles);
    }

    // Load workflows from runtime into storage
    const workflows = this.runtime.getWorkflows();
    this.storage.setWorkflows(workflows);

    // Set first workflow as active
    if (workflows.length > 0) {
      this.storage.setActiveWorkflowId(workflows[0]!.id);
    }

    console.log('SDK: Initialized with', workflows.length, 'workflows,', diagnostics.length, 'diagnostics');
  }

  // ============================================================================
  // File Management API
  // ============================================================================

  files = {
    /**
     * Update files and recreate runtime
     * Matches wasmAtom pattern: create new runtime on every file change
     */
    update: async (files: Record<string, string>) => {
      console.log('SDK: Updating files, creating new runtime instance');

      // Store new files
      this.currentFiles = files;
      this.storage.setBAMLFiles(files);

      // Get current env vars and feature flags
      const envVars = this.storage.getEnvVars();
      const featureFlags = this.storage.getFeatureFlags();

      // Create new runtime instance (like wasmAtom creating new WasmProject)
      this.runtime = await this.runtimeFactory(files, envVars, featureFlags);

      // Extract and store diagnostics
      const diagnostics = this.runtime.getDiagnostics();
      this.storage.setDiagnostics(diagnostics);

      // Check if runtime is valid
      const hasErrors = diagnostics.some((d) => d.type === 'error');
      this.storage.setLastValidRuntime(!hasErrors);

      // Extract and store generated files (only if runtime is valid)
      if (!hasErrors) {
        const generatedFiles = this.runtime.getGeneratedFiles();
        this.storage.setGeneratedFiles(generatedFiles);
      }

      // Update workflows in storage
      const workflows = this.runtime.getWorkflows();
      this.storage.setWorkflows(workflows);

      console.log('SDK: Runtime recreated with', workflows.length, 'workflows,', diagnostics.length, 'diagnostics');
    },

    getCurrent: () => {
      return { ...this.currentFiles };
    },
  };

  // ============================================================================
  // Workflow API
  // ============================================================================

  workflows = {
    getAll: (): WorkflowDefinition[] => this.storage.getWorkflows(),

    getById: (id: string): WorkflowDefinition | null => {
      return this.storage.getWorkflows().find((w) => w.id === id) ?? null;
    },

    getActive: (): WorkflowDefinition | null => {
      const id = this.storage.getActiveWorkflowId();
      if (!id) return null;
      return this.workflows.getById(id);
    },

    setActive: (id: string) => {
      this.storage.setActiveWorkflowId(id);
    },
  };

  // ============================================================================
  // Execution API
  // ============================================================================

  executions = {
    start: async (
      workflowId: string,
      inputs: Record<string, any>,
      options?: { clearCache?: boolean; startFromNodeId?: string }
    ): Promise<string> => {
      const workflow = this.workflows.getById(workflowId);
      if (!workflow) throw new Error(`Workflow ${workflowId} not found`);

      // Clear cache if requested
      if (options?.clearCache) {
        this.cache.clear({ workflowId });
      }

      // Clear node states
      this.storage.clearAllNodeStates();

      // Wait for visual feedback
      await new Promise((resolve) => setTimeout(resolve, 200));

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

      // Add to storage
      this.storage.addExecution(workflowId, execution);

      // Run execution via runtime
      this.runExecution(executionId, workflowId, inputs, options);

      return executionId;
    },

    getAll: (workflowId: string): ExecutionSnapshot[] => {
      return this.storage.getExecutions(workflowId);
    },

    cancel: (executionId: string) => {
      const controller = this.activeExecutions.get(executionId);
      if (controller) {
        controller.abort();
        this.activeExecutions.delete(executionId);
      }
    },
  };

  // ============================================================================
  // Cache API
  // ============================================================================

  cache = {
    get: (nodeId: string, inputsHash: string): CacheEntry | null => {
      return this.storage.getCacheEntry(nodeId, inputsHash);
    },

    set: (entry: CacheEntry) => {
      this.storage.setCacheEntry(entry);
    },

    clear: (scope?: { workflowId?: string; nodeId?: string }) => {
      this.storage.clearCache(scope);
    },
  };

  // ============================================================================
  // Graph API
  // ============================================================================

  graph = {
    /**
     * Get graph structure for a workflow
     */
    getGraph: (workflowId: string): { nodes: any[]; edges: any[] } | null => {
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
      const workflows = this.storage.getWorkflows();
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

      this.storage.setWorkflows(updatedWorkflows);
    },
  };

  // ============================================================================
  // Test Cases API
  // ============================================================================

  testCases = {
    get: (nodeId: string): TestCaseInput[] => {
      if (!this.runtime) return [];
      return this.runtime.getTestCases(nodeId);
    },
  };

  // ============================================================================
  // Environment Variables API
  // ============================================================================

  envVars = {
    /**
     * Update environment variables and recreate runtime
     */
    update: async (envVars: Record<string, string>) => {
      console.log('SDK: Updating environment variables');

      // Store new env vars
      this.storage.setEnvVars(envVars);

      // Recreate runtime with new env vars
      const featureFlags = this.storage.getFeatureFlags();
      this.runtime = await this.runtimeFactory(this.currentFiles, envVars, featureFlags);

      // Extract and update state
      const diagnostics = this.runtime.getDiagnostics();
      this.storage.setDiagnostics(diagnostics);

      const hasErrors = diagnostics.some((d) => d.type === 'error');
      this.storage.setLastValidRuntime(!hasErrors);

      if (!hasErrors) {
        const generatedFiles = this.runtime.getGeneratedFiles();
        this.storage.setGeneratedFiles(generatedFiles);
      }

      const workflows = this.runtime.getWorkflows();
      this.storage.setWorkflows(workflows);

      console.log('SDK: Runtime recreated with updated env vars');
    },

    getCurrent: () => {
      return this.storage.getEnvVars();
    },
  };

  // ============================================================================
  // Feature Flags API
  // ============================================================================

  featureFlags = {
    /**
     * Update feature flags and recreate runtime
     */
    update: async (featureFlags: string[]) => {
      console.log('SDK: Updating feature flags');

      // Store new feature flags
      this.storage.setFeatureFlags(featureFlags);

      // Recreate runtime with new feature flags
      const envVars = this.storage.getEnvVars();
      this.runtime = await this.runtimeFactory(this.currentFiles, envVars, featureFlags);

      // Extract and update state
      const diagnostics = this.runtime.getDiagnostics();
      this.storage.setDiagnostics(diagnostics);

      const hasErrors = diagnostics.some((d) => d.type === 'error');
      this.storage.setLastValidRuntime(!hasErrors);

      if (!hasErrors) {
        const generatedFiles = this.runtime.getGeneratedFiles();
        this.storage.setGeneratedFiles(generatedFiles);
      }

      const workflows = this.runtime.getWorkflows();
      this.storage.setWorkflows(workflows);

      console.log('SDK: Runtime recreated with updated feature flags');
    },

    getCurrent: () => {
      return this.storage.getFeatureFlags();
    },
  };

  // ============================================================================
  // Generated Files API
  // ============================================================================

  generatedFiles = {
    /**
     * Get all generated files
     */
    getAll: (): typeof coreAtoms.generatedFilesAtom extends infer T ? T : never => {
      return this.storage.getGeneratedFiles() as any;
    },

    /**
     * Get generated files filtered by language
     */
    getByLanguage: (lang: string) => {
      return this.storage.getGeneratedFiles().filter((f) => f.outputDir.includes(lang));
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
      console.debug('[SDK] Cursor updated from range:', params);
    },

    /**
     * Select a function (navigate to it in the UI)
     */
    selectFunction: (functionName: string): void => {
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
    run: async (
      functionName: string,
      testCaseName: string
    ): Promise<{
      executionId: string;
      status: 'success' | 'error';
      duration: number;
      outputs?: Record<string, any>;
      error?: Error;
    }> => {
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
    ): AsyncGenerator<any> => {
      const self = this;
      async function* gen(): AsyncGenerator<any> {
        console.debug('[SDK] runAll:', tests, options);
        // TODO: Implement test running
      }
      return gen();
    },
  };

  // ============================================================================
  // Private: Run execution and update storage based on runtime events
  // ============================================================================

  private async runExecution(
    executionId: string,
    workflowId: string,
    inputs: Record<string, any>,
    options?: { clearCache?: boolean; startFromNodeId?: string }
  ) {
    if (!this.runtime) {
      console.error('Cannot run execution: runtime not initialized');
      return;
    }

    const controller = new AbortController();
    this.activeExecutions.set(executionId, controller);

    try {
      // Execute via runtime (stateless)
      for await (const event of this.runtime.executeWorkflow(workflowId, inputs, options)) {
        if (controller.signal.aborted) break;

        // Update storage based on event
        this.handleExecutionEvent(executionId, event);
      }

      // Mark execution as completed
      this.storage.updateExecution(executionId, {
        status: 'completed',
        duration: Date.now() - this.storage.getExecutions(workflowId).find((e) => e.id === executionId)!.timestamp,
      });
    } catch (error) {
      // Mark execution as error
      this.storage.updateExecution(executionId, {
        status: 'error',
        error: error as Error,
      });
    } finally {
      this.activeExecutions.delete(executionId);
    }
  }

  /**
   * Handle execution events from runtime and update storage
   * This is the key method that translates runtime events to state updates
   */
  private handleExecutionEvent(executionId: string, event: any) {
    switch (event.type) {
      case 'node.started':
        console.log(`▶️ Node started: ${event.nodeId}`);
        this.storage.setNodeState(event.nodeId, 'running');

        // Create preliminary node execution entry
        this.storage.addNodeExecution(executionId, event.nodeId, {
          nodeId: event.nodeId,
          executionId,
          state: 'running',
          inputs: event.inputs,
          outputs: undefined,
          logs: [],
          startTime: Date.now(),
          endTime: undefined,
          duration: undefined,
        });
        break;

      case 'node.completed':
        console.log(`✅ Node completed: ${event.nodeId}`);
        this.storage.setNodeState(event.nodeId, 'success');

        // Update node execution with results
        const existingNode = this.storage.getNodeExecution(executionId, event.nodeId);
        this.storage.addNodeExecution(executionId, event.nodeId, {
          ...existingNode!,
          state: 'success',
          outputs: event.outputs,
          endTime: Date.now(),
          duration: event.duration,
        });
        break;

      case 'node.error':
        console.error(`❌ Node error: ${event.nodeId}`);
        this.storage.setNodeState(event.nodeId, 'error');

        const errorNode = this.storage.getNodeExecution(executionId, event.nodeId);
        this.storage.addNodeExecution(executionId, event.nodeId, {
          ...errorNode!,
          state: 'error',
          error: event.error,
          endTime: Date.now(),
        });
        break;

      case 'node.cached':
        console.log(`💾 Node cached: ${event.nodeId}`);
        this.storage.setNodeState(event.nodeId, 'cached');
        break;

      case 'node.log':
        // Add log to node execution
        const node = this.storage.getNodeExecution(executionId, event.nodeId);
        if (node) {
          this.storage.addNodeExecution(executionId, event.nodeId, {
            ...node,
            logs: [...node.logs, event.log],
          });
        }
        break;
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
export function createBAMLSDK(
  runtimeFactory: BamlRuntimeFactory,
  storage: SDKStorage
): BAMLSDK {
  return new BAMLSDK(runtimeFactory, storage);
}
