# Phase 5: EventListener Refactor

**Timeline:** Week 4
**Dependencies:** Phase 4 (Execution Engine)
**Risk Level:** Medium

## Purpose

Transform EventListener from directly updating atoms to calling SDK methods. This makes EventListener a **thin adapter layer** that translates IDE messages into SDK API calls, preserving platform-specific quirks while delegating all business logic to the SDK.

**Benefits**:
- Separation of concerns: EventListener = message routing, SDK = business logic
- Easier testing: Mock SDK instead of atoms
- Better maintainability: All logic in one place (SDK)
- Platform agnostic: SDK can work without EventListener

## What This Document Will Cover

- New EventListener architecture (thin adapter pattern)
- Message type handling (ide_message vs lsp_message)
- Complete message → SDK method mapping
- Platform-specific handling (JetBrains delays, Zed quirks)
- Error handling and recovery
- WebSocket integration for non-VSCode IDEs
- Backward compatibility during migration
- Testing strategy for EventListener
- Step-by-step refactoring guide

## Key Decisions

**Architecture**:
- EventListener receives `sdk` via `useBAMLSDK()` hook
- Each message type maps to specific SDK method
- Platform quirks stay in EventListener (e.g., JetBrains 1s delay)
- EventListener remains a React component (for useEffect lifecycle)

**Business Logic**:
- NO business logic in EventListener (only message parsing and routing)
- All state updates through SDK methods
- All validation in SDK
- File debouncing stays in EventListener (platform quirk)

**Migration**:
- Gradual refactor: One message type at a time
- Keep old atom updates temporarily during migration
- Feature flag for new vs old behavior
- Test each message type thoroughly

## Source Files to Reference

### Current EventListener
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/baml_wasm_web/EventListener.tsx` (lines 57-217 - current implementation)

### Message Types
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/baml_wasm_web/vscode-to-webview-rpc.ts` (lines 4-88 - message type definitions)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/vscode-ext/src/panels/vscode-to-webview-rpc.ts` (lines 4-73 - extension side types)

### SDK Methods to Call
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/index.ts` (lines 74-349 - SDK API surface)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC.md` (lines 1548-1605 - Modified EventListener section)
- `/Users/aaronvillalpando/Projects/baml/typescript/BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md` (lines 882-950 - EventListener as thin bridge)

---

## Part 1: Current vs New Architecture

### 1.1 Current Architecture (Before)

```typescript
// Current: EventListener directly updates atoms
export const EventListener: React.FC = () => {
  const updateCursor = useSetAtom(updateCursorAtom);
  const setBamlFileMap = useAtom(filesAtom);
  const setSelectedFunction = useSetAtom(selectedFunctionAtom);
  const setSelectedTestcase = useSetAtom(selectedTestcaseAtom);
  const setBamlConfig = useSetAtom(bamlConfig);
  const { runTests } = useRunBamlTests();

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      const { source, payload } = event.data;

      switch (source) {
        case 'ide_message':
          switch (payload.command) {
            case 'update_cursor':
              // Direct atom update
              updateCursor(payload.content);
              break;
            case 'baml_settings_updated':
              // Direct atom update
              setBamlConfig(payload.content);
              break;
          }
          break;

        case 'lsp_message':
          switch (payload.method) {
            case 'runtime_updated':
              // Direct atom update
              setBamlFileMap(payload.params.files);
              break;
            case 'workspace/executeCommand':
              if (payload.params.command === 'baml.runBamlTest') {
                // Call hook (which updates atoms directly)
                runTests([{
                  functionName: payload.params.functionName,
                  testName: payload.params.testName
                }]);
              }
              break;
          }
          break;
      }
    };

    window.addEventListener('message', handler);
    return () => window.removeEventListener('message', handler);
  }, [/* many dependencies */]);

  return null;
};
```

**Problems**:
- ❌ Direct atom manipulation (business logic scattered)
- ❌ Many dependencies in useEffect
- ❌ Hard to test (need to mock all atoms)
- ❌ Tight coupling to state management
- ❌ Can't use SDK without React

### 1.2 New Architecture (After)

```typescript
// New: EventListener calls SDK methods
export const EventListener: React.FC = () => {
  const sdk = useBAMLSDK(); // Get SDK instance
  const debouncedUpdateFiles = useDebounceCallback(
    (files: Record<string, string>) => sdk.files.update(files),
    50,
    true
  );

  useEffect(() => {
    const handler = async (event: MessageEvent) => {
      const { source, payload } = event.data;

      try {
        switch (source) {
          case 'ide_message':
            await handleIDEMessage(sdk, payload);
            break;

          case 'lsp_message':
            await handleLSPMessage(sdk, payload, debouncedUpdateFiles);
            break;
        }
      } catch (error) {
        console.error('[EventListener] Error handling message:', error);
        // Optionally notify user
      }
    };

    window.addEventListener('message', handler);
    return () => window.removeEventListener('message', handler);
  }, [sdk]); // Single dependency

  return null;
};

