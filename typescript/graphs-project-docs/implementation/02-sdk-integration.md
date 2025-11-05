# Phase 2: SDK Integration

**Timeline:** Week 2-3
**Dependencies:** Phase 1 (Unified Atoms)
**Risk Level:** High

## Purpose

Move the BAMLSDK from `apps/baml-graph` into `packages/playground-common`, extending it with WASM runtime integration, test execution, and VSCode-specific capabilities. The SDK becomes the central API layer for all business logic.

## What This Document Will Cover

- Complete SDK API surface definition with all namespaced methods
- Integration with unified atoms from Phase 1
- Extension points for WASM runtime
- Mode-based initialization (`vscode`, `mock`, `standalone`)
- Provider abstraction pattern for data sources
- Event system and subscription model
- Error handling and recovery strategies
- SDK lifecycle management (initialization, disposal)
- Testing strategy for SDK in isolation
- Migration path from EventListener to SDK

## Key Decisions

1. **Direct Store Access**: SDK operates on Jotai store directly for synchronous access (no `useAtom` hooks)
2. **Provider Pattern**: Abstracts data sources (mock for browser, VSCode for extension, future server)
3. **Event Emission**: SDK emits events for decoupling (components subscribe to changes)
4. **Namespaced API**: Organized by domain (`workflows.*`, `executions.*`, `graph.*`, `testCases.*`)
5. **Mode Detection**: Automatic based on environment (VSCode API presence)
6. **Backward Compatibility**: Preserve existing EventListener during migration
7. **Type Safety**: Full TypeScript support with comprehensive type definitions

## Source Files to Reference

### From baml-graph (SDK source)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/index.ts` (lines 39-508 - main SDK class)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/provider.tsx` (lines 23-82 - React provider)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/hooks.ts` (lines 1-200 - React hooks API)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/types.ts` (lines 1-318 - type definitions)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/mock.ts` (lines 1-1094 - mock data provider)

### From playground-common (integration points)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/baml_wasm_web/EventListener.tsx` (lines 57-217 - will call SDK)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/vscode.ts` (lines 63-535 - VSCode API wrapper)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/atoms.ts` (lines 121-339 - WASM runtime atoms)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md` (lines 882-1073 - SDK vs EventListener Architecture)
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC.md` (lines 520-689 - EventListener vs bamlSDK Pattern Analysis)
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC_ANSWERS.md` (lines 1-600 - all answers inform SDK design)

---

## Part 1: Complete SDK API Surface

### 1.1 SDK Directory Structure

```
packages/playground-common/src/sdk/
├── index.ts                  # Main BAMLSDK class and factory function
├── types.ts                  # All type definitions
├── provider.tsx              # BAMLSDKProvider React component
├── hooks.ts                  # React hooks for SDK access
├── mock.ts                   # Mock data provider for browser mode
├── vscode-provider.ts        # VSCode data provider (NEW)
├── wasm-integration.ts       # WASM runtime integration (NEW)
├── event-emitter.ts          # Event emission system
└── __tests__/
    ├── sdk.test.ts          # SDK unit tests
    ├── mock-provider.test.ts
    └── hooks.test.ts
```

### 1.2 Core SDK Class Structure

The SDK is organized into **6 namespaced APIs**:

```typescript
/**
 * Main BAML SDK Class
 *
 * Source: apps/baml-graph/src/sdk/index.ts:39-508
 * Extended with WASM and test execution capabilities
 */
export class BAMLSDK {
  // 1. Workflow Management
  workflows: {
    getAll(): WorkflowDefinition[]
    getById(id: string): WorkflowDefinition | null
    getActive(): WorkflowDefinition | null
    setActive(id: string | null): void
    getRecentWorkflows(): string[]
  }

  // 2. Execution Management
  executions: {
    getAll(workflowId: string): ExecutionSnapshot[]
    getById(executionId: string): ExecutionSnapshot | null
    getActive(): ExecutionSnapshot | null
    start(workflowId: string, inputs: Record<string, any>, options?: ExecutionOptions): Promise<string>
    cancel(executionId: string): void
    getNodeExecution(executionId: string, nodeId: string): NodeExecution | null
    getNodeState(nodeId: string): NodeExecutionState
    getEventStream(): BAMLEvent[]
  }

  // 3. Graph Operations
  graph: {
    getNodes(workflowId: string): GraphNode[]
    getEdges(workflowId: string): GraphEdge[]
    getNodeById(workflowId: string, nodeId: string): GraphNode | null
    updateNodePosition(workflowId: string, nodeId: string, position: { x: number; y: number }): void
  }

  // 4. Cache Management
  cache: {
    getEntry(nodeId: string, inputsHash: string): CacheEntry | null
    invalidateNode(nodeId: string): void
    invalidateWorkflow(workflowId: string): void
    clearAll(): void
    getStats(workflowId: string): { hits: number; misses: number }
  }

  // 5. Test Case Management
  testCases: {
    getAll(nodeId: string): TestCaseInput[]
    getById(testId: string): TestCaseInput | null
    run(testId: string): Promise<void>
    runAll(nodeId: string): Promise<void>
  }

  // 6. Input Library (NEW - Phase 6)
  inputs: {
    getAll(nodeId: string): InputSource[]
    getExecutionInputs(nodeId: string): ExecutionInput[]
    getTestInputs(nodeId: string): TestCaseInput[]
    getSelectedInput(): InputSource | null
    setSelectedInput(inputId: string): void
  }

  // Lifecycle methods
  initialize(): Promise<void>
  dispose(): void

  // Event subscription
  on(event: BAMLEvent['type'], callback: (event: BAMLEvent) => void): () => void
  off(event: BAMLEvent['type'], callback: (event: BAMLEvent) => void): void
  emit(event: BAMLEvent): void

  // WASM Integration (NEW)
  wasm: {
    getRuntime(): any | null
    compile(): Promise<void>
    getDiagnostics(): any[]
    getVersion(): string
  }

  // Settings (NEW)
  settings: {
    get<T>(key: string): T | undefined
    set<T>(key: string, value: T): void
    getAll(): Record<string, any>
  }
}
```

---

## Part 2: Complete Type Definitions

### 2.1 types.ts - Core Type Definitions

This file contains all type definitions used by the SDK and throughout the application.

