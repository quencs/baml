/**
 * Execution Simulator for Mock Runtime
 *
 * Simulates workflow execution with realistic behavior including:
 * - Branching (conditional nodes)
 * - Delays based on node type
 * - Cache hits
 * - Errors
 * - Logging
 */

import type { WorkflowDefinition, GraphNode } from '../types';
import type { ExecutionEvent } from './BamlRuntimeInterface';
import type { MockRuntimeConfig } from '../mock-config/types';
import type { LogEntry } from '../types';

/**
 * Simulate workflow execution following the graph structure
 */
export async function* simulateExecution(
  workflow: WorkflowDefinition,
  config: MockRuntimeConfig,
  inputs: Record<string, unknown>,
  executionId: string,
  startFromNodeId?: string
): AsyncGenerator<ExecutionEvent> {
  const visited = new Set<string>();
  let currentNodes = [startFromNodeId || workflow.entryPoint];
  let iterationCount = 0;
  const maxIterations = 20;

  // Context accumulates throughout execution
  const context: Record<string, unknown> = { ...inputs };

  if (startFromNodeId && startFromNodeId !== workflow.entryPoint) {
    console.log(`⏩ Starting execution from node: ${startFromNodeId}`);
  }

  while (currentNodes.length > 0 && iterationCount < maxIterations) {
    iterationCount++;
    const nextNodes: string[] = [];

    for (const nodeId of currentNodes) {
      const node = workflow.nodes.find((n) => n.id === nodeId);
      if (!node) continue;

      // Skip if already visited (unless it's a loop)
      if (visited.has(nodeId) && node.type !== 'loop') {
        continue;
      }

      visited.add(nodeId);

      // Execute node
      const result = yield* executeNode(
        node,
        executionId,
        context,
        workflow,
        config
      );

      if (result.error) {
        // Stop execution on error
        return;
      }

      // Merge outputs into context
      if (result.outputs) {
        Object.assign(context, result.outputs);
      }

      // Determine next nodes based on node type
      const outgoingEdges = workflow.edges.filter((e) => e.source === nodeId);

      if (node.type === 'conditional' && result.outputs?.condition) {
        // Follow the branch that matches the condition
        const chosenEdge = outgoingEdges.find(
          (e) => e.label === result.outputs?.condition
        );
        if (chosenEdge) {
          nextNodes.push(chosenEdge.target);

          if (config.executionBehavior.verboseLogging) {
            yield {
              type: 'node.log',
              nodeId,
              log: createLog(
                executionId,
                'info',
                `Branch: ${result.outputs.condition} → ${chosenEdge.target}`
              ),
            };
          }
        }
      } else if (node.type === 'return') {
        // End execution
        break;
      } else {
        // Follow all outgoing edges
        nextNodes.push(...outgoingEdges.map((e) => e.target));
      }
    }

    currentNodes = nextNodes;

    // Small delay between execution steps
    await delay(100 * config.executionBehavior.speedMultiplier);
  }
}

/**
 * Execute a single node with realistic simulation
 */
async function* executeNode(
  node: GraphNode,
  executionId: string,
  context: Record<string, unknown>,
  workflow: WorkflowDefinition,
  config: MockRuntimeConfig
): AsyncGenerator<
  ExecutionEvent,
  { outputs?: Record<string, unknown>; error?: Error },
  undefined