/**
 * Handle IDE messages
 */
async function handleIDEMessage(sdk: BAMLSDK, payload: IDEMessagePayload) {
  switch (payload.command) {
    case 'update_cursor':
      // Call SDK method
      sdk.navigation.updateCursor(payload.content);
      break;

    case 'baml_settings_updated':
      // Call SDK method
      await sdk.settings.update(payload.content);
      break;

    case 'baml_cli_version':
      // Non-core state can still update atoms directly
      // (CLI version doesn't affect SDK logic)
      // OR: add sdk.info.setCliVersion() if needed
      break;
  }
}

/**
 * Handle LSP messages
 */
async function handleLSPMessage(
  sdk: BAMLSDK,
  payload: LSPMessagePayload,
  debouncedUpdateFiles: (files: Record<string, string>) => void
) {
  switch (payload.method) {
    case 'runtime_updated':
      // Debounce file updates (platform quirk)
      debouncedUpdateFiles(payload.params.files);
      break;

    case 'workspace/executeCommand':
      await handleWorkspaceCommand(sdk, payload.params);
      break;

    case 'textDocument/codeAction':
      sdk.navigation.updateCursorFromRange(payload.params.range);
      break;

    case 'baml_settings_updated':
      await sdk.settings.update(payload.params);
      break;
  }
}

/**
 * Handle workspace commands
 */
async function handleWorkspaceCommand(
  sdk: BAMLSDK,
  params: WorkspaceCommandParams
) {
  const { command, arguments: args } = params;

  switch (command) {
    case 'baml.openBamlPanel':
      sdk.navigation.selectFunction(args.functionName);
      break;

    case 'baml.runBamlTest':
      await sdk.tests.run(args.functionName, args.testCaseName);
      break;

    case 'baml.executeWorkflow':
      await sdk.executions.start(args.workflowId, args.inputs);
      break;
  }
}
```

**Benefits**:
- ✅ Thin adapter (only message routing)
- ✅ Single dependency (SDK)
- ✅ Easy to test (mock SDK)
- ✅ Loose coupling
- ✅ SDK usable without React

---

## Part 2: Complete Message → SDK Mapping

### 2.1 IDE Messages (ide_message)

| IDE Command | Current Atom Update | New SDK Method | Notes |
|-------------|-------------------|----------------|-------|
| `update_cursor` | `updateCursorAtom` | `sdk.navigation.updateCursor(cursor)` | Platform quirk: Some IDEs send spurious updates |
| `baml_settings_updated` | `bamlConfigAtom` | `sdk.settings.update(settings)` | |
| `baml_cli_version` | `bamlCliVersionAtom` | `sdk.info.setCliVersion(version)` | Non-core state, can keep as atom |

### 2.2 LSP Messages (lsp_message)

| LSP Method | Current Atom Update | New SDK Method | Notes |
|------------|-------------------|----------------|-------|
| `runtime_updated` | `filesAtom` (debounced) | `sdk.files.update(files)` | Keep 50ms debounce |
| `workspace/executeCommand` → `baml.openBamlPanel` | `selectedFunctionAtom` | `sdk.navigation.selectFunction(name)` | |
| `workspace/executeCommand` → `baml.runBamlTest` | Call `runTests` hook | `sdk.tests.run(fn, test)` | |
| `workspace/executeCommand` → `baml.executeWorkflow` | Call `executeWorkflow` | `sdk.executions.start(id, inputs)` | |
| `textDocument/codeAction` | `updateCursorAtom` | `sdk.navigation.updateCursorFromRange(range)` | |
| `baml_settings_updated` | `bamlConfigAtom` | `sdk.settings.update(settings)` | |

### 2.3 New SDK Methods to Add

The SDK needs these methods to support all EventListener messages:

```typescript
// File: src/sdk/index.ts

