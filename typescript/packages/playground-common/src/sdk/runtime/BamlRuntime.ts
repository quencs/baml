/**
 * Real BAML Runtime Implementation
 *
 * Wraps the WASM runtime and implements the BamlRuntimeInterface.
 * This is the production runtime that uses the actual BAML compiler.
 *
 * Key responsibilities:
 * - Load and initialize WASM module
 * - Create WasmProject from BAML files
 * - Extract workflows, functions, diagnostics, and generated files
 * - Execute workflows via WASM runtime
 */

import type {
  WasmProject,
  WasmRuntime,
  WasmDiagnosticError,
  WasmFunction,
  WasmTestCase,
  WasmError,
  WasmSpan,
  WasmTestResponse,
} from '@gloo-ai/baml-schema-wasm-web/baml_schema_build';
import type {
  BamlRuntimeInterface,
  ExecutionEvent,
  FunctionDefinition,
  CursorPosition,
  CursorNavigationResult,
  TestExecutionOptions,
} from './BamlRuntimeInterface';
import type {
  WorkflowDefinition,
  TestCaseInput,
  BAMLFile,
  BAMLTest,
} from '../types';
import type { DiagnosticError, GeneratedFile } from '../atoms/core.atoms';
import { vscode } from '../../shared/baml-project-panel/vscode';





// Type for the WASM module that contains all exports
type BamlWasmModule = typeof import('@gloo-ai/baml-schema-wasm-web/baml_schema_build');

// // Type for WASM diagnostic error objects
// type WasmDiagnosticErrorObject = {
//   type?: string;
//   message?: string;
//   file_path?: string;
//   line?: number;
//   column?: number;
// };

// Type for WASM generator output
type WasmGeneratorOutput = {
  output_dir: string;
  files: Array<{
    path_in_output_dir: string;
    contents: string;
  }>;
};

// Type for test execution callbacks
type WasmPartialResponse = unknown; // The partial response shape varies
type WasmNotification = { variable_name?: string; channel_name?: string; block_name?: string; is_stream: boolean; value: string };

// Type for test result from run_tests
// type WasmTestResult = {
//   func_test_pair: () => { function_name: string; test_name: string };
//   status: () => number; // TestStatus enum value
//   parse_output: () => unknown;
//   raw_output: () => string;
//   llm_output_text: () => string;
// };

// ============================================================================
// Module-Level WASM Cache
// ============================================================================

/**
 * WASM module cache - loaded once and reused across all runtime instances
 * This prevents reloading the entire WASM module on every file change
 */
let wasmModuleCache: BamlWasmModule | null = null;

/**
 * Load WASM module once and cache it
 * Subsequent calls return the cached module immediately
 */
async function getWasmModule(): Promise<BamlWasmModule> {
  if (!wasmModuleCache) {
    console.log('[BamlRuntime] Loading WASM module for the first time...');
    wasmModuleCache = await import('@gloo-ai/baml-schema-wasm-web/baml_schema_build');

    // CRITICAL: Initialize callback bridge ONCE when module is loaded
    // This enables AWS/GCP credential loading
    console.log('[BamlRuntime] Initializing WASM callback bridge');
    wasmModuleCache.init_js_callback_bridge(vscode.loadAwsCreds, vscode.loadGcpCreds);

    console.log('[BamlRuntime] WASM module loaded and cached ✓');
  }

  return wasmModuleCache;
}

/**
 * Real BAML Runtime wrapping WASM
 */
export class BamlRuntime implements BamlRuntimeInterface {
  private wasmProject: WasmProject;
  private wasmRuntime: WasmRuntime | undefined;
  private diagnostics: DiagnosticError[] = [];
  private wasm: BamlWasmModule;

  private constructor(
    wasm: BamlWasmModule,
    wasmProject: WasmProject,
    wasmRuntime: WasmRuntime | undefined,
    diagnostics: DiagnosticError[]
  ) {
    this.wasm = wasm;
    this.wasmProject = wasmProject;
    this.wasmRuntime = wasmRuntime;
    this.diagnostics = diagnostics;
  }

