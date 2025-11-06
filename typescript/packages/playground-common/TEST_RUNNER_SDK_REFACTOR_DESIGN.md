# Design Doc: Refactor Test Runner to Use SDK

**Date:** 2025-11-04
**Status:** Draft
**Author:** Claude Code
**References:**
- MERGE_DESIGN_DOC.md (Section: EventListener vs bamlSDK Pattern)
- MERGE_DESIGN_DOC_ANSWERS.md (Question 7: Unified Execution Model)

---

## Problem Statement

The current `useRunBamlTests` hook in `test-runner.ts` directly calls WASM runtime and manually updates atoms. This violates the SDK pattern established in our architecture where:

1. **SDK handles business logic** (execution, state management)
2. **Components call SDK methods** (not WASM directly)
3. **SDK emits events** that update atoms
4. **SDK manages test history** and watch notifications

**Current Issues:**
- ❌ Test execution bypasses SDK ExecutionEngine
- ❌ `test-runner.ts` directly manipulates atoms (`areTestsRunningAtom`, `testHistoryAtom`, `runningTestsAtom`)
- ❌ Watch notification handling is duplicated (should be in SDK)
- ❌ Can't leverage SDK's unified execution model (caching, events, etc.)
- ❌ Test history management is ad-hoc (should be centralized)
- ❌ Hard to test (tightly coupled to WASM)

---

## Goals

1. **SDK manages test execution** - `sdk.tests.runAll()` handles execution logic
2. **SDK manages test history** - Test history atom updates are SDK responsibility
3. **SDK emits watch notifications** - Watch notifications flow through SDK events
4. **Hook becomes thin adapter** - `useRunBamlTests` just calls SDK and subscribes to events
5. **Backward compatibility** - Existing components work without changes
6. **State updates preserved** - Test history, running tests, watch notifications all still update

---

## Current Architecture

```
┌────────────────────────────────────────────────────────┐
│ useRunBamlTests Hook (test-runner.ts)                  │
│ ~630 lines of complex logic                            │
└────────────────────────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
   WASM Runtime    Jotai Atoms    Watch Notifications
   (direct calls)  (direct sets)   (manual tracking)
        │                │                │
        │                │                │
   rt.run_tests()  testHistoryAtom  currentWatchNotificationsAtom
                   areTestsRunningAtom
                   runningTestsAtom
```

**Problems:**
- Hook has too many responsibilities
- Business logic in UI layer
- Atoms updated in multiple places
- Can't test without WASM
- No event system

---

## Proposed Architecture

```
┌────────────────────────────────────────────────────────┐
│ useRunBamlTests Hook (test-runner.ts)                  │
│ ~100 lines - thin adapter                              │
└────────────────────────────────────────────────────────┘
                         │
                         ▼
                sdk.tests.runAll()
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
  ExecutionEngine   Event Emission   State Management
  (Phase 4)         (AsyncGenerator)  (Atoms via store)
        │                │                │
        │                │                │
  WASM Runtime      Subscribers      testHistoryAtom
  (provider)        get events       areTestsRunningAtom
                                     watchNotificationsAtom
```

**Benefits:**
- ✅ Clean separation of concerns
- ✅ SDK handles business logic
- ✅ Hook just orchestrates UI
- ✅ Testable (mock SDK)
- ✅ Unified with workflow execution
- ✅ Event-driven updates

---

## Design Details

### 1. SDK Test Execution API

**Already exists from Phase 4:**
```typescript
class BAMLSDK {
  tests = {
    /**
     * Run multiple tests in parallel or series
     */
    runAll: async (
      tests: Array<{ functionName: string; testName: string }>,
      options?: {
        parallel?: boolean;
        onProgress?: (event: TestExecutionEvent) => void;
      }
    ): Promise<ExecutionResult[]> => {
      // Uses ExecutionEngine internally
      // Emits events via AsyncGenerator
      // Returns results
    },

    /**
     * Cancel running tests
     */
    cancel: (): void => {
      // Aborts all active executions
    }
  }
}
```

**What's missing:**
- Test history tracking in SDK
- Watch notification events
- Running tests state management

