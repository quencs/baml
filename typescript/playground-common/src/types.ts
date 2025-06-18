// Temporary types file to avoid import issues during refactoring
// These should match the actual types from @gloo-ai/baml-schema-wasm-web

export interface WasmRuntime {
  required_env_vars(): string[];
  run_tests(testCases: any[], onPartial: any, findMediaFile: any, envVars: any): Promise<any>;
}

export interface WasmProject {
  new(path: string, files: [string, string][]): WasmProject;
  runtime(envVars: Record<string, string>): WasmRuntime;
  diagnostics(runtime: WasmRuntime): { errors(): WasmDiagnosticError[] };
  run_generators(): any[];
}

export interface WasmDiagnosticError {
  type: string;
  message: string;
}

export interface WasmFunction {
  name: string;
  run_test_with_expr_events(
    runtime: WasmRuntime,
    testName: string,
    onPartial: any,
    findMediaFile: any,
    onSpans: any,
    envVars: any
  ): Promise<any>;
}

export interface WasmTestCase {
  name: string;
  inputs: any;
}

export interface WasmFunctionResponse {
  func_test_pair(): { function_name: string; test_name: string };
  status(): any;
}

export interface WasmTestResponse {
  func_test_pair(): { function_name: string; test_name: string };
  status(): any;
}

export interface WasmSpan {
  file_path: string;
  start_line: number;
  start: number;
  end_line: number;
  end: number;
}