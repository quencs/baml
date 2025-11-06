/**
 * Cursor-to-CodeClick Enrichment
 *
 * Transforms cursor positions into rich semantic CodeClickEvents by introspecting the WASM runtime.
 * This enables unified handling of both cursor movements and explicit code clicks.
 *
 * Source: graphs-project-docs/implementation/06-cursor-enrichment.md
 */

import type { WasmRuntime } from '@gloo-ai/baml-schema-wasm-web/baml_schema_build';
import type { CodeClickEvent } from './ui.atoms';

/**
 * Calculate byte index from line/column position in file content
 */
export function calculateByteIndex(
  fileContent: string,
  line: number,
  column: number
): number {
  const lines = fileContent.split('\n');

  let cursorIdx = 0;
  for (let i = 0; i < line; i++) {
    cursorIdx += (lines[i]?.length ?? 0) + 1; // +1 for the newline character
  }
  cursorIdx += column;

  return cursorIdx;
}

/**
 * Determine function type from WASM function object
 *
 * WASM runtime provides function metadata that includes type information.
 * This helper extracts and normalizes it to our CodeClickEvent types.
 */
export function determineFunctionType(
  wasmFunction: any
): 'workflow' | 'function' | 'llm_function' {
  // Check WASM function metadata for type
  // The exact property name depends on WASM implementation
  const fnType = wasmFunction.fn_type || wasmFunction.type || wasmFunction.kind;

  if (fnType === 'workflow' || fnType === 'Workflow') {
    return 'workflow';
  }

  if (fnType === 'llm' || fnType === 'llm_function' || fnType === 'LLMFunction') {
    return 'llm_function';
  }

  return 'function';
}

/**
 * Determine node type for test cases
 *
 * Test cases can belong to either llm_function or regular function nodes.
 */
export function determineFunctionNodeType(
  wasmFunction: any
): 'llm_function' | 'function' {
  const fnType = determineFunctionType(wasmFunction);

  // Workflows are treated as functions in node context
  if (fnType === 'llm_function') {
    return 'llm_function';
  }

  return 'function';
}

/**
 * Enrich cursor position into CodeClickEvent
 *
 * Uses WASM runtime introspection to extract semantic information about
 * what the cursor is pointing at (function definition, test case, or nothing).
 *
 * @param runtime - WASM runtime instance
 * @param fileName - File path
 * @param line - Line number (0-indexed)
 * @param column - Column number (0-indexed)
 * @param fileContent - Full file content
 * @param currentFunctionName - Currently selected function (for context)
 * @returns CodeClickEvent or null if cursor is not on a semantic element
 */
export function enrichCursorToCodeClick(
  runtime: WasmRuntime,
  fileName: string,
  line: number,
  column: number,
  fileContent: string,
  currentFunctionName?: string
): CodeClickEvent | null {
  // Calculate byte index from line/column
  const cursorIdx = calculateByteIndex(fileContent, line, column);

  // Try to get function at this position
  const selectedFunc = runtime.get_function_at_position(
    fileName,
    currentFunctionName ?? '',
    cursorIdx
  );

  if (!selectedFunc) {
    // Cursor is not in any function (whitespace, comments, etc.)
    return null;
  }

  // Check if cursor is in a test case within the function
  const selectedTestcase = runtime.get_testcase_from_position(selectedFunc, cursorIdx);

  if (selectedTestcase) {
    // Cursor is in a test case
    // Check if the test case references a nested function
    const nestedFunc = runtime.get_function_of_testcase(fileName, cursorIdx);

    if (nestedFunc) {
      // Test case is for a nested function
      return {
        type: 'test',
        testName: selectedTestcase.name,
        functionName: nestedFunc.name,
        filePath: fileName,
        nodeType: determineFunctionNodeType(nestedFunc),
      };
    }

    // Test case is for the current function
    return {
      type: 'test',
      testName: selectedTestcase.name,
      functionName: selectedFunc.name,
      filePath: fileName,
      nodeType: determineFunctionNodeType(selectedFunc),
    };
  }

  // Cursor is in a function definition (not in a test case)
  return {
    type: 'function',
    functionName: selectedFunc.name,
    functionType: determineFunctionType(selectedFunc),
    filePath: fileName,
  };
}
