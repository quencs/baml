# BAML Graphs Integration Status

**Date:** 2025-11-05
**Phase:** 8 - Graph Components Package
**Status:** In Progress

## ✅ Completed

### 1. File Migration
- ✅ Copied baml-graph SDK to `src/sdk/`
- ✅ Copied baml-graph atoms to `src/sdk/atoms/`
- ✅ Copied graph-primitives to `src/graph-primitives/`
- ✅ Copied all features (graph, workflow, detail-panel, debug, navigation, execution, llm)
- ✅ Copied supporting files (states, data, utils)
- ✅ Copied WorkflowApp.tsx (baml-graph's App.tsx)
- ✅ Copied workflow-styles.css (baml-graph's index.css)

### 2. Dependencies
- ✅ Added elkjs@^0.11.0
- ✅ Added @radix-ui/react-dropdown-menu
- ✅ Added @radix-ui/react-select
- ✅ Added @del-wang/utils
- ✅ Created lib/utils.ts with `cn()` function

### 3. Component Integration
- ✅ Updated `LLMOnlyPanel` to use playground-common's `PromptPreview`
- ✅ Updated `LLMTestPanel` to use playground-common's `TestPanel`
- ✅ Fixed UI component imports to use `@baml/ui` instead of local copies
- ✅ Removed copied UI components (dropdown-menu, resizable, select, spinner)

### 4. Import Path Fixes
- ✅ Fixed `@/lib/utils` → relative paths
- ✅ Fixed `@/sdk/*` → relative paths
- ✅ Fixed `@/states/reactflow` → relative paths
- ✅ Fixed `@/features/*` → relative paths
- ✅ Fixed `@/components/ui/*` → `@baml/ui/*`

## 🚧 In Progress / Remaining Work

### Type Errors (214 total)
Main categories:
1. **36 errors**: `Object is possibly 'undefined'` - Need null checks/assertions
2. **17 errors**: Test files (expect, it, describe) - Need test type definitions
3. **13 errors**: `ControlPoint | undefined` type mismatches - Need type guards
4. **11 errors**: Cannot find module `@/data/types` - More @ imports to fix
5. **9 errors**: SDK config mismatch - `provider` property doesn't exist
6. **5 errors**: Duplicate object properties
7. **4 errors**: SDK missing methods (navigation, tests, files)

### SDK Integration
The copied SDK from baml-graph has some differences from playground-common's existing SDK work:
- ❌ Missing `sdk.navigation` namespace
- ❌ Missing `sdk.tests` namespace
- ❌ Missing `sdk.files` property
- ⚠️ BAMLSDKConfig incompatibility (provider field)

### Remaining @ Import Issues
- `@/data/types` - 11 occurrences
- `@/states/reactflow` - 5 occurrences
- `@/sdk/hooks` - 3 occurrences
- `@/features/graph/layout/edge/point` - 3 occurrences
- `@/features/graph/layout/edge/edge` - 3 occurrences

## 📋 Next Steps

### Priority 1: Fix Remaining @ Imports
```bash
find src -name "*.ts" -o -name "*.tsx" | xargs grep -l "@/" | wc -l
```
Systematically replace all remaining @ aliases with relative paths.

### Priority 2: Reconcile SDK Implementations
Two SDKs exist:
1. `src/sdk/` (from baml-graph) - Has workflow, execution, graph, cache, navigation
2. Existing playground-common SDK work - Has different structure

**Options:**
- Merge the two, taking best from each
- Add missing methods to baml-graph SDK
- Create adapters

### Priority 3: Type Safety Fixes
- Add null checks for possibly undefined objects
- Add type guards for ControlPoint usage
- Fix duplicate object properties
- Add test type definitions (vitest or jest)

### Priority 4: Testing
- Test WorkflowApp renders without errors
- Test LLM-only mode shows PromptPreview correctly
- Test graph mode shows workflow graph
- Test navigation between views

## 📁 Directory Structure (Current)

```
packages/playground-common/src/
├── baml_wasm_web/         # EventListener (existing)
├── components/            # API keys, status bar (existing)
├── features/              # NEW: Copied from baml-graph
│   ├── debug/
│   ├── detail-panel/
│   ├── execution/
│   ├── graph/
│   ├── llm/               # Updated to use PromptPreview/TestPanel
│   ├── navigation/
│   └── workflow/
├── graph-primitives/      # NEW: ReactFlow nodes and edges
│   ├── edges/
│   └── nodes/
├── lib/                   # NEW: utils.ts with cn()
├── sdk/                   # NEW: Copied from baml-graph
│   ├── atoms/
│   ├── execution/         # Existing work from previous phases
│   ├── providers/         # Existing work from previous phases
│   └── utils/             # Existing work from previous phases
├── shared/                # Existing playground-common structure
│   ├── atoms/             # NEW: Graph atoms copied here
│   ├── baml-project-panel/
│   └── components/
├── states/                # NEW: reactflow store
├── data/                  # NEW: Graph data converters
├── utils/                 # NEW: Misc utilities
├── WorkflowApp.tsx        # NEW: Main graph/workflow app
└── workflow-styles.css    # NEW: Styles for workflow view

```

## 🎯 Success Criteria

- [ ] TypeScript compiles with no errors
- [ ] WorkflowApp renders in browser
- [ ] LLM-only mode shows PromptPreview correctly
- [ ] Workflow graph mode displays correctly
- [ ] Navigation between modes works
- [ ] Detail panel shows node/function details
- [ ] Test execution works in both modes

## 📝 Notes

### Architecture Decision
Per design docs, we're integrating baml-graph INTO playground-common, not creating a separate package. The existing PromptPreview/TestPanel components are used when in LLM-only mode.

### Key Integration Points
1. `LLMOnlyPanel` → `PromptPreview` (line 10 in LLMOnlyPanel.tsx)
2. `LLMTestPanel` → `TestPanel` (line 10 in LLMTestPanel.tsx)
3. `WorkflowApp` uses both graph view and LLM-only view based on `isLLMOnlyModeAtom`

### Dependencies Alignment
- Using @baml/ui for all UI components (not copying them)
- Using @xyflow/react@12.7.0 (already in playground-common)
- Using elkjs for graph layout
- Using @del-wang/utils for utility functions