```typescript
/**
 * BAML SDK Type Definitions
 *
 * Source: apps/baml-graph/src/sdk/types.ts:1-318
 * Base types from baml-graph, extended for playground-common integration
 */

// ============================================================================
// Core Node & Execution States
// ============================================================================

/**
 * Node execution state
 * Source: types.ts:10-17
 */
export type NodeExecutionState =
  | 'not-started' // Never executed
  | 'pending'     // Waiting for dependencies
  | 'running'     // Currently executing
  | 'success'     // Completed successfully
  | 'error'       // Failed with error
  | 'skipped'     // Conditionally skipped
  | 'cached';     // Using cached result

/**
 * Overall execution status
 * Source: types.ts:19-25
 */
export type ExecutionStatus =
  | 'pending'
  | 'running'
  | 'paused'
  | 'completed'
  | 'error'
  | 'cancelled';

// ============================================================================
// Graph Structure Types
// ============================================================================

/**
 * Node type classifications
 * Source: types.ts:31-37
 */
export type NodeType =
  | 'function'      // Regular function node
  | 'llm_function'  // LLM-powered function
  | 'conditional'   // Branching logic
  | 'loop'          // Iteration
  | 'return'        // Exit point
  | 'group';        // Container/subgraph node

/**
 * Graph node definition
 * Source: types.ts:39-55
 */
export interface GraphNode {
  id: string;
  type: NodeType;
  label: string;
  functionName?: string;
  position?: { x: number; y: number };
  parent?: string; // ID of parent group node (for subgraphs)

  // Cache invalidation tracking
  codeHash: string;      // Hash of the node's implementation
  lastModified: number;  // Timestamp when node code last changed

  // LLM-specific metadata
  llmClient?: string; // e.g., "GPT4o", "Claude-3"

  metadata?: Record<string, unknown>;
}

/**
 * Graph edge definition
 * Source: types.ts:57-63
 */
export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  label?: string;
  condition?: string; // For conditional branches
}

/**
 * Function parameter definition
 * Source: types.ts:65-70
 */
export interface Parameter {
  name: string;
  type: string;
  optional: boolean;
  defaultValue?: unknown;
}

// ============================================================================
// Workflow Definition
// ============================================================================

/**
 * Complete workflow definition
 * Source: types.ts:76-90
 */
export interface WorkflowDefinition {
  id: string;              // Function name
  displayName: string;     // Human-readable name
  filePath: string;        // Source file path
  startLine: number;
  endLine: number;
  nodes: GraphNode[];
  edges: GraphEdge[];
  entryPoint: string;      // Node ID to start execution
  parameters: Parameter[];
  returnType: string;
  childFunctions: string[]; // Functions called by this workflow
  lastModified: number;
  codeHash: string;        // For cache invalidation
}

// ============================================================================
// Execution & Node Execution
// ============================================================================

/**
 * Log entry for execution tracking
 * Source: types.ts:96-102
 */
export interface LogEntry {
  timestamp: number;
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
  metadata?: Record<string, unknown>;
  executionId: string;
}

/**
 * Node execution details
 * Source: types.ts:104-121
 */
export interface NodeExecution {
  nodeId: string;
  executionId: string;
  state: NodeExecutionState;
  inputs: Record<string, any>;
  outputs?: Record<string, any>;
  logs: LogEntry[];
  startTime: number;
  endTime?: number;
  duration?: number;
  error?: Error;
  metadata?: {
    llmClient?: string;
    llmRequest?: Record<string, any>;
    llmResponse?: Record<string, any>;
    [key: string]: any;
  };
}

/**
 * Complete execution snapshot
 * Source: types.ts:123-143
 */
export interface ExecutionSnapshot {
  id: string;
  workflowId: string;
  timestamp: number;

  // Frozen graph structure at execution time
  graphSnapshot: {
    nodes: GraphNode[];
    edges: GraphEdge[];
    codeHash: string;
  };

  status: ExecutionStatus;
  nodeExecutions: Map<string, NodeExecution>;
  trigger: 'manual' | 'auto' | 'test';
  duration?: number;
  branchPath: string[];    // Which conditional branches were taken
  inputs: Record<string, any>;
  outputs?: Record<string, any>;
  error?: Error;
}

// ============================================================================
// Cache Types
// ============================================================================

/**
 * Cache entry for node results
 * Source: types.ts:149-158
 */
export interface CacheEntry {
  nodeId: string;
  codeHash: string;        // Hash of node implementation when cached
  inputs: Record<string, any>;
  inputsHash: string;      // Hash of inputs for comparison
  outputs: Record<string, any>;
  executionId: string;
  timestamp: number;
  duration: number;
}

/**
 * Cache policy options
 * Source: types.ts:160
 */
export type CachePolicy = 'auto' | 'always-run' | 'always-cache';

// ============================================================================
// Code Synchronization
// ============================================================================

/**
 * Code position for navigation
 * Source: types.ts:166-171
 */
export interface CodePosition {
  filePath: string;
  line: number;
  column: number;
  functionName?: string;
}

// ============================================================================
// Events
// ============================================================================

/**
 * All SDK events
 * Source: types.ts:177-211
 */
export type BAMLEvent =
  // Workflow events
  | { type: 'workflow.discovered'; workflow: WorkflowDefinition }
  | { type: 'workflow.updated'; workflow: WorkflowDefinition; changes: string[] }
  | { type: 'workflow.deleted'; workflowId: string }
  | { type: 'workflow.selected'; workflowId: string }

  // Execution events
  | { type: 'execution.started'; executionId: string; workflowId: string }
  | { type: 'execution.completed'; executionId: string; duration: number; outputs: Record<string, unknown> }
  | { type: 'execution.error'; executionId: string; error: Error; nodeId?: string }
  | { type: 'execution.cancelled'; executionId: string; reason?: string }

  // Node events
  | { type: 'node.started'; executionId: string; nodeId: string; inputs: Record<string, unknown> }
  | { type: 'node.progress'; executionId: string; nodeId: string; progress: number }
  | { type: 'node.completed'; executionId: string; nodeId: string; inputs?: Record<string, unknown>; outputs: Record<string, unknown>; duration: number }
  | { type: 'node.error'; executionId: string; nodeId: string; error: Error }
  | { type: 'node.log'; executionId: string; nodeId: string; log: LogEntry }
  | { type: 'node.cached'; executionId: string; nodeId: string; fromExecutionId: string }

  // Graph events
  | { type: 'graph.changed'; workflowId: string; graph: { nodes: GraphNode[]; edges: GraphEdge[] } }
  | { type: 'graph.layout.updated'; workflowId: string; positions: Map<string, { x: number; y: number }> }

  // Code events
  | { type: 'code.modified'; filePath: string; affectedWorkflows: string[] }
  | { type: 'code.function.clicked'; functionName: string; position: CodePosition }
  | { type: 'code.function.hovered'; functionName: string; position: CodePosition }

  // Cache events
  | { type: 'cache.hit'; nodeId: string; executionId: string }
  | { type: 'cache.miss'; nodeId: string; executionId: string }
  | { type: 'cache.invalidated'; nodeIds: string[]; reason: string }
  | { type: 'cache.cleared'; scope: 'all' | 'workflow'; workflowId?: string };

// ============================================================================
// Input Library Types (Phase 6: Cursor Enrichment)
// ============================================================================

/**
 * Input source from a previous execution
 * Source: types.ts:219-230
 */
export interface ExecutionInput {
  id: string;          // executionId
  name: string;        // "Execution #3" or custom name
  source: 'execution';
  nodeId: string;
  executionId: string;
  timestamp: number;
  inputs: Record<string, any>;
  outputs?: Record<string, any>;
  status: 'success' | 'error' | 'running';
}

/**
 * Input source from a test case
 * Source: types.ts:235-245
 */
export interface TestCaseInput {
  id: string;          // testId
  name: string;        // "success_case"
  source: 'test';
  nodeId: string;
  filePath: string;
  inputs: Record<string, any>;
  expectedOutput?: Record<string, any>;
  status?: 'passing' | 'failing' | 'unknown';
  lastRun?: number;
}

/**
 * Manually created input source
 * Source: types.ts:250-258
 */
export interface ManualInput {
  id: string;          // UUID
  name: string;        // user-provided name
  source: 'manual';
  nodeId: string;
  inputs: Record<string, any>;
  createdAt: number;
  saved: boolean;
}

/**
 * Union type for all input sources
 * Source: types.ts:263
 */
export type InputSource = ExecutionInput | TestCaseInput | ManualInput;

// ============================================================================
// Debug Panel Types (for simulating BAML file interactions)
// ============================================================================

/**
 * BAML function metadata
 * Source: types.ts:269-273
 */
export interface BAMLFunction {
  name: string;
  type: 'workflow' | 'function' | 'llm_function';
  filePath: string; // Relative to project/
}

/**
 * BAML test metadata
 * Source: types.ts:275-280
 */
export interface BAMLTest {
  name: string;
  functionName: string; // Which function this test is for
  filePath: string;
  nodeType: 'llm_function' | 'function';
}

/**
 * BAML file structure
 * Source: types.ts:282-286
 */
export interface BAMLFile {
  path: string; // e.g., "workflows/workflow1.baml"
  functions: BAMLFunction[];
  tests: BAMLTest[];
}

/**
 * Code click events for navigation
 * Source: types.ts:288-299
 */
export type CodeClickEvent = {
  type: 'function';
  functionName: string;
  functionType: 'workflow' | 'function' | 'llm_function';
  filePath: string;
} | {
  type: 'test';
  testName: string;
  functionName: string;
  filePath: string;
  nodeType: 'llm_function' | 'function';
};

// ============================================================================
// SDK Configuration
// ============================================================================

/**
 * SDK configuration options
 * Source: types.ts:305-309
 */
export interface BAMLSDKConfig {
  mode: 'vscode' | 'mock' | 'server';
  mockData?: MockDataProvider;
  serverUrl?: string;
}

/**
 * Mock data provider interface
 * Source: types.ts:311-317
 */
export interface MockDataProvider {
  getWorkflows(): WorkflowDefinition[];
  getExecutions(workflowId: string): ExecutionSnapshot[];
  getTestCases(workflowId: string, nodeId: string): TestCaseInput[];
  simulateExecution(workflowId: string, inputs: Record<string, any>, startFromNodeId?: string): AsyncGenerator<BAMLEvent>;
  getBAMLFiles(): BAMLFile[];
}
```

