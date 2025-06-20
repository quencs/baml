import type React from 'react';

interface PromptStatsProps {
  tokenCount?: number;
  characterCount?: number;
  className?: string;
}

export const PromptStats: React.FC<PromptStatsProps> = ({
  tokenCount,
  characterCount,
  className
}) => {
  return (
    <div className={className}>
      {tokenCount !== undefined && (
        <span className="text-xs text-muted-foreground">
          Tokens: {tokenCount}
        </span>
      )}
      {characterCount !== undefined && (
        <span className="text-xs text-muted-foreground ml-2">
          Characters: {characterCount}
        </span>
      )}
    </div>
  );
};