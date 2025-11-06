/**
 * UI State Atoms
 *
 * State related to UI components, panels, selection, and user interactions.
 */

import { atom } from 'jotai';
import type { BAMLFile, CodeClickEvent } from '../types';

// ============================================================================
// View Mode Atoms
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
// ============================================================================

/**
 * Selected node ID in the graph
 */
export const selectedNodeIdAtom = atom<string | null>(null);

// ============================================================================
// Panel State Atoms
// ============================================================================

/**
 * Detail panel state
 */
export const detailPanelAtom = atom<{
  isOpen: boolean;
  position: 'bottom' | 'right' | 'floating';
  activeTab: 'io' | 'logs' | 'history';
}>({
  isOpen: false,
  position: 'bottom',
  activeTab: 'io',
});

// ============================================================================
// Layout Atoms
// ============================================================================

/**
 * Graph layout direction
 */
export const layoutDirectionAtom = atom<'horizontal' | 'vertical'>('horizontal');

// ============================================================================
// Input Library Atoms
// ============================================================================

/**
 * Selected input source for a node
 * null = latest execution (default)
 */
export const selectedInputSourceAtom = atom<{
  nodeId: string;
  sourceType: 'execution' | 'test' | 'manual';
  sourceId: string; // executionId, testId, or manualId
} | null>(null);

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
// ============================================================================

/**
 * All BAML files with their functions and tests
 */
export const bamlFilesAtom = atom<BAMLFile[]>([]);

/**
 * Currently active code click event (simulates clicking in a BAML file)
 */
export const activeCodeClickAtom = atom<CodeClickEvent | null>(null);