export class BAMLSDK {
  // Navigation methods
  navigation = {
    /**
     * Update cursor position from IDE
     */
    updateCursor(cursor: { fileName: string; line: number; column: number }): void {
      // Update cursor atom
      this.store.set(updateCursorAtom, cursor);

      // Enrich to CodeClickEvent (Phase 6)
      // const codeClick = enrichCursorToCodeClick(cursor, runtime, files);
      // this.store.set(activeCodeClickAtom, codeClick);
    },

    /**
     * Update cursor from text range
     */
    updateCursorFromRange(range: { start: Position; end: Position }): void {
      this.navigation.updateCursor({
        fileName: range.start.fileName,
        line: range.start.line,
        column: range.start.column,
      });
    },

    /**
     * Select function by name
     */
    selectFunction(functionName: string): void {
      // Set selected function
      this.store.set(selectedFunctionAtom, functionName);

      // Navigate to workflow if function is in one
      const workflows = this.workflows.getAll();
      const workflow = workflows.find((w) =>
        w.nodes.some((n) => n.functionName === functionName)
      );

      if (workflow) {
        this.workflows.setActive(workflow.id);
      }
    },
  };

  // File management
  files = {
    /**
     * Update files from LSP
     */
    update(files: Record<string, string>): void {
      // Update files atom
      this.store.set(filesAtom, files);

      // Trigger compilation
      // WASM runtime will recompile automatically via atom dependency
    },

    /**
     * Watch file changes
     */
    watch(callback: (files: Record<string, string>) => void): () => void {
      return this.store.sub(filesAtom, () => {
        const files = this.store.get(filesAtom);
        callback(files);
      });
    },
  };

  // Settings management
  settings = {
    /**
     * Update settings
     */
    async update(settings: Partial<BAMLSettings>): Promise<void> {
      const current = await this.store.get(vscodeSettingsAtom);
      this.store.set(vscodeSettingsAtom, { ...current, ...settings });

      // Emit event
      this.emitEvent({ type: 'settings.updated', settings });
    },

    /**
     * Get current settings
     */
    async get(): Promise<BAMLSettings> {
      return await this.store.get(vscodeSettingsAtom);
    },
  };

  // Tests (already exists, but needs update)
  tests = {
    /**
     * Run a single test
     */
    async run(
      functionName: string,
      testName: string,
      options?: { inputs?: Record<string, unknown> }
    ): Promise<ExecutionResult> {
      // Use ExecutionEngine
      const events: ExecutionEvent[] = [];

      for await (const event of this.execute({
        mode: 'function-isolated',
        functionName,
        testName,
        inputs: options?.inputs,
      })) {
        events.push(event);
        this.emitEvent(event);
      }

      return this.buildResultFromEvents(events);
    },

    /**
     * Run multiple tests
     */
    async runAll(
      tests: Array<{ functionName: string; testName: string }>,
      options?: { parallel?: boolean }
    ): Promise<ExecutionResult[]> {
      // Implementation
    },

    /**
     * Cancel running tests
     */
    cancel(): void {
      // Cancel via ExecutionEngine
    },
  };

