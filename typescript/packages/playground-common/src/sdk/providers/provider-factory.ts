/**
 * Provider Factory
 *
 * Selects and creates the appropriate data provider based on environment
 */

import type { createStore } from 'jotai';
import type { DataProvider } from './base';
import { createMockProvider } from './mock-provider';
import { createVSCodeProvider } from './vscode-provider';

/**
 * Provider mode
 */
export type ProviderMode = 'mock' | 'vscode' | 'server';

/**
 * Provider configuration
 */
export interface ProviderConfig {
  mode: ProviderMode;
  mockConfig?: {
    cacheHitRate?: number;
    errorRate?: number;
    verboseLogging?: boolean;
    speedMultiplier?: number;
  };
  serverUrl?: string;
}

/**
 * Auto-detect provider mode based on environment
 */
export function detectProviderMode(): ProviderMode {
  // Check if running in VSCode webview
  if (typeof (globalThis as any).acquireVsCodeApi === 'function') {
    return 'vscode';
  }

  // Check if server is available
  // TODO: Ping server to check availability

  // Default to mock mode
  return 'mock';
}

/**
 * Create data provider based on config
 */
export function createDataProvider(
  config: ProviderConfig,
  store?: ReturnType<typeof createStore>
): DataProvider {
  console.log('[ProviderFactory] Creating provider:', config.mode);

  switch (config.mode) {
    case 'mock':
      return createMockProvider(config.mockConfig);

    case 'vscode':
      if (!store) {
        throw new Error('Store required for VSCode provider');
      }
      return createVSCodeProvider(store);

    case 'server':
      // TODO: Implement server provider
      throw new Error('Server provider not implemented yet');

    default:
      throw new Error(`Unknown provider mode: ${config.mode}`);
  }
}

/**
 * Create provider with auto-detection
 */
export function createAutoProvider(store?: ReturnType<typeof createStore>): DataProvider {
  const mode = detectProviderMode();
  return createDataProvider({ mode }, store);
}
