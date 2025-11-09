# Playground Common Package - Detailed Exploration Report

## Project Context

**Repository**: BAML TypeScript Workspace  
**Working Directory**: `/Users/aaronvillalpando/Projects/baml/typescript`  
**Current Branch**: `aaron/graphs`  
**Target Package**: `packages/playground-common`

---

## Executive Summary

The `playground-common` package is a comprehensive state management and component library for the BAML playground. It implements a **unified state architecture** using Jotai atoms, a **navigation heuristic system** for handling code click events, and a **debug panel** for simulating IDE interactions with BAML files (functions, tests, workflows).

### Key Architectural Patterns:
1. **Immutable Runtime Pattern**: Runtime instances are recreated on file changes (similar to wasmAtom)
2. **Jotai Atoms for State**: All state is centralized in atoms, organized into core and test atoms
3. **Navigation Heuristic**: Context-aware algorithm determines UI actions based on code clicks
4. **Debug Panel**: Interactive component for testing how the app responds to code navigation events

---

## 1. State Management Architecture

### 1.1 Core Atoms Location
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/atoms/core.atoms.ts`

**Lines: 1-638** (Full file length)

#### State Organization

The atoms are organized into logical sections:

##### A. Core State (Source of Truth)
- **Line 28**: `runtimeInstanceAtom` - The primary runtime instance; all other state is derived from this
- **Line 33**: `workflowsAtom` - All available workflows (derived from runtime)
- **Line 43**: `activeWorkflowIdAtom` - Currently active workflow ID
- **Line 49**: `workflowExecutionsAtomFamily` - Per-workflow execution history
- **Line 56**: `selectedExecutionIdAtom` - Currently viewed execution snapshot

##### B. Node State Management (Lines 59-110)
- **Line 62**: `nodeStateAtomFamily` - Individual node execution states using atomFamily (O(1) updates)
- **Line 70**: `nodeRegistryAtom` - Set of all node IDs that exist
- **Line 75**: `registerNodeAtom` - Write-only atom to register nodes
- **Line 90**: `clearAllNodeStatesAtom` - Write-only atom to reset all node states
- **Line 103**: `allNodeStatesAtom` - Read-only derived atom returning Map of all states

##### C. Cache Management (Lines 115-122)
- **Line 115**: `cacheAtom` - Map<string, CacheEntry> for node execution caching
- **Line 120**: `getCacheKey()` helper - Generates cache keys from nodeId + inputsHash

##### D. Event Stream (Lines 127-139)
- **Line 127**: `eventStreamAtom` - Circular buffer of last 100 BAML events
- **Line 132**: `addEventAtom` - Write-only atom to append events

##### E. UI State (Lines 148-194)
- **Line 148**: `viewModeAtom` - 'editor' | 'execution' mode
- **Line 155**: `selectedNodeIdAtom` - Currently selected node in graph
- **Line 160-170**: `detailPanelAtom` - DetailPanel state (isOpen, position, activeTab)
- **Line 175**: `layoutDirectionAtom` - 'vertical' | 'horizontal' graph layout
- **Line 180**: `selectedInputSourceAtom` - Selected input source for node (execution/test/manual)
- **Line 189**: `activeNodeInputsAtom` - Editable node inputs
- **Line 194**: `inputsDirtyAtom` - Whether inputs have been modified

##### F. Debug Panel State (Lines 199-204)
- **Line 199**: `bamlFilesAtom` - Array of parsed BAML files with functions and tests
- **Line 204**: `activeCodeClickAtom` - Last code click event from IDE/debug panel

##### G. Derived State (Lines 213-285)
- **Line 213**: `activeWorkflowAtom` - Derived: current workflow object
- **Line 223**: `activeWorkflowExecutionsAtom` - Derived: executions for active workflow
- **Line 232**: `selectedExecutionAtom` - Derived: currently viewed execution
- **Line 249**: `latestExecutionAtom` - Derived: most recent execution
- **Line 257**: `nodeExecutionsAtom` - Derived: node executions from latest execution
- **Line 265**: `selectExecutionAtom` - Write-only: select and switch to execution mode
- **Line 278**: `recentWorkflowsAtom` - Derived: last 5 workflows by modification time
- **Line 293**: `allFunctionsMapAtom` - Derived: Map<functionName, FunctionWithCallGraph> for O(1) lookup

##### H. Selection State (Lines 308-385)
- **Line 311**: `selectedFunctionNameAtom` - Currently selected function name
- **Line 316**: `selectedTestCaseNameAtom` - Currently selected test case name
- **Line 322**: `selectedFunctionObjectAtom` - Derived: full function object
- **Line 334**: `selectedTestCaseAtom` - Derived: test case object from selected function
- **Line 346**: `selectionAtom` - Derived: { selectedFn, selectedTc } for backward compatibility
- **Line 355-366**: `updateSelectionAtom` - Write-only: Central update point for selection changes

**Key Update Flow** (Lines 355-366):
```typescript
export const updateSelectionAtom = atom(
  null,
  (get, set, update: { functionName: string | null; testCaseName?: string | null }) => {
    console.log('[updateSelection]', update);
    set(selectedFunctionNameAtom, update.functionName);
    set(selectedTestCaseNameAtom, update.testCaseName ?? null);
  }
);
```

This is the **shared update point** used by both DebugPanel and the code navigation system.

##### I. Diagnostics System (Lines 420-448)
- **Line 405**: `DiagnosticError` interface
- **Line 420**: `diagnosticsAtom` - Derived from runtime
- **Line 428**: `functionsAtom` - All functions from runtime
- **Line 436**: `isRuntimeValid` - Whether runtime has no errors
- **Line 444**: `numErrorsAtom` - Error and warning counts

##### J. Generated Files (Lines 454-476)
- **Line 463**: `generatedFilesAtom` - Generated code files from runtime
- **Line 471**: `generatedFilesByLangAtomFamily` - Per-language filtered files

##### K. Feature Flags & Settings (Lines 485-545)
- **Line 485**: `featureFlagsAtom` - Runtime feature flags
- **Line 490**: `betaFeatureEnabledAtom` - Derived: is beta enabled
- **Line 501**: `bamlFilesTrackedAtom` - Current BAML files by runtime
- **Line 506**: `sandboxFilesTrackedAtom` - Temporary test files
- **Line 520**: `vscodeSettingsAtom` - VSCode settings (async loaded)
- **Line 525**: `playgroundPortAtom` - Playground proxy port
- **Line 530**: `proxyUrlAtom` - Derived: proxy URL config
- **Line 545**: `envVarsAtom` - Environment variables/API keys

##### L. WASM Panic Handling (Lines 391-399)
- **Line 399**: `wasmPanicAtom` - Tracks runtime panics with msg and timestamp

##### M. Runtime & Backward Compatibility (Lines 555-637)
- **Line 555**: `orchIndexAtom` - Orchestration graph index
- **Line 561**: `wasmAtom` - WASM module instance
- **Line 568**: `lastValidWasmAtom` - Most recent error-free WASM runtime
- **Line 579**: `filesAtom` - Derived: current BAML files
- **Line 585**: `currentWasmRuntimeAtom` - Derived: current WasmRuntime instance
- **Line 594**: `runtimeAtom` - Backward compatible runtime state { rt, diags, lastValidRt }
- **Line 616**: `ctxAtom` - Derived: WasmCallContext for WASM operations
- **Line 634**: `versionAtom` - Derived: BAML runtime version

### 1.2 Test Atoms Location
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/atoms/test.atoms.ts`

