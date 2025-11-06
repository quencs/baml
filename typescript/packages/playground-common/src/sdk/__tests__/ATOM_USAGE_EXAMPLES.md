# SDK Atom Usage Examples

The SDK exposes all atoms via `sdk.atoms`, with test-related atoms namespaced under `sdk.atoms.test`.

## Atom Structure

```typescript
sdk.atoms = {
  // Core atoms (top level)
  workflowsAtom,
  activeWorkflowIdAtom,
  diagnosticsAtom,
  generatedFilesAtom,
  envVarsAtom,
  featureFlagsAtom,
  // ... etc

  // Test atoms (namespaced)
  test: {
    testHistoryAtom,
    areTestsRunningAtom,
    currentWatchNotificationsAtom,
    highlightedBlocksAtom,
    flashRangesAtom,
    // ... etc
  }
}
```

## Usage in React Components

### Reading Core State

```typescript
import { useAtomValue } from 'jotai';
import { useBamlSDK } from './sdk-provider';

function DiagnosticsPanel() {
  const sdk = useBamlSDK();

  // Read core atoms
  const diagnostics = useAtomValue(sdk.atoms.diagnosticsAtom);
  const { errors, warnings } = useAtomValue(sdk.atoms.numErrorsAtom);
  const isValid = useAtomValue(sdk.atoms.lastValidRuntimeAtom);

  return (
    <div>
      <h2>Diagnostics</h2>
      <p>Errors: {errors}, Warnings: {warnings}</p>
      <p>Runtime Valid: {isValid ? 'Yes' : 'No'}</p>
    </div>
  );
}
```

### Reading Test State

```typescript
import { useAtomValue } from 'jotai';
import { useBamlSDK } from './sdk-provider';

function TestHistoryPanel() {
  const sdk = useBamlSDK();

  // Read test atoms (note the .test namespace)
  const testHistory = useAtomValue(sdk.atoms.test.testHistoryAtom);
  const areRunning = useAtomValue(sdk.atoms.test.areTestsRunningAtom);
  const notifications = useAtomValue(sdk.atoms.test.currentWatchNotificationsAtom);

  return (
    <div>
      <h2>Test History</h2>
      {areRunning && <div>Tests are running...</div>}

      {testHistory.map((run) => (
        <TestRunDisplay key={run.timestamp} run={run} />
      ))}

      <h3>Watch Notifications</h3>
      {notifications.map((n, i) => (
        <NotificationDisplay key={i} notification={n} />
      ))}
    </div>
  );
}
```

### Combined View

```typescript
function FullStateView() {
  const sdk = useBamlSDK();

  // Core state
  const workflows = useAtomValue(sdk.atoms.workflowsAtom);
  const diagnostics = useAtomValue(sdk.atoms.diagnosticsAtom);

  // Test state (namespaced)
  const testHistory = useAtomValue(sdk.atoms.test.testHistoryAtom);
  const areTestsRunning = useAtomValue(sdk.atoms.test.areTestsRunningAtom);

  return (
    <div>
      <section>
        <h2>Workflows ({workflows.length})</h2>
        <p>Diagnostics: {diagnostics.length}</p>
      </section>

      <section>
        <h2>Tests</h2>
        <p>Running: {areTestsRunning ? 'Yes' : 'No'}</p>
        <p>History: {testHistory.length} runs</p>
      </section>
    </div>
  );
}
```

## Running Tests

The SDK automatically manages all test state. UI components just call `sdk.tests.run()` and read atoms:

```typescript
function TestRunner() {
  const sdk = useBamlSDK();

  // Read atoms (SDK updates these automatically)
  const testHistory = useAtomValue(sdk.atoms.test.testHistoryAtom);
  const areRunning = useAtomValue(sdk.atoms.test.areTestsRunningAtom);

  const runTest = async () => {
    // ✅ SDK handles ALL state management automatically:
    // - Creates test history run
    // - Sets areTestsRunning = true
    // - Clears watch notifications
    // - Updates test state during execution
    // - Adds final result to history
    // - Sets areTestsRunning = false
    await sdk.tests.run('MyFunction', 'Test1');

    // That's it! Just read atoms to display state
  };

  return (
    <div>
      <button onClick={runTest} disabled={areRunning}>
        Run Test
      </button>

      {/* UI just displays state from atoms */}
      {testHistory.map((run) => (
        <TestRunDisplay key={run.timestamp} run={run} />
      ))}
    </div>
  );
}
```

### When to Use Storage Directly

You **rarely** need to call `sdk.storage.*` directly. The SDK manages state automatically.

Only use storage for:
- Custom UI state (e.g., manually selecting a different history index)
- Advanced features not covered by SDK methods

```typescript
// Example: Let user manually select a history entry
const selectHistory = (index: number) => {
  sdk.storage.setSelectedHistoryIndex(index);
};
```

## Why Namespace Test Atoms?

1. **Clarity**: Immediately obvious which atoms are test-related
2. **Organization**: Keeps the SDK API clean and organized
3. **Discoverability**: IDE autocomplete shows `sdk.atoms.test.*` as a logical group
4. **Future-proofing**: Easy to add more namespaces (e.g., `sdk.atoms.execution.*`, `sdk.atoms.cache.*`)

## Complete Atom Reference

### Core Atoms (`sdk.atoms.*`)
- `workflowsAtom` - All workflows
- `activeWorkflowIdAtom` - Currently selected workflow
- `diagnosticsAtom` - Compilation errors/warnings
- `generatedFilesAtom` - Generated code files
- `envVarsAtom` - Environment variables
- `featureFlagsAtom` - Runtime feature flags
- `bamlFilesTrackedAtom` - BAML source files
- `wasmPanicAtom` - WASM crash state

### Test Atoms (`sdk.atoms.test.*`)
- `testHistoryAtom` - All test run history
- `selectedHistoryIndexAtom` - Selected history index
- `selectedTestHistoryAtom` - Currently selected run (derived)
- `areTestsRunningAtom` - Execution status
- `currentAbortControllerAtom` - Abort controller for cancellation
- `currentWatchNotificationsAtom` - Watch notifications
- `highlightedBlocksAtom` - Highlighted code blocks
- `flashRangesAtom` - Code highlight ranges
- `categorizedNotificationsAtom` - Categorized notifications (derived)
