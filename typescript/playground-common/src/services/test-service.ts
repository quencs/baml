import type { WasmRuntime, WasmFunction, WasmTestCase, WasmFunctionResponse, WasmTestResponse } from '../types'
import type { TestExecution } from '../contexts/test-context'

export interface TestCase {
  fn: WasmFunction;
  tc: WasmTestCase;
}

export interface TestResult {
  functionName: string;
  testName: string;
  result: WasmFunctionResponse | WasmTestResponse;
  latency: number;
  status: 'passed' | 'llm_failed' | 'parse_failed' | 'constraints_failed' | 'assert_failed' | 'error';
}

export class TestService {
  static async runTest(
    runtime: WasmRuntime,
    testCase: TestCase,
    envVars: Record<string, string>,
    onPartial?: (partial: WasmFunctionResponse) => void,
    onSpans?: (spans: any[]) => void,
    findMediaFile?: (path: string) => any
  ): Promise<TestResult> {
    const startTime = performance.now();
    
    try {
      const result = await testCase.fn.run_test_with_expr_events(
        runtime,
        testCase.tc.name,
        onPartial || (() => {}),
        findMediaFile || (() => {}),
        onSpans || (() => {}),
        envVars,
      );

      const endTime = performance.now();
      const status = this.mapTestStatus(result.status());
      
      return {
        functionName: testCase.fn.name,
        testName: testCase.tc.name,
        result,
        latency: endTime - startTime,
        status,
      };
    } catch (error) {
      const endTime = performance.now();
      throw {
        functionName: testCase.fn.name,
        testName: testCase.tc.name,
        error: error instanceof Error ? error.message : 'Unknown error',
        latency: endTime - startTime,
        status: 'error' as const,
      };
    }
  }

  static async runParallelTests(
    runtime: WasmRuntime,
    testCases: TestCase[],
    envVars: Record<string, string>,
    onPartial?: (partial: WasmFunctionResponse) => void,
    findMediaFile?: (path: string) => any
  ): Promise<TestResult[]> {
    const startTime = performance.now();
    
    try {
      const testInputs = testCases.map(testCase => ({
        functionName: testCase.fn.name,
        testName: testCase.tc.name,
        inputs: testCase.tc.inputs,
      }));

      const results = await runtime.run_tests(
        testInputs,
        onPartial || (() => {}),
        findMediaFile || (() => {}),
        envVars,
      );

      const endTime = performance.now();
      const testResults: TestResult[] = [];

      // Process results from the iterator
      let response: WasmTestResponse | null | undefined;
      while ((response = results.yield_next()) != undefined) {
        const pair = response.func_test_pair();
        const status = this.mapTestStatus(response.status());
        
        testResults.push({
          functionName: pair.function_name,
          testName: pair.test_name,
          result: response,
          latency: endTime - startTime,
          status,
        });
      }

      return testResults;
    } catch (error) {
      // Return error results for all test cases
      return testCases.map(testCase => ({
        functionName: testCase.fn.name,
        testName: testCase.tc.name,
        result: null as any,
        latency: 0,
        status: 'error' as const,
        error: error instanceof Error ? error.message : 'Unknown error',
      }));
    }
  }

  static async runTestBatch(
    runtime: WasmRuntime,
    testCases: TestCase[],
    envVars: Record<string, string>,
    maxBatchSize: number = 5,
    isParallel: boolean = false,
    onTestUpdate?: (functionName: string, testName: string, update: Partial<TestExecution>) => void,
    onSpans?: (spans: any[]) => void,
    findMediaFile?: (path: string) => any
  ): Promise<TestResult[]> {
    if (isParallel) {
      return this.runParallelTests(
        runtime,
        testCases,
        envVars,
        onTestUpdate ? (partial: WasmFunctionResponse) => {
          const pair = partial.func_test_pair();
          onTestUpdate(pair.function_name, pair.test_name, { 
            status: 'running',
            response: partial 
          });
        } : undefined,
        findMediaFile
      );
    }

    // Sequential execution with batching
    const batches: TestCase[][] = [];
    for (let i = 0; i < testCases.length; i += maxBatchSize) {
      batches.push(testCases.slice(i, i + maxBatchSize));
    }

    const allResults: TestResult[] = [];

    for (const batch of batches) {
      for (const testCase of batch) {
        if (onTestUpdate) {
          onTestUpdate(testCase.fn.name, testCase.tc.name, { status: 'queued' });
        }

        try {
          if (onTestUpdate) {
            onTestUpdate(testCase.fn.name, testCase.tc.name, { status: 'running' });
          }

          const result = await this.runTest(
            runtime,
            testCase,
            envVars,
            onTestUpdate ? (partial: WasmFunctionResponse) => {
              onTestUpdate(testCase.fn.name, testCase.tc.name, {
                status: 'running',
                response: partial
              });
            } : undefined,
            onSpans,
            findMediaFile
          );

          allResults.push(result);

          if (onTestUpdate) {
            onTestUpdate(testCase.fn.name, testCase.tc.name, {
              status: 'done',
              response: result.result,
              response_status: result.status,
              latency_ms: result.latency
            });
          }
        } catch (error: any) {
          if (onTestUpdate) {
            onTestUpdate(testCase.fn.name, testCase.tc.name, {
              status: 'error',
              message: error.error || (error instanceof Error ? error.message : 'Unknown error')
            });
          }
        }
      }
    }

    return allResults;
  }

  private static mapTestStatus(status: any): 'passed' | 'llm_failed' | 'parse_failed' | 'constraints_failed' | 'assert_failed' | 'error' {
    // This would map the actual WASM status enum values
    // For now, returning a placeholder
    if (typeof status === 'string') {
      return status as any;
    }
    return 'error';
  }

  static clearHighlights(postMessage?: (message: any) => void) {
    if (postMessage) {
      try {
        postMessage({
          command: 'clearHighlights',
        });
      } catch (e) {
        console.error('Failed to clear highlights:', e);
      }
    }
  }

  static sendSpansToVSCode(spans: any[], postMessage?: (message: any) => void) {
    if (postMessage && spans.length > 0) {
      try {
        const formattedSpans = spans.map((span) => ({
          file_path: span.file_path,
          start_line: span.start_line,
          start: span.start,
          end_line: span.end_line,
          end: span.end,
        }));
        
        postMessage({
          command: 'set_flashing_regions',
          content: { spans: formattedSpans },
        });
      } catch (e) {
        console.error('Failed to send spans to VSCode:', e);
      }
    }
  }

  static sendTelemetry(numTests: number, isParallel: boolean, postMessage?: (message: any) => void) {
    if (postMessage) {
      try {
        postMessage({
          command: 'telemetry',
          meta: {
            action: 'run_tests',
            data: {
              num_tests: numTests,
              parallel: isParallel,
            },
          },
        });
      } catch (e) {
        console.error('Failed to send telemetry:', e);
      }
    }
  }
}