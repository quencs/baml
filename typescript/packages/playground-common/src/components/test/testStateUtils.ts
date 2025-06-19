import type { TestState } from '../../atoms';

export type FinalTestStatus = 
  | 'passed' 
  | 'llm_failed' 
  | 'parse_failed' 
  | 'constraints_failed' 
  | 'assert_failed' 
  | 'error';

export function getStatus(state: TestState): FinalTestStatus | undefined {
  if (state.status !== 'done') return undefined;
  
  // Add logic to determine final status based on response
  // This is a simplified implementation
  if (state.response) {
    return 'passed';
  }
  return 'error';
}

export function getTestStateResponse(state: TestState) {
  return state.response;
}

export function getExplanation(state: TestState): string | undefined {
  // Return explanation if available
  return undefined;
}