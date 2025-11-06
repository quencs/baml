/**
 * BAML SDK Provider for React
 *
 * Provides the SDK instance through React Context
 */

import { Provider as JotaiProvider, createStore } from 'jotai';
import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from 'react';
import { BAMLSDK, createBAMLSDK } from './index';
import { createMockSDKConfig } from './mock';
import type { BAMLSDKConfig } from './types';

const BAMLSDKContext = createContext<BAMLSDK | null>(null);

interface BAMLSDKProviderProps {
  children: ReactNode;
  config?: BAMLSDKConfig;
}

/**
 * Provider component that wraps the app and provides SDK access
 */
export function BAMLSDKProvider({ children, config }: BAMLSDKProviderProps) {
  // Create refs to ensure single instance creation
  const storeRef = useRef<ReturnType<typeof createStore> | undefined>(undefined);
  const sdkRef = useRef<BAMLSDK | undefined>(undefined);

  // Initialize store and SDK only once
  if (!storeRef.current) {
    storeRef.current = createStore();
  }

  if (!sdkRef.current) {
    const sdkConfig = config ?? createMockSDKConfig();
    console.log('🚀 Creating BAML SDK with config:', sdkConfig.mode);
    sdkRef.current = createBAMLSDK(sdkConfig, storeRef.current);
  }

  const [isInitialized, setIsInitialized] = useState(false);

  // Handle async initialization only once
  useEffect(() => {
    let mounted = true;

    async function init() {
      if (!sdkRef.current) return;

      console.log('⏳ Initializing SDK...');
      await sdkRef.current.initialize();
      if (mounted) {
        console.log('✅ SDK initialized successfully');
        const workflows = sdkRef.current.workflows.getAll();
        console.log('📦 Loaded workflows:', workflows.length, workflows.map(w => w.id));
        setIsInitialized(true);
      }
    }

    init();

    return () => {
      mounted = false;
    };
  }, []); // Empty deps - only run once

  // Show loading state while SDK initializes
  if (!isInitialized) {
    return (
      <div className="w-screen h-screen flex items-center justify-center bg-background">
        <div className="text-center">
          <div className="text-xl font-semibold">Loading BAML SDK...</div>
          <div className="text-sm text-muted-foreground mt-2">Initializing workflows</div>
        </div>
      </div>
    );
  }

  return (
    <BAMLSDKContext.Provider value={sdkRef.current}>
      <JotaiProvider store={storeRef.current}>{children}</JotaiProvider>
    </BAMLSDKContext.Provider>
  );
}

/**
 * Hook to access the SDK instance
 */
export function useBAMLSDK(): BAMLSDK {
  const sdk = useContext(BAMLSDKContext);
  if (!sdk) {
    throw new Error('useBAMLSDK must be used within BAMLSDKProvider');
  }
  return sdk;
}
