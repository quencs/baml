// Refactored Playground Common - Clean Barrel Exports

// Contexts
export { RuntimeProvider, useRuntime, useWasm, useProject, useRuntimeInstance, useDiagnostics, useFiles, useEnvVars } from './contexts/runtime-context'
export { TestProvider, useTest, type TestExecution, type TestHistoryRun } from './contexts/test-context'

// Hooks
export { useVSCode, useVSCodeMessageHandler, useVSCodeFileHandler, useVSCodeTestHandler, useVSCodeSelectionHandler, useVSCodeConfigHandler, useVSCodeFlashingHandler } from './hooks/use-vscode'
export { useTestRunner, useTestSelection, useTestHistory, useTestConfig, type TestInput } from './hooks/use-test-runner'

// Services  
export { TestService, type TestCase, type TestResult } from './services/test-service'

// Components
export { AppRoot } from './components/app-root'
export { VSCodeHandler } from './components/vscode-handler'
export { ErrorBoundary } from './components/error-boundary'
export { ErrorCount } from './components/error-count'
export { VersionDisplay } from './components/version-display'
export { TestPanel } from './components/test-panel-refactored'

// Types
export type * from './types'

// Note: The moved components will need to be refactored to use named exports
// For now, they can be imported directly from their paths when needed
// export { TestTabularView } from './components/test-tabular-view'
// export { TestMenu } from './components/test-menu'
// export { TestStatus } from './components/test-status'
// export { ViewSelector } from './components/view-selector'

// Utils (if any)
// export * from './utils'