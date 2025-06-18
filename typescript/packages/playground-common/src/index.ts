// Main exports for playground-common package

// ===== CONTEXTS =====
export * from './contexts/runtime-context';
export * from './contexts/test-context';

// ===== COMPONENTS =====
export * from './components/app-root';
export * from './components/vscode-handler';
export * from './components/runtime-initializer';
export * from './components/status-bar';
export * from './components/error-count';
export * from './components/test-panel';
export * from './components/test-result-view';
export * from './components/test-tabular-view';
export * from './components/test-menu';
export * from './components/test-status';
export * from './components/test-card-view';
export * from './components/response-renderer';

// ===== HOOKS =====
export * from './hooks/use-test-runner';
export * from './hooks/use-vscode';

// ===== SERVICES =====
export * from './services/test-service';

// ===== UTILS =====
export * from './utils/file-utils';
export * from './utils/format-utils';

// ===== TYPES =====
export * from './types';

// ===== LEGACY EXPORTS (for backwards compatibility during transition) =====
// Re-export original deeply nested components with deprecation notice
// These should be removed after migration is complete

// Original atoms and legacy components
export * from './shared/baml-project-panel/atoms';
export * from './shared/baml-project-panel/playground-panel/atoms';
export * from './shared/baml-project-panel/playground-panel/atoms-orch-graph';
export * from './baml_wasm_web/JotaiProvider';
export * from './baml_wasm_web/bamlConfig';

// Legacy utilities
export * from './utils/ErrorFallback';

// Other original exports
export * from './wasm';
export * from './Tree';
export * from './lib';
