/**
 * Data Providers - Central Export
 *
 * This file exports all provider-related types and factory functions
 */

// Base interface
export type { DataProvider, TestExecutionEvent, Diagnostic } from './base';

// Implementations
export { MockDataProvider, createMockProvider, createFastMockProvider } from './mock-provider';
export { VSCodeDataProvider, createVSCodeProvider } from './vscode-provider';

// Factory
export {
  createDataProvider,
  createAutoProvider,
  detectProviderMode,
  type ProviderMode,
  type ProviderConfig,
} from './provider-factory';
