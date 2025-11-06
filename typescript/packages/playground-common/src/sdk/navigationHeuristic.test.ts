/**
 * Unit tests for navigation heuristic
 *
 * Tests all scenarios described in navigationHeuristic.ts documentation
 */

import { describe, it, expect } from 'vitest';
import { determineNavigationAction, type NavigationState } from './navigationHeuristic';
import type { CodeClickEvent, WorkflowDefinition, BAMLFile } from './types';

// ============================================================================
// Test Data Setup
// ============================================================================

const mockWorkflows: WorkflowDefinition[] = [
  {
    id: 'simpleWorkflow',
    displayName: 'Simple Workflow',
    filePath: 'workflows/simple.baml',
    startLine: 1,
    endLine: 100,
    codeHash: 'hash1',
    entryPoint: 'simpleWorkflow',
    parameters: [],
    returnType: 'any',
    childFunctions: ['fetchData', 'processData', 'saveResult'],
    lastModified: Date.now(),
    nodes: [
      { id: 'simpleWorkflow', type: 'function', label: 'Start', position: { x: 0, y: 0 }, codeHash: 'hash1', lastModified: Date.now() },
      { id: 'fetchData', type: 'function', label: 'Fetch Data', position: { x: 0, y: 100 }, codeHash: 'hash1', lastModified: Date.now() },
      { id: 'processData', type: 'llm_function', label: 'Process Data', position: { x: 0, y: 200 }, codeHash: 'hash1', lastModified: Date.now() },
      { id: 'saveResult', type: 'function', label: 'Save Result', position: { x: 0, y: 300 }, codeHash: 'hash1', lastModified: Date.now() },
    ],
    edges: [
      { id: 'e1', source: 'simpleWorkflow', target: 'fetchData' },
      { id: 'e2', source: 'fetchData', target: 'processData' },
      { id: 'e3', source: 'processData', target: 'saveResult' },
    ],
  },
  {
    id: 'conditionalWorkflow',
    displayName: 'Conditional Workflow',
    filePath: 'workflows/conditional.baml',
    startLine: 1,
    endLine: 100,
    codeHash: 'hash2',
    entryPoint: 'conditionalWorkflow',
    parameters: [],
    returnType: 'any',
    childFunctions: ['validateInput', 'handleSuccess', 'handleFailure'],
    lastModified: Date.now(),
    nodes: [
      { id: 'conditionalWorkflow', type: 'function', label: 'Start', position: { x: 0, y: 0 }, codeHash: 'hash2', lastModified: Date.now() },
      { id: 'validateInput', type: 'function', label: 'Validate', position: { x: 0, y: 100 }, codeHash: 'hash2', lastModified: Date.now() },
      { id: 'handleSuccess', type: 'llm_function', label: 'Success Handler', position: { x: -100, y: 200 }, codeHash: 'hash2', lastModified: Date.now() },
      { id: 'handleFailure', type: 'function', label: 'Failure Handler', position: { x: 100, y: 200 }, codeHash: 'hash2', lastModified: Date.now() },
    ],
    edges: [
      { id: 'e1', source: 'conditionalWorkflow', target: 'validateInput' },
      { id: 'e2', source: 'validateInput', target: 'handleSuccess', label: 'success' },
      { id: 'e3', source: 'validateInput', target: 'handleFailure', label: 'failure' },
    ],
  },
];

const mockBAMLFiles: BAMLFile[] = [
  {
    path: 'workflows/simple.baml',
    functions: [
      { name: 'simpleWorkflow', type: 'workflow', filePath: 'workflows/simple.baml' },
      { name: 'fetchData', type: 'function', filePath: 'workflows/simple.baml' },
      { name: 'processData', type: 'llm_function', filePath: 'workflows/simple.baml' },
      { name: 'saveResult', type: 'function', filePath: 'workflows/simple.baml' },
    ],
    tests: [
      { name: 'test_simple_success', functionName: 'simpleWorkflow', filePath: 'workflows/simple.baml', nodeType: 'function' },
      { name: 'test_simple_with_invalid_data', functionName: 'simpleWorkflow', filePath: 'workflows/simple.baml', nodeType: 'function' },
    ],
  },
  {
    path: 'workflows/conditional.baml',
    functions: [
      { name: 'conditionalWorkflow', type: 'workflow', filePath: 'workflows/conditional.baml' },
      { name: 'validateInput', type: 'function', filePath: 'workflows/conditional.baml' },
      { name: 'handleSuccess', type: 'llm_function', filePath: 'workflows/conditional.baml' },
      { name: 'handleFailure', type: 'function', filePath: 'workflows/conditional.baml' },
    ],
    tests: [
      { name: 'test_conditional_success_path', functionName: 'conditionalWorkflow', filePath: 'workflows/conditional.baml', nodeType: 'function' },
    ],
  },
  {
    path: 'functions/utils.baml',
    functions: [
      { name: 'extractUser', type: 'llm_function', filePath: 'functions/utils.baml' },
      { name: 'helperFunction', type: 'function', filePath: 'functions/utils.baml' },
    ],
    tests: [
      { name: 'test_extract_valid_user', functionName: 'extractUser', filePath: 'functions/utils.baml', nodeType: 'llm_function' },
    ],
  },
];

