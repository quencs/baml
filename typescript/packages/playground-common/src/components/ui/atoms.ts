import { atom } from 'jotai';

// Active file atom for tree view
export const activeFileAtom = atom<string | null>(null);

// Function object atom
export const functionObjectAtom = atom<any>(null);

// Test case object atom (if needed for UI components)
export const testcaseObjectAtom = atom<any>(null);

// Show token counts atom for render text
export const showTokenCountsAtom = atom<boolean>(false);