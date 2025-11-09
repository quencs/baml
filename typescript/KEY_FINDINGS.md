# Key Findings: Playground-Common Package

## 1. State Management Architecture

| Aspect | Implementation | Notes |
|--------|----------------|-------|
| State Library | Jotai atoms | Minimal, focused on derivation |
| Core State | `runtimeInstanceAtom` | Single source of truth |
| Atoms Count | 40+ | Split between core (30+) and test (15+) |
| Derived Atoms | 15+ | Computed from core state |
| atomFamily Usage | `nodeStateAtomFamily`, `workflowExecutionsAtomFamily` | O(1) per-item updates |
| Storage Pattern | Write-only atoms | `updateSelectionAtom` as central update point |

## 2. DebugPanel Component

| Feature | Implementation | Location |
|---------|----------------|----------|
| File Loading | `sdk.diagnostics.getBAMLFiles()` | Line 29 |
| Function Click | `handleFunctionClick()` | Lines 50-62 |
| Test Click | `handleTestClick()` | Lines 64-77 |
| Test Run | `handleTestRun()` | Lines 79-87 |
| File Addition | `handleAddNewFile()` | Lines 101-167 |
| Function Edit | `handleModifyFunction()` | Lines 169-214 |
| UI Layout | Bottom-right floating panel | Line 217 |
| Active Highlight | Blue background + `isActive()` | Lines 285, 310 |

## 3. Navigation System

| Component | Lines | Key Function | Purpose |
|-----------|-------|--------------|---------|
| CodeClickEvent Type | types.ts:339-350 | Event definition | Captures click intent |
| Navigation Heuristic | navigationHeuristic.ts:109-122 | `determineNavigationAction()` | Decision algorithm |
| Navigation Hook | useCodeNavigation.ts:46-250 | Effect listener | Executes decisions |
| Navigation Actions | navigationHeuristic.ts:83-88 | 5 action types | Possible outcomes |
| Timeout Management | useCodeNavigation.ts:59,236 | Array tracking | Cleanup on unmount |

## 4. Navigation Heuristic Priorities

```
TEST Click:
  1. Is target a workflow? → switch-workflow
  2. Otherwise → empty-state

FUNCTION Click:
  1. In current workflow? → select-node
  2. In another workflow? → switch-and-select
  3. Has tests? → show-function-tests (LLM-only)
  4. No context? → empty-state
```

## 5. Critical Data Structures

### CodeClickEvent
```typescript
type: 'function' | 'test'
functionName: string
functionType?: 'workflow' | 'function' | 'llm_function'
testName?: string
filePath: string
nodeType?: 'llm_function' | 'function'
```

### BAMLFile
```typescript
path: string
functions: FunctionWithCallGraph[]
tests: BAMLTest[]
```

### NavigationAction
```typescript
| { type: 'switch-workflow'; workflowId: string }
| { type: 'select-node'; workflowId: string; nodeId: string; testId?: string }
| { type: 'switch-and-select'; workflowId: string; nodeId: string; testId?: string }
| { type: 'show-function-tests'; functionName: string; tests: string[] }
| { type: 'empty-state'; reason: string; functionName: string }
```

## 6. Critical Atoms for Navigation

| Atom | Type | Purpose | Consumers |
|------|------|---------|-----------|
| `activeCodeClickAtom` | Write | Stores click event | useCodeNavigation |
| `updateSelectionAtom` | Write-only | Updates function/test | DebugPanel, navigation |
| `selectedFunctionNameAtom` | Read/Write | Current function | Selection derived atoms |
| `selectedTestCaseNameAtom` | Read/Write | Current test | Selection derived atoms |
| `activeWorkflowIdAtom` | Read/Write | Current workflow | useCodeNavigation |
| `selectedNodeIdAtom` | Read/Write | Graph selection | useCodeNavigation |
| `detailPanelAtom` | Read/Write | Panel state | useCodeNavigation |

## 7. Update Flow Analysis

**Single Click → UI Update (5 steps)**:
1. DebugPanel calls `setActiveCodeClick(CodeClickEvent)`
2. `activeCodeClickAtom` updates
3. `useCodeNavigation()` hook effect fires
4. `determineNavigationAction()` returns action
5. Action handlers update workflow/node atoms
6. Components re-render based on atom changes

**Total Latency**: Synchronous (unless async workflow loading)

## 8. Missing Navigation Features

| Feature | Status | Gap |
|---------|--------|-----|
| Conditional.baml clicking | Files loaded, no special UI | No conditional-specific handling |
| File header clicking | Not implemented | Can't navigate to file |
| Breadcrumb navigation | Not implemented | No file context shown |
| Branch condition display | Not implemented | Can't see conditional branches |
| View auto-switching | Basic, manual | No smart mode detection |
| Execution refresh | Working via atoms | But could be optimized |

## 9. Performance Characteristics

