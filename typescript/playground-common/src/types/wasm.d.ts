// Type declarations for @gloo-ai/baml-schema-wasm-web module
declare module '@gloo-ai/baml-schema-wasm-web' {
  export function version(): string;

  export class WasmProject {
    static new(path: string, files: [string, string][]): WasmProject;
    runtime(envVars: Record<string, string>): WasmRuntime;
    diagnostics(runtime: WasmRuntime): WasmDiagnosticError;
    run_generators(): WasmGenerator[];
  }

  export class WasmDiagnosticError {
    errors(): WasmError[];
  }

  export class WasmCallContext {
    constructor();
    node_index: number;
  }

  export interface WasmRuntime {
    list_functions(): WasmFunction[];
    required_env_vars(): string[];
    get_function_at_position(fileName: string, selectedFn: string, cursorIdx: number): WasmFunction | undefined;
    get_testcase_from_position(fn: WasmFunction, cursorIdx: number): WasmTestCase | undefined;
    get_function_of_testcase(fileName: string, cursorIdx: number): WasmFunction | undefined;
    run_tests(
      testCases: any[],
      callback: (partial: WasmFunctionResponse) => void,
      findMediaFile: (path: string) => Promise<Uint8Array>,
      envVars: Record<string, string>
    ): Promise<WasmTestResultsIterator>;
  }

  export interface WasmTestResultsIterator {
    yield_next(): WasmTestResponse | undefined;
  }

  export interface WasmError {
    type: 'error' | 'warning';
    message: string;
    text: string;
    start: number;
    end: number;
    start_ch: number;
    end_ch: number;
  }

  export interface WasmFunction {
    name: string;
    test_cases: WasmTestCase[];
    test_snippet: string;
    span?: WasmSpan;
    orchestration_graph(runtime: WasmRuntime): any;
    render_prompt_for_test(
      runtime: WasmRuntime,
      testName: string,
      ctx: WasmCallContext,
      findMediaFile: (path: string) => Promise<Uint8Array>,
      envVars: Record<string, string>
    ): Promise<WasmPrompt>;
    render_raw_curl_for_test(
      runtime: WasmRuntime,
      testName: string,
      ctx: WasmCallContext,
      flag1: boolean,
      flag2: boolean,
      findMediaFile: (path: string) => Promise<Uint8Array>,
      envVars: Record<string, string>
    ): Promise<string>;
    run_test_with_expr_events(
      runtime: WasmRuntime,
      testName: string,
      callback: (partial: WasmFunctionResponse) => void,
      findMediaFile: (path: string) => Promise<Uint8Array>,
      spanCallback: (spans: WasmSpan[]) => void,
      envVars: Record<string, string>
    ): Promise<WasmTestResponse>;
  }

  export interface WasmTestCase {
    name: string;
    inputs: WasmParam[];
    span?: WasmSpan;
  }

  export interface WasmParam {
    value: any;
  }

  export interface WasmPrompt {
    as_chat(): WasmChatMessage[] | null;
  }

  export interface WasmChatMessage {
    role: string;
    parts: WasmChatMessagePart[];
  }

  export interface WasmChatMessagePart {
    is_text(): boolean;
    is_image(): boolean;
    is_audio(): boolean;
    as_text(): string | null;
    as_media(): WasmChatMessagePartMedia | null;
  }

  export interface WasmChatMessagePartMedia {
    url: string;
    content_type: string;
    type: WasmChatMessagePartMediaType;
    content: string;
  }

  export enum WasmChatMessagePartMediaType {
    File = 'File',
    Url = 'Url',
    Error = 'Error',
  }

  export interface WasmGenerator {
    output_dir: string;
    files: WasmGeneratedFile[];
  }

  export interface WasmGeneratedFile {
    path_in_output_dir: string;
    contents: string;
  }

  export interface WasmFunctionResponse {
    llm_failure(): WasmLLMFailure | null;
    llm_response(): WasmLLMResponse | null;
    parsed_response(): WasmParsedResponse | null;
    failure_message(): string;
    func_test_pair(): WasmFuncTestPair;
    status(): TestStatus;
  }

  export interface WasmTestResponse {
    llm_failure(): WasmLLMFailure | null;
    llm_response(): WasmLLMResponse | null;
    parsed_response(): WasmParsedResponse | null;
    failure_message(): string;
    func_test_pair(): WasmFuncTestPair;
    status(): TestStatus;
    yield_next?(): WasmTestResponse | undefined;
  }

  export interface WasmParsedResponse {
    value: string;
    explanation?: string;
    check_count: number;
  }

  export interface WasmFuncTestPair {
    function_name: string;
    test_name: string;
  }

  export interface WasmLLMFailure {
    message: string;
    code: string;
  }

  export interface WasmLLMResponse {
    content: string;
    model: string;
    latency_ms: number;
    input_tokens?: number;
    output_tokens?: number;
  }

  export interface WasmSpan {
    name: string;
    start_time: number;
    end_time: number;
    file_path: string;
    start_line: number;
    start: number;
    end_line: number;
    end: number;
  }

  export enum TestStatus {
    Passed = 0,
    LLMFailure = 1,
    ParseFailure = 2,
    ConstraintsFailed = 3,
    AssertFailed = 4,
    UnableToRun = 5,
    FinishReasonFailed = 6,
  }

  export function lint(input: string): string;
  export function init_js_callback_bridge(
    loadAwsCreds: (profile: string | null) => Promise<any>, 
    loadGcpCreds: () => any
  ): void;
}

declare module '@gloo-ai/baml-schema-wasm-web/baml_schema_build' {
  export * from '@gloo-ai/baml-schema-wasm-web';
}