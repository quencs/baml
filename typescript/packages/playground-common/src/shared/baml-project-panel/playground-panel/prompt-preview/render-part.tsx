import type {
  WasmChatMessagePart,
  WasmTestCase,
} from '@gloo-ai/baml-schema-wasm-web';
import he from 'he';
import { extractStringValues, getHighlightChunks } from './highlight-utils';
import { RenderPromptPart } from './render-text';
import { WebviewMedia } from './webview-media';

export const RenderPart: React.FC<{
  part: WasmChatMessagePart;
  testCase?: WasmTestCase;
  maxTextLength?: number;
}> = ({ part, testCase, maxTextLength = 20000 }) => {
  if (part.is_text()) {
    // this makes it so that we can escape html
    const text = he.encode(part.as_text() ?? '');
    // Skip processing if any input value is too large
    const hasLargeInput = (testCase?.inputs ?? []).some(
      (input) =>
        typeof input.value === 'string' && input.value.length > maxTextLength,
    );

    const allChunks = hasLargeInput
      ? []
      : extractStringValues(testCase?.inputs ?? []);
    console.log('Debug - Text:', text);
    console.log('Debug - All chunks:', allChunks);

    const highlightChunks = getHighlightChunks(text, allChunks);
    console.log('Debug - Final highlight chunks:', highlightChunks);

    return text ? (
      <RenderPromptPart text={text} highlightChunks={highlightChunks} />
    ) : null;
  }

  const media = part.as_media();
  if (!media) {
    return null;
  }

  if (part.is_image()) {
    return <WebviewMedia bamlMediaType="image" media={media} />;
  }

  if (part.is_audio()) {
    return <WebviewMedia bamlMediaType="audio" media={media} />;
  }

  return null;
};
