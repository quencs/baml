# Cursor to CodeClick Event Unification

**Date:** 2025-11-04
**Status:** Design Proposal

---

## Executive Summary

The `update_cursor` message from VSCode and `CodeClickEvent` in baml-graph serve overlapping purposes but with different levels of semantic richness. **They can and should be unified** by transforming cursor events into enriched CodeClick events using WASM runtime introspection.

---

## Current State Analysis

### 1. VSCode's `update_cursor` Message

**Location:** `apps/vscode-ext/src/extension.ts:320-331`

**What it sends:**
```typescript
{
  source: 'ide_message',
  payload: {
    command: 'update_cursor',
    content: {
      fileName: string,  // e.g., "main.baml"
      line: number,       // 0-indexed line number
      column: number      // 0-indexed column number
    }
  }
}
```

**Trigger:** VSCode text editor cursor position change (selection change event)

**Purpose:** Notify playground which function/test the user is currently viewing

**Limitations:**
- Only has positional data (file, line, column)
- No semantic information about what's at that position
- Webview must derive meaning from the position

---

### 2. playground-common's `updateCursorAtom`

**Location:** `packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts:84-139`

**What it does:**
```typescript
export const updateCursorAtom = atom(null, (get, set, cursor: {
  fileName: string;
  line: number;
  column: number;
}) => {
  const runtime = get(runtimeAtom)?.rt;
  const fileContent = get(filesAtom)[cursor.fileName];

  // Convert line/column to byte index
  const cursorIdx = calculateByteIndex(fileContent, cursor.line, cursor.column);

  // 1. Find function at cursor position
  const selectedFunc = runtime.get_function_at_position(
    fileName,
    get(selectedFunctionAtom) ?? '',
    cursorIdx
  );

  if (selectedFunc) {
    set(selectedFunctionAtom, selectedFunc.name);

    // 2. Check if cursor is within a test case
    const selectedTestcase = runtime.get_testcase_from_position(
      selectedFunc,
      cursorIdx
    );

    if (selectedTestcase) {
      set(selectedTestcaseAtom, selectedTestcase.name);

      // 3. If test case, check if it's testing a nested function
      const nestedFunc = runtime.get_function_of_testcase(
        fileName,
        cursorIdx
      );

      if (nestedFunc) {
        set(selectedFunctionAtom, nestedFunc.name);
      }
    }
  }
});
```

**Key WASM Runtime Methods:**
- `runtime.get_function_at_position(fileName, currentFunc, byteIndex)` - Returns function at cursor
- `runtime.get_testcase_from_position(function, byteIndex)` - Returns test case if cursor is in one
- `runtime.get_function_of_testcase(fileName, byteIndex)` - Returns which function the test is for

**Output:** Sets `selectedFunctionAtom` and `selectedTestcaseAtom`

**Purpose:** Derive semantic meaning from cursor position and update selection state

---

### 3. baml-graph's `CodeClickEvent`

**Location:** `apps/baml-graph/src/sdk/types.ts:288-299`

**What it contains:**
```typescript
export type CodeClickEvent =
  | {
      type: 'function';
      functionName: string;        // "ExtractResume"
      functionType: 'workflow' | 'function' | 'llm_function';
      filePath: string;            // "main.baml"
    }
  | {
      type: 'test';
      testName: string;            // "test_valid_resume"
      functionName: string;        // Function being tested
      filePath: string;
      nodeType: 'llm_function' | 'function';
    };
```

**Trigger:** Simulated clicks in Debug Panel for testing navigation heuristics

**Purpose:**
- Rich semantic information about what was clicked
- Powers sophisticated navigation logic
- Determines whether to switch workflows, select nodes, etc.

**Usage:** `apps/baml-graph/src/features/navigation/hooks/useCodeNavigation.ts:28-170`

---

## Key Insight: Cursor → CodeClick Transformation

The WASM runtime methods in `updateCursorAtom` **already extract the semantic information** needed to create a `CodeClickEvent`!

### Current Flow (playground-common):
```
VSCode cursor change
  → update_cursor {fileName, line, column}
    → updateCursorAtom
      → WASM runtime introspection
        → Update selectedFunctionAtom
        → Update selectedTestcaseAtom
```

### Proposed Unified Flow:
```
VSCode cursor change
  → update_cursor {fileName, line, column}
    → enrichCursorToCodeClick()
      → WASM runtime introspection
        → Create CodeClickEvent {type, functionName, functionType, ...}
          → Navigation heuristic
            → Unified navigation logic (switch workflow, select node, etc.)
```

