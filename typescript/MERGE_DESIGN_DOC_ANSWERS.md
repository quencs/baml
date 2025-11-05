# BAML Graphs Integration: Design Decisions & Answers

**Date:** 2025-11-04
**Status:** Approved Decisions
**References:** MERGE_DESIGN_DOC.md, BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md

---

## Question 1: WASM Runtime Workflow Support

### Answer
✅ **WASM runtime will support emitting workflows**

The BAML runtime will add capability to get all 'graphs'/workflows. We can assume this API will be available.

### Implementation Notes

**Expected WASM API:**
```typescript
interface WasmRuntime {
  list_workflows(): Workflow[]
  get_workflow(id: string): Workflow | null
  // Existing methods
  list_functions(): Function[]
  get_function(name: string): Function | null
}
```

**VSCodeDataProvider can directly query:**
```typescript
async getWorkflows(): Promise<Workflow[]> {
  const runtime = this.store.get(runtimeAtom)?.runtime
  if (!runtime) return []
  return runtime.list_workflows()
}
```

**Timeline Impact:** None - proceed with implementation assuming this API

**Recommendation:** Work with WASM team to define the `Workflow` type structure:
- What fields does it have? (id, name, nodes, edges, metadata?)
- How are nodes represented? (function references? inline definitions?)
- Are edges typed? (data flow, conditional, loop?)

---

## Question 2: Graph Layout Persistence

### Answer
✅ **Store in memory using Jotai localStorage**

No server-side persistence needed. Use Jotai's `atomWithStorage` for browser localStorage.

### Implementation Notes

**Atom Definition:**
```typescript
// In unified atoms file
export const workflowLayoutsAtom = atomWithStorage<Record<string, NodePositions>>(
  'baml:workflow:layouts',
  {}
)

type NodePositions = Record<string, { x: number; y: number }>
```

**Save on drag end:**
```typescript
// In WorkflowGraph component
const onNodeDragStop = useCallback((event, node) => {
  const layouts = store.get(workflowLayoutsAtom)
  store.set(workflowLayoutsAtom, {
    ...layouts,
    [workflowId]: {
      ...layouts[workflowId],
      [node.id]: { x: node.position.x, y: node.position.y }
    }
  })
}, [workflowId])
```

**Load on mount:**
```typescript
const savedLayout = useAtomValue(workflowLayoutsAtom)[workflowId]
// Apply saved positions to nodes
```

**Benefits:**
- ✅ Simple implementation
- ✅ Persists across browser reloads
- ✅ Per-user (localStorage is browser-specific)
- ✅ No server/file I/O needed

**Limitations:**
- ❌ Not shared across machines
- ❌ Cleared if user clears browser data
- ❌ Limited to ~5-10MB total localStorage

**Recommendation:** This is perfect for the use case. If future requirement emerges for shared layouts, can add export/import JSON feature.

---

## Question 3: Feature Rollout Strategy

### Answer
✅ **Graph view as default, prompt preview for standalone LLM functions**

- Large UI change is acceptable
- LLM-only functions (not in any workflow) → render existing prompt preview + test panel
- Functions in workflows → render graph view
- Can gate behind feature flag if needed
- UI changes are acceptable

### Implementation Notes

**View Selection Logic:**
```typescript
export function UnifiedPlaygroundView() {
  const selectedFunction = useAtomValue(selectedFunctionAtom)
  const workflows = useAtomValue(workflowsAtom)
  const isLLMOnly = useAtomValue(isLLMOnlyModeAtom)

  // Determine if function is in any workflow
  const functionInWorkflow = workflows.some(w =>
    w.nodes.some(n => n.functionName === selectedFunction)
  )

  if (selectedFunction && !functionInWorkflow && isLLMOnly) {
    // Standalone LLM function → existing prompt preview
    return <PromptPreview />
  }

  if (functionInWorkflow) {
    // Function is part of workflow → graph view
    return <WorkflowGraphView />
  }

  return <EmptyState />
}
```