### 2. Enhanced SDK Test API

**Add test history management:**
```typescript
// In src/sdk/index.ts

class BAMLSDK {
  tests = {
    /**
     * Run multiple tests with history tracking
     */
    runAll: async (
      tests: Array<{ functionName: string; testName: string }>,
      options?: {
        parallel?: boolean;
        trackHistory?: boolean; // Default: true
      }
    ): Promise<ExecutionResult[]> => {
      console.log('[SDK.tests.runAll]', { count: tests.length, parallel: options?.parallel });

      // Update running state
      this.store.set(areTestsRunningAtom, true);

      // Track active tests
      const runningTests = tests.map(test => ({
        functionName: test.functionName,
        testName: test.testName,
        state: { status: 'queued' } as TestState,
      }));
      this.store.set(runningTestsAtom, runningTests);

      try {
        const results: ExecutionResult[] = [];
        const watchNotificationsByTest: Record<string, WatchNotification[]> = {};

        if (options?.parallel) {
          // Run in parallel
          const promises = tests.map(async (test) => {
            const testKey = `${test.functionName}:${test.testName}`;
            watchNotificationsByTest[testKey] = [];

            // Use ExecutionEngine
            const events: ExecutionEvent[] = [];
            for await (const event of this.executionEngine.execute({
              mode: 'function-isolated',
              functionName: test.functionName,
              testName: test.testName,
            })) {
              events.push(event);

              // Extract watch notifications from events
              if (event.type === 'node.watch' || event.type === 'execution.watch') {
                watchNotificationsByTest[testKey].push(event.notification);
              }

              // Update running test state
              if (event.type === 'node.started') {
                this.updateRunningTestState(test.functionName, test.testName, {
                  status: 'running',
                  watchNotifications: watchNotificationsByTest[testKey],
                });
              } else if (event.type === 'node.completed') {
                this.updateRunningTestState(test.functionName, test.testName, {
                  status: 'done',
                  response_status: 'passed',
                  response: event.result,
                  latency_ms: event.duration || 0,
                  watchNotifications: watchNotificationsByTest[testKey],
                });
              } else if (event.type === 'node.error') {
                this.updateRunningTestState(test.functionName, test.testName, {
                  status: 'error',
                  message: event.error.message,
                  watchNotifications: watchNotificationsByTest[testKey],
                });
              }

              // Emit to subscribers
              this.emitEvent(event);
            }

            return this.buildResultFromEvents(events);
          });

          results.push(...(await Promise.all(promises)));
        } else {
          // Run sequentially
          for (const test of tests) {
            const testKey = `${test.functionName}:${test.testName}`;
            watchNotificationsByTest[testKey] = [];

            const events: ExecutionEvent[] = [];
            for await (const event of this.executionEngine.execute({
              mode: 'function-isolated',
              functionName: test.functionName,
              testName: test.testName,
            })) {
              events.push(event);

              // Extract watch notifications
              if (event.type === 'node.watch' || event.type === 'execution.watch') {
                watchNotificationsByTest[testKey].push(event.notification);
              }

              // Update running test state (same as above)
              // ... (same logic as parallel)

              this.emitEvent(event);
            }

            results.push(this.buildResultFromEvents(events));
          }
        }

        // Add to history if tracking enabled
        if (options?.trackHistory !== false) {
          const historyEntry: TestHistoryRun = {
            timestamp: Date.now(),
            tests: results.map((result, idx) => ({
              timestamp: Date.now(),
              functionName: tests[idx].functionName,
              testName: tests[idx].testName,
              response: this.convertResultToTestState(result),
              input: result.inputs,
            })),
          };

          this.store.set(testHistoryAtom, (prev) => [historyEntry, ...prev]);
          this.store.set(selectedHistoryIndexAtom, 0); // Select latest
        }

        return results;
      } finally {
        // Clear running state
        this.store.set(areTestsRunningAtom, false);
        this.store.set(runningTestsAtom, []);
      }
    },

    /**
     * Cancel running tests
     */
    cancel: (): void => {
      this.store.get(currentAbortControllerAtom)?.abort();
      this.store.set(areTestsRunningAtom, false);
      this.store.set(runningTestsAtom, []);
    },
  };

  // Helper methods
  private updateRunningTestState(
    functionName: string,
    testName: string,
    state: TestState
  ): void {
    this.store.set(runningTestsAtom, (prev) =>
      prev.map((test) =>
        test.functionName === functionName && test.testName === testName
          ? { ...test, state }
          : test
      )
    );
  }

  private convertResultToTestState(result: ExecutionResult): TestState {
    if (result.status === 'success') {
      return {
        status: 'done',
        response_status: 'passed',
        response: result.outputs as any, // WasmTestResponse
        latency_ms: result.duration || 0,
      };
    } else if (result.status === 'error') {
      return {
        status: 'error',
        message: result.error?.message || 'Unknown error',
      };
    } else if (result.status === 'cancelled') {
      return { status: 'idle' };
    }
    return { status: 'queued' };
  }
}
```

