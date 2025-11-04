# Design Document: Merging baml-graph with playground-common

**Author:** Claude Code
**Date:** 2025-11-04
**Status:** Draft

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Current Architecture Analysis](#current-architecture-analysis)
3. [State Management Comparison](#state-management-comparison)
4. [EventListener vs bamlSDK Pattern Analysis](#eventlistener-vs-bamlsdk-pattern-analysis)
5. [Unifying Cursor Updates and Code Click Events](#unifying-cursor-updates-and-code-click-events)
6. [Debug and Mock Capabilities](#debug-and-mock-capabilities)
7. [Proposed Merge Strategy](#proposed-merge-strategy)
8. [Implementation Plan](#implementation-plan)
9. [Open Questions and Decisions](#open-questions-and-decisions)

---

## Executive Summary

### Goal
Merge the new `apps/baml-graph` application with the existing `packages/playground-common` to create a unified playground experience that supports:
- VSCode extension webview integration
- Standalone browser testing with mock data
- Advanced workflow graph visualization
- Debug panel for navigation testing
- Unified state management

### Key Findings

**baml-graph** introduces:
- **bamlSDK**: A centralized SDK class that manages workflows, executions, graph data, and cache
- **Feature-based architecture**: Clean separation of concerns (debug, execution, navigation, graph)
- **Mock execution simulation**: Complete workflow execution with realistic delays and events
- **Debug panel**: Simulates clicking on BAML functions/tests to verify navigation heuristics
- **Jotai atoms**: Modern state management with atomFamily for granular updates

**playground-common** provides:
- **EventListener pattern**: Message-based integration with VSCode/IDEs
- **WASM integration**: Direct compilation and runtime management
- **Platform abstraction**: Works across VSCode, JetBrains, Zed, and standalone
- **Mature UI components**: API keys, status bar, test execution, prompt preview
- **RPC system**: Bidirectional communication with host environment

### Recommendation

Adopt the **bamlSDK pattern** as the primary abstraction layer while preserving the **EventListener** as the integration bridge. This hybrid approach:
1. Keeps SDK responsible for business logic (compiletime info, execution state, graphs)
2. Uses EventListener to translate host messages into SDK calls
3. Enables browser-based testing with mock mode
4. Maintains backward compatibility with existing VSCode integration

---

## Current Architecture Analysis

### baml-graph Architecture

**Location:** `apps/baml-graph/`

#### Entry Points
- **Main Entry:** `src/main.tsx:1-14`
  - Wraps app in `BAMLSDKProvider`
  - Renders `WorkFlow` component
- **App Component:** `src/App.tsx:35-227`
  - Main `EditWorkFlow` component
  - Manages ReactFlow instance
  - Handles dark mode, node selection, layout

#### Directory Structure (78 files)
```
src/
├── components/ui/          # ShadCN components
├── data/                   # Graph data types and converters
├── features/               # Feature modules (debug, detail-panel, execution, graph, llm, navigation, workflow)
├── graph-primitives/       # ReactFlow nodes/edges
├── sdk/                    # BAML SDK (core state management)
│   ├── index.ts           # SDK class (lines 39-508)
│   ├── provider.tsx       # React provider (lines 23-82)
│   ├── atoms/             # Jotai atoms organized by domain
│   ├── mock.ts            # Mock data provider (lines 93-1094)
│   ├── adapter.ts         # Graph format conversions
│   └── navigationHeuristic.ts  # Navigation logic
├── states/                # ReactFlow store
└── utils/                 # Utilities
```

#### bamlSDK Class (`src/sdk/index.ts:39-508`)

The SDK is the **backbone** of baml-graph, providing:

**1. Workflow Management** (lines 74-110)
```typescript
workflows: {
  getAll(): Workflow[]
  getById(id): Workflow | undefined
  getActive(): Workflow | undefined
  setActive(id): void
}
```

**2. Execution Management** (lines 116-220)
```typescript
executions: {
  start(workflowId, options): Promise<ExecutionSnapshot>
  getExecutions(workflowId): ExecutionSnapshot[]
  cancel(executionId): void
}
```
- Lines 120-179: `start()` clears node states, creates snapshot, runs mock simulation
- Lines 355-495: Mock execution traverses graph, simulates nodes with delays

**3. Graph API** (lines 226-269)
```typescript
graph: {
  getGraph(workflowId, mode): Graph | undefined
  updateNodePositions(workflowId, positions): void
}
```

**4. Cache API** (lines 275-313)
```typescript
cache: {
  get(nodeId, inputHash): CacheEntry | undefined
  set(nodeId, inputHash, result): void
  clear(scope): void
}
```

**5. Test Cases API** (lines 319-329)
```typescript
testCases: {
  get(nodeId): TestCase[]
}
```

**6. Event System** (lines 335-349)
```typescript
emitEvent(event: SDKEvent): void
onEvent(handler: (event) => void): () => void
```

**SDK Modes** (`src/sdk/types.ts:305-309`)
- `vscode`: Integrates with VSCode extension
- `mock`: Uses mock data provider (current)
- `server`: Connects to remote server

#### Atom Organization (`src/sdk/atoms/`)

**Workflow Atoms** (`workflow.atoms.ts`)
- `workflowsAtom`: All workflows
- `activeWorkflowIdAtom`: Selected workflow ID
- `activeWorkflowAtom`: Derived active workflow
- `recentWorkflowsAtom`: Recently accessed

**Execution Atoms** (`execution.atoms.ts`)
- `workflowExecutionsAtomFamily`: Per-workflow executions (lines 27-29)
- `selectedExecutionIdAtom`: Selected execution
- `nodeStateAtomFamily`: Per-node execution state (lines 99-101)
- `allNodeStatesAtom`: Map of all node states (lines 113-122)
- `registerNodeAtom`: Register active nodes (lines 127-135)
- `clearAllNodeStatesAtom`: Reset all nodes (lines 141-149)
- `nodeExecutionsAtom`: Node execution data (lines 155-162)
- `eventStreamAtom`: Real-time events (line 172)
- `cacheAtom`: Cache storage (line 198)

**UI Atoms** (`ui.atoms.ts`)
- `viewModeAtom`: Editor vs snapshot view (lines 17-20)
- `selectedNodeIdAtom`: Selected graph node (line 29)
- `detailPanelAtom`: Panel state (lines 38-46)
- `layoutDirectionAtom`: Graph layout (line 55)
- `selectedInputSourceAtom`: Input source (lines 65-69)
- `activeNodeInputsAtom`: Editable inputs (line 74)
- `bamlFilesAtom`: All BAML files (line 88)
- `activeCodeClickAtom`: Current code click event (line 93)

**Derived Atoms** (`derived.atoms.ts`)
- `allFunctionsMapAtom`: O(1) function lookup (lines 26-37)
- `functionsByTypeAtom`: Grouped by type (lines 44-61)
- `workflowFunctionIdsAtom`: Set of workflow function IDs (lines 73-84)
- `standaloneFunctionsAtom`: Non-workflow functions (lines 91-104)
- `selectedFunctionAtom`: Currently selected (lines 116-122)
- `isLLMOnlyModeAtom`: LLM-only view toggle (lines 148-178)

#### Key Hooks (`src/sdk/hooks.ts`)
- `useWorkflows()`: Get all workflows
- `useActiveWorkflow()`: Get/set active workflow (lines 50-77)
- `useNodeState(nodeId)`: Granular node state (lines 138-140)
- `useDetailPanel()`: Panel controls (lines 180-213)
- `useCurrentGraph()`: Editor or snapshot graph (lines 259-283)
- `useActiveNode()`: Selected node with data (lines 288-309)
- `useNodeInputSources(nodeId)`: Input sources (lines 319-346)

---

### playground-common Architecture

**Location:** `packages/playground-common/`

#### Package Structure

**Entry Points** (`package.json:15-30`)
```json
{
  ".": "./src/index.ts",
  "./jotai-provider": "./src/baml_wasm_web/JotaiProvider.tsx",
  "./event-listener": "./src/baml_wasm_web/EventListener.tsx",
  "./prompt-preview": "./src/shared/baml-project-panel/...",
  "./baml-project-panel/atoms": "./src/shared/baml-project-panel/atoms.ts"
}
```

#### Directory Structure (~13,688 lines)
```
src/
├── baml_wasm_web/          # Core integration layer
│   ├── JotaiProvider.tsx  # Jotai setup with VSCode persistence
│   ├── EventListener.tsx  # Message handler
│   └── bamlConfig.ts      # Config atoms
├── shared/
│   └── baml-project-panel/
│       ├── atoms.ts       # Core WASM/runtime atoms
│       ├── vscode.ts      # VSCodeAPIWrapper singleton
│       ├── playground-panel/
│       │   ├── atoms.ts   # Playground-specific atoms
│       │   └── prompt-preview/
│       ├── codemirror-panel/
│       └── theme/
├── components/
│   ├── api-keys-dialog/   # API key management
│   │   └── atoms.ts       # API key atoms
│   └── status-bar.tsx
├── wasm/                   # WASM utilities
├── utils/                  # Error boundaries
└── lib/                    # Feedback widget
```

#### EventListener Pattern (`baml_wasm_web/EventListener.tsx:57-217`)

**Purpose:** Bridge between host environment and application state

**Message Sources:**
1. **VSCode postMessage API** (via `acquireVsCodeApi()`)
2. **WebSocket** (for JetBrains/Zed) on `/ws` endpoint
3. **Window messages** forwarded from above

**Message Handlers** (lines 110-210):

**IDE Messages** (lines 116-129)
```typescript
{ source: 'ide_message', payload: {
  command: 'update_cursor' → updateCursorAtom
  command: 'baml_cli_version' → bamlCliVersionAtom
  command: 'baml_settings_updated' → bamlConfig
}}
```

**LSP Messages** (lines 130-195)
```typescript
{ source: 'lsp_message', payload: {
  method: 'runtime_updated' → filesAtom (debounced 50ms)
  method: 'baml_settings_updated' → bamlConfig
  method: 'workspace/executeCommand' → {
    command: 'baml.openBamlPanel' → select function
    command: 'baml.runBamlTest' → run test (1s delay for JetBrains)
  }
  method: 'textDocument/codeAction' → updateCursorAtom (Zed)
}}
```

#### Core Atoms (`shared/baml-project-panel/atoms.ts`)

**WASM & Runtime** (lines 121-240)
- `wasmAtom`: Loaded WASM module (async, lines 121-133)
- `filesAtom`: BAML source files (line 140)
- `sandboxFilesAtom`: Sandbox files (line 141)
- `projectAtom`: Compiled BAML project (lines 143-156)
- `ctxAtom`: Execution context (lines 158-167)
- `runtimeAtom`: Runtime with diagnostics (lines 169-240)
  - Compiles with env vars and feature flags
  - Returns last valid runtime on error
- `diagnosticsAtom`: Compilation errors (lines 242-245)
- `numErrorsAtom`: Error/warning counts (lines 247-253)

**Generated Code** (lines 256-290)
- `generatedFilesAtom`: Generated code files (lines 256-275)
- `generatedFilesByLangAtom`: Filtered by language (atomFamily, lines 277-290)

**Settings** (lines 294-339)
- `vscodeSettingsAtom`: VSCode settings (async, lines 294-314)
- `playgroundPortAtom`: Proxy port (async, lines 316-328)
- `proxyUrlAtom`: Proxy URL (lines 330-339)
- `betaFeatureEnabledAtom`: Beta feature flag (lines 101-118)

**Panic Handling** (lines 28-94)
- `wasmPanicAtom`: WASM panic state (line 28)
- `useWasmPanicHandler()`: Hook to wire up handler (lines 60-85)
- `useClearWasmPanic()`: Clear panic (lines 91-94)

#### Playground Panel Atoms (`playground-panel/atoms.ts`)

**Function/Test Selection** (lines 13-172)
- `runtimeStateAtom`: List of functions (lines 13-32)
- `selectedFunctionAtom`: Selected function name (line 34)
- `selectedTestcaseAtom`: Selected test name (line 35)
- `selectedItemAtom`: Combined selection (lines 37-55)
- `functionObjectAtom`: atomFamily for function objects (lines 57-66)
- `testcaseObjectAtom`: atomFamily for test objects (lines 68-82)
- `updateCursorAtom`: Update selection from cursor (lines 84-139)
- `selectionAtom`: Current selection with objects (lines 141-172)

**Test Execution** (lines 209-250)
- `testCaseAtom`: atomFamily for test cases (lines 209-220)
- `functionTestSnippetAtom`: atomFamily for snippets (lines 222-231)
- `testCaseResponseAtom`: atomFamily for responses (lines 233-245)
- `areTestsRunningAtom`: Running state (line 246)
- `runningTestsAtom`: Array of running tests (lines 248-250)
- `currentAbortControllerAtom`: For cancellation (line 253)

**UI State** (line 263)
- `flashRangesAtom`: Code ranges to highlight

#### API Keys Atoms (`components/api-keys-dialog/atoms.ts`)

**Key Management** (lines 8-346)
- `apiKeyVisibilityAtom`: Visibility state (line 8)
- `showApiKeyDialogAtom`: Dialog visibility (lines 25-53)
- `envKeyValuesAtom`: Key-value pairs (lines 61-106)
- `userApiKeysAtom`: User's keys without proxy (lines 108-135)
- `apiKeysAtom`: Keys with proxy logic (lines 138-196)
- `requiredApiKeysAtom`: Required keys from runtime (lines 198-212)
- `localApiKeysAtom`: Local copy for editing (line 287)
- `hasLocalChangesAtom`: Unsaved changes (line 288)
- `renderedApiKeysAtom`: Computed list for UI (lines 305-346)

#### VSCode Integration (`shared/baml-project-panel/vscode.ts`)

**VSCodeAPIWrapper Singleton** (lines 63-535)

**RPC Methods:**
- `rpc<TRequest, TResponse>()`: Core RPC (lines 398-440)
  - Uses postMessage (VSCode) or WebSocket (others)
  - Tracks pending calls with IDs
  - 5s timeout
- `jumpToFile(span)`: Navigate to file (lines 154-183)
- `readFile(path)`: Read file contents (lines 199-221)
- `getVSCodeSettings()`: Get settings (lines 257-274)
- `loadAwsCreds(profile)`: Load AWS creds (lines 285-292)
- `loadGcpCreds()`: Load GCP creds (lines 294-300)
- `loadMediaFile(path)`: Platform-agnostic file loading (lines 352-396)
- `sendTelemetry(meta)`: Send telemetry (lines 453-458)

---

## State Management Comparison

### Architecture Philosophy

| Aspect | baml-graph | playground-common |
|--------|------------|-------------------|
| **Central Abstraction** | SDK class + Jotai atoms | Jotai atoms only |
| **Business Logic** | Encapsulated in SDK methods | Spread across atoms and hooks |
| **State Updates** | SDK methods → emit events → atoms | Direct atom updates |
| **Execution** | SDK.executions.start() | useRunBamlTests() hook |
| **Mode Switching** | SDK config (vscode/mock/server) | Environment detection (isVscode) |

### Atoms Comparison

#### Workflow/Function Management

| baml-graph | playground-common | Notes |
|------------|-------------------|-------|
| `workflowsAtom` | N/A | New concept in baml-graph |
| `activeWorkflowIdAtom` | N/A | New |
| `activeWorkflowAtom` | `selectedFunctionAtom` | Similar purpose, different scope |
| `workflowFunctionIdsAtom` | `runtimeStateAtom` | Both track available functions |
| `standaloneFunctionsAtom` | (implicit in `runtimeStateAtom`) | Filtering logic differs |
| `allFunctionsMapAtom` | `functionObjectAtom` (atomFamily) | Different patterns for lookup |
| `selectedFunctionAtom` | `selectedFunctionAtom` | Same name, similar purpose |

#### Execution State

| baml-graph | playground-common | Notes |
|------------|-------------------|-------|
| `workflowExecutionsAtomFamily` | N/A | Workflow-level execution tracking |
| `selectedExecutionIdAtom` | N/A | Snapshot selection |
| `nodeStateAtomFamily` | N/A | Per-node execution state |
| `allNodeStatesAtom` | N/A | Map of all node states |
| `nodeExecutionsAtom` | N/A | Node-level execution data |
| `eventStreamAtom` | N/A | Real-time event stream |
| `cacheAtom` | N/A | Cache storage |
| N/A | `testCaseResponseAtom` | Test execution results |
| N/A | `areTestsRunningAtom` | Running state |
| N/A | `runningTestsAtom` | Array of running tests |

**Analysis:** baml-graph has workflow-centric execution tracking, while playground-common has test-centric execution. Both needed for unified app.

#### WASM & Compilation

| baml-graph | playground-common | Notes |
|------------|-------------------|-------|
| N/A | `wasmAtom` | WASM module loading |
| N/A | `filesAtom` | Source files |
| N/A | `projectAtom` | Compiled project |
| N/A | `ctxAtom` | Execution context |
| N/A | `runtimeAtom` | Compiled runtime |
| N/A | `diagnosticsAtom` | Compilation errors |
| N/A | `wasmPanicAtom` | Panic handling |

**Analysis:** playground-common has mature WASM integration. baml-graph currently uses mock data but needs this layer.

#### UI State

| baml-graph | playground-common | Notes |
|------------|-------------------|-------|
| `viewModeAtom` | N/A | Editor vs snapshot view |
| `selectedNodeIdAtom` | N/A | Graph node selection |
| `detailPanelAtom` | N/A | Panel state |
| `layoutDirectionAtom` | N/A | Graph layout |
| `selectedInputSourceAtom` | N/A | Input source for testing |
| `activeNodeInputsAtom` | N/A | Editable inputs |
| `bamlFilesAtom` | `filesAtom` | Same purpose |
| `activeCodeClickAtom` | N/A | Code click events |
| N/A | `selectedTestcaseAtom` | Test selection |
| N/A | `flashRangesAtom` | Code highlighting |
| N/A | `isPanelVisibleAtom` | Panel visibility |

#### Settings & Configuration

| baml-graph | playground-common | Notes |
|------------|-------------------|-------|
| N/A | `vscodeSettingsAtom` | VSCode settings |
| N/A | `proxyUrlAtom` | Proxy configuration |
| N/A | `betaFeatureEnabledAtom` | Feature flags |
| N/A | `apiKeysAtom` | API key management |

**Analysis:** playground-common has robust settings management. baml-graph needs to adopt this.

### Hooks Comparison

#### baml-graph Hooks

```typescript
// Workflow management
useWorkflows() → Workflow[]
useActiveWorkflow() → [Workflow | undefined, (id: string) => void]

// Execution
useNodeState(nodeId) → NodeExecutionState
useExecutions(workflowId) → ExecutionSnapshot[]

// UI
useDetailPanel() → DetailPanelControls
useCurrentGraph() → Graph | undefined
useActiveNode() → ActiveNodeData
useNodeInputSources(nodeId) → InputSource[]

// Navigation
useCodeNavigation() → void (side effect hook)
useGraphSync() → void (side effect hook)
useExecutionSync() → void (side effect hook)
```

#### playground-common Hooks

```typescript
// WASM
useWaitForWasm() → boolean
useWasmPanicHandler() → void (setup hook)
useClearWasmPanic() → () => void

// Test execution
useRunBamlTests() → { runTests, cancelTests }

// Feedback
useFeedbackWidget() → void (setup hook)

// (Most state access via useAtom directly)
```

**Analysis:** baml-graph has more domain-specific hooks that encapsulate business logic. playground-common relies more on direct atom access.

### Patterns and Best Practices

#### baml-graph Patterns

✅ **Strengths:**
- SDK class centralizes business logic
- Clear API surface (workflows, executions, graph, cache, testCases)
- Event-driven updates (emitEvent/onEvent)
- AtomFamily for granular updates (nodeStateAtomFamily)
- Feature-based organization
- Mock mode enables browser testing

⚠️ **Considerations:**
- Additional abstraction layer (SDK class)
- Two sources of truth (SDK internal state + atoms)
- More complex to understand initially

#### playground-common Patterns

✅ **Strengths:**
- Direct atom access (simple mental model)
- Mature WASM integration
- Platform abstraction (VSCode/JetBrains/Zed/standalone)
- Proven in production
- RPC system for host communication

⚠️ **Considerations:**
- Business logic spread across atoms and components
- No centralized API (harder to discover capabilities)
- Test execution tightly coupled to UI

---

## EventListener vs bamlSDK Pattern Analysis

### EventListener Pattern (playground-common)

**Location:** `packages/playground-common/src/baml_wasm_web/EventListener.tsx`

**Architecture:**
```
VSCode Extension
    ↓ postMessage
EventListener (background component)
    ↓ directly updates atoms
Jotai Atoms
    ↓ useAtom
React Components
```

**Responsibilities:**
1. Listen to window messages
2. Parse message type (ide_message vs lsp_message)
3. Route to appropriate atom updates
4. Handle debouncing (e.g., 50ms for file updates)
5. Special handling for different IDEs (JetBrains 1s delay)

**Example Flow:**
```typescript
// VSCode sends cursor update
postMessage({
  source: 'ide_message',
  payload: {
    command: 'update_cursor',
    content: { fileName, line, column }
  }
})

// EventListener processes
useEffect(() => {
  window.addEventListener('message', (event) => {
    if (event.data.source === 'ide_message') {
      if (event.data.payload.command === 'update_cursor') {
        // Directly update atom
        setUpdateCursor([fileName, line, column])
      }
    }
  })
}, [setUpdateCursor])
```

**Pros:**
- Simple, direct mapping
- Low overhead
- Easy to debug (console.log in one place)
- Works well for message-driven architecture

**Cons:**
- No API surface (hard to discover what messages exist)
- Tight coupling between message format and atoms
- Business logic in event handler (e.g., JetBrains delay)
- No mock mode (relies on host)

### bamlSDK Pattern (baml-graph)

**Location:** `apps/baml-graph/src/sdk/index.ts`

**Architecture:**
```
External Source (VSCode/Mock/Server)
    ↓ config determines provider
BAML SDK (business logic)
    ↓ emitEvent()
Event Stream Atom
    ↓ side effects update atoms
Jotai Atoms
    ↓ useAtom
React Components
```

**Responsibilities:**
1. Abstract data source (mock/vscode/server)
2. Provide business logic API (workflows, executions, graph, cache)
3. Emit events for state changes
4. Handle execution simulation
5. Manage cache invalidation

**Example Flow:**
```typescript
// Component calls SDK
const sdk = useBAMLSDK()
await sdk.executions.start(workflowId, { startNodeId })

// SDK internal logic
async start(workflowId, options) {
  // Clear previous state
  this.store.set(clearAllNodeStatesAtom)

  // Create execution snapshot
  const snapshot = { id, workflowId, status: 'running' }
  this.store.set(workflowExecutionsAtomFamily(workflowId), prev => [...prev, snapshot])

  // Run execution (mock or real)
  if (this.config.mode === 'mock') {
    await this.mockExecutionSimulation(workflowId, options)
  } else {
    await this.vscodeExecutionHandler(workflowId, options)
  }

  // Emit completion event
  this.emitEvent({ type: 'execution.completed', executionId: id })
}
```

**Pros:**
- Clear API surface (discoverable)
- Centralized business logic
- Mode switching (mock/vscode/server)
- Event system decouples SDK from atoms
- Enables browser testing without host
- Easier to test (mock SDK)

**Cons:**
- Additional abstraction layer
- More complex setup (SDK provider)
- Potential for duplication (SDK state + atoms)
- Learning curve for contributors

### Hybrid Approach (Recommendation)

**Combine both patterns:**

```
VSCode Extension
    ↓ postMessage
EventListener
    ↓ translates messages to SDK calls
BAML SDK
    ↓ business logic + emitEvent()
Jotai Atoms
    ↓ useAtom
React Components
```

**EventListener becomes a thin adapter:**
```typescript
// EventListener.tsx
useEffect(() => {
  window.addEventListener('message', (event) => {
    if (event.data.source === 'lsp_message') {
      if (event.data.payload.method === 'runtime_updated') {
        // Translate to SDK call
        sdk.files.update(event.data.payload.params.files)
      }
      if (event.data.payload.method === 'workspace/executeCommand') {
        if (event.data.payload.params.command === 'baml.runBamlTest') {
          // Translate to SDK call
          const [fnName, testName] = event.data.payload.params.arguments
          sdk.tests.run(fnName, testName)
        }
      }
    }
  })
}, [sdk])
```

**Benefits:**
- ✅ SDK provides API surface and business logic
- ✅ EventListener handles platform-specific quirks (JetBrains delays)
- ✅ Mock mode works in browser (SDK uses mock provider)
- ✅ VSCode mode works via EventListener → SDK
- ✅ Easy to test (mock SDK, test EventListener separately)

---

## Unifying Cursor Updates and Code Click Events

### The Problem: Two Parallel Navigation Systems

Currently, the two codebases have complementary but disconnected navigation patterns:

**playground-common: updateCursorAtom**
- **Location:** `packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:84-139`
- **Trigger:** VSCode cursor movement (line/column position)
- **Process:** WASM runtime resolves cursor → function/test
- **Output:** Updates `selectedFunctionAtom` and `selectedTestcaseAtom`
- **Limitation:** Only updates selection atoms, no rich navigation logic

**baml-graph: activeCodeClickAtom**
- **Location:** `apps/baml-graph/src/sdk/atoms/ui.atoms.ts:93`
- **Trigger:** Debug panel clicks, explicit navigation commands
- **Process:** Already has function/test identified
- **Output:** Triggers sophisticated navigation heuristic
- **Limitation:** Doesn't handle cursor movements from IDE

### Current Flow Comparison

```
┌─────────────────────────────────────────────────────────────────┐
│                    playground-common (current)                   │
└─────────────────────────────────────────────────────────────────┘

VSCode Cursor Move
    ↓
EventListener receives { fileName, line, column }
    ↓
updateCursorAtom
    ↓
runtime.get_function_at_position()
    ↓
selectedFunctionAtom = "functionName"
selectedTestcaseAtom = "testName"
    ↓
UI updates (simple selection, no navigation logic)


┌─────────────────────────────────────────────────────────────────┐
│                      baml-graph (current)                        │
└─────────────────────────────────────────────────────────────────┘

Debug Panel Click
    ↓
activeCodeClickAtom = {
  type: 'function',
  functionName: 'fetchData',
  functionType: 'workflow',
  filePath: 'workflows/simple.baml'
}
    ↓
useCodeNavigation() processes event
    ↓
determineNavigationAction() (navigation heuristic)
    ↓
Execute action:
  - switch-workflow
  - select-node
  - switch-and-select
  - show-function-tests
    ↓
Complex navigation with workflow switching, camera panning, etc.
```

### The Key Insight: updateCursor Provides the Missing Link

The `updateCursorAtom` already does the hard work of **cursor → function/test resolution** using WASM. We can enhance it to create rich `CodeClickEvent` objects, giving cursor movements the same navigation power as debug panel clicks!

### Proposed Unified Pattern

**Enhanced updateCursorAtom creates CodeClickEvent:**

```typescript
// packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts
export const updateCursorAtom = atom(
  null,
  (get, set, cursor: { fileName: string; line: number; column: number }) => {
    const runtime = get(runtimeAtom)?.rt;
    if (!runtime) return;

    const fileContent = get(filesAtom)[cursor.fileName];
    if (!fileContent) return;

    // Convert line/column to cursor index
    const cursorIdx = calculateCursorIndex(fileContent, cursor.line, cursor.column);

    // Use WASM to resolve cursor position
    const selectedFunc = runtime.get_function_at_position(
      cursor.fileName,
      get(selectedFunctionAtom) ?? '',
      cursorIdx
    );

    if (selectedFunc) {
      // Check if cursor is in a test
      const selectedTestcase = runtime.get_testcase_from_position(
        selectedFunc,
        cursorIdx
      );

      // Create rich CodeClickEvent with all metadata
      const codeClickEvent: CodeClickEvent = selectedTestcase
        ? {
            // CURSOR IN TEST CASE
            type: 'test',
            testName: selectedTestcase.name,
            functionName: selectedFunc.name,
            filePath: cursor.fileName,
            nodeType: getFunctionNodeType(selectedFunc), // 'llm_function' | 'function'
            span: selectedTestcase.span, // Include span for navigation/highlighting
          }
        : {
            // CURSOR IN FUNCTION DEFINITION
            type: 'function',
            functionName: selectedFunc.name,
            functionType: getFunctionType(selectedFunc), // 'workflow' | 'function' | 'llm_function'
            filePath: cursor.fileName,
            span: selectedFunc.span,
          };

      // Set CodeClickEvent which triggers unified navigation system
      set(activeCodeClickAtom, codeClickEvent);

      // Also update simple selection atoms for backward compatibility
      set(selectedFunctionAtom, selectedFunc.name);
      if (selectedTestcase) {
        set(selectedTestcaseAtom, selectedTestcase.name);
      } else {
        set(selectedTestcaseAtom, undefined);
      }
    } else {
      // Cursor not in any function - clear selection
      set(activeCodeClickAtom, null);
      set(selectedFunctionAtom, undefined);
      set(selectedTestcaseAtom, undefined);
    }
  }
);

// Helper functions to extract type metadata from WASM objects
function getFunctionType(fn: WasmFunction): 'workflow' | 'function' | 'llm_function' {
  // Check if function is a workflow
  if (fn.is_workflow?.() || fn.workflow_id) {
    return 'workflow';
  }
  // Check if it's an LLM function (has client config)
  if (fn.client_config || fn.is_llm_function?.()) {
    return 'llm_function';
  }
  return 'function';
}

function getFunctionNodeType(fn: WasmFunction): 'llm_function' | 'function' {
  return fn.client_config || fn.is_llm_function?.()
    ? 'llm_function'
    : 'function';
}

function calculateCursorIndex(
  fileContent: string,
  line: number,
  column: number
): number {
  const lines = fileContent.split('\n');
  let cursorIdx = 0;
  for (let i = 0; i < line; i++) {
    cursorIdx += (lines[i]?.length ?? 0) + 1; // +1 for newline
  }
  cursorIdx += column;
  return cursorIdx;
}
```

### Unified Navigation Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    UNIFIED SYSTEM (proposed)                     │
└─────────────────────────────────────────────────────────────────┘

                    ┌──────────────────────────┐
                    │   Navigation Triggers    │
                    └──────────────────────────┘
                              ↓
        ┌─────────────────────┼─────────────────────┐
        ↓                     ↓                     ↓
  VSCode Cursor          Debug Panel          LSP Command
  Move Event             Click Event          (baml.openBamlPanel)
        ↓                     ↓                     ↓
  updateCursorAtom      Direct set           EventListener
        ↓                     ↓                     ↓
        └─────────────────────┼─────────────────────┘
                              ↓
                    activeCodeClickAtom
                    {
                      type: 'function' | 'test',
                      functionName: string,
                      functionType?: ...,
                      testName?: string,
                      filePath: string,
                      span: WasmSpan
                    }
                              ↓
                    useCodeNavigation() hook
                              ↓
                    determineNavigationAction()
                              ↓
        ┌─────────────────────┼─────────────────────┐
        ↓                     ↓                     ↓
   switch-workflow      select-node        show-function-tests
        ↓                     ↓                     ↓
   Complex navigation with workflow switching, node selection,
   camera panning, test case selection, etc.
```

### Benefits of Unification

**1. Consistent Navigation**
- Cursor movements and explicit clicks use the same navigation logic
- No more discrepancy between "select function" and "navigate to function"

**2. Rich Context Everywhere**
- Every navigation event has full metadata (file path, span, function type)
- Enables smarter navigation decisions

**3. Workflow-Aware Cursor**
- When cursor enters a function that's in a workflow → automatically switch to workflow view
- When cursor enters standalone function → show function-only view
- Implements the same smart navigation heuristic as debug panel

**4. Easier Testing**
- Single navigation system to test
- Debug panel and cursor movements go through same code path

**5. Better UX**
- Cursor movements can trigger workflow view automatically
- No manual "open workflow" command needed
- Context-sensitive navigation

### Optional: Debounced Navigation

With cursor movements triggering full navigation, we should add smart debouncing to avoid jarring UI changes:

```typescript
// Debounce cursor-triggered navigation to avoid jank
const debouncedCodeClickAtom = atom(
  (get) => get(activeCodeClickAtom),
  (get, set, event: CodeClickEvent | null) => {
    const source = get(navigationSourceAtom); // Track event source

    if (source === 'cursor') {
      // Debounce cursor movements (300ms)
      clearTimeout(cursorDebounceTimeout);
      cursorDebounceTimeout = setTimeout(() => {
        set(activeCodeClickAtom, event);
      }, 300);
    } else {
      // Immediate navigation for explicit clicks
      set(activeCodeClickAtom, event);
    }
  }
);
```

This prevents navigation spam when users arrow-key through code, while keeping explicit clicks responsive.

### Code Examples from Both Codebases

**Current playground-common updateCursorAtom:**
```typescript
// packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:84-139
const selectedFunc = runtime.get_function_at_position(fileName, currentFn, cursorIdx);
if (selectedFunc) {
  set(selectedFunctionAtom, selectedFunc.name);  // ← Only sets simple atom

  const selectedTestcase = runtime.get_testcase_from_position(selectedFunc, cursorIdx);
  if (selectedTestcase) {
    set(selectedTestcaseAtom, selectedTestcase.name);  // ← Only sets simple atom
  }
}
```

**Current baml-graph CodeClickEvent:**
```typescript
// apps/baml-graph/src/sdk/types.ts:288-299
export type CodeClickEvent = {
  type: 'function';
  functionName: string;
  functionType: 'workflow' | 'function' | 'llm_function';  // ← Rich metadata
  filePath: string;
} | {
  type: 'test';
  testName: string;
  functionName: string;
  filePath: string;
  nodeType: 'llm_function' | 'function';  // ← Rich metadata
};
```

**baml-graph navigation usage:**
```typescript
// apps/baml-graph/src/features/navigation/hooks/useCodeNavigation.ts:28-170
useEffect(() => {
  if (!activeCodeClick) return;

  const action = determineNavigationAction(activeCodeClick, navState);

  switch (action.type) {
    case 'switch-workflow':
      setActiveWorkflow(action.workflowId);  // ← Complex workflow switching
      break;
    case 'select-node':
      setSelectedNodeId(action.nodeId);
      openDetailPanel();
      panToNodeIfNeeded(node, flowStore);  // ← Camera panning
      break;
    // ... more complex navigation
  }
}, [activeCodeClick]);
```

### Implementation Checklist

When merging, the implementation should:

- [ ] Enhance `updateCursorAtom` to create `CodeClickEvent` objects
- [ ] Add helper functions to extract function type metadata from WASM
- [ ] Ensure `activeCodeClickAtom` is set by both cursor updates and explicit clicks
- [ ] Add optional debouncing for cursor-triggered navigation
- [ ] Keep backward compatibility by also updating `selectedFunctionAtom`/`selectedTestcaseAtom`
- [ ] Update EventListener to set `activeCodeClickAtom` for LSP commands
- [ ] Test navigation from all sources: cursor, debug panel, LSP commands
- [ ] Document the unified navigation flow

### Integration with Navigation Heuristic

The navigation heuristic (`apps/baml-graph/src/sdk/navigationHeuristic.ts:108-299`) is designed to work with `CodeClickEvent`. By having cursor updates create these events, we get smart navigation for free:

**Example Scenarios:**

1. **User cursors into a workflow function:**
   ```
   updateCursorAtom creates: {
     type: 'function',
     functionName: 'fetchData',
     functionType: 'workflow',  // ← Detected via getFunctionType()
     filePath: 'workflows/simple.baml'
   }
   ↓
   Navigation heuristic: "This function is in a workflow"
   ↓
   Action: switch-workflow → select-node
   ↓
   Result: Workflow graph opens with node selected
   ```

2. **User cursors into a test:**
   ```
   updateCursorAtom creates: {
     type: 'test',
     testName: 'test_success',
     functionName: 'fetchData',
     nodeType: 'function'
   }
   ↓
   Navigation heuristic: "This test targets a function in current workflow"
   ↓
   Action: select-node (with testId)
   ↓
   Result: Node selected + test case input source selected in detail panel
   ```

3. **User cursors into standalone LLM function:**
   ```
   updateCursorAtom creates: {
     type: 'function',
     functionName: 'AnalyzeSentiment',
     functionType: 'llm_function',  // ← No workflow association
     filePath: 'llm_only.baml'
   }
   ↓
   Navigation heuristic: "Standalone function with no workflow"
   ↓
   Action: show-function-tests
   ↓
   Result: LLM-only view showing function with prompt preview
   ```

This creates a **context-aware cursor** that automatically adapts the UI based on what code the user is looking at!

---

## Debug and Mock Capabilities

### baml-graph Debug Panel

**Location:** `apps/baml-graph/src/features/debug/components/DebugPanel.tsx:15-202`

**Purpose:** Simulates clicking on BAML functions/tests in IDE to verify navigation heuristics work correctly.

**Features:**

1. **File Tree Display** (lines 103-186)
   - Shows all BAML files
   - Lists functions and tests within each file
   - Collapsible sections
   - Active state highlighting

2. **Function Click Simulation** (lines 42-51)
   ```typescript
   const handleFunctionClick = (fn: WasmFunction) => {
     const event: CodeClickEvent = {
       type: 'function',
       filePath: fn.span().file_path,
       name: fn.name,
       span: fn.span
     }
     setActiveCodeClick(event) // Triggers navigation
   }
   ```

3. **Test Click Simulation** (lines 53-63)
   ```typescript
   const handleTestClick = (test: WasmTestCase) => {
     const event: CodeClickEvent = {
       type: 'test',
       filePath: test.file_path,
       functionName: test.function_name,
       testName: test.name
     }
     setActiveCodeClick(event) // Triggers navigation
   }
   ```

4. **Test Run Button** (lines 65-81)
   - "Run workflow for this test" button
   - Starts workflow execution for selected test
   - Useful for testing execution flows

5. **Visual Design**
   - Top-left corner overlay (line 95)
   - 200px wide, max 500px height
   - Sticky header and footer
   - Shows "Active: FunctionName/TestName"

**Navigation Testing Flow:**
```
User clicks function in Debug Panel
    ↓
setActiveCodeClick(event)
    ↓
activeCodeClickAtom updated
    ↓
useCodeNavigation() hook processes (src/features/navigation/hooks/useCodeNavigation.ts:28-170)
    ↓
determineNavigationAction() (src/sdk/navigationHeuristic.ts:108-299)
    ↓
Executes action:
  - switch-workflow → setActiveWorkflow()
  - select-node → selectNode() + camera pan
  - switch-and-select → both
  - show-function-tests → LLM-only view
  - empty-state → show message
```

**Navigation Heuristic** (`src/sdk/navigationHeuristic.ts:108-299`)

**For Test Clicks:**
1. If test targets workflow → Switch to that workflow
2. If test targets function in current workflow → Stay and select node
3. If test targets function in different workflow → Switch and select
4. If test targets standalone function → Show function with tests
5. Otherwise → Empty state

**For Function Clicks:**
1. If function in current workflow → Select node (stay in context)
2. If function in another workflow → Switch to that workflow
3. If function has tests (standalone) → Show in isolation
4. Otherwise → Empty state

### Mock Data System

**Location:** `apps/baml-graph/src/sdk/mock.ts:93-1094`

**Mock Configuration** (lines 24-33)
```typescript
const MOCK_CONFIG = {
  cacheHitRate: 0.3,        // 30% cache hits
  errorRate: 0.1,           // 10% errors
  verboseLogging: true,     // Console logs
  speedMultiplier: 1.0      // Execution speed
}
```

**Sample Workflows** (lines 158-231)

1. **Simple Workflow** (lines 161-176)
   - Linear: fetchData → processData → saveResult
   - Good for basic testing

2. **Conditional Workflow** (lines 178-207)
   - Has branches and subgraphs
   - Includes PROCESSING_SUBGRAPH group node
   - Tests complex logic

3. **Shared Workflow** (lines 209-231)
   - Demonstrates function reuse
   - aggregateData → fetchData

**Mock Test Cases** (lines 244-384)
- Each function has 3-5 test cases
- Mix of passing/failing tests
- Realistic input/output data
- Last run timestamps

**Mock BAML Files** (lines 390-564)
Organized by file path:
```
workflows/simple.baml          # Simple workflow + functions + tests
workflows/conditional.baml     # Conditional workflow
shared/workflows/shared.baml   # Shared functions
llm_only.baml                  # Standalone LLM function
```

**Mock Execution Simulation** (lines 570-811)

**Graph Traversal** (lines 590-701)
- BFS traversal from start node
- Follows edges (including conditionals)
- Handles loops and branches
- Tracks visited nodes

**Node Execution** (lines 706-811)
```typescript
const executeNode = async (nodeId: string) => {
  // Emit started event
  emitEvent({ type: 'node.started', nodeId })

  // Simulate work (500-2000ms)
  const duration = 500 + Math.random() * 1500
  await sleep(duration)

  // Check for errors/cache
  const shouldError = Math.random() < errorRate
  const isCacheHit = Math.random() < cacheHitRate

  if (shouldError) {
    emitEvent({
      type: 'node.error',
      nodeId,
      error: 'Mock error'
    })
  } else {
    emitEvent({
      type: 'node.completed',
      nodeId,
      result: mockOutput,
      cached: isCacheHit,
      duration
    })
  }
}
```

**Mock Output Generation** (lines 831-983)
- Workflow-specific outputs
- Realistic JSON structures
- Includes metadata (timestamps, tokens)

**Validation** (lines 116-156)
- Validates all test references on init
- Ensures test→function mappings are valid
- Prevents runtime errors

### Preserving Debug Capabilities in Merged App

**Requirements:**
1. ✅ Run app in browser without VSCode
2. ✅ Load mock project with files, workflows, tests
3. ✅ Simulate clicking on functions/tests
4. ✅ Verify navigation logic works
5. ✅ Test execution flows
6. ✅ Toggle between mock and real mode

**Proposed Implementation:**

**1. Dev Mode Toggle UI**
```tsx
// In merged app header/settings
{import.meta.env.DEV && (
  <div className="dev-controls">
    <Toggle
      checked={mockModeEnabled}
      onChange={setMockMode}
      label="Mock Mode"
    />
    {mockModeEnabled && <DebugPanel />}
  </div>
)}
```

**2. SDK Config from Environment**
```typescript
// In BAMLSDKProvider
const config: BAMLSDKConfig = useMemo(() => {
  if (import.meta.env.DEV && mockModeEnabled) {
    return { mode: 'mock', provider: new MockDataProvider() }
  } else if (vscode.isVscode()) {
    return { mode: 'vscode', provider: new VSCodeDataProvider() }
  } else {
    return { mode: 'server', provider: new ServerDataProvider() }
  }
}, [mockModeEnabled])
```

**3. Mock Data Provider**
```typescript
class MockDataProvider implements DataProvider {
  async getWorkflows() { return MOCK_WORKFLOWS }
  async getFiles() { return MOCK_BAML_FILES }
  async executeWorkflow(id, opts) {
    return mockExecutionSimulation(id, opts)
  }
  // ... other methods
}

class VSCodeDataProvider implements DataProvider {
  async getWorkflows() {
    // Use runtimeAtom from playground-common
    const runtime = await vscode.getRuntime()
    return runtime.list_workflows()
  }
  async getFiles() {
    return store.get(filesAtom)
  }
  async executeWorkflow(id, opts) {
    // Trigger real BAML runtime execution
    return runtime.execute_workflow(id, opts)
  }
}
```

**4. Debug Panel Conditional Rendering**
```tsx
// App.tsx
const showDebugPanel = import.meta.env.DEV && mockModeEnabled

return (
  <div className="app">
    {showDebugPanel && <DebugPanel />}
    <WorkflowEditor />
    <DetailPanel />
  </div>
)
```

**Benefits:**
- ✅ Production builds have no debug overhead
- ✅ Development can test without VSCode
- ✅ Same codebase for both modes
- ✅ Easy to toggle during development

---

## Proposed Merge Strategy

### High-Level Approach

**Phase 1: Foundation**
1. Copy baml-graph SDK to playground-common
2. Adapt EventListener to call SDK methods
3. Create hybrid state management

**Phase 2: Features**
4. Integrate graph visualization into playground-common
5. Add debug panel as dev-only feature
6. Merge atoms (unified namespace)

**Phase 3: Testing**
7. Test in VSCode extension
8. Test in standalone playground
9. Test in mock mode

### Detailed Design

#### 1. SDK Integration into playground-common

**New Structure:**
```
packages/playground-common/src/
├── sdk/                           # NEW: from baml-graph
│   ├── index.ts                   # SDK class
│   ├── provider.tsx               # BAMLSDKProvider
│   ├── types.ts                   # Types
│   ├── navigationHeuristic.ts     # Navigation logic
│   ├── adapter.ts                 # Graph conversions
│   ├── providers/                 # NEW: data providers
│   │   ├── base.ts                # DataProvider interface
│   │   ├── mock.ts                # MockDataProvider
│   │   ├── vscode.ts              # VSCodeDataProvider
│   │   └── server.ts              # ServerDataProvider (future)
│   └── atoms/                     # NEW: SDK atoms
│       ├── workflow.atoms.ts      # Workflow state
│       ├── execution.atoms.ts     # Execution state
│       ├── ui.atoms.ts            # UI state
│       └── derived.atoms.ts       # Derived state
├── baml_wasm_web/                 # EXISTING
│   ├── EventListener.tsx          # MODIFY: call SDK
│   └── JotaiProvider.tsx          # KEEP
├── shared/
│   └── baml-project-panel/
│       ├── atoms.ts               # MERGE with sdk/atoms
│       └── ...
└── features/                      # NEW: from baml-graph
    ├── debug/
    ├── graph/
    ├── navigation/
    └── execution/
```

**SDK Modifications:**
```typescript
// sdk/index.ts
export class BAMLSDK {
  constructor(
    private store: ReturnType<typeof createStore>,
    private config: BAMLSDKConfig,
    private provider: DataProvider  // NEW: abstracted data source
  ) {}

  // Existing APIs
  workflows = { ... }
  executions = { ... }
  graph = { ... }
  cache = { ... }
  testCases = { ... }

  // NEW: Files API (from playground-common)
  files = {
    getAll: () => this.provider.getFiles(),
    update: (files: Record<string, string>) => {
      this.store.set(filesAtom, files)
    }
  }

  // NEW: Runtime API (from playground-common)
  runtime = {
    get: () => this.store.get(runtimeAtom),
    getDiagnostics: () => this.store.get(diagnosticsAtom)
  }

  // NEW: Tests API (merged with existing)
  tests = {
    run: async (fnName: string, testName?: string) => {
      if (this.config.mode === 'mock') {
        return this.mockTestExecution(fnName, testName)
      } else {
        return this.vscodeTestExecution(fnName, testName)
      }
    },
    cancel: () => {
      this.store.get(currentAbortControllerAtom)?.abort()
    }
  }
}
```

**Data Provider Interface:**
```typescript
// sdk/providers/base.ts
export interface DataProvider {
  // Workflows
  getWorkflows(): Promise<Workflow[]>

  // Files
  getFiles(): Promise<Record<string, string>>

  // Execution
  executeWorkflow(id: string, opts: ExecutionOptions): Promise<ExecutionResult>
  executeTest(fnName: string, testName?: string): Promise<TestResult>

  // Graph
  getGraph(workflowId: string): Promise<Graph>
  updateNodePositions(workflowId: string, positions: NodePosition[]): Promise<void>

  // Cache
  getCache(nodeId: string, inputHash: string): Promise<CacheEntry | undefined>
  setCache(nodeId: string, inputHash: string, result: any): Promise<void>

  // Navigation
  jumpToFile(span: WasmSpan): Promise<void>

  // Settings
  getSettings(): Promise<Settings>
  updateSettings(settings: Partial<Settings>): Promise<void>
}
```

**VSCode Provider Implementation:**
```typescript
// sdk/providers/vscode.ts
export class VSCodeDataProvider implements DataProvider {
  constructor(
    private store: ReturnType<typeof createStore>,
    private vscodeApi: VSCodeAPIWrapper
  ) {}

  async getWorkflows(): Promise<Workflow[]> {
    const runtime = this.store.get(runtimeAtom)
    if (!runtime?.rt) return []
    return runtime.rt.list_workflows() // If WASM has this method
  }

  async getFiles(): Promise<Record<string, string>> {
    return this.store.get(filesAtom)
  }

  async executeTest(fnName: string, testName?: string): Promise<TestResult> {
    const runtime = this.store.get(runtimeAtom)
    const testCase = this.store.get(testCaseAtom([fnName, testName]))
    // Use existing playground-common test execution logic
    return await runBamlTest(runtime, testCase, this.vscodeApi)
  }

  async jumpToFile(span: WasmSpan): Promise<void> {
    return this.vscodeApi.jumpToFile(span)
  }

  // ... other methods
}
```

**Mock Provider Implementation:**
```typescript
// sdk/providers/mock.ts (from baml-graph/src/sdk/mock.ts)
export class MockDataProvider implements DataProvider {
  constructor(
    private mockData: MockData = DEFAULT_MOCK_DATA,
    private config: MockConfig = DEFAULT_MOCK_CONFIG
  ) {}

  async getWorkflows() {
    return this.mockData.workflows
  }

  async executeWorkflow(id, opts) {
    return mockExecutionSimulation(id, opts, this.mockData, this.config)
  }

  // ... (rest of mock.ts logic)
}
```

#### 2. Modified EventListener

**EventListener as SDK Adapter:**
```typescript
// baml_wasm_web/EventListener.tsx
export function EventListener() {
  const sdk = useBAMLSDK()

  useEffect(() => {
    const handler = async (event: MessageEvent) => {
      const { source, payload } = event.data

      if (source === 'ide_message') {
        if (payload.command === 'update_cursor') {
          // Use SDK
          sdk.navigation.updateCursor(
            payload.content.fileName,
            payload.content.line,
            payload.content.column
          )
        }
        if (payload.command === 'baml_settings_updated') {
          sdk.settings.update(payload.content)
        }
      }

      if (source === 'lsp_message') {
        if (payload.method === 'runtime_updated') {
          // Use SDK
          await sdk.files.update(payload.params.files)
        }
        if (payload.method === 'workspace/executeCommand') {
          if (payload.params.command === 'baml.runBamlTest') {
            const [fnName, testName] = payload.params.arguments
            await sdk.tests.run(fnName, testName)
          }
          if (payload.params.command === 'baml.openBamlPanel') {
            const [fnName] = payload.params.arguments
            sdk.navigation.selectFunction(fnName)
          }
        }
      }
    }

    window.addEventListener('message', handler)
    return () => window.removeEventListener('message', handler)
  }, [sdk])

  return null
}
```

**Benefits:**
- EventListener handles message parsing and IDE quirks
- SDK handles business logic
- Easy to test separately
- Clear separation of concerns

#### 3. Unified Atom Namespace

**Strategy: Merge atoms while preserving both use cases**

**atoms.ts (unified):**
```typescript
// WASM & Compilation (from playground-common)
export const wasmAtom = atomAsync(async () => { ... })
export const filesAtom = atom<Record<string, string>>({})
export const projectAtom = atom(get => { ... })
export const runtimeAtom = atom(get => { ... })
export const diagnosticsAtom = atom(get => { ... })

// Workflows (from baml-graph, uses runtimeAtom)
export const workflowsAtom = atom(get => {
  const runtime = get(runtimeAtom)
  if (!runtime?.rt) return []
  // Extract workflows from runtime (if WASM supports)
  // OR use SDK provider
  return get(sdkWorkflowsAtom)
})

export const activeWorkflowIdAtom = atom<string | undefined>(undefined)
export const activeWorkflowAtom = atom(get => {
  const id = get(activeWorkflowIdAtom)
  const workflows = get(workflowsAtom)
  return workflows.find(w => w.id === id)
})

// Function Selection (merge playground-common + baml-graph)
export const selectedFunctionAtom = atom<string | undefined>(undefined)
export const selectedTestcaseAtom = atom<string | undefined>(undefined)

// Execution State (from baml-graph)
export const workflowExecutionsAtomFamily = atomFamily((workflowId: string) =>
  atom<ExecutionSnapshot[]>([])
)
export const nodeStateAtomFamily = atomFamily((nodeId: string) =>
  atom<NodeExecutionState>({ status: 'not-started' })
)

// Test Execution (from playground-common)
export const testCaseResponseAtom = atomFamily(/* ... */)
export const areTestsRunningAtom = atom(false)

// UI State (merge both)
export const viewModeAtom = atom<'editor' | 'snapshot'>('editor')
export const selectedNodeIdAtom = atom<string | undefined>(undefined)
export const detailPanelAtom = atom<DetailPanelState>({ open: false })
export const flashRangesAtom = atom<FlashRange[]>([])

// Settings (from playground-common)
export const vscodeSettingsAtom = atomAsync(/* ... */)
export const proxyUrlAtom = atom(/* ... */)
export const apiKeysAtom = atom(/* ... */)

// Feature Flags (merge)
export const betaFeatureEnabledAtom = atom(get => {
  // Check both VSCode settings and standalone flags
  const vscodeSettings = get(vscodeSettingsAtom)
  const standalonFlags = get(standaloneFeatureFlagsAtom)
  return vscodeSettings?.config?.featureFlags?.includes('beta') ||
         standaloneFlags?.beta ||
         false
})

// Dev Mode (NEW)
export const mockModeEnabledAtom = atomWithStorage(
  'baml:devMode:mockEnabled',
  false
)
export const debugPanelVisibleAtom = atom(get => {
  return import.meta.env.DEV && get(mockModeEnabledAtom)
})
```

**Migration Strategy:**
1. Keep both atom sets initially
2. Create adapter atoms that map between them
3. Gradually migrate components to unified atoms
4. Remove old atoms once migration complete

#### 4. Graph Visualization Integration

**New Component Structure:**
```
packages/playground-common/src/
├── shared/
│   └── baml-project-panel/
│       ├── playground-panel/       # EXISTING: function list, tests
│       ├── graph-panel/            # NEW: from baml-graph
│       │   ├── WorkflowGraph.tsx   # ReactFlow graph
│       │   ├── nodes/              # Custom nodes
│       │   ├── edges/              # Custom edges
│       │   └── hooks/
│       │       ├── useGraphSync.ts
│       │       └── useExecutionSync.ts
│       └── views/                  # NEW: unified views
│           ├── FunctionView.tsx    # Shows single function (existing)
│           ├── WorkflowView.tsx    # Shows workflow graph (new)
│           └── LLMOnlyView.tsx     # LLM-only view (new)
```

**Unified View Component:**
```tsx
// shared/baml-project-panel/views/UnifiedView.tsx
export function UnifiedView() {
  const activeWorkflow = useAtomValue(activeWorkflowAtom)
  const selectedFunction = useAtomValue(selectedFunctionAtom)
  const isLLMOnly = useAtomValue(isLLMOnlyModeAtom)

  if (isLLMOnly && selectedFunction) {
    return <LLMOnlyView functionName={selectedFunction} />
  }

  if (activeWorkflow) {
    return <WorkflowView workflow={activeWorkflow} />
  }

  if (selectedFunction) {
    return <FunctionView functionName={selectedFunction} />
  }

  return <EmptyState />
}
```

**Conditional Graph Rendering:**
```tsx
// WorkflowView.tsx
export function WorkflowView({ workflow }: { workflow: Workflow }) {
  const graph = useCurrentGraph() // From SDK
  const [nodes, setNodes, onNodesChange] = useNodesState([])
  const [edges, setEdges, onEdgesChange] = useEdgesState([])

  // Sync from SDK graph to ReactFlow
  useGraphSync(graph, setNodes, setEdges)

  // Sync execution state to node styles
  useExecutionSync(nodes, setNodes)

  return (
    <div className="workflow-view">
      <WorkflowToolbar workflow={workflow} />
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={NODE_TYPES}
        edgeTypes={EDGE_TYPES}
      />
      <DetailPanel />
    </div>
  )
}
```

#### 5. Debug Panel as Dev-Only Feature

**Conditional Loading:**
```tsx
// App.tsx or main layout
const showDebugPanel = import.meta.env.DEV && useAtomValue(debugPanelVisibleAtom)

return (
  <div className="app">
    {showDebugPanel && <DebugPanel />}
    <UnifiedView />
  </div>
)
```

**Dev Mode Toggle:**
```tsx
// In settings panel or header
{import.meta.env.DEV && (
  <div className="dev-controls">
    <label>
      <input
        type="checkbox"
        checked={mockModeEnabled}
        onChange={(e) => setMockMode(e.target.checked)}
      />
      Enable Mock Mode
    </label>
    {mockModeEnabled && (
      <label>
        <input
          type="checkbox"
          checked={debugPanelVisible}
          onChange={(e) => setDebugPanelVisible(e.target.checked)}
        />
        Show Debug Panel
      </label>
    )}
  </div>
)}
```

**Debug Panel Integration:**
```tsx
// features/debug/components/DebugPanel.tsx (mostly unchanged)
export function DebugPanel() {
  const bamlFiles = useAtomValue(bamlFilesAtom)
  const [activeCodeClick, setActiveCodeClick] = useAtom(activeCodeClickAtom)
  const sdk = useBAMLSDK()

  const handleFunctionClick = (fn: WasmFunction) => {
    setActiveCodeClick({
      type: 'function',
      filePath: fn.span().file_path,
      name: fn.name,
      span: fn.span
    })
  }

  const handleTestClick = (test: WasmTestCase) => {
    setActiveCodeClick({
      type: 'test',
      filePath: test.file_path,
      functionName: test.function_name,
      testName: test.name
    })
  }

  const handleRunTest = async (test: WasmTestCase) => {
    await sdk.tests.run(test.function_name, test.name)
  }

  return (/* existing UI */)
}
```

#### 6. Package Exports

**Updated package.json:**
```json
{
  "exports": {
    ".": "./src/index.ts",
    "./jotai-provider": "./src/baml_wasm_web/JotaiProvider.tsx",
    "./event-listener": "./src/baml_wasm_web/EventListener.tsx",
    "./sdk": "./src/sdk/index.ts",
    "./sdk/provider": "./src/sdk/provider.tsx",
    "./atoms": "./src/shared/baml-project-panel/atoms.ts",
    "./prompt-preview": "./src/shared/baml-project-panel/...",
    "./workflow-graph": "./src/shared/baml-project-panel/graph-panel/WorkflowGraph.tsx",
    "./debug-panel": "./src/features/debug/components/DebugPanel.tsx"
  }
}
```

### Testing Strategy

#### Unit Tests
```typescript
// SDK tests (sdk/index.test.ts)
describe('BAMLSDK', () => {
  it('switches workflows', () => {
    const sdk = new BAMLSDK(store, { mode: 'mock', provider })
    sdk.workflows.setActive('workflow-1')
    expect(sdk.workflows.getActive()?.id).toBe('workflow-1')
  })

  it('executes workflow in mock mode', async () => {
    const sdk = new BAMLSDK(store, { mode: 'mock', provider })
    const result = await sdk.executions.start('workflow-1')
    expect(result.status).toBe('completed')
  })
})

// EventListener tests (baml_wasm_web/EventListener.test.tsx)
describe('EventListener', () => {
  it('translates runtime_updated to SDK call', async () => {
    const sdk = createMockSDK()
    render(<EventListener />, { sdk })

    window.postMessage({
      source: 'lsp_message',
      payload: {
        method: 'runtime_updated',
        params: { files: { 'test.baml': 'content' } }
      }
    })

    await waitFor(() => {
      expect(sdk.files.update).toHaveBeenCalledWith({ 'test.baml': 'content' })
    })
  })
})

// Navigation tests (navigationHeuristic.test.ts)
describe('determineNavigationAction', () => {
  it('selects node when function in current workflow', () => {
    const action = determineNavigationAction(
      { type: 'function', name: 'fetchData' },
      'workflow-1',
      workflows
    )
    expect(action.type).toBe('select-node')
  })
})
```

#### Integration Tests
```typescript
// Full flow test
describe('Workflow execution flow', () => {
  it('executes workflow and updates UI', async () => {
    const { getByText, getAllByTestId } = render(<App />, {
      sdk: createMockSDK({ mode: 'mock' })
    })

    // Select workflow
    fireEvent.click(getByText('Simple Workflow'))

    // Start execution
    fireEvent.click(getByText('Run Workflow'))

    // Wait for nodes to update
    await waitFor(() => {
      const nodes = getAllByTestId('workflow-node')
      expect(nodes[0]).toHaveClass('node-running')
    })

    await waitFor(() => {
      expect(nodes[0]).toHaveClass('node-completed')
    }, { timeout: 3000 })
  })
})
```

#### VSCode Extension Tests
```typescript
// apps/vscode-ext/src/test/suite/webview.test.ts
describe('Webview integration', () => {
  it('sends runtime_updated when files change', async () => {
    const panel = await createWebviewPanel()

    // Simulate file change
    await workspace.fs.writeFile(
      vscode.Uri.file('test.baml'),
      Buffer.from('new content')
    )

    // Check webview received message
    const messages = await panel.getReceivedMessages()
    expect(messages).toContainEqual({
      source: 'lsp_message',
      payload: {
        method: 'runtime_updated',
        params: expect.objectContaining({
          files: expect.objectContaining({ 'test.baml': 'new content' })
        })
      }
    })
  })
})
```

#### Browser Tests (Mock Mode)
```typescript
// Playwright/Cypress test
describe('Mock mode in browser', () => {
  it('loads mock data and shows debug panel', async () => {
    await page.goto('http://localhost:5173')

    // Enable mock mode
    await page.click('[data-testid="settings-toggle"]')
    await page.click('[data-testid="mock-mode-toggle"]')

    // Verify debug panel appears
    await expect(page.locator('[data-testid="debug-panel"]')).toBeVisible()

    // Click function in debug panel
    await page.click('text=fetchData')

    // Verify navigation
    await expect(page.locator('[data-testid="workflow-graph"]')).toBeVisible()
    await expect(page.locator('[data-selected-node="fetchData"]')).toHaveClass(/selected/)
  })
})
```

---

## Implementation Plan

### Phase 1: Foundation (Week 1-2)

**Goal:** Establish SDK in playground-common without breaking existing functionality

**Tasks:**
1. [ ] Copy SDK files from baml-graph to playground-common
   - `src/sdk/` directory structure
   - Atoms, hooks, types
   - Navigation heuristic
   - Mock data provider

2. [ ] Create DataProvider interface and implementations
   - `sdk/providers/base.ts` - Interface definition
   - `sdk/providers/mock.ts` - Mock provider (from baml-graph)
   - `sdk/providers/vscode.ts` - VSCode provider (use existing atoms/WASM)

3. [ ] Set up BAMLSDKProvider
   - Configure based on environment (dev vs production)
   - Support mode switching (mock/vscode)
   - Initialize with appropriate provider

4. [ ] Add dev mode atoms
   - `mockModeEnabledAtom`
   - `debugPanelVisibleAtom`

5. [ ] Update package.json exports
   - Export SDK and provider
   - Export new atoms

**Validation:**
- [ ] Existing VSCode extension still works
- [ ] Can instantiate SDK in test environment
- [ ] Mock provider loads and provides data

### Phase 2: EventListener Integration (Week 2-3)

**Goal:** Make EventListener call SDK methods instead of directly updating atoms

**Tasks:**
1. [ ] Add SDK methods for all EventListener actions
   - `sdk.files.update(files)`
   - `sdk.navigation.updateCursor(file, line, col)`
   - `sdk.tests.run(fnName, testName)`
   - `sdk.settings.update(settings)`

2. [ ] Modify EventListener to use SDK
   - Replace direct atom updates with SDK calls
   - Preserve IDE-specific logic (delays, etc.)
   - Add error handling

3. [ ] Test integration
   - VSCode cursor updates
   - File updates from LSP
   - Test execution
   - Settings updates

4. [ ] Add VSCodeDataProvider methods
   - Implement all DataProvider interface methods
   - Use existing WASM runtime
   - Use existing RPC methods

**Validation:**
- [ ] VSCode extension works with SDK
- [ ] Cursor updates trigger correct navigation
- [ ] File updates compile correctly
- [ ] Test execution works

### Phase 3: Unified Atoms (Week 3-4)

**Goal:** Merge atom namespaces from both apps

**Tasks:**
1. [ ] Create unified atoms.ts
   - Merge workflow and function selection atoms
   - Keep WASM/compilation atoms from playground-common
   - Add execution atoms from baml-graph
   - Merge UI state atoms

2. [ ] Create adapter atoms for migration
   - Map old atom names to new unified atoms
   - Ensure backward compatibility

3. [ ] Update SDK to use unified atoms
   - Reference correct atom names
   - Update hooks

4. [ ] Migrate components incrementally
   - Start with leaf components
   - Update to use unified atoms
   - Test each migration

**Validation:**
- [ ] No duplicate state
- [ ] All components render correctly
- [ ] State updates propagate correctly

### Phase 4: Graph Visualization (Week 4-5)

**Goal:** Integrate workflow graph from baml-graph

**Tasks:**
1. [ ] Copy graph components
   - `features/graph/` directory
   - `graph-primitives/` directory
   - ReactFlow setup

2. [ ] Create WorkflowView component
   - Use useCurrentGraph hook
   - Set up ReactFlow
   - Add useGraphSync and useExecutionSync hooks

3. [ ] Create UnifiedView router
   - Switch between FunctionView and WorkflowView
   - Handle LLM-only mode
   - Handle empty state

4. [ ] Update main layout
   - Replace existing function view with UnifiedView
   - Preserve existing UI (function list, test panel)

5. [ ] Add dependencies
   - @xyflow/react
   - ELK layout library
   - Other graph dependencies

**Validation:**
- [ ] Graph renders for workflows
- [ ] Single functions still render as before
- [ ] Node selection works
- [ ] Execution state updates nodes visually

### Phase 5: Navigation Integration (Week 5-6)

**Goal:** Integrate navigation system from baml-graph

**Tasks:**
1. [ ] Copy navigation feature
   - `features/navigation/` directory
   - `sdk/navigationHeuristic.ts`

2. [ ] Add navigation atoms
   - `activeCodeClickAtom`
   - Update `activeWorkflowIdAtom` usage

3. [ ] Integrate useCodeNavigation hook
   - Call from EventListener (for baml.openBamlPanel)
   - Call from DebugPanel (for clicks)

4. [ ] Test navigation heuristics
   - Test → workflow navigation
   - Function → workflow navigation
   - Standalone function navigation
   - Camera panning

**Validation:**
- [ ] Clicking function in VSCode opens correct workflow
- [ ] Clicking test in VSCode runs correct test
- [ ] Navigation heuristic follows priority rules

### Phase 6: Debug Panel (Week 6)

**Goal:** Add debug panel for browser testing

**Tasks:**
1. [ ] Copy debug feature
   - `features/debug/` directory
   - DebugPanel component

2. [ ] Add dev mode UI
   - Settings toggle for mock mode
   - Toggle for debug panel visibility

3. [ ] Conditional rendering
   - Only show in development builds
   - Only when mock mode enabled

4. [ ] Wire up mock data
   - Ensure MockDataProvider provides BAML files
   - DebugPanel displays files from SDK

**Validation:**
- [ ] Debug panel not in production builds
- [ ] Can toggle mock mode in dev
- [ ] Can click functions/tests in debug panel
- [ ] Navigation works from debug panel clicks

### Phase 7: Testing & Polish (Week 7-8)

**Goal:** Comprehensive testing and bug fixes

**Tasks:**
1. [ ] Unit tests
   - SDK methods
   - Navigation heuristic
   - Data providers
   - Atom logic

2. [ ] Integration tests
   - EventListener → SDK flow
   - Execution flow
   - Navigation flow

3. [ ] VSCode extension tests
   - Message passing
   - RPC calls
   - File synchronization

4. [ ] Browser tests
   - Mock mode
   - Debug panel
   - Navigation

5. [ ] Performance testing
   - Large workflows
   - Many nodes
   - Execution updates

6. [ ] Bug fixes and polish
   - Edge cases
   - Error handling
   - Loading states

**Validation:**
- [ ] All tests passing
- [ ] No regressions in VSCode extension
- [ ] Mock mode works in browser
- [ ] Performance acceptable

### Phase 8: Documentation (Week 8)

**Goal:** Document architecture and usage

**Tasks:**
1. [ ] Architecture documentation
   - SDK design
   - Data provider pattern
   - Atom organization
   - Integration patterns

2. [ ] Usage documentation
   - How to use in VSCode
   - How to test in browser
   - How to add new features

3. [ ] Migration guide
   - For contributors familiar with old structure
   - Atom mapping
   - Component locations

4. [ ] API documentation
   - SDK public API
   - Key hooks
   - Important atoms

**Deliverables:**
- [ ] README updates
- [ ] Architecture diagram
- [ ] API reference
- [ ] Migration guide

---

## Open Questions and Decisions

### 1. WASM Support for Workflows

**Question:** Does the current BAML WASM runtime support workflow primitives?

**Context:**
- baml-graph SDK expects `runtime.list_workflows()`, `runtime.get_workflow(id)`, etc.
- Current playground-common uses WASM for functions, tests, but unclear if workflows are supported

**Options:**
A. WASM already supports workflows → Use directly
B. WASM doesn't support workflows yet → Add WASM methods first
C. Workflows are client-side only → Keep in SDK layer

**Decision needed from:** @team

**Impact:**
- Affects VSCodeDataProvider implementation
- May require WASM updates before merge
- Could affect timeline

---

### 2. Graph Data Persistence

**Question:** Where should workflow graph layouts be persisted?

**Context:**
- baml-graph has `sdk.graph.updateNodePositions()` for saving layouts
- Need to decide on storage mechanism

**Options:**
A. VSCode workspace storage (`.vscode/baml-layouts.json`)
B. Project-local file (`.baml/layouts.json`)
C. In-memory only (reset on reload)
D. Server-side (for standalone playground)

**Recommendation:** Option A for VSCode, Option D for standalone

**Decision needed from:** @team

---

### 3. Feature Flag Strategy

**Question:** How should we roll out the graph view?

**Context:**
- Large change to UI
- Want to ensure stability
- May want gradual rollout

**Options:**
A. Behind beta feature flag (user opt-in)
B. Default on for new users, opt-in for existing
C. Default on for everyone
D. Separate command (`baml.openGraphView` vs `baml.openBamlPanel`)

**Recommendation:** Option A initially, move to C after testing

**Decision needed from:** @team

---

### 4. Prompt Preview Integration

**Question:** How does the existing prompt preview fit with workflow graphs?

**Context:**
- Existing prompt preview shows single function execution
- Workflows may have multiple LLM calls
- Need unified UX

**Options:**
A. Prompt preview shows active node's prompt in workflow
B. Separate prompt preview for each node (tabbed)
C. Keep existing prompt preview, workflows use different UI
D. Detail panel shows prompt for selected node

**Recommendation:** Option D (most flexible)

**Decision needed from:** @team, @design

---

### 5. Execution State Persistence

**Question:** Should execution history be persisted?

**Context:**
- baml-graph tracks execution snapshots
- Useful for debugging
- Could grow large

**Options:**
A. In-memory only (cleared on reload)
B. Session storage (cleared when webview closed)
C. Workspace storage (persisted across sessions, size limit)
D. Optional persistence (user preference)

**Recommendation:** Option B for VSCode, A for standalone

**Decision needed from:** @team

---

### 6. Mock Data Management

**Question:** Should mock data be customizable?

**Context:**
- Current mock data is hardcoded
- Developers may want to test specific scenarios

**Options:**
A. Hardcoded only (current approach)
B. Load from JSON file (`.baml/mock-data.json`)
C. UI for creating mock workflows
D. Import real BAML project in browser

**Recommendation:** Option B (easy to implement, flexible)

**Decision needed from:** @team

---

### 7. Test Execution in Workflows

**Question:** How should test execution work for workflows vs functions?

**Context:**
- Current: run test for single function
- Workflows: multiple nodes, may want to test end-to-end

**Options:**
A. Tests stay function-scoped, workflows show aggregated results
B. Add workflow-level tests
C. Test specific path through workflow
D. Parameterize workflow execution with test inputs

**Recommendation:** Option A initially, Option B in future

**Decision needed from:** @team

---

### 8. Bundle Size Concerns

**Question:** Is the added bundle size acceptable?

**Context:**
- ReactFlow + dependencies add ~500KB
- Graph layouts (ELK) add ~300KB
- Total: ~800KB added to VSCode extension

**Options:**
A. Accept the size increase
B. Lazy load graph components (code splitting)
C. Make graph view a separate extension
D. Use lighter graph library

**Recommendation:** Option B (lazy load when graph opened)

**Decision needed from:** @team

---

### 9. Standalone Playground Architecture

**Question:** Should standalone playground be a separate app or same codebase?

**Context:**
- Currently `apps/playground` imports from `playground-common`
- baml-graph is standalone app
- Want to avoid duplication

**Options:**
A. Keep apps/playground, add graph support
B. Replace apps/playground with baml-graph
C. Unified app with route-based views
D. Two separate apps (function playground + workflow playground)

**Recommendation:** Option A (incremental, less risk)

**Decision needed from:** @team

---

### 10. Backward Compatibility

**Question:** How important is backward compatibility for existing atom consumers?

**Context:**
- Merging atoms may break external consumers
- Unclear if other apps use playground-common directly

**Options:**
A. Breaking changes OK (major version bump)
B. Keep deprecated atoms with adapters
C. Gradual migration with deprecation warnings
D. Maintain both versions (branching)

**Recommendation:** Option C (safest approach)

**Decision needed from:** @team

---

## Appendix: Key File Locations

### baml-graph

**SDK:**
- SDK class: `apps/baml-graph/src/sdk/index.ts:39-508`
- Provider: `apps/baml-graph/src/sdk/provider.tsx:23-82`
- Types: `apps/baml-graph/src/sdk/types.ts`
- Mock data: `apps/baml-graph/src/sdk/mock.ts:93-1094`
- Navigation: `apps/baml-graph/src/sdk/navigationHeuristic.ts:108-299`

**Atoms:**
- Workflow: `apps/baml-graph/src/sdk/atoms/workflow.atoms.ts`
- Execution: `apps/baml-graph/src/sdk/atoms/execution.atoms.ts`
- UI: `apps/baml-graph/src/sdk/atoms/ui.atoms.ts`
- Derived: `apps/baml-graph/src/sdk/atoms/derived.atoms.ts`

**Hooks:**
- SDK hooks: `apps/baml-graph/src/sdk/hooks.ts`

**Features:**
- Debug panel: `apps/baml-graph/src/features/debug/components/DebugPanel.tsx:15-202`
- Navigation: `apps/baml-graph/src/features/navigation/hooks/useCodeNavigation.ts:28-170`
- Graph sync: `apps/baml-graph/src/features/graph/hooks/useGraphSync.ts:16-86`
- Execution sync: `apps/baml-graph/src/features/execution/hooks/useExecutionSync.ts:24-78`

**Entry:**
- Main: `apps/baml-graph/src/main.tsx:1-14`
- App: `apps/baml-graph/src/App.tsx:35-227`

### playground-common

**Core:**
- Jotai provider: `packages/playground-common/src/baml_wasm_web/JotaiProvider.tsx:1-116`
- EventListener: `packages/playground-common/src/baml_wasm_web/EventListener.tsx:57-217`
- VSCode API: `packages/playground-common/src/shared/baml-project-panel/vscode.ts:63-535`

**Atoms:**
- Core atoms: `packages/playground-common/src/shared/baml-project-panel/atoms.ts`
- Playground atoms: `packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts`
- API keys atoms: `packages/playground-common/src/components/api-keys-dialog/atoms.ts`

**Test Execution:**
- Test runner: `packages/playground-common/src/shared/baml-project-panel/prompt-preview/test-panel/test-runner.ts:538-629`

### VSCode Extension

**Integration:**
- Extension entry: `apps/vscode-ext/src/extension.ts:162-196`
- Webview host: `apps/vscode-ext/src/panels/WebviewPanelHost.ts:92-134`
- Message handling: `apps/vscode-ext/src/panels/WebviewPanelHost.ts:415-772`
- LSP integration: `apps/vscode-ext/src/plugins/language-server-client/index.ts:513-524`

**Message Types:**
- Webview → VSCode: `apps/vscode-ext/src/webview-to-vscode-rpc.ts:145-159`
- VSCode → Webview: `apps/vscode-ext/src/panels/vscode-to-webview-rpc.ts:4-73`

---

## Conclusion

This design document outlines a comprehensive strategy for merging baml-graph with playground-common. The key insight is that **both architectures have strengths**:

- **baml-graph** provides excellent abstraction (SDK), testability (mock mode), and navigation UX
- **playground-common** provides mature WASM integration, platform support, and production stability

The proposed **hybrid approach** combines the best of both:
1. Adopt the SDK pattern for business logic and API surface
2. Keep the EventListener for platform integration
3. Merge atoms into unified namespace
4. Add graph visualization while preserving existing UI
5. Enable browser testing with mock mode and debug panel

The phased implementation plan minimizes risk by:
- Preserving backward compatibility
- Incremental migration
- Thorough testing at each phase
- Clear rollback points

**Timeline:** 8 weeks (2 months) with 1-2 developers

**Risk Level:** Medium (substantial refactoring, but well-planned)

**User Impact:** High value (workflow visualization, better testing, unified UX)

---

**Next Steps:**
1. Review and approve this design doc
2. Answer open questions
3. Begin Phase 1 implementation
4. Set up project tracking (GitHub project, milestones)