**Optional Feature Flag:**
```typescript
// Can add if rollout needs to be gradual
export const graphViewEnabledAtom = atomWithStorage(
  'baml:featureFlags:graphView',
  true // Default enabled
)

// In UnifiedPlaygroundView
const graphViewEnabled = useAtomValue(graphViewEnabledAtom)
if (functionInWorkflow && graphViewEnabled) {
  return <WorkflowGraphView />
}
```

**Recommendation:** Start without feature flag. If issues arise during rollout, can add flag in a hotfix. The logic is clean enough that both views can coexist.

---

## Question 4: Prompt Preview Integration

### Answer
✅ **Prompt preview shown for standalone LLM functions; graph view has toggle/button for prompt**

- Standalone LLM function (not in workflow) → show existing prompt preview UI
- Function in workflow → show graph, with button/toggle to view prompt

### Implementation Notes

**Approach A: Detail Panel Shows Prompt (Recommended)**

When node selected in graph view, detail panel shows:
- **Input/Output** tab
- **Prompt** tab (for LLM nodes)
- **Logs** tab
- **LLM Request/Response** tab (for LLM nodes)

```typescript
// DetailPanel.tsx
export function DetailPanel() {
  const selectedNode = useAtomValue(selectedNodeIdAtom)
  const node = useAtomValue(nodeDataAtom(selectedNode))

  const tabs = useMemo(() => {
    const baseTabs = [
      { id: 'inputs', label: 'Inputs' },
      { id: 'outputs', label: 'Outputs' },
      { id: 'logs', label: 'Logs' }
    ]

    if (node?.type === 'llm_function') {
      baseTabs.push(
        { id: 'prompt', label: 'Prompt' },
        { id: 'llm-request', label: 'LLM Request' },
        { id: 'llm-response', label: 'LLM Response' }
      )
    }

    return baseTabs
  }, [node?.type])

  return (
    <div className="detail-panel">
      <Tabs tabs={tabs} />
      {activeTab === 'prompt' && <PromptView node={node} />}
      {/* ... other tabs */}
    </div>
  )
}
```

**Approach B: Toggle Button in Toolbar**

```typescript
// WorkflowToolbar.tsx
export function WorkflowToolbar() {
  const [showPrompt, setShowPrompt] = useState(false)
  const selectedNode = useAtomValue(selectedNodeIdAtom)
  const isLLMNode = /* check if selected node is LLM */

  return (
    <div className="toolbar">
      {/* ... other buttons */}
      {isLLMNode && (
        <Button onClick={() => setShowPrompt(!showPrompt)}>
          {showPrompt ? 'Show Graph' : 'Show Prompt'}
        </Button>
      )}
    </div>
  )
}

// In WorkflowGraphView
{showPrompt ? <PromptPreview /> : <ReactFlowGraph />}
```

**Recommendation:** Use **Approach A** (Detail Panel). Reasons:
1. Detail panel already exists in baml-graph
2. Natural place for node-specific info
3. No need to toggle entire view
4. Can see graph + prompt simultaneously
5. Matches existing playground-common patterns (they already show detailed info in panels)

**Migration Path:**
1. Keep existing `PromptPreview` component for standalone functions
2. Extract reusable `PromptView` component from it
3. Use `PromptView` in both standalone mode and detail panel "Prompt" tab
4. Add "Prompt" tab to detail panel when LLM node selected

---

## Question 5: Execution History Persistence

### Answer
✅ **No persistence needed - in-memory only**

### Implementation Notes

**Execution History Atom:**
```typescript
// No atomWithStorage, just regular atom
export const executionHistoryAtom = atom<ExecutionSnapshot[]>([])

// Optionally: limit to last N executions to prevent memory bloat
export const addExecutionAtom = atom(
  null,
  (get, set, execution: ExecutionSnapshot) => {
    const history = get(executionHistoryAtom)
    set(executionHistoryAtom, [...history, execution].slice(-50)) // Keep last 50
  }
)
```

**Benefits:**
- ✅ Simple implementation
- ✅ No storage concerns
- ✅ Fast (no serialization)
- ✅ Automatically cleared on refresh

**Recommendation:** This is perfect. Execution history is for debugging current session, not long-term storage. If users want to save results, they can export/copy from the UI.