---

## Part 3: Main SDK Implementation

### 3.1 index.ts - BAMLSDK Class

```typescript
/**
 * BAML SDK - Central API for all BAML operations
 *
 * Source: apps/baml-graph/src/sdk/index.ts:39-508
 * Extended with WASM integration, test execution, and VSCode capabilities
 */

import { atom, type Atom } from 'jotai';
import type { Store } from 'jotai/vanilla';
import EventEmitter from 'eventemitter3';
import {
  // Import unified atoms from Phase 1
  workflowsAtom,
  activeWorkflowIdAtom,
  activeWorkflowAtom,
  workflowExecutionsAtomFamily,
  activeExecutionIdAtom,
  nodeStateAtomFamily,
  nodeExecutionAtomFamily,
  executionEventStreamAtom,
  cacheEntriesAtomFamily,
  selectedInputSourceAtom,
  wasmAtom,
  runtimeAtom,
  diagnosticsAtom,
  settingsAtom,
} from '../shared/atoms';
import type {
  WorkflowDefinition,
  ExecutionSnapshot,
  NodeExecution,
  NodeExecutionState,
  GraphNode,
  GraphEdge,
  BAMLEvent,
  CacheEntry,
  TestCaseInput,
  InputSource,
  ExecutionInput,
  BAMLSDKConfig,
  MockDataProvider,
  BAMLFile,
} from './types';

/**
 * Execution options
 */
export interface ExecutionOptions {
  startFromNodeId?: string;
  cachePolicy?: 'auto' | 'always-run' | 'always-cache';
  inputs?: Record<string, any>;
}

/**
 * Main BAML SDK Class
 */
export class BAMLSDK {
  private store: Store;
  private config: BAMLSDKConfig;
  private eventEmitter: EventEmitter;
  private mockProvider?: MockDataProvider;
  private activeExecutionGenerator?: AsyncGenerator<BAMLEvent>;
  private isInitialized = false;

  constructor(config: BAMLSDKConfig, store: Store) {
    this.config = config;
    this.store = store;
    this.eventEmitter = new EventEmitter();

    if (config.mode === 'mock' && config.mockData) {
      this.mockProvider = config.mockData;
    }

    console.log('[BAMLSDK] Created SDK instance', { mode: config.mode });
  }

  // ============================================================================
  // 1. WORKFLOW MANAGEMENT
  // ============================================================================

  /**
   * Workflow management namespace
   * Source: apps/baml-graph/src/sdk/index.ts:74-110
   */
  workflows = {
    /**
     * Get all available workflows
     */
    getAll: (): WorkflowDefinition[] => {
      return this.store.get(workflowsAtom);
    },

    /**
     * Get workflow by ID
     */
    getById: (id: string): WorkflowDefinition | null => {
      const workflows = this.store.get(workflowsAtom);
      return workflows.find((w) => w.id === id) ?? null;
    },

    /**
     * Get currently active workflow
     */
    getActive: (): WorkflowDefinition | null => {
      return this.store.get(activeWorkflowAtom);
    },

    /**
     * Set active workflow
     */
    setActive: (id: string | null): void => {
      this.store.set(activeWorkflowIdAtom, id);

      if (id) {
        this.emit({
          type: 'workflow.selected',
          workflowId: id,
        });
      }
    },

    /**
     * Get recent workflows
     */
    getRecentWorkflows: (): string[] => {
      // TODO: Implement recent workflows tracking
      return [];
    },
  };

  // ============================================================================
  // 2. EXECUTION MANAGEMENT
  // ============================================================================

  /**
   * Execution management namespace
   * Source: apps/baml-graph/src/sdk/index.ts:116-220
   */
  executions = {
    /**
     * Get all executions for a workflow
     */
    getAll: (workflowId: string): ExecutionSnapshot[] => {
      const executionsAtom = workflowExecutionsAtomFamily(workflowId);
      return this.store.get(executionsAtom);
    },

    /**
     * Get execution by ID
     */
    getById: (executionId: string): ExecutionSnapshot | null => {
      // TODO: Need to search across all workflows
      // For now, check active workflow
      const activeWorkflow = this.workflows.getActive();
      if (!activeWorkflow) return null;

      const executions = this.executions.getAll(activeWorkflow.id);
      return executions.find((e) => e.id === executionId) ?? null;
    },

    /**
     * Get currently active execution
     */
    getActive: (): ExecutionSnapshot | null => {
      const activeExecutionId = this.store.get(activeExecutionIdAtom);
      if (!activeExecutionId) return null;
      return this.executions.getById(activeExecutionId);
    },

    /**
     * Start a new execution
     * Source: apps/baml-graph/src/sdk/index.ts:355-495
     */
    start: async (
      workflowId: string,
      inputs: Record<string, any>,
      options?: ExecutionOptions
    ): Promise<string> => {
      const workflow = this.workflows.getById(workflowId);
      if (!workflow) {
        throw new Error(`Workflow ${workflowId} not found`);
      }

      // Generate execution ID
      const executionId = `exec_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

      console.log('[BAMLSDK] Starting execution', { executionId, workflowId, inputs });

      // Emit start event
      this.emit({
        type: 'execution.started',
        executionId,
        workflowId,
      });

      // Create execution snapshot
      const snapshot: ExecutionSnapshot = {
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
        inputs,
        branchPath: [],
      };

      // Add to executions list
      const executionsAtom = workflowExecutionsAtomFamily(workflowId);
      const currentExecutions = this.store.get(executionsAtom);
      this.store.set(executionsAtom, [...currentExecutions, snapshot]);

      // Set as active execution
      this.store.set(activeExecutionIdAtom, executionId);

      // Start execution (async)
      this.runExecution(executionId, workflow, inputs, options);

      return executionId;
    },

    /**
     * Cancel an execution
     */
    cancel: (executionId: string): void => {
      console.log('[BAMLSDK] Cancelling execution', { executionId });

      // Stop the execution generator
      if (this.activeExecutionGenerator) {
        this.activeExecutionGenerator.return(undefined);
        this.activeExecutionGenerator = undefined;
      }

      // Emit cancel event
      this.emit({
        type: 'execution.cancelled',
        executionId,
        reason: 'User cancelled',
      });

      // Update execution status
      // TODO: Update snapshot status to 'cancelled'
    },

    /**
     * Get node execution details
     */
    getNodeExecution: (executionId: string, nodeId: string): NodeExecution | null => {
      const nodeExecutionAtom = nodeExecutionAtomFamily({ executionId, nodeId });
      return this.store.get(nodeExecutionAtom);
    },

    /**
     * Get current state of a node
     */
    getNodeState: (nodeId: string): NodeExecutionState => {
      const nodeStateAtom = nodeStateAtomFamily(nodeId);
      return this.store.get(nodeStateAtom);
    },

    /**
     * Get event stream for current execution
     */
    getEventStream: (): BAMLEvent[] => {
      return this.store.get(executionEventStreamAtom);
    },
  };

  // ============================================================================
  // 3. GRAPH OPERATIONS
  // ============================================================================

  /**
   * Graph operations namespace
   */
  graph = {
    /**
     * Get all nodes in a workflow
     */
    getNodes: (workflowId: string): GraphNode[] => {
      const workflow = this.workflows.getById(workflowId);
      return workflow?.nodes ?? [];
    },

    /**
     * Get all edges in a workflow
     */
    getEdges: (workflowId: string): GraphEdge[] => {
      const workflow = this.workflows.getById(workflowId);
      return workflow?.edges ?? [];
    },

    /**
     * Get specific node by ID
     */
    getNodeById: (workflowId: string, nodeId: string): GraphNode | null => {
      const nodes = this.graph.getNodes(workflowId);
      return nodes.find((n) => n.id === nodeId) ?? null;
    },

    /**
     * Update node position (for layout persistence)
     */
    updateNodePosition: (
      workflowId: string,
      nodeId: string,
      position: { x: number; y: number }
    ): void => {
      const workflows = this.store.get(workflowsAtom);
      const updatedWorkflows = workflows.map((w) => {
        if (w.id !== workflowId) return w;

        return {
          ...w,
          nodes: w.nodes.map((n) =>
            n.id === nodeId ? { ...n, position } : n
          ),
        };
      });

      this.store.set(workflowsAtom, updatedWorkflows);

      // Emit layout update event
      this.emit({
        type: 'graph.layout.updated',
        workflowId,
        positions: new Map([[nodeId, position]]),
      });
    },
  };

  // ============================================================================
  // 4. CACHE MANAGEMENT
  // ============================================================================

  /**
   * Cache management namespace
   */
  cache = {
    /**
     * Get cache entry for node + inputs
     */
    getEntry: (nodeId: string, inputsHash: string): CacheEntry | null => {
      const cacheEntriesAtom = cacheEntriesAtomFamily(nodeId);
      const entries = this.store.get(cacheEntriesAtom);
      return entries.find((e) => e.inputsHash === inputsHash) ?? null;
    },

    /**
     * Invalidate cache for a specific node
     */
    invalidateNode: (nodeId: string): void => {
      const cacheEntriesAtom = cacheEntriesAtomFamily(nodeId);
      this.store.set(cacheEntriesAtom, []);

      this.emit({
        type: 'cache.invalidated',
        nodeIds: [nodeId],
        reason: 'Manual invalidation',
      });
    },

    /**
     * Invalidate cache for entire workflow
     */
    invalidateWorkflow: (workflowId: string): void => {
      const workflow = this.workflows.getById(workflowId);
      if (!workflow) return;

      // Clear cache for all nodes in workflow
      workflow.nodes.forEach((node) => {
        this.cache.invalidateNode(node.id);
      });

      this.emit({
        type: 'cache.cleared',
        scope: 'workflow',
        workflowId,
      });
    },

    /**
     * Clear all cache entries
     */
    clearAll: (): void => {
      // TODO: Need a way to clear all atomFamily instances
      this.emit({
        type: 'cache.cleared',
        scope: 'all',
      });
    },

    /**
     * Get cache statistics
     */
    getStats: (workflowId: string): { hits: number; misses: number } => {
      // TODO: Track cache hits/misses
      return { hits: 0, misses: 0 };
    },
  };

  // ============================================================================
  // 5. TEST CASE MANAGEMENT
  // ============================================================================

  /**
   * Test case management namespace
   */
  testCases = {
    /**
     * Get all test cases for a node
     */
    getAll: (nodeId: string): TestCaseInput[] => {
      if (this.config.mode === 'mock' && this.mockProvider) {
        const activeWorkflow = this.workflows.getActive();
        if (!activeWorkflow) return [];
        return this.mockProvider.getTestCases(activeWorkflow.id, nodeId);
      }

      // TODO: Get from WASM runtime
      return [];
    },

    /**
     * Get test case by ID
     */
    getById: (testId: string): TestCaseInput | null => {
      // TODO: Implement test lookup
      return null;
    },

    /**
     * Run a single test case
     */
    run: async (testId: string): Promise<void> => {
      console.log('[BAMLSDK] Running test', { testId });
      // TODO: Implement test execution via WASM
    },

    /**
     * Run all tests for a node
     */
    runAll: async (nodeId: string): Promise<void> => {
      const tests = this.testCases.getAll(nodeId);
      console.log('[BAMLSDK] Running all tests', { nodeId, count: tests.length });

      for (const test of tests) {
        await this.testCases.run(test.id);
      }
    },
  };

  // ============================================================================
  // 6. INPUT LIBRARY (Phase 6)
  // ============================================================================

  /**
   * Input library namespace
   */
  inputs = {
    /**
     * Get all input sources for a node
     */
    getAll: (nodeId: string): InputSource[] => {
      const executionInputs = this.inputs.getExecutionInputs(nodeId);
      const testInputs = this.inputs.getTestInputs(nodeId);
      return [...executionInputs, ...testInputs];
    },

    /**
     * Get execution-based inputs
     */
    getExecutionInputs: (nodeId: string): ExecutionInput[] => {
      // TODO: Build from execution history
      return [];
    },

    /**
     * Get test-based inputs
     */
    getTestInputs: (nodeId: string): TestCaseInput[] => {
      return this.testCases.getAll(nodeId);
    },

    /**
     * Get selected input source
     */
    getSelectedInput: (): InputSource | null => {
      return this.store.get(selectedInputSourceAtom);
    },

    /**
     * Set selected input source
     */
    setSelectedInput: (inputId: string): void => {
      const allInputs = this.inputs.getAll(''); // Get all inputs
      const input = allInputs.find((i) => i.id === inputId);
      if (input) {
        this.store.set(selectedInputSourceAtom, input);
      }
    },
  };

  // ============================================================================
  // 7. WASM INTEGRATION (NEW)
  // ============================================================================

  /**
   * WASM runtime integration namespace
   */
  wasm = {
    /**
     * Get WASM runtime instance
     */
    getRuntime: (): any | null => {
      return this.store.get(runtimeAtom);
    },

    /**
     * Compile BAML files
     */
    compile: async (): Promise<void> => {
      console.log('[BAMLSDK] Compiling BAML files');
      // Trigger compilation via WASM
      const wasm = this.store.get(wasmAtom);
      if (!wasm) {
        throw new Error('WASM not loaded');
      }
      // TODO: Trigger recompilation
    },

    /**
     * Get compilation diagnostics
     */
    getDiagnostics: (): any[] => {
      return this.store.get(diagnosticsAtom);
    },

    /**
     * Get WASM version
     */
    getVersion: (): string => {
      const wasm = this.store.get(wasmAtom);
      return wasm?.version() ?? 'unknown';
    },
  };

  // ============================================================================
  // 8. SETTINGS (NEW)
  // ============================================================================

  /**
   * Settings management namespace
   */
  settings = {
    /**
     * Get setting value
     */
    get: <T>(key: string): T | undefined => {
      const settings = this.store.get(settingsAtom);
      return settings[key] as T;
    },

    /**
     * Set setting value
     */
    set: <T>(key: string, value: T): void => {
      const settings = this.store.get(settingsAtom);
      this.store.set(settingsAtom, { ...settings, [key]: value });
    },

    /**
     * Get all settings
     */
    getAll: (): Record<string, any> => {
      return this.store.get(settingsAtom);
    },
  };

  // ============================================================================
  // LIFECYCLE METHODS
  // ============================================================================

  /**
   * Initialize the SDK
   * Loads initial data based on mode
   */
  async initialize(): Promise<void> {
    if (this.isInitialized) {
      console.warn('[BAMLSDK] Already initialized');
      return;
    }

    console.log('[BAMLSDK] Initializing SDK', { mode: this.config.mode });

    if (this.config.mode === 'mock' && this.mockProvider) {
      // Load mock workflows
      const workflows = this.mockProvider.getWorkflows();
      this.store.set(workflowsAtom, workflows);
      console.log('[BAMLSDK] Loaded mock workflows', { count: workflows.length });

      // Set first workflow as active
      if (workflows.length > 0) {
        this.workflows.setActive(workflows[0].id);
      }
    } else if (this.config.mode === 'vscode') {
      // Wait for WASM to load, then derive workflows from runtime
      // TODO: Implement workflow discovery from WASM
      console.log('[BAMLSDK] VSCode mode - waiting for WASM runtime');
    }

    this.isInitialized = true;
    console.log('[BAMLSDK] Initialization complete');
  }

  /**
   * Dispose SDK and cleanup resources
   */
  dispose(): void {
    console.log('[BAMLSDK] Disposing SDK');
    this.eventEmitter.removeAllListeners();
    this.isInitialized = false;
  }

  // ============================================================================
  // EVENT SYSTEM
  // ============================================================================

  /**
   * Subscribe to SDK events
   */
  on(event: BAMLEvent['type'], callback: (event: BAMLEvent) => void): () => void {
    this.eventEmitter.on(event, callback);
    return () => this.eventEmitter.off(event, callback);
  }

  /**
   * Unsubscribe from SDK events
   */
  off(event: BAMLEvent['type'], callback: (event: BAMLEvent) => void): void {
    this.eventEmitter.off(event, callback);
  }

  /**
   * Emit an SDK event
   */
  emit(event: BAMLEvent): void {
    console.log('[BAMLSDK] Event:', event.type, event);
    this.eventEmitter.emit(event.type, event);

    // Also add to event stream atom for UI display
    const currentStream = this.store.get(executionEventStreamAtom);
    this.store.set(executionEventStreamAtom, [...currentStream, event].slice(-100)); // Keep last 100
  }

  // ============================================================================
  // PRIVATE EXECUTION ENGINE
  // ============================================================================

  /**
   * Run execution with event streaming
   * Source: apps/baml-graph/src/sdk/index.ts:355-495
   */
  private async runExecution(
    executionId: string,
    workflow: WorkflowDefinition,
    inputs: Record<string, any>,
    options?: ExecutionOptions
  ): Promise<void> {
    try {
      if (this.config.mode === 'mock' && this.mockProvider) {
        // Use mock execution generator
        this.activeExecutionGenerator = this.mockProvider.simulateExecution(
          workflow.id,
          inputs,
          options?.startFromNodeId
        );

        // Process events from generator
        for await (const event of this.activeExecutionGenerator) {
          this.handleExecutionEvent(executionId, event);
        }

        // Mark execution as completed
        this.emit({
          type: 'execution.completed',
          executionId,
          duration: Date.now() - Date.now(), // TODO: Track actual duration
          outputs: {},
        });
      } else {
        // Real execution via WASM
        // TODO: Implement real execution
        console.log('[BAMLSDK] Real execution not yet implemented');
      }
    } catch (error) {
      console.error('[BAMLSDK] Execution error', error);
      this.emit({
        type: 'execution.error',
        executionId,
        error: error as Error,
      });
    }
  }

  /**
   * Handle execution event and update atoms
   */
  private handleExecutionEvent(executionId: string, event: BAMLEvent): void {
    // Update node states based on events
    if (event.type === 'node.started') {
      const nodeStateAtom = nodeStateAtomFamily(event.nodeId);
      this.store.set(nodeStateAtom, 'running');
    } else if (event.type === 'node.completed') {
      const nodeStateAtom = nodeStateAtomFamily(event.nodeId);
      this.store.set(nodeStateAtom, 'success');

      // Store node execution details
      const nodeExecutionAtom = nodeExecutionAtomFamily({
        executionId: event.executionId,
        nodeId: event.nodeId,
      });
      const nodeExecution: NodeExecution = {
        nodeId: event.nodeId,
        executionId: event.executionId,
        state: 'success',
        inputs: event.inputs || {},
        outputs: event.outputs,
        logs: [],
        startTime: Date.now() - event.duration,
        endTime: Date.now(),
        duration: event.duration,
      };
      this.store.set(nodeExecutionAtom, nodeExecution);
    } else if (event.type === 'node.error') {
      const nodeStateAtom = nodeStateAtomFamily(event.nodeId);
      this.store.set(nodeStateAtom, 'error');
    } else if (event.type === 'node.cached') {
      const nodeStateAtom = nodeStateAtomFamily(event.nodeId);
      this.store.set(nodeStateAtom, 'cached');
    }

    // Emit event for subscribers
    this.emit(event);
  }
}