| Feature | Implementation | Complexity |
|---------|----------------|-----------|
| Function lookup | Map-based (allFunctionsMapAtom) | O(1) |
| Workflow search | Array iteration | O(n) |
| File parsing | During SDK initialization | One-time |
| Node state updates | atomFamily (per-node) | O(1), no sibling renders |
| Derived atoms | Cached until deps change | O(1) reads |
| File updates | Debounced 50ms | Prevents thrashing |

## 10. Critical File Paths

| Category | Path | Size |
|----------|------|------|
| Core State | `/src/sdk/atoms/core.atoms.ts` | 638 lines |
| Test State | `/src/sdk/atoms/test.atoms.ts` | 150 lines |
| Debug UI | `/src/features/debug-panel/components/DebugPanel.tsx` | 349 lines |
| Navigation | `/src/features/navigation/hooks/useCodeNavigation.ts` | 252 lines |
| Heuristic | `/src/sdk/navigationHeuristic.ts` | 250+ lines |
| SDK | `/src/sdk/index.ts` | 600+ lines |
| Provider | `/src/sdk/provider.tsx` | 153 lines |
| Event Handling | `/src/baml_wasm_web/EventListener.tsx` | 150+ lines |

## 11. Integration Points

```
DebugPanel
  ├─ Calls: sdk.diagnostics.getBAMLFiles()
  ├─ Updates: activeCodeClickAtom
  ├─ Updates: updateSelectionAtom
  └─ Posts: window.postMessage() for file changes

useCodeNavigation Hook
  ├─ Reads: activeCodeClickAtom
  ├─ Reads: NavigationState from SDK
  ├─ Calls: setActiveWorkflow()
  ├─ Calls: setSelectedNodeId()
  ├─ Updates: unifiedSelectionAtom
  └─ Calls: panToNodeIfNeeded()

SDK
  ├─ Provides: bamlFilesAtom
  ├─ Manages: runtimeInstanceAtom
  ├─ Exposes: workflows, files, diagnostics APIs
  └─ Triggers: Runtime recreation on file changes

EventListener
  ├─ Routes: IDE messages
  ├─ Routes: LSP messages
  ├─ Calls: sdk.files.update()
  └─ Triggers: Runtime recreation via atoms
```

## 12. Code Quality Observations

### Strengths
- Clear separation of concerns (state/navigation/UI)
- Comprehensive type system (no implicit any)
- Well-organized atom definitions with comments
- Immutable runtime pattern prevents state corruption
- Extensive use of derived atoms for computed state
- Performance optimizations (atomFamily, Map lookups)

### Areas for Improvement
- Navigation heuristic could be simplified
- No integration tests for navigation flow
- Limited error handling for edge cases
- Conditional blocks not treated as first-class nodes
- File-level navigation not implemented
- View mode switching could be more explicit

## 13. Testing Recommendations

```typescript
// Test navigation heuristic
test('function click in current workflow → select-node', () => {
  const event = { type: 'function', functionName: 'foo' };
  const action = determineNavigationAction(event, {
    activeWorkflowId: 'workflow1',
    workflows: [{ id: 'workflow1', nodes: [{ id: 'foo' }] }]
  });
  expect(action.type).toBe('select-node');
});

// Test atom updates
test('updateSelectionAtom updates both atoms', () => {
  const store = createStore();
  store.set(updateSelectionAtom, { functionName: 'myFunc', testCaseName: 'test1' });
  expect(store.get(selectedFunctionNameAtom)).toBe('myFunc');
  expect(store.get(selectedTestCaseNameAtom)).toBe('test1');
});

// Test DebugPanel integration
test('clicking function in DebugPanel updates activeCodeClickAtom', () => {
  const { getByText } = render(<DebugPanel />);
  fireEvent.click(getByText('myFunction'));
  expect(store.get(activeCodeClickAtom).type).toBe('function');
});
```

## 14. Roadmap for Conditional.baml Support

1. **Parse Conditionals**: Extend BAML parser to identify conditional blocks
2. **Type System**: Add 'conditional' to NodeType enum
3. **DebugPanel**: Show conditional branches under functions
4. **Navigation**: Handle conditional node clicking
5. **Graph UI**: Visual representation of branches
6. **Detail Panel**: Show branch conditions and paths
7. **Execution View**: Highlight active branch during execution

## 15. Known Limitations

| Limitation | Impact | Workaround |
|-----------|--------|-----------|
| No file-level navigation | Can't browse by file | Use function list |
| Conditional blocks hidden | Can't click conditionals | They appear as functions |
| No breadcrumb | Lost context switching | See atom state in console |
| View mode manual | Must click to switch | Uses heuristic for defaults |
| No undo/redo | Can't go back | Refresh browser |
| Single active workflow | Can't compare | Would need new UI layout |

---

**Last Updated**: November 9, 2025
**Package**: playground-common
**Branch**: aaron/graphs
**Focus**: State management, navigation heuristic, DebugPanel