---

## Question 6: Mock Data Customization

### Answer
✅ **Hardcoded mock data in TypeScript files is fine**

### Implementation Notes

Keep existing approach from `apps/baml-graph/src/sdk/mock.ts`:
```typescript
// Mock workflows
const MOCK_WORKFLOWS = [
  {
    id: 'simple-workflow',
    name: 'Simple Workflow',
    nodes: [...],
    edges: [...]
  },
  // ... more workflows
]

// Mock test cases
const MOCK_TEST_CASES = {
  'fetchData': [
    { name: 'success_case', inputs: {...}, expectedOutput: {...} },
    { name: 'error_case', inputs: {...}, expectedOutput: {...} }
  ]
}

// Mock BAML files
const MOCK_BAML_FILES = {
  'workflows/simple.baml': `
    function ExtractResume(resume: string) -> Resume {
      client GPT4
      prompt #"Extract: {{ resume }}"#
    }
  `,
  // ... more files
}
```

**Recommendation:** Keep it simple. Hardcoded mocks are:
- Easy to maintain
- Good for demos and screenshots
- Fast to load
- No parsing complexity

If custom mocks become important later, can add JSON import feature.

---

## Question 7: Unifying `runTests` vs `run workflow` ⭐

### Answer
✅ **Unify execution model: both are essentially "run function with inputs"**

Key insight from user: "Run workflow is basically identical as runTests, it's just that it runs a non-llm function that has more stuff happening."

### Deep Analysis & Recommendations

#### Current State

**playground-common: `runTests`** (`test-runner.ts:538-629`)
```typescript
async function runTest(
  functionName: string,
  testName: string,
  inputs: Record<string, any>
) {
  // 1. Get runtime
  const runtime = store.get(runtimeAtom)

  // 2. Execute function
  const result = await runtime.execute_function(functionName, inputs)

  // 3. Track state
  store.set(runningTestsAtom, [...running, { functionName, testName, state: 'running' }])

  // 4. Handle result
  store.set(testCaseResponseAtom([functionName, testName]), result)

  // 5. Update UI
  store.set(flashRangesAtom, [...]) // Highlight code
  store.set(testHistoryAtom, [...]) // Add to history
}
```

**baml-graph: `executeWorkflow`** (`sdk/index.ts:120-179`)
```typescript
async function executeWorkflow(
  workflowId: string,
  options: { startFromNodeId?: string }
) {
  // 1. Clear previous state
  store.set(clearAllNodeStatesAtom)

  // 2. Create execution snapshot
  const execution = { id, workflowId, status: 'running', ... }
  store.set(workflowExecutionsAtomFamily(workflowId), [...execs, execution])

  // 3. Execute nodes in order
  for (const node of traverseGraph(workflow, startFromNodeId)) {
    // Set node state to running
    store.set(nodeStateAtomFamily(node.id), 'running')

    // Execute node
    const result = await runtime.execute_function(node.functionName, inputs)

    // Set node state to completed
    store.set(nodeStateAtomFamily(node.id), 'success')
    store.set(nodeExecutionsAtom, new Map(..., [node.id, { result }]))
  }

  // 4. Mark execution complete
  execution.status = 'completed'
}
```

#### Unified Execution Model

Both are fundamentally: **"Execute BAML function(s) with inputs and track results"**

