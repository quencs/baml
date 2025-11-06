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
} from '@gloo-ai/baml-schema-wasm-web/baml_schema_build';

import type { BamlRuntimeInterface, ExecutionEvent } from './BamlRuntimeInterface';
import type {
  WorkflowDefinition,
  TestCaseInput,
  BAMLFile,
} from '../types';
import type { DiagnosticError, GeneratedFile } from '../atoms/core.atoms';

import { vscode } from '../../shared/baml-project-panel/vscode';

/**
 * Real BAML Runtime wrapping WASM
 */
export class BamlRuntime implements BamlRuntimeInterface {
  private wasmProject: WasmProject;
  private wasmRuntime: WasmRuntime | undefined;
  private diagnostics: DiagnosticError[] = [];
  private wasm: any;

  private constructor(
    wasm: any,
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
  ): Promise<BamlRuntime> {
    console.log('[BamlRuntime] Creating runtime with', Object.keys(files).length, 'files');

    // Load WASM module
    const wasm = await import('@gloo-ai/baml-schema-wasm-web/baml_schema_build');

    // CRITICAL: Initialize callback bridge BEFORE creating WasmProject
    // This enables AWS/GCP credential loading
    console.log('[BamlRuntime] Initializing WASM callback bridge');
    wasm.init_js_callback_bridge(vscode.loadAwsCreds, vscode.loadGcpCreds);

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
        diagnostics = diags.errors().map((e: any, index: number) => ({
          id: `diag_${index}`,
          type: e.type || 'error',
          message: e.message || String(e),
          filePath: e.file_path,
          line: e.line,
          column: e.column,
        }));
      }

      console.log('[BamlRuntime] Runtime created successfully with', diagnostics.length, 'diagnostics');
    } catch (e) {
      console.error('[BamlRuntime] Error creating runtime:', e);

      // Check if it's a WasmDiagnosticError
      if (wasm.WasmDiagnosticError && e instanceof wasm.WasmDiagnosticError) {
        const wasmDiagError = e as WasmDiagnosticError;
        diagnostics = wasmDiagError.errors().map((err: any, index: number) => ({
          id: `diag_${index}`,
          type: err.type || 'error',
          message: err.message || String(err),
          filePath: err.file_path,
          line: err.line,
          column: err.column,
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

    return new BamlRuntime(wasm, wasmProject, wasmRuntime, diagnostics);
  }

  // ============================================================================
  // BamlRuntimeInterface Implementation
  // ============================================================================

  getVersion(): string {
    return this.wasm.version();
  }

  getWorkflows(): WorkflowDefinition[] {
    // TODO: Extract workflows from WASM project
    // For now, return empty array
    // This will need to be implemented once we understand the WASM API
    console.warn('[BamlRuntime] getWorkflows() not yet implemented');
    return [];
  }

  getFunctions() {
    // TODO: Extract functions from WASM project
    console.warn('[BamlRuntime] getFunctions() not yet implemented');
    return [];
  }

  getTestCases(nodeId?: string): TestCaseInput[] {
    // Need valid runtime to get test cases
    if (!this.wasmRuntime) {
      console.log('[BamlRuntime] Cannot get test cases - runtime is invalid');
      return [];
    }

    try {
      // Get all test cases from WASM runtime
      const allTestCases = this.wasmRuntime.list_testcases();

      return allTestCases
        .filter((tc: any) => {
          if (!nodeId) return true;
          // Filter by nodeId - check if this test belongs to the specified function
          return tc.parent_functions.some((pf: any) => pf.name === nodeId);
        })
        .map((tc: any, index: number) => {
          // Convert WasmParam[] to Record<string, any>
          const inputs: Record<string, any> = {};
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
            nodeId: tc.parent_functions[0]?.name || '',
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
    // TODO: Extract BAML file structure from WASM project
    console.warn('[BamlRuntime] getBAMLFiles() not yet implemented');
    return [];
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
      const generators = this.wasmProject.run_generators();
      const files = generators.flatMap((gen: any) =>
        gen.files.map((f: any) => ({
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

  async *executeTest(testId: string): AsyncGenerator<ExecutionEvent> {
    if (!this.wasmRuntime) {
      throw new Error('Cannot execute test - runtime is invalid');
    }

    // Parse testId to get function name and test name
    // Format: "functionName:testName" or just use test name and search
    const testCases = this.wasmRuntime.list_testcases();
    const testCase = testCases.find((tc: any) => tc.name === testId);

    if (!testCase) {
      throw new Error(`Test case not found: ${testId}`);
    }

    // Get the function for this test
    const functions = this.wasmRuntime.list_functions();
    const functionName = testCase.parent_functions[0]?.name;
    const wasmFunction = functions.find((fn: any) => fn.name === functionName);

    if (!wasmFunction) {
      throw new Error(`Function not found for test: ${functionName}`);
    }

    const nodeId = functionName;

    if (!nodeId) {
      throw new Error(`Node ID not found for test: ${testId}`);
    }

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
      let lastPartialResponse: any = null;

      // Execute the test
      const result = await wasmFunction.run_test_with_expr_events(
        this.wasmRuntime,
        testCase.name,
        // on_partial_response callback
        (partial: any) => {
          lastPartialResponse = partial;
          // Could yield progress events here if needed
        },
        // get_baml_src_cb - load media files
        vscode.loadMediaFile,
        // on_expr_event - expression evaluation events (for highlighting)
        (_spans: any) => {
          // Could yield log events for expression evaluation
        },
        // env - API keys / environment
        {},
        // abort_signal
        null,
        // watch_handler - for watch notifications
        (_notification: any) => {
          // Could yield log events for watch notifications
        }
      );

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

  async cancelExecution(executionId: string): Promise<void> {
    // TODO: Implement execution cancellation
    console.warn('[BamlRuntime] cancelExecution() not yet implemented');
  }
}