**Lines: 1-150**

#### Test Execution State

##### Types (Lines 17-60)
- **Line 20-30**: `TestState` union - queued | running | done | error
- **Line 35-41**: `TestHistoryEntry` - Single test execution record
- **Line 46-49**: `TestHistoryRun` - A test run with multiple entries
- **Line 54-60**: `FlashRange` - Code ranges to highlight during execution

##### Atoms (Lines 70-149)
- **Line 70**: `testHistoryAtom` - Array of TestHistoryRun (most recent first)
- **Line 75**: `selectedHistoryIndexAtom` - Currently viewed history index
- **Line 80-84**: `selectedTestHistoryAtom` - Derived: current test history run
- **Line 93**: `areTestsRunningAtom` - Boolean flag for active test execution
- **Line 98**: `currentAbortControllerAtom` - Abort controller for cancelling tests
- **Line 107**: `currentWatchNotificationsAtom` - Array of watch notifications
- **Line 112**: `highlightedBlocksAtom` - Set of highlighted block IDs
- **Line 117**: `flashRangesAtom` - Array of code ranges to flash
- **Line 128-149**: `categorizedNotificationsAtom` - Derived: notifications grouped by type (blocks, streams, regular)

### 1.3 Backward Compatibility Atoms
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/atoms.ts`

**Lines: 1-158** (Re-export wrapper with compatibility aliases)

**Key Aliases**:
- `selectedItemAtom` → `selectionAtom`
- `functionObjectAtom` → `selectedFunctionObjectAtom`
- `testcaseObjectAtom` → `selectedTestCaseAtom`

---

## 2. DebugPanel Component Implementation

### 2.1 Location & Purpose
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/features/debug-panel/components/DebugPanel.tsx`

