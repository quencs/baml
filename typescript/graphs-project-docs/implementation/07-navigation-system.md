# Phase 7: Navigation System Integration

**Timeline:** Week 5
**Dependencies:** Phase 6 (Cursor Enrichment)
**Risk Level:** Medium

## Purpose

Integrate the sophisticated navigation heuristic from baml-graph that determines how to respond to CodeClick events (cursor movements, debug panel clicks, LSP commands). This provides context-aware navigation that stays in current workflow when possible, switches workflows intelligently, and handles standalone functions gracefully.

## What This Document Will Cover

- Navigation heuristic algorithm and priority rules
- `determineNavigationAction()` function implementation
- Navigation action types (switch-workflow, select-node, show-function-tests, etc.)
- `useCodeNavigation()` hook implementation
- Integration with ReactFlow for camera panning
- Workflow switching logic
- Node selection and detail panel opening
- Test case selection in detail panel
- Empty state handling
- Navigation history tracking (optional)

## Key Decisions

- Priority-based decision tree (stay in workflow > switch workflow > show isolated > empty)
- Cursor and explicit clicks use same navigation logic
- Debounced cursor events (300ms) to avoid jank
- Camera panning to selected nodes
- Navigation actions are declarative (not imperative)

## Source Files to Reference

### Navigation Heuristic
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/navigationHeuristic.ts` (lines 1-300 - complete algorithm)

### Navigation Hook
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/navigation/hooks/useCodeNavigation.ts` (lines 1-170 - hook implementation)

### Camera Panning
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/utils/cameraPan.ts` (camera pan utility)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md` (lines 1227-1269 - Navigation heuristic integration)
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC.md` (lines 1160-1173 - Navigation Heuristic section)
- `/Users/aaronvillalpando/Projects/baml/typescript/CURSOR_TO_CODECLICK_UNIFICATION.md` (lines 227-320 - Navigation integration)

## Implementation Checklist

- [ ] Copy `navigationHeuristic.ts` to `packages/playground-common/src/sdk/`
- [ ] Copy navigation types (NavigationAction, NavigationState)
- [ ] Implement `determineNavigationAction()` function
- [ ] Implement `getCurrentNavigationState()` helper
- [ ] Create `features/navigation/` directory in playground-common
- [ ] Implement `useCodeNavigation()` hook
- [ ] Integrate with `activeCodeClickAtom` (or `debouncedCodeClickAtom`)
- [ ] Add workflow switching logic
- [ ] Add node selection logic
- [ ] Add camera panning integration (if graph visible)
- [ ] Add test case selection in detail panel
- [ ] Add empty state handling
- [ ] Handle LLM-only mode (standalone functions)
- [ ] Add unit tests for navigation heuristic
- [ ] Add integration tests for navigation hook
- [ ] Document navigation rules and priority

## Validation Criteria

- [ ] Click on workflow function → switches to workflow, selects node
- [ ] Click on function in current workflow → stays in workflow, selects node
- [ ] Click on test for workflow → switches to workflow
- [ ] Click on standalone LLM function → shows LLM-only view
- [ ] Cursor movement triggers debounced navigation
- [ ] Camera pans to selected node
- [ ] Detail panel opens when node selected
- [ ] Test case selected if test clicked
- [ ] Empty state shown for functions with no context