  /**
   * Factory method to create a new runtime instance
   *
   * @param files - BAML files (must end with .baml)
   * @param envVars - Environment variables for runtime
   * @param featureFlags - Feature flags for runtime
   */
  static async create(
    files: Record<string, string>,
    envVars: Record<string, string> = {},
    featureFlags: string[] = []
  ): Promise<{ wasm: typeof import('@gloo-ai/baml-schema-wasm-web/baml_schema_build'), runtime: BamlRuntime }> {
    console.log('[BamlRuntime] Creating runtime with', Object.keys(files).length, 'files');

    // Get cached WASM module (loads once, then reuses)
    const wasm = await getWasmModule();

    // Filter to .baml files only
    const bamlFiles = Object.entries(files).filter(([path]) => path.endsWith('.baml'));
    console.log('[BamlRuntime] Filtered to', bamlFiles.length, 'BAML files');

    // Create WasmProject (matches wasmAtom pattern)
    const wasmProject = wasm.WasmProject.new('./', bamlFiles);

    // Try to create runtime and collect diagnostics
    let wasmRuntime: WasmRuntime | undefined;
    let diagnostics: DiagnosticError[] = [];

    try {
      // Create runtime with env vars and feature flags
      wasmRuntime = wasmProject.runtime(envVars, featureFlags);

      // Get diagnostics from project
      const diags = wasmProject.diagnostics(wasmRuntime);
      if (diags) {
        diagnostics = diags.errors().map((e: WasmError, index: number) => ({
          id: `diag_${index}`,
          type: e.type as 'error' | 'warning',
          message: e.message || String(e),
          filePath: e.file_path,
          line: e.start_line,
          column: e.start_column,
        }));
      }

      console.log('[BamlRuntime] Runtime created successfully with', diagnostics.length, 'diagnostics');
    } catch (e) {
      console.error('[BamlRuntime] Error creating runtime:', e);

      // Check if it's a WasmDiagnosticError
      if (wasm.WasmDiagnosticError && e instanceof wasm.WasmDiagnosticError) {
        const wasmDiagError = e as WasmDiagnosticError;
        diagnostics = wasmDiagError.errors().map((err: WasmError, index: number) => ({
          id: `diag_${index}`,
          type: err.type as 'error' | 'warning',
          message: err.message || String(err),
          filePath: err.file_path,
          line: err.start_line,
          column: err.start_column,
        }));
        console.log('[BamlRuntime] Captured', diagnostics.length, 'diagnostics from error');
      } else {
        // Unknown error - create a generic diagnostic
        diagnostics = [{
          id: 'diag_unknown',
          type: 'error',
          message: e instanceof Error ? e.message : String(e),
        }];
      }

      // Runtime is undefined if there was an error
      wasmRuntime = undefined;
    }

    return { wasm, runtime: new BamlRuntime(wasm, wasmProject, wasmRuntime, diagnostics) };
  }

  // ============================================================================
  // BamlRuntimeInterface Implementation
  // ============================================================================

  getVersion(): string {
    return this.wasm.version();
  }

  getWasmRuntime(): WasmRuntime | undefined {
    return this.wasmRuntime;
  }

  getWorkflows(): WorkflowDefinition[] {
    // TODO: Extract workflows from WASM project
    // For now, return empty array
    // This will need to be implemented once we understand the WASM API
    console.warn('[BamlRuntime] getWorkflows() not yet implemented');
    return [];
  }

  getFunctions(): FunctionDefinition[] {
    if (!this.wasmRuntime) {
      console.log('[BamlRuntime] Cannot get functions - runtime is invalid');
      return [];
    }

    try {
      const functions: WasmFunction[] = this.wasmRuntime.list_functions();
      return functions.map((fn) => {

        return {
          type: 'llm_function' as const,
          name: fn.name,
          span: fn.span,
          test_snippet: fn.test_snippet,
          signature: fn.signature,
          test_cases: fn.test_cases,
          inner: fn
        };
      });
    } catch (e) {
      console.error('[BamlRuntime] Error getting functions:', e);
      return [];
    }
  }

