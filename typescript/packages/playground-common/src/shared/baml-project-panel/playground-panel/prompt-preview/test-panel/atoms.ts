import { atom } from 'jotai';
import { atomWithStorage } from 'jotai/utils';
import type { TestHistoryRun } from '../../../../../types';

// Test panel atoms
export const testHistoryAtom = atom<TestHistoryRun[]>([]);
export const selectedHistoryIndexAtom = atom<number>(0);
export const isParallelTestsEnabledAtom = atomWithStorage<boolean>('parallel-tests-enabled', false);

// Re-export types
export type { TestHistoryRun };