**What atoms does SDK update:**
- ✅ `areTestsRunningAtom` - Set before/after execution
- ✅ `runningTestsAtom` - Track active tests with state
- ✅ `testHistoryAtom` - Add completed run to history
- ✅ `selectedHistoryIndexAtom` - Select latest run
- ✅ `currentAbortControllerAtom` - For cancellation

**What atoms SDK reads:**
- ✅ `testCaseAtom` - Get test inputs
- ✅ `runtimeAtom` - Access WASM runtime (via provider)

---

### 3. Refactored useRunBamlTests Hook

**New implementation:**
```typescript
// src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/test-runner.ts

import { useCallback } from 'react';
import { useBAMLSDK } from '../../../../../sdk/provider';
import { useAtomValue, useSetAtom } from 'jotai';
import {
  currentWatchNotificationsAtom,
  highlightedBlocksAtom,
  isParallelTestsEnabledAtom,
} from './atoms';

/**
 * Hook for running BAML tests
 *
 * REFACTORED: Now uses SDK instead of direct WASM calls
 */
export function useRunBamlTests() {
  const sdk = useBAMLSDK();
  const isParallel = useAtomValue(isParallelTestsEnabledAtom);
  const setCurrentWatchNotifications = useSetAtom(currentWatchNotificationsAtom);
  const setHighlightedBlocks = useSetAtom(highlightedBlocksAtom);

  const runTests = useCallback(
    async (tests: Array<{ functionName: string; testName: string }>) => {
      console.log('[useRunBamlTests] Running tests via SDK', {
        count: tests.length,
        parallel: isParallel,
      });

      // Clear previous watch notifications
      setCurrentWatchNotifications([]);
      setHighlightedBlocks(new Set());

      // Subscribe to SDK events for watch notifications
      const unsubscribe = sdk.onEvent((event) => {
        // Handle watch notification events
        if (event.type === 'node.watch' || event.type === 'execution.watch') {
          const notification = enrichNotification(event.notification);
          setCurrentWatchNotifications((prev) => [...prev, notification]);

          // Auto-highlight blocks
          if (notification.block_name) {
            setHighlightedBlocks((prev) => new Set([...prev, notification.block_name!]));
          }
        }
      });

      try {
        // Call SDK - it handles everything!
        const results = await sdk.tests.runAll(tests, {
          parallel: isParallel,
          trackHistory: true,
        });

        console.log('[useRunBamlTests] Tests completed', { results });
        return results;
      } finally {
        // Cleanup subscription
        unsubscribe();
      }
    },
    [sdk, isParallel, setCurrentWatchNotifications, setHighlightedBlocks]
  );

  const cancelTests = useCallback(() => {
    console.log('[useRunBamlTests] Cancelling tests via SDK');
    sdk.tests.cancel();
  }, [sdk]);

  return { runTests, cancelTests };
}

// Helper function to enrich watch notifications with block labels
function enrichNotification(notification: WatchNotification): WatchNotification {
  // Parse block label from value if it's a JSON object
  try {
    const parsed = JSON.parse(notification.value);
    if (parsed?.type === 'block' && parsed.label) {
      return {
        ...notification,
        block_name: parsed.label,
      };
    }
  } catch {}

  return notification;
}
```

