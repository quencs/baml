# @baml/playground-common

A clean, flat React architecture for BAML playground components and utilities.

## 🎯 **Refactored Architecture**

This package has been completely refactored from a deeply nested structure with 50+ scattered Jotai atoms to a **flat, maintainable React architecture**.

### **Directory Structure**

```
src/
├── components/           # All React components (flat)
│   ├── app-root.tsx              # Main app composition
│   ├── vscode-handler.tsx        # VSCode integration
│   ├── runtime-initializer.tsx   # WASM initialization
│   ├── status-bar.tsx            # Status display
│   ├── error-count.tsx           # Error UI
│   ├── test-*.tsx               # Test-related components
│   └── ...                      # Other UI components
├── hooks/                # Custom React hooks (flat)
│   ├── use-test-runner.ts        # Test execution interface
│   └── use-vscode.ts             # VSCode integration hook
├── contexts/             # React contexts (replacing 50+ atoms)
│   ├── runtime-context.tsx       # Runtime state management
│   └── test-context.tsx          # Test state management
├── services/             # Business logic classes (flat)
│   └── test-service.ts           # Test execution logic
├── utils/                # Pure utilities (flat)
│   ├── file-utils.ts             # File operations
│   ├── format-utils.ts           # Display formatting
│   └── ...                      # Other utilities
├── types.ts              # All TypeScript interfaces
└── index.ts              # Clean barrel exports
```

## 🚀 **Usage**

### **Basic Setup**

```typescript
import { AppRoot } from '@baml/playground-common';

function App() {
  return (
    <AppRoot>
      {/* Your app content */}
    </AppRoot>
  );
}
```

### **Using Contexts**

```typescript
import { useRuntime, useTest } from '@baml/playground-common';

function MyComponent() {
  const { state: runtimeState } = useRuntime();
  const { state: testState } = useTest();
  
  // Clean, predictable state access
  if (!runtimeState.isReady) {
    return <div>Loading WASM runtime...</div>;
  }
  
  return (
    <div>
      <p>Runtime ready with {runtimeState.diagnostics.length} diagnostics</p>
      <p>Tests: {testState.history.length} runs in history</p>
    </div>
  );
}
```

### **Running Tests**

```typescript
import { useTestRunner, TestService } from '@baml/playground-common';

function TestRunner() {
  const { runTests, isRunning } = useTestRunner();
  
  const handleRunTests = async () => {
    const tests = [
      { functionName: 'MyFunction', testName: 'test1' },
      { functionName: 'MyFunction', testName: 'test2' }
    ];
    
    await runTests(tests);
  };
  
  return (
    <button onClick={handleRunTests} disabled={isRunning}>
      {isRunning ? 'Running...' : 'Run Tests'}
    </button>
  );
}
```

### **Direct Service Usage**

```typescript
import { TestService } from '@baml/playground-common';

// For direct control over test execution
const results = await TestService.runParallelTests(runtime, tests);
const singleResult = await TestService.runTest(runtime, testCase);
```

## 📊 **Migration Benefits**

| **Metric** | **Before** | **After** | **Improvement** |
|------------|------------|-----------|-----------------|
| Directory Nesting | 6 levels deep | 2 levels max | **70% reduction** |
| Jotai Atoms | 50+ scattered | 2 React contexts | **90% reduction** |
| Import Paths | 60+ characters | <30 characters | **50% shorter** |
| Largest Component | 464 lines | <200 lines | **Clear boundaries** |

### **Before:**
```typescript
// ❌ Deeply nested nightmare
import { SimpleTestResultView } from '../shared/baml-project-panel/playground-panel/prompt-preview/test-panel/components/SimpleTestResultView'
```

### **After:**
```typescript
// ✅ Clean flat imports
import { TestResultView } from '../components/test-result-view'
import { useTestRunner } from '../hooks/use-test-runner'
import { TestService } from '../services/test-service'
```

## 🔧 **Components**

### **Core Components**
- `AppRoot` - Main app composition with providers
- `VSCodeHandler` - Pure VSCode message integration
- `RuntimeInitializer` - WASM runtime initialization
- `StatusBar` - Bottom status display
- `ErrorCount` - Error/warning indicator

### **Test Components**
- `TestPanel` - Main test interface
- `TestResultView` - Test result display
- `TestTabularView` - Tabular test results
- `TestMenu` - Test control menu
- `TestStatus` - Test status indicator

### **Utility Components**
- `CodeMirrorViewer` - Code editor component
- `MarkdownRenderer` - Markdown display
- `ResponseRenderer` - Response formatting

## 🎣 **Hooks**

### **Test Hooks**
- `useTestRunner()` - Test execution interface
- `useTest()` - Test state management
- `useTestActions()` - Test actions
- `useCurrentTestRun()` - Current test run data

### **Runtime Hooks**
- `useRuntime()` - Runtime state management
- `useRuntimeState()` - Runtime state access
- `useRuntimeActions()` - Runtime actions

### **Integration Hooks**
- `useVSCode()` - VSCode integration

## 🛠️ **Services**

### **TestService**
Static class for test execution:
- `runTest(runtime, testCase)` - Run single test
- `runParallelTests(runtime, tests)` - Run multiple tests
- `getAvailableTests(runtime)` - Get all available tests
- `getTestsForFunction(runtime, functionName)` - Get tests for function

## 🔄 **Migration Guide**

### **Replacing Atoms with Contexts**

```typescript
// ❌ Before: Jotai atoms
const wasm = useAtomValue(wasmAtom);
const diagnostics = useAtomValue(diagnosticsAtom);
const [selectedFunc, setSelectedFunc] = useAtom(selectedFunctionAtom);

// ✅ After: React contexts
const { state } = useRuntime();
const { wasm, diagnostics } = state;
const { state: testState, dispatch } = useTest();
```

### **Replacing EventListener**

```typescript
// ❌ Before: 288-line god component
<EventListener>
  <YourApp />
</EventListener>

// ✅ After: Clean composition
<AppRoot>
  <YourApp />
</AppRoot>
```

## 📖 **Development**

```bash
# Install dependencies
pnpm install

# Development mode
pnpm dev

# Type checking
pnpm typecheck

# Clean build artifacts
pnpm clean
```

---

**This package now follows modern React best practices with clear separation of concerns, making it much easier to understand, maintain, and extend!** 🎉