// ============================================================================
// Test Suites
// ============================================================================

describe('Navigation Heuristic - Test Click Events', () => {
  it('should switch to workflow when test targets a workflow', () => {
    const event: CodeClickEvent = {
      type: 'test',
      testName: 'test_simple_success',
      functionName: 'simpleWorkflow',
      filePath: 'workflows/simple.baml',
      nodeType: 'function',
    };

    const state: NavigationState = {
      activeWorkflowId: null,
      workflows: mockWorkflows,
      bamlFiles: mockBAMLFiles,
    };

    const action = determineNavigationAction(event, state);

    expect(action).toEqual({
      type: 'switch-workflow',
      workflowId: 'simpleWorkflow',
    });
  });

  it('should switch to workflow and select node when test targets a function in a workflow', () => {
    const event: CodeClickEvent = {
      type: 'test',
      testName: 'test_fetchData_success',
      functionName: 'fetchData',
      filePath: 'workflows/simple.baml',
      nodeType: 'function',
    };

    const state: NavigationState = {
      activeWorkflowId: null,
      workflows: mockWorkflows,
      bamlFiles: mockBAMLFiles,
    };

    const action = determineNavigationAction(event, state);

    expect(action).toEqual({
      type: 'switch-and-select',
      workflowId: 'simpleWorkflow',
      nodeId: 'fetchData',
      testId: 'test_fetchData_success',
    });
  });

  it('should switch to workflow and select node when test targets an LLM function in a workflow', () => {
    const event: CodeClickEvent = {
      type: 'test',
      testName: 'test_processData_valid',
      functionName: 'processData',
      filePath: 'workflows/simple.baml',
      nodeType: 'llm_function',
    };

    const state: NavigationState = {
      activeWorkflowId: 'conditionalWorkflow', // Different workflow is active
      workflows: mockWorkflows,
      bamlFiles: mockBAMLFiles,
    };

    const action = determineNavigationAction(event, state);

    expect(action).toEqual({
      type: 'switch-and-select',
      workflowId: 'simpleWorkflow',
      nodeId: 'processData',
      testId: 'test_processData_valid',
    });
  });

  it('should show function tests when test targets a standalone function with tests', () => {
    const event: CodeClickEvent = {
      type: 'test',
      testName: 'test_extract_valid_user',
      functionName: 'extractUser',
      filePath: 'functions/utils.baml',
      nodeType: 'llm_function',
    };

    const state: NavigationState = {
      activeWorkflowId: null,
      workflows: mockWorkflows,
      bamlFiles: mockBAMLFiles,
    };

    const action = determineNavigationAction(event, state);

    expect(action).toEqual({
      type: 'show-function-tests',
      functionName: 'extractUser',
      tests: ['test_extract_valid_user'],
    });
  });

  it('should show empty state when test targets a function with no workflow or tests', () => {
    const event: CodeClickEvent = {
      type: 'test',
      testName: 'test_unknown_function',
      functionName: 'unknownFunction',
      filePath: 'unknown.baml',
      nodeType: 'function',
    };

    const state: NavigationState = {
      activeWorkflowId: null,
      workflows: mockWorkflows,
      bamlFiles: mockBAMLFiles,
    };

    const action = determineNavigationAction(event, state);

    expect(action).toEqual({
      type: 'empty-state',
      reason: 'Test function is not part of any workflow',
      functionName: 'unknownFunction',
    });
  });

  it('should stay in current workflow when test targets a function that exists in both current and other workflows', () => {
    // Setup: Create a scenario where fetchData exists in both simpleWorkflow and sharedWorkflow
    const sharedWorkflow: WorkflowDefinition = {
      id: 'sharedWorkflow',
      displayName: 'Shared Workflow',
      filePath: 'shared/workflows/shared.baml',
      startLine: 1,
      endLine: 100,
      codeHash: 'hash3',
      entryPoint: 'sharedWorkflow',
      parameters: [],
      returnType: 'any',
      childFunctions: ['aggregateData', 'fetchData'],
      lastModified: Date.now(),
      nodes: [
        { id: 'sharedWorkflow', type: 'function', label: 'Start', position: { x: 0, y: 0 }, codeHash: 'hash', lastModified: Date.now() },
        { id: 'aggregateData', type: 'function', label: 'Aggregate Data', position: { x: 0, y: 100 }, codeHash: 'hash', lastModified: Date.now() },
        { id: 'fetchData', type: 'function', label: 'Fetch Data', position: { x: 0, y: 200 }, codeHash: 'hash', lastModified: Date.now() },
      ],
      edges: [
        { id: 'e1', source: 'sharedWorkflow', target: 'aggregateData' },
        { id: 'e2', source: 'aggregateData', target: 'fetchData' },
      ],
    };

    const event: CodeClickEvent = {
      type: 'test',
      testName: 'test_fetchData_in_shared',
      functionName: 'fetchData',
      filePath: 'shared/workflows/shared.baml',
      nodeType: 'function',
    };

    const state: NavigationState = {
      activeWorkflowId: 'sharedWorkflow', // Currently viewing sharedWorkflow
      workflows: [...mockWorkflows, sharedWorkflow], // fetchData exists in simpleWorkflow and sharedWorkflow
      bamlFiles: mockBAMLFiles,
    };

    const action = determineNavigationAction(event, state);

    // Should select the node in the current workflow, not switch to simpleWorkflow
    expect(action).toEqual({
      type: 'select-node',
      workflowId: 'sharedWorkflow',
      nodeId: 'fetchData',
      testId: 'test_fetchData_in_shared',
    });
  });
});

