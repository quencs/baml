# Phase 4: Unified Execution Engine

**Timeline:** Week 3-4
**Dependencies:** Phase 3 (Data Providers)
**Risk Level:** High

## Purpose

Create a unified execution system that handles both test execution (single function) and workflow execution (multiple nodes). This replaces the separate `runTests` and `executeWorkflow` implementations with a single, flexible `execute()` method.

## What This Document Will Cover

- Unified `execute()` API design and semantics
- Three execution modes: isolated function, function-in-workflow, full workflow
- ExecutionEngine class implementation
- Graph traversal algorithms (topological sort, dependency resolution)
- Node execution with state management
- Input resolution (from test cases, previous nodes, user input)
- Error handling and recovery strategies
- Abort/cancellation support
- Watch notification collection
- Cache integration
- Progress tracking and event emission
- Backward compatibility wrappers (`runTest()`, `runWorkflow()`)

## Key Decisions

- Single `sdk.execute(options)` method with typed options
- ExecutionEngine as internal implementation detail
- Graph traversal happens in ExecutionEngine
- Node state updates happen incrementally (not batched)
- Support for partial workflow execution (start from node)
- Abort via AbortController
- Events emitted at each execution milestone

## Source Files to Reference

### From baml-graph (Workflow Execution)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/index.ts` (lines 120-179 - execution start)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/index.ts` (lines 355-495 - mock execution simulation)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/mock.ts` (lines 570-811 - graph traversal)

### From playground-common (Test Execution)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/test-runner.ts` (lines 34-305 - useRunTests hook)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/test-runner.ts` (lines 307-536 - parallel test execution)
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/test-panel/test-runner.ts` (lines 538-629 - main test runner)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC_ANSWERS.md` (lines 413-832 - Question 7: Unifying runTests vs run workflow)
- `/Users/aaronvillalpando/Projects/baml/typescript/BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md` (lines 100-150 - SDK execution methods)

## Implementation Checklist

- [ ] Design `ExecutionOptions` type with all variants
- [ ] Design `ExecutionResult` type
- [ ] Create `sdk/execution/engine.ts` with `ExecutionEngine` class
- [ ] Implement `executeFunction()` method
- [ ] Implement `executeWorkflow()` method with graph traversal
- [ ] Implement topological sort for workflow nodes
- [ ] Add input resolution logic (test cases, previous nodes, manual)
- [ ] Add state tracking (node states, execution snapshots)
- [ ] Implement abort/cancellation
- [ ] Add watch notification collection
- [ ] Integrate cache lookup and storage
- [ ] Add event emission at each step
- [ ] Create `sdk.execute()` wrapper in BAMLSDK
- [ ] Add backward compatibility helpers (`sdk.runTest()`, `sdk.runWorkflow()`)
- [ ] Write unit tests for ExecutionEngine
- [ ] Write integration tests for different execution modes

## Validation Criteria

- [ ] Single function execution works (isolated)
- [ ] Single function execution works (in workflow context)
- [ ] Full workflow execution works
- [ ] Partial workflow execution works (start from node)
- [ ] Node states update correctly during execution
- [ ] Execution can be cancelled
- [ ] Cache hits/misses work correctly
- [ ] Watch notifications collected
- [ ] Events emitted at correct times
- [ ] Backward compatibility methods work
