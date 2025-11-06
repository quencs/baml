/**
 * Compatibility layer for old atom imports
 *
 * This file re-exports atoms from the SDK to maintain backward compatibility
 * with existing code that imports from this location.
 *
 * Eventually, code should migrate to importing directly from the SDK.
 */

// Re-export all core atoms from SDK
export {
  // WASM & Runtime
  wasmAtom,
  filesAtom,
  runtimeAtom,
  ctxAtom,

  // Diagnostics
  diagnosticsAtom,
  lastValidRuntimeAtom,
  numErrorsAtom,

  // Generated Files
  generatedFilesAtom,
  generatedFilesByLangAtomFamily,

  // Environment & Settings
  envVarsAtom,
  featureFlagsAtom,
  betaFeatureEnabledAtom,
  vscodeSettingsAtom,
  proxyUrlAtom,

  // Files Tracking
  bamlFilesTrackedAtom,
  sandboxFilesTrackedAtom,

  // WASM Panic
  wasmPanicAtom,

  // Selection
  selectedFunctionNameAtom,
  selectedTestCaseNameAtom,
  selectedFunctionObjectAtom,
  selectedTestCaseAtom,
  selectionAtom,
  functionTestSnippetAtom,
} from '../../sdk/atoms/core.atoms';

// Re-export types
export type {
  WasmPanicState,
  DiagnosticError,
  GeneratedFile,
  VSCodeSettings,
} from '../../sdk/atoms/core.atoms';
