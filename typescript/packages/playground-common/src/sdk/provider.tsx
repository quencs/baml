/**
 * BAML SDK Provider for React - Refactored
 *
 * Provides the SDK instance through React Context
 * Uses the new runtime factory pattern
 */

import { Provider as JotaiProvider, createStore } from 'jotai';
import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from 'react';
import { BAMLSDK } from '.';
import { createMockSDK } from './factory';

const BAMLSDKContext = createContext<BAMLSDK | null>(null);

interface BAMLSDKProviderProps {
  children: ReactNode;
  mode?: 'mock';
}

/**
 * Provider component that wraps the app and provides SDK access
 */
export function BAMLSDKProvider({ children, mode = 'mock' }: BAMLSDKProviderProps) {
  // Create refs to ensure single instance creation
  const storeRef = useRef<ReturnType<typeof createStore> | undefined>(undefined);
  const sdkRef = useRef<BAMLSDK | undefined>(undefined);

  // Initialize store and SDK only once
  if (!storeRef.current) {
    storeRef.current = createStore();
  }

  if (!sdkRef.current) {
    console.log('🚀 Creating BAML SDK with mode:', mode);
    if (mode === 'mock') {
      sdkRef.current = createMockSDK(storeRef.current);
    } else {
      throw new Error(`Unsupported mode: ${mode}`);
    }
  }

  const [isInitialized, setIsInitialized] = useState(false);

  // Handle async initialization only once
  useEffect(() => {
    let mounted = true;

    async function init() {
      if (!sdkRef.current) return;

      console.log('⏳ Initializing SDK...');

      // Initialize with empty files for mock mode
      const initialFiles = {
        'workflows/simple.baml': '// Mock workflow file',
        'workflows/conditional.baml': '// Mock conditional workflow',
      };

      await sdkRef.current.initialize(initialFiles);

      if (mounted) {
        console.log('✅ SDK initialized successfully');
        const workflows = sdkRef.current.workflows.getAll();
        console.log('📦 Loaded workflows:', workflows.length, workflows.map((w) => w.id));
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