> {
  // Capture inputs at the start
  const nodeInputs = { ...context };

  // Emit start event
  yield {
    type: 'node.started',
    nodeId: node.id,
    inputs: nodeInputs,
  };

  // Check for cache hit
  const shouldUseCache =
    Math.random() < config.executionBehavior.cacheHitRate;
  if (shouldUseCache) {
    yield {
      type: 'node.cached',
      nodeId: node.id,
      fromExecutionId: `exec_${Date.now() - 60000}_1`,
    };

    const cachedOutputs = generateOutputs(node, workflow, context, config);
    yield {
      type: 'node.completed',
      nodeId: node.id,
      inputs: nodeInputs,
      outputs: cachedOutputs,
      duration: 50, // Cached is fast
    };

    return { outputs: cachedOutputs };
  }

  // Simulate processing with logs
  const duration = getNodeDuration(node.type, config);
  const startTime = Date.now();

  // Generate realistic logs during execution
  const logCount = node.type === 'llm_function' ? 3 : 1;
  for (let i = 0; i < logCount; i++) {
    await delay((duration / logCount) * config.executionBehavior.speedMultiplier);

    if (config.executionBehavior.verboseLogging) {
      yield {
        type: 'node.log',
        nodeId: node.id,
        log: createLog(executionId, 'info', getLogMessage(node, i, logCount)),
      };
    }

    // Emit progress for long-running nodes
    if (node.type === 'llm_function' && i < logCount - 1) {
      yield {
        type: 'node.progress',
        nodeId: node.id,
        progress: ((i + 1) / logCount) * 100,
      };
    }
  }

  // Simulate errors (based on configured error rate)
  const shouldError = Math.random() < config.executionBehavior.errorRate;

  if (shouldError) {
    const error = new Error(getErrorMessage(node));
    yield {
      type: 'node.error',
      nodeId: node.id,
      error,
    };
    return { error };
  }

  // Generate outputs
  const outputs = generateOutputs(node, workflow, context, config);
  const actualDuration = Date.now() - startTime;

  yield {
    type: 'node.completed',
    nodeId: node.id,
    inputs: nodeInputs,
    outputs,
    duration: actualDuration,
  };

  return { outputs };
}

/**
 * Generate mock outputs using the configured output generators
 */
function generateOutputs(
  node: GraphNode,
  workflow: WorkflowDefinition,
  context: Record<string, unknown>,
  config: MockRuntimeConfig
): Record<string, unknown> {
  // Try workflow-specific generator first
  const workflowSpecificKey = `${workflow.id}.${node.id}`;
  const workflowGenerator = config.nodeOutputs[workflowSpecificKey];
  if (workflowGenerator) {
    return workflowGenerator(context, { ...context });
  }

  // Try node-specific generator
  const nodeGenerator = config.nodeOutputs[node.id];
  if (nodeGenerator) {
    return nodeGenerator(context, { ...context });
  }

  // Fallback to generic outputs
  return { completed: true, timestamp: Date.now() };
}

function getNodeDuration(
  nodeType: GraphNode['type'],
  config: MockRuntimeConfig
): number {
  const delayFn = config.executionBehavior.nodeDelays[nodeType];
  if (delayFn) {
    return delayFn();
  }

  // Default delays
  switch (nodeType) {
    case 'llm_function':
      return 1500 + Math.random() * 1000;
    case 'conditional':
      return 300 + Math.random() * 200;
    case 'function':
      return 400 + Math.random() * 300;
    default:
      return 500 + Math.random() * 500;
  }
}

function getLogMessage(
  node: GraphNode,
  step: number,
  totalSteps: number
): string {
  if (node.type === 'llm_function') {
    const messages = [
      `Preparing prompt for ${node.label}`,
      `Calling LLM API (model: gpt-4)`,
      `Received and processing response`,
    ];
    return messages[step] || `Processing ${node.label}...`;
  }

  if (node.type === 'conditional') {
    return `Evaluating condition: ${node.label}`;
  }

  return `Executing ${node.label}`;
}

function getErrorMessage(node: GraphNode): string {
  const errors = [
    `Timeout while executing ${node.label}`,
    `Invalid response from ${node.label}`,
    `Resource not found in ${node.label}`,
    `Rate limit exceeded in ${node.label}`,
  ];
  return errors[Math.floor(Math.random() * errors.length)] ?? '';
}

function createLog(
  executionId: string,
  level: 'debug' | 'info' | 'warn' | 'error',
  message: string
): LogEntry {
  return {
    timestamp: Date.now(),
    level,
    message,
    executionId,
  };
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
