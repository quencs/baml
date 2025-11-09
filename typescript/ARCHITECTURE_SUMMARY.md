# Playground-Common Architecture Summary

## Quick Reference: State Flow Diagram

```
IDE/DebugPanel Action
    │
    ├─ Function Click ──→ CodeClickEvent { type: 'function', functionName, ... }
    └─ Test Click ──────→ CodeClickEvent { type: 'test', testName, functionName, ... }
            │
            ↓
    activeCodeClickAtom = event
    updateSelectionAtom({ functionName, testCaseName })
            │
            ├─ selectedFunctionNameAtom = functionName
            └─ selectedTestCaseNameAtom = testCaseName
            │
            ↓
    useCodeNavigation() effect triggers
            │
            ↓
    determineNavigationAction(event, navState)
            │
            ├─→ switch-workflow { workflowId }
            ├─→ select-node { nodeId, testId? }
            ├─→ switch-and-select { workflowId, nodeId, testId? }
            ├─→ show-function-tests { functionName, tests }
            └─→ empty-state { reason, functionName }
            │
            ↓
    Execute Action
            │
            ├─→ setActiveWorkflow(id)
            ├─→ setSelectedNodeId(id)
            ├─→ openDetailPanel()
            └─→ panToNode()
            │
            ↓
    Atoms Update
            │
            ├─→ activeWorkflowAtom
            ├─→ selectedNodeIdAtom
            └─→ detailPanelAtom
            │
            ↓
    Components Re-Render
            │
            ├─→ Graph switches workflow
            ├─→ Node highlighted
            ├─→ Detail panel opens
            └─→ Camera pans to node
```

## Core Atoms Hierarchy

```
runtimeInstanceAtom (source of truth)
    │
    ├─→ workflowsAtom (derived)
    │   └─→ activeWorkflowAtom (derived)
    │       └─→ activeWorkflowExecutionsAtom (derived)
    │
    ├─→ functionsAtom (derived)
    │   └─→ allFunctionsMapAtom (derived, O(1) lookup)
    │
    ├─→ diagnosticsAtom (derived)
    │   ├─→ isRuntimeValid (derived)
    │   └─→ numErrorsAtom (derived)
    │
    └─→ generatedFilesAtom (derived)

selectedFunctionNameAtom
    │
    └─→ selectedFunctionObjectAtom (derived)
        │
        └─→ selectionAtom (derived) { selectedFn, selectedTc }

selectedTestCaseNameAtom
    │
    └─→ selectedTestCaseAtom (derived)
        │
        └─→ selectionAtom (derived) { selectedFn, selectedTc }

activeCodeClickAtom
    │
    └─→ useCodeNavigation() (effect)
        │
        └─→ Executes NavigationAction
```

## DebugPanel Component

```
DebugPanel
├─ Mount
│  └─ sdk.diagnostics.getBAMLFiles() → bamlFilesAtom
│
├─ File Tree UI
│  ├─ Collapsible files
│  ├─ Functions (excluding workflows)
│  └─ Tests
│
├─ Click Handlers
│  ├─ handleFunctionClick(func)
│  │  └─ setActiveCodeClick(CodeClickEvent)
│  │     └─ updateSelection({ functionName })
│  │
│  ├─ handleTestClick(test)
│  │  └─ setActiveCodeClick(CodeClickEvent)
│  │     └─ updateSelection({ functionName, testCaseName })
│  │
│  └─ handleTestRun(test)
│     └─ runBamlTests([{ functionName, testName }])
│
└─ Utility Buttons
   ├─ Add File → sdk.files.update() → window.postMessage()
   └─ Edit Function → sdk.files.update() → window.postMessage()
```

## Navigation Heuristic Decision Tree

```
CodeClickEvent
    │
    ├─ Type === 'test'
    │  │
    │  └─ Find workflow being tested
    │     ├─ Workflow exists? → switch-workflow
    │     └─ Workflow not found? → empty-state
    │
    └─ Type === 'function'
       │
       ├─ Priority 1: Function in current workflow?
       │  └─ YES → select-node
       │
       ├─ Priority 2: Function in another workflow?
       │  └─ YES → switch-and-select
       │
       ├─ Priority 3: Function has tests?
       │  └─ YES → show-function-tests (LLM-only view)
       │
       └─ Priority 4: No context
          └─ empty-state
```

## File Paths Quick Lookup

