'use client'

import React, { createContext, useContext, useReducer, ReactNode } from 'react'
import type { WasmFunctionResponse, WasmTestResponse } from '../types'

export interface TestExecution {
  functionName: string;
  testName: string;
  status: 'queued' | 'running' | 'done' | 'error';
  response?: WasmFunctionResponse | WasmTestResponse;
  response_status?: 'passed' | 'llm_failed' | 'parse_failed' | 'constraints_failed' | 'assert_failed' | 'error';
  latency_ms?: number;
  message?: string;
  input?: any;
  timestamp: number;
}

export interface TestHistoryRun {
  timestamp: number;
  tests: TestExecution[];
}

interface TestState {
  runningTests: TestExecution[];
  history: TestHistoryRun[];
  selectedHistoryIndex: number;
  isRunning: boolean;
  selectedFunction?: string;
  selectedTestcase?: string;
  isParallelEnabled: boolean;
}

type TestAction =
  | { type: 'START_TESTS'; tests: TestExecution[] }
  | { type: 'UPDATE_TEST'; functionName: string; testName: string; update: Partial<TestExecution> }
  | { type: 'TESTS_COMPLETE' }
  | { type: 'TESTS_ERROR'; error: string }
  | { type: 'SET_SELECTED_FUNCTION'; functionName: string }
  | { type: 'SET_SELECTED_TESTCASE'; testName: string }
  | { type: 'SET_HISTORY_INDEX'; index: number }
  | { type: 'ADD_HISTORY_RUN'; run: TestHistoryRun }
  | { type: 'TOGGLE_PARALLEL'; enabled: boolean };

const initialState: TestState = {
  runningTests: [],
  history: [],
  selectedHistoryIndex: 0,
  isRunning: false,
  isParallelEnabled: false,
};

function testReducer(state: TestState, action: TestAction): TestState {
  switch (action.type) {
    case 'START_TESTS':
      return {
        ...state,
        runningTests: action.tests,
        isRunning: true,
      };
    case 'UPDATE_TEST':
      return {
        ...state,
        runningTests: state.runningTests.map(test =>
          test.functionName === action.functionName && test.testName === action.testName
            ? { ...test, ...action.update, timestamp: Date.now() }
            : test
        ),
        history: state.history.map((run, index) =>
          index === 0
            ? {
                ...run,
                tests: run.tests.map(test =>
                  test.functionName === action.functionName && test.testName === action.testName
                    ? { ...test, ...action.update, timestamp: Date.now() }
                    : test
                ),
              }
            : run
        ),
      };
    case 'TESTS_COMPLETE':
      return {
        ...state,
        isRunning: false,
      };
    case 'TESTS_ERROR':
      return {
        ...state,
        isRunning: false,
        runningTests: state.runningTests.map(test => ({
          ...test,
          status: 'error' as const,
          message: action.error,
        })),
      };
    case 'SET_SELECTED_FUNCTION':
      return {
        ...state,
        selectedFunction: action.functionName,
      };
    case 'SET_SELECTED_TESTCASE':
      return {
        ...state,
        selectedTestcase: action.testName,
      };
    case 'SET_HISTORY_INDEX':
      return {
        ...state,
        selectedHistoryIndex: action.index,
      };
    case 'ADD_HISTORY_RUN':
      return {
        ...state,
        history: [action.run, ...state.history],
        selectedHistoryIndex: 0,
      };
    case 'TOGGLE_PARALLEL':
      return {
        ...state,
        isParallelEnabled: action.enabled,
      };
    default:
      return state;
  }
}

interface TestContextType {
  state: TestState;
  dispatch: React.Dispatch<TestAction>;
}

const TestContext = createContext<TestContextType | null>(null);

export function TestProvider({ children }: { children: ReactNode }) {
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

// Convenience hooks for specific parts of the test state
export function useTestHistory() {
  const { state } = useTest();
  return {
    history: state.history,
    selectedIndex: state.selectedHistoryIndex,
    currentRun: state.history[state.selectedHistoryIndex],
  };
}

export function useTestSelection() {
  const { state, dispatch } = useTest();
  return {
    selectedFunction: state.selectedFunction,
    selectedTestcase: state.selectedTestcase,
    setSelectedFunction: (functionName: string) =>
      dispatch({ type: 'SET_SELECTED_FUNCTION', functionName }),
    setSelectedTestcase: (testName: string) =>
      dispatch({ type: 'SET_SELECTED_TESTCASE', testName }),
  };
}

export function useTestExecution() {
  const { state, dispatch } = useTest();
  return {
    isRunning: state.isRunning,
    runningTests: state.runningTests,
    startTests: (tests: TestExecution[]) => dispatch({ type: 'START_TESTS', tests }),
    updateTest: (functionName: string, testName: string, update: Partial<TestExecution>) =>
      dispatch({ type: 'UPDATE_TEST', functionName, testName, update }),
    completeTests: () => dispatch({ type: 'TESTS_COMPLETE' }),
    errorTests: (error: string) => dispatch({ type: 'TESTS_ERROR', error }),
  };
}