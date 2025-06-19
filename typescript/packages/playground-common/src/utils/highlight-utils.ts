import type { WasmParam } from '@gloo-ai/baml-schema-wasm-web';
import he from 'he';

/**
 * Extracts string values from WasmParam inputs, handling JSON and plain strings.
 */
export const extractStringValues = (inputs: WasmParam[]): string[] => {
  if (!inputs || !Array.isArray(inputs)) return [];
  return inputs.flatMap((input) => {
    if (typeof input.value === 'string') {
      try {
        // Try to parse the string as JSON
        const parsed = JSON.parse(input.value);
        if (typeof parsed === 'object') {
          return Object.values(parsed).filter(
            (val): val is string => typeof val === 'string',
          );
        } else {
          // Split the string into individual phrases if it contains repeated text
          const phrases = parsed.split(/\s{2,}/).filter(Boolean);
          return phrases.map((phrase: string) => he.encode(phrase.trim()));
        }
      } catch {
        // If parsing fails, treat it as a regular string
        // Split the string into individual phrases if it contains repeated text
        const phrases = input.value.split(/\s{2,}/).filter(Boolean);
        return phrases.map((phrase: string) => he.encode(phrase.trim()));
      }
    }
    if (typeof input.value === 'object') {
      return Object.values(input.value).filter(
        (val): val is string => typeof val === 'string',
      );
    }
    return [];
  });
};

/**
 * Returns the list of highlight chunks that appear in the text.
 * Escapes regex characters and checks for matches.
 */
export const getHighlightChunks = (
  text: string,
  allChunks: string[],
): string[] => {
  return allChunks.filter((chunk) => {
    if (!chunk || !text) return false;
    try {
      // Escape special regex characters in the chunk
      const escapedChunk = chunk.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      // Use unicode flag to handle emojis correctly
      const regex = new RegExp(escapedChunk, 'gu');
      const matches = text.match(regex);
      // Include chunks that appear at least once in the text
      return matches && matches.length > 0;
    } catch (e) {
      return false;
    }
  });
};

/**
 * Returns the first non-empty line from the first text part in the array.
 */
export function getFirstLine(text: string): string {
  if (!text) return '';
  const lines = text.split('\n');
  return lines[0] || '';
}

/**
 * Splits text into parts, marking which parts should be highlighted based on highlightChunks.
 * Uses whitespace-tolerant regex logic.
 */
export function getHighlightedParts(
  text: string,
  highlightChunks: string[],
): Array<{ text: string; highlight: boolean }> {
  if (!highlightChunks?.length) return [{ text, highlight: false }];

  const regex = new RegExp(
    highlightChunks
      .filter(Boolean)
      .sort((a, b) => b.length - a.length)
      .map((chunk) =>
        chunk.replace(/[.*+?^${}()|[\\]\\]/g, '\\$&').replace(/ /g, '\\s+'),
      )
      .join('|'),
    'gms',
  );

  const parts: Array<{ text: string; highlight: boolean }> = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push({
        text: text.slice(lastIndex, match.index),
        highlight: false,
      });
    }
    parts.push({ text: match[0], highlight: true });
    lastIndex = regex.lastIndex;
  }
  if (lastIndex < text.length) {
    parts.push({ text: text.slice(lastIndex), highlight: false });
  }
  return parts;
}

// Mock WasmParam type to avoid WASM dependency
export interface WasmParam {
  name: string;
  value: any;
}

export function highlightParams(params: WasmParam[]): string {
  return params.map(p => `${p.name}: ${p.value}`).join(', ');
}