### State Management
- Core atoms: `/src/sdk/atoms/core.atoms.ts` (638 lines)
- Test atoms: `/src/sdk/atoms/test.atoms.ts` (150 lines)
- Backward compat: `/src/shared/baml-project-panel/atoms.ts`

### Components
- DebugPanel: `/src/features/debug-panel/components/DebugPanel.tsx` (349 lines)
- DetailPanel: `/src/features/detail-panel/components/DetailPanel.tsx`
- Graph nodes: `/src/graph-primitives/nodes/`
- Graph edges: `/src/graph-primitives/edges/`

### Navigation
- Hook: `/src/features/navigation/hooks/useCodeNavigation.ts` (252 lines)
- Heuristic: `/src/sdk/navigationHeuristic.ts` (250+ lines)

### Types & Interfaces
- SDK types: `/src/sdk/types.ts` (350+ lines)
- Interface types: `/src/sdk/interface/types.ts` (200+ lines)
- Mock data: `/src/mock-data/data-workflow.ts`
- Graph types: `/src/mock-data/types.ts`

### SDK
- Main SDK: `/src/sdk/index.ts` (600+ lines)
- Provider: `/src/sdk/provider.tsx` (153 lines)
- EventListener: `/src/baml_wasm_web/EventListener.tsx` (150+ lines)

## Atom Update Patterns

### Pattern 1: Direct Write-Only Atoms
```typescript
// Define
export const updateSelectionAtom = atom(
  null,
  (get, set, update) => {
    set(selectedFunctionNameAtom, update.functionName);
    set(selectedTestCaseNameAtom, update.testCaseName ?? null);
  }
);

// Use
const updateSelection = useSetAtom(updateSelectionAtom);
updateSelection({ functionName: 'myFunc' });
```

### Pattern 2: Derived Atoms (Computed State)
```typescript
export const selectedFunctionObjectAtom = atom((get) => {
  const funcName = get(selectedFunctionNameAtom);
  if (!funcName) return null;
  const functionsMap = get(allFunctionsMapAtom);
  return functionsMap.get(funcName) || null;
});
```

### Pattern 3: atomFamily for Per-Item State
```typescript
export const nodeStateAtomFamily = atomFamily((nodeId: string) =>
  atom<NodeExecutionState>('not-started')
);

// Use in component
const [state, setState] = useAtom(nodeStateAtomFamily(nodeId));
```

### Pattern 4: Registry for Derived Computation
```typescript
const nodeRegistryAtom = atom<Set<string>>(new Set());

export const allNodeStatesAtom = atom((get) => {
  const registry = get(nodeRegistryAtom);
  const states = new Map<string, NodeExecutionState>();
  registry.forEach((nodeId) => {
    states.set(nodeId, get(nodeStateAtomFamily(nodeId)));
  });
  return states;
});
```

## Key Performance Optimizations

1. **atomFamily** - Per-node state doesn't cause sibling re-renders
2. **Derived atoms** - Computed state cached until dependencies change
3. **O(1) function lookup** - `allFunctionsMapAtom` uses Map instead of array search
4. **Debounced file updates** - 50ms debounce on sdk.files.update()
5. **Immutable runtime** - No mutations, new instance on changes

## Event Flow for File Changes

```
DebugPanel "Add File" button
    │
    └─→ sdk.files.getCurrent()
        └─→ Add new file to files object
            └─→ window.postMessage({ 
                  source: 'lsp_message',
                  payload: { method: 'runtime_updated', params: { files } }
                })
            └─→ EventListener catches postMessage
                └─→ handleLSPMessage()
                    └─→ sdk.files.update(files)
                        └─→ debouncedUpdateFiles()
                            └─→ storage.setBAMLFiles()
                                └─→ bamlFilesTrackedAtom updated
                                    └─→ Triggers runtime recreation
                                        └─→ runtimeInstanceAtom updated
                                            └─→ All derived atoms cascade update
                                                └─→ Components re-render
```

## Testing the Navigation System

Use DebugPanel to:
1. Click functions → Watch handleFunctionClick()
2. Click tests → Watch handleTestClick()
3. Observe activeCodeClickAtom changes
4. See useCodeNavigation() execute NavigationAction
5. Verify UI updates (workflow switch, node select, detail panel)

Watch browser console for logs:
- `[DebugPanel]` - Panel lifecycle
- `🔍 Simulated X click:` - Click events
- `🧭 Navigation action:` - Heuristic results
- `📍 Code click event:` - Navigation hook triggered

