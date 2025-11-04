# BAML Graphs Architecture Integration Plan

**Status:** Design Proposal
**Author:** Claude Code
**Date:** 2025-11-04
**Purpose:** Design document for merging `apps/baml-graph` with `packages/playground-common`

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Current State Analysis](#current-state-analysis)
3. [Key Architectural Differences](#key-architectural-differences)
4. [Integration Strategy](#integration-strategy)
5. [Unified State Management](#unified-state-management)
6. [SDK vs EventListener Architecture](#sdk-vs-eventlistener-architecture)
7. [Cursor to CodeClick Unification](#cursor-to-codeclick-unification)
8. [Mock Data & Testing Strategy](#mock-data--testing-strategy)
9. [Migration Plan](#migration-plan)
10. [Implementation Phases](#implementation-phases)
11. [Risk Assessment](#risk-assessment)

---

## Executive Summary

This document outlines a comprehensive plan to merge the new `apps/baml-graph` application with the existing `packages/playground-common` infrastructure. The goal is to:

1. **Unify state management** - Consolidate Jotai atoms from both applications into a coherent structure
2. **Adopt SDK architecture** - Replace EventListener pattern with BAMLSDK for better separation of concerns
3. **Preserve testing capabilities** - Maintain baml-graph's mock data testing infrastructure with UI toggle
4. **Enable standalone development** - Allow browser-based testing of full projects without VSCode

### Key Findings

**baml-graph Strengths:**
- Clean SDK architecture with namespaced APIs (`apps/baml-graph/src/sdk/index.ts:40-508`)
- Domain-organized state management (4 focused atom files)
- Sophisticated mock data infrastructure with realistic execution simulation
- Performance-optimized with `atomFamily` patterns
- Debug panel for testing navigation heuristics

**playground-common Strengths:**
- Battle-tested VSCode integration with RPC protocol
- Full WASM runtime integration for compilation
- Comprehensive test execution with parallel support
- Rich UI components for test visualization
- Extensive API key management

### Proposed Solution

**Hybrid Architecture:**
1. Adopt BAMLSDK as the core abstraction layer
2. Keep EventListener as a thin bridge for IDE communication
3. Consolidate atoms into domain-organized structure following baml-graph pattern
4. Add development mode toggle for mock data testing
5. Unify execution models to support both workflow and function execution

---

## Current State Analysis

### apps/baml-graph Structure

**Location:** `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph`

#### State Management (Jotai Atoms)

**Workflow Atoms** (`src/sdk/atoms/workflow.atoms.ts:17-38`):
```typescript
workflowsAtom              // All workflow definitions
activeWorkflowIdAtom       // Currently selected workflow ID
activeWorkflowAtom         // Derived active workflow
recentWorkflowsAtom       // Recent workflow access tracking
```

**Execution Atoms** (`src/sdk/atoms/execution.atoms.ts:27-205`):
```typescript
// Execution tracking
workflowExecutionsAtomFamily(workflowId)  // Per-workflow executions
selectedExecutionIdAtom                    // Current execution view
nodeStateAtomFamily(nodeId)                // Per-node execution states

// Event streaming
eventStreamAtom            // Last 100 events circular buffer
addEventAtom              // Write-only event emitter

// Caching
cacheAtom                 // Execution cache by ${nodeId}:${inputsHash}
```

**UI Atoms** (`src/sdk/atoms/ui.atoms.ts:17-93`):
```typescript
viewModeAtom              // 'editor' | 'execution' snapshot mode
selectedNodeIdAtom        // Selected graph node
detailPanelAtom          // Panel state (open, position, activeTab)
layoutDirectionAtom      // 'horizontal' | 'vertical'
selectedInputSourceAtom  // Input source selection
bamlFilesAtom           // All BAML files with functions/tests
activeCodeClickAtom     // Code click events for navigation
```

**Derived Atoms** (`src/sdk/atoms/derived.atoms.ts:26-179`):
```typescript
allFunctionsMapAtom       // O(1) function lookup Map
functionsByTypeAtom       // Functions grouped by type
isLLMOnlyModeAtom        // Computed LLM-only mode flag
```

#### BAMLSDK Architecture

**Main SDK Class** (`src/sdk/index.ts:40-508`):

```typescript
class BAMLSDK {
  // Namespaced APIs
  workflows: {
    getAll(): WorkflowDefinition[]
    getById(id): WorkflowDefinition | null
    getActive(): WorkflowDefinition | null
    setActive(id): void
  }

  executions: {
    start(workflowId, inputs, opts): string
    cancel(executionId): void
    getExecutions(workflowId): ExecutionSnapshot[]
    getExecution(executionId): ExecutionSnapshot | null
  }

  graph: {
    getGraph(): WorkflowGraph | null
    updateNodePosition(nodeId, position): void
  }

  cache: {
    get(key): unknown
    set(key, value): void
    clear(nodeId?): void
  }

  testCases: {
    get(nodeId): TestCase[]
  }
}
```

**Responsibilities:**
- **Execution orchestration** (lines 120-179): Manages workflow lifecycle
- **State synchronization** (lines 355-495): Updates atoms during execution
- **Event emission** (lines 334-349): Publishes comprehensive event stream
- **Mock data simulation** (via `DefaultMockProvider`, `src/sdk/mock.ts:93-1094`)

#### Mock Data Infrastructure

**Debug Panel** (`src/features/debug/components/DebugPanel.tsx:1-203`):
- Interactive file browser simulating IDE clicks
- Function and test lists with click handlers
- Emits `CodeClickEvent` to test navigation heuristics
- Validates mock data at startup

**Navigation Heuristic** (`src/sdk/navigationHeuristic.ts:1-300`):
- Priority-based decision tree for code clicks
- Handles test clicks, function clicks, workflow switching
- Context-aware: stays in current workflow when possible

**Mock Provider** (`src/sdk/mock.ts:93-1094`):
- Sample workflows: simple, conditional, shared
- Per-node test cases with inputs/outputs
- Async execution simulation with configurable timing
- Realistic LLM response generation
- Configurable cache hit rate and error rate

---

### packages/playground-common Structure

**Location:** `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common`

#### State Management (Jotai Atoms)

**Core Runtime Atoms** (`src/shared/baml-project-panel/atoms.ts:121-339`):
```typescript
// WASM integration
wasmAtom                  // WASM module instance
wasmPanicAtom            // Panic tracking with timestamp
filesAtom                // File path → content map
projectAtom              // WASM project from files
runtimeAtom              // WASM runtime with diagnostics
diagnosticsAtom          // Compilation errors/warnings
generatedFilesAtom       // Generated code files

// VSCode integration
vscodeSettingsAtom       // VSCode settings via RPC
proxyUrlAtom            // Playground proxy config
```

**Playground Atoms** (`src/shared/baml-project-panel/playground-panel/atoms.ts:13-264`):
```typescript
// Selection
selectedFunctionAtom      // Selected function name string
selectedTestcaseAtom     // Selected test name string
updateCursorAtom         // Write-only cursor update

// Test execution
runningTestsAtom         // Array of test states
areTestsRunningAtom      // Boolean flag
testCaseResponseAtom(fn, test)  // Per-test responses (atomFamily)
flashRangesAtom         // Code highlight ranges
```

**Test History Atoms** (`src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/atoms.ts:21-114`):
```typescript
testHistoryAtom          // Array of test runs
selectedHistoryIndexAtom // Current history view
isParallelTestsEnabledAtom  // Parallel execution flag
currentWatchNotificationsAtom  // Watch notifications
highlightedBlocksAtom    // Set of highlighted blocks
```

**API Keys Atoms** (`src/components/api-keys-dialog/atoms.ts:8-459`):
- 25+ atoms for comprehensive API key management
- Storage persistence with VSCode adapter
- Required vs optional key tracking
- Proxy integration logic

#### EventListener Architecture

**Main Component** (`src/baml_wasm_web/EventListener.tsx:57-217`):

**Pattern:** React component listening to IDE messages

```typescript
function EventListener() {
  // Hook-based state access
  const [bamlFileMap, setBamlFileMap] = useAtom(filesAtom)
  const setSelectedFunction = useSetAtom(selectedFunctionAtom)
  const updateCursor = useSetAtom(updateCursorAtom)

  // Message handler (lines 110-210)
  useEffect(() => {
    const handler = (event) => {
      if (event.data.source === 'ide_message') {
        // Handle IDE commands
        switch (event.data.command) {
          case 'update_cursor': updateCursor(content); break;
          case 'baml_settings_updated': setBamlConfig(content); break;
        }
      } else if (event.data.source === 'lsp_message') {
        // Handle LSP notifications
        switch (event.data.method) {
          case 'runtime_updated': setBamlFileMap(files); break;
          case 'workspace/executeCommand':
            if (args.command === 'baml.runBamlTest') {
              runBamlTests([{functionName, testName}]);
            }
            break;
        }
      }
    };
    window.addEventListener('message', handler);
    return () => window.removeEventListener('message', handler);
  }, [dependencies]);

  return null;  // No render
}
```

**Responsibilities:**
- File synchronization from LSP (lines 147-155)
- Cursor tracking (line 120)
- Settings management (lines 122-145)
- Test execution triggering (lines 165-178)
- WebSocket fallback for non-VSCode (lines 89-108)

#### VSCode Integration

**VSCode API Wrapper** (`src/shared/baml-project-panel/vscode.ts:1-535`):
- RPC implementation with request/response correlation (lines 398-440)
- Platform detection (VSCode vs JetBrains vs Zed) (lines 150-152)
- File navigation (lines 154-183)
- Credential loading (AWS/GCP) (lines 285-300)
- Settings management (lines 257-283)

**Message Protocol** (`src/baml_wasm_web/vscode-to-webview-rpc.ts:4-88`):
```typescript
// IDE → Webview messages
type VSCodeToWebviewMessage =
  | { source: 'ide_message', command: 'update_cursor' | 'baml_settings_updated' }
  | { source: 'lsp_message', method: 'runtime_updated' | 'workspace/executeCommand' }
```

**Extension Integration** (`apps/vscode-ext/src/panels/WebviewPanelHost.ts:136-772`):
- Webview HTML generation (lines 274-406)
- Message listener (lines 415-772)
- RPC handling (lines 720-737)
- Pending command queue (lines 137-145)

---

## Key Architectural Differences

### Execution Paradigm

| Aspect | baml-graph | playground-common |
|--------|-----------|------------------|
| **Unit of execution** | Workflow with nodes | Individual function tests |
| **Execution model** | Workflow → Nodes (hierarchical) | Function → Tests (flat) |
| **State tracking** | Per-node via `atomFamily` | Per-test via `atomFamily` |
| **History** | Execution snapshots with frozen graphs | Test history with timestamps |
| **Concurrency** | Sequential node execution | Parallel test execution support |

### State Organization

| Aspect | baml-graph | playground-common |
|--------|-----------|------------------|
| **Total atoms** | ~35 atoms | ~70+ atoms |
| **Organization** | Domain-driven (4 files) | Feature-driven (13+ files) |
| **File structure** | `sdk/atoms/{domain}.atoms.ts` | Scattered across features |
| **Performance** | Optimized with `atomFamily`, Maps | Functional approach |
| **Persistence** | Minimal | Heavy (10+ persisted atoms) |

### Abstraction Layer

| Aspect | baml-graph | playground-common |
|--------|-----------|------------------|
| **API** | BAMLSDK class with namespaces | Direct atom manipulation |
| **Event system** | Comprehensive event emission | No event emission |
| **Testing** | Easy (class-based) | Difficult (React hooks) |
| **Reusability** | Platform-agnostic | React-specific |
| **Lifecycle** | Independent | React component lifecycle |

### Runtime Integration

| Aspect | baml-graph | playground-common |
|--------|-----------|------------------|
| **WASM** | External (assumes pre-parsed) | Direct integration |
| **Compilation** | Not handled | Full pipeline with diagnostics |
| **File editing** | Read-only | Full editing support |
| **Code generation** | Not present | Multiple languages |

---

## Integration Strategy

### Guiding Principles

1. **Preserve Strengths**: Keep what works well in each system
2. **Minimize Disruption**: Incremental migration path
3. **Improve Developer Experience**: Better testing and debugging
4. **Maintain Compatibility**: VSCode extension continues to work throughout migration

### High-Level Approach

```
┌─────────────────────────────────────────────────────────────┐
│                        playground-common                     │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │              BAMLSDK (Core Abstraction)            │    │
│  │  - Workflow management                              │    │
│  │  - Execution orchestration                         │    │
│  │  - State synchronization                           │    │
│  │  - Event emission                                  │    │
│  │  - Mock data provider (dev mode)                   │    │
│  └─────────────────┬──────────────────────────────────┘    │
│                    │                                         │
│  ┌─────────────────▼──────────────────────────────────┐    │
│  │         Unified Jotai Atoms (Domain-Organized)     │    │
│  │  - workflow.atoms.ts                               │    │
│  │  - execution.atoms.ts                              │    │
│  │  - runtime.atoms.ts (WASM integration)             │    │
│  │  - ui.atoms.ts                                     │    │
│  │  - derived.atoms.ts                                │    │
│  └─────────────────┬──────────────────────────────────┘    │
│                    │                                         │
│  ┌─────────────────▼──────────────────────────────────┐    │
│  │  EventListener (IDE Communication Bridge)          │    │
│  │  - Receives IDE messages                           │    │
│  │  - Calls SDK methods                               │    │
│  │  - Subscribes to SDK events                        │    │
│  │  - Posts updates to IDE                            │    │
│  └────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │  UI Components       │
                    │  - PromptPreview     │
                    │  - WorkflowGraph     │
                    │  - DetailPanel       │
                    │  - DebugPanel (dev)  │
                    └──────────────────────┘
```

### Mode-Based Architecture

The unified playground will support multiple modes:

```typescript
type PlaygroundMode =
  | 'vscode'      // VSCode webview (production)
  | 'standalone'  // Browser with WebSocket (development)
  | 'mock'        // Browser with mock data (testing)
```

**Mode Detection** (proposed location: `packages/playground-common/src/shared/mode.ts`):
```typescript
function detectPlaygroundMode(): PlaygroundMode {
  if (typeof acquireVsCodeApi !== 'undefined') {
    return 'vscode';
  }

  // Check for dev mode via env variable or URL param
  const urlParams = new URLSearchParams(window.location.search);
  const useMock = urlParams.get('mock') === 'true' ||
                  import.meta.env.VITE_MOCK_MODE === 'true';

  return useMock ? 'mock' : 'standalone';
}
```

---

## Unified State Management

### Proposed Atom Structure

**New Location:** `packages/playground-common/src/shared/atoms/`

```
src/shared/atoms/
├── index.ts                 # Main export file
├── workflow.atoms.ts        # Workflow definitions and selection
├── execution.atoms.ts       # Execution state and history
├── runtime.atoms.ts         # WASM runtime and compilation
├── ui.atoms.ts             # UI state and interactions
├── derived.atoms.ts        # Computed/optimized atoms
├── api-keys.atoms.ts       # API key management (existing)
└── legacy.atoms.ts         # Deprecated atoms during migration
```

### workflow.atoms.ts

**Consolidates:**
- `apps/baml-graph/src/sdk/atoms/workflow.atoms.ts` (entire file)
- `packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:13-32` (runtime state)

```typescript
// From baml-graph
export const workflowsAtom = atom<WorkflowDefinition[]>([])
export const activeWorkflowIdAtom = atom<string | null>(null)
export const activeWorkflowAtom = atom<WorkflowDefinition | null>(get => {
  const workflows = get(workflowsAtom)
  const activeId = get(activeWorkflowIdAtom)
  return workflows.find(w => w.id === activeId) ?? null
})
export const recentWorkflowsAtom = atom<string[]>([])

// Extended for playground-common compatibility
export const selectedFunctionAtom = atom<string | null>(
  get => get(selectedNodeIdAtom),  // Bridge to node selection
  (get, set, functionName: string | null) => {
    // Find node ID for function name
    const allFunctions = get(allFunctionsMapAtom)
    const func = allFunctions.get(functionName ?? '')
    if (func?.nodeId) {
      set(selectedNodeIdAtom, func.nodeId)
    }
  }
)

export const selectedTestcaseAtom = atom<string | null>(null)
```

### execution.atoms.ts

**Consolidates:**
- `apps/baml-graph/src/sdk/atoms/execution.atoms.ts` (entire file)
- `packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:179-264` (test execution state)
- `packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/atoms.ts:21-114` (test history)

```typescript
// Workflow execution (from baml-graph)
export const workflowExecutionsAtomFamily = atomFamily((workflowId: string) =>
  atom<ExecutionSnapshot[]>([])
)

export const selectedExecutionIdAtom = atom<string | null>(null)

export const nodeStateAtomFamily = atomFamily((nodeId: string) =>
  atom<NodeState>('not-started')
)

export const nodeExecutionsAtom = atom<Map<string, NodeExecution>>(new Map())

// Event streaming (from baml-graph)
export const eventStreamAtom = atom<BAMLEvent[]>([])
export const addEventAtom = atom(null, (get, set, event: BAMLEvent) => {
  const events = get(eventStreamAtom)
  set(eventStreamAtom, [...events, event].slice(-100))
})

// Cache (from baml-graph)
export const cacheAtom = atom<Map<string, unknown>>(new Map())

// Test execution (from playground-common, adapted)
export const testExecutionStateAtom = atom<{
  running: Array<{functionName: string, testName: string, state: TestState}>
  history: Array<TestHistoryEntry>
}>({
  running: [],
  history: []
})

export const areTestsRunningAtom = atom(get =>
  get(testExecutionStateAtom).running.length > 0
)

export const currentAbortControllerAtom = atom<AbortController | null>(null)

// Watch notifications (from playground-common)
export const currentWatchNotificationsAtom = atom<WatchNotification[]>([])
export const highlightedBlocksAtom = atom<Set<string>>(new Set())
```

### runtime.atoms.ts

**Consolidates:**
- `packages/playground-common/src/shared/baml-project-panel/atoms.ts:121-339` (WASM runtime)
- Adds mode-based behavior

```typescript
// WASM panic handling
export const wasmPanicAtom = atom<{ message: string; timestamp: number } | null>(null)

// WASM loading
export const wasmAtomAsync = atom(async () => {
  const mode = detectPlaygroundMode()
  if (mode === 'mock') {
    // Return mock WASM stub
    return createMockWasm()
  }
  // Load real WASM
  return await loadWasm()
})

export const wasmAtom = unwrap(wasmAtomAsync, prev => prev ?? null)

// Files and project
export const filesAtom = atom<Record<string, string>>({})
export const sandboxFilesAtom = atom<Record<string, string>>({})

export const projectAtom = atom(get => {
  const wasm = get(wasmAtom)
  const files = get(filesAtom)
  if (!wasm) return null
  return wasm.createProject(files)
})

// Runtime with compilation
export const runtimeAtom = atom(get => {
  const project = get(projectAtom)
  const config = get(vscodeSettingsAtom)
  if (!project) return null

  const runtime = project.createRuntime({
    envVars: get(apiKeysAtom),
    featureFlags: config.betaFlags
  })

  return {
    runtime,
    diagnostics: runtime.getDiagnostics(),
    lastValidRt: runtime.isValid() ? runtime : get(runtimeAtom)?.lastValidRt
  }
})

export const diagnosticsAtom = atom(get => {
  const rt = get(runtimeAtom)
  return rt?.diagnostics ?? []
})

// Generated files
export const generatedFilesAtom = atom(get => {
  const rt = get(runtimeAtom)
  if (!rt?.lastValidRt) return []
  return rt.lastValidRt.generateFiles()
})

// VSCode settings (async RPC)
export const vscodeSettingsAtom = atom(async () => {
  const mode = detectPlaygroundMode()
  if (mode === 'mock') {
    return DEFAULT_MOCK_SETTINGS
  }
  return await vscode.getVSCodeSettings()
})

// Proxy configuration
export const proxyUrlAtom = atom(get => {
  const settings = get(vscodeSettingsAtom)
  const port = settings.playgroundPort
  return {
    url: port ? `http://localhost:${port}` : null,
    enabled: settings.useProxy
  }
})
```

### ui.atoms.ts

**Consolidates:**
- `apps/baml-graph/src/sdk/atoms/ui.atoms.ts` (entire file)
- `packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:263` (flash ranges)
- `packages/playground-common/src/components/status-bar.tsx` related state
- Sidebar and panel state atoms

```typescript
// View modes
export const viewModeAtom = atom<'editor' | 'execution'>('editor')
export const playgroundModeAtom = atom<PlaygroundMode>(detectPlaygroundMode())

// Node/function selection
export const selectedNodeIdAtom = atom<string | null>(null)

// Panel state
export const detailPanelAtom = atom<{
  isOpen: boolean
  position: 'right' | 'bottom'
  activeTab: 'inputs' | 'outputs' | 'logs' | 'llm-request' | 'llm-response'
}>({
  isOpen: false,
  position: 'right',
  activeTab: 'inputs'
})

export const isSidebarOpenAtom = atomWithStorage('playground:sidebarOpen', true)
export const isPanelVisibleAtom = atom<boolean>(true)

// Layout
export const layoutDirectionAtom = atom<'horizontal' | 'vertical'>('horizontal')

// Input management
export const selectedInputSourceAtom = atom<InputSource | null>(null)
export const activeNodeInputsAtom = atom<Record<string, unknown>>({})
export const inputsDirtyAtom = atom<boolean>(false)

// Code interaction
export const activeCodeClickAtom = atom<CodeClickEvent | null>(null)
export const flashRangesAtom = atom<Array<{start: number, end: number}>>([])

// Dev mode UI
export const showDebugPanelAtom = atomWithStorage(
  'playground:showDebugPanel',
  import.meta.env.DEV
)

// Media display (from playground-common)
export const imageStatsMapAtom = atom<Map<string, ImageStats>>(new Map())
export const mediaCollapsedMapAtom = atom<Map<string, boolean>>(new Map())

// Test panel view (from playground-common)
export const testPanelViewTypeAtom = atomWithStorage<'tabular' | 'card_expanded' | 'card_simple'>(
  'playground:testPanelView',
  'tabular'
)
export const tabularViewConfigAtom = atomWithStorage('playground:tabularViewConfig', {})
```

### derived.atoms.ts

**Consolidates:**
- `apps/baml-graph/src/sdk/atoms/derived.atoms.ts` (entire file)
- Selection-related derived atoms from playground-common

```typescript
// Function lookup optimization (from baml-graph)
export const allFunctionsMapAtom = atom(get => {
  const files = get(bamlFilesAtom)
  const map = new Map<string, BAMLFunction>()
  for (const file of files) {
    for (const func of file.functions) {
      map.set(func.name, { ...func, filePath: file.path })
    }
  }
  return map
})

export const functionsByTypeAtom = atom(get => {
  const functions = Array.from(get(allFunctionsMapAtom).values())
  return {
    llm: functions.filter(f => f.type === 'llm_function'),
    client: functions.filter(f => f.type === 'client'),
    enum: functions.filter(f => f.type === 'enum'),
    class: functions.filter(f => f.type === 'class'),
    retry_policy: functions.filter(f => f.type === 'retry_policy')
  }
})

// Workflow membership (from baml-graph)
export const workflowFunctionIdsAtom = atom(get => {
  const workflows = get(workflowsAtom)
  const ids = new Set<string>()
  for (const workflow of workflows) {
    for (const node of workflow.graph.nodes) {
      ids.add(node.id)
    }
  }
  return ids
})

export const standaloneFunctionsAtom = atom(get => {
  const allFunctions = Array.from(get(allFunctionsMapAtom).values())
  const workflowIds = get(workflowFunctionIdsAtom)
  return allFunctions.filter(f => !workflowIds.has(f.name))
})

// Selection state (from baml-graph + playground-common)
export const selectedFunctionAtom = atom(get => {
  const nodeId = get(selectedNodeIdAtom)
  const functionsMap = get(allFunctionsMapAtom)
  return functionsMap.get(nodeId ?? '') ?? null
})

export const isSelectedFunctionStandaloneAtom = atom(get => {
  const selected = get(selectedFunctionAtom)
  if (!selected) return false
  const workflowIds = get(workflowFunctionIdsAtom)
  return !workflowIds.has(selected.name)
})

// LLM-only mode (from baml-graph)
export const isLLMOnlyModeAtom = atom(get => {
  const selected = get(selectedFunctionAtom)
  const isStandalone = get(isSelectedFunctionStandaloneAtom)
  return (
    selected !== null &&
    selected.type === 'llm_function' &&
    isStandalone
  )
})

// Selection with test case (from playground-common)
export const selectionAtom = atom(get => {
  const functionName = get(selectedFunctionAtom)?.name
  const testName = get(selectedTestcaseAtom)
  const functionsMap = get(allFunctionsMapAtom)

  return {
    function: functionName ? functionsMap.get(functionName) : null,
    testCase: testName ? { name: testName } : null  // TODO: get full test object
  }
})
```

### Migration Benefits

1. **Reduced Complexity**: 70+ atoms → ~50 atoms in organized structure
2. **Better Performance**: atomFamily patterns prevent unnecessary re-renders
3. **Easier Maintenance**: Domain organization makes changes localized
4. **Type Safety**: Consolidated types reduce duplication
5. **Testability**: Clear boundaries between concerns

---

## SDK vs EventListener Architecture

### Proposed: BAMLSDK as Core Layer

**Location:** `packages/playground-common/src/sdk/` (moved from apps/baml-graph)

#### SDK Interface (Enhanced)

```typescript
interface BAMLSDKConfig {
  mode: 'vscode' | 'standalone' | 'mock'
  mockData?: MockDataProvider
  wasmProvider?: WasmProvider  // NEW: Inject WASM runtime
}

class BAMLSDK {
  constructor(config: BAMLSDKConfig, store?: Store)

  // Existing from baml-graph
  workflows: { /* ... */ }
  executions: { /* ... */ }
  graph: { /* ... */ }
  cache: { /* ... */ }
  testCases: { /* ... */ }

  // NEW: WASM runtime integration
  runtime: {
    getFiles(): Record<string, string>
    setFiles(files: Record<string, string>): void
    getProject(): WasmProject | null
    getRuntime(): WasmRuntime | null
    getDiagnostics(): Diagnostic[]
    generateFiles(language?: string): GeneratedFile[]
  }

  // NEW: Test execution (function-level)
  tests: {
    run(args: {
      functionName: string
      testName: string
      inputs?: Record<string, unknown>
      parallel?: boolean
    }): Promise<TestResult>

    runAll(functionName?: string): Promise<TestResult[]>
    cancel(): void
    getHistory(): TestHistoryEntry[]
  }

  // NEW: Settings management
  settings: {
    get(): Promise<VSCodeSettings>
    update(partial: Partial<VSCodeSettings>): void
    getApiKeys(): ApiKeys
    setApiKeys(keys: ApiKeys): void
  }

  // Existing event system
  onEvent(callback: (event: BAMLEvent) => void): () => void
  dispose(): void
}
```

#### SDK Implementation Strategy

**Phase 1:** Extend existing SDK with WASM integration

**File:** `packages/playground-common/src/sdk/index.ts`

```typescript
export class BAMLSDK {
  private wasmProvider: WasmProvider | null = null

  constructor(config: BAMLSDKConfig, store?: Store) {
    // Existing initialization...

    if (config.wasmProvider) {
      this.wasmProvider = config.wasmProvider
      this.initializeWasmRuntime()
    }
  }

  private initializeWasmRuntime() {
    // Subscribe to filesAtom changes
    this.store.sub(filesAtom, () => {
      const files = this.store.get(filesAtom)
      // Trigger WASM compilation
      this.compileProject(files)
    })
  }

  private compileProject(files: Record<string, string>) {
    // Update projectAtom and runtimeAtom through WASM provider
    // Extract workflows from compiled runtime
    const workflows = this.extractWorkflowsFromRuntime()
    this.store.set(workflowsAtom, workflows)

    // Emit events
    for (const workflow of workflows) {
      this.emitEvent({ type: 'workflow.discovered', workflow })
    }
  }

  // NEW: Runtime methods
  runtime = {
    getFiles: () => this.store.get(filesAtom),

    setFiles: (files: Record<string, string>) => {
      this.store.set(filesAtom, files)
      // Compilation happens automatically via subscription
    },

    getProject: () => this.store.get(projectAtom),

    getRuntime: () => this.store.get(runtimeAtom)?.runtime ?? null,

    getDiagnostics: () => this.store.get(diagnosticsAtom),

    generateFiles: (language?: string) => {
      const files = this.store.get(generatedFilesAtom)
      return language
        ? files.filter(f => f.language === language)
        : files
    }
  }

  // NEW: Test execution methods
  tests = {
    run: async (args) => {
      return await runSingleTest(
        this.store,
        this.wasmProvider,
        args
      )
    },

    runAll: async (functionName) => {
      const functions = functionName
        ? [this.store.get(allFunctionsMapAtom).get(functionName)]
        : Array.from(this.store.get(allFunctionsMapAtom).values())

      // Run tests with parallel support
      return await runBatchTests(this.store, this.wasmProvider, functions)
    },

    cancel: () => {
      const controller = this.store.get(currentAbortControllerAtom)
      controller?.abort()
    },

    getHistory: () => this.store.get(testExecutionStateAtom).history
  }

  // NEW: Settings methods
  settings = {
    get: async () => this.store.get(vscodeSettingsAtom),

    update: (partial) => {
      const current = this.store.get(vscodeSettingsAtom)
      this.store.set(vscodeSettingsAtom, { ...current, ...partial })

      // Notify VSCode extension
      if (this.config.mode === 'vscode') {
        vscode.setProxySettings(partial)
      }
    },

    getApiKeys: () => this.store.get(apiKeysAtom),

    setApiKeys: (keys) => {
      this.store.set(userApiKeysAtom, keys)
      this.store.set(saveApiKeyChangesAtom)
    }
  }
}
```

### EventListener as Thin Bridge

**Location:** `packages/playground-common/src/baml_wasm_web/EventListener.tsx`

**New Architecture:**

```typescript
interface EventListenerProps {
  sdk: BAMLSDK  // Inject SDK instance
}

export function EventListener({ sdk }: EventListenerProps) {
  // Message handler
  useEffect(() => {
    const handler = async (event: MessageEvent) => {
      const { source, command, method, content, args } = event.data

      if (source === 'ide_message') {
        switch (command) {
          case 'update_cursor':
            // Call SDK method instead of atom
            sdk.updateCursor(content)
            break

          case 'baml_settings_updated':
            await sdk.settings.update(content)
            break

          case 'baml_cli_version':
            // Update atom directly (non-core state)
            setBamlCliVersion(content)
            break
        }
      }
      else if (source === 'lsp_message') {
        switch (method) {
          case 'runtime_updated':
            // Update files through SDK
            sdk.runtime.setFiles(content)
            break

          case 'workspace/executeCommand':
            if (args.command === 'baml.openBamlPanel') {
              sdk.workflows.setActive(args.functionName)
              sdk.selectNode(args.functionName)
            }
            else if (args.command === 'baml.runBamlTest') {
              await sdk.tests.run({
                functionName: args.functionName,
                testName: args.testCaseName
              })
            }
            break

          case 'textDocument/codeAction':
            sdk.updateCursorFromRange(content.range)
            break
        }
      }
    }

    window.addEventListener('message', handler)
    return () => window.removeEventListener('message', handler)
  }, [sdk])

  // Subscribe to SDK events and post to IDE
  useEffect(() => {
    return sdk.onEvent((event) => {
      // Post important events back to IDE
      if (shouldNotifyIDE(event)) {
        vscode.postMessageToHost({
          type: 'baml.event',
          event
        })
      }
    })
  }, [sdk])

  return null
}
```

### Benefits of SDK Architecture

1. **Separation of Concerns**
   - SDK handles business logic
   - EventListener handles IDE communication
   - Atoms handle reactive state

2. **Testability**
   - SDK can be tested without React
   - Mock provider for unit tests
   - Integration tests via SDK API

3. **Reusability**
   - SDK can be used in CLI tools
   - Node.js scripts
   - Electron apps
   - Other IDEs (JetBrains, Zed)

4. **Type Safety**
   - Single source of truth for operations
   - Compile-time checking of SDK calls
   - Clear API contracts

5. **Event Tracing**
   - All operations emit events
   - Easy to add logging/telemetry
   - Debug event stream in UI

### Migration Path

**Phase 1: Introduce SDK alongside EventListener**
- Copy SDK from baml-graph to playground-common
- Add WASM integration to SDK
- EventListener continues to work with atoms
- Gradual migration of atom calls to SDK calls

**Phase 2: Refactor EventListener to use SDK**
- Inject SDK into EventListener
- Replace atom updates with SDK method calls
- Keep event subscription for IDE notifications

**Phase 3: Refactor components to use SDK**
- Replace `useAtomValue(workflowsAtom)` with `useBAMLSDK().workflows.getAll()`
- Custom hooks can wrap SDK methods
- Atoms become implementation detail

**Phase 4: Deprecate direct atom access**
- Mark atoms as `@internal`
- All external access goes through SDK
- Atoms only accessed by SDK internals

---

## Cursor to CodeClick Unification

### Problem Statement

Currently, there are two separate systems for handling code navigation:

1. **VSCode's `update_cursor`**: Low-level cursor position `{fileName, line, column}`
2. **baml-graph's `CodeClickEvent`**: High-level semantic events `{type, functionName, functionType, testName}`

These serve overlapping purposes but `update_cursor` lacks semantic richness needed for sophisticated navigation heuristics.

### Solution: Enrich Cursor Events with WASM Runtime

The key insight is that `playground-common` already has the infrastructure to derive semantic information from cursor positions via WASM runtime methods:

**Existing WASM Methods** (`packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:113-136`):
- `runtime.get_function_at_position(fileName, currentFunc, byteIndex)` - Find function at cursor
- `runtime.get_testcase_from_position(function, byteIndex)` - Find test case at cursor
- `runtime.get_function_of_testcase(fileName, byteIndex)` - Find which function a test is for

**Current usage:** `updateCursorAtom` uses these to set `selectedFunctionAtom` and `selectedTestcaseAtom`

**Proposed enhancement:** Use the same methods to create rich `CodeClickEvent` objects

### Unified Event Type

```typescript
export type CodeClickEvent =
  | {
      type: 'function';
      functionName: string;
      functionType: 'workflow' | 'function' | 'llm_function';
      filePath: string;
    }
  | {
      type: 'test';
      testName: string;
      functionName: string;  // Function being tested
      filePath: string;
      nodeType: 'llm_function' | 'function';
    };
```

### Implementation

**New File:** `packages/playground-common/src/shared/atoms/cursor-enrichment.ts`

```typescript
/**
 * Enriches a cursor position into a semantic CodeClickEvent using WASM runtime
 */
export function enrichCursorToCodeClick(
  cursor: { fileName: string; line: number; column: number },
  runtime: WasmRuntime,
  fileContent: string,
  currentSelectedFunction?: string
): CodeClickEvent | null {
  // 1. Convert line/column to byte index
  const cursorIdx = calculateByteIndex(fileContent, cursor.line, cursor.column);

  // 2. Get function at cursor position
  const selectedFunc = runtime.get_function_at_position(
    cursor.fileName,
    currentSelectedFunction ?? '',
    cursorIdx
  );

  if (!selectedFunc) return null;

  // 3. Check if cursor is within a test case
  const selectedTestcase = runtime.get_testcase_from_position(selectedFunc, cursorIdx);

  if (selectedTestcase) {
    const testedFunc = runtime.get_function_of_testcase(cursor.fileName, cursorIdx);
    return {
      type: 'test',
      testName: selectedTestcase.name,
      functionName: testedFunc?.name ?? selectedFunc.name,
      filePath: cursor.fileName,
      nodeType: selectedFunc.type === 'llm_function' ? 'llm_function' : 'function'
    };
  }

  // 4. Return function click event
  return {
    type: 'function',
    functionName: selectedFunc.name,
    functionType: determineWorkflowOrFunction(selectedFunc),
    filePath: cursor.fileName
  };
}
```

**Updated Atom:** `packages/playground-common/src/shared/atoms/ui.atoms.ts`

```typescript
// New: Single atom for all code click events
export const codeClickEventAtom = atom<CodeClickEvent | null>(null);

// Updated: Cursor atom creates CodeClickEvent via enrichment
export const updateCursorAtom = atom(
  null,
  (get, set, cursor: { fileName: string; line: number; column: number }) => {
    const runtime = get(runtimeAtom)?.runtime;
    const fileContent = get(filesAtom)[cursor.fileName];
    if (!runtime || !fileContent) return;

    const codeClickEvent = enrichCursorToCodeClick(
      cursor,
      runtime,
      fileContent,
      get(selectedFunctionAtom) ?? undefined
    );

    if (codeClickEvent) {
      set(codeClickEventAtom, codeClickEvent);
    }
  }
);
```

### Benefits

1. **Single Navigation Path**: Both VSCode cursors and debug panel clicks create `CodeClickEvent`
2. **Rich Semantics**: Every cursor movement enriched with function/test metadata
3. **Unified Heuristics**: Same navigation logic for all sources
4. **Better Testing**: Mock CodeClickEvents instead of cursor positions
5. **Extensible**: Easy to add metadata (containing class, line numbers, etc.)

### Event Flow Comparison

**Before (Separate Systems):**
```
VSCode cursor → update_cursor → updateCursorAtom → selectedFunctionAtom
Debug panel → (no cursor) → Direct atom updates
```

**After (Unified System):**
```
VSCode cursor → update_cursor → enrichment → CodeClickEvent → Navigation handler
Debug panel → Code click → CodeClickEvent → Navigation handler
                                                    ↓
                                        Unified navigation heuristic
                                                    ↓
                                        (switch workflow, select node, etc.)
```

### Integration with Navigation Heuristic

Once cursor events are enriched to `CodeClickEvent`, they can use baml-graph's sophisticated navigation heuristic:

**Location:** `packages/playground-common/src/sdk/navigationHeuristic.ts` (copied from baml-graph)

```typescript
export function determineNavigationAction(
  event: CodeClickEvent,
  state: NavigationState
): NavigationAction {
  if (event.type === 'test') {
    // Find workflow being tested
    const workflow = findWorkflowForFunction(event.functionName, state.workflows);
    return workflow
      ? { type: 'switch-workflow', workflowId: workflow.id }
      : { type: 'show-function-tests', functionName: event.functionName };
  }

  // Priority 1: Stay in current workflow if possible
  if (state.activeWorkflowId) {
    const node = findNodeInWorkflow(event.functionName, state.activeWorkflowId);
    if (node) {
      return { type: 'select-node', workflowId: state.activeWorkflowId, nodeId: node.id };
    }
  }

  // Priority 2: Find another workflow containing this function
  const workflow = findWorkflowForFunction(event.functionName, state.workflows);
  if (workflow) {
    return { type: 'switch-and-select', workflowId: workflow.id, nodeId: event.functionName };
  }

  // Priority 3: Show function in isolation (if it has tests)
  const func = findFunction(event.functionName, state.bamlFiles);
  if (func?.tests?.length > 0) {
    return { type: 'show-function-tests', functionName: event.functionName };
  }

  // Priority 4: Empty state
  return { type: 'empty-state', reason: 'no-workflow-or-tests', functionName: event.functionName };
}
```

### Debouncing Cursor Events

VSCode sends cursor updates frequently. Add debouncing:

```typescript
export const debouncedCodeClickAtom = atomWithDebounce(codeClickEventAtom, 150);
```

**Usage in navigation hook:**
```typescript
export function useNavigationHandler() {
  const codeClick = useAtomValue(debouncedCodeClickAtom);  // Debounced
  // ... handle navigation
}
```

### Migration Notes

**Phase 1: Add enrichment without breaking changes**
- Create `cursor-enrichment.ts`
- Add `codeClickEventAtom` alongside existing atoms
- Update `updateCursorAtom` to populate both old and new
- Feature flag to enable new navigation

**Phase 2: Unified navigation**
- Copy navigation heuristic from baml-graph
- Create `useNavigationHandler` hook
- Test thoroughly with feature flag

**Phase 3: Deprecate old atoms**
- Remove direct usage of `selectedFunctionAtom` in favor of derived state
- Make old atoms read from navigation state for compatibility

**See also:** `CURSOR_TO_CODECLICK_UNIFICATION.md` for full details

---

## Mock Data & Testing Strategy

### Requirements

1. **Browser-based testing** without VSCode
2. **Mock entire BAML project** with realistic data
3. **UI toggle** for dev mode features
4. **Debug panel** for simulating IDE interactions
5. **Navigation heuristic testing** for workflow switching
6. **Preserve existing capability** from baml-graph

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Dev Mode Detection                       │
│  - URL param: ?mock=true                                     │
│  - Env var: VITE_MOCK_MODE=true                             │
│  - localStorage: playground:mockMode                         │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────────┐
          │  Mode: 'mock' | 'standalone' │
          │       | 'vscode'             │
          └──────────────┬───────────────┘
                         │
         ┌───────────────┴───────────────┐
         │                               │
         ▼                               ▼
┌─────────────────┐           ┌──────────────────┐
│  Mock Provider  │           │  Real Provider   │
│  - Sample       │           │  - WASM runtime  │
│    workflows    │           │  - LSP messages  │
│  - Test cases   │           │  - File sync     │
│  - Simulated    │           │  - Real          │
│    execution    │           │    execution     │
└─────────────────┘           └──────────────────┘
```

### Mock Data Provider

**Location:** `packages/playground-common/src/sdk/mock.ts` (moved from baml-graph)

Keep existing implementation from `apps/baml-graph/src/sdk/mock.ts:93-1094` with enhancements:

```typescript
export interface MockDataProvider {
  // Existing methods
  getWorkflows(): WorkflowDefinition[]
  getTestCases(nodeId: string): TestCase[]
  simulateExecution(workflowId: string, inputs: unknown): AsyncGenerator<BAMLEvent>

  // NEW: File system simulation
  getFiles(): Record<string, string>
  getBamlFiles(): BAMLFile[]  // Parsed structure

  // NEW: Runtime simulation
  getDiagnostics(): Diagnostic[]
  getGeneratedFiles(): GeneratedFile[]
}

export class DefaultMockProvider implements MockDataProvider {
  // Existing implementation...

  // NEW: Generate BAML file content
  getFiles(): Record<string, string> {
    return {
      'main.baml': `
        function ExtractResume(resume: string) -> Resume {
          client GPT4
          prompt #"
            Extract structured data from the resume:
            {{ resume }}
          "#
        }

        class Resume {
          name string
          email string
          experience Experience[]
        }

        test ExtractResume {
          functions [ExtractResume]
          args {
            resume "John Doe\njohn@example.com\nSoftware Engineer at ACME Corp"
          }
        }
      `,
      'workflows.baml': `
        workflow ExtractAndValidateResume {
          ExtractResume -> ValidateResume -> ScoreResume
        }
      `
    }
  }

  getBamlFiles(): BAMLFile[] {
    // Parse files into structured format
    return parseBAMLFiles(this.getFiles())
  }

  getDiagnostics(): Diagnostic[] {
    // Return empty or sample diagnostics
    return []
  }

  getGeneratedFiles(): GeneratedFile[] {
    // Return sample generated code
    return [
      {
        path: 'baml_client/types.ts',
        language: 'typescript',
        content: 'export interface Resume { name: string; email: string; }'
      }
    ]
  }
}
```

### Debug Panel Integration

**Location:** `packages/playground-common/src/features/debug/` (moved from baml-graph)

**Component:** `DebugPanel.tsx` (from `apps/baml-graph/src/features/debug/components/DebugPanel.tsx:1-203`)

```typescript
export function DebugPanel() {
  const sdk = useBAMLSDK()
  const mode = useAtomValue(playgroundModeAtom)
  const showDebug = useAtomValue(showDebugPanelAtom)

  // Only show in mock or standalone mode
  if (mode === 'vscode' || !showDebug) {
    return null
  }

  const files = sdk.runtime.getFiles()
  const bamlFiles = parseBamlFiles(files)

  const handleFileClick = (filePath: string) => {
    sdk.emitCodeClick({ type: 'file', path: filePath })
  }

  const handleFunctionClick = (functionName: string) => {
    sdk.emitCodeClick({ type: 'function', name: functionName })
  }

  const handleTestClick = async (functionName: string, testName: string) => {
    sdk.emitCodeClick({ type: 'test', functionName, testName })

    // Run test
    await sdk.tests.run({ functionName, testName })
  }

  return (
    <div className="debug-panel">
      <h3>Debug Panel (Dev Mode)</h3>

      {/* File browser */}
      <div className="file-browser">
        {Object.keys(files).map(path => (
          <div key={path} onClick={() => handleFileClick(path)}>
            {path}
          </div>
        ))}
      </div>

      {/* Function list */}
      <div className="function-list">
        {bamlFiles.flatMap(f => f.functions).map(func => (
          <div key={func.name}>
            <div onClick={() => handleFunctionClick(func.name)}>
              {func.name}
              <span className="badge">{func.type}</span>
            </div>

            {/* Tests for function */}
            <div className="test-list">
              {func.tests?.map(test => (
                <div key={test.name} onClick={() => handleTestClick(func.name, test.name)}>
                  ▶ {test.name}
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
```

### Dev Mode Toggle

**UI Component:** `DevModeToggle.tsx` (new)

```typescript
export function DevModeToggle() {
  const [showDebug, setShowDebug] = useAtom(showDebugPanelAtom)
  const mode = useAtomValue(playgroundModeAtom)

  // Only show toggle in dev environments
  if (!import.meta.env.DEV) {
    return null
  }

  return (
    <div className="dev-mode-toggle">
      <label>
        <input
          type="checkbox"
          checked={showDebug}
          onChange={(e) => setShowDebug(e.target.checked)}
        />
        Show Debug Panel
      </label>

      <span className="mode-badge">{mode}</span>
    </div>
  )
}
```

**Integration in App:**

```typescript
// In apps/playground/src/App.tsx or playground-common's main component
export function App() {
  const mode = useAtomValue(playgroundModeAtom)

  return (
    <div className="app">
      <ThemeProvider>
        {/* Main content */}
        <div className="main-content">
          <PromptPreview />
        </div>

        {/* Debug panel (only in mock/standalone mode) */}
        {(mode === 'mock' || mode === 'standalone') && <DebugPanel />}

        {/* Dev mode toggle (only in development) */}
        {import.meta.env.DEV && <DevModeToggle />}

        <EventListener />
      </ThemeProvider>
    </div>
  )
}
```

### Testing Workflows

**Scenario 1: Test navigation heuristic**

1. Open playground in browser with `?mock=true`
2. Debug panel shows sample BAML files
3. Click function "ExtractResume"
4. Verify: Correct workflow activates, node selected
5. Click test "test_valid_resume"
6. Verify: Test inputs populate, detail panel opens

**Scenario 2: Test execution simulation**

1. Click "Run" on a test in debug panel
2. Verify: SDK emits `execution.started` event
3. Verify: Nodes transition through states (not-started → running → success)
4. Verify: Detail panel shows outputs
5. Verify: Test history updates

**Scenario 3: Test cache behavior**

1. Run test with caching enabled
2. Verify: First run shows "running" state
3. Run same test again
4. Verify: Second run shows "cached" state immediately
5. Clear cache via UI
6. Run test again
7. Verify: Cache miss, re-executes

### Mock Data Configuration

**UI for mock configuration** (optional, advanced feature):

```typescript
// Settings panel for mock data customization
export function MockDataSettings() {
  const sdk = useBAMLSDK()
  const mode = useAtomValue(playgroundModeAtom)

  if (mode !== 'mock') return null

  const [cacheHitRate, setCacheHitRate] = useState(0.3)
  const [errorRate, setErrorRate] = useState(0.1)
  const [speedMultiplier, setSpeedMultiplier] = useState(1)

  const handleUpdate = () => {
    // Update mock provider config
    sdk.updateMockConfig({ cacheHitRate, errorRate, speedMultiplier })
  }

  return (
    <div className="mock-settings">
      <h4>Mock Data Settings</h4>

      <label>
        Cache Hit Rate: {cacheHitRate}
        <input type="range" min="0" max="1" step="0.1"
               value={cacheHitRate} onChange={e => setCacheHitRate(+e.target.value)} />
      </label>

      <label>
        Error Rate: {errorRate}
        <input type="range" min="0" max="1" step="0.1"
               value={errorRate} onChange={e => setErrorRate(+e.target.value)} />
      </label>

      <label>
        Speed Multiplier: {speedMultiplier}x
        <input type="range" min="0.1" max="5" step="0.1"
               value={speedMultiplier} onChange={e => setSpeedMultiplier(+e.target.value)} />
      </label>

      <button onClick={handleUpdate}>Apply</button>
    </div>
  )
}
```

---

## Migration Plan

### Pre-Migration Preparation

1. **Create feature flag**
   - Add `useNewPlaygroundArchitecture` feature flag
   - Default: false (use existing code)
   - Enable progressively for testing

2. **Set up parallel structure**
   - Create `src/shared/atoms-v2/` directory
   - Copy SDK to `src/sdk/`
   - New components in `src/features-v2/`

3. **Add comprehensive tests**
   - Unit tests for SDK methods
   - Integration tests for atom synchronization
   - E2E tests for VSCode message flow

### Phase 1: Foundation (Week 1-2)

**Goal:** Establish new atom structure and SDK without breaking existing code

**Tasks:**

1. **Create unified atom files**
   - File: `packages/playground-common/src/shared/atoms-v2/workflow.atoms.ts`
   - File: `packages/playground-common/src/shared/atoms-v2/execution.atoms.ts`
   - File: `packages/playground-common/src/shared/atoms-v2/runtime.atoms.ts`
   - File: `packages/playground-common/src/shared/atoms-v2/ui.atoms.ts`
   - File: `packages/playground-common/src/shared/atoms-v2/derived.atoms.ts`
   - File: `packages/playground-common/src/shared/atoms-v2/index.ts`

2. **Copy SDK to playground-common**
   - Dir: `packages/playground-common/src/sdk/`
   - Files: `index.ts`, `types.ts`, `mock.ts`, `adapter.ts`, `navigationHeuristic.ts`
   - Copy atom files from baml-graph as starting point
   - Add exports in `src/index.ts`

3. **Add mode detection**
   - File: `packages/playground-common/src/shared/mode.ts`
   - Implement `detectPlaygroundMode()` function
   - Add `playgroundModeAtom` to `ui.atoms.ts`

4. **Extend SDK with WASM integration**
   - Add `WasmProvider` interface to SDK
   - Implement `runtime.*` methods
   - Implement `tests.*` methods
   - Implement `settings.*` methods

**Validation:**
- New atoms pass type checks
- SDK instantiates without errors
- Mock mode works in isolation

### Phase 2: EventListener Refactor (Week 3)

**Goal:** Migrate EventListener to use SDK while maintaining compatibility

**Tasks:**

1. **Create new EventListener**
   - File: `packages/playground-common/src/baml_wasm_web/EventListener-v2.tsx`
   - Inject SDK instance as prop
   - Replace atom updates with SDK method calls
   - Keep existing message handling logic

2. **Add SDK provider**
   - File: `packages/playground-common/src/sdk/provider.tsx`
   ```typescript
   export function BAMLSDKProvider({ children }: { children: ReactNode }) {
     const mode = detectPlaygroundMode()
     const sdkRef = useRef<BAMLSDK>()

     if (!sdkRef.current) {
       const config: BAMLSDKConfig = {
         mode,
         mockData: mode === 'mock' ? new DefaultMockProvider() : undefined,
         wasmProvider: mode !== 'mock' ? new RealWasmProvider() : undefined
       }
       sdkRef.current = new BAMLSDK(config)

       if (mode === 'mock') {
         sdkRef.current.initialize()
       }
     }

     return (
       <BAMLSDKContext.Provider value={sdkRef.current}>
         {children}
       </BAMLSDKContext.Provider>
     )
   }
   ```

3. **Add SDK hook**
   - File: `packages/playground-common/src/sdk/hooks.ts`
   ```typescript
   export function useBAMLSDK(): BAMLSDK {
     const sdk = useContext(BAMLSDKContext)
     if (!sdk) throw new Error('useBAMLSDK must be used within BAMLSDKProvider')
     return sdk
   }
   ```

4. **Feature flag integration**
   - Update App to conditionally use new EventListener
   ```typescript
   const useNewArchitecture = useFeatureFlag('useNewPlaygroundArchitecture')
   return (
     <>
       {useNewArchitecture ? (
         <BAMLSDKProvider>
           <EventListener-v2 />
           {children}
         </BAMLSDKProvider>
       ) : (
         <>
           <EventListener />
           {children}
         </>
       )}
     </>
   )
   ```

**Validation:**
- VSCode messages still processed correctly
- File sync works
- Test execution works
- Settings updates work
- No regressions in existing functionality

### Phase 3: Component Migration (Week 4-5)

**Goal:** Migrate UI components to use SDK and new atoms

**Tasks:**

1. **Create bridge hooks** (for gradual migration)
   ```typescript
   // File: src/sdk/bridge-hooks.ts

   // Hook that works with old or new atoms
   export function useSelectedFunction() {
     const useNew = useFeatureFlag('useNewPlaygroundArchitecture')
     const sdk = useBAMLSDK()

     if (useNew) {
       const nodeId = useAtomValue(selectedNodeIdAtom)
       const functionsMap = useAtomValue(allFunctionsMapAtom)
       return functionsMap.get(nodeId ?? '')
     } else {
       // Old atom
       return useAtomValue(selectedFunctionObjectAtom)
     }
   }
   ```

2. **Migrate core components**
   - `PromptPreview.tsx`: Use `useSelectedFunction()` bridge hook
   - `TestPanel.tsx`: Use `sdk.tests.*` methods
   - `DetailPanel.tsx`: Use new `detailPanelAtom`
   - `Sidebar.tsx`: Use new `isSidebarOpenAtom`

3. **Migrate test execution**
   - Update `test-runner.ts` to work with SDK
   - Keep existing logic but call through SDK
   - Use `sdk.tests.run()` and `sdk.tests.runAll()`

4. **Migrate API key management**
   - Keep existing api-keys atoms (they're already well-organized)
   - Add `sdk.settings.getApiKeys()` / `sdk.settings.setApiKeys()` wrappers

**Validation:**
- All UI components render correctly
- Test execution works with new architecture
- API key management works
- No console errors

### Phase 4: Debug Panel Integration (Week 6)

**Goal:** Add debug panel and mock mode support

**Tasks:**

1. **Copy debug panel components**
   - Dir: `packages/playground-common/src/features/debug/`
   - Components: `DebugPanel.tsx`, `FileTree.tsx`, `FunctionList.tsx`

2. **Add dev mode UI**
   - Component: `DevModeToggle.tsx`
   - Add to toolbar or status bar

3. **Integration in main app**
   - Update `App.tsx` to show debug panel conditionally
   - Add keyboard shortcut to toggle (e.g., Ctrl+Shift+D)

4. **Test mock mode**
   - Open playground with `?mock=true`
   - Verify: Debug panel appears
   - Verify: Sample data loads
   - Verify: Navigation heuristics work
   - Verify: Test execution simulates correctly

**Validation:**
- Debug panel renders in mock mode
- File browser works
- Function/test clicks trigger correct behavior
- Mock execution simulates realistically

### Phase 5: Graph Visualization (Week 7-8)

**Goal:** Integrate workflow graph from baml-graph

**Tasks:**

1. **Copy graph components**
   - Dir: `packages/playground-common/src/features/graph/`
   - Components: `WorkflowGraph.tsx`, graph primitives (BaseNode, LLMNode, etc.)
   - Layout: `layout/` directory with ELK integration

2. **Add graph mode toggle**
   - New atom: `graphViewModeAtom` ('function' | 'workflow')
   - UI toggle in toolbar

3. **Conditional rendering**
   ```typescript
   export function PromptPreview() {
     const viewMode = useAtomValue(graphViewModeAtom)
     const isLLMOnly = useAtomValue(isLLMOnlyModeAtom)

     if (isLLMOnly) {
       return <LLMOnlyPanel />
     }

     if (viewMode === 'workflow') {
       return (
         <div className="workflow-view">
           <WorkflowGraph />
           <DetailPanel />
         </div>
       )
     }

     return (
       <div className="function-view">
         {/* Existing playground UI */}
       </div>
     )
   }
   ```

4. **Sync with navigation heuristic**
   - When workflow clicked in debug panel → switch to workflow view
   - When standalone function clicked → stay in function view

**Validation:**
- Graph renders correctly
- Node selection works
- Detail panel shows correct data
- Layout algorithm performs well

### Phase 6: Cleanup & Optimization (Week 9)

**Goal:** Remove old code, optimize performance, finalize migration

**Tasks:**

1. **Remove old atoms**
   - Delete `src/shared/baml-project-panel/playground-panel/atoms.ts` (old atoms)
   - Delete scattered atom files
   - Keep only `src/shared/atoms/` directory

2. **Remove feature flag**
   - Set `useNewPlaygroundArchitecture` to always true
   - Remove conditional logic
   - Remove old EventListener

3. **Rename files**
   - `EventListener-v2.tsx` → `EventListener.tsx`
   - `atoms-v2/` → `atoms/`

4. **Performance optimization**
   - Profile with React DevTools
   - Optimize expensive derived atoms with `useMemo`
   - Add `atomFamily` caching where needed

5. **Documentation**
   - Update README with new architecture
   - Document SDK API
   - Add migration guide for future changes

**Validation:**
- No references to old code
- Bundle size not significantly increased
- Performance metrics maintained or improved
- All tests pass

### Phase 7: VSCode Extension Update (Week 10)

**Goal:** Update VSCode extension to leverage new capabilities

**Tasks:**

1. **Update webview HTML**
   - File: `apps/vscode-ext/src/panels/WebviewPanelHost.ts:274-406`
   - Pass mode information to webview
   - Add support for new RPC methods if needed

2. **Update message protocol**
   - Add new event types if needed
   - Update `vscode-to-webview-rpc.ts` and `webview-to-vscode-rpc.ts`

3. **Test in VSCode**
   - Open VSCode extension host
   - Test all commands: `baml.openBamlPanel`, `baml.runBamlTest`
   - Test file synchronization
   - Test settings updates

**Validation:**
- VSCode extension works identically to before
- No regressions
- New features available (if any)

---

## Implementation Phases

### Summary Timeline

| Phase | Duration | Focus | Risk |
|-------|----------|-------|------|
| 1. Foundation | 2 weeks | Atom structure, SDK setup | Low |
| 2. EventListener | 1 week | SDK integration | Medium |
| 3. Components | 2 weeks | UI migration | Medium |
| 4. Debug Panel | 1 week | Mock mode | Low |
| 5. Graph | 2 weeks | Workflow visualization | Medium |
| 6. Cleanup | 1 week | Optimization | Low |
| 7. VSCode | 1 week | Extension testing | High |
| **Total** | **10 weeks** | | |

### Resource Requirements

**Development:**
- 1 senior engineer (full-time)
- 1 engineer for testing support (part-time)

**Testing:**
- QA engineer for VSCode extension testing (week 7, 10)
- Beta users for feedback (week 8-9)

### Success Criteria

**Functionality:**
- [ ] All existing VSCode features work
- [ ] Mock mode enables standalone testing
- [ ] Debug panel simulates IDE interactions
- [ ] Navigation heuristics work correctly
- [ ] Workflow graph renders and is interactive
- [ ] Test execution (function and workflow) works
- [ ] API key management preserved

**Performance:**
- [ ] No perceivable performance degradation
- [ ] Bundle size increase < 10%
- [ ] Initial load time < 3 seconds

**Code Quality:**
- [ ] Test coverage > 80%
- [ ] No TypeScript errors
- [ ] All linting rules pass
- [ ] Documentation complete

---

## Risk Assessment

### High Risks

**1. Breaking VSCode Extension**

**Impact:** High - Users can't use playground
**Likelihood:** Medium

**Mitigation:**
- Extensive manual testing in VSCode
- Automated E2E tests
- Feature flag for rollback
- Beta release to internal users first

**Contingency:**
- Keep old code path with feature flag
- Quick rollback capability
- Hotfix process defined

**2. Performance Degradation**

**Impact:** Medium - Slower experience
**Likelihood:** Low

**Mitigation:**
- Performance benchmarks before/after
- Profile with React DevTools
- Optimize critical paths with `atomFamily`
- Lazy load heavy components

**Contingency:**
- Roll back specific optimizations
- Defer graph visualization if needed

### Medium Risks

**3. State Synchronization Issues**

**Impact:** Medium - Inconsistent UI state
**Likelihood:** Medium

**Mitigation:**
- Comprehensive unit tests for atoms
- Integration tests for SDK ↔ atoms
- Manual testing of all workflows

**Contingency:**
- Add debug logging to track state changes
- Hotfix specific synchronization issues

**4. Mock Data Accuracy**

**Impact:** Low - Tests don't reflect reality
**Likelihood:** Medium

**Mitigation:**
- Base mock data on real BAML files
- Regular updates to mock data
- Compare mock vs real execution

**Contingency:**
- Update mock provider based on feedback
- Add configuration for mock behavior

### Low Risks

**5. Increased Bundle Size**

**Impact:** Low - Slower initial load
**Likelihood:** High

**Mitigation:**
- Code splitting
- Lazy load graph components
- Tree shaking optimizations

**Contingency:**
- Accept small increase if necessary
- Document bundle size growth

**6. Developer Confusion**

**Impact:** Low - Slower onboarding
**Likelihood:** Medium

**Mitigation:**
- Comprehensive documentation
- Code comments
- Example usage in README

**Contingency:**
- Hold team training sessions
- Update documentation based on questions

---

## Appendix: File Locations Reference

### Current State

**apps/baml-graph:**
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/index.ts:40-508` - BAMLSDK class
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/atoms/workflow.atoms.ts:17-38` - Workflow atoms
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/atoms/execution.atoms.ts:27-205` - Execution atoms
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/atoms/ui.atoms.ts:17-93` - UI atoms
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/atoms/derived.atoms.ts:26-179` - Derived atoms
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/mock.ts:93-1094` - Mock data provider
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/debug/components/DebugPanel.tsx:1-203` - Debug panel

**packages/playground-common:**
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/baml_wasm_web/EventListener.tsx:57-217` - EventListener
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/atoms.ts:121-339` - Core atoms
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:13-264` - Playground atoms
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/vscode.ts:1-535` - VSCode API wrapper

**apps/vscode-ext:**
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/vscode-ext/src/panels/WebviewPanelHost.ts:136-772` - Webview host
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/vscode-ext/src/extension.ts:162-228` - Extension commands

### Proposed State (After Migration)

**packages/playground-common:**
- `src/shared/atoms/index.ts` - Main atom exports
- `src/shared/atoms/workflow.atoms.ts` - Workflow state
- `src/shared/atoms/execution.atoms.ts` - Execution state
- `src/shared/atoms/runtime.atoms.ts` - WASM runtime state
- `src/shared/atoms/ui.atoms.ts` - UI state
- `src/shared/atoms/derived.atoms.ts` - Derived atoms
- `src/shared/atoms/api-keys.atoms.ts` - API keys (existing)
- `src/sdk/index.ts` - BAMLSDK class
- `src/sdk/types.ts` - Type definitions
- `src/sdk/mock.ts` - Mock data provider
- `src/sdk/provider.tsx` - BAMLSDKProvider component
- `src/sdk/hooks.ts` - React hooks for SDK
- `src/baml_wasm_web/EventListener.tsx` - Refactored EventListener
- `src/features/debug/DebugPanel.tsx` - Debug panel
- `src/features/graph/WorkflowGraph.tsx` - Workflow graph
- `src/shared/mode.ts` - Mode detection

---

## Conclusion

This integration plan provides a comprehensive roadmap for merging `apps/baml-graph` with `packages/playground-common`. The key innovations are:

1. **SDK Architecture**: Adopting BAMLSDK as the core abstraction provides better separation of concerns, testability, and reusability.

2. **Unified State Management**: Consolidating 70+ scattered atoms into ~50 domain-organized atoms improves maintainability and performance.

3. **Mode-Based Design**: Supporting 'vscode', 'standalone', and 'mock' modes enables flexible development and testing.

4. **Preserved Testing Capability**: The debug panel and mock data infrastructure from baml-graph are integrated, allowing browser-based testing of full workflows.

5. **Incremental Migration**: The phased approach with feature flags minimizes risk and allows for gradual rollout.

The estimated timeline of 10 weeks is realistic for a senior engineer with testing support. The architecture preserves all existing functionality while adding significant new capabilities for development and testing.
