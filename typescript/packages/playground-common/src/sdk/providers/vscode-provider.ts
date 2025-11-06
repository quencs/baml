/**
 * VSCode Data Provider
 *
 * Wraps playground-common WASM runtime for SDK access
 * Integrates with existing EventListener and atoms
 */

import type { createStore } from 'jotai';
import type { DataProvider, TestExecutionEvent, Diagnostic } from './base';
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
import {
  wasmAtom,
  runtimeAtom,
  diagnosticsAtom,
  filesAtom,
  ctxAtom,
  testCaseAtom,
} from '../../shared/atoms';
import { vscode } from '../../shared/baml-project-panel/vscode';
import { apiKeysAtom } from '../../components/api-keys-dialog/atoms';
import type { WasmFunctionResponse, WasmSpan } from '@gloo-ai/baml-schema-wasm-web';
import { AsyncIterableQueue } from '../utils/async-queue';
import type { CodeSpan, WatchNotification } from './base';

/**
 * VSCode Data Provider Implementation
 *
 * Wraps existing WASM runtime and VSCode API
 * Does NOT duplicate functionality - delegates to existing systems
 */
export class VSCodeDataProvider implements DataProvider {
  private store: ReturnType<typeof createStore>;
  private abortController: AbortController | null = null;

  constructor(store: ReturnType<typeof createStore>) {
    this.store = store;
    console.log('[VSCodeProvider] Created');
  }

  // ============================================================================
  // WORKFLOW DATA
  // ============================================================================

