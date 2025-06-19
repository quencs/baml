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

// UI Components (8 components)
export * from './components/ui/file-viewer';
export * from './components/ui/tree-node';
export * from './components/ui/webview-media';
export * from './components/ui/long-text';
export * from './components/ui/collapsible-message';
export * from './components/ui/function-test-name';
export * from './components/ui/theme-provider';
export * from './components/ui/theme-toggle';
export * from './components/ui/code-mirror-viewer';

// Legacy Components (3 components)
export * from './components/legacy/jotai-provider';
export * from './components/legacy/jotai-context';
export * from './components/legacy/event-listener-original';

// ===== HOOKS =====
export * from './hooks/use-test-runner';
export * from './hooks/use-vscode';
export * from './hooks/use-debounced-atom';

// ===== SERVICES =====
export * from './services/test-service';
export { vscode, VSCodeAPIWrapper } from './services/vscode-service';

// ===== LIB =====
export * from './lib/vscode-rpc';

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

// ===== ATOMS =====
// Export only specific atoms to avoid conflicts
export {
  wasmAtom,
  filesAtom,
  projectAtom,
  runtimeAtom,
  diagnosticsAtom,
  selectedFunctionAtom,
  selectedTestcaseAtom,
  flashRangesAtom,
  updateCursorAtom,
  runningTestsAtom,
  CodeMirrorDiagnosticsAtom
} from './atoms';

// ===== OTHER =====
export * from './wasm';
export * from './Tree';