/**
 * Factory function to create SDK instance
 */
export function createBAMLSDK(config: BAMLSDKConfig, store: Store): BAMLSDK {
  return new BAMLSDK(config, store);
}
```

### 3.2 Mode Detection Utility

```typescript
/**
 * Detect SDK mode based on environment
 */
export function detectSDKMode(): BAMLSDKConfig['mode'] {
  // Check if running in VSCode webview
  if (typeof acquireVsCodeApi === 'function') {
    return 'vscode';
  }

  // Check if running in standalone browser (playground)
  if (typeof window !== 'undefined' && window.location.hostname === 'localhost') {
    // Could be standalone playground with server
    // TODO: Check if server is available
    return 'mock';
  }

  // Default to mock mode
  return 'mock';
}

/**
 * Auto-create SDK config based on environment
 */
export function createAutoSDKConfig(): BAMLSDKConfig {
  const mode = detectSDKMode();

  if (mode === 'mock') {
    return {
      mode: 'mock',
      mockData: new DefaultMockProvider(),
    };
  }

  return { mode };
}
```

---

## Part 4: React Integration

### 4.1 provider.tsx - React Provider Component

```typescript
/**
 * BAML SDK Provider for React
 *
 * Source: apps/baml-graph/src/sdk/provider.tsx:23-82
 * Provides SDK instance through React Context
 */

