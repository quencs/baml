'use client'

import React, { ReactNode, useState } from 'react'
import { RuntimeProvider } from '../contexts/runtime-context'
import { TestProvider } from '../contexts/test-context'
import { VSCodeHandler } from './vscode-handler'
import { ErrorCount } from './error-count'
import { VersionDisplay } from './version-display'
import { ErrorBoundary } from './error-boundary'

interface AppRootProps {
  children: ReactNode;
}

export function AppRoot({ children }: AppRootProps) {
  const [bamlCliVersion, setBamlCliVersion] = useState<string | null>(null);
  const [bamlConfig, setBamlConfig] = useState<any>(null);

  return (
    <RuntimeProvider>
      <TestProvider>
        <ErrorBoundary message="Error loading project">
          {/* VSCode integration - pure side effects */}
          <VSCodeHandler
            onConfigUpdate={setBamlConfig}
            onVersionUpdate={setBamlCliVersion}
          />
          
          {/* Main application content */}
          {children}
          
          {/* Status overlay */}
          <div className='flex absolute right-2 bottom-2 z-50 flex-row gap-2 text-xs bg-transparent'>
            {bamlCliVersion && (
              <div className='pr-4 whitespace-nowrap'>
                baml-cli {bamlCliVersion}
              </div>
            )}
            <ErrorCount />
            <VersionDisplay />
          </div>
        </ErrorBoundary>
      </TestProvider>
    </RuntimeProvider>
  );
}