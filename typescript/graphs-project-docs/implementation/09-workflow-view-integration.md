# Phase 9: Workflow View Integration

**Timeline:** Week 6-7
**Dependencies:** Phase 8 (Graph Components Package)
**Risk Level:** Medium

## Purpose

Integrate workflow graph visualization into playground-common, creating a unified view that switches between prompt preview (for standalone functions) and workflow graph (for functions in workflows). This is the primary user-facing feature of the integration.

## What This Document Will Cover

- `UnifiedPlaygroundView` component architecture
- View selection logic (prompt preview vs workflow graph vs LLM-only)
- `WorkflowView` component implementation
- Integration with `@baml/baml-graph-components`
- ReactFlow setup and configuration
- Graph toolbar implementation
- Workflow indicators and metadata display
- Responsive layout (graph + detail panel)
- Loading states and error handling
- Transition animations between views

## Key Decisions

- Single `UnifiedPlaygroundView` routes to correct view based on context
- Workflow graph shown when selected function is in a workflow
- Prompt preview shown for standalone LLM functions
- Empty state shown when no selection
- Detail panel appears as sidebar when node selected in graph
- Toolbar provides workflow controls (run, layout, settings)

## Source Files to Reference

### From baml-graph (Main App)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/App.tsx` (lines 35-227 - EditWorkFlow component)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/workflow/components/WorkflowToolbar.tsx` (toolbar implementation)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/workflow/components/WorkflowIndicator.tsx` (workflow name display)

### From playground-common (Existing Views)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/index.tsx` (lines 71-112 - PromptPreview)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/side-bar/index.tsx` (lines 47-261 - TestingSidebar)

### Graph Components Package
- `@baml/baml-graph-components` (from Phase 8)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC.md` (lines 1689-1777 - Unified View Component section)
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC_ANSWERS.md` (lines 230-280 - Prompt Preview Integration answer)

## Implementation Checklist

- [ ] Create `features/unified-view/` directory in playground-common
- [ ] Implement `UnifiedPlaygroundView` component (view router)
- [ ] Implement view selection logic (isLLMOnly, functionInWorkflow)
- [ ] Create `WorkflowView` component
- [ ] Set up ReactFlow in WorkflowView
- [ ] Import and use components from `@baml/baml-graph-components`
- [ ] Integrate `useGraphSync()` hook
- [ ] Integrate `useExecutionSync()` hook
- [ ] Create `WorkflowToolbar` component
- [ ] Add workflow controls (run, stop, layout, clear cache)
- [ ] Create `WorkflowIndicator` component
- [ ] Add responsive layout (ResizablePanelGroup)
- [ ] Preserve existing `PromptPreview` for standalone functions
- [ ] Add loading states for graph rendering
- [ ] Add error boundaries
- [ ] Add empty state component
- [ ] Style graph components consistently with playground theme
- [ ] Add smooth transitions between views
- [ ] Test view switching logic
- [ ] Document view selection rules

## Validation Criteria

- [ ] View switches correctly based on selection
- [ ] Workflow graph renders for functions in workflows
- [ ] Prompt preview renders for standalone LLM functions
- [ ] Empty state shows when nothing selected
- [ ] Graph layout algorithm works
- [ ] Nodes and edges render correctly
- [ ] Detail panel appears when node selected
- [ ] Toolbar controls work (run, layout)
- [ ] Responsive layout works on different screen sizes
- [ ] No layout thrashing or jank
- [ ] Loading states display correctly
- [ ] Error states handled gracefully
