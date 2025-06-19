'use client';

import type { WasmDiagnosticError, WasmProject, WasmRuntime } from '@gloo-ai/baml-schema-wasm-web/baml_schema_build';
import { createContext, useContext, useEffect, useReducer } from 'react';

interface RuntimeState {
	wasm?: any; // WasmModule
	project?: WasmProject;
	runtime?: WasmRuntime;
	diagnostics: WasmDiagnosticError[];
	files: Record<string, string>;
	isReady: boolean;
	error?: string;
}

type RuntimeAction =
	| { type: 'WASM_READY'; wasm: any }
	| { type: 'SET_FILES'; files: Record<string, string> }
	| { type: 'PROJECT_UPDATED'; project: WasmProject }
	| { type: 'RUNTIME_UPDATED'; runtime: WasmRuntime; diagnostics: WasmDiagnosticError[] }
	| { type: 'ERROR'; error: string }
	| { type: 'RESET' };

const initialState: RuntimeState = {
	diagnostics: [],
	files: {},
	isReady: false,
};

function runtimeReducer(state: RuntimeState, action: RuntimeAction): RuntimeState {
	switch (action.type) {
		case 'WASM_READY':
			return { ...state, wasm: action.wasm, isReady: true };

		case 'SET_FILES':
			return { ...state, files: action.files };

		case 'PROJECT_UPDATED':
			return { ...state, project: action.project };

		case 'RUNTIME_UPDATED':
			return { ...state, runtime: action.runtime, diagnostics: action.diagnostics };

		case 'ERROR':
			return { ...state, error: action.error };

		case 'RESET':
			return initialState;

		default:
			return state;
	}
}

interface RuntimeContextValue {
	state: RuntimeState;
	dispatch: React.Dispatch<RuntimeAction>;
}

const RuntimeContext = createContext<RuntimeContextValue | null>(null);

export function RuntimeProvider({ children }: { children: React.ReactNode }) {
	const [state, dispatch] = useReducer(runtimeReducer, initialState);

	// Initialize WASM
	useEffect(() => {
		const initWasm = async () => {
			try {
				const wasm = await import('@gloo-ai/baml-schema-wasm-web/baml_schema_build');
				dispatch({ type: 'WASM_READY', wasm });
			} catch (error) {
				dispatch({ type: 'ERROR', error: `Failed to load WASM: ${error}` });
			}
		};
		initWasm();
	}, []);

	// Update project when files change
	useEffect(() => {
		if (!state.wasm || !Object.keys(state.files).length) return;

		try {
			const bamlFiles = Object.entries(state.files).filter(([path]) =>
				path.endsWith('.baml')
			);
			const project = state.wasm.WasmProject.new('./', bamlFiles);
			dispatch({ type: 'PROJECT_UPDATED', project });
		} catch (error) {
			dispatch({ type: 'ERROR', error: `Failed to create project: ${error}` });
		}
	}, [state.wasm, state.files]);

	// Update runtime when project changes
	useEffect(() => {
		if (!state.project) return;

		try {
			const runtime = state.project.runtime({});
			const diagnostics = state.project.diagnostics(runtime);
			dispatch({ type: 'RUNTIME_UPDATED', runtime, diagnostics: diagnostics?.errors() ?? [] });
		} catch (error) {
			dispatch({ type: 'ERROR', error: `Failed to create runtime: ${error}` });
		}
	}, [state.project]);

	return (
		<RuntimeContext.Provider value={{ state, dispatch }}>
			{children}
		</RuntimeContext.Provider>
	);
}

export function useRuntime() {
	const context = useContext(RuntimeContext);
	if (!context) {
		throw new Error('useRuntime must be used within RuntimeProvider');
	}
	return context;
}

// Convenience hooks for specific runtime state
export function useRuntimeState() {
	const { state } = useRuntime();
	return state;
}

export function useRuntimeActions() {
	const { dispatch } = useRuntime();
	return {
		setFiles: (files: Record<string, string>) => dispatch({ type: 'SET_FILES', files }),
		reset: () => dispatch({ type: 'RESET' }),
	};
}