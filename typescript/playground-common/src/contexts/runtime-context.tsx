'use client'

import React, { createContext, useContext, useReducer, useEffect, ReactNode } from 'react'
import type { WasmRuntime, WasmProject, WasmDiagnosticError } from '../types'

interface RuntimeState {
  wasm?: any;
  project?: WasmProject;
  runtime?: WasmRuntime;
  diagnostics: WasmDiagnosticError[];
  files: Record<string, string>;
  envVars: Record<string, string>;
  isReady: boolean;
  isLoading: boolean;
  error?: string;
}

type RuntimeAction =
  | { type: 'WASM_READY'; wasm: any }
  | { type: 'SET_FILES'; files: Record<string, string> }
  | { type: 'SET_ENV_VARS'; envVars: Record<string, string> }
  | { type: 'PROJECT_UPDATE'; project: WasmProject }
  | { type: 'RUNTIME_UPDATE'; runtime: WasmRuntime; diagnostics: WasmDiagnosticError[] }
  | { type: 'SET_ERROR'; error: string }
  | { type: 'SET_LOADING'; loading: boolean };

const initialState: RuntimeState = {
  diagnostics: [],
  files: {},
  envVars: {},
  isReady: false,
  isLoading: true,
};

function runtimeReducer(state: RuntimeState, action: RuntimeAction): RuntimeState {
  switch (action.type) {
    case 'WASM_READY':
      return {
        ...state,
        wasm: action.wasm,
        isLoading: false,
        isReady: true,
      };
    case 'SET_FILES':
      return {
        ...state,
        files: action.files,
      };
    case 'SET_ENV_VARS':
      return {
        ...state,
        envVars: action.envVars,
      };
    case 'PROJECT_UPDATE':
      return {
        ...state,
        project: action.project,
      };
    case 'RUNTIME_UPDATE':
      return {
        ...state,
        runtime: action.runtime,
        diagnostics: action.diagnostics,
      };
    case 'SET_ERROR':
      return {
        ...state,
        error: action.error,
        isLoading: false,
      };
    case 'SET_LOADING':
      return {
        ...state,
        isLoading: action.loading,
      };
    default:
      return state;
  }
}

interface RuntimeContextType {
  state: RuntimeState;
  dispatch: React.Dispatch<RuntimeAction>;
}

const RuntimeContext = createContext<RuntimeContextType | null>(null);

export function RuntimeProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(runtimeReducer, initialState);

  // Initialize WASM on mount
  useEffect(() => {
    const initWasm = async () => {
      try {
        dispatch({ type: 'SET_LOADING', loading: true });
        // For now, use a mock during refactoring
        // const wasm = await import('@gloo-ai/baml-schema-wasm-web/baml_schema_build');
        const wasm = { 
          WasmProject: { new: () => ({}) },
          version: () => 'dev-version'
        };
        // Initialize callbacks if needed
        // wasm.init_js_callback_bridge(vscode.loadAwsCreds, vscode.loadGcpCreds);
        dispatch({ type: 'WASM_READY', wasm });
      } catch (error) {
        dispatch({ type: 'SET_ERROR', error: error instanceof Error ? error.message : 'Failed to load WASM' });
      }
    };

    initWasm();
  }, []);

  // Update project when files or wasm change
  useEffect(() => {
    if (state.wasm && Object.keys(state.files).length > 0) {
      try {
        const bamlFiles = Object.entries(state.files).filter(([path]) => path.endsWith('.baml'));
        const project = state.wasm.WasmProject.new('./', bamlFiles);
        dispatch({ type: 'PROJECT_UPDATE', project });
      } catch (error) {
        dispatch({ type: 'SET_ERROR', error: error instanceof Error ? error.message : 'Failed to create project' });
      }
    }
  }, [state.wasm, state.files]);

  // Update runtime when project or env vars change
  useEffect(() => {
    if (state.project) {
      try {
        const selectedEnvVars = Object.fromEntries(
          Object.entries(state.envVars).filter(([, value]) => value !== undefined)
        );
        const runtime = state.project.runtime(selectedEnvVars);
        const diagnostics = state.project.diagnostics(runtime);
        dispatch({ type: 'RUNTIME_UPDATE', runtime, diagnostics: diagnostics?.errors() ?? [] });
      } catch (error) {
        dispatch({ type: 'SET_ERROR', error: error instanceof Error ? error.message : 'Failed to create runtime' });
      }
    }
  }, [state.project, state.envVars]);

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

// Convenience hooks for specific parts of the state
export function useWasm() {
  const { state } = useRuntime();
  return state.wasm;
}

export function useProject() {
  const { state } = useRuntime();
  return state.project;
}

export function useRuntimeInstance() {
  const { state } = useRuntime();
  return state.runtime;
}

export function useDiagnostics() {
  const { state } = useRuntime();
  return state.diagnostics;
}

export function useFiles() {
  const { state, dispatch } = useRuntime();
  return {
    files: state.files,
    setFiles: (files: Record<string, string>) => dispatch({ type: 'SET_FILES', files }),
  };
}

export function useEnvVars() {
  const { state, dispatch } = useRuntime();
  return {
    envVars: state.envVars,
    setEnvVars: (envVars: Record<string, string>) => dispatch({ type: 'SET_ENV_VARS', envVars }),
  };
}