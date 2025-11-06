/**
 * Test Runner (SDK-based)
 *
 * Refactored to use BAML SDK instead of direct WASM calls.
 * SDK handles execution, this hook manages UI state (testHistoryAtom).
 */

import { useCallback } from 'react';
import { useSetAtom, useAtomValue } from 'jotai';
import type { TestExecutionEvent } from '../../../../../sdk';
import { useBAMLSDK } from '../../../../../sdk/hooks';
import { vscode } from '../../../vscode';
import {
  testHistoryAtom,
  selectedHistoryIndexAtom,
  type TestHistoryRun,
  isParallelTestsEnabledAtom,
  currentWatchNotificationsAtom,
  highlightedBlocksAtom,
} from './atoms';
import {
  type TestState,
  currentAbortControllerAtom,
  areTestsRunningAtom,
  flashRangesAtom,
  testCaseAtom,
} from '../../../../atoms';

const enrichNotification = (notification: any): any => {
  if (!notification.block_name) {
    try {
      const parsed = JSON.parse(notification.value) as { type?: string; label?: string } | undefined;
      if (parsed?.type === 'block' && typeof parsed.label === 'string') {
        notification.block_name = parsed.label;
      }
    } catch {}
  }
  return notification;
};

export function useRunBamlTests() {
  const sdk = useBAMLSDK();
  const setTestHistory = useSetAtom(testHistoryAtom);
  const setSelectedHistoryIndex = useSetAtom(selectedHistoryIndexAtom);
  const setCurrentAbortController = useSetAtom(currentAbortControllerAtom);
  const setAreTestsRunning = useSetAtom(areTestsRunningAtom);
  const setCurrentWatchNotifications = useSetAtom(currentWatchNotificationsAtom);
  const setHighlightedBlocks = useSetAtom(highlightedBlocksAtom);
  const setFlashRanges = useSetAtom(flashRangesAtom);
  const isParallel = useAtomValue(isParallelTestsEnabledAtom);
  const currentAbortController = useAtomValue(currentAbortControllerAtom);

  const runTests = useCallback(
    async (tests: Array<{ functionName: string; testName: string }>) => {
      console.log('[useRunBamlTests] Running tests', { count: tests.length, parallel: isParallel });

      // Create abort controller
      const controller = new AbortController();
      setCurrentAbortController(controller);
      setAreTestsRunning(true);

      // Create history run
      const historyRun: TestHistoryRun = {
        timestamp: Date.now(),
        tests: tests.map((test) => ({
          timestamp: Date.now(),
          functionName: test.functionName,
          testName: test.testName,
          response: { status: 'queued' },
          input: undefined, // Will be populated from testCaseAtom
        })),
      };

      // Add to history
      setTestHistory((prev) => [historyRun, ...prev]);
      setSelectedHistoryIndex(0);
      setCurrentWatchNotifications([]);
      setHighlightedBlocks(new Set());

      // Send telemetry
      vscode.sendTelemetry({
        action: 'run_tests',
        data: {
          num_tests: tests.length,
          parallel: isParallel,
        },
      });

      try {
        // Call SDK
        for await (const event of sdk.tests.runAll(tests, {
          parallel: isParallel,
          abortSignal: controller.signal,
        })) {
          // Update history based on event type
          handleTestEvent(event, {
            setTestHistory,
            setCurrentWatchNotifications,
            setHighlightedBlocks,
            setFlashRanges,
          });
        }
      } catch (error) {
        console.error('[useRunBamlTests] Error running tests', error);
      } finally {
        setAreTestsRunning(false);
        setCurrentAbortController(null);
      }
    },
    [
      sdk,
      isParallel,
      setTestHistory,
      setSelectedHistoryIndex,
      setCurrentAbortController,
      setAreTestsRunning,
      setCurrentWatchNotifications,
      setHighlightedBlocks,
      setFlashRanges,
    ]
  );

  const cancelTests = useCallback(() => {
    console.log('[useRunBamlTests] Cancelling tests');
    if (currentAbortController) {
      currentAbortController.abort();
      setCurrentAbortController(null);
      setAreTestsRunning(false);
    }
  }, [currentAbortController, setCurrentAbortController, setAreTestsRunning]);

  return { runTests, cancelTests };
}

