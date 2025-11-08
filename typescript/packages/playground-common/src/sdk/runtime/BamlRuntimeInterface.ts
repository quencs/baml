/**
 * BAML Runtime Interface
 *
 * Responsible for:
 * - Loading and parsing BAML files (pushed from external sources)
 * - Discovering workflows, functions, and tests
 * - Executing workflows and tests (stateless)
 *
 * The runtime is PASSIVE and IMMUTABLE:
 * - Doesn't watch files - files are pushed to it
 * - Once created, runtime is immutable
 * - On file changes, create a new runtime instance (like wasmAtom pattern)
 */

import type {
  WorkflowDefinition,
  TestCaseInput,
  BAMLFile,
  LogEntry,
} from '../types';

import type { DiagnosticError, GeneratedFile } from '../atoms/core.atoms';
import { WasmFunction, WasmRuntime, WasmSpan, WasmTestCase } from '@gloo-ai/baml-schema-wasm-web';

/**
 * Watch notification from test execution
 */
export interface WatchNotification {
  variable_name?: string;
  channel_name?: string;
  block_name?: string;
  is_stream: boolean;
  value: string;
}

// /**
//  * Code span for highlighting during execution
//  */
// export interface CodeSpan {
//   file_path: string;
//   start_line: number;
//   start: number;
//   end_line: number;
//   end: number;
// }

/**
 * Execution events emitted by the runtime during execution
 */
export type ExecutionEvent =
  // Workflow/Node events
  | { type: 'node.started'; nodeId: string; inputs: Record<string, any> }
  | {
    type: 'node.completed';
    nodeId: string;
    inputs?: Record<string, any>;
    outputs: Record<string, any>;
    duration: number;
  }
  | { type: 'node.error'; nodeId: string; error: Error }
  | { type: 'node.log'; nodeId: string; log: LogEntry }
  | { type: 'node.cached'; nodeId: string; fromExecutionId: string }
  | { type: 'node.progress'; nodeId: string; progress: number }
  // Test execution events
  | { type: 'test.partial'; functionName: string; testName: string; response: any }
  | { type: 'test.watch'; functionName: string; testName: string; notification: WatchNotification }
  | { type: 'test.highlight'; spans: WasmSpan[] };

export interface FunctionDefinition {
  type: 'function' | 'llm_function' | 'workflow';
  name: string;
  span: WasmSpan;
  test_snippet: string;
  signature: string;
  test_cases: WasmTestCase[];
  inner: WasmFunction;
}

/**
 * Cursor position in a file
 */
export interface CursorPosition {
  fileName: string;
  line: number;
  column: number;
}

/**
 * Result of cursor navigation - which function/test is at the cursor
 */
export interface CursorNavigationResult {
  functionName: string | null;
  testCaseName: string | null;
}

export interface ExecutionOptions {
  clearCache?: boolean;
  startFromNodeId?: string;
}

/**
 * Options for test execution
 */
export interface TestExecutionOptions {
  apiKeys?: Record<string, string>;
  abortSignal?: AbortSignal;
  loadMediaFile?: (path: string) => Promise<string>;
}

/**
 * BAML Runtime Interface
 */
export interface BamlRuntimeInterface {
  /**
   * Get BAML runtime version
   */
  getVersion(): string;

  /**
   * Get the WASM runtime instance (for legacy compatibility with wasmAtom)
   * This returns the WasmRuntime, not the WASM module
   * Returns undefined for mock runtimes or when runtime has errors
   */
  getWasmRuntime(): WasmRuntime | undefined;

  /**
   * Get all discovered workflows
   */
  getWorkflows(): WorkflowDefinition[];

  /**
   * Get all discovered functions (including LLM functions)
   */
  getFunctions(): FunctionDefinition[];

  /**
   * Get all discovered test cases
   */
  getTestCases(nodeId?: string): TestCaseInput[];

  /**
   * Get BAML file structure (for debug panel)
   */
  getBAMLFiles(): BAMLFile[];

  /**
   * Get compilation diagnostics (errors and warnings)
   * Returns empty array if compilation was successful
   */
  getDiagnostics(): DiagnosticError[];

  /**
   * Get generated code files from BAML runtime
   * Returns empty array if no generators are configured or runtime is invalid
   */
  getGeneratedFiles(): GeneratedFile[];

  /**
   * Execute a workflow (stateless - just yields events)
   * Does NOT track state - that's the SDK's job
   */
  executeWorkflow(
    workflowId: string,
    inputs: Record<string, any>,
    options?: ExecutionOptions
  ): AsyncGenerator<ExecutionEvent>;

  /**
   * Execute a single test
   *
   * @param functionName - The function to test
   * @param testName - The test case name
   * @param options - Test execution options (apiKeys, abortSignal, loadMediaFile)
   */
  executeTest(
    functionName: string,
    testName: string,
    options?: TestExecutionOptions
  ): AsyncGenerator<ExecutionEvent>;

  /**
   * Execute multiple tests (potentially in parallel)
   *
   * @param tests - Array of tests to execute
   * @param options - Test execution options (apiKeys, abortSignal, loadMediaFile)
   */
  executeTests(
    tests: Array<{ functionName: string; testName: string }>,
    options?: TestExecutionOptions
  ): AsyncGenerator<ExecutionEvent>;

  /**
   * Cancel a running execution
   */
  cancelExecution(executionId: string): Promise<void>;

  /**
   * Update cursor position and determine which function/test is at that position
   *
   * @param cursor - Cursor position (fileName, line, column)
   * @param fileContents - Map of file paths to their contents
   * @param currentSelection - Currently selected function name (for context)
   * @returns The function and test case at the cursor position
   */
  updateCursor(
    cursor: CursorPosition,
    fileContents: Record<string, string>,
    currentSelection: string | null
  ): CursorNavigationResult;
}

/**
 * Factory type for creating runtime instances
 * Accepts files, environment variables, and feature flags
 */
export type BamlRuntimeFactory = (
  files: Record<string, string>,
  envVars?: Record<string, string>,
  featureFlags?: string[]
) => Promise<BamlRuntimeInterface>;