describe('Navigation Heuristic - Function Click Events', () => {
  describe('Priority 1: Stay in current workflow', () => {
    it('should select node when function exists in current workflow', () => {
      const event: CodeClickEvent = {
        type: 'function',
        functionName: 'processData',
        functionType: 'llm_function',
        filePath: 'workflows/simple.baml',
      };

      const state: NavigationState = {
        activeWorkflowId: 'simpleWorkflow',
        workflows: mockWorkflows,
        bamlFiles: mockBAMLFiles,
      };

      const action = determineNavigationAction(event, state);

      expect(action).toEqual({
        type: 'select-node',
        workflowId: 'simpleWorkflow',
        nodeId: 'processData',
      });
    });

    it('should select workflow node itself when clicking on workflow function in current workflow', () => {
      const event: CodeClickEvent = {
        type: 'function',
        functionName: 'simpleWorkflow',
        functionType: 'workflow',
        filePath: 'workflows/simple.baml',
      };

      const state: NavigationState = {
        activeWorkflowId: 'simpleWorkflow',
        workflows: mockWorkflows,
        bamlFiles: mockBAMLFiles,
      };

      const action = determineNavigationAction(event, state);

      expect(action).toEqual({
        type: 'select-node',
        workflowId: 'simpleWorkflow',
        nodeId: 'simpleWorkflow',
      });
    });
  });

  describe('Priority 2: Switch to workflow containing function', () => {
    it('should switch workflow when function exists in different workflow', () => {
      const event: CodeClickEvent = {
        type: 'function',
        functionName: 'handleSuccess',
        functionType: 'llm_function',
        filePath: 'workflows/conditional.baml',
      };

      const state: NavigationState = {
        activeWorkflowId: 'simpleWorkflow',
        workflows: mockWorkflows,
        bamlFiles: mockBAMLFiles,
      };

      const action = determineNavigationAction(event, state);

      expect(action).toEqual({
        type: 'switch-and-select',
        workflowId: 'conditionalWorkflow',
        nodeId: 'handleSuccess',
      });
    });

    it('should switch to workflow when no current workflow is active', () => {
      const event: CodeClickEvent = {
        type: 'function',
        functionName: 'fetchData',
        functionType: 'function',
        filePath: 'workflows/simple.baml',
      };

      const state: NavigationState = {
        activeWorkflowId: null,
        workflows: mockWorkflows,
        bamlFiles: mockBAMLFiles,
      };

      const action = determineNavigationAction(event, state);

      expect(action).toEqual({
        type: 'switch-and-select',
        workflowId: 'simpleWorkflow',
        nodeId: 'fetchData',
      });
    });

    it('should switch to workflow when function is a workflow itself', () => {
      const event: CodeClickEvent = {
        type: 'function',
        functionName: 'conditionalWorkflow',
        functionType: 'workflow',
        filePath: 'workflows/conditional.baml',
      };

      const state: NavigationState = {
        activeWorkflowId: 'simpleWorkflow',
        workflows: mockWorkflows,
        bamlFiles: mockBAMLFiles,
      };

      const action = determineNavigationAction(event, state);

      expect(action).toEqual({
        type: 'switch-and-select',
        workflowId: 'conditionalWorkflow',
        nodeId: 'conditionalWorkflow',
      });
    });
  });

  describe('Priority 3: Show function in isolation (with tests)', () => {
    it('should show function tests when function has tests but is not in any workflow', () => {
      const event: CodeClickEvent = {
        type: 'function',
        functionName: 'extractUser',
        functionType: 'llm_function',
        filePath: 'functions/utils.baml',
      };

      const state: NavigationState = {
        activeWorkflowId: null,
        workflows: mockWorkflows,
        bamlFiles: mockBAMLFiles,
      };

      const action = determineNavigationAction(event, state);

      expect(action).toEqual({
        type: 'show-function-tests',
        functionName: 'extractUser',
        tests: ['test_extract_valid_user'],
      });
    });
  });

  describe('Priority 4: Empty state', () => {
    it('should show empty state when function is not in any workflow and has no tests', () => {
      const event: CodeClickEvent = {
        type: 'function',
        functionName: 'helperFunction',
        functionType: 'function',
        filePath: 'functions/utils.baml',
      };

      const state: NavigationState = {
        activeWorkflowId: null,
        workflows: mockWorkflows,
        bamlFiles: mockBAMLFiles,
      };

      const action = determineNavigationAction(event, state);

      expect(action).toEqual({
        type: 'empty-state',
        reason: 'Function is not part of any workflow and has no tests',
        functionName: 'helperFunction',
      });
    });

    it('should show empty state when function does not exist anywhere', () => {
      const event: CodeClickEvent = {
        type: 'function',
        functionName: 'nonExistentFunction',
        functionType: 'function',
        filePath: 'unknown.baml',
      };

      const state: NavigationState = {
        activeWorkflowId: null,
        workflows: mockWorkflows,
        bamlFiles: mockBAMLFiles,
      };

      const action = determineNavigationAction(event, state);

      expect(action).toEqual({
        type: 'empty-state',
        reason: 'Function is not part of any workflow and has no tests',
        functionName: 'nonExistentFunction',
      });
    });
  });
});