  // Executions (already exists, but needs consistency)
  executions = {
    /**
     * Start workflow execution
     */
    async start(
      workflowId: string,
      inputs: Record<string, unknown>,
      options?: { startFromNodeId?: string; clearCache?: boolean }
    ): Promise<string> {
      // Use ExecutionEngine
      const executionId = `exec_${Date.now()}`;

      (async () => {
        for await (const event of this.execute({
          mode: 'workflow',
          workflowId,
          inputs,
          ...options,
        })) {
          this.emitEvent(event);
        }
      })();

      return executionId;
    },

    // ... other methods
  };

  // Info (for non-core state like CLI version)
  info = {
    setCliVersion(version: string): void {
      // This can stay as direct atom update since it's not core business logic
      this.store.set(bamlCliVersionAtom, version);
    },

    getCliVersion(): string | null {
      return this.store.get(bamlCliVersionAtom);
    },
  };
}
```

---

## Part 3: Platform-Specific Quirks

### 3.1 File Update Debouncing

**Why**: LSP sends rapid file updates during typing. Debouncing prevents excessive WASM recompilation.

```typescript
// File: EventListener.tsx

export const EventListener: React.FC = () => {
  const sdk = useBAMLSDK();

  // Debounce file updates (50ms)
  // This is a platform quirk, not business logic
  const debouncedUpdateFiles = useDebounceCallback(
    (files: Record<string, string>) => {
      console.log('[EventListener] Debounced file update');
      sdk.files.update(files);
    },
    50,
    true // Leading edge (update immediately on first change)
  );

  // ... message handler uses debouncedUpdateFiles
};
```

### 3.2 JetBrains IDE Delay

**Why**: JetBrains IDEs have a ~1s delay before webview is fully ready.

```typescript
// Platform quirk: JetBrains needs delay before marking initialized
useEffect(() => {
  if (!wasm) return;

  const isJetBrains = vscode.isJetBrains();

  if (isJetBrains) {
    // Wait 1s for JetBrains to be ready
    setTimeout(() => {
      vscode.markInitialized();
    }, 1000);
  } else {
    vscode.markInitialized();
  }
}, [wasm]);
```

### 3.3 WebSocket Fallback (Non-VSCode)

**Why**: Standalone playground uses WebSocket instead of VSCode message API.

```typescript
// File: EventListener.tsx

export const EventListener: React.FC = () => {
  const sdk = useBAMLSDK();
  const isVSCodeWebview = vscode.isVscode();

  // WebSocket for non-VSCode environments
  useEffect(() => {
    if (isVSCodeWebview) {
      return; // Use window.postMessage in VSCode
    }

    // Connect to WebSocket server
    const scheme = window.location.protocol === 'https:' ? 'wss' : 'ws';
    const ws = new WebSocket(`${scheme}://${window.location.host}/ws`);

    ws.onmessage = (e) => {
      try {
        const payload = JSON.parse(e.data);
        // Forward to main message handler
        window.postMessage(payload, '*');
      } catch (err) {
        console.error('[EventListener] Invalid WS payload', err);
      }
    };

    return () => ws.close();
  }, [isVSCodeWebview]);

  // ... rest of EventListener
};
```

---

## Part 4: Error Handling

### 4.1 Message Handler Error Handling

```typescript
useEffect(() => {
  const handler = async (event: MessageEvent) => {
    const { source, payload } = event.data;

    try {
      switch (source) {
        case 'ide_message':
          await handleIDEMessage(sdk, payload);
          break;

        case 'lsp_message':
          await handleLSPMessage(sdk, payload, debouncedUpdateFiles);
          break;

        default:
          console.warn('[EventListener] Unknown message source:', source);
      }
    } catch (error) {
      // Log error
      console.error('[EventListener] Error handling message:', {
        source,
        payload,
        error,
      });

      // Optionally show error to user
      // sdk.notifications.showError('Failed to process IDE message');

      // Don't crash EventListener - continue listening
    }
  };

  window.addEventListener('message', handler);
  return () => window.removeEventListener('message', handler);
}, [sdk, debouncedUpdateFiles]);
```

### 4.2 SDK Method Error Handling

```typescript
async function handleWorkspaceCommand(
  sdk: BAMLSDK,
  params: WorkspaceCommandParams
) {
  const { command, arguments: args } = params;

  try {
    switch (command) {
      case 'baml.runBamlTest':
        await sdk.tests.run(args.functionName, args.testCaseName);
        break;

      case 'baml.executeWorkflow':
        await sdk.executions.start(args.workflowId, args.inputs);
        break;

      default:
        console.warn('[EventListener] Unknown workspace command:', command);
    }
  } catch (error) {
    console.error('[EventListener] Command execution failed:', {
      command,
      args,
      error,
    });

    // Show error in UI
    // sdk.notifications.showError(`Failed to execute ${command}: ${error.message}`);
  }
}
```

---

## Part 5: Testing Strategy

### 5.1 Mock SDK for Testing

```typescript
// File: EventListener.test.tsx

