// Utility functions for highlighting and text processing

export function getFirstLine(text: string): string {
  if (!text) return '';
  const lines = text.split('\n');
  return lines[0] || '';
}