**Unified SDK Method:**
```typescript
class BAMLSDK {
  execute(options: ExecutionOptions): Promise<ExecutionResult> {
    const {
      target,      // What to execute
      inputs,      // Input values
      context,     // Additional context
      mode         // Execution mode
    } = options

    if (target.type === 'function' && target.scope === 'isolated') {
      return this.executeFunctionIsolated(target.functionName, inputs, context)
    }

    if (target.type === 'function' && target.scope === 'workflow') {
      return this.executeFunctionInWorkflow(target.functionName, inputs, context)
    }

    if (target.type === 'workflow') {
      return this.executeWorkflow(target.workflowId, inputs, context)
    }
  }
}

type ExecutionOptions = {
  // Target
  target:
    | { type: 'function', scope: 'isolated', functionName: string }
    | { type: 'function', scope: 'workflow', functionName: string, workflowId: string }
    | { type: 'workflow', workflowId: string, startFromNode?: string }

  // Inputs
  inputs?: Record<string, any>
  testCaseId?: string  // If using test case inputs

  // Context
  context?: {
    rerunWholeWorkflow?: boolean  // If function in workflow, rerun whole thing?
    clearCache?: boolean
    abortSignal?: AbortSignal
  }

  // Tracking
  trackAs?: {
    type: 'test' | 'execution'
    id?: string
  }
}

type ExecutionResult = {
  id: string
  status: 'success' | 'error' | 'cancelled'

  // Function execution
  result?: any
  error?: Error
  duration?: number

  // Workflow execution
  nodeResults?: Map<string, NodeExecutionResult>

  // Tracking
  watchNotifications?: WatchNotification[]
  logs?: LogEntry[]
}
```

#### Three Execution Modes

**1. Isolated Function Execution** (current `runTests`)
```typescript
sdk.execute({
  target: { type: 'function', scope: 'isolated', functionName: 'ExtractResume' },
  inputs: { resume: '...' },
  testCaseId: 'test_valid_resume',
  trackAs: { type: 'test' }
})

// Internally:
// - Executes just this function
// - No workflow context
// - Updates testCaseResponseAtom
// - Adds to test history
// - Shows in prompt preview panel
```

**2. Function in Workflow Context** (new capability)
```typescript
sdk.execute({
  target: {
    type: 'function',
    scope: 'workflow',
    functionName: 'ProcessData',
    workflowId: 'simple-workflow'
  },
  inputs: { data: '...' },
  context: { rerunWholeWorkflow: false }, // Just this node
  trackAs: { type: 'execution' }
})

// Internally:
// - Executes just this function
// - BUT with workflow context (upstream node outputs available)
// - Updates nodeStateAtomFamily(nodeId)
// - Updates nodeExecutionsAtom
// - Highlights node in graph
```

**3. Full Workflow Execution** (new)
```typescript
sdk.execute({
  target: { type: 'workflow', workflowId: 'simple-workflow' },
  inputs: { /* initial inputs */ },
  context: { clearCache: true },
  trackAs: { type: 'execution' }
})

// Internally:
// - Traverses graph from start
// - Executes nodes in topological order
// - Updates nodeStateAtomFamily for each
// - Shows progress in graph view
// - Handles errors, retries, caching
```

#### Implementation Strategy

**Phase 1: Extract Common Execution Logic**
```typescript
// sdk/execution/base.ts
export class ExecutionEngine {
  constructor(
    private runtime: WasmRuntime,
    private store: Store
  ) {}

  async executeFunction(
    functionName: string,
    inputs: Record<string, any>,
    options: ExecutionOptions
  ): Promise<FunctionResult> {
    // Common logic:
    // - Validate inputs
    // - Call WASM runtime
    // - Collect watch notifications
    // - Handle errors
    // - Track duration
    return result
  }

  async executeWorkflow(
    workflowId: string,
    inputs: Record<string, any>,
    options: ExecutionOptions
  ): Promise<WorkflowResult> {
    const workflow = this.store.get(workflowsAtom).find(w => w.id === workflowId)
    const nodes = topologicalSort(workflow.nodes, workflow.edges)

    const nodeResults = new Map<string, NodeResult>()

    for (const node of nodes) {
      // Use executeFunction for each node
      const result = await this.executeFunction(
        node.functionName,
        this.resolveNodeInputs(node, nodeResults),
        options
      )

      nodeResults.set(node.id, result)

      // Update node state
      this.store.set(nodeStateAtomFamily(node.id),
        result.error ? 'error' : 'success'
      )

      if (result.error && !options.continueOnError) {
        break // Stop workflow on error
      }
    }

    return { nodeResults, status: 'completed' }
  }
}
```