**Lines: 1-349**

**Purpose**: Simulates clicking on functions and tests in BAML files to test how the app reacts to code navigation events. This is a development tool for testing the navigation heuristic.

### 2.2 State Management
**Lines 17-34**

```typescript
const sdk = useBAMLSDK();
const { runTests: runBamlTests } = useRunBamlTests();
const [bamlFiles, setBAMLFiles] = useAtom(bamlFilesAtom);
const setActiveCodeClick = useSetAtom(activeCodeClickAtom);
const [activeCodeClick] = useAtom(activeCodeClickAtom);
const updateSelection = useSetAtom(updateSelectionAtom);
const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set());
```

**Data Flow**:
1. Loads BAML files from SDK on mount (lines 27-34)
2. Uses local state for expanded/collapsed file tree (`expandedFiles`)
3. Sets `activeCodeClickAtom` when user clicks function/test
4. Updates `selectedFunctionNameAtom` and `selectedTestCaseNameAtom` via `updateSelectionAtom`

### 2.3 Key Event Handlers

#### Function Click Handler (Lines 50-62)
```typescript
const handleFunctionClick = (func: BAMLFunction) => {
  const event: CodeClickEvent = {
    type: 'function',
    functionName: func.name,
    functionType: func.type,
    filePath: func.filePath,
  };
  setActiveCodeClick(event);
  console.log('🔍 Simulated function click:', event);
  updateSelection({ functionName: func.name, testCaseName: null });
};
```

**Triggered by**: Clicking any function in the file tree (lines 282-296)

#### Test Click Handler (Lines 64-77)
```typescript
const handleTestClick = (test: BAMLTest) => {
  const event: CodeClickEvent = {
    type: 'test',
    testName: test.name,
    functionName: test.functionName,
    filePath: test.filePath,
    nodeType: test.nodeType,
  };
  setActiveCodeClick(event);
  console.log('🔍 Simulated test click:', event);
  updateSelection({ functionName: test.functionName, testCaseName: test.name });
};
```

**Triggered by**: Clicking any test in the file tree (lines 306-323)

#### Test Run Handler (Lines 79-87)
```typescript
const handleTestRun = async (test: BAMLTest, e: React.MouseEvent) => {
  e.stopPropagation();
  console.log('▶️ Running test:', test.name, '→', test.functionName);
  await runBamlTests([{ functionName: test.functionName, testName: test.name }]);
};
```

**Triggered by**: Green play button on hover (lines 315-321)

**Important Note**: Does NOT call `handleTestClick` first - the SDK's test runner will automatically set selection when test history is created.

#### Add File Button (Lines 101-167)
Simulates adding a new BAML file by:
1. Creating new file content with sample functions and tests
2. Getting current files via `sdk.files.getCurrent()`
3. Adding new file: `'baml_src/additional_functions.baml'`
4. Posting `runtime_updated` LSP message to EventListener

#### Modify Function Button (Lines 169-214)
Simulates editing an existing function by:
1. Finding `baml_src/main.baml`
2. Adding timestamp and note to ExtractResume prompt
3. Posting `runtime_updated` LSP message to EventListener

