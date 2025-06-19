// Main exports for playground-common package

// ===== CONTEXTS =====
export * from './contexts/runtime-context';
export * from './contexts/test-context';

// ===== COMPONENTS =====

// Core Components (5 components)
export * from './components/core/app-root';
export * from './components/core/vscode-handler';
export * from './components/core/runtime-initializer';
export * from './components/core/status-bar';
export * from './components/core/error-count';

// Test Components (9 components)
export * from './components/test/test-panel';
export * from './components/test/test-result-view';
export * from './components/test/test-tabular-view';
export * from './components/test/test-menu';
export * from './components/test/test-status';
export * from './components/test/test-card-view';
export * from './components/test/test-view-selector';
export * from './components/test/simple-card-view';
export * from './components/test/client-graph-view';

// Prompt Components (7 components)
export * from './components/prompt/prompt-preview';
export * from './components/prompt/prompt-preview-content';
export * from './components/prompt/prompt-preview-curl';
export * from './components/prompt/prompt-render-wrapper';
export * from './components/prompt/prompt-stats';
export * from './components/prompt/preview-components';
export * from './components/prompt/preview-toolbar';

// Render Components (7 components)
export * from './components/render/response-renderer';
export * from './components/render/markdown-renderer';
export * from './components/render/parsed-response-render';
export * from './components/render/render-text';
export * from './components/render/render-part';
export * from './components/render/render-prompt';
export * from './components/render/render-tokens';

// UI Components (6 components)
export * from './components/ui/file-viewer';
export * from './components/ui/tree-node';
export * from './components/ui/webview-media';
export * from './components/ui/long-text';
export * from './components/ui/collapsible-message';
export * from './components/ui/function-test-name';

// Legacy Components (3 components)
export * from './components/legacy/jotai-provider';
export * from './components/legacy/jotai-context';
export * from './components/legacy/event-listener-original';

// ===== HOOKS =====
export * from './hooks/use-test-runner';
export * from './hooks/use-vscode';

// ===== SERVICES =====
export * from './services/test-service';

// ===== UTILS =====
export * from './utils/file-utils';
export * from './utils/format-utils';
export * from './utils/media-utils';
export * from './utils/highlight-utils';
export * from './utils/test-state-utils';
export * from './utils/vscode';
export * from './utils/baml-config';

// ===== TYPES =====
export * from './types';

// ===== LEGACY EXPORTS (for backwards compatibility during transition) =====
// Re-export original deeply nested components with deprecation notice
// These should be removed after migration is complete

// Original atoms and legacy components (keeping for compatibility)
export * from './shared/baml-project-panel/atoms';
export * from './shared/baml-project-panel/playground-panel/atoms';
export * from './shared/baml-project-panel/playground-panel/atoms-orch-graph';

// Legacy utilities
export * from './utils/ErrorFallback';

// Other original exports that might still be needed
export * from './wasm';
export * from './Tree';
export * from './lib';