import { Provider as JotaiProvider, createStore } from 'jotai';
import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from 'react';
import { BAMLSDK, createBAMLSDK } from './index';
import { createMockSDKConfig } from './mock';
import type { BAMLSDKConfig } from './types';

const BAMLSDKContext = createContext<BAMLSDK | null>(null);

interface BAMLSDKProviderProps {
  children: ReactNode;
  config?: BAMLSDKConfig;
}

/**
 * Provider component that wraps the app and provides SDK access
 *
 * Usage:
 * ```tsx
 * <BAMLSDKProvider config={config}>
 *   <App />
 * </BAMLSDKProvider>
 * ```
 */
export function BAMLSDKProvider({ children, config }: BAMLSDKProviderProps) {
  // Create refs to ensure single instance creation
  const storeRef = useRef<ReturnType<typeof createStore> | undefined>(undefined);
  const sdkRef = useRef<BAMLSDK | undefined>(undefined);

  // Initialize store and SDK only once
  if (!storeRef.current) {
    storeRef.current = createStore();
  }

  if (!sdkRef.current) {
    const sdkConfig = config ?? createMockSDKConfig();
    console.log('🚀 Creating BAML SDK with config:', sdkConfig.mode);
    sdkRef.current = createBAMLSDK(sdkConfig, storeRef.current);
  }

  const [isInitialized, setIsInitialized] = useState(false);

  // Handle async initialization only once
  useEffect(() => {
    let mounted = true;

    async function init() {
      if (!sdkRef.current) return;

      console.log('⏳ Initializing SDK...');
      await sdkRef.current.initialize();
      if (mounted) {
        console.log('✅ SDK initialized successfully');
        const workflows = sdkRef.current.workflows.getAll();
        console.log('📦 Loaded workflows:', workflows.length, workflows.map(w => w.id));
        setIsInitialized(true);
      }
    }

    init();

    return () => {
      mounted = false;
      // Cleanup SDK on unmount
      if (sdkRef.current) {
        sdkRef.current.dispose();
      }
    };
  }, []); // Empty deps - only run once

  // Show loading state while SDK initializes
  if (!isInitialized) {
    return (
      <div className="w-screen h-screen flex items-center justify-center bg-background">
        <div className="text-center">
          <div className="text-xl font-semibold">Loading BAML SDK...</div>
          <div className="text-sm text-muted-foreground mt-2">Initializing workflows</div>
        </div>
      </div>
    );
  }

  return (
    <BAMLSDKContext.Provider value={sdkRef.current}>
      <JotaiProvider store={storeRef.current}>{children}</JotaiProvider>
    </BAMLSDKContext.Provider>
  );
}

