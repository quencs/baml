/**
 * Execution Engine Types
 *
 * Defines types for the unified execution system that handles:
 * 1. Isolated function execution (test mode)
 * 2. Function-in-workflow execution (single node with context)
 * 3. Full workflow execution (entire graph)
 */

import type { BAMLEvent, CachePolicy } from '../types';

/**
 * Unified execution options - discriminated union for three execution modes
 */
export type ExecutionOptions =
  // Mode 1: Execute single function in isolation (test mode)
  | {
      mode: 'function-isolated';
      functionName: string;
      testName?: string; // Optional: use specific test case
      inputs?: Record<string, unknown>; // Optional: override test inputs
      cachePolicy?: CachePolicy;
    }
  // Mode 2: Execute single function within workflow context
  | {
      mode: 'function-in-workflow';
      workflowId: string;
      nodeId: string; // Which function to execute
      inputs?: Record<string, unknown>; // Optional: override inputs
      cachePolicy?: CachePolicy;
    }
  // Mode 3: Execute full workflow
  | {
      mode: 'workflow';
      workflowId: string;
      inputs: Record<string, unknown>; // Workflow entry inputs
      startFromNodeId?: string; // Optional: partial execution
      cachePolicy?: CachePolicy;
      clearCache?: boolean;
    };

/**
 * Result of execution
 */
export interface ExecutionResult {
  executionId: string;
  status: 'success' | 'error' | 'cancelled';
  duration: number;

  // For function-isolated and function-in-workflow
  outputs?: Record<string, unknown>;
  error?: Error;

  // For workflow
  nodeResults?: Map<string, NodeExecutionResult>;

  // Watch notifications (for all modes)
  watchNotifications?: WatchNotification[];

  // Cache statistics
  cacheStats?: {
    hits: number;
    misses: number;
  };
}

export interface NodeExecutionResult {
  nodeId: string;
  status: 'success' | 'error' | 'skipped' | 'cached';
  inputs: Record<string, unknown>;
  outputs?: Record<string, unknown>;
  error?: Error;
  duration: number;
  cached?: boolean;
}

/**
 * Watch notification from BAML runtime
 */
export interface WatchNotification {
  key: string;
  value: string;
  block_name?: string;
}

/**
 * Execution event - superset of BAMLEvent with additional execution-specific events
 */
export type ExecutionEvent = BAMLEvent;

/**
 * Input resolution parameters
 */
export interface InputResolutionParams {
  functionName: string;
  testName?: string;
  manualInputs?: Record<string, unknown>;
  context?: Record<string, unknown>; // For workflow context
  functionSignature?: FunctionSignature;
}

/**
 * Function signature for input mapping
 */
export interface FunctionSignature {
  parameters: Array<{
    name: string;
    type: string;
    optional: boolean;
    defaultValue?: unknown;
  }>;
  returnType: string;
}