  getTestCases(nodeId?: string): TestCaseInput[] {
    // Need valid runtime to get test cases
    if (!this.wasmRuntime) {
      console.log('[BamlRuntime] Cannot get test cases - runtime is invalid');
      return [];
    }

    try {
      // Get all test cases from WASM runtime
      const allTestCases: WasmTestCase[] = this.wasmRuntime.list_testcases();

      return allTestCases
        .filter((tc) => {
          if (!nodeId) return true;
          // Filter by nodeId - check if this test belongs to the specified function
          return tc.parent_functions.some((pf) => pf.name === nodeId);
        })
        .map((tc, index) => {
          // Convert WasmParam[] to Record<string, unknown>
          const inputs: Record<string, unknown> = {};
          for (const param of tc.inputs) {
            if (param.value !== undefined) {
              try {
                // Try to parse as JSON first
                inputs[param.name] = JSON.parse(param.value);
              } catch {
                // If not JSON, use as string
                inputs[param.name] = param.value;
              }
            }
          }

          return {
            id: `${tc.name}_${index}`,
            name: tc.name,
            source: 'test' as const,
            functionId: tc.parent_functions[0]?.name || '',
            filePath: tc.span.file_path,
            inputs,
            status: tc.error ? ('failing' as const) : ('unknown' as const),
          };
        });
    } catch (e) {
      console.error('[BamlRuntime] Error getting test cases:', e);
      return [];
    }
  }

  getBAMLFiles(): BAMLFile[] {
    if (!this.wasmRuntime) {
      console.log('[BamlRuntime] Cannot get BAML files - runtime is invalid');
      return [];
    }

    try {
      // Get all functions and test cases
      const functions: WasmFunction[] = this.wasmRuntime.list_functions();
      const testCases: WasmTestCase[] = this.wasmRuntime.list_testcases();

      // Group by file path
      const fileMap = new Map<string, { functions: FunctionDefinition[], tests: BAMLTest[] }>();

      // Add functions to map
      for (const fn of functions) {
        const filePath = fn.span?.file_path || 'unknown.baml';
        if (!fileMap.has(filePath)) {
          fileMap.set(filePath, { functions: [], tests: [] });
        }
        const fnWithType = fn as WasmFunction & { type?: string };
        fileMap.get(filePath)!.functions.push({
          name: fn.name,
          type: fnWithType.type as 'function' | 'llm_function' | 'workflow',
          span: fn.span,
          test_snippet: fn.test_snippet,
          signature: fn.signature,
          test_cases: fn.test_cases,
          inner: fn
        });
      }

      // Add tests to map - transform WasmTestCase to BAMLTest
      for (const tc of testCases) {
        const filePath = tc.span?.file_path || 'unknown.baml';
        if (!fileMap.has(filePath)) {
          fileMap.set(filePath, { functions: [], tests: [] });
        }
        const parentFn = tc.parent_functions[0];

        // Transform WasmTestCase to BAMLTest
        const bamlTest: BAMLTest = {
          name: tc.name,
          functionName: parentFn?.name || 'unknown',
          filePath: filePath,
          nodeType: (parentFn as any)?.type === 'llm_function' ? 'llm_function' : 'function',
        };

        fileMap.get(filePath)!.tests.push(bamlTest);
      }

      // Convert map to array of BAMLFile objects
      return Array.from(fileMap.entries()).map(([path, data]) => ({
        path,
        functions: data.functions,
        tests: data.tests,
      }));
    } catch (e) {
      console.error('[BamlRuntime] Error getting BAML files:', e);
      return [];
    }
  }

  getDiagnostics(): DiagnosticError[] {
    return this.diagnostics;
  }