### 2.4 UI Structure (Lines 216-348)

**Layout**: Floating panel in bottom-right corner, max 500px height

**Sections**:
1. **Header** (lines 219-240): Debug title, Add File and Edit Function buttons
2. **File List** (lines 244-331): 
   - Collapsible file tree
   - Functions filtered (excluding workflows)
   - Tests organized by file
   - Active item highlighting (blue background)
3. **Active Event Display** (lines 335-345): Shows last clicked function/test

**Visual Indicators**:
- LLM functions: Purple "LLM" badge
- Active selection: Blue background with `isActive()` check

---

## 3. Navigation System

### 3.1 Navigation Types
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/types.ts`

**Lines: 320-350**

#### Code Click Event Types
```typescript
export interface BAMLFunction {
  name: string;
  type: 'workflow' | 'function' | 'llm_function';
  filePath: string;
}

export interface BAMLTest {
  name: string;
  functionName: string;
  filePath: string;
  nodeType: 'llm_function' | 'function';
}

export interface BAMLFile {
  path: string;
  functions: FunctionWithCallGraph[];
  tests: BAMLTest[];
}

export type CodeClickEvent = {
  type: 'function';
  functionName: string;
  functionType: 'workflow' | 'function' | 'llm_function';
  filePath: string;
} | {
  type: 'test';
  testName: string;
  functionName: string;
  filePath: string;
  nodeType: 'llm_function' | 'function';
};
```

### 3.2 Navigation Heuristic
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/navigationHeuristic.ts`

**Lines: 1-250** (Shown: 1-150)

#### Algorithm Decision Tree (Lines 9-74 in comments)

**For TEST clicks** (Lines 127-???):
1. Find the workflow being tested
2. If workflow exists: Switch to that workflow
3. Otherwise: Show empty state

**For FUNCTION clicks** (Priority-based):
1. **Priority 1**: Check if function exists in current workflow → Select node
2. **Priority 2**: Find another workflow containing this function → Switch and select
3. **Priority 3**: Show function in isolation if it has tests → LLM-only view
4. **Priority 4**: Empty state (no workflow, no tests)

#### Navigation Action Types (Lines 83-88)
```typescript
export type NavigationAction =
  | { type: 'switch-workflow'; workflowId: string }
  | { type: 'select-node'; workflowId: string; nodeId: string; testId?: string }
  | { type: 'switch-and-select'; workflowId: string; nodeId: string; testId?: string }
  | { type: 'show-function-tests'; functionName: string; tests: string[] }
  | { type: 'empty-state'; reason: string; functionName: string };
```

#### Navigation State (Lines 93-100)
```typescript
export interface NavigationState {
  activeWorkflowId: string | null;
  workflows: FunctionWithCallGraph[];
  bamlFiles: BAMLFile[];
}
```

### 3.3 Code Navigation Hook
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/features/navigation/hooks/useCodeNavigation.ts`

**Lines: 1-252**

#### Effect Hook (Lines 46-250)
Listens to `activeCodeClickAtom` changes and executes navigation actions

**Dependencies**: activeCodeClick, sdk, setActiveWorkflow, setSelectedNodeId, selectSource, openDetailPanel, setUnifiedSelection, setActiveTab, setDetailPanelState

#### Action Execution (Lines 62-234)

**switch-workflow** (lines 63-90):
- Checks if workflow exists before switching
- Sets activeWorkflow
- Updates unified selection state
- Sets active tab to 'graph'

**select-node** (lines 92-125):
- Sets selected node in graph
- Opens detail panel
- Selects input source if testId provided
- Pans to node after 100ms delay

**switch-and-select** (lines 127-194):
- Clears selected node first (exit LLM-only mode)
- Switches to new workflow
- Waits 400ms for workflow to load
- Then selects node and pans to it

**show-function-tests** (lines 196-216):
- Clears active workflow (LLM-only view)
- Sets detail panel to show tests
- Pans to function

**empty-state** (lines 218-233):
- Clears all selection
- Shows preview tab
- Closes detail panel

#### Timeout Management (Lines 59, 236-239)
```typescript
const timeouts: ReturnType<typeof setTimeout>[] = [];
// ... code that adds timeouts ...
return () => {
  timeouts.forEach(clearTimeout);
};
```

Tracks all setTimeout calls for cleanup on unmount or dependency change.

---

## 4. Type System & BAML File Parsing

### 4.1 Unified Type System
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/interface/types.ts`

