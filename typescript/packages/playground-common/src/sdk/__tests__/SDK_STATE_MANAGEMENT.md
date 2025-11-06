# SDK State Management Philosophy

## Core Principle

**The SDK manages ALL application state. UI components just read atoms and call SDK methods.**

## Pattern

```typescript
// ✅ Good: SDK manages state
function TestPanel() {
  const sdk = useBamlSDK();

  // 1. Read atoms (SDK updates automatically)
  const testHistory = useAtomValue(sdk.atoms.test.testHistoryAtom);
  const areRunning = useAtomValue(sdk.atoms.test.areTestsRunningAtom);

  // 2. Call SDK methods (they update atoms automatically)
  const runTest = async () => {
    await sdk.tests.run('MyFunction', 'Test1');
  };

  // 3. UI just displays the state
  return (
    <div>
      <button onClick={runTest} disabled={areRunning}>Run Test</button>
      {testHistory.map(run => <TestRun key={run.timestamp} run={run} />)}
    </div>
  );
}
```

## What SDK Handles Automatically

When you call `sdk.tests.run()`, the SDK automatically:

1. ✅ Creates test history run with inputs
2. ✅ Sets `areTestsRunningAtom = true`
3. ✅ Clears watch notifications
4. ✅ Clears highlighted blocks
5. ✅ Updates test state during execution (running → done/error)
6. ✅ Captures outputs and errors
7. ✅ Records latency
8. ✅ Sets `areTestsRunningAtom = false`

**UI components don't need to manage any of this!**

## When to Use Storage Directly

You **rarely** need `sdk.storage.*` methods. Only use them for:

### 1. Custom UI State

```typescript
// Let user manually select a history entry
const selectHistory = (index: number) => {
  sdk.storage.setSelectedHistoryIndex(index);
};
```

### 2. Advanced Features (Future)

```typescript
// When implementing callback-based updates (future)
sdk.tests.runWithCallbacks('MyFunction', 'Test1', {
  onWatchNotification: (notification) => {
    // SDK would call this, updating storage internally
    sdk.storage.addWatchNotification(notification);
  },
});
```

## Architecture

```
┌─────────────────────────────────────────────┐
│              UI Components                  │
│  - Read atoms via useAtomValue()           │
│  - Call SDK methods                        │
│  - Display state                           │
└──────────────┬──────────────────────────────┘
               │
               │ reads atoms
               │ calls methods
               ↓
┌─────────────────────────────────────────────┐
│                BAML SDK                     │
│  - Orchestrates execution                  │
│  - Manages ALL state automatically         │
│  - Updates atoms via storage               │
└──────────────┬──────────────────────────────┘
               │
               │ updates
               ↓
┌─────────────────────────────────────────────┐
│             SDK Storage                     │
│  - JotaiStorage / ReduxStorage / etc       │
│  - Updates Jotai atoms                     │
└──────────────┬──────────────────────────────┘
               │
               │ updates
               ↓
┌─────────────────────────────────────────────┐
│             Jotai Atoms                     │
│  sdk.atoms.test.testHistoryAtom           │
│  sdk.atoms.test.areTestsRunningAtom       │
│  sdk.atoms.test.currentWatchNotificationsAtom│
└─────────────────────────────────────────────┘
               │
               │ subscribed via useAtomValue
               ↓
       (back to UI Components)
```

## Benefits

### 1. **Separation of Concerns**
- SDK = business logic + state management
- UI = display + user interactions
- Storage = abstraction layer

### 2. **Simple UI Code**
```typescript
// Before (manual state management)
const runTest = async () => {
  setAreRunning(true);
  createHistoryRun();
  updateState({ status: 'running' });
  const result = await executeTest();
  updateState({ status: 'done', result });
  setAreRunning(false);
};

// After (SDK handles it)
const runTest = async () => {
  await sdk.tests.run('MyFunction', 'Test1');
};
```

### 3. **Consistent State**
SDK ensures state is always updated correctly. No risk of UI forgetting to clear notifications, mark as not running, etc.

### 4. **Testable**
Can test SDK methods without UI components:

```typescript
test('sdk.tests.run updates history', async () => {
  const sdk = createRealBAMLSDK(store);
  await sdk.initialize(files);

  await sdk.tests.run('Foo', 'Test1');

  const history = store.get(sdk.atoms.test.testHistoryAtom);
  expect(history).toHaveLength(1);
  expect(history[0].tests[0].testName).toBe('Test1');
});
```

### 5. **Reusable**
Same SDK works in:
- VSCode extension
- Web playground
- CLI tool
- Different UI frameworks (React, Vue, Svelte)

## Comparison with Old Pattern

### Old test-runner.ts Pattern
```typescript
// UI had to manage state manually
const setState = (update) => {
  set(testHistoryAtom, (prev) => {
    // manual history update logic
  });
};

setState({ status: 'running' });
const result = await fn.run_test(...);
setState({ status: 'done', result });
```

**Problem**: State management logic scattered across UI components.

### New SDK Pattern
```typescript
// SDK manages state automatically
await sdk.tests.run('Foo', 'Test1');
```

**Benefit**: State management centralized in SDK. UI just reads and displays.

## Summary

- ✅ **SDK = Single Source of Truth** for application state
- ✅ **UI = Read atoms + Call methods** (no manual state updates)
- ✅ **Storage = Abstraction** (can swap Jotai for Redux, etc.)
- ✅ **Simple, testable, reusable**

This is the recommended pattern for all SDK features, not just tests!
