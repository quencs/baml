import { useCallback } from 'react';
import { useRuntime } from '../contexts/runtime-context';
import { type TestCase, useTest } from '../contexts/test-context';
import { TestService } from '../services/test-service';

export function useTestRunner() {
	const { state: runtimeState } = useRuntime();
	const { state: testState, dispatch } = useTest();

	const runTests = useCallback(async (tests: TestCase[]) => {
		if (!runtimeState.runtime) {
			console.error('Runtime not available');
			return;
		}

		if (testState.isRunning) {
			console.warn('Tests are already running');
			return;
		}

		dispatch({ type: 'START_TESTS', tests });

		try {
			const results = await TestService.runParallelTests(
				runtimeState.runtime,
				tests,
				{}, // TODO: pass env vars from context
				(result) => {
					dispatch({ type: 'TEST_COMPLETE', result });
				}
			);

			dispatch({ type: 'TESTS_COMPLETE', results });
			return results;
		} catch (error) {
			console.error('Error running tests:', error);
			dispatch({ type: 'TESTS_ERROR', error: String(error) });
		}
	}, [runtimeState.runtime, testState.isRunning, dispatch]);

	const runSingleTest = useCallback(async (testCase: TestCase) => {
		return runTests([testCase]);
	}, [runTests]);

	const runTestsForFunction = useCallback(async (functionName: string) => {
		if (!runtimeState.runtime) {
			console.error('Runtime not available');
			return;
		}

		const tests = TestService.getTestsForFunction(runtimeState.runtime, functionName);
		if (tests.length === 0) {
			console.warn(`No tests found for function ${functionName}`);
			return;
		}

		return runTests(tests);
	}, [runtimeState.runtime, runTests]);

	const getAvailableTests = useCallback(() => {
		if (!runtimeState.runtime) return [];
		return TestService.getAvailableTests(runtimeState.runtime);
	}, [runtimeState.runtime]);

	const getTestsForFunction = useCallback((functionName: string) => {
		if (!runtimeState.runtime) return [];
		return TestService.getTestsForFunction(runtimeState.runtime, functionName);
	}, [runtimeState.runtime]);

	return {
		runTests,
		runSingleTest,
		runTestsForFunction,
		getAvailableTests,
		getTestsForFunction,
		isRunning: testState.isRunning,
		canRun: !!runtimeState.runtime && !testState.isRunning,
	};
}