**Lines: 1-200+**

#### Core Types (Lines 19-141)

**SpanInfo** (lines 20-28): File location information
```typescript
export interface SpanInfo {
  filePath: string;
  start: number;
  end: number;
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
}
```

**FunctionMetadata** (lines 88-103):
- name, type ('function' | 'llm_function' | 'workflow')
- span, signature, testSnippet
- testCases: TestCaseMetadata[]
- clientName (LLM-specific)
- orchestrationGraph

**FunctionWithCallGraph**: Extension of FunctionMetadata with call graph for complex workflows

**NodeType** (lines 109-115):
```typescript
export type NodeType =
  | 'function'
  | 'llm_function'
  | 'conditional'
  | 'loop'
  | 'return'
  | 'group';
```

**GraphNode** (lines 117-133): Node in workflow graph
- id, type, label, functionName
- position, parent (for subgraphs)
- codeHash, lastModified (cache invalidation)
- llmClient, metadata

### 4.2 BAML File Discovery
**Location**: SDK's `diagnostics.getBAMLFiles()` method

**Data Flow**:
1. DebugPanel calls `sdk.diagnostics.getBAMLFiles()` on mount (line 29)
2. Returns array of BAMLFile objects
3. Each BAMLFile contains:
   - path: relative to project
   - functions: array of parsed functions
   - tests: array of associated tests

---

## 5. Mock Data Structure

### 5.1 Workflow Data
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/mock-data/data-workflow.ts`

**Lines: 1-80+**

Example structure with decision nodes (diamond) and loop nodes (hexagon):
```typescript
export const workflowData: Graph = {
  nodes: [
    { id: 'A', label: 'Start', kind: 'item' },
    { id: 'B', label: 'Fetch user data', kind: 'item' },
    { id: 'C', label: 'Is user active?', kind: 'item', shape: 'diamond' },
    // ... more nodes
  ],
  edges: [
    { id: 'e_A_B', from: 'A', to: 'B', style: 'solid' },
    // ... more edges
  ]
};
```

### 5.2 Graph Types
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/mock-data/types.ts`

**Lines: 1-168**

#### Graph Interfaces
```typescript
export interface Graph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export type NodeShape = 'rect' | 'diamond' | 'hexagon' | 'stadium' | 'circle' | 'cylinder' | 'round';

export interface GraphNode {
  id: string;
  label: string;
  kind: 'item' | 'group';
  shape?: NodeShape;
  parent?: string;
}

export interface GraphEdge {
  id: string;
  from: string;
  to: string;
  style: 'solid' | 'dashed';
}
```

#### Reactflow Types (Lines 147-167)
```typescript
export type ReactflowNodeData = WorkflowNode & {
  sourceHandles: string[];
  targetHandles: string[];
  direction?: 'vertical' | 'horizontal';
  executionState?: 'not-started' | 'pending' | 'running' | 'success' | 'error' | 'skipped' | 'cached';
  isExecutionActive?: boolean;
  llmClient?: string;
  outputs?: Record<string, unknown>;
  error?: Error | string;
};
```

---

## 6. Graph Rendering & Visualization

### 6.1 Graph Components Location
**Path**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/graph-primitives/`

Key subdirectories:
- `nodes/`: Node components (BaseNode, GroupNode, DiamondNode, HexagonNode, LLMNode)
- `edges/`: Edge components and rendering logic
- `edges/EdgeController/`: Edge event handling
- `edges/BaseEdge/`: Base edge implementation

### 6.2 Node Types
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/graph-primitives/nodes/index.tsx`

Exports:
- `BaseNode`: Default rectangular node
- `GroupNode`: Container/subgraph node
- `DiamondNode`: Conditional/decision node
- `HexagonNode`: Loop iteration node
- `LLMNode`: LLM function node with special styling

