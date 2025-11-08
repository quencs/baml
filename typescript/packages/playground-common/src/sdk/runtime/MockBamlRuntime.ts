/**
 * MockBamlRuntime - In-memory mock implementation
 *
 * Uses centralized mock configuration (static, doesn't change with files)
 */

import type { WorkflowDefinition, TestCaseInput, BAMLFile } from '../types';
import type {
  BamlRuntimeInterface,
  ExecutionEvent,
  ExecutionOptions,
  FunctionDefinition,
  CursorPosition,
  CursorNavigationResult,
} from './BamlRuntimeInterface';
import type { MockRuntimeConfig } from '../mock-config/types';
import type { DiagnosticError, GeneratedFile } from '../atoms/core.atoms';
import { simulateExecution } from './simulator';

export class MockBamlRuntime implements BamlRuntimeInterface {
  private config: MockRuntimeConfig;
  private executionCount = 0;

  private constructor(config: MockRuntimeConfig) {
    this.config = config;
  }

  /**
   * Factory method to create new runtime instance
   * Matches wasmAtom pattern: WasmProject.new('./', bamlFiles)
   */
  static async create(
    files: Record<string, string>,
    config: MockRuntimeConfig
  ): Promise<MockBamlRuntime> {
    console.log(
      'Creating new MockBamlRuntime with',
      Object.keys(files).length,
      'files'
    );
    // Mock: We ignore files and use static config
    // In a real runtime, this would parse/compile the files
    return new MockBamlRuntime(config);
  }

  getVersion(): string {
    return '0.0.0-mock';
  }

  getWasmRuntime(): undefined {
    return undefined;
  }

  getWorkflows(): WorkflowDefinition[] {
    return this.config.workflows;
  }

  getFunctions(): FunctionDefinition[] {
    return this.config.functions;
  }

  getTestCases(nodeId?: string): TestCaseInput[] {
    if (!nodeId) {
      return Object.values(this.config.testCases).flat();
    }
    return this.config.testCases[nodeId] || [];
  }

  getBAMLFiles(): BAMLFile[] {
    return this.config.bamlFiles;
  }

  getDiagnostics(): DiagnosticError[] {
    // Mock runtime has no diagnostics (always valid)
    return [];
  }

  getGeneratedFiles(): GeneratedFile[] {
    // Mock runtime doesn't generate files
    return [];
  }

  async *executeWorkflow(
    workflowId: string,
    inputs: Record<string, any>,
    options?: ExecutionOptions
  ): AsyncGenerator<ExecutionEvent> {
    const workflow = this.config.workflows.find((w) => w.id === workflowId);
    if (!workflow) {
      throw new Error(`Workflow ${workflowId} not found`);
    }

    this.executionCount++;
    const executionId = `exec_${Date.now()}_${this.executionCount}`;

    // Use centralized execution simulator
    yield* simulateExecution(
      workflow,
      this.config,
      inputs,
      executionId,
      options?.startFromNodeId
    );
  }

  async *executeTest(
    functionName: string,
    testName: string,
    options?: import('./BamlRuntimeInterface').TestExecutionOptions
  ): AsyncGenerator<ExecutionEvent> {
    // Find test and execute
    const test = this.getTestCases(functionName).find((t) => t.name === testName);
    if (!test) throw new Error(`Test ${testName} not found for function ${functionName}`);

    // Execute with test inputs
    yield* this.executeWorkflow(test.functionId, test.inputs);
  }

  async *executeTests(
    tests: Array<{ functionName: string; testName: string }>,
    options?: import('./BamlRuntimeInterface').TestExecutionOptions
  ): AsyncGenerator<ExecutionEvent> {
    // Mock implementation: run tests sequentially
    for (const test of tests) {
      yield* this.executeTest(test.functionName, test.testName, options);
    }
  }

  async cancelExecution(executionId: string): Promise<void> {
    console.log(`Cancelling execution: ${executionId}`);
  }

  updateCursor(
    cursor: CursorPosition,
    fileContents: Record<string, string>,
    currentSelection: string | null
  ): CursorNavigationResult {
    // Mock implementation - could be enhanced to parse the file and find functions/tests
    // For now, just return null (no navigation)
    return { functionName: null, testCaseName: null };
  }
}
