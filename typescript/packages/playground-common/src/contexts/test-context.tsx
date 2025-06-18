'use client';

import { createContext, useContext, useReducer } from 'react';

export interface TestCase {
  functionName: string;
  testName: string;
}

export interface TestResult {
  functionName: string;
  testName: string;
  status: 'running' | 'done' | 'error';
  response?: any;
  message?: string;
  latency?: number;
  timestamp: number;
}

export interface TestHistoryRun {
  id: string;
  timestamp: number;
  tests: TestResult[];
}

interface TestState {
  runningTests: TestCase[];
  history: TestHistoryRun[];
  selectedHistoryIndex: number;
  selectedFunction?: string;
  selectedTestcase?: string;
  isRunning: boolean;
  viewType: 'simple' | 'tabular' | 'card';
}

type TestAction =
  | { type: 'START_TESTS'; tests: TestCase[] }
  | { type: 'TEST_COMPLETE'; result: TestResult }
  | { type: 'TESTS_COMPLETE'; results: TestResult[] }
  | { type: 'TESTS_ERROR'; error: string }
  | { type: 'SELECT_FUNCTION'; functionName: string }
  | { type: 'SELECT_TESTCASE'; testName: string }
  | { type: 'SELECT_HISTORY'; index: number }
  | { type: 'SET_VIEW_TYPE'; viewType: TestState['viewType'] }
  | { type: 'CLEAR_HISTORY' };

const initialState: TestState = {
  runningTests: [],
  history: [],
  selectedHistoryIndex: 0,
  isRunning: false,
  viewType: 'simple',
};

function testReducer(state: TestState, action: TestAction): TestState {
  switch (action.type) {
    case 'START_TESTS':
      const newRun: TestHistoryRun = {
        id: `run-${Date.now()}`,
        timestamp: Date.now(),
        tests: action.tests.map(test => ({
          ...test,
          status: 'running' as const,
          timestamp: Date.now(),
        })),
      };
      return {
        ...state,
        runningTests: action.tests,
        isRunning: true,
        history: [newRun, ...state.history],
        selectedHistoryIndex: 0,
      };

    case 'TEST_COMPLETE':
      const updatedHistory = state.history.map((run, index) => {
        if (index === 0) {
          return {
            ...run,
            tests: run.tests.map(test =>
              test.functionName === action.result.functionName && 
              test.testName === action.result.testName
                ? action.result
                : test
            ),
          };
        }
        return run;
      });
      return { ...state, history: updatedHistory };

    case 'TESTS_COMPLETE':
      const completedHistory = state.history.map((run, index) => {
        if (index === 0) {
          return { ...run, tests: action.results };
        }
        return run;
      });
      return {
        ...state,
        runningTests: [],
        isRunning: false,
        history: completedHistory,
      };

    case 'TESTS_ERROR':
      return {
        ...state,
        runningTests: [],
        isRunning: false,
      };

    case 'SELECT_FUNCTION':
      return { ...state, selectedFunction: action.functionName };

    case 'SELECT_TESTCASE':
      return { ...state, selectedTestcase: action.testName };

    case 'SELECT_HISTORY':
      return { ...state, selectedHistoryIndex: action.index };

    case 'SET_VIEW_TYPE':
      return { ...state, viewType: action.viewType };

    case 'CLEAR_HISTORY':
      return { ...state, history: [], selectedHistoryIndex: 0 };

    default:
      return state;
  }
}

interface TestContextValue {
  state: TestState;
  dispatch: React.Dispatch<TestAction>;
}

const TestContext = createContext<TestContextValue | null>(null);

export function TestProvider({ children }: { children: React.ReactNode }) {
  const [state, dispatch] = useReducer(testReducer, initialState);

  return (
    <TestContext.Provider value={{ state, dispatch }}>
      {children}
    </TestContext.Provider>
  );
}

export function useTest() {
  const context = useContext(TestContext);
  if (!context) {
    throw new Error('useTest must be used within TestProvider');
  }
  return context;
}

// Convenience hooks for specific test state
export function useTestState() {
  const { state } = useTest();
  return state;
}

export function useTestActions() {
  const { dispatch } = useTest();
  return {
    startTests: (tests: TestCase[]) => dispatch({ type: 'START_TESTS', tests }),
    completeTest: (result: TestResult) => dispatch({ type: 'TEST_COMPLETE', result }),
    completeAllTests: (results: TestResult[]) => dispatch({ type: 'TESTS_COMPLETE', results }),
    selectFunction: (functionName: string) => dispatch({ type: 'SELECT_FUNCTION', functionName }),
    selectTestcase: (testName: string) => dispatch({ type: 'SELECT_TESTCASE', testName }),
    selectHistory: (index: number) => dispatch({ type: 'SELECT_HISTORY', index }),
    setViewType: (viewType: TestState['viewType']) => dispatch({ type: 'SET_VIEW_TYPE', viewType }),
    clearHistory: () => dispatch({ type: 'CLEAR_HISTORY' }),
  };
}

// Computed selectors
export function useCurrentTestRun(): TestHistoryRun | undefined {
  const { state } = useTest();
  return state.history[state.selectedHistoryIndex];
}

export function useTestStatus() {
  const { state } = useTest();
  const currentRun = state.history[state.selectedHistoryIndex];
  
  if (!currentRun) return { total: 0, completed: 0, running: 0, errors: 0 };
  
  const total = currentRun.tests.length;
  const completed = currentRun.tests.filter((t: TestResult) => t.status === 'done').length;
  const running = currentRun.tests.filter((t: TestResult) => t.status === 'running').length;
  const errors = currentRun.tests.filter((t: TestResult) => t.status === 'error').length;
  
  return { total, completed, running, errors };
}