### 6.3 Execution History Visualization
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/graph-primitives/nodes/subcomponents/ExecutionHistoryDots.tsx`

Shows execution state history as dots under each node during/after execution.

---

## 7. Event Handling & Updates

### 7.1 EventListener Component
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/baml_wasm_web/EventListener.tsx`

**Lines: 1-100+**

#### Purpose
Routes IDE and LSP messages to SDK methods

#### Message Flow (Lines 91-115)
```typescript
const handler = async (event: MessageEvent<VscodeToWebviewCommand>) => {
  const { source, payload } = event.data;
  console.debug('[EventListener] Handling command:', { source, payload });

  try {
    switch (source) {
      case 'ide_message':
        await handleIDEMessage(sdk, payload, setBamlCliVersion, setBamlConfig);
        break;
      case 'lsp_message':
        await handleLSPMessage(sdk, payload);
        break;
```

#### Message Types
- `ide_message`: IDE commands (settings, diagnostics, etc.)
- `lsp_message`: LSP protocol messages (e.g., `runtime_updated`)

**Critical for DebugPanel**: 
- DebugPanel adds files via `window.postMessage` with `runtime_updated` LSP message (line 166)
- EventListener catches these messages and calls `sdk.files.update()`

### 7.2 Message Handler Files
**Location**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/baml_wasm_web/message-handlers.ts`

Implements `handleIDEMessage()` and `handleLSPMessage()` functions that update SDK state

---

## 8. SDK Architecture

### 8.1 SDK Class
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/index.ts`

**Lines: 1-200+**

#### Public API Methods

**Workflow Management**:
- `sdk.workflows.getAll()`
- `sdk.workflows.getById(id)`
- `sdk.workflows.getByFunction(functionName)`

**File Management**:
- `sdk.files.update(files)`
- `sdk.files.getCurrent()`

**Diagnostics**:
- `sdk.diagnostics.getBAMLFiles()`
- `sdk.diagnostics.getErrors()`

**Test Execution**:
- `sdk.test.run(functionName, testName, inputs)`

**Atoms**:
- `sdk.atoms.*` - Direct access to all Jotai atoms

#### Immutable Runtime Pattern
```typescript
async recreateRuntime() {
  const runtime = await this.runtimeFactory.create(this.currentFiles);
  this.storage.setRuntimeInstance(runtime);
}
```

Runtime is recreated (not mutated) when files change.

### 8.2 SDK Provider
**File**: `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/sdk/provider.tsx`

**Lines: 1-153**

#### Two Runtime Modes
- **mock**: `createMockSDK()` - For development/testing
- **wasm**: `createRealBAMLSDK()` - Real BAML compiler

#### Initialization Flow (Lines 85-116)
1. Create Jotai store and SDK instance
2. Initialize SDK with initial files
3. Set first workflow as active
4. Return to context

---

## 9. State Update Flow Diagram

```
DebugPanel Click
    ↓
handleFunctionClick() or handleTestClick()
    ↓
setActiveCodeClick(CodeClickEvent)
    ├→ Updates activeCodeClickAtom
    └→ useCodeNavigation() hook triggers
    ↓
determineNavigationAction()
    ↓
Execute NavigationAction
    ├→ setActiveWorkflow() + setUnifiedSelection()
    ├→ setSelectedNodeId()
    ├→ openDetailPanel()
    └→ selectSource() (for tests)
    ↓
Update UI
    ├→ Graph switches to new workflow
    ├→ Node gets highlighted
    ├→ Detail panel opens/updates
    └→ Camera pans to node
```

---

## 10. Missing Navigation Logic & Gaps

### 10.1 Conditional.baml File Handling

**Current Status**: The DebugPanel loads all BAML files via `sdk.diagnostics.getBAMLFiles()`, which should include conditional.baml files.

**Missing Implementation**:
1. **File Type Detection**: No explicit handling for conditional.baml vs. other .baml files
2. **Conditional Node Clicking**: Clicking on conditional nodes doesn't trigger special navigation
3. **Conditional Content Rendering**: No special UI for showing conditional block structure or branch conditions