/**
 * Handle test execution events and update atoms
 */
function handleTestEvent(
  event: TestExecutionEvent,
  handlers: {
    setTestHistory: (updater: (prev: TestHistoryRun[]) => TestHistoryRun[]) => void;
    setCurrentWatchNotifications: (updater: (prev: any[]) => any[]) => void;
    setHighlightedBlocks: (updater: (prev: Set<string>) => Set<string>) => void;
    setFlashRanges: (ranges: any[]) => void;
  }
) {
  const { setTestHistory, setCurrentWatchNotifications, setHighlightedBlocks, setFlashRanges } = handlers;

  switch (event.type) {
    case 'test.started':
      setTestHistory((prev) => {
        const newHistory = [...prev];
        const currentRun = newHistory[0];
        if (!currentRun) return prev;

        const testIndex = currentRun.tests.findIndex(
          (t) => t.functionName === event.functionName && t.testName === event.testName
        );
        if (testIndex === -1) return prev;

        currentRun.tests[testIndex] = {
          ...currentRun.tests[testIndex]!,
          response: { status: 'running' },
          timestamp: event.timestamp,
        };
        return newHistory;
      });
      break;

    case 'test.partial':
      setTestHistory((prev) => {
        const newHistory = [...prev];
        const currentRun = newHistory[0];
        if (!currentRun) return prev;

        const testIndex = currentRun.tests.findIndex(
          (t) => t.functionName === event.functionName && t.testName === event.testName
        );
        if (testIndex === -1) return prev;

        currentRun.tests[testIndex] = {
          ...currentRun.tests[testIndex]!,
          response: {
            status: 'running',
            response: event.partialResponse,
          },
        };
        return newHistory;
      });
      break;

    case 'test.watch':
      const enriched = enrichNotification(event.notification);
      setCurrentWatchNotifications((prev) => [...prev, enriched]);
      if (enriched.block_name) {
        setHighlightedBlocks((prev) => {
          const next = new Set(prev);
          next.add(enriched.block_name);
          return next;
        });
      }
      break;

    case 'test.span':
      const flashRanges = event.spans.map((span) => ({
        filePath: span.file_path,
        startLine: span.start_line,
        startCol: span.start,
        endLine: span.end_line,
        endCol: span.end,
      }));
      setFlashRanges(flashRanges);
      vscode.setFlashingRegions(event.spans);
      break;

    case 'test.completed':
      setTestHistory((prev) => {
        const newHistory = [...prev];
        const currentRun = newHistory[0];
        if (!currentRun) return prev;

        const testIndex = currentRun.tests.findIndex(
          (t) => t.functionName === event.functionName && t.testName === event.testName
        );
        if (testIndex === -1) return prev;

        currentRun.tests[testIndex] = {
          ...currentRun.tests[testIndex]!,
          response: {
            status: 'done',
            response_status: event.status as any,
            response: event.response,
            latency_ms: event.duration,
          },
          timestamp: Date.now(),
        };
        return newHistory;
      });
      break;

    case 'test.error':
      setTestHistory((prev) => {
        const newHistory = [...prev];
        const currentRun = newHistory[0];
        if (!currentRun) return prev;

        const testIndex = currentRun.tests.findIndex(
          (t) => t.functionName === event.functionName && t.testName === event.testName
        );
        if (testIndex === -1) return prev;

        currentRun.tests[testIndex] = {
          ...currentRun.tests[testIndex]!,
          response: {
            status: 'error',
            message: typeof event.error === 'string' ? event.error : event.error.message,
          },
          timestamp: Date.now(),
        };
        return newHistory;
      });
      break;

    case 'test.cancelled':
      setTestHistory((prev) => {
        const newHistory = [...prev];
        const currentRun = newHistory[0];
        if (!currentRun) return prev;

        const testIndex = currentRun.tests.findIndex(
          (t) => t.functionName === event.functionName && t.testName === event.testName
        );
        if (testIndex === -1) return prev;

        currentRun.tests[testIndex] = {
          ...currentRun.tests[testIndex]!,
          response: {
            status: 'error',
            message: 'Test execution was cancelled by user',
          },
          timestamp: Date.now(),
        };
        return newHistory;
      });
      break;
  }
}
