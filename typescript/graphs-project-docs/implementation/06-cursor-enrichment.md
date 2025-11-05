# Phase 6: Cursor-to-CodeClick Enrichment

**Timeline:** Week 4-5
**Dependencies:** Phase 5 (EventListener Refactor)
**Risk Level:** Medium

## Purpose

Transform low-level cursor position events into rich semantic CodeClick events using WASM runtime introspection. This unifies cursor movements and explicit clicks, enabling sophisticated navigation heuristics for both.

## What This Document Will Cover

- `enrichCursorToCodeClick()` function implementation
- WASM runtime introspection methods usage
- CodeClickEvent type definition
- Cursor position to byte index conversion
- Function type detection (workflow, function, llm_function)
- Test case detection at cursor position
- Debouncing strategy for cursor events
- Integration with existing `updateCursorAtom`
- Backward compatibility with `selectedFunctionAtom`/`selectedTestcaseAtom`

## Key Decisions

- WASM runtime methods already extract semantic info (reuse them)
- `updateCursorAtom` creates CodeClickEvent instead of just updating selection
- Debounce cursor events (300ms) to avoid navigation jank
- Keep backward compatibility atoms during migration
- CodeClickEvent has rich metadata (type, functionName, functionType, filePath, span)

## Source Files to Reference

### Current Cursor Handling
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/atoms.ts` (lines 84-139 - updateCursorAtom)

### WASM Runtime Methods
- `runtime.get_function_at_position()` (used at line 113)
- `runtime.get_testcase_from_position()` (used at line 121)
- `runtime.get_function_of_testcase()` (used at line 128)

### CodeClick Event Definition
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/types.ts` (lines 288-299)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/CURSOR_TO_CODECLICK_UNIFICATION.md` (entire document - complete design)
- `/Users/aaronvillalpando/Projects/baml/typescript/BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md` (lines 1080-1305 - Cursor to CodeClick Unification section)
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC.md` (lines 692-1084 - Unifying Cursor Updates section)

## Implementation Checklist

- [ ] Create `shared/atoms/cursor-enrichment.ts`
- [ ] Implement `enrichCursorToCodeClick()` function
- [ ] Implement `calculateByteIndex()` helper
- [ ] Implement `determineFunctionType()` helper (workflow/function/llm_function)
- [ ] Implement `determineFunctionNodeType()` helper (llm_function/function)
- [ ] Update `updateCursorAtom` to create CodeClickEvent
- [ ] Add `codeClickEventAtom` to unified atoms
- [ ] Implement debouncing with `atomWithDebounce`
- [ ] Keep backward compatibility (`selectedFunctionAtom`, `selectedTestcaseAtom`)
- [ ] Update EventListener to emit CodeClickEvent for LSP commands
- [ ] Add unit tests for enrichment function
- [ ] Test with different cursor positions (function, test, whitespace)
- [ ] Document CodeClickEvent structure

## Validation Criteria

- [ ] Cursor in function definition → creates function CodeClickEvent
- [ ] Cursor in test case → creates test CodeClickEvent
- [ ] Cursor in whitespace → returns null (no event)
- [ ] Function type detected correctly (workflow, function, llm_function)
- [ ] Test case function reference resolved correctly
- [ ] Debouncing works (no jank when scrolling)
- [ ] Backward compatibility atoms still update
- [ ] Navigation system receives enriched events
