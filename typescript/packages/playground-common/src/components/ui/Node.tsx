import type React from 'react';

interface NodeProps {
  children?: React.ReactNode;
  className?: string;
}

const Node: React.FC<NodeProps> = ({ children, className }) => {
  return (
    <div className={className}>
      {children}
    </div>
  );
};

export default Node;