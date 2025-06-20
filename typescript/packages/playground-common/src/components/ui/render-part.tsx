import type React from 'react';

interface RenderPartProps {
  part: any;
  className?: string;
}

export const RenderPart: React.FC<RenderPartProps> = ({ part, className }) => {
  if (!part) {
    return null;
  }

  // Basic rendering - this would be more sophisticated in the actual implementation
  return (
    <div className={className}>
      {typeof part === 'string' ? part : JSON.stringify(part)}
    </div>
  );
};