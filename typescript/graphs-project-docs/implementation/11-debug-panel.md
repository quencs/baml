# Phase 11: Debug Panel Integration

**Timeline:** Week 7
**Dependencies:** Phase 9 (Workflow View Integration)
**Risk Level:** Low

## Purpose

Add the debug panel from baml-graph to playground-common as a development-only feature. This enables browser-based testing of navigation heuristics and workflow behavior without VSCode, with a UI toggle for enabling mock mode.

## What This Document Will Cover

- Debug panel component implementation
- File tree browser for BAML files
- Function list with click simulation
- Test list with run capability
- Mock mode toggle UI
- Conditional rendering (dev builds only)
- Integration with mock data provider
- CodeClick event emission from debug panel
- Navigation heuristic testing workflow
- Dev mode settings persistence

## Key Decisions

- Debug panel only appears in development builds (`import.meta.env.DEV`)
- Requires mock mode to be enabled (user toggles on)
- Positioned as overlay (top-left corner, per baml-graph)
- Emits CodeClick events (same as cursor/LSP commands)
- Uses mock data from MockDataProvider
- Persists enabled state in localStorage

## Source Files to Reference

### From baml-graph (Debug Panel)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/debug/components/DebugPanel.tsx` (lines 15-202 - complete implementation)

### Mock Data
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/mock.ts` (lines 390-564 - mock BAML files)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC.md` (lines 1087-1356 - Debug and Mock Capabilities section)
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC_ANSWERS.md` (lines 309-383 - Mock Data answer, lines 834-917 - Package Structure)

## Implementation Checklist

- [ ] Create `features/debug/` directory in playground-common
- [ ] Copy `DebugPanel` component from baml-graph
- [ ] Adapt to use unified atoms
- [ ] Implement file tree browser
- [ ] Implement function list with type badges
- [ ] Implement test list with run buttons
- [ ] Add CodeClick event emission on function/test clicks
- [ ] Create `DevModeToggle` component
- [ ] Add mock mode toggle checkbox
- [ ] Add debug panel visibility toggle
- [ ] Implement conditional rendering (dev builds only)
- [ ] Add to main app layout (conditional)
- [ ] Integrate with MockDataProvider for file data
- [ ] Persist mock mode enabled state in localStorage
- [ ] Style consistently with playground UI
- [ ] Add collapsible sections for file tree
- [ ] Add active state highlighting
- [ ] Add "Run workflow" button for tests
- [ ] Test navigation from debug panel clicks
- [ ] Document debug panel usage

## Validation Criteria

- [ ] Debug panel not included in production builds
- [ ] Mock mode toggle appears in dev builds
- [ ] Debug panel appears when mock mode enabled
- [ ] File tree displays mock BAML files
- [ ] Functions listed with correct type badges
- [ ] Tests listed under each function
- [ ] Clicking function emits CodeClick event
- [ ] Clicking test emits CodeClick event
- [ ] Navigation heuristic processes events correctly
- [ ] Workflow switches when appropriate
- [ ] Nodes selected correctly
- [ ] Test execution works from debug panel
- [ ] Active state highlights current selection
- [ ] Panel can be collapsed/expanded
- [ ] State persists across page reloads
