import { useCallback, useEffect } from 'react'

// Mock VSCode API for development
const mockVscode = {
  isVscode: () => false,
  postMessage: (message: any) => {
    console.log('Mock VSCode postMessage:', message);
  },
  markInitialized: () => {
    console.log('Mock VSCode markInitialized');
  },
  getPlaygroundPort: () => Promise.resolve(0),
  loadAwsCreds: () => {},
  loadGcpCreds: () => {},
};

// This would be replaced with actual VSCode imports in the real environment
const vscode = mockVscode;

export interface VSCodeMessage {
  command: string;
  content?: any;
  meta?: any;
}

export interface MessageHandler {
  (message: any): void;
}

export interface FileMessage {
  command: 'modify_file' | 'add_project' | 'remove_project';
  content: {
    root_path: string;
    name?: string;
    content?: string;
    files?: Record<string, string>;
  };
}

export interface FlashingRegionsMessage {
  command: 'set_flashing_regions';
  content: {
    spans: Array<{
      file_path: string;
      start_line: number;
      start: number;
      end_line: number;
      end: number;
    }>;
  };
}

export interface FunctionSelectionMessage {
  command: 'select_function';
  content: {
    root_path: string;
    function_name: string;
  };
}

export interface CursorUpdateMessage {
  command: 'update_cursor';
  content: {
    cursor: {
      fileName: string;
      fileText: string;
      line: number;
      column: number;
    };
  };
}

export interface TestRunMessage {
  command: 'run_test';
  content: {
    test_name: string;
  };
}

export interface ConfigMessage {
  command: 'baml_settings_updated' | 'baml_cli_version';
  content: any;
}

export type IncomingVSCodeMessage = 
  | FileMessage
  | FlashingRegionsMessage
  | FunctionSelectionMessage
  | CursorUpdateMessage
  | TestRunMessage
  | ConfigMessage;

export function useVSCode() {
  const postMessage = useCallback((message: VSCodeMessage) => {
    try {
      vscode.postMessage(message);
    } catch (error) {
      console.error('Failed to post message to VSCode:', error);
    }
  }, []);

  const markInitialized = useCallback(() => {
    try {
      vscode.markInitialized();
    } catch (error) {
      console.error('Failed to mark VSCode as initialized:', error);
    }
  }, []);

  const getPlaygroundPort = useCallback(async () => {
    try {
      return await vscode.getPlaygroundPort();
    } catch (error) {
      console.error('Failed to get playground port:', error);
      return 0;
    }
  }, []);

  const isVSCodeEnvironment = useCallback(() => {
    return vscode.isVscode();
  }, []);

  return {
    postMessage,
    markInitialized,
    getPlaygroundPort,
    isVSCodeEnvironment,
  };
}

export function useVSCodeMessageHandler(handler: MessageHandler) {
  useEffect(() => {
    const messageListener = (event: MessageEvent<IncomingVSCodeMessage>) => {
      try {
        handler(event.data);
      } catch (error) {
        console.error('Error handling VSCode message:', error);
      }
    };

    window.addEventListener('message', messageListener);
    return () => window.removeEventListener('message', messageListener);
  }, [handler]);
}

// Convenience hooks for specific message types
export function useVSCodeFileHandler(
  onAddProject: (files: Record<string, string>) => void,
  onModifyFile?: (name: string, content: string | undefined) => void,
  onRemoveProject?: () => void
) {
  useVSCodeMessageHandler(useCallback((message: IncomingVSCodeMessage) => {
    switch (message.command) {
      case 'add_project':
        if (message.content?.files) {
          onAddProject(message.content.files);
        }
        break;
      case 'modify_file':
        if (onModifyFile && message.content?.name) {
          onModifyFile(message.content.name, message.content.content);
        }
        break;
      case 'remove_project':
        if (onRemoveProject) {
          onRemoveProject();
        }
        break;
    }
  }, [onAddProject, onModifyFile, onRemoveProject]));
}

export function useVSCodeTestHandler(
  onRunTest: (testName: string) => void,
  selectedFunction?: string
) {
  useVSCodeMessageHandler(useCallback((message: IncomingVSCodeMessage) => {
    if (message.command === 'run_test' && message.content?.test_name) {
      if (selectedFunction) {
        onRunTest(message.content.test_name);
      } else {
        console.error('No function selected for test run');
      }
    }
  }, [onRunTest, selectedFunction]));
}

export function useVSCodeSelectionHandler(
  onSelectFunction: (functionName: string) => void,
  onUpdateCursor?: (cursor: any) => void
) {
  useVSCodeMessageHandler(useCallback((message: IncomingVSCodeMessage) => {
    switch (message.command) {
      case 'select_function':
        if (message.content?.function_name) {
          onSelectFunction(message.content.function_name);
        }
        break;
      case 'update_cursor':
        if (onUpdateCursor && message.content?.cursor) {
          onUpdateCursor(message.content.cursor);
        }
        break;
    }
  }, [onSelectFunction, onUpdateCursor]));
}

export function useVSCodeConfigHandler(
  onConfigUpdate: (config: any) => void,
  onVersionUpdate?: (version: string) => void
) {
  useVSCodeMessageHandler(useCallback((message: IncomingVSCodeMessage) => {
    switch (message.command) {
      case 'baml_settings_updated':
        onConfigUpdate(message.content);
        break;
      case 'baml_cli_version':
        if (onVersionUpdate) {
          onVersionUpdate(message.content);
        }
        break;
    }
  }, [onConfigUpdate, onVersionUpdate]));
}

export function useVSCodeFlashingHandler(
  onSetFlashingRegions: (spans: any[]) => void
) {
  useVSCodeMessageHandler(useCallback((message: IncomingVSCodeMessage) => {
    if (message.command === 'set_flashing_regions' && message.content?.spans) {
      onSetFlashingRegions(message.content.spans.map(span => ({
        filePath: span.file_path,
        startLine: span.start_line,
        startCol: span.start,
        endLine: span.end_line,
        endCol: span.end,
      })));
    }
  }, [onSetFlashingRegions]));
}