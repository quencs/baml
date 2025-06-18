import type { WasmRuntime } from '@gloo-ai/baml-schema-wasm-web/baml_schema_build';
import type { TestCase, TestResult } from '../contexts/test-context';

export class TestService {
  static async runTest(
    runtime: WasmRuntime,
    testCase: TestCase,
    envVars: Record<string, string> = {}
  ): Promise<TestResult> {
    const startTime = performance.now();
    
    try {
      const fn = runtime.get_function(testCase.functionName);
      if (!fn) {
        throw new Error(`Function ${testCase.functionName} not found`);
      }

      const testSpec = fn.get_test_case(testCase.testName);
      if (!testSpec) {
        throw new Error(`Test ${testCase.testName} not found for function ${testCase.functionName}`);
      }

      const result = await testSpec.run_test_with_expr_events(
        runtime,
        envVars,
        undefined // ctx parameter
      );

      const latency = performance.now() - startTime;

      return {
        functionName: testCase.functionName,
        testName: testCase.testName,
        status: 'done',
        response: result,
        latency,
        timestamp: Date.now(),
      };
    } catch (error) {
      const latency = performance.now() - startTime;
      return {
        functionName: testCase.functionName,
        testName: testCase.testName,
        status: 'error',
        message: error instanceof Error ? error.message : String(error),
        latency,
        timestamp: Date.now(),
      };
    }
  }

  static async runParallelTests(
    runtime: WasmRuntime,
    tests: TestCase[],
    envVars: Record<string, string> = {},
    onTestComplete?: (result: TestResult) => void
  ): Promise<TestResult[]> {
    const testPromises = tests.map(async (testCase) => {
      const result = await this.runTest(runtime, testCase, envVars);
      onTestComplete?.(result);
      return result;
    });

    return Promise.all(testPromises);
  }

  static async runSequentialTests(
    runtime: WasmRuntime,
    tests: TestCase[],
    envVars: Record<string, string> = {},
    onTestComplete?: (result: TestResult) => void
  ): Promise<TestResult[]> {
    const results: TestResult[] = [];
    
    for (const testCase of tests) {
      const result = await this.runTest(runtime, testCase, envVars);
      results.push(result);
      onTestComplete?.(result);
    }
    
    return results;
  }

  static getAvailableTests(runtime: WasmRuntime): TestCase[] {
    const tests: TestCase[] = [];
    
    try {
      const functions = runtime.get_functions();
      
      for (const functionName of functions) {
        const fn = runtime.get_function(functionName);
        if (fn) {
          const testCases = fn.get_test_cases();
          for (const testName of testCases) {
            tests.push({ functionName, testName });
          }
        }
      }
    } catch (error) {
      console.error('Error getting available tests:', error);
    }
    
    return tests;
  }

  static getTestsForFunction(runtime: WasmRuntime, functionName: string): TestCase[] {
    try {
      const fn = runtime.get_function(functionName);
      if (!fn) return [];
      
      const testCases = fn.get_test_cases();
      return testCases.map(testName => ({ functionName, testName }));
    } catch (error) {
      console.error(`Error getting tests for function ${functionName}:`, error);
      return [];
    }
  }
}