import { atom } from 'jotai';
import type React from 'react';

// Export the showTokenCountsAtom that was being imported
export const showTokenCountsAtom = atom<boolean>(false);

interface RenderTextProps {
  text: string;
  showTokens?: boolean;
  className?: string;
}

export const RenderText: React.FC<RenderTextProps> = ({
  text,
  showTokens = false,
  className
}) => {
  return (
    <div className={className}>
      <pre className="whitespace-pre-wrap break-words">{text}</pre>
      {showTokens && (
        <div className="text-xs text-muted-foreground mt-1">
          Tokens: {text.split(/\s+/).length}
        </div>
      )}
    </div>
  );
};