/**
 * Unit tests for navigation heuristic
 *
 * Tests all scenarios described in navigationHeuristic.ts documentation
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { determineNavigationAction, type NavigationState } from './navigationHeuristic';
import type { CodeClickEvent, BAMLFile } from './types';
import type { FunctionWithCallGraph } from './interface';
import { createMockRuntimeConfig } from './mock-config/config';

// ============================================================================
// Test Data Setup - Using Centralized Mock Config
// ============================================================================

let mockWorkflows: FunctionWithCallGraph[];
let mockBAMLFiles: BAMLFile[];

beforeAll(() => {
  const mockConfig = createMockRuntimeConfig();
  mockWorkflows = mockConfig.workflows;
  mockBAMLFiles = mockConfig.bamlFiles;
});

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
    // Setup: fetchData exists in both simpleWorkflow and sharedWorkflow (from mock config)
    const event: CodeClickEvent = {
      type: 'test',
      testName: 'test_fetchData_in_shared',
      functionName: 'fetchData',
      filePath: '/mock/sharedWorkflow.baml',
      nodeType: 'function',
    };

    const state: NavigationState = {
      activeWorkflowId: 'sharedWorkflow', // Currently viewing sharedWorkflow
      workflows: mockWorkflows, // fetchData exists in simpleWorkflow and sharedWorkflow
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
