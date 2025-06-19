'use client';

import React from 'react';
import CustomErrorBoundary from '../../utils/ErrorFallback';
import { RuntimeInitializer } from './runtime-initializer';
import { StatusBar } from './status-bar';
import { VSCodeHandler } from './vscode-handler';

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
			<CustomErrorBoundary>
				{children}
			</CustomErrorBoundary>
		</>
	);
}