  async getWorkflows(): Promise<WorkflowDefinition[]> {
    const runtimeData = this.store.get(runtimeAtom);
    const runtime = runtimeData?.rt;
    if (!runtime) {
      console.warn('[VSCodeProvider] Runtime not available');
      return [];
    }

    try {
      // Get functions from runtime
      // TODO: Implement getFunctions in WASM runtime
      const functions = (runtime as any).getFunctions?.() ?? [];

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
    const runtimeData = this.store.get(runtimeAtom);
    const runtime = runtimeData?.rt;
    if (!runtime) return [];

    try {
      // Get files from runtime
      // TODO: Implement getFiles in WASM runtime
      const files = (runtime as any).getFiles?.() ?? [];
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
    const runtimeData = this.store.get(runtimeAtom);
    const runtime = runtimeData?.rt;
    if (!runtime) return [];

    try {
      // Get tests from runtime
      // TODO: Implement getTests in WASM runtime
      const tests = (runtime as any).getTests?.(functionName) ?? [];
      return tests.map((test: any) => this.convertToTestCase(test, functionName));
    } catch (error) {
      console.error('[VSCodeProvider] Failed to get test cases', error);
      return [];
    }
  }

  async *runTest(
    functionName: string,
    testName: string,
    options?: { abortSignal?: AbortSignal }
  ): AsyncGenerator<TestExecutionEvent> {
    const runtimeData = this.store.get(runtimeAtom);
    const runtime = runtimeData?.rt;
    const wasm = this.store.get(wasmAtom);
    const ctx = this.store.get(ctxAtom);
    const apiKeys = this.store.get(apiKeysAtom);

    if (!runtime || !wasm || !ctx) {
      yield {
        type: 'test.error',
        functionName,
        testName,
        error: 'WASM runtime not available. Try reloading the playground.',
      };
      return;
    }

    // Get test case from atom
    const testCase = this.store.get(testCaseAtom({ functionName, testName }));
    if (!testCase) {
      yield {
        type: 'test.error',
        functionName,
        testName,
        error: `Test case ${testName} not found for function ${functionName}`,
      };
      return;
    }

    // Create event queue for streaming
    const eventQueue = new AsyncIterableQueue<TestExecutionEvent>();

    // Start event
    eventQueue.push({ type: 'test.started', functionName, testName, timestamp: Date.now() });

    const startTime = performance.now();

    // Execute test and stream events concurrently
    const executeTest = async () => {
      try {
        // Call WASM run_test_with_expr_events (matches test-runner.ts:144)
        const result = await testCase.fn.run_test_with_expr_events(
          runtime,
          testCase.tc.name,
          // Partial response callback
          (partial: WasmFunctionResponse) => {
            eventQueue.push({
              type: 'test.partial',
              functionName,
              testName,
              partialResponse: partial,
            });
          },
          // Media loader
          vscode.loadMediaFile,
          // Span callback for highlighting
          (spans: WasmSpan[]) => {
            const codeSpans: CodeSpan[] = spans.map((span) => ({
              file_path: span.file_path,
              start_line: span.start_line,
              start: span.start,
              end_line: span.end_line,
              end: span.end,
            }));
            eventQueue.push({
              type: 'test.span',
              functionName,
              testName,
              spans: codeSpans,
            });
          },
          // API keys
          apiKeys,
          // Abort signal
          options?.abortSignal,
          // Watch notification callback
          (notification: any) => {
            const watchNotification: WatchNotification = {
              variable_name: notification.variable_name,
              channel_name: notification.channel_name,
              block_name: notification.block_name,
              function_name: notification.function_name,
              test_name: notification.test_name,
              is_stream: notification.is_stream,
              value: notification.value,
            };
            eventQueue.push({
              type: 'test.watch',
              functionName,
              testName,
              notification: watchNotification,
            });
          }
        );

        const endTime = performance.now();
        const responseStatus = result.status();

        const responseStatusMap = {
          [wasm.TestStatus.Passed]: 'passed',
          [wasm.TestStatus.LLMFailure]: 'llm_failed',
          [wasm.TestStatus.ParseFailure]: 'parse_failed',
          [wasm.TestStatus.ConstraintsFailed]: 'constraints_failed',
          [wasm.TestStatus.AssertFailed]: 'assert_failed',
          [wasm.TestStatus.UnableToRun]: 'error',
          [wasm.TestStatus.FinishReasonFailed]: 'error',
        } as const;

        eventQueue.push({
          type: 'test.completed',
          functionName,
          testName,
          duration: endTime - startTime,
          response: result,
          status: responseStatusMap[responseStatus] || 'error',
        });
        eventQueue.complete();
      } catch (error) {
        // Check if it's an abort error
        if (error instanceof Error && (error.name === 'AbortError' || error.message?.includes('BamlAbortError'))) {
          eventQueue.push({
            type: 'test.cancelled',
            functionName,
            testName,
          });
        } else {
          eventQueue.push({
            type: 'test.error',
            functionName,
            testName,
            error: error instanceof Error ? error.message : String(error),
          });
        }
        eventQueue.complete();
      }
    };

    // Start test execution (don't await)
    executeTest();

    // Yield events as they arrive
    for await (const event of eventQueue) {
      yield event;
    }
  }

  async *runTests(
    tests: Array<{ functionName: string; testName: string }>,
    options?: { parallel?: boolean; abortSignal?: AbortSignal }
  ): AsyncGenerator<TestExecutionEvent> {
    if (options?.parallel) {
      // Parallel execution using AsyncIterableQueue
      const eventQueue = new AsyncIterableQueue<TestExecutionEvent>();
      let completedCount = 0;

      // Start all tests concurrently
      const promises = tests.map(async (test) => {
        try {
          for await (const event of this.runTest(test.functionName, test.testName, {
            abortSignal: options?.abortSignal,
          })) {
            eventQueue.push(event);
          }
        } catch (error) {
          eventQueue.push({
            type: 'test.error',
            functionName: test.functionName,
            testName: test.testName,
            error: error instanceof Error ? error.message : String(error),
          });
        } finally {
          completedCount++;
          if (completedCount === tests.length) {
            eventQueue.complete();
          }
        }
      });

      // Yield events as they arrive
      for await (const event of eventQueue) {
        yield event;
      }

      // Ensure all promises are awaited
      await Promise.allSettled(promises);
    } else {
      // Sequential execution
      for (const test of tests) {
        yield* this.runTest(test.functionName, test.testName, {
          abortSignal: options?.abortSignal,
        });
      }
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
    // jumpToFile expects WasmSpan but only uses these fields internally
    await vscode.jumpToFile({
      file_path: position.filePath,
      start_line: position.line,
      start_column: position.column,
      end_line: position.line,
      end_column: position.column,
    } as any);
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
   * Simulate workflow execution (for compatibility)
   * In VSCode mode, this just delegates to executeWorkflow
   */
  async *simulateExecution(
    workflowId: string,
    inputs: Record<string, any>,
    startFromNodeId?: string
  ): AsyncGenerator<BAMLEvent> {
    yield* this.executeWorkflow(workflowId, inputs);
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
export function createVSCodeProvider(store: ReturnType<typeof createStore>): DataProvider {
  return new VSCodeDataProvider(store);
}
