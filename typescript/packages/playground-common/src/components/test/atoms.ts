import { atom } from 'jotai';
import { atomWithStorage } from 'jotai/utils';
import type { TestHistoryRun } from '../../types';

// Response view type for tabular view
export type ResponseViewType = 'parsed' | 'pretty' | 'raw';

// Test panel view type
export type TestPanelViewType = 'simple' | 'card' | 'tabular';

// Configuration for tabular view
export type TabularViewConfig = {
  showInputs: boolean;
  showModel: boolean;
  showDuration: boolean;
  responseViewType: ResponseViewType;
};

// Atoms
export const tabularViewConfigAtom = atomWithStorage<TabularViewConfig>(
  'tabular-view-config',
  {
    showInputs: true,
    showModel: true,
    showDuration: true,
    responseViewType: 'parsed',
  }
);

export const testPanelViewTypeAtom = atomWithStorage<TestPanelViewType>(
  'test-panel-view-type',
  'simple'
);

export const testHistoryAtom = atom<TestHistoryRun[]>([]);

export const selectedHistoryIndexAtom = atom<number>(0);

// Re-export types
export type { TestHistoryRun };