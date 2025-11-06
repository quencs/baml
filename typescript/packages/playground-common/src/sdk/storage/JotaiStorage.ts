/**
 * Jotai-based storage implementation
 *
 * Implements SDKStorage interface using Jotai atoms
 */

import type { createStore } from 'jotai';
import type { SDKStorage } from './SDKStorage';
import type {
  WorkflowDefinition,
  ExecutionSnapshot,
  NodeExecutionState,
  NodeExecution,
  CacheEntry,
} from '../types';

// Import atoms directly from core.atoms.ts (no barrel exports)
import {
  workflowsAtom,
  activeWorkflowIdAtom,
  workflowExecutionsAtomFamily,
  nodeStateAtomFamily,
  registerNodeAtom,
  cacheAtom,
  getCacheKey,
  diagnosticsAtom,
  lastValidRuntimeAtom,
  generatedFilesAtom,
  wasmPanicAtom,
  featureFlagsAtom,
  envVarsAtom,
  bamlFilesTrackedAtom,
  sandboxFilesTrackedAtom,
  vscodeSettingsAtom,
  playgroundPortAtom,
} from '../atoms/core.atoms';

import type {
  DiagnosticError,
  GeneratedFile,
  WasmPanicState,
  VSCodeSettings,
} from '../atoms/core.atoms';

export class JotaiStorage implements SDKStorage {
  constructor(private store: ReturnType<typeof createStore>) {}

  // ============================================================================
  // Workflows
  // ============================================================================

  setWorkflows(workflows: WorkflowDefinition[]) {
    this.store.set(workflowsAtom, workflows);
  }

  getWorkflows() {
    return this.store.get(workflowsAtom);
  }

  setActiveWorkflowId(id: string | null) {
    this.store.set(activeWorkflowIdAtom, id);
  }

  getActiveWorkflowId() {
    return this.store.get(activeWorkflowIdAtom);
  }

  // ============================================================================
  // Executions
  // ============================================================================

  addExecution(workflowId: string, execution: ExecutionSnapshot) {
    const executionsAtom = workflowExecutionsAtomFamily(workflowId);
    const executions = this.store.get(executionsAtom);
    this.store.set(executionsAtom, [execution, ...executions]);
  }

  getExecutions(workflowId: string) {
    const executionsAtom = workflowExecutionsAtomFamily(workflowId);
    return this.store.get(executionsAtom);
  }

  updateExecution(executionId: string, updates: Partial<ExecutionSnapshot>) {
    // Find execution across all workflows
    const workflows = this.store.get(workflowsAtom);

    for (const workflow of workflows) {
      const executionsAtom = workflowExecutionsAtomFamily(workflow.id);
      const executions = this.store.get(executionsAtom);
      const index = executions.findIndex((e) => e.id === executionId);

      if (index !== -1) {
        const updated = [...executions];
        updated[index] = { ...updated[index]!, ...updates };
        this.store.set(executionsAtom, updated);
        break;
      }
    }
  }

  // ============================================================================
  // Node States
  // ============================================================================

  setNodeState(nodeId: string, state: NodeExecutionState) {
    // Register node first (ensures atom exists)
    this.store.set(registerNodeAtom, nodeId);
    // Set state
    this.store.set(nodeStateAtomFamily(nodeId), state);
  }

  getNodeState(nodeId: string) {
    return this.store.get(nodeStateAtomFamily(nodeId));
  }

  clearAllNodeStates() {
    // Note: We can't iterate over all atom family instances
    // So we rely on the SDK to track which nodes were used
    // For now, this is a no-op since we can't access all instances
    // The atoms will naturally reset when new execution starts
  }

  // ============================================================================
  // Node Executions (I/O data)
  // ============================================================================

  addNodeExecution(executionId: string, nodeId: string, data: NodeExecution) {
    // Find execution and update its nodeExecutions map
    const workflows = this.store.get(workflowsAtom);

    for (const workflow of workflows) {
      const executionsAtom = workflowExecutionsAtomFamily(workflow.id);
      const executions = this.store.get(executionsAtom);
      const execution = executions.find((e) => e.id === executionId);

      if (execution) {
        execution.nodeExecutions.set(nodeId, data);
        // Trigger reactivity by setting new array
        this.store.set(executionsAtom, [...executions]);
        break;
      }
    }
  }