/**
 * Hook to access the SDK instance
 *
 * Usage:
 * ```tsx
 * function MyComponent() {
 *   const sdk = useBAMLSDK();
 *   const workflows = sdk.workflows.getAll();
 *   return <div>...</div>;
 * }
 * ```
 */
export function useBAMLSDK(): BAMLSDK {
  const sdk = useContext(BAMLSDKContext);
  if (!sdk) {
    throw new Error('useBAMLSDK must be used within BAMLSDKProvider');
  }
  return sdk;
}
```

### 4.2 hooks.ts - React Hooks

```typescript
/**
 * React Hooks for BAML SDK
 *
 * Source: apps/baml-graph/src/sdk/hooks.ts
 * Convenience hooks that wrap SDK + Jotai atoms
 */

import { useAtomValue, useSetAtom } from 'jotai';
import { useCallback, useEffect, useState } from 'react';
import { useBAMLSDK } from './provider';
import {
  workflowsAtom,
  activeWorkflowAtom,
  activeExecutionIdAtom,
  nodeStateAtomFamily,
  executionEventStreamAtom,
} from '../shared/atoms';
import type {
  WorkflowDefinition,
  ExecutionSnapshot,
  NodeExecutionState,
  BAMLEvent,
} from './types';

/**
 * Hook to get all workflows
 */
export function useWorkflows(): WorkflowDefinition[] {
  return useAtomValue(workflowsAtom);
}

/**
 * Hook to get and set active workflow
 */
export function useActiveWorkflow() {
  const sdk = useBAMLSDK();
  const activeWorkflow = useAtomValue(activeWorkflowAtom);

  const setActive = useCallback(
    (id: string | null) => {
      sdk.workflows.setActive(id);
    },
    [sdk]
  );

  return [activeWorkflow, setActive] as const;
}

/**
 * Hook to get specific workflow by ID
 */
export function useWorkflow(workflowId: string): WorkflowDefinition | null {
  const sdk = useBAMLSDK();
  const workflows = useWorkflows();

  return sdk.workflows.getById(workflowId);
}

/**
 * Hook to get executions for active workflow
 */
export function useExecutions(): ExecutionSnapshot[] {
  const sdk = useBAMLSDK();
  const activeWorkflow = useAtomValue(activeWorkflowAtom);

  if (!activeWorkflow) return [];

  return sdk.executions.getAll(activeWorkflow.id);
}

/**
 * Hook to start an execution
 */
export function useStartExecution() {
  const sdk = useBAMLSDK();

  return useCallback(
    async (workflowId: string, inputs: Record<string, any>) => {
      return await sdk.executions.start(workflowId, inputs);
    },
    [sdk]
  );
}

/**
 * Hook to get node state with reactivity
 */
export function useNodeState(nodeId: string): NodeExecutionState {
  return useAtomValue(nodeStateAtomFamily(nodeId));
}

/**
 * Hook to get event stream
 */
export function useEventStream(): BAMLEvent[] {
  return useAtomValue(executionEventStreamAtom);
}

/**
 * Hook to subscribe to specific SDK events
 */
export function useSDKEvent(
  eventType: BAMLEvent['type'],
  callback: (event: BAMLEvent) => void
) {
  const sdk = useBAMLSDK();

  useEffect(() => {
    const unsubscribe = sdk.on(eventType, callback);
    return unsubscribe;
  }, [sdk, eventType, callback]);
}

/**
 * Hook to track execution progress
 */
export function useExecutionProgress(executionId: string | null) {
  const sdk = useBAMLSDK();
  const [progress, setProgress] = useState(0);
  const [status, setStatus] = useState<'running' | 'completed' | 'error'>('running');

  useEffect(() => {
    if (!executionId) return;

    const unsubscribeProgress = sdk.on('node.progress', (event) => {
      if (event.executionId === executionId) {
        setProgress(event.progress);
      }
    });

    const unsubscribeCompleted = sdk.on('execution.completed', (event) => {
      if (event.executionId === executionId) {
        setStatus('completed');
        setProgress(100);
      }
    });

    const unsubscribeError = sdk.on('execution.error', (event) => {
      if (event.executionId === executionId) {
        setStatus('error');
      }
    });

    return () => {
      unsubscribeProgress();
      unsubscribeCompleted();
      unsubscribeError();
    };
  }, [sdk, executionId]);

  return { progress, status };
}

/**
 * Hook to get graph data (nodes + edges)
 */
export function useGraph(workflowId: string | null) {
  const sdk = useBAMLSDK();

  if (!workflowId) {
    return { nodes: [], edges: [] };
  }

  return {
    nodes: sdk.graph.getNodes(workflowId),
    edges: sdk.graph.getEdges(workflowId),
  };
}

/**
 * Hook to manage test cases
 */
export function useTestCases(nodeId: string) {
  const sdk = useBAMLSDK();
  const [testCases, setTestCases] = useState(sdk.testCases.getAll(nodeId));

  const refresh = useCallback(() => {
    setTestCases(sdk.testCases.getAll(nodeId));
  }, [sdk, nodeId]);

  const runTest = useCallback(
    async (testId: string) => {
      await sdk.testCases.run(testId);
      refresh();
    },
    [sdk, refresh]
  );

  const runAll = useCallback(async () => {
    await sdk.testCases.runAll(nodeId);
    refresh();
  }, [sdk, nodeId, refresh]);

  return {
    testCases,
    runTest,
    runAll,
    refresh,
  };
}
```

---

## Part 5: WASM Integration Extension

### 5.1 wasm-integration.ts - WASM Runtime Bridge

```typescript
/**
 * WASM Runtime Integration for SDK
 *
 * Extends SDK with WASM-specific capabilities:
 * - Workflow discovery from compiled BAML
 * - Test execution via WASM runtime
 * - Compilation and diagnostics
 */

import type { Store } from 'jotai/vanilla';
import {
  wasmAtom,
  runtimeAtom,
  diagnosticsAtom,
  filesAtom,
  workflowsAtom,
} from '../shared/atoms';
import type { WorkflowDefinition, BAMLFile, BAMLFunction } from './types';

/**
 * Discover workflows from WASM runtime
 *
 * Integration point: playground-common/src/shared/baml-project-panel/atoms.ts:169-240
 */
export async function discoverWorkflowsFromWASM(store: Store): Promise<WorkflowDefinition[]> {
  const runtime = store.get(runtimeAtom);

  if (!runtime) {
    console.warn('[WASM] Runtime not available');
    return [];
  }

  try {
    // Get all functions from runtime
    const functions = runtime.getFunctions?.() ?? [];

    // Convert BAML functions to workflow definitions
    const workflows: WorkflowDefinition[] = functions
      .filter((fn: any) => fn.type === 'workflow')
      .map((fn: any) => convertBAMLFunctionToWorkflow(fn));

    console.log('[WASM] Discovered workflows', { count: workflows.length });

    return workflows;
  } catch (error) {
    console.error('[WASM] Failed to discover workflows', error);
    return [];
  }
}

/**
 * Convert BAML function to workflow definition
 */
