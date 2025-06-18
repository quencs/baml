'use client'

import React from 'react'
import { useTestHistory, useTestRunner } from '../hooks/use-test-runner'

interface TestPanelProps {
  className?: string;
}

export function TestPanel({ className }: TestPanelProps) {
  const { history, selectedIndex, currentRun } = useTestHistory();
  const { isRunning } = useTestRunner();

  if (!currentRun) {
    return (
      <div className={`flex items-center justify-center p-8 ${className}`}>
        <div className="text-center">
          <h3 className="text-lg font-medium text-gray-900 mb-2">
            No test results yet
          </h3>
          <p className="text-gray-500">
            Run some tests to see results here.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className={className}>
      <div className="border-b border-gray-200 pb-4 mb-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-medium text-gray-900">
            Test Results
          </h2>
          <div className="flex items-center gap-2">
            {isRunning && (
              <div className="inline-flex items-center gap-2 text-blue-600">
                <div className="w-4 h-4 border-2 border-blue-600 border-t-transparent rounded-full animate-spin" />
                Running...
              </div>
            )}
            <span className="text-sm text-gray-500">
              {new Date(currentRun.timestamp).toLocaleTimeString()}
            </span>
          </div>
        </div>
      </div>

      <div className="space-y-4">
        {currentRun.tests.map((test, index) => (
          <TestResultCard
            key={`${test.functionName}-${test.testName}-${index}`}
            test={test}
          />
        ))}
      </div>

      {history.length > 1 && (
        <div className="mt-6 pt-4 border-t border-gray-200">
          <h3 className="text-sm font-medium text-gray-700 mb-2">
            Test History ({history.length} runs)
          </h3>
          <div className="text-xs text-gray-500">
            Use the history selector to view previous test runs.
          </div>
        </div>
      )}
    </div>
  );
}

interface TestResultCardProps {
  test: {
    functionName: string;
    testName: string;
    status: 'queued' | 'running' | 'done' | 'error';
    response_status?: string;
    latency_ms?: number;
    message?: string;
  };
}

function TestResultCard({ test }: TestResultCardProps) {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'passed':
        return 'text-green-600 bg-green-50 border-green-200';
      case 'error':
      case 'llm_failed':
      case 'parse_failed':
        return 'text-red-600 bg-red-50 border-red-200';
      case 'running':
        return 'text-blue-600 bg-blue-50 border-blue-200';
      case 'queued':
        return 'text-yellow-600 bg-yellow-50 border-yellow-200';
      default:
        return 'text-gray-600 bg-gray-50 border-gray-200';
    }
  };

  const statusDisplay = test.status === 'done' ? test.response_status : test.status;
  const colorClasses = getStatusColor(statusDisplay || test.status);

  return (
    <div className={`border rounded-lg p-4 ${colorClasses}`}>
      <div className="flex items-center justify-between mb-2">
        <div>
          <h4 className="font-medium">
            {test.functionName} → {test.testName}
          </h4>
        </div>
        <div className="flex items-center gap-2">
          <span className="capitalize text-sm font-medium">
            {statusDisplay}
          </span>
          {test.latency_ms && (
            <span className="text-xs text-gray-500">
              {Math.round(test.latency_ms)}ms
            </span>
          )}
        </div>
      </div>
      
      {test.message && (
        <p className="text-sm mt-2">
          {test.message}
        </p>
      )}
      
      {test.status === 'running' && (
        <div className="mt-2">
          <div className="w-full bg-gray-200 rounded-full h-1">
            <div className="bg-blue-600 h-1 rounded-full animate-pulse w-1/3"></div>
          </div>
        </div>
      )}
    </div>
  );
}