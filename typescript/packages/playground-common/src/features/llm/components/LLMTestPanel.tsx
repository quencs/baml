/**
 * LLM Test Panel Component
 *
 * Displays tests and test inputs for standalone LLM functions
 * that are not part of any workflow.
 *
 * This uses the playground-common's TestPanel component.
 */

import { TestPanel } from '../../../shared/baml-project-panel/playground-panel/prompt-preview/test-panel';

export function LLMTestPanel() {
  return <TestPanel />;
}