**Phase 2: Unified SDK API**
```typescript
// sdk/index.ts
class BAMLSDK {
  private executionEngine: ExecutionEngine

  execute(options: ExecutionOptions): Promise<ExecutionResult> {
    // Dispatch to appropriate method
    switch (options.target.type) {
      case 'function':
        return this.executeFunction(options)
      case 'workflow':
        return this.executeWorkflow(options)
    }
  }

  private async executeFunction(options) {
    const { target, inputs, testCaseId, trackAs } = options

    // Get inputs (from test case or provided)
    const resolvedInputs = testCaseId
      ? this.getTestCaseInputs(target.functionName, testCaseId)
      : inputs

    // Execute
    const result = await this.executionEngine.executeFunction(
      target.functionName,
      resolvedInputs,
      options.context
    )

    // Track appropriately
    if (trackAs?.type === 'test') {
      this.trackAsTest(target.functionName, testCaseId, result)
    } else {
      this.trackAsExecution(target.functionName, result)
    }

    return result
  }

  private async executeWorkflow(options) {
    const { target, inputs } = options

    // Execute workflow
    const result = await this.executionEngine.executeWorkflow(
      target.workflowId,
      inputs,
      options.context
    )

    // Track
    this.trackWorkflowExecution(target.workflowId, result)

    return result
  }

  // Backward compatibility helpers
  async runTest(functionName: string, testName: string) {
    return this.execute({
      target: { type: 'function', scope: 'isolated', functionName },
      testCaseId: testName,
      trackAs: { type: 'test' }
    })
  }

  async runWorkflow(workflowId: string, inputs?: Record<string, any>) {
    return this.execute({
      target: { type: 'workflow', workflowId },
      inputs,
      trackAs: { type: 'execution' }
    })
  }
}
```

**Phase 3: UI Integration**

**Run Button Logic:**
```typescript
// When user clicks "Run" button
function RunButton() {
  const sdk = useBAMLSDK()
  const selectedFunction = useAtomValue(selectedFunctionAtom)
  const activeWorkflow = useAtomValue(activeWorkflowAtom)
  const selectedTest = useAtomValue(selectedTestcaseAtom)

  const handleRun = async () => {
    // Case 1: Test selected → run test
    if (selectedTest) {
      await sdk.execute({
        target: { type: 'function', scope: 'isolated', functionName: selectedFunction },
        testCaseId: selectedTest,
        trackAs: { type: 'test' }
      })
    }
    // Case 2: Function in workflow → run workflow or just function?
    else if (activeWorkflow) {
      const rerunWorkflow = await askUser("Run entire workflow or just this function?")

      if (rerunWorkflow) {
        await sdk.execute({
          target: { type: 'workflow', workflowId: activeWorkflow.id }
        })
      } else {
        await sdk.execute({
          target: {
            type: 'function',
            scope: 'workflow',
            functionName: selectedFunction,
            workflowId: activeWorkflow.id
          }
        })
      }
    }
    // Case 3: Standalone function → run isolated
    else {
      await sdk.execute({
        target: { type: 'function', scope: 'isolated', functionName: selectedFunction }
      })
    }
  }

  return <Button onClick={handleRun}>Run</Button>
}
```

#### Key Design Decisions

**1. Single Execution Method with Options**
✅ Better than separate `runTest()`, `runWorkflow()`, `runFunction()` methods
- Easier to understand
- Flexible for future extensions
- Clear semantics via options

**2. Backward Compatibility Helpers**
✅ Keep `runTest()` and `runWorkflow()` as convenience wrappers
- Existing code doesn't break
- Clear migration path

**3. Context-Aware Execution**
✅ Function execution can be aware of workflow context
- Can access upstream node outputs
- Can use workflow-level cache
- Still isolatable for testing

**4. Unified Result Type**
✅ Both return `ExecutionResult` with different fields populated
- Test execution: `result`, `duration`, `watchNotifications`
- Workflow execution: `nodeResults`, `status`

#### Benefits of Unification

1. **Single Code Path** - Less duplication, easier to maintain
2. **Consistent Behavior** - Same error handling, caching, abort logic
3. **Flexible Execution** - Can run function isolated OR in workflow context
4. **Clear Semantics** - `target` and `context` options are self-documenting
5. **Future-Proof** - Easy to add new execution modes (e.g., partial workflow, multi-workflow)

---

## Question 8: Bundle Size

### Answer
✅ **Bundle size is not a problem**

ReactFlow (~500KB) + ELK (~300KB) = ~800KB increase is acceptable.

### Implementation Notes

No optimization needed, but good practices:
- Use tree-shaking (Vite does this by default)
- Lazy load heavy graph components if needed later
- Monitor bundle size in CI (optional)

**Optional monitoring:**
```json
// package.json
{
  "scripts": {
    "build:analyze": "vite build --mode analyze"
  }
}
```

**Recommendation:** Ship it. Modern VSCode extensions can be several MB. 800KB for a rich graph visualization is very reasonable.

---

## Question 9: Package Structure

### Answer
✅ **Keep `apps/playground`, add graph support**

Consider creating separate package: `packages/baml-graph-components` for reusable graph utilities.

### Implementation Notes

**Option A: Everything in playground-common** (Simpler)
```
packages/playground-common/src/
├── sdk/                    # SDK from baml-graph
├── features/
│   ├── debug/
│   ├── graph/             # Graph components
│   ├── prompt-preview/
│   └── navigation/
└── shared/
    ├── atoms/
    └── components/
```

**Option B: Separate graph package** (More modular)
```
packages/baml-graph-components/
├── src/
│   ├── components/
│   │   ├── WorkflowGraph.tsx
│   │   ├── nodes/
│   │   └── edges/
│   ├── hooks/
│   │   ├── useGraphLayout.ts
│   │   └── useGraphSync.ts
│   ├── layout/
│   │   └── elk-layout.ts
│   └── types.ts
├── package.json           # Dependencies: @xyflow/react, elkjs
└── README.md

packages/playground-common/
├── dependencies: ["@baml/baml-graph-components"]
└── src/
    ├── sdk/
    ├── features/
    │   └── workflow-view/  # Uses baml-graph-components
    └── ...
```

### Recommendation: **Option B** (Separate Package)

**Benefits:**
1. **Reusability** - Other apps can use graph components
2. **Clear Dependencies** - ReactFlow/ELK isolated to graph package
3. **Separate Testing** - Test graph logic independently
4. **Independent Versioning** - Can update graph without touching playground
5. **Smaller Bundle for Non-Graph Users** - If someone only wants playground-common without graphs

**Package Structure:**
```typescript
// packages/baml-graph-components/package.json
{
  "name": "@baml/baml-graph-components",
  "version": "1.0.0",
  "exports": {
    ".": "./src/index.ts",
    "./components": "./src/components/index.ts",
    "./hooks": "./src/hooks/index.ts",
    "./layout": "./src/layout/index.ts"
  },
  "dependencies": {
    "@xyflow/react": "^12.0.0",
    "elkjs": "^0.9.0",
    "jotai": "^2.0.0"
  },
  "peerDependencies": {
    "react": "^18.0.0"
  }
}
```

**Import in playground-common:**
```typescript
import { WorkflowGraph, useGraphLayout } from '@baml/baml-graph-components'
```

---

## Question 10: Cursor Auto-Navigation

### Answer
✅ **Implement automatic navigation with debouncing**

When cursor moves into a function/test in the editor, automatically update UI (switch workflows, select nodes, etc.)

### Implementation Notes

**From CURSOR_TO_CODECLICK_UNIFICATION.md:**

```typescript
// Debounced cursor events
export const debouncedCodeClickAtom = atomWithDebounce(
  codeClickEventAtom,
  300  // 300ms delay
)

// Navigation handler uses debounced atom
export function useNavigationHandler() {
  const codeClick = useAtomValue(debouncedCodeClickAtom)  // Not the raw one
  const sdk = useBAMLSDK()

  useEffect(() => {
    if (!codeClick) return

    const action = determineNavigationAction(codeClick, sdk)

    // Execute navigation (switch workflow, select node, etc.)
    executeNavigationAction(action, sdk)
  }, [codeClick, sdk])
}
```

