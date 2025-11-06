/**
 * UI State Atoms
 *
 * State related to UI components, panels, selection, and user interactions.
 * Source: apps/baml-graph/src/sdk/atoms/ui.atoms.ts
 */

import { atom } from 'jotai';
import { atomWithStorage } from 'jotai/utils';
import { vscodeLocalStorageStore } from '../baml-project-panel/Jotai';

// Temporary type definitions
export interface BAMLFile {
  path: string;
  functions: any[];
  tests: any[];
}

/**
 * CodeClickEvent - Enriched cursor/click event with semantic information
 * Source: apps/baml-graph/src/sdk/types.ts:288-299
 */
export type CodeClickEvent = {
  type: 'function';
  functionName: string;
  functionType: 'workflow' | 'function' | 'llm_function';
  filePath: string;
} | {
  type: 'test';
  testName: string;
  functionName: string;
  filePath: string;
  nodeType: 'llm_function' | 'function';
};

export type InputSource = any;

// ============================================================================
// View Mode Atoms
// Source: ui.atoms.ts:17-20
// ============================================================================

/**
 * View mode: 'editor' (current code) vs 'execution' (historical snapshot)
 */
export const viewModeAtom = atom<
  | { mode: 'editor' }
  | { mode: 'execution'; executionId: string }
>({ mode: 'editor' });

// ============================================================================
// Selection Atoms
// Source: ui.atoms.ts:27-29
// ============================================================================

/**
 * Selected node ID in the graph
 */
export const selectedNodeIdAtom = atom<string | null>(null);

// ============================================================================
// Panel State Atoms
// Source: ui.atoms.ts:38-46
// ============================================================================

/**
 * Detail panel state
 */
export const detailPanelAtom = atomWithStorage(
  'baml:detailPanel',
  {
    isOpen: false,
    position: 'bottom' as 'bottom' | 'right' | 'floating',
    activeTab: 'io' as 'io' | 'logs' | 'history',
  },
  vscodeLocalStorageStore
);

// ============================================================================
// Layout Atoms
// Source: ui.atoms.ts:55
// ============================================================================

/**
 * Graph layout direction
 */
export const layoutDirectionAtom = atomWithStorage(
  'baml:layoutDirection',
  'horizontal' as 'horizontal' | 'vertical',
  vscodeLocalStorageStore
);

// ============================================================================
// Input Library Atoms (Phase 6)
// Source: ui.atoms.ts:65-79
// ============================================================================

/**
 * Selected input source for a node
 * null = latest execution (default)
 */
export const selectedInputSourceAtom = atom<InputSource | null>(null);

/**
 * Active (editable) inputs for the selected node
 */
export const activeNodeInputsAtom = atom<Record<string, any>>({});

/**
 * Whether the active inputs have been modified
 */
export const inputsDirtyAtom = atom<boolean>(false);

// ============================================================================
// Debug Panel Atoms (for simulating BAML file interactions)
// Source: ui.atoms.ts:88-93
// ============================================================================

/**
 * All BAML files with their functions and tests
 */
export const bamlFilesAtom = atom<BAMLFile[]>([]);

/**
 * Currently active code click event (simulates clicking in a BAML file)
 */
export const activeCodeClickAtom = atom<CodeClickEvent | null>(null);

// ============================================================================
// Code Highlighting Atoms (from playground-common)
// Source: playground-panel/atoms.ts:286-294
// ============================================================================

/**
 * Flash range interface for code highlighting
 */
export interface FlashRange {
  filePath: string;
  startLine: number;
  startCol: number;
  endLine: number;
  endCol: number;
}

/**
 * Flash ranges for code highlighting
 * Source: playground-panel/atoms.ts:294
 */
export const flashRangesAtom = atom<FlashRange[]>([]);
