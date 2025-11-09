/**
 * Debug Panel Component
 *
 * Simulates clicking on functions and tests in BAML files
 * to test how the app reacts to code navigation events
 */

import { useAtom, useSetAtom } from 'jotai';
import { Play, FileCode, Folder, FolderOpen, ChevronRight, ChevronDown, Plus, Edit } from 'lucide-react';
import { useState, useEffect } from 'react';
import { bamlFilesAtom, activeCodeClickAtom } from '../../../sdk/atoms/core.atoms';
import type { BAMLFunction, BAMLTest, CodeClickEvent } from '../../../sdk/types';
import { useBAMLSDK } from '../../../sdk/provider';
import type { VscodeToWebviewCommand } from '../../../baml_wasm_web/vscode-to-webview-rpc';
import { useRunBamlTests } from '../../../shared/baml-project-panel/playground-panel/prompt-preview/test-panel/test-runner';
import { unifiedSelectionAtom } from '../../../shared/baml-project-panel/playground-panel/unified-atoms';

export function DebugPanel() {
  const sdk = useBAMLSDK();
  const { runTests: runBamlTests } = useRunBamlTests();
  const [bamlFiles, setBAMLFiles] = useAtom(bamlFilesAtom);
  const setActiveCodeClick = useSetAtom(activeCodeClickAtom);
  const [activeCodeClick] = useAtom(activeCodeClickAtom);
  const setUnifiedSelection = useSetAtom(unifiedSelectionAtom);
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set());

  // Load BAML files on mount
  useEffect(() => {
    console.log('[DebugPanel] Mounted, loading BAML files...');
    const files = sdk.diagnostics.getBAMLFiles();
    // console.log('[DebugPanel] Loaded files:', files);
    setBAMLFiles(files);
    // Expand all files by default
    setExpandedFiles(new Set(files.map((f: any) => f.path)));
  }, [sdk, setBAMLFiles]);

  console.log('[DebugPanel] Rendering with', bamlFiles.length, 'files');

  const toggleFile = (path: string) => {
    setExpandedFiles(prev => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  const handleFunctionClick = (func: BAMLFunction) => {
    const event: CodeClickEvent = {
      type: 'function',
      functionName: func.name,
      functionType: func.type,
      filePath: func.filePath,
    };
    setActiveCodeClick(event);
    console.log('🔍 Simulated function click:', event);

    // Update unified selection (mirrors SDK atoms)
    setUnifiedSelection((prev) => ({
      ...prev,
      functionName: func.name,
      testName: null,
    }));
  };

  const handleTestClick = (test: BAMLTest) => {
    const event: CodeClickEvent = {
      type: 'test',
      testName: test.name,
      functionName: test.functionName,
      filePath: test.filePath,
      nodeType: test.nodeType,
    };
    setActiveCodeClick(event);
    console.log('🔍 Simulated test click:', event);

    setUnifiedSelection((prev) => ({
      ...prev,
      functionName: test.functionName,
      testName: test.name,
    }));
  };

  const handleTestRun = async (test: BAMLTest, e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent triggering the test click
    console.log('▶️ Running test:', test.name, '→', test.functionName);

    // Run the test - SDK will automatically set selection and trigger scroll
    // Don't call handleTestClick first, as that would set selection before test history exists,
    // preventing the scroll from triggering when selection changes
    await runBamlTests([{ functionName: test.functionName, testName: test.name }]);
  };

  const isActive = (item: BAMLFunction | BAMLTest, type: 'function' | 'test') => {
    if (!activeCodeClick) return false;
    if (type === 'function' && activeCodeClick.type === 'function') {
      return activeCodeClick.functionName === (item as BAMLFunction).name;
    }
    if (type === 'test' && activeCodeClick.type === 'test') {
      return activeCodeClick.testName === (item as BAMLTest).name;
    }
    return false;
  };

  // Simulate adding a new file with CheckAvailability function
  const handleAddNewFile = () => {
    console.log('🆕 Simulating adding new file with CheckAvailability function');

    const newFileContent = `// New functions for testing
function CheckAvailability2(day: string) -> bool {
  client GPT4o

  prompt #"
    Is the office open on {{ day }}?
    The office is open Monday through Friday.

    Return true if open, false if closed.

    {{ ctx.output_format }}
  "#
}

function CountItems2(text: string) -> int {
  client GPT4o

  prompt #"
    Count how many items are mentioned in the following text:

    {{ text }}

    {{ ctx.output_format }}
  "#
}

test CheckAvailabilityTest2 {
  functions [CheckAvailability2]
  args {
    day "Monday"
  }
}

test CountItemsTest2 {
  functions [CountItems2]
  args {
    text "apples, oranges, bananas"
  }
}`;

    // Get current files from SDK
    const currentFiles = sdk.files.getCurrent();

    // Add the new file
    const updatedFiles = {
      ...currentFiles,
      'baml_src/additional_functions.baml': newFileContent,
    };

    // Post message to EventListener (simulates IDE file change)
    const message: VscodeToWebviewCommand = {
      source: 'lsp_message',
      payload: {
        method: 'runtime_updated',
        params: {
          root_path: '/fake/root',
          files: updatedFiles,
        },
      },
    };

    console.log('📨 Posting runtime_updated message with new file');
    window.postMessage(message, '*');
  };

  // Simulate modifying an existing function to add a sentence
  const handleModifyFunction = () => {
    console.log('✏️ Simulating modifying ExtractResume function');

    const currentFiles = sdk.files.getCurrent();

    // Find and modify the main.baml file
    const mainFile = currentFiles['baml_src/main.baml'];
    if (!mainFile) {
      console.warn('⚠️ main.baml not found, cannot modify');
      return;
    }

    // Add a sentence to the ExtractResume prompt
    // Edit the ExtractResume prompt and add the date at the end
    const now = new Date();
    const dateTime = now.toISOString().replace('T', ' ').slice(0, 19); // "YYYY-MM-DD HH:MM:SS"
    const modifiedFile = mainFile.replace(
      'Parse the following resume and return a structured representation of the data in the schema below.',
      `Parse the following resume and return a structured representation of the data in the schema below.

Please be thorough and accurate in your extraction.

Date: ${dateTime}`
    );

    const updatedFiles = {
      ...currentFiles,
      'baml_src/main.baml': modifiedFile,
    };

    // Post message to EventListener (simulates IDE file edit)
    const message: VscodeToWebviewCommand = {
      source: 'lsp_message',
      payload: {
        method: 'runtime_updated',
        params: {
          root_path: '/fake/root',
          files: updatedFiles,
        },
      },
    };

    console.log('📨 Posting runtime_updated message with modified file');
    window.postMessage(message, '*');
  };

  return (
    <div className="absolute bottom-2 right-2 z-[1000] w-[200px] bg-card border border-border rounded-md shadow-lg max-h-[500px] overflow-y-auto">
      {/* Header */}
      <div className="sticky top-0 bg-card border-b border-border px-2 py-1">
        <h3 className="text-[10px] font-semibold text-foreground uppercase tracking-wide mb-1">Debug</h3>

        {/* Test Buttons */}
        <div className="flex gap-1 mb-1">
          <button
            onClick={handleAddNewFile}
            className="flex items-center gap-1 px-1.5 py-0.5 text-[9px] bg-green-600 hover:bg-green-700 text-white rounded transition-colors"
            title="Add new file with CheckAvailability & CountItems functions"
          >
            <Plus className="w-2.5 h-2.5" />
            <span>Add File</span>
          </button>
          <button
            onClick={handleModifyFunction}
            className="flex items-center gap-1 px-1.5 py-0.5 text-[9px] bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors"
            title="Modify ExtractResume function prompt"
          >
            <Edit className="w-2.5 h-2.5" />
            <span>Edit Func</span>
          </button>
        </div>
      </div>

      {/* File List */}
      <div className="p-1">
        {bamlFiles.length === 0 ? (
          <div className="text-[10px] text-muted-foreground p-2 text-center">
            No BAML files loaded yet
          </div>
        ) : (
          bamlFiles.map((file) => {
            const isExpanded = expandedFiles.has(file.path);
            return (
              <div key={file.path} className="mb-1">
                {/* File Header */}
                <button
                  onClick={() => toggleFile(file.path)}
                  className="w-full flex items-center gap-1 px-1 py-0.5 text-[10px] hover:bg-muted/50 rounded transition-colors"
                >
                  {isExpanded ? (
                    <ChevronDown className="w-3 h-3 text-muted-foreground" />
                  ) : (
                    <ChevronRight className="w-3 h-3 text-muted-foreground" />
                  )}
                  {isExpanded ? (
                    <FolderOpen className="w-3 h-3 text-blue-500" />
                  ) : (
                    <Folder className="w-3 h-3 text-blue-500" />
                  )}
                  <span className="font-medium text-foreground truncate">{file.path}</span>
                </button>

                {/* File Contents */}
                {isExpanded && (
                  <div className="ml-4 mt-0.5 space-y-1">
                    {/* Functions Section - exclude workflow functions */}
                    {file.functions.filter((func: BAMLFunction) => func.type !== 'workflow').length > 0 && (
                      <div>
                        <div className="text-[9px] font-semibold text-muted-foreground uppercase tracking-wide px-1 py-0.5">
                          Functions
                        </div>
                        {file.functions.filter((func: BAMLFunction) => func.type !== 'workflow').map((func: BAMLFunction) => (
                          <button
                            key={func.name}
                            onClick={() => handleFunctionClick(func)}
                            className={`w-full flex items-center gap-1 px-1 py-0.5 text-[10px] hover:bg-muted/50 rounded transition-colors ${isActive(func, 'function') ? 'bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300' : ''
                              }`}
                          >
                            <FileCode className="w-3 h-3 text-muted-foreground" />
                            <span className="truncate">{func.name}</span>
                            {func.type === 'llm_function' && (
                              <span className="ml-auto text-[8px] px-1 py-0.5 bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300 rounded">
                                LLM
                              </span>
                            )}
                          </button>
                        ))}
                      </div>
                    )}

                    {/* Tests Section */}
                    {file.tests.length > 0 && (
                      <div>
                        <div className="text-[9px] font-semibold text-muted-foreground uppercase tracking-wide px-1 py-0.5">
                          Tests
                        </div>
                        {file.tests.map((test: BAMLTest) => (
                          <div
                            key={test.name}
                            onClick={() => handleTestClick(test)}
                            className={`w-full flex items-center gap-1 px-1 py-0.5 text-[10px] hover:bg-muted/50 rounded transition-colors group cursor-pointer ${isActive(test, 'test') ? 'bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300' : ''
                              }`}
                          >
                            <Play className="w-3 h-3 text-muted-foreground" />
                            <span className="truncate flex-1 text-left">{test.name}</span>
                            <button
                              onClick={(e) => handleTestRun(test, e)}
                              className="opacity-0 group-hover:opacity-100 p-0.5 bg-green-600 hover:bg-green-700 text-white rounded transition-opacity"
                              title={`Run test for ${test.functionName}`}
                            >
                              <Play className="w-2.5 h-2.5" />
                            </button>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>

      {/* Active Event Display */}
      {activeCodeClick && (
        <div className="sticky bottom-0 bg-card border-t border-border px-2 py-1">
          <div className="text-[9px] font-semibold text-muted-foreground mb-0.5">Active:</div>
          <div className="text-[10px] font-mono text-foreground truncate">
            {activeCodeClick.type === 'function'
              ? `${activeCodeClick.functionName} (${activeCodeClick.functionType})`
              : `${activeCodeClick.testName} → ${activeCodeClick.functionName}`
            }
          </div>
        </div>
      )}
    </div>
  );
}
