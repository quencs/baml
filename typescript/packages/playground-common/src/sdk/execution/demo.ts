/**
 * Demo/Example of ExecutionEngine usage
 *
 * This file demonstrates how to use the unified execution engine
 * and can be run to verify the implementation works correctly.
 */

import { createStore } from 'jotai';
import { createBAMLSDK } from '../index';
import { DefaultMockProvider } from '../mock';

/**
 * Demo 1: Execute a workflow
 */
async function demoWorkflowExecution() {
  console.log('\n=== Demo 1: Workflow Execution ===\n');

  // Create SDK with mock provider
  const provider = new DefaultMockProvider();
  const sdk = createBAMLSDK({
    mode: 'mock',
    provider,
  });

  // Initialize SDK
  await sdk.initialize();

  // Get first workflow
  const workflows = sdk.workflows.getAll();
  if (workflows.length === 0) {
    console.error('No workflows found');
    return;
  }

  const workflow = workflows[0]!;
  console.log(`Executing workflow: ${workflow.displayName}`);

  // Execute workflow using unified execute() API
  for await (const event of sdk.execute({
    mode: 'workflow',
    workflowId: workflow.id,
    inputs: { message: 'Hello from demo!' },
  })) {
    // Log events
    if (event.type === 'execution.started') {
      console.log(`✓ Execution started: ${event.executionId}`);
    } else if (event.type === 'node.started') {
      console.log(`  ▶ Node started: ${event.nodeId}`);
    } else if (event.type === 'node.completed') {
      console.log(`  ✓ Node completed: ${event.nodeId} (${event.duration}ms)`);
    } else if (event.type === 'node.cached') {
      console.log(`  💾 Node cached: ${event.nodeId}`);
    } else if (event.type === 'node.error') {
      console.error(`  ✗ Node error: ${event.nodeId}`, event.error.message);
    } else if (event.type === 'execution.completed') {
      console.log(`✓ Execution completed in ${event.duration}ms`);
    } else if (event.type === 'execution.error') {
      console.error(`✗ Execution error:`, event.error.message);
    }
  }

  await sdk.dispose();
}

/**
 * Demo 2: Execute a single test (function-isolated mode)
 */
async function demoTestExecution() {
  console.log('\n=== Demo 2: Test Execution (Function Isolated) ===\n');

  const provider = new DefaultMockProvider();
  const sdk = createBAMLSDK({
    mode: 'mock',
    provider,
  });

  await sdk.initialize();

  // Execute a test using backward-compatible API
  console.log('Running test: fetchData.success_case');

  const result = await sdk.tests.run('fetchData', 'success_case');

  console.log(`Test result:`, {
    executionId: result.executionId,
    status: result.status,
    duration: result.duration,
  });

  await sdk.dispose();
}

/**
 * Demo 3: Execute workflow using backward-compatible API
 */
async function demoBackwardCompatibility() {
  console.log('\n=== Demo 3: Backward Compatibility (executions.start) ===\n');

  const provider = new DefaultMockProvider();
  const sdk = createBAMLSDK({
    mode: 'mock',
    provider,
  });

  await sdk.initialize();

  const workflows = sdk.workflows.getAll();
  if (workflows.length === 0) {
    console.error('No workflows found');
    return;
  }

  const workflow = workflows[0]!;

  // Use old-style executions.start() API
  const executionId = await sdk.executions.start(workflow.id, { test: true });
  console.log(`Started execution via executions.start(): ${executionId}`);

  // Wait a bit for execution to complete
  await new Promise(resolve => setTimeout(resolve, 2000));

  // Get execution results
  const executions = sdk.executions.getExecutions(workflow.id);
  console.log(`Found ${executions.length} executions for workflow`);

  await sdk.dispose();
}

/**
 * Run all demos
 */
async function runDemos() {
  try {
    await demoWorkflowExecution();
    await demoTestExecution();
    await demoBackwardCompatibility();

    console.log('\n=== All demos completed successfully! ===\n');
  } catch (error) {
    console.error('Demo error:', error);
    process.exit(1);
  }
}

// Run demos if executed directly
if (require.main === module) {
  runDemos().catch(console.error);
}

export { demoWorkflowExecution, demoTestExecution, demoBackwardCompatibility };
