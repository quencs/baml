import React from 'react';
import { Tree, type TreeApi } from 'react-arborist';
import Node from './Node';

export const data = [
  {
    id: 'prompt-engineering',
    icon: 'star',
    name: 'Prompt engineering',
    children: [{ id: 'system_user_prompts', name: 'Prompt roles' }],
  },
  {
    id: 'testing',
    icon: 'beakers',
    name: 'Testing',
    children: [
      { id: 'test_ai_function', name: 'Test an AI function' },
      { id: 'evaluate_results', name: 'Evaluate LLM results' },
    ],
  },
  {
    id: 'resilience_reliability',
    icon: 'shield',
    name: 'Resilence / Reliability',
    children: [
      { id: 'add_retries', name: 'Function retries' },
      { id: 'fall_back', name: 'Model fall-back' },
    ],
  },
  {
    id: 'streaming_dir',
    icon: 'waves',
    name: 'Streaming',
    children: [{ id: 'streaming_structured', name: 'Structured streaming' }],
  },
];

export const FileViewer = () => {
  const treeRef = React.useRef<TreeApi<any> | null>(null);

  return (
    <div className="flex flex-col w-full h-full overflow-hidden">
      <Tree
        ref={treeRef}
        openByDefault={true}
        data={data}
        rowHeight={32}
        className="tree-container"
      >
        {Node}
      </Tree>
    </div>
  );
};

export default FileViewer;
