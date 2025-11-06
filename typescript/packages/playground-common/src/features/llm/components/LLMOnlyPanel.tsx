/**
 * LLM Only Panel Component
 *
 * Displays a focused view for LLM functions, showing prompt preview
 * instead of the full workflow graph.
 *
 * This uses the playground-common's PromptPreview component.
 */

import { PromptPreview } from '../../../shared/baml-project-panel/playground-panel/prompt-preview';

export function LLMOnlyPanel() {
  return <PromptPreview />;
}
