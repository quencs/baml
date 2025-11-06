/**
 * Data Provider Interface
 *
 * Abstracts data sources for the SDK:
 * - MockDataProvider: Hardcoded data for browser/testing
 * - VSCodeDataProvider: WASM runtime integration
 * - ServerDataProvider: Future remote API (not implemented yet)
 *
 * All methods are async to support various backends.
 * Source: Design Doc Phase 3
 */

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
 * Watch notification from test execution
 */
export interface WatchNotification {
  variable_name?: string;
  channel_name?: string;
  block_name?: string;
  function_name?: string;
  test_name?: string;
  is_stream: boolean;
  value: string;
}

/**
 * Code span for highlighting
 */
export interface CodeSpan {
  file_path: string;
  start_line: number;
  start: number;
  end_line: number;
  end: number;
}

/**
 * Test execution event - rich events for streaming test execution
 */
export type TestExecutionEvent =
  | { type: 'test.started'; functionName: string; testName: string; timestamp: number }
  | { type: 'test.partial'; functionName: string; testName: string; partialResponse: any }
  | { type: 'test.watch'; functionName: string; testName: string; notification: WatchNotification }
  | { type: 'test.span'; functionName: string; testName: string; spans: CodeSpan[] }
  | { type: 'test.completed'; functionName: string; testName: string; duration: number; response: any; status: string }
  | { type: 'test.error'; functionName: string; testName: string; error: Error | string }
  | { type: 'test.cancelled'; functionName: string; testName: string };

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
 * Data Provider Interface
 *
 * Abstracts all external data sources the SDK needs
 */
export interface DataProvider {
  // Workflow Data
  getWorkflows(): Promise<WorkflowDefinition[]>;
  getWorkflow(workflowId: string): Promise<WorkflowDefinition | null>;

  // File System & Code
  getBAMLFiles(): Promise<BAMLFile[]>;
  getFileContent(filePath: string): Promise<string>;
  watchFiles(callback: (files: Record<string, string>) => void): () => void;

  // Execution
  getExecutions(workflowId: string): Promise<ExecutionSnapshot[]>;
  executeWorkflow(
    workflowId: string,
    inputs: Record<string, any>,
    options?: ExecutionOptions
  ): AsyncGenerator<BAMLEvent>;
  cancelExecution(executionId: string): Promise<void>;

  // Simulation (for MockDataProvider compatibility)
  simulateExecution(
    workflowId: string,
    inputs: Record<string, any>,
    startFromNodeId?: string
  ): AsyncGenerator<BAMLEvent>;

  // Test Execution
  getTestCases(functionName: string): Promise<TestCaseInput[]>;
  runTest(
    functionName: string,
    testName: string,
    options?: { abortSignal?: AbortSignal }
  ): AsyncGenerator<TestExecutionEvent>;
  runTests(
    tests: Array<{ functionName: string; testName: string }>,
    options?: { parallel?: boolean; abortSignal?: AbortSignal }
  ): AsyncGenerator<TestExecutionEvent>;
  cancelTests(): Promise<void>;

  // Graph & Structure
  getGraph(workflowId: string): Promise<{
    nodes: GraphNode[];
    edges: GraphEdge[];
  }>;
  getFunctions(): Promise<BAMLFunction[]>;

  // Cache Management
  getCacheEntries(nodeId: string): Promise<CacheEntry[]>;
  saveCacheEntry(entry: CacheEntry): Promise<void>;
  clearCache(scope: 'all' | 'workflow' | 'node', id?: string): Promise<void>;

  // Navigation & Code Sync
  navigateToCode(position: CodePosition): Promise<void>;
  highlightCode(ranges: CodePosition[]): Promise<void>;

  // Settings & Configuration
  getSettings(): Promise<Record<string, any>>;
  updateSetting(key: string, value: any): Promise<void>;

  // Runtime & Compilation
  getRuntimeVersion(): Promise<string>;
  getDiagnostics(): Promise<Diagnostic[]>;
  compile(): Promise<void>;

  // Lifecycle
  initialize(): Promise<void>;
  dispose(): Promise<void>;
}
