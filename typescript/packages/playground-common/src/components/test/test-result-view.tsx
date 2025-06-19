import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@baml/ui/collapsible';
import { cn } from '@baml/ui/lib/utils';
import { ChevronDown, ChevronUp } from 'lucide-react';
import { useState } from 'react';
import type { TestHistoryRun } from '../types';
import { RenderPromptPart } from './render-text';

interface TestResultViewProps {
  currentRun?: TestHistoryRun;
}

interface TestResultMessageProps {
  functionName: string;
  testName: string;
  historicalResponse?: any;
}

const TestResultMessage: React.FC<TestResultMessageProps> = ({
  functionName,
  testName,
  historicalResponse,
}) => {
  const [open, setOpen] = useState(false);
  // Use the historical response directly since we're showing historical test results
  const displayResponse = historicalResponse;

  if (!displayResponse) {
    return null;
  }

  const hasError = displayResponse.status === 'error';
  const isRunning = displayResponse.status === 'running';
  const isDone = displayResponse.status === 'done';

  // Get the response content
  const getResponseContent = () => {
    if (hasError) {
      return displayResponse.message || 'Unknown error';
    }

    if (displayResponse.response) {
      const llmResponse = displayResponse.response.llm_response();
      if (llmResponse) {
        return llmResponse.content;
      }
    }

    return isRunning ? 'Running...' : 'No response';
  };

  const responseContent = getResponseContent();
  const firstLine = responseContent.split('\n')[0].slice(0, 100) + (responseContent.length > 100 ? '...' : '');

  // Status indicator color
  const getBorderColor = () => {
    if (hasError) return 'border-[var(--vscode-charts-red)]';
    if (isRunning) return 'border-[var(--vscode-charts-yellow)]';
    if (isDone) return 'border-[var(--vscode-charts-green)]';
    return 'border-[var(--vscode-charts-gray)]';
  };

  const getStatusText = () => {
    if (hasError) return 'Error';
    if (isRunning) return 'Running';
    if (isDone) return 'Response';
    return 'Test Result';
  };

  return (
    <div className={cn('border-l-4 pl-2 rounded mb-4', getBorderColor())}>
      <Collapsible open={open} onOpenChange={setOpen}>
        <CollapsibleTrigger
          className={cn(
            'flex w-full items-center justify-between p-3 transition-colors',
            'data-[state=closed]:bg-card rounded-t data-[state=closed]:hover:bg-card/80 cursor-pointer data-[state=open]:hover:bg-card/80',
          )}
        >
          <div className="flex flex-col items-start gap-1 flex-1 overflow-hidden min-w-0">
            <div className="flex items-center w-full justify-between">
              <div className="flex items-center gap-2">
                <div className="text-xs text-muted-foreground">
                  {getStatusText()}
                </div>
                <div className="text-xs font-mono text-muted-foreground/70">
                  {functionName}.{testName}
                </div>
              </div>
              {open ? (
                <ChevronUp className="size-4 ml-4 flex-shrink-0" />
              ) : (
                <ChevronDown className="size-4 ml-4 flex-shrink-0" />
              )}
            </div>
            {!open && firstLine && (
              <div className="text-sm truncate whitespace-nowrap w-full text-left">
                {firstLine}
              </div>
            )}
          </div>
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-3">
          <div className="p-0">
            {hasError ? (
              <div className="p-3 text-red-500 bg-red-50 dark:bg-red-950/20 rounded">
                <pre className="whitespace-pre-wrap text-xs">{responseContent}</pre>
              </div>
            ) : (
              <RenderPromptPart text={responseContent} />
            )}
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
};

export const TestResultView: React.FC<TestResultViewProps> = ({
  currentRun,
}) => {
  if (!currentRun || !currentRun.tests.length) {
    return (
      <div className="p-4 text-center text-muted-foreground">
        No test results to display
      </div>
    );
  }

  return (
    <div className="space-y-0">
      {currentRun.tests.map((test, index) => (
        <TestResultMessage
          key={`${test.functionName}-${test.testName}-${index}`}
          functionName={test.functionName}
          testName={test.testName}
          historicalResponse={test.response}
        />
      ))}
    </div>
  );
};

// Also export with the old name for backward compatibility
export const SimpleTestResultView = TestResultView;