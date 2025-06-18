'use client';

import CustomErrorBoundary from '../utils/ErrorFallback';
import { VSCodeHandler } from './vscode-handler';
import { RuntimeInitializer } from './runtime-initializer';
import { StatusBar } from './status-bar';

interface AppRootProps {
  children: React.ReactNode;
}

export function AppRoot({ children }: AppRootProps) {
  return (
    <>
      {/* Side effect components - no UI, just effects */}
      <VSCodeHandler />
      <RuntimeInitializer />
      
      {/* Status UI */}
      <StatusBar />
      
      {/* Main content with error boundary */}
      <CustomErrorBoundary message="Error loading project">
        {children}
      </CustomErrorBoundary>
    </>
  );
}