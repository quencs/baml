# Phase 8: Graph Components Package

**Timeline:** Week 5-6
**Dependencies:** Phase 7 (Navigation System)
**Risk Level:** Low

## Purpose

Create a separate `@baml/baml-graph-components` package containing all ReactFlow graph components, layout algorithms, and graph-related utilities. This provides a clean separation of concerns and enables reusability across different apps.

## What This Document Will Cover

- Package structure and organization
- ReactFlow setup and configuration
- Custom node types (BaseNode, LLMNode, DiamondNode, HexagonNode, GroupNode)
- Custom edge types
- ELK layout integration for automatic graph layout
- Graph synchronization hooks (`useGraphSync`, `useExecutionSync`)
- Node styling based on execution state
- Graph primitives (nodes, edges, connections)
- Export strategy and public API
- Package dependencies and peer dependencies

## Key Decisions

- Separate package for modularity and reusability
- Uses ReactFlow v12+ (latest stable)
- ELK for automatic layout (hierarchical layout algorithm)
- Export components, hooks, and utilities separately
- Jotai as peer dependency (not bundled)
- React as peer dependency

## Source Files to Reference

### Graph Components
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/graph-primitives/` (entire directory - all node/edge components)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/graph-primitives/nodes/BaseNode/index.tsx` (lines 7-219)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/graph-primitives/nodes/LLMNode/index.tsx` (lines 7-222)

### Layout System
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/graph/layout/` (entire directory - ELK layout)

### Sync Hooks
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/graph/hooks/useGraphSync.ts` (lines 16-86)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/features/execution/hooks/useExecutionSync.ts` (lines 24-78)

### Data Adapters
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/adapter.ts` (graph format conversions)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/data/convert.ts` (data converters)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC_ANSWERS.md` (lines 834-917 - Package Structure recommendation)

## Implementation Checklist

- [ ] Create `packages/baml-graph-components/` directory
- [ ] Initialize package.json with correct exports and dependencies
- [ ] Set up TypeScript configuration
- [ ] Create `src/components/` directory
- [ ] Copy and adapt node components (BaseNode, LLMNode, DiamondNode, etc.)
- [ ] Copy and adapt edge components
- [ ] Create `src/hooks/` directory
- [ ] Implement `useGraphLayout()` hook (ELK integration)
- [ ] Implement `useGraphSync()` hook
- [ ] Implement `useExecutionSync()` hook
- [ ] Create `src/layout/` directory for ELK layout logic
- [ ] Create `src/types.ts` for graph types
- [ ] Create `src/index.ts` with public exports
- [ ] Add ReactFlow and ELK as dependencies
- [ ] Add React and Jotai as peer dependencies
- [ ] Write package README
- [ ] Add unit tests for layout algorithms
- [ ] Add component tests (Storybook or similar)
- [ ] Configure build system (tsup or vite)

## Validation Criteria

- [ ] Package builds successfully
- [ ] Can import components from package
- [ ] ReactFlow graph renders
- [ ] Nodes display correctly with different types
- [ ] Edges render correctly
- [ ] ELK layout produces reasonable graph layouts
- [ ] Node styles update based on execution state
- [ ] useGraphSync correctly converts SDK graph to ReactFlow
- [ ] useExecutionSync updates node styles during execution
- [ ] No runtime errors
- [ ] TypeScript types exported correctly
