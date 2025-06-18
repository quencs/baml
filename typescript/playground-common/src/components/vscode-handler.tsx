'use client'

import { useCallback, useEffect } from 'react'
import { useFiles, useEnvVars } from '../contexts/runtime-context'
import { useTestSelection } from '../hooks/use-test-runner'
import { useTestRunner } from '../hooks/use-test-runner'
import { 
  useVSCodeFileHandler, 
  useVSCodeSelectionHandler, 
  useVSCodeTestHandler, 
  useVSCodeConfigHandler,
  useVSCode 
} from '../hooks/use-vscode'

interface VSCodeHandlerProps {
  onConfigUpdate?: (config: any) => void;
  onVersionUpdate?: (version: string) => void;
  onFlashingRegions?: (spans: any[]) => void;
}

export function VSCodeHandler({ 
  onConfigUpdate, 
  onVersionUpdate, 
  onFlashingRegions 
}: VSCodeHandlerProps) {
  const { setFiles } = useFiles();
  const { selectedFunction, selectFunction, selectTestcase } = useTestSelection();
  const { runSingleTest } = useTestRunner();
  const { markInitialized } = useVSCode();

  // Handle file operations
  useVSCodeFileHandler(
    useCallback((files: Record<string, string>) => {
      // Debounce in a real implementation
      setFiles(files);
    }, [setFiles]),
    undefined, // onModifyFile - not needed for now
    useCallback(() => {
      setFiles({});
    }, [setFiles])
  );

  // Handle function selection and cursor updates
  useVSCodeSelectionHandler(
    useCallback((functionName: string) => {
      selectFunction(functionName);
    }, [selectFunction]),
    useCallback((cursor: any) => {
      // Handle cursor update logic here
      console.log('Cursor updated:', cursor);
    }, [])
  );

  // Handle test running
  useVSCodeTestHandler(
    useCallback((testName: string) => {
      if (selectedFunction) {
        selectTestcase(testName);
        runSingleTest(selectedFunction, testName);
      }
    }, [selectedFunction, selectTestcase, runSingleTest]),
    selectedFunction
  );

  // Handle config updates
  useVSCodeConfigHandler(
    useCallback((config: any) => {
      onConfigUpdate?.(config);
    }, [onConfigUpdate]),
    useCallback((version: string) => {
      onVersionUpdate?.(version);
    }, [onVersionUpdate])
  );

  // Mark VSCode as initialized when component mounts
  useEffect(() => {
    markInitialized();
  }, [markInitialized]);

  // This is a pure side-effect component, so it renders nothing
  return null;
}