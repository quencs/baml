'use client';

import { useAtomValue } from 'jotai';
import { useEffect } from 'react';
import { wasmAtom } from '../shared/baml-project-panel/atoms';
import { vscode } from '../shared/baml-project-panel/vscode';

export function RuntimeInitializer() {
	const wasm = useAtomValue(wasmAtom);

	useEffect(() => {
		if (wasm) {
			console.log('WASM runtime ready!');
			try {
				vscode.markInitialized();
			} catch (error) {
				console.error('Error marking VSCode as initialized:', error);
			}
		}
	}, [wasm]);

	// This is a pure side-effect component, no UI
	return null;
}