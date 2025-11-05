# Phase 5: EventListener Refactor

**Timeline:** Week 4
**Dependencies:** Phase 4 (Execution Engine)
**Risk Level:** Medium

## Purpose

Transform EventListener from directly updating atoms to calling SDK methods. This makes EventListener a thin adapter layer that translates IDE messages into SDK API calls, preserving platform-specific quirks while delegating business logic to the SDK.

## What This Document Will Cover

- New EventListener architecture (thin adapter pattern)
- Message type handling (ide_message vs lsp_message)
- Translation from messages to SDK method calls
- Platform-specific handling (JetBrains delays, Zed quirks)
- Error handling and recovery
- WebSocket integration for non-VSCode IDEs
- Backward compatibility during migration
- Testing strategy for EventListener

## Key Decisions

- EventListener receives `sdk` as prop or via context
- Each message type maps to specific SDK method
- Platform quirks stay in EventListener (e.g., JetBrains 1s delay)
- EventListener remains a React component (for useEffect)
- No business logic in EventListener (only message parsing and routing)

## Source Files to Reference

### Current EventListener
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/baml_wasm_web/EventListener.tsx` (lines 57-217 - current implementation)

### Message Types
- `/Users/aaronvillalpando/Projects/baml/typescript/packages/playground-common/src/baml_wasm_web/vscode-to-webview-rpc.ts` (lines 4-88 - message type definitions)
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/vscode-ext/src/panels/vscode-to-webview-rpc.ts` (lines 4-73 - extension side types)

### SDK Methods to Call
- `/Users/aaronvillalpando/Projects/baml/typescript/apps/baml-graph/src/sdk/index.ts` (lines 74-349 - SDK API surface)

### Design References
- `/Users/aaronvillalpando/Projects/baml/typescript/MERGE_DESIGN_DOC.md` (lines 1548-1605 - Modified EventListener section)
- `/Users/aaronvillalpando/Projects/baml/typescript/BAML_GRAPHS_ARCHITECTURE_INTEGRATION_PLAN.md` (lines 882-950 - EventListener as thin bridge)

## Implementation Checklist

- [ ] Update EventListener to accept SDK via prop or useBAMLSDK()
- [ ] Map `update_cursor` → `sdk.navigation.updateCursor()`
- [ ] Map `runtime_updated` → `sdk.files.update()`
- [ ] Map `baml_settings_updated` → `sdk.settings.update()`
- [ ] Map `baml.openBamlPanel` → `sdk.navigation.selectFunction()`
- [ ] Map `baml.runBamlTest` → `sdk.tests.run()`
- [ ] Map `textDocument/codeAction` → `sdk.navigation.updateCursorFromRange()`
- [ ] Preserve JetBrains 1s delay workaround
- [ ] Preserve file update debouncing (50ms)
- [ ] Add error handling for SDK method failures
- [ ] Remove direct atom updates (keep only via SDK)
- [ ] Update EventListener tests to mock SDK
- [ ] Document message → SDK method mapping

## Validation Criteria

- [ ] VSCode cursor updates trigger SDK navigation
- [ ] File updates from LSP trigger WASM compilation
- [ ] Test execution commands work
- [ ] Settings updates work
- [ ] Platform-specific delays preserved
- [ ] No direct atom access in EventListener
- [ ] All messages handled correctly
- [ ] Error handling works