**Reduction:**
- ❌ **Before:** ~630 lines (complex WASM calls, atom updates, state tracking)
- ✅ **After:** ~80 lines (SDK call + event subscription)

---

### 4. Watch Notification Events

**Add to ExecutionEngine:**
```typescript
// In src/sdk/execution/engine.ts

async *execute(options: ExecutionOptions): AsyncGenerator<ExecutionEvent> {
  // ... existing code ...

  // When executing a test, emit watch notifications
  if (options.mode === 'function-isolated') {
    const runtime = this.store.get(runtimeAtom)?.rt;
    if (!runtime) {
      throw new Error('Runtime not available');
    }

    // Call WASM with watch notification callback
    const watchNotifications: WatchNotification[] = [];
    const result = await runtime.run_test(
      options.functionName,
      options.testName,
      {
        // Watch notification callback
        onWatch: (notification: WatchNotification) => {
          watchNotifications.push(notification);

          // Emit watch event
          yield {
            type: 'execution.watch',
            notification,
          };
        },
      }
    );

    // ... rest of execution logic ...
  }
}
```

**Event Types:**
```typescript
// In src/sdk/execution/types.ts

export type ExecutionEvent =
  | { type: 'execution.started'; executionId: string }
  | { type: 'execution.watch'; notification: WatchNotification } // NEW
  | { type: 'node.started'; nodeId: string }
  | { type: 'node.watch'; notification: WatchNotification } // NEW
  | { type: 'node.completed'; nodeId: string; result: any; duration: number }
  | { type: 'node.error'; nodeId: string; error: Error }
  | { type: 'execution.completed'; executionId: string; duration: number };
```

---

### 5. Test History Atoms

**Already exist, SDK just updates them:**
```typescript
// src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/atoms.ts

export interface TestHistoryEntry {
  timestamp: number;
  functionName: string;
  testName: string;
  response: TestState;
  input?: any;
}

export interface TestHistoryRun {
  timestamp: number;
  tests: TestHistoryEntry[];
}

export const testHistoryAtom = atom<TestHistoryRun[]>([]);
export const selectedHistoryIndexAtom = atom<number>(0);
```

**SDK updates these:**
```typescript
// After all tests complete
this.store.set(testHistoryAtom, (prev) => [newRun, ...prev]);
this.store.set(selectedHistoryIndexAtom, 0);
```

---

## Migration Plan

### Phase 1: Add SDK Test History Management

**Tasks:**
1. Add `testHistoryAtom` imports to SDK
2. Add `updateRunningTestState()` helper
3. Add `convertResultToTestState()` helper
4. Enhance `sdk.tests.runAll()` to update history

**Validation:**
- SDK can update test history atoms
- Test history persists across runs

### Phase 2: Add Watch Notification Events

**Tasks:**
1. Add `execution.watch` and `node.watch` event types
2. Update ExecutionEngine to emit watch events
3. Provider calls WASM with watch callback

**Validation:**
- Watch notifications flow through events
- Subscribers receive notifications

### Phase 3: Refactor useRunBamlTests Hook

**Tasks:**
1. Simplify hook to call `sdk.tests.runAll()`
2. Subscribe to SDK events for watch notifications
3. Remove direct WASM calls
4. Remove direct atom updates

**Validation:**
- Tests still run correctly
- Test history updates
- Watch notifications appear
- UI updates in real-time

### Phase 4: Cleanup

**Tasks:**
1. Remove unused imports from test-runner.ts
2. Remove unused helper functions
3. Update tests to mock SDK
4. Add JSDoc to new hook

**Validation:**
- Type check passes
- No unused code
- Tests pass

---

## Testing Strategy

### Unit Tests

