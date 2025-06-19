import React from 'react';

interface ParsedResponseRendererProps {
  response: any;
}

export const ParsedResponseRenderer: React.FC<ParsedResponseRendererProps> = ({ response }) => {
  if (!response) {
    return <div className="text-muted-foreground">No response</div>;
  }

  try {
    const displayValue = typeof response === 'string' ? response : JSON.stringify(response, null, 2);
    return (
      <pre className="text-xs whitespace-pre-wrap break-words bg-muted/50 p-2 rounded">
        {displayValue}
      </pre>
    );
  } catch (error) {
    return <div className="text-red-500">Error rendering response</div>;
  }
};