  getGeneratedFiles(): GeneratedFile[] {
    // Only return generated files if runtime is valid
    if (!this.wasmRuntime) {
      console.log('[BamlRuntime] Cannot generate files - runtime is invalid');
      return [];
    }

    try {
      const generators: WasmGeneratorOutput[] = this.wasmProject.run_generators();
      const files = generators.flatMap((gen) =>
        gen.files.map((f) => ({
          path: f.path_in_output_dir,
          content: f.contents,
          outputDir: gen.output_dir,
        }))
      );

      console.log('[BamlRuntime] Generated', files.length, 'files');
      return files;
    } catch (e) {
      console.error('[BamlRuntime] Error generating files:', e);
      return [];
    }
  }

  async *executeWorkflow(
    workflowId: string,
    inputs: Record<string, any>,
    options?: { clearCache?: boolean; startFromNodeId?: string }
  ): AsyncGenerator<ExecutionEvent> {
    // TODO: Implement workflow execution
    console.warn('[BamlRuntime] executeWorkflow() not yet implemented');
    throw new Error('Workflow execution not yet implemented for BamlRuntime');
  }

  async *executeTest(
    functionName: string,
    testName: string,
    options?: TestExecutionOptions
  ): AsyncGenerator<ExecutionEvent> {
    if (!this.wasmRuntime) {
      throw new Error('Cannot execute test - runtime is invalid');
    }

    // Find the test case
    const testCases: WasmTestCase[] = this.wasmRuntime.list_testcases();
    const testCase = testCases.find((tc) => tc.name === testName);

    if (!testCase) {
      throw new Error(`Test case not found: ${testName}`);
    }

    // Get the function for this test
    const functions: WasmFunction[] = this.wasmRuntime.list_functions();
    const wasmFunction = functions.find((fn) => fn.name === functionName);

    if (!wasmFunction) {
      throw new Error(`Function not found: ${functionName}`);
    }

    const nodeId = functionName;

    try {
      // Extract inputs from test case
      const inputs: Record<string, any> = {};
      for (const param of testCase.inputs) {
        if (param.value !== undefined) {
          try {
            inputs[param.name] = JSON.parse(param.value);
          } catch {
            inputs[param.name] = param.value;
          }
        }
      }

      // Yield started event
      yield {
        type: 'node.started',
        nodeId,
        inputs,
      };

      const startTime = performance.now();

      // Create a generator-friendly way to yield events from callbacks
      const events: ExecutionEvent[] = [];
      const pushEvent = (event: ExecutionEvent) => {
        events.push(event);
      };

      // Execute the test with all callbacks yielding events
      const result = await wasmFunction.run_test_with_expr_events(
        this.wasmRuntime,
        testCase.name,
        // on_partial_response callback
        (partial: WasmPartialResponse) => {
          pushEvent({
            type: 'test.partial',
            functionName,
            testName,
            response: partial,
          });
        },
        // get_baml_src_cb - load media files
        options?.loadMediaFile || vscode.loadMediaFile,
        // on_expr_event - expression evaluation events (for highlighting)
        (spans: WasmSpan[]) => {
          if (spans && spans.length > 0) {
            pushEvent({
              type: 'test.highlight',
              spans: spans.map((span) => span),
            });
          }
        },
        // env - API keys / environment
        options?.apiKeys || {},
        // abort_signal
        options?.abortSignal || null,
        // watch_handler - for watch notifications
        (notification: WasmNotification) => {
          pushEvent({
            type: 'test.watch',
            functionName,
            testName,
            notification: {
              variable_name: notification.variable_name,
              channel_name: notification.channel_name,
              block_name: notification.block_name,
              is_stream: notification.is_stream,
              value: notification.value,
            },
          });
        }
      );

      // Yield all accumulated events from callbacks
      for (const event of events) {
        yield event;
      }

      const endTime = performance.now();
      const duration = endTime - startTime;

      // Parse the result
      const status = result.status();
      const statusMap = {
        [this.wasm.TestStatus.Passed]: 'passed',
        [this.wasm.TestStatus.LLMFailure]: 'llm_failed',
        [this.wasm.TestStatus.ParseFailure]: 'parse_failed',
        [this.wasm.TestStatus.ConstraintsFailed]: 'constraints_failed',
        [this.wasm.TestStatus.AssertFailed]: 'assert_failed',
        [this.wasm.TestStatus.UnableToRun]: 'error',
        [this.wasm.TestStatus.FinishReasonFailed]: 'error',
      } as const;

      const testStatus = statusMap[status] || 'error';

      // Extract outputs
      let outputs: Record<string, any> = {};

      if (testStatus === 'passed') {
        const parsedResponse = result.parsed_response();
        if (parsedResponse) {
          try {
            outputs = { result: JSON.parse(parsedResponse.value) };
          } catch {
            outputs = { result: parsedResponse.value };
          }
        }
      } else {
        // Get error information
        const failureMsg = result.failure_message();
        if (failureMsg) {
          outputs = { error: failureMsg };
        }
      }

      // Yield completion or error event
      if (testStatus === 'passed') {
        yield {
          type: 'node.completed',
          nodeId,
          inputs,
          outputs,
          duration,
        };
      } else {
        yield {
          type: 'node.error',
          nodeId,
          error: new Error(outputs.error || `Test failed with status: ${testStatus}`),
        };
      }
    } catch (error) {
      yield {
        type: 'node.error',
        nodeId,
        error: error instanceof Error ? error : new Error(String(error)),
      };
    }
  }

  async *executeTests(
    tests: Array<{ functionName: string; testName: string }>,
    options?: TestExecutionOptions
  ): AsyncGenerator<ExecutionEvent> {
    if (!this.wasmRuntime) {
      throw new Error('Cannot execute tests - runtime is invalid');
    }

    try {
      // Prepare test cases for run_tests
      const testCases = tests.map((test) => {
        const allTestCases: WasmTestCase[] = this.wasmRuntime!.list_testcases();
        const testCase = allTestCases.find((tc) => tc.name === test.testName);

        if (!testCase) {
          throw new Error(`Test case not found: ${test.testName}`);
        }

        // Convert inputs
        const inputs: Record<string, unknown> = {};
        for (const param of testCase.inputs) {
          if (param.value !== undefined) {
            try {
              inputs[param.name] = JSON.parse(param.value);
            } catch {
              inputs[param.name] = param.value;
            }
          }
        }

        return {
          functionName: test.functionName,
          testName: test.testName,
          inputs,
        };
      });

      // Yield started events for all tests
      for (const test of tests) {
        const testCase = testCases.find((tc) => tc.testName === test.testName);
        if (testCase) {
          yield {
            type: 'node.started',
            nodeId: test.functionName,
            inputs: testCase.inputs,
          };
        }
      }

      // Create event collectors for callbacks
      const events: ExecutionEvent[] = [];
      const pushEvent = (event: ExecutionEvent) => {
        events.push(event);
      };

      // Execute all tests in parallel via run_tests
      const results = await this.wasmRuntime.run_tests(
        testCases,
        // on_partial_response callback
        (partial: WasmPartialResponse & { func_test_pair: () => { function_name: string; test_name: string } }) => {
          const pair = partial.func_test_pair();
          pushEvent({
            type: 'test.partial',
            functionName: pair.function_name,
            testName: pair.test_name,
            response: partial,
          });
        },
        // get_baml_src_cb - load media files
        options?.loadMediaFile || vscode.loadMediaFile,
        // env - API keys / environment
        options?.apiKeys || {},
        // abort_signal
        options?.abortSignal || null,
        // watch_handler - for watch notifications
        (notification: WasmNotification & { function_name?: string; test_name?: string }) => {
          // Watch notifications should have function_name and test_name from parallel execution
          pushEvent({
            type: 'test.watch',
            functionName: notification.function_name || 'unknown',
            testName: notification.test_name || 'unknown',
            notification: {
              variable_name: notification.variable_name,
              channel_name: notification.channel_name,
              block_name: notification.block_name,
              is_stream: notification.is_stream,
              value: notification.value,
            },
          });
        }
      );

      // Yield all accumulated events from callbacks
      for (const event of events) {
        yield event;
      }

      // Process results
      let response: WasmTestResponse | undefined;
      while ((response = results.yield_next()) !== undefined) {
        const pair = response.func_test_pair();
        const status = response.status();

        const statusMap = {
          [this.wasm.TestStatus.Passed]: 'passed',
          [this.wasm.TestStatus.LLMFailure]: 'llm_failed',
          [this.wasm.TestStatus.ParseFailure]: 'parse_failed',
          [this.wasm.TestStatus.ConstraintsFailed]: 'constraints_failed',
          [this.wasm.TestStatus.AssertFailed]: 'assert_failed',
          [this.wasm.TestStatus.UnableToRun]: 'error',
          [this.wasm.TestStatus.FinishReasonFailed]: 'error',
        } as const;

        const testStatus = statusMap[status] || 'error';

        // Extract outputs
        let outputs: Record<string, any> = {};
        if (testStatus === 'passed') {
          const parsedResponse = response.parsed_response();
          if (parsedResponse) {
            try {
              outputs = { result: JSON.parse(parsedResponse.value) };
            } catch {
              outputs = { result: parsedResponse.value };
            }
          }
        } else {
          const failureMsg = response.failure_message();
          if (failureMsg) {
            outputs = { error: failureMsg };
          }
        }

        // Yield completion or error event
        if (testStatus === 'passed') {
          yield {
            type: 'node.completed',
            nodeId: pair.function_name,
            outputs,
            duration: 0, // TODO: Track duration for parallel tests
          };
        } else {
          yield {
            type: 'node.error',
            nodeId: pair.function_name,
            error: new Error(outputs.error || `Test failed with status: ${testStatus}`),
          };
        }
      }
    } catch (error) {
      // Yield error for all tests
      for (const test of tests) {
        yield {
          type: 'node.error',
          nodeId: test.functionName,
          error: error instanceof Error ? error : new Error(String(error)),
        };
      }
    }
  }

  async cancelExecution(executionId: string): Promise<void> {
    // TODO: Implement execution cancellation
    console.warn('[BamlRuntime] cancelExecution() not yet implemented');
  }

  updateCursor(
    cursor: CursorPosition,
    fileContents: Record<string, string>,
    currentSelection: string | null
  ): CursorNavigationResult {
    if (!this.wasmRuntime) {
      return { functionName: null, testCaseName: null };
    }

    const fileContent = fileContents[cursor.fileName];
    if (!fileContent) {
      return { functionName: null, testCaseName: null };
    }

    // Convert line/column to character index
    const lines = fileContent.split('\n');
    let cursorIdx = 0;
    for (let i = 0; i < cursor.line; i++) {
      cursorIdx += (lines[i]?.length ?? 0) + 1; // +1 for newline
    }
    cursorIdx += cursor.column;

    // Get function at cursor position
    const selectedFunc = this.wasmRuntime.get_function_at_position(
      cursor.fileName,
      currentSelection ?? '',
      cursorIdx
    );

    if (!selectedFunc) {
      return { functionName: null, testCaseName: null };
    }

    // Check if cursor is in a test case
    const selectedTestcase = this.wasmRuntime.get_testcase_from_position(
      selectedFunc,
      cursorIdx
    );

    if (selectedTestcase) {
      // Check for nested function in test case
      const nestedFunc = this.wasmRuntime.get_function_of_testcase(
        cursor.fileName,
        cursorIdx
      );

      return {
        functionName: nestedFunc ? nestedFunc.name : selectedFunc.name,
        testCaseName: selectedTestcase.name,
      };
    }

    // Just a function, no test case
    return {
      functionName: selectedFunc.name,
      testCaseName: null,
    };
  }
}
