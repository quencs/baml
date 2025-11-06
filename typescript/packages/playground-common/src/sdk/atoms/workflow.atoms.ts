/**
 * Workflow Domain Atoms
 *
 * State related to workflows, their definitions, and navigation.
 */

import { atom } from 'jotai';
import type { WorkflowDefinition } from '../types';

// ============================================================================
// Workflow Atoms
// ============================================================================

/**
 * All available workflows (all functions in the codebase)
 */
export const workflowsAtom = atom<WorkflowDefinition[]>([]);

/**
 * Currently selected/active workflow
 */
export const activeWorkflowIdAtom = atom<string | null>(null);

/**
 * Derived atom for the active workflow
 */
export const activeWorkflowAtom = atom((get) => {
  const workflows = get(workflowsAtom);
  const activeId = get(activeWorkflowIdAtom);
  return workflows.find((w) => w.id === activeId) ?? null;
});

/**
 * Recent workflows (for quick access)
 */
export const recentWorkflowsAtom = atom<
  Array<{ workflowId: string; lastAccessed: number }>
>([]);