---

## Proposed Unification

### Step 1: Create Enrichment Function

**Location:** `packages/playground-common/src/shared/atoms/cursor-enrichment.ts` (new file)

```typescript
import type { WasmRuntime } from '@baml/wasm-schema-web';
import type { CodeClickEvent } from '../sdk/types';

/**
 * Enriches a cursor position into a semantic CodeClickEvent using WASM runtime
 */
export function enrichCursorToCodeClick(
  cursor: { fileName: string; line: number; column: number },
  runtime: WasmRuntime,
  fileContent: string,
  currentSelectedFunction?: string
): CodeClickEvent | null {
  // Convert line/column to byte index
  const lines = fileContent.split('\n');
  let cursorIdx = 0;
  for (let i = 0; i < cursor.line; i++) {
    cursorIdx += (lines[i]?.length ?? 0) + 1; // +1 for newline
  }
  cursorIdx += cursor.column;

  // 1. Get function at cursor position
  const selectedFunc = runtime.get_function_at_position(
    cursor.fileName,
    currentSelectedFunction ?? '',
    cursorIdx
  );

  if (!selectedFunc) {
    return null; // Cursor not in any function
  }

  // 2. Check if cursor is within a test case
  const selectedTestcase = runtime.get_testcase_from_position(
    selectedFunc,
    cursorIdx
  );

  if (selectedTestcase) {
    // It's a test click!
    // Check if test is testing a nested function
    const testedFunc = runtime.get_function_of_testcase(
      cursor.fileName,
      cursorIdx
    );

    return {
      type: 'test',
      testName: selectedTestcase.name,
      functionName: testedFunc?.name ?? selectedFunc.name,
      filePath: cursor.fileName,
      nodeType: determineFunctionType(testedFunc ?? selectedFunc)
    };
  }

  // 3. It's a function click
  return {
    type: 'function',
    functionName: selectedFunc.name,
    functionType: determineWorkflowOrFunction(selectedFunc),
    filePath: cursor.fileName
  };
}

/**
 * Helper to determine if function is a workflow, regular function, or LLM function
 */
function determineWorkflowOrFunction(func: any): 'workflow' | 'function' | 'llm_function' {
  // Check if function is actually a workflow
  if (func.workflow_type) {
    return 'workflow';
  }

  // Check if it's an LLM function
  if (func.type === 'llm_function' || func.client) {
    return 'llm_function';
  }

  return 'function';
}

/**
 * Helper to determine node type for tests
 */
function determineFunctionType(func: any): 'llm_function' | 'function' {
  if (func.type === 'llm_function' || func.client) {
    return 'llm_function';
  }
  return 'function';
}
```

---

### Step 2: Update `updateCursorAtom` to Emit CodeClickEvent

**Location:** `packages/playground-common/src/shared/atoms/ui.atoms.ts` (after migration)

```typescript
import { enrichCursorToCodeClick } from './cursor-enrichment';

// New atom to store the enriched code click
export const codeClickEventAtom = atom<CodeClickEvent | null>(null);

// Updated cursor atom that creates CodeClickEvent
export const updateCursorAtom = atom(
  null,
  (get, set, cursor: { fileName: string; line: number; column: number }) => {
    const runtime = get(runtimeAtom)?.runtime;
    if (!runtime) return;

    const fileContent = get(filesAtom)[cursor.fileName];
    if (!fileContent) return;

    // Enrich cursor to CodeClickEvent
    const codeClickEvent = enrichCursorToCodeClick(
      cursor,
      runtime,
      fileContent,
      get(selectedFunctionAtom) ?? undefined
    );

    if (codeClickEvent) {
      // Emit the enriched event
      set(codeClickEventAtom, codeClickEvent);

      // Also update legacy atoms for backward compatibility (during migration)
      if (codeClickEvent.type === 'test') {
        set(selectedFunctionAtom, codeClickEvent.functionName);
        set(selectedTestcaseAtom, codeClickEvent.testName);
      } else {
        set(selectedFunctionAtom, codeClickEvent.functionName);
        set(selectedTestcaseAtom, null);
      }
    }
  }
);
```

---

### Step 3: Unified Navigation Hook

**Location:** `packages/playground-common/src/features/navigation/useNavigationHandler.ts` (new)

