/**
 * Centralized Mock Runtime Configuration
 *
 * All mock data in one place, strongly typed
 */

import type {
  WorkflowDefinition,
  TestCaseInput,
  BAMLFile,
} from '../types';
import type { FunctionDefinition } from '../runtime/BamlRuntimeInterface';

/**
 * Node output generator function
 * Takes context and inputs, returns outputs
 */
export type NodeOutputGenerator = (
  context: Record<string, any>,
  inputs: Record<string, any>
) => Record<string, any>;

/**
 * Centralized mock runtime configuration
 */
export interface MockRuntimeConfig {
  // Workflows discovered by the runtime
  workflows: WorkflowDefinition[];

  // Standalone functions (not in workflows)
  functions: FunctionDefinition[];

  // Test cases organized by node ID
  testCases: Record<string, TestCaseInput[]>;

  // Node output generators (one per node or workflow.node)
  nodeOutputs: Record<string, NodeOutputGenerator>;

  // Execution behavior configuration
  executionBehavior: {
    cacheHitRate: number;
    errorRate: number;
    verboseLogging: boolean;
    speedMultiplier: number;
    nodeDelays: Record<string, () => number>;
    conditionalBranches?: Record<string, () => string>;
  };

  // BAML files (for debug panel)
  bamlFiles: BAMLFile[];
}
