import type { DoneTestStatusType, TestState } from '../../atoms';

export type FinalTestStatus = DoneTestStatusType | 'running' | 'idle' | 'cancelled';

export const getStatus = (response: TestState) => {
  if (response.status === 'running') {
    return 'running';
  }
  if (response.status === 'cancelled') {
    return 'cancelled';
  }
  if (response.status === 'done') {
    return response.response_status;
  }
  return 'idle';
};

export const getTestStateResponse = (response: TestState) => {
  if (response.status === 'done') {
    return response.response;
  } else if (response.status === 'running') {
    return response.response;
  }
  return undefined;
};

export const getExplanation = (response: TestState) => {
  if (response.status === 'done') {
    return response.response.parsed_response()?.explanation;
  }
  return undefined;
};
