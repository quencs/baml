/**
 * Storage interface - abstracts state management
 *
 * This interface allows the SDK to be storage-agnostic.
 * Implementations can use Jotai, Redux, Zustand, or any other state management solution.
 */

import type {
  WorkflowDefinition,
  ExecutionSnapshot,
  NodeExecutionState,
  NodeExecution,
  CacheEntry,
} from '../types';

import type {
  DiagnosticError,
  GeneratedFile,
  WasmPanicState,
  VSCodeSettings,
} from '../atoms/core.atoms';

/**
 * Storage interface for SDK state management
 */
export interface SDKStorage {
  // ============================================================================
  // Workflows
  // ============================================================================

  setWorkflows(workflows: WorkflowDefinition[]): void;
  getWorkflows(): WorkflowDefinition[];
  setActiveWorkflowId(id: string | null): void;
  getActiveWorkflowId(): string | null;

  // ============================================================================
  // Executions
  // ============================================================================

  addExecution(workflowId: string, execution: ExecutionSnapshot): void;
  getExecutions(workflowId: string): ExecutionSnapshot[];
  updateExecution(executionId: string, updates: Partial<ExecutionSnapshot>): void;

  // ============================================================================
  // Node States
  // ============================================================================

  setNodeState(nodeId: string, state: NodeExecutionState): void;
  getNodeState(nodeId: string): NodeExecutionState;
  clearAllNodeStates(): void;

  // ============================================================================
  // Node Executions (I/O data)
  // ============================================================================

  addNodeExecution(executionId: string, nodeId: string, data: NodeExecution): void;
  getNodeExecution(executionId: string, nodeId: string): NodeExecution | null;

  // ============================================================================
  // Cache
  // ============================================================================

  setCacheEntry(entry: CacheEntry): void;
  getCacheEntry(nodeId: string, inputsHash: string): CacheEntry | null;
  clearCache(scope?: { workflowId?: string; nodeId?: string }): void;

  // ============================================================================
  // Diagnostics
  // ============================================================================

  setDiagnostics(diagnostics: DiagnosticError[]): void;
  getDiagnostics(): DiagnosticError[];
  setLastValidRuntime(valid: boolean): void;
  getLastValidRuntime(): boolean;

  // ============================================================================
  // Generated Files
  // ============================================================================

  setGeneratedFiles(files: GeneratedFile[]): void;
  getGeneratedFiles(): GeneratedFile[];

  // ============================================================================
  // WASM Panic
  // ============================================================================

  setWasmPanic(panic: WasmPanicState | null): void;
  getWasmPanic(): WasmPanicState | null;

  // ============================================================================
  // Feature Flags
  // ============================================================================

  setFeatureFlags(flags: string[]): void;
  getFeatureFlags(): string[];

  // ============================================================================
  // Environment Variables
  // ============================================================================

  setEnvVars(envVars: Record<string, string>): void;
  getEnvVars(): Record<string, string>;

  // ============================================================================
  // Files Tracking
  // ============================================================================

  setBAMLFiles(files: Record<string, string>): void;
  getBAMLFiles(): Record<string, string>;
  setSandboxFiles(files: Record<string, string>): void;
  getSandboxFiles(): Record<string, string>;

  // ============================================================================
  // VSCode Integration
  // ============================================================================

  setVSCodeSettings(settings: VSCodeSettings | null): void;
  getVSCodeSettings(): VSCodeSettings | null;
  setPlaygroundPort(port: number): void;
  getPlaygroundPort(): number;
}
