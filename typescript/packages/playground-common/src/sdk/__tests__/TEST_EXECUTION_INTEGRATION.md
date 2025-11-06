# Test Execution Integration

This document explains how the SDK now supports test execution with full UI state management.

## Architecture

### 1. Test Execution Atoms (`test.atoms.ts`)

All test execution state is managed through Jotai atoms:

```typescript
// Test history - tracks all test runs
export const testHistoryAtom = atom<TestHistoryRun[]>([]);
export const selectedHistoryIndexAtom = atom<number>(0);

// Execution state
export const areTestsRunningAtom = atom<boolean>(false);
export const currentAbortControllerAtom = atom<AbortController | null>(null);

// Watch notifications & highlighting
export const currentWatchNotificationsAtom = atom<WatchNotification[]>([]);
export const highlightedBlocksAtom = atom<Set<string>>(new Set());
export const flashRangesAtom = atom<FlashRange[]>([]);
```

### 2. Storage Layer (`SDKStorage`)

The storage interface now includes methods for managing test execution state:

```typescript
interface SDKStorage {
  // Test History
  addTestHistoryRun(run: TestHistoryRun): void;
  updateTestInHistory(runIndex: number, testIndex: number, update: TestState): void;
  getTestHistory(): TestHistoryRun[];

  // Execution State
  setAreTestsRunning(running: boolean): void;
  setCurrentAbortController(controller: AbortController | null): void;

  // Watch Notifications
  addWatchNotification(notification: WatchNotification): void;
  clearWatchNotifications(): void;

  // Highlighting
  addHighlightedBlock(blockName: string): void;
  setFlashRanges(ranges: FlashRange[]): void;
  // ... etc
}
```

### 3. SDK Integration

The SDK exposes test atoms via `sdk.atoms.test`:

```typescript
const sdk = createRealBAMLSDK(store);

// Access test atoms (namespaced)
sdk.atoms.test.testHistoryAtom;
sdk.atoms.test.areTestsRunningAtom;
sdk.atoms.test.currentWatchNotificationsAtom;
// ... etc

// Core atoms remain at top level
sdk.atoms.workflowsAtom;
sdk.atoms.diagnosticsAtom;
sdk.atoms.generatedFilesAtom;
```

## Usage Example: UI Component

Here's how a UI component would run tests and track state:

```typescript
import { useAtomValue, useSetAtom } from 'jotai';
import { useBamlSDK } from './sdk-provider';

function TestPanel() {
  const sdk = useBamlSDK();

  // Read test atoms (SDK updates these automatically)
  const testHistory = useAtomValue(sdk.atoms.test.testHistoryAtom);
  const areRunning = useAtomValue(sdk.atoms.test.areTestsRunningAtom);
  const watchNotifications = useAtomValue(sdk.atoms.test.currentWatchNotificationsAtom);

  const runTest = async () => {
    // ✅ SDK manages ALL state automatically!
    // - Creates test history run
    // - Sets areTestsRunning = true
    // - Clears notifications
    // - Updates during execution
    // - Adds final result
    // - Sets areTestsRunning = false
    await sdk.tests.run('ExtractResume', 'Test1');

    // That's it! SDK handles everything
  };

  return (
    <div>
      <button onClick={runTest} disabled={areRunning}>
        Run Test
      </button>

      {/* UI just displays state from atoms */}
      {testHistory.map((run, index) => (
        <TestRunDisplay key={run.timestamp} run={run} />
      ))}

      {watchNotifications.map((notification, index) => (
        <WatchNotification key={index} notification={notification} />
      ))}
    </div>
  );
}
```

## Next Steps: Enhanced Test Execution

The current `sdk.tests.run()` is basic. To support real-time UI updates during execution, we need to:

### Option 1: Callback-based (Like old test-runner)

```typescript
sdk.tests.runWithCallbacks({
  functionName: 'ExtractResume',
  testName: 'Test1',

  onPartialResponse: (response) => {
    // Update UI with streaming response
    sdk.storage.updateTestInHistory(0, 0, {
      status: 'running',
      response,
    });
  },

  onWatchNotification: (notification) => {
    // Add watch notification
    sdk.storage.addWatchNotification(notification);
    if (notification.block_name) {
      sdk.storage.addHighlightedBlock(notification.block_name);
    }
  },

  onExprEvent: (spans) => {
    // Highlight code ranges
    sdk.storage.setFlashRanges(spans);
  },
});
```

### Option 2: Event Stream (More flexible)

```typescript
for await (const event of sdk.tests.runStream('ExtractResume', 'Test1')) {
  switch (event.type) {
    case 'partial-response':
      sdk.storage.updateTestInHistory(0, 0, {
        status: 'running',
        response: event.data,
      });
      break;

    case 'watch-notification':
      sdk.storage.addWatchNotification(event.data);
      break;

    case 'highlight':
      sdk.storage.setFlashRanges(event.data.spans);
      break;

    case 'completed':
      sdk.storage.updateTestInHistory(0, 0, {
        status: 'done',
        response: event.data,
        response_status: 'passed',
        latency_ms: event.duration,
      });
      break;
  }
}
```

## Benefits

1. **Separation of Concerns**: SDK manages execution, UI components manage display
2. **Storage Abstraction**: Could swap Jotai for Redux/Zustand without changing SDK
3. **Reusable**: Same test execution logic works in VSCode extension, web playground, etc.
4. **Type-Safe**: Full TypeScript support for all state and events
5. **Testable**: Can test execution logic without UI components

## Migration Path

Components using the old `test-runner.ts` can migrate to the new SDK by:

1. Replace `useRunTests()` hook with `sdk.tests.run()`
2. Use `sdk.atoms.*` instead of direct atom imports
3. Use `sdk.storage.*` methods to update state
4. Keep the same UI components - just change data source

The atoms are the same, so UI components don't need to change!