```typescript
// test-runner.test.ts
describe('useRunBamlTests', () => {
  it('calls sdk.tests.runAll with correct parameters', async () => {
    const mockSdk = createMockSDK();
    const { result } = renderHook(() => useRunBamlTests(), {
      wrapper: ({ children }) => (
        <BAMLSDKProvider value={mockSdk}>{children}</BAMLSDKProvider>
      ),
    });

    await act(async () => {
      await result.current.runTests([
        { functionName: 'test1', testName: 'case1' },
      ]);
    });

    expect(mockSdk.tests.runAll).toHaveBeenCalledWith(
      [{ functionName: 'test1', testName: 'case1' }],
      { parallel: expect.any(Boolean), trackHistory: true }
    );
  });

  it('subscribes to SDK events and updates watch notifications', async () => {
    const mockSdk = createMockSDK();
    const { result } = renderHook(() => useRunBamlTests(), {
      wrapper: ({ children }) => (
        <BAMLSDKProvider value={mockSdk}>{children}</BAMLSDKProvider>
      ),
    });

    // Emit watch event
    mockSdk.emitEvent({
      type: 'execution.watch',
      notification: {
        variable_name: 'test_var',
        value: 'test value',
        is_stream: false,
      },
    });

    // Verify watch notification atom was updated
    const watchNotifications = store.get(currentWatchNotificationsAtom);
    expect(watchNotifications).toHaveLength(1);
    expect(watchNotifications[0].variable_name).toBe('test_var');
  });
});
```

### Integration Tests

```typescript
describe('SDK test execution integration', () => {
  it('executes test and updates history', async () => {
    const sdk = createBAMLSDK({ mode: 'mock', provider: mockProvider });
    await sdk.initialize();

    const results = await sdk.tests.runAll(
      [{ functionName: 'fetchData', testName: 'success_case' }],
      { trackHistory: true }
    );

    expect(results).toHaveLength(1);
    expect(results[0].status).toBe('success');

    // Verify history updated
    const history = store.get(testHistoryAtom);
    expect(history).toHaveLength(1);
    expect(history[0].tests).toHaveLength(1);
    expect(history[0].tests[0].functionName).toBe('fetchData');
  });
});
```

---

## Backward Compatibility

**All existing UI components continue to work:**
- ✅ Test panel still displays running tests
- ✅ Test history still shows past runs
- ✅ Watch notifications still appear
- ✅ Abort button still cancels tests

**Breaking changes:**
- ❌ None! Hook signature stays the same:
  ```typescript
  const { runTests, cancelTests } = useRunBamlTests();
  ```

**Migration for other code:**
- Any code calling WASM directly should also migrate to SDK
- Search for `runtime.run_test` or `runtime.run_tests` calls
- Replace with `sdk.tests.runAll()`

---

## Open Questions

### 1. Should SDK support streaming test results?

**Context:** Currently `runAll()` waits for all tests to complete before returning.

**Options:**
A. Keep current API (simple)
B. Return AsyncGenerator for streaming results
C. Both (AsyncGenerator + Promise)

**Recommendation:** A for now (existing behavior), B in future if needed

---

### 2. How should watch notifications be enriched?

**Context:** Currently `enrichNotification()` parses JSON to extract block labels.

**Options:**
A. Keep enrichment in hook (current proposal)
B. Move enrichment to SDK/ExecutionEngine
C. WASM should emit enriched notifications

**Recommendation:** C is ideal (fix in WASM), A for now (compatibility)

---

### 3. Should test history have size limits?

**Context:** Test history array grows unbounded.

**Options:**
A. No limit (current)
B. Keep last N runs (e.g., 50)
C. Let user configure limit

**Recommendation:** B (keep last 50), add cleanup logic to SDK

---

## Summary

**Current State:**
- Test runner directly calls WASM (~630 lines)
- Business logic in UI layer
- Hard to test and maintain

**Proposed State:**
- Test runner calls SDK (~80 lines)
- SDK handles execution, history, notifications
- Clean separation of concerns
- Easy to test

**Benefits:**
- ✅ 88% code reduction in test-runner.ts
- ✅ Unified execution model
- ✅ Centralized state management
- ✅ Event-driven architecture
- ✅ Testable with mocks
- ✅ Backward compatible

**Timeline:** 1-2 days
**Risk:** Low (backward compatible, well-defined interface)
**Impact:** High (cleaner architecture, easier maintenance)