```typescript
import { useEffect } from 'react';
import { useAtomValue } from 'jotai';
import { codeClickEventAtom } from '@/shared/atoms/ui.atoms';
import { useBAMLSDK } from '@/sdk/provider';
import { determineNavigationAction, getCurrentNavigationState } from '@/sdk/navigationHeuristic';

/**
 * Unified hook that handles navigation for both:
 * - VSCode cursor changes (enriched to CodeClickEvent)
 * - Debug panel clicks (direct CodeClickEvent)
 */
export function useNavigationHandler() {
  const codeClickEvent = useAtomValue(codeClickEventAtom);
  const sdk = useBAMLSDK();

  useEffect(() => {
    if (!codeClickEvent) return;

    console.log('📍 Code click event:', codeClickEvent);

    // Get current navigation state
    const navState = getCurrentNavigationState(sdk);

    // Determine navigation action using heuristic
    const action = determineNavigationAction(codeClickEvent, navState);
    console.log('🧭 Navigation action:', action);

    // Execute action
    switch (action.type) {
      case 'switch-workflow':
        sdk.workflows.setActive(action.workflowId);
        break;

      case 'select-node':
        sdk.selectNode(action.nodeId);
        if (action.testId) {
          sdk.selectTestInput(action.nodeId, action.testId);
        }
        break;

      case 'switch-and-select':
        sdk.workflows.setActive(action.workflowId);
        setTimeout(() => {
          sdk.selectNode(action.nodeId);
          if (action.testId) {
            sdk.selectTestInput(action.nodeId, action.testId);
          }
        }, 100); // Allow workflow switch to complete
        break;

      case 'show-function-tests':
        // Show standalone function with tests in isolation
        sdk.showFunctionInIsolation(action.functionName);
        break;

      case 'empty-state':
        console.warn('No navigation action for:', action.reason);
        break;
    }
  }, [codeClickEvent, sdk]);
}
```

---

### Step 4: EventListener Integration

**Location:** `packages/playground-common/src/baml_wasm_web/EventListener.tsx`

```typescript
export function EventListener() {
  const updateCursor = useSetAtom(updateCursorAtom);
  const setCodeClick = useSetAtom(codeClickEventAtom);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      const { source, payload } = event.data;

      if (source === 'ide_message') {
        switch (payload.command) {
          case 'update_cursor':
            // This will automatically create a CodeClickEvent via enrichment
            updateCursor(payload.content);
            break;
        }
      }
      else if (source === 'lsp_message') {
        switch (payload.method) {
          case 'workspace/executeCommand':
            if (payload.params.command === 'baml.openBamlPanel') {
              // Direct CodeClickEvent from command
              setCodeClick({
                type: 'function',
                functionName: payload.params.arguments[0].functionName,
                functionType: 'function', // Could enhance this
                filePath: '' // Could get from runtime
              });
            }
            else if (payload.params.command === 'baml.runBamlTest') {
              // Direct CodeClickEvent from test command
              setCodeClick({
                type: 'test',
                testName: payload.params.arguments[0].testCaseName,
                functionName: payload.params.arguments[0].functionName,
                filePath: '',
                nodeType: 'function'
              });
            }
            break;
        }
      }
    };

    window.addEventListener('message', handler);
    return () => window.removeEventListener('message', handler);
  }, [updateCursor, setCodeClick]);

  return null;
}
```

---

## Benefits of Unification

### 1. Single Source of Truth for Navigation
- One `codeClickEventAtom` drives all navigation
- Same logic for VSCode cursor changes and debug panel clicks
- Easier to reason about and test

### 2. Richer Semantic Information
- Cursor positions automatically enriched with function/test metadata
- Type information (workflow vs function vs llm_function) available
- Powers sophisticated navigation heuristics

### 3. Better Navigation Heuristics
- Can use baml-graph's priority-based decision tree
- Context-aware: stays in current workflow when possible
- Handles edge cases (standalone functions, missing workflows)

### 4. Easier Testing
- Mock CodeClickEvents for unit tests
- Debug panel creates same events as real IDE
- Navigation logic independent of event source

### 5. Future Extensibility
- Easy to add new event sources (JetBrains, Zed)
- Can enhance events with more metadata
- Could add user preferences for navigation behavior

---

## Migration Strategy

