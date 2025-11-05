# Phase 10: Detail Panel Enhancement

**Timeline:** Week 7
**Dependencies:** Phase 9 (Workflow View Integration)
**Risk Level:** Low

## Purpose

Enhance the detail panel to work in both workflow and standalone contexts, adding a "Prompt" tab for LLM nodes and unifying the UI for inputs, outputs, logs, and LLM-specific information.

## What This Document Will Cover

- Unified DetailPanel component architecture
- Tab system for different views (Inputs, Outputs, Logs, Prompt, LLM Request/Response)
- Prompt tab implementation (shows LLM prompt for selected node)
- Input source selection (test cases, execution history, manual)
- Output display for function results
- Logs panel for execution logs
- LLM request/response inspection
- Node metadata display
- Run/replay controls
- Responsive sizing and positioning

## Key Decisions

- Detail panel shared between standalone and workflow views
- Tab visibility depends on node type (LLM nodes get extra tabs)
- Prompt tab reuses existing PromptView component
- Input source dropdown merges test cases and execution history
- Panel can be positioned right or bottom (user preference)
- Panel state persisted in localStorage

## Source Files to Reference

### From baml-graph (Detail Panel)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/detail-panel/components/DetailPanel.tsx` (lines 1-739 - complete implementation)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/detail-panel/components/LLMNodeContent.tsx` (lines 150-440 - LLM-specific content)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/detail-panel/components/StandardNodeContent.tsx` (lines 443-739 - standard content)

### From playground-common (Prompt Preview Components)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/components/TabularView.tsx` (tabular test results)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/components/ResponseRenderer.tsx` (response display)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/components/EnhancedErrorRenderer.tsx` (error display)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC_ANSWERS.md` (lines 230-280 - Prompt Preview Integration)
- `/Users/aaronvillalpando/Projects/baml/typescript/BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md` (lines 1173-1200 - Detail Panel section)

## Implementation Checklist

- [ ] Create unified `DetailPanel` component
- [ ] Implement tab system with dynamic tab visibility
- [ ] Create `InputsTab` component
- [ ] Create `OutputsTab` component
- [ ] Create `LogsTab` component
- [ ] Create `PromptTab` component (LLM nodes only)
- [ ] Create `LLMRequestTab` component (LLM nodes only)
- [ ] Create `LLMResponseTab` component (LLM nodes only)
- [ ] Extract reusable `PromptView` from existing PromptPreview
- [ ] Implement input source dropdown (test cases + execution history)
- [ ] Add node metadata display (type, status, duration)
- [ ] Add "Run from here" button for workflow nodes
- [ ] Add "Replay" button for execution history
- [ ] Implement panel positioning (right/bottom)
- [ ] Add panel resize controls
- [ ] Add panel collapse/expand
- [ ] Persist panel state in localStorage
- [ ] Style consistently with existing playground UI
- [ ] Add loading states
- [ ] Add error states
- [ ] Test with different node types (function, LLM function)
- [ ] Test in both standalone and workflow contexts

## Validation Criteria

- [ ] Panel opens when node selected in workflow
- [ ] Panel opens for standalone function
- [ ] Tabs display correctly based on node type
- [ ] LLM nodes show Prompt, LLM Request, LLM Response tabs
- [ ] Non-LLM nodes hide LLM-specific tabs
- [ ] Input source dropdown works
- [ ] Test cases and execution history displayed
- [ ] Prompt displays correctly for LLM nodes
- [ ] Run/replay buttons work
- [ ] Panel resizing works
- [ ] Panel positioning (right/bottom) works
- [ ] Panel state persists across reloads
- [ ] Responsive layout works
