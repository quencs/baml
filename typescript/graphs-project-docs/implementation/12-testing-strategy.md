# Phase 12: Testing & Validation

**Timeline:** Week 8
**Dependencies:** All previous phases
**Risk Level:** Low

## Purpose

Comprehensive testing strategy covering unit tests, integration tests, E2E tests, and VSCode extension testing. Ensures all features work correctly in isolation and together, with no regressions from the integration.

## What This Document Will Cover

- Unit testing strategy and coverage goals
- Integration testing approach
- E2E testing with Playwright or Cypress
- VSCode extension testing
- Mock data testing
- Performance testing
- Regression testing
- Test organization and structure
- CI/CD integration
- Coverage reporting

## Key Testing Areas

1. **Unified Atoms** - Ensure state updates work correctly
2. **SDK Methods** - Test all public API methods
3. **Data Providers** - Mock and VSCode providers
4. **Execution Engine** - All execution modes
5. **Navigation Heuristic** - All navigation scenarios
6. **EventListener** - Message handling and SDK integration
7. **Graph Components** - Component rendering and interactions
8. **Workflow View** - View switching and graph display
9. **Detail Panel** - Tab switching and data display
10. **Debug Panel** - Mock mode and click simulation

## Source Files to Reference

### Existing Tests
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/shared/baml-project-panel/playground-panel/prompt-preview/__tests__/highlight-utils.test.ts` (existing unit tests)

### Test Targets (examples from each phase)
- SDK: `packages/playground-common/src/sdk/index.ts`
- Atoms: `packages/playground-common/src/shared/atoms/*.atoms.ts`
- Navigation: `packages/playground-common/src/sdk/navigationHeuristic.ts`
- EventListener: `packages/playground-common/src/baml_wasm_web/EventListener.tsx`
- Components: All UI components

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC.md` (lines 1860-1989 - Testing Strategy section)
- `/Users/aaronvillalpando/Projects/baml/typescript/BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md` (lines 1859-1966 - Testing Strategy)

## Implementation Checklist

### Unit Tests
- [ ] Set up Vitest configuration
- [ ] Test unified atoms (all domain files)
- [ ] Test SDK methods (workflows, executions, graph, cache, tests)
- [ ] Test DataProvider implementations (mock, VSCode)
- [ ] Test ExecutionEngine (all execution modes)
- [ ] Test navigation heuristic (all scenarios)
- [ ] Test cursor enrichment function
- [ ] Test graph layout algorithms
- [ ] Achieve 80%+ code coverage

### Integration Tests
- [ ] Test EventListener → SDK flow
- [ ] Test SDK → Atoms updates
- [ ] Test Atoms → Component rendering
- [ ] Test end-to-end execution flow
- [ ] Test navigation from different sources
- [ ] Test view switching logic
- [ ] Test detail panel integration
- [ ] Test debug panel in mock mode

### E2E Tests (Browser)
- [ ] Set up Playwright or Cypress
- [ ] Test mock mode in standalone playground
- [ ] Test debug panel interactions
- [ ] Test navigation from debug panel
- [ ] Test workflow execution
- [ ] Test graph interactions (node selection, panning, zoom)
- [ ] Test detail panel tabs
- [ ] Test input source selection
- [ ] Test run/replay buttons

### VSCode Extension Tests
- [ ] Set up VSCode extension test environment
- [ ] Test webview initialization
- [ ] Test message passing (VSCode → webview)
- [ ] Test RPC calls (webview → VSCode)
- [ ] Test file synchronization
- [ ] Test cursor updates
- [ ] Test command execution (baml.openBamlPanel, baml.runBamlTest)
- [ ] Test settings updates

### Performance Tests
- [ ] Benchmark atom update performance
- [ ] Benchmark graph rendering with large workflows
- [ ] Benchmark execution with many nodes
- [ ] Test memory usage during long sessions
- [ ] Test bundle size (< 800KB increase)

### Regression Tests
- [ ] Test all existing playground-common features still work
- [ ] Test existing VSCode extension features
- [ ] Test API key management
- [ ] Test test execution (existing behavior)
- [ ] Test prompt preview (standalone mode)

## Test Organization

```
packages/playground-common/src/
├── sdk/
│   ├── __tests__/
│   │   ├── index.test.ts           # SDK unit tests
│   │   ├── execution.test.ts       # Execution engine tests
│   │   └── navigation.test.ts      # Navigation heuristic tests
│   └── providers/
│       └── __tests__/
│           ├── mock.test.ts        # Mock provider tests
│           └── vscode.test.ts      # VSCode provider tests
├── shared/
│   └── atoms/
│       └── __tests__/
│           ├── workflow.atoms.test.ts
│           ├── execution.atoms.test.ts
│           └── runtime.atoms.test.ts
├── baml_wasm_web/
│   └── __tests__/
│       └── EventListener.test.tsx
└── __tests__/
    ├── integration/
    │   ├── execution-flow.test.tsx
    │   ├── navigation-flow.test.tsx
    │   └── view-switching.test.tsx
    └── e2e/
        ├── mock-mode.spec.ts       # Playwright/Cypress
        ├── workflow-execution.spec.ts
        └── graph-interaction.spec.ts

packages/baml-graph-components/src/
└── __tests__/
    ├── components/
    │   ├── WorkflowGraph.test.tsx
    │   └── nodes.test.tsx
    └── hooks/
        ├── useGraphLayout.test.ts
        └── useGraphSync.test.ts

apps/vscode-ext/src/test/
└── suite/
    ├── webview.test.ts
    ├── messaging.test.ts
    └── integration.test.ts
```

## CI/CD Integration

- [ ] Add test scripts to package.json
- [ ] Configure GitHub Actions workflow for tests
- [ ] Add coverage reporting (Codecov or similar)
- [ ] Add bundle size tracking
- [ ] Add performance benchmarking
- [ ] Set up test result reporting
- [ ] Add pre-commit hooks for test running

## Validation Criteria

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] All E2E tests pass
- [ ] VSCode extension tests pass
- [ ] Code coverage > 80%
- [ ] No regressions in existing features
- [ ] Performance benchmarks within acceptable range
- [ ] Bundle size increase < 800KB
- [ ] All test suites run in CI