function convertBAMLFunctionToWorkflow(bamlFunction: any): WorkflowDefinition {
  // TODO: Parse function body to extract nodes and edges
  // This is a simplified version
  return {
    id: bamlFunction.name,
    displayName: bamlFunction.name,
    filePath: bamlFunction.filePath ?? '',
    startLine: bamlFunction.span?.start_line ?? 0,
    endLine: bamlFunction.span?.end_line ?? 0,
    nodes: [], // TODO: Parse from function body
    edges: [], // TODO: Parse from function body
    entryPoint: '', // TODO: Determine entry point
    parameters: bamlFunction.parameters ?? [],
    returnType: bamlFunction.returnType ?? 'any',
    childFunctions: [], // TODO: Extract child function calls
    lastModified: Date.now(),
    codeHash: computeHash(bamlFunction),
  };
}

/**
 * Compute hash for cache invalidation
 */
function computeHash(data: any): string {
  return `hash_${JSON.stringify(data)}_${Date.now()}`;
}

/**
 * Execute test via WASM runtime
 */
export async function executeTestViaWASM(
  store: Store,
  functionName: string,
  testName: string
): Promise<void> {
  const runtime = store.get(runtimeAtom);

  if (!runtime) {
    throw new Error('WASM runtime not available');
  }

  console.log('[WASM] Executing test', { functionName, testName });

  try {
    // Execute test via runtime
    const result = await runtime.runTest?.(functionName, testName);
    console.log('[WASM] Test result', result);
  } catch (error) {
    console.error('[WASM] Test execution failed', error);
    throw error;
  }
}

/**
 * Trigger WASM compilation
 */
export async function compileBAMLFiles(store: Store): Promise<void> {
  const wasm = store.get(wasmAtom);

  if (!wasm) {
    throw new Error('WASM not loaded');
  }

  console.log('[WASM] Starting compilation');

  try {
    // Trigger recompilation
    const files = store.get(filesAtom);
    // TODO: Call WASM compile method
    console.log('[WASM] Compilation complete');
  } catch (error) {
    console.error('[WASM] Compilation failed', error);
    throw error;
  }
}
```

---

## Part 6: Migration Strategy

### 6.1 Migration Phases

**Phase 2.1: Create SDK Package (Week 2, Days 1-2)**

1. Create SDK directory structure
2. Copy types.ts from baml-graph
3. Copy and adapt index.ts (main SDK class)
4. Copy provider.tsx and hooks.ts
5. Copy mock.ts for browser mode
6. Run type checking - fix imports
7. Write basic SDK tests

**Phase 2.2: WASM Integration (Week 2, Days 3-4)**

1. Create wasm-integration.ts
2. Extend SDK with wasm namespace
3. Implement workflow discovery from WASM
4. Test in VSCode extension with real WASM

**Phase 2.3: EventListener Coexistence (Week 2, Day 5)**

1. Keep existing EventListener intact
2. Have EventListener call SDK methods
3. SDK emits events, EventListener forwards them
4. Gradual migration without breaking changes

Example coexistence pattern:

```typescript
// EventListener.tsx (updated)
export const EventListener: React.FC = () => {
  const sdk = useBAMLSDK(); // Access SDK
  const updateCursor = useSetAtom(updateCursorAtom);

  useEffect(() => {
    const fn = (event: MessageEvent<VscodeToWebviewCommand>) => {
      const { source, payload } = event.data;

      switch (source) {
        case 'ide_message':
          const { command, content } = payload;
          switch (command) {
            case 'update_cursor':
              // Still use existing atoms
              updateCursor(content);

              // But also notify SDK
              sdk.emit({
                type: 'code.function.clicked',
                functionName: content.functionName,
                position: {
                  filePath: content.fileName,
                  line: content.line,
                  column: content.column,
                },
              });
              break;
          }
          break;
      }
    };

    window.addEventListener('message', fn);
    return () => window.removeEventListener('message', fn);
  }, [sdk, updateCursor]);

  return null;
};
```

**Phase 2.4: Component Integration (Week 3, Days 1-3)**

1. Update WorkflowView to use SDK hooks
2. Update graph components to use SDK
3. Update test panel to use SDK
4. Verify all features work with SDK

**Phase 2.5: Testing & Documentation (Week 3, Days 4-5)**

1. Write comprehensive SDK tests
2. Test in all modes (mock, VSCode, standalone)
3. Document SDK API
4. Create migration guide for components

### 6.2 Backward Compatibility Strategy

Keep EventListener during migration:

```typescript
/**
 * Compatibility layer - EventListener calls SDK
 */
export const EventListenerWithSDK: React.FC = () => {
  const sdk = useBAMLSDK();

  // EventListener continues to handle VSCode messages
  // but delegates business logic to SDK

  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      // Parse message
      // Call appropriate SDK method
      // SDK updates atoms
      // Components react to atom changes
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, [sdk]);

  return null;
};
```

### 6.3 Testing Strategy

**Unit Tests (sdk.test.ts)**

```typescript
import { createStore } from 'jotai';
import { BAMLSDK } from '../index';
import { createMockSDKConfig } from '../mock';

