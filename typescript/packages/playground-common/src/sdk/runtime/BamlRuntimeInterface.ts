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

/**
 * Execution events emitted by the runtime during execution
 */
export type ExecutionEvent =
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
  | { type: 'node.progress'; nodeId: string; progress: number };

export interface FunctionDefinition {
  name: string;
  type: 'function' | 'llm_function' | 'workflow';
  filePath: string;
  parameters?: Array<{ name: string; type: string; optional: boolean }>;
  returnType?: string;
}

export interface ExecutionOptions {
  clearCache?: boolean;
  startFromNodeId?: string;
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
   */
  executeTest(testId: string): AsyncGenerator<ExecutionEvent>;

  /**
   * Cancel a running execution
   */
  cancelExecution(executionId: string): Promise<void>;
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