describe('Navigation Heuristic - Edge Cases', () => {
  it('should handle empty workflows list', () => {
    const event: CodeClickEvent = {
      type: 'function',
      functionName: 'someFunction',
      functionType: 'function',
      filePath: 'test.baml',
    };

    const state: NavigationState = {
      activeWorkflowId: null,
      workflows: [],
      bamlFiles: mockBAMLFiles,
    };

    const action = determineNavigationAction(event, state);

    // Function doesn't exist in BAML files, so should show empty state
    expect(action.type).toBe('empty-state');
  });

  it('should handle empty BAML files list', () => {
    const event: CodeClickEvent = {
      type: 'function',
      functionName: 'someFunction',
      functionType: 'function',
      filePath: 'test.baml',
    };

    const state: NavigationState = {
      activeWorkflowId: null,
      workflows: mockWorkflows,
      bamlFiles: [],
    };

    const action = determineNavigationAction(event, state);

    expect(action.type).toBe('empty-state');
  });

  it('should handle workflow that does not exist in workflows list', () => {
    const event: CodeClickEvent = {
      type: 'function',
      functionName: 'someFunction',
      functionType: 'function',
      filePath: 'test.baml',
    };

    const state: NavigationState = {
      activeWorkflowId: 'nonExistentWorkflow',
      workflows: mockWorkflows,
      bamlFiles: mockBAMLFiles,
    };

    const action = determineNavigationAction(event, state);

    // Should skip Priority 1 (current workflow check) and continue to other priorities
    expect(action.type).toBe('empty-state');
  });
});

describe('Navigation Heuristic - Complex Scenarios', () => {
  it('should prioritize current workflow over switching to different workflow', () => {
    // Function exists in both current workflow and another workflow
    // Should select node in current workflow (Priority 1)
    const event: CodeClickEvent = {
      type: 'function',
      functionName: 'fetchData',
      functionType: 'function',
      filePath: 'workflows/simple.baml',
    };

    const state: NavigationState = {
      activeWorkflowId: 'simpleWorkflow',
      workflows: mockWorkflows,
      bamlFiles: mockBAMLFiles,
    };

    const action = determineNavigationAction(event, state);

    expect(action.type).toBe('select-node');
    expect(action).toHaveProperty('workflowId', 'simpleWorkflow');
  });

  it('should handle function with multiple tests', () => {
    const event: CodeClickEvent = {
      type: 'function',
      functionName: 'simpleWorkflow',
      functionType: 'workflow',
      filePath: 'workflows/simple.baml',
    };

    const state: NavigationState = {
      activeWorkflowId: 'simpleWorkflow',
      workflows: mockWorkflows,
      bamlFiles: mockBAMLFiles,
    };

    const action = determineNavigationAction(event, state);

    // simpleWorkflow is in the current workflow, so should select it
    expect(action).toEqual({
      type: 'select-node',
      workflowId: 'simpleWorkflow',
      nodeId: 'simpleWorkflow',
    });
  });
});