  getNodeExecution(executionId: string, nodeId: string) {
    const workflows = this.store.get(workflowsAtom);

    for (const workflow of workflows) {
      const executionsAtom = workflowExecutionsAtomFamily(workflow.id);
      const executions = this.store.get(executionsAtom);
      const execution = executions.find((e) => e.id === executionId);

      if (execution) {
        return execution.nodeExecutions.get(nodeId) || null;
      }
    }
    return null;
  }

  // ============================================================================
  // Cache
  // ============================================================================

  setCacheEntry(entry: CacheEntry) {
    const cache = this.store.get(cacheAtom);
    const key = getCacheKey(entry.nodeId, entry.inputsHash);
    cache.set(key, entry);
    this.store.set(cacheAtom, new Map(cache));
  }

  getCacheEntry(nodeId: string, inputsHash: string) {
    const cache = this.store.get(cacheAtom);
    const key = getCacheKey(nodeId, inputsHash);
    return cache.get(key) || null;
  }

  clearCache(scope?: { workflowId?: string; nodeId?: string }) {
    if (!scope) {
      this.store.set(cacheAtom, new Map());
    } else {
      // For now, just clear all
      // TODO: Implement scoped cache clearing
      this.store.set(cacheAtom, new Map());
    }
  }

  // ============================================================================
  // Diagnostics
  // ============================================================================

  setDiagnostics(diagnostics: DiagnosticError[]) {
    this.store.set(diagnosticsAtom, diagnostics);
  }

  getDiagnostics() {
    return this.store.get(diagnosticsAtom);
  }

  setLastValidRuntime(valid: boolean) {
    this.store.set(lastValidRuntimeAtom, valid);
  }

  getLastValidRuntime() {
    return this.store.get(lastValidRuntimeAtom);
  }

  // ============================================================================
  // Generated Files
  // ============================================================================

  setGeneratedFiles(files: GeneratedFile[]) {
    this.store.set(generatedFilesAtom, files);
  }

  getGeneratedFiles() {
    return this.store.get(generatedFilesAtom);
  }

  // ============================================================================
  // WASM Panic
  // ============================================================================

  setWasmPanic(panic: WasmPanicState | null) {
    this.store.set(wasmPanicAtom, panic);
  }

  getWasmPanic() {
    return this.store.get(wasmPanicAtom);
  }

  // ============================================================================
  // Feature Flags
  // ============================================================================

  setFeatureFlags(flags: string[]) {
    this.store.set(featureFlagsAtom, flags);
  }

  getFeatureFlags() {
    return this.store.get(featureFlagsAtom);
  }

  // ============================================================================
  // Environment Variables
  // ============================================================================

  setEnvVars(envVars: Record<string, string>) {
    this.store.set(envVarsAtom, envVars);
  }

  getEnvVars() {
    return this.store.get(envVarsAtom);
  }

  // ============================================================================
  // Files Tracking
  // ============================================================================

  setBAMLFiles(files: Record<string, string>) {
    this.store.set(bamlFilesTrackedAtom, files);
  }

  getBAMLFiles() {
    return this.store.get(bamlFilesTrackedAtom);
  }

  setSandboxFiles(files: Record<string, string>) {
    this.store.set(sandboxFilesTrackedAtom, files);
  }

  getSandboxFiles() {
    return this.store.get(sandboxFilesTrackedAtom);
  }

  // ============================================================================
  // VSCode Integration
  // ============================================================================

  setVSCodeSettings(settings: VSCodeSettings | null) {
    this.store.set(vscodeSettingsAtom, settings);
  }

  getVSCodeSettings() {
    return this.store.get(vscodeSettingsAtom);
  }

  setPlaygroundPort(port: number) {
    this.store.set(playgroundPortAtom, port);
  }

  getPlaygroundPort() {
    return this.store.get(playgroundPortAtom);
  }
}