### Phase 1: Add Enrichment (No Breaking Changes)
1. Create `cursor-enrichment.ts` with enrichment function
2. Add `codeClickEventAtom` alongside existing atoms
3. Update `updateCursorAtom` to populate both old and new atoms
4. Add feature flag to enable new navigation logic

### Phase 2: Implement Unified Navigation
1. Copy navigation heuristic from baml-graph
2. Create `useNavigationHandler` hook
3. Test with feature flag enabled
4. Gradually migrate components to use new system

### Phase 3: Deprecate Old Atoms
1. Remove direct usage of `selectedFunctionAtom` / `selectedTestcaseAtom`
2. Make them derived from `codeClickEventAtom` + navigation state
3. Update all components to use navigation handler
4. Remove feature flag

---

## Implementation Example

### Before (Current playground-common):
```typescript
// EventListener.tsx
case 'update_cursor':
  updateCursor(payload.content);  // Updates selectedFunctionAtom
  break;

// Component.tsx
const selectedFunction = useAtomValue(selectedFunctionAtom);
const selectedTest = useAtomValue(selectedTestcaseAtom);
```

### After (Unified approach):
```typescript
// EventListener.tsx
case 'update_cursor':
  updateCursor(payload.content);  // Creates CodeClickEvent via enrichment
  break;

// App.tsx
useNavigationHandler();  // Listens to codeClickEventAtom, handles all navigation

// Component.tsx
const selectedNode = useAtomValue(selectedNodeIdAtom);  // Set by navigation handler
const workflow = useAtomValue(activeWorkflowAtom);       // Set by navigation handler
```

---

## Additional Enhancements

### 1. Debouncing Cursor Events
VSCode sends cursor updates frequently. We should debounce:

```typescript
export const debouncedCursorAtom = atomWithDebounce(
  codeClickEventAtom,
  150  // 150ms delay
);
```

### 2. Cursor History
Track cursor history for "go back" navigation:

```typescript
export const cursorHistoryAtom = atom<CodeClickEvent[]>([]);

export const addToCursorHistoryAtom = atom(
  null,
  (get, set, event: CodeClickEvent) => {
    const history = get(cursorHistoryAtom);
    set(cursorHistoryAtom, [...history, event].slice(-20)); // Keep last 20
  }
);
```

### 3. Enhanced Metadata
Add more context to CodeClickEvent:

```typescript
export type EnrichedCodeClickEvent = CodeClickEvent & {
  timestamp: number;
  lineNumber: number;
  columnNumber: number;
  containingClass?: string;
  containingEnum?: string;
  // Could add AST node information, etc.
};
```

---

## Questions to Consider

### 1. Should we enhance VSCode to send richer data?
**Option A:** Keep VSCode sending minimal data, enrich in webview
- ✅ Separation of concerns
- ✅ Works with other IDEs that don't have semantic info
- ❌ Requires WASM runtime in webview

**Option B:** Have VSCode send enriched CodeClickEvent directly
- ✅ No WASM dependency in webview
- ✅ Faster (no runtime introspection needed)
- ❌ More complex VSCode extension
- ❌ Tighter coupling

**Recommendation:** Option A for now, consider Option B if performance becomes an issue

### 2. How to handle ambiguous cursor positions?
If cursor is between functions or in whitespace:
- Keep last valid selection
- Or clear selection and show empty state
- Or show nearest function

### 3. Should navigation be automatic on cursor change?
**Current:** Automatic - cursor change immediately updates selection
**Alternative:** Require explicit action (e.g., click, keyboard shortcut)

**Recommendation:** Keep automatic for now, but consider adding preference

---

## Conclusion

Unifying `update_cursor` and `CodeClickEvent` provides significant architectural benefits:

1. **Simplified mental model** - one event type for all code navigation
2. **Richer semantics** - cursor positions enriched with function/test metadata
3. **Better navigation** - sophisticated heuristics from baml-graph
4. **Easier testing** - mock events, debug panel uses same paths
5. **Future-proof** - extensible for new IDEs and features

The key enabler is the existing WASM runtime methods (`get_function_at_position`, etc.) that already provide the semantic information needed to transform low-level cursor positions into high-level code click events.

**Next Steps:**
1. Implement `cursor-enrichment.ts` enrichment function
2. Add `codeClickEventAtom` to unified atom structure
3. Update `updateCursorAtom` to create CodeClickEvents
4. Port navigation heuristic from baml-graph
5. Test with feature flag before full rollout