import { render } from '@testing-library/react';
import { EventListener } from './EventListener';
import { BAMLSDKProvider } from '../sdk/provider';

// Create mock SDK
const createMockSDK = (): jest.Mocked<BAMLSDK> => ({
  navigation: {
    updateCursor: jest.fn(),
    updateCursorFromRange: jest.fn(),
    selectFunction: jest.fn(),
  },
  files: {
    update: jest.fn(),
    watch: jest.fn(() => () => {}),
  },
  settings: {
    update: jest.fn(),
    get: jest.fn(),
  },
  tests: {
    run: jest.fn(),
    runAll: jest.fn(),
    cancel: jest.fn(),
  },
  executions: {
    start: jest.fn(),
    cancel: jest.fn(),
  },
  info: {
    setCliVersion: jest.fn(),
    getCliVersion: jest.fn(),
  },
});

describe('EventListener', () => {
  let mockSDK: jest.Mocked<BAMLSDK>;

  beforeEach(() => {
    mockSDK = createMockSDK();
  });

  it('handles update_cursor message', () => {
    render(
      <BAMLSDKProvider sdk={mockSDK}>
        <EventListener />
      </BAMLSDKProvider>
    );

    // Send message
    window.postMessage(
      {
        source: 'ide_message',
        payload: {
          command: 'update_cursor',
          content: { fileName: 'test.baml', line: 10, column: 5 },
        },
      },
      '*'
    );

    // Assert SDK method called
    expect(mockSDK.navigation.updateCursor).toHaveBeenCalledWith({
      fileName: 'test.baml',
      line: 10,
      column: 5,
    });
  });

  it('handles runtime_updated message with debouncing', async () => {
    jest.useFakeTimers();

    render(
      <BAMLSDKProvider sdk={mockSDK}>
        <EventListener />
      </BAMLSDKProvider>
    );

    // Send multiple rapid file updates
    window.postMessage(
      {
        source: 'lsp_message',
        payload: {
          method: 'runtime_updated',
          params: { files: { 'test.baml': 'content1' } },
        },
      },
      '*'
    );

    window.postMessage(
      {
        source: 'lsp_message',
        payload: {
          method: 'runtime_updated',
          params: { files: { 'test.baml': 'content2' } },
        },
      },
      '*'
    );

    // Should not be called yet (debounced)
    expect(mockSDK.files.update).not.toHaveBeenCalled();

    // Advance timers past debounce delay
    jest.advanceTimersByTime(50);

    // Should be called once with latest files
    expect(mockSDK.files.update).toHaveBeenCalledTimes(1);
    expect(mockSDK.files.update).toHaveBeenCalledWith({
      'test.baml': 'content2',
    });

    jest.useRealTimers();
  });

  it('handles baml.runBamlTest command', async () => {
    render(
      <BAMLSDKProvider sdk={mockSDK}>
        <EventListener />
      </BAMLSDKProvider>
    );

    window.postMessage(
      {
        source: 'lsp_message',
        payload: {
          method: 'workspace/executeCommand',
          params: {
            command: 'baml.runBamlTest',
            arguments: {
              functionName: 'TestFunction',
              testCaseName: 'test_case_1',
            },
          },
        },
      },
      '*'
    );

    // Wait for async handling
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(mockSDK.tests.run).toHaveBeenCalledWith('TestFunction', 'test_case_1');
  });

  it('handles errors gracefully', async () => {
    const consoleSpy = jest.spyOn(console, 'error').mockImplementation();

    mockSDK.tests.run.mockRejectedValue(new Error('Test failed'));

    render(
      <BAMLSDKProvider sdk={mockSDK}>
        <EventListener />
      </BAMLSDKProvider>
    );

    window.postMessage(
      {
        source: 'lsp_message',
        payload: {
          method: 'workspace/executeCommand',
          params: {
            command: 'baml.runBamlTest',
            arguments: {
              functionName: 'TestFunction',
              testCaseName: 'test_case_1',
            },
          },
        },
      },
      '*'
    );

    await new Promise((resolve) => setTimeout(resolve, 0));

    // Should log error
    expect(consoleSpy).toHaveBeenCalledWith(
      expect.stringContaining('Command execution failed'),
      expect.any(Object)
    );

    // Should not crash EventListener
    expect(mockSDK.tests.run).toHaveBeenCalled();

    consoleSpy.mockRestore();
  });
});
```

### 5.2 Integration Tests

```typescript
// Test with real SDK and mock provider
describe('EventListener Integration', () => {
  it('complete message flow works', async () => {
    const mockProvider = createMockProvider();
    const store = createStore();
    const sdk = createBAMLSDK({ mode: 'mock', provider: mockProvider }, store);

    render(
      <BAMLSDKProvider sdk={sdk}>
        <EventListener />
      </BAMLSDKProvider>
    );

    // Send test execution command
    window.postMessage(
      {
        source: 'lsp_message',
        payload: {
          method: 'workspace/executeCommand',
          params: {
            command: 'baml.runBamlTest',
            arguments: {
              functionName: 'fetchData',
              testCaseName: 'success_case',
            },
          },
        },
      },
      '*'
    );

    // Wait for execution
    await new Promise((resolve) => setTimeout(resolve, 100));

    // Verify test ran
    const history = store.get(testHistoryAtom);
    expect(history).toHaveLength(1);
    expect(history[0].tests[0].functionName).toBe('fetchData');
  });
});
```

---

## Part 6: Migration Guide

### Step 1: Add SDK Methods

1. Add `navigation`, `files`, `settings`, `info` to BAMLSDK
2. Test each method independently
3. Ensure backward compatibility

### Step 2: Update EventListener (Gradual)

Refactor one message type at a time:

```typescript
// Phase 1: Add SDK, keep atoms
const sdk = useBAMLSDK();
const updateCursor = useSetAtom(updateCursorAtom); // Keep for now

