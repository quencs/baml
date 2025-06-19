import React from 'react';

interface MarkdownRendererProps {
  source: string;
}

export const MarkdownRenderer: React.FC<MarkdownRendererProps> = ({ source }) => {
  // Simple markdown rendering - could be enhanced with a proper markdown library
  return (
    <div className="prose prose-sm max-w-none">
      <pre className="whitespace-pre-wrap break-words">{source}</pre>
    </div>
  );
};