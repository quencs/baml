// Central atoms file for playground-common
// Re-exports main atoms during refactor transition

// Re-export main atoms from shared folder
export * from './shared/baml-project-panel/atoms';
export * from './shared/baml-project-panel/playground-panel/atoms';
export * from './shared/baml-project-panel/playground-panel/atoms-orch-graph';

// Create a codemirror diagnostics atom placeholder
// This should be moved to a proper location later
import { atom } from 'jotai';

export const CodeMirrorDiagnosticsAtom = atom<any[]>([]);