describe('BAMLSDK', () => {
  let store: ReturnType<typeof createStore>;
  let sdk: BAMLSDK;

  beforeEach(() => {
    store = createStore();
    const config = createMockSDKConfig();
    sdk = new BAMLSDK(config, store);
  });

  afterEach(() => {
    sdk.dispose();
  });

  describe('workflows', () => {
    it('should get all workflows', async () => {
      await sdk.initialize();
      const workflows = sdk.workflows.getAll();
      expect(workflows).toHaveLength(3);
    });

    it('should set active workflow', async () => {
      await sdk.initialize();
      const workflows = sdk.workflows.getAll();

      sdk.workflows.setActive(workflows[0].id);
      const active = sdk.workflows.getActive();

      expect(active?.id).toBe(workflows[0].id);
    });
  });

  describe('executions', () => {
    it('should start execution', async () => {
      await sdk.initialize();
      const workflows = sdk.workflows.getAll();

      const executionId = await sdk.executions.start(
        workflows[0].id,
        { input: 'test' }
      );

      expect(executionId).toMatch(/^exec_/);
    });

    it('should emit execution events', async () => {
      await sdk.initialize();
      const workflows = sdk.workflows.getAll();

      const events: any[] = [];
      sdk.on('execution.started', (event) => events.push(event));

      await sdk.executions.start(workflows[0].id, { input: 'test' });

      expect(events).toHaveLength(1);
      expect(events[0].type).toBe('execution.started');
    });
  });

  describe('event system', () => {
    it('should subscribe to events', () => {
      const callback = jest.fn();
      const unsubscribe = sdk.on('workflow.selected', callback);

      sdk.emit({ type: 'workflow.selected', workflowId: 'test' });

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith({
        type: 'workflow.selected',
        workflowId: 'test',
      });

      unsubscribe();
    });

    it('should unsubscribe from events', () => {
      const callback = jest.fn();
      const unsubscribe = sdk.on('workflow.selected', callback);

      unsubscribe();
      sdk.emit({ type: 'workflow.selected', workflowId: 'test' });

      expect(callback).not.toHaveBeenCalled();
    });
  });
});
```

**Integration Tests**

```typescript
describe('SDK Integration', () => {
  it('should work with React components', () => {
    const { result } = renderHook(() => useWorkflows(), {
      wrapper: ({ children }) => (
        <BAMLSDKProvider config={createMockSDKConfig()}>
          {children}
        </BAMLSDKProvider>
      ),
    });

    expect(result.current).toHaveLength(3);
  });

  it('should sync state across hooks', async () => {
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <BAMLSDKProvider config={createMockSDKConfig()}>
        {children}
      </BAMLSDKProvider>
    );

    const { result: workflowsResult } = renderHook(() => useWorkflows(), { wrapper });
    const { result: activeResult } = renderHook(() => useActiveWorkflow(), { wrapper });

    // Set active workflow
    act(() => {
      activeResult.current[1](workflowsResult.current[0].id);
    });

    // Verify state is synced
    expect(activeResult.current[0]?.id).toBe(workflowsResult.current[0].id);
  });
});
```

---

## Part 7: Implementation Checklist

### 7.1 Core SDK Implementation

- [ ] Create `packages/playground-common/src/sdk/` directory
- [ ] **types.ts**: Copy all type definitions from baml-graph (318 lines)
- [ ] **index.ts**: Implement BAMLSDK class with all 6 namespaces (~800 lines)
  - [ ] workflows namespace (5 methods)
  - [ ] executions namespace (7 methods)
  - [ ] graph namespace (4 methods)
  - [ ] cache namespace (5 methods)
  - [ ] testCases namespace (4 methods)
  - [ ] inputs namespace (5 methods)
  - [ ] wasm namespace (4 methods) - NEW
  - [ ] settings namespace (3 methods) - NEW
  - [ ] Event system (on/off/emit)
  - [ ] Execution engine (runExecution, handleExecutionEvent)
- [ ] **provider.tsx**: React provider component (100 lines)
- [ ] **hooks.ts**: React hooks for SDK access (200 lines)
  - [ ] useWorkflows
  - [ ] useActiveWorkflow
  - [ ] useExecutions
  - [ ] useStartExecution
  - [ ] useNodeState
  - [ ] useEventStream
  - [ ] useSDKEvent
  - [ ] useExecutionProgress
  - [ ] useGraph
  - [ ] useTestCases

### 7.2 WASM Integration

- [ ] **wasm-integration.ts**: WASM runtime bridge (~200 lines)
  - [ ] discoverWorkflowsFromWASM()
  - [ ] executeTestViaWASM()
  - [ ] compileBAMLFiles()
  - [ ] convertBAMLFunctionToWorkflow()
- [ ] Test WASM integration in VSCode extension
- [ ] Verify workflow discovery from compiled BAML

### 7.3 Mock Data Provider

- [ ] **mock.ts**: Copy from baml-graph (1094 lines)
  - [ ] DefaultMockProvider class
  - [ ] Sample workflows (3 workflows)
  - [ ] Test cases (20+ tests)
  - [ ] BAMLFile definitions
  - [ ] Execution simulation with event streaming
- [ ] Verify mock data validation
- [ ] Test mock execution in browser

### 7.4 EventListener Integration

- [ ] Update EventListener to call SDK methods
- [ ] Preserve existing EventListener functionality
- [ ] Add SDK event emission from EventListener
- [ ] Test backward compatibility

### 7.5 Testing

- [ ] **sdk.test.ts**: Unit tests for BAMLSDK class
  - [ ] Test all namespaced APIs
  - [ ] Test event subscription
  - [ ] Test execution flow
  - [ ] Test error handling
- [ ] **hooks.test.ts**: Tests for React hooks
- [ ] **integration.test.ts**: End-to-end integration tests
- [ ] Manual testing in VSCode extension
- [ ] Manual testing in browser (mock mode)

### 7.6 Documentation

- [ ] API documentation (all namespaces)
- [ ] Migration guide for components
- [ ] Usage examples
- [ ] Troubleshooting guide

---

## Part 8: Validation Criteria

### 8.1 Functional Requirements

- [ ] SDK can be instantiated in test environment
- [ ] SDK can be instantiated in VSCode extension
- [ ] SDK can be instantiated in browser (mock mode)
- [ ] All namespaced APIs work correctly:
  - [ ] workflows.* methods return correct data
  - [ ] executions.* methods can start/cancel executions
  - [ ] graph.* methods return nodes/edges
  - [ ] cache.* methods manage cache entries
  - [ ] testCases.* methods run tests
  - [ ] inputs.* methods manage input sources
  - [ ] wasm.* methods access WASM runtime
  - [ ] settings.* methods manage settings

### 8.2 Event System

- [ ] Event emission works correctly
- [ ] Event subscription works correctly
- [ ] Event unsubscription prevents callbacks
- [ ] Events are added to event stream atom
- [ ] Components can subscribe to specific events

### 8.3 Integration

- [ ] SDK integrates with unified atoms (Phase 1)
- [ ] SDK works with EventListener
- [ ] SDK works with existing components
- [ ] No circular dependencies
- [ ] Type checking passes
- [ ] No TypeScript errors

### 8.4 WASM Integration

- [ ] SDK can access WASM runtime
- [ ] SDK can discover workflows from WASM
- [ ] SDK can execute tests via WASM
- [ ] SDK can trigger compilation
- [ ] SDK can get diagnostics

### 8.5 Performance

- [ ] No memory leaks (event listeners cleaned up)
- [ ] Efficient atom updates (no unnecessary re-renders)
- [ ] Execution simulation performs smoothly
- [ ] Event streaming doesn't block UI

### 8.6 Testing

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Manual testing in VSCode passes
- [ ] Manual testing in browser passes
- [ ] Test coverage > 80%

---

## Part 9: Risk Mitigation

### 9.1 High-Risk Areas

**1. Circular Dependencies**

Risk: SDK imports atoms, atoms might need SDK

Mitigation:
- Keep SDK separate from atoms
- SDK operates on store, not atom hooks
- Use factory pattern for SDK creation

**2. WASM Integration Complexity**

Risk: WASM runtime API might be unstable or poorly documented

Mitigation:
- Abstract WASM access in wasm-integration.ts
- Create fallback for missing WASM features
- Test thoroughly with real WASM

**3. EventListener Migration**

Risk: Breaking existing functionality during migration

Mitigation:
- Keep EventListener intact during migration
- Gradual migration with feature flags
- Comprehensive testing before/after changes

### 9.2 Rollback Plan

If SDK integration causes critical issues:

1. Keep Phase 1 atoms (they're safe)
2. Revert SDK integration
3. Continue using EventListener pattern
4. Re-evaluate SDK design

Rollback is safe because:
- Phase 1 atoms are independent
- EventListener stays functional
- No breaking changes to components

---

## Part 10: Success Metrics

- [ ] SDK package created and compiles
- [ ] All 37 SDK methods implemented
- [ ] All React hooks working
- [ ] WASM integration functional
- [ ] Mock mode works in browser
- [ ] VSCode mode works in extension
- [ ] All tests pass (>80% coverage)
- [ ] No regressions in existing features
- [ ] Performance benchmarks met
- [ ] Documentation complete

---

## Appendix A: Complete SDK Method Count

### Namespaces

1. **workflows**: 5 methods
2. **executions**: 7 methods
3. **graph**: 4 methods
4. **cache**: 5 methods
5. **testCases**: 4 methods
6. **inputs**: 5 methods
7. **wasm**: 4 methods
8. **settings**: 3 methods

**Total: 37 methods**

### Additional Methods

- **Lifecycle**: initialize(), dispose() (2 methods)
- **Events**: on(), off(), emit() (3 methods)
- **Private**: runExecution(), handleExecutionEvent() (2 methods)

**Total SDK methods: 44**

---

## Appendix B: File Size Estimates

| File | Lines of Code | Complexity |
|------|--------------|------------|
| types.ts | ~400 | Low (type definitions) |
| index.ts | ~800 | High (main SDK class) |
| provider.tsx | ~100 | Medium (React provider) |
| hooks.ts | ~250 | Medium (React hooks) |
| mock.ts | ~1100 | Medium (copied from baml-graph) |
| wasm-integration.ts | ~200 | High (WASM bridge) |
| event-emitter.ts | ~50 | Low (simple wrapper) |
| **Total** | ~2900 | **High** |

---

## Appendix C: Dependencies

### New Dependencies to Add

```json
{
  "dependencies": {
    "eventemitter3": "^5.0.1"
  },
  "devDependencies": {
    "@types/eventemitter3": "^2.0.2"
  }
}
```

### Internal Dependencies

- Phase 1 unified atoms (all atom files)
- VSCode API wrapper (vscode.ts)
- WASM types (@gloo-ai/baml-schema-wasm-web)

---

**Last Updated**: 2025-11-04
**Status**: Phase 2 implementation document complete - Ready for implementation
**Next Phase**: Phase 3 (Data Providers)