**Required**:
- Parse conditional block structure from BAML AST
- Add conditional-specific node types to navigation heuristic
- Show branch conditions in detail panel
- Highlight active branch during execution

### 10.2 File Click Handling

**Current Status**: DebugPanel shows functions and tests, not files themselves.

**Missing**:
- Clicking on a file header doesn't navigate to that file
- No breadcrumb navigation showing current file
- File-level operations (search, refactor, etc.) not connected

### 10.3 Graph View Updates on File Changes

**Current Status**: When files are modified via "Add File" or "Edit Function", EventListener posts `runtime_updated` which triggers SDK recreation.

**What Updates**:
- Atoms trigger re-renders ✓
- Graph nodes update ✓

**What Might Be Missing**:
- If user is viewing an execution, graph doesn't refresh to show new structure ✓ (works because atoms trigger re-render)
- Edge cases with stale cached state? (should be fine due to immutable pattern)

### 10.4 View Mode Switching Triggers

**Current Status**: View modes ('editor' vs 'execution') are managed but switching logic is basic.

**Missing Context**:
- When should view auto-switch to execution?
- When should it stay in editor despite execution?

---

## 11. File Paths & Key Locations Summary

| Component | Path | Lines | Key Functions |
|-----------|------|-------|----------------|
| Core Atoms | `/sdk/atoms/core.atoms.ts` | 638 | 30+ atom definitions |
| Test Atoms | `/sdk/atoms/test.atoms.ts` | 150 | Test execution state |
| DebugPanel | `/features/debug-panel/components/DebugPanel.tsx` | 349 | Click handlers, file tree UI |
| Navigation Hook | `/features/navigation/hooks/useCodeNavigation.ts` | 252 | Action execution |
| Navigation Heuristic | `/sdk/navigationHeuristic.ts` | 250+ | Decision tree algorithm |
| SDK Types | `/sdk/types.ts` | 350+ | CodeClickEvent, BAMLFile, etc. |
| Interface Types | `/sdk/interface/types.ts` | 200+ | FunctionMetadata, GraphNode, etc. |
| Mock Data | `/mock-data/data-workflow.ts` | 80+ | Example workflow graph |
| EventListener | `/baml_wasm_web/EventListener.tsx` | 150+ | Message routing |
| SDK Class | `/sdk/index.ts` | 600+ | BAML SDK API |
| SDK Provider | `/sdk/provider.tsx` | 153 | React context setup |

---

## 12. Key Insights

1. **Centralized State**: All UI state flows through Jotai atoms, making state predictable and testable

2. **Navigation is Heuristic-Driven**: The `determineNavigationAction()` function makes smart decisions about what view to show based on context

3. **DebugPanel is Low-Level**: It directly manipulates atoms and bypasses normal IDE integration - good for testing, not suitable for production UI

4. **Immutable Runtime Pattern**: Runtimes are never mutated; new instances are created on file changes, ensuring consistency

5. **Event-Driven Updates**: File changes trigger LSP messages → EventListener → SDK.files.update() → Runtime recreation → Atom updates → UI re-renders

6. **Performance Optimizations**:
   - atomFamily for per-node state (O(1) updates)
   - Derived atoms for computed state
   - AllFunctionsMapAtom for O(1) function lookup
   - Debounced file updates

7. **Missing piece for conditional.baml**: The system loads conditionals as functions, but doesn't have special UI or navigation for conditional branches within those functions

---

## 13. Development Recommendations

### For Conditional.baml Navigation:
1. Extend `NodeType` to include 'conditional' as a distinct type
2. Update DebugPanel to show conditional branches differently (indented or nested)
3. Update navigation heuristic to handle conditional node selection
4. Add conditional-specific icons in graph nodes
5. Show branch conditions in detail panel

### For View Updates:
1. Add explicit view-switching triggers in `useCodeNavigation` based on workflow context
2. Create derived atom for "recommended view mode" based on execution state
3. Add smooth transitions between view modes

### For File Navigation:
1. Allow clicking file headers in DebugPanel
2. Show file breadcrumb in detail panel
3. Create file-level operations in UI

---

**Report Generated**: November 9, 2025
**Package**: playground-common
**Branch**: aaron/graphs