**Benefits:**
1. **Feels Alive** - UI responds to code navigation
2. **Context-Aware** - Automatically shows relevant workflow
3. **Smooth UX** - Debouncing prevents jank when scrolling through code
4. **Smart Heuristics** - Stays in current workflow when possible (from baml-graph)

**Potential Issue:** Might be jarring for users who don't expect it

**Solution:** Add preference toggle (but default ON):
```typescript
export const autoNavigationEnabledAtom = atomWithStorage(
  'baml:preferences:autoNavigation',
  true  // Default enabled
)
```

**Recommendation:** Ship with auto-navigation ON. Gather feedback. If users complain, add toggle in settings.

---

## Summary: Architecture Implications

Based on these answers, here's the final architecture:

### Package Structure
```
packages/
├── baml-graph-components/       # NEW: Reusable graph components
│   ├── components/ (WorkflowGraph, nodes, edges)
│   ├── hooks/ (useGraphLayout, useGraphSync)
│   └── layout/ (ELK integration)
├── playground-common/           # ENHANCED: Add SDK + workflow support
│   ├── sdk/
│   │   ├── index.ts (BAMLSDK with unified execution)
│   │   ├── execution/ (ExecutionEngine)
│   │   ├── providers/ (MockDataProvider, VSCodeDataProvider)
│   │   └── atoms/ (unified atoms)
│   ├── features/
│   │   ├── workflow-view/ (uses baml-graph-components)
│   │   ├── prompt-preview/ (existing, for standalone functions)
│   │   ├── debug/ (from baml-graph)
│   │   └── navigation/ (from baml-graph)
│   └── shared/
│       ├── atoms/ (merged + unified)
│       └── components/
└── ui/                          # EXISTING: ShadCN components

apps/
├── playground/                  # ENHANCED: Add graph support
│   ├── Uses playground-common with graph features
│   └── Dev mode toggle for mock data
└── vscode-ext/                  # UNCHANGED: Works via playground-common
    └── Webview uses playground-common
```

### Key Integration Points

1. **WASM → SDK**: Runtime provides workflows via `list_workflows()`
2. **EventListener → SDK**: Translates IDE messages to SDK calls
3. **SDK → Atoms**: SDK updates unified Jotai atoms
4. **Components → Atoms**: UI subscribes to atoms for reactivity
5. **Graph Components**: Imported from separate `@baml/baml-graph-components` package

### Execution Flow

```
User clicks "Run" button
    ↓
UI determines context (test? workflow? function?)
    ↓
sdk.execute({ target, inputs, context })
    ↓
ExecutionEngine
    ↓
├─ Function execution → WASM runtime.execute_function()
│  └─ Update testCaseResponseAtom OR nodeExecutionsAtom
│
└─ Workflow execution → Loop through nodes
   └─ Update nodeStateAtomFamily for each node
    ↓
UI re-renders (atoms changed)
```

### View Selection Logic

```
User navigates to function (cursor or click)
    ↓
updateCursorAtom enriches to CodeClickEvent
    ↓
useNavigationHandler (debounced 300ms)
    ↓
Navigation heuristic determines action
    ↓
├─ Function in workflow? → Show WorkflowGraphView
│  └─ Detail panel shows node info + prompt tab
│
└─ Standalone LLM function? → Show PromptPreview
   └─ Existing UI with test panel
```

---

## Next Steps

1. ✅ Create `packages/baml-graph-components` package
2. ✅ Copy SDK to `packages/playground-common/src/sdk/`
3. ✅ Implement unified execution engine
4. ✅ Merge atoms into unified structure
5. ✅ Build `UnifiedPlaygroundView` component
6. ✅ Add cursor-to-CodeClick enrichment
7. ✅ Test in VSCode extension
8. ✅ Add debug panel with dev mode toggle
9. ✅ Update documentation

**Timeline:** 8-10 weeks with 1-2 developers (per BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md)

---

## References

- **MERGE_DESIGN_DOC.md**: Original comprehensive design document
- **BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md**: Detailed integration strategy with phases
- **CURSOR_TO_CODECLICK_UNIFICATION.md**: Deep dive on cursor enrichment