// Phase 2: Call both SDK and atoms
case 'update_cursor':
  sdk.navigation.updateCursor(content); // New
  updateCursor(content); // Old (keep temporarily)
  break;

// Phase 3: Feature flag
case 'update_cursor':
  if (useNewArchitecture) {
    sdk.navigation.updateCursor(content);
  } else {
    updateCursor(content);
  }
  break;

// Phase 4: Remove atom, keep SDK only
case 'update_cursor':
  sdk.navigation.updateCursor(content);
  break;
```

### Step 3: Update Tests

1. Create mock SDK
2. Update tests to use mock SDK instead of atoms
3. Verify all message types tested

### Step 4: Remove Old Code

1. Remove direct atom imports from EventListener
2. Remove old hooks (useRunBamlTests)
3. Clean up dependencies

---

## Implementation Checklist

### SDK Updates
- [ ] **Add navigation methods** - `updateCursor`, `selectFunction`, `updateCursorFromRange`
- [ ] **Add files methods** - `update`, `watch`
- [ ] **Add settings methods** - `update`, `get`
- [ ] **Add info methods** - `setCliVersion`, `getCliVersion`
- [ ] **Update tests methods** - Ensure `run`, `runAll`, `cancel` work
- [ ] **Update executions methods** - Ensure `start`, `cancel` work

### EventListener Refactor
- [ ] **Add useBAMLSDK hook** - Get SDK instance
- [ ] **Create message handlers** - `handleIDEMessage`, `handleLSPMessage`, `handleWorkspaceCommand`
- [ ] **Update update_cursor** - Call `sdk.navigation.updateCursor()`
- [ ] **Update runtime_updated** - Call `sdk.files.update()` with debouncing
- [ ] **Update baml_settings_updated** - Call `sdk.settings.update()`
- [ ] **Update baml.openBamlPanel** - Call `sdk.navigation.selectFunction()`
- [ ] **Update baml.runBamlTest** - Call `sdk.tests.run()`
- [ ] **Update baml.executeWorkflow** - Call `sdk.executions.start()`
- [ ] **Update textDocument/codeAction** - Call `sdk.navigation.updateCursorFromRange()`
- [ ] **Add error handling** - Try-catch around all SDK calls
- [ ] **Preserve debouncing** - Keep 50ms debounce for file updates
- [ ] **Preserve JetBrains delay** - Keep 1s delay for initialization
- [ ] **Preserve WebSocket** - Keep WebSocket fallback for non-VSCode

### Testing
- [ ] **Create mock SDK** - For unit tests
- [ ] **Test each message type** - Verify SDK method called
- [ ] **Test debouncing** - Verify file updates debounced
- [ ] **Test error handling** - Verify errors don't crash EventListener
- [ ] **Integration tests** - Test with real SDK and mock provider
- [ ] **Manual testing** - Test in VSCode extension
- [ ] **Regression tests** - Ensure no breaking changes

### Documentation
- [ ] **Document message mapping** - Complete table of messages → SDK methods
- [ ] **Document platform quirks** - Debouncing, delays, WebSocket
- [ ] **Update EventListener comments** - Explain new architecture
- [ ] **Add SDK method documentation** - JSDoc for all new methods

---

## Validation Criteria

### Functional Requirements
- [ ] VSCode cursor updates trigger SDK navigation
- [ ] File updates from LSP trigger WASM compilation
- [ ] Test execution commands work
- [ ] Workflow execution commands work
- [ ] Settings updates work
- [ ] Function selection works
- [ ] Platform-specific delays preserved
- [ ] Debouncing works correctly

### Architecture
- [ ] No direct atom access in EventListener (except non-core state)
- [ ] All messages handled correctly
- [ ] Error handling works
- [ ] Single SDK dependency
- [ ] Message handlers are pure functions

### Testing
- [ ] All message types have unit tests
- [ ] Mock SDK works correctly
- [ ] Integration tests pass
- [ ] Manual testing in VSCode passes
- [ ] No regressions

### Performance
- [ ] No performance degradation
- [ ] Debouncing reduces WASM compilation
- [ ] Message handling is fast (<10ms)

---

## Risk Mitigation

### High-Risk Areas

**1. Breaking VSCode Integration**
- Risk: Messages not handled correctly, features broken
- Mitigation: Gradual refactor, feature flag, extensive testing

**2. State Synchronization Issues**
- Risk: SDK and atoms out of sync
- Mitigation: SDK methods update atoms directly, no duplication

**3. Platform Quirks Lost**
- Risk: Debouncing or delays removed by accident
- Mitigation: Document all quirks, keep them in EventListener

### Testing Strategy

1. **Unit Tests** - Test each message handler
2. **Integration Tests** - Test with real SDK
3. **Manual Testing** - Test in VSCode
4. **Regression Tests** - Ensure no breaking changes

---

## Success Metrics

- [ ] EventListener refactored (~200 lines → ~150 lines)
- [ ] Zero direct atom updates (except non-core state)
- [ ] All SDK methods implemented
- [ ] All message types handled
- [ ] Test coverage > 90%
- [ ] Zero regressions
- [ ] Backward compatibility maintained

---

**Last Updated**: 2025-11-04
**Status**: Ready for implementation
**Estimated Effort**: 2-3 days for experienced developer
**Dependencies**: Phase 4 (Execution Engine) must be complete
