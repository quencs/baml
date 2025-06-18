import { useCallback } from 'react'
import { useRuntime } from '../contexts/runtime-context'
import { useTest, useTestExecution, type TestExecution, type TestHistoryRun } from '../contexts/test-context'
import { TestService, type TestCase } from '../services/test-service'
import { useVSCode } from './use-vscode'

export interface TestInput {
  functionName: string;
  testName: string;
}

export function useTestRunner() {
  const { state: runtimeState } = useRuntime();
  const { state: testState } = useTest();
  const { startTests, updateTest, completeTests, errorTests } = useTestExecution();
  const { postMessage } = useVSCode();

  const runTests = useCallback(async (tests: TestInput[]) => {
    if (!runtimeState.runtime || !runtimeState.wasm) {
      errorTests('Runtime not ready. Please wait for WASM to load.');
      return;
    }

    if (tests.length === 0) {
      errorTests('No tests provided to run.');
      return;
    }

    // Create test executions
    const testExecutions: TestExecution[] = tests.map(test => ({
      functionName: test.functionName,
      testName: test.testName,
      status: 'queued',
      timestamp: Date.now(),
    }));

    // Create history run
    const historyRun: TestHistoryRun = {
      timestamp: Date.now(),
      tests: [...testExecutions],
    };

    // Start tests
    startTests(testExecutions);

    // Mock test cases for now - in real implementation, these would be fetched from the runtime
    const testCases: TestCase[] = tests.map(test => ({
      fn: {
        name: test.functionName,
        run_test_with_expr_events: async () => ({ status: () => 'passed' }),
      } as any,
      tc: {
        name: test.testName,
        inputs: {},
      },
    }));

    try {
      TestService.sendTelemetry(tests.length, testState.isParallelEnabled, postMessage);

      await TestService.runTestBatch(
        runtimeState.runtime,
        testCases,
        runtimeState.envVars,
        5, // maxBatchSize
        testState.isParallelEnabled,
        updateTest,
        (spans) => TestService.sendSpansToVSCode(spans, postMessage),
        () => {} // findMediaFile
      );

      completeTests();
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred';
      errorTests(errorMessage);
    } finally {
      TestService.clearHighlights(postMessage);
    }
  }, [
    runtimeState.runtime,
    runtimeState.wasm,
    runtimeState.envVars,
    testState.isParallelEnabled,
    startTests,
    updateTest,
    completeTests,
    errorTests,
    postMessage,
  ]);

  const runSingleTest = useCallback(async (functionName: string, testName: string) => {
    await runTests([{ functionName, testName }]);
  }, [runTests]);

  return {
    runTests,
    runSingleTest,
    isRunning: testState.isRunning,
  };
}

export function useTestSelection() {
  const { state, dispatch } = useTest();

  const selectFunction = useCallback((functionName: string) => {
    dispatch({ type: 'SET_SELECTED_FUNCTION', functionName });
  }, [dispatch]);

  const selectTestcase = useCallback((testName: string) => {
    dispatch({ type: 'SET_SELECTED_TESTCASE', testName });
  }, [dispatch]);

  return {
    selectedFunction: state.selectedFunction,
    selectedTestcase: state.selectedTestcase,
    selectFunction,
    selectTestcase,
  };
}

export function useTestHistory() {
  const { state, dispatch } = useTest();

  const setHistoryIndex = useCallback((index: number) => {
    dispatch({ type: 'SET_HISTORY_INDEX', index });
  }, [dispatch]);

  const currentRun = state.history[state.selectedHistoryIndex];

  return {
    history: state.history,
    selectedIndex: state.selectedHistoryIndex,
    currentRun,
    setHistoryIndex,
  };
}

export function useTestConfig() {
  const { state, dispatch } = useTest();

  const toggleParallel = useCallback((enabled: boolean) => {
    dispatch({ type: 'TOGGLE_PARALLEL', enabled });
  }, [dispatch]);

  return {
    isParallelEnabled: state.isParallelEnabled,
    toggleParallel,
  };
}