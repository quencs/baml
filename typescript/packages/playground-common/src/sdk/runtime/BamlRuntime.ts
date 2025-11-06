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
    try {
      // Get all test cases from WASM project
      const allTestCases = this.wasmProject.list_testcases();

      return allTestCases
        .filter((tc: any) => {
          if (!nodeId) return true;
          // Filter by nodeId - check if this test belongs to the specified function
          return tc.parent_functions.some((pf: any) => pf.function_name === nodeId);
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
            nodeId: tc.parent_functions[0]?.function_name || '',
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
    // TODO: Implement test execution
    console.warn('[BamlRuntime] executeTest() not yet implemented');
    throw new Error('Test execution not yet implemented for BamlRuntime');
  }

  async cancelExecution(executionId: string): Promise<void> {
    // TODO: Implement execution cancellation
    console.warn('[BamlRuntime] cancelExecution() not yet implemented');
  }
}
