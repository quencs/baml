import { atom } from 'jotai';

// Prompt panel related atoms
export const selectedFunctionAtom = atom<string | null>(null);
export const selectedTestcaseAtom = atom<string | null>(null);
export const isClientCallGraphEnabledAtom = atom<boolean>(false);