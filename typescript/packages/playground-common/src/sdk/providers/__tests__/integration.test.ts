// /**
//  * Provider Integration Tests
//  *
//  * Validates that both MockProvider and VSCodeProvider work correctly with the SDK
//  */

// import { createStore } from 'jotai';
// import { createBAMLSDK } from '../../index';
// import { createMockProvider } from '../mock-provider';
// import { createVSCodeProvider } from '../vscode-provider';
// import { createDataProvider } from '../provider-factory';
// import { runtimeAtom, filesAtom } from '../../../shared/atoms';

// describe('Provider Integration', () => {
//   describe('MockProvider Integration', () => {
//     it('should create SDK with mock provider', async () => {
//       const store = createStore();
//       const provider = createMockProvider({ speedMultiplier: 0.1, errorRate: 0 });

//       const sdk = createBAMLSDK(
//         {
//           mode: 'mock',
//           provider,
//         },
//         store
//       );

//       await sdk.initialize();

//       const workflows = sdk.workflows.getAll();
//       expect(workflows).toBeDefined();
//       expect(workflows.length).toBeGreaterThan(0);
//     });

//     it('should execute workflow via mock provider', async () => {
//       const store = createStore();
//       const provider = createMockProvider({ speedMultiplier: 0.1, errorRate: 0 });

//       const sdk = createBAMLSDK(
//         {
//           mode: 'mock',
//           provider,
//         },
//         store
//       );

//       await sdk.initialize();

//       const workflows = sdk.workflows.getAll();
//       const workflowId = workflows[0]?.id;

//       expect(workflowId).toBeDefined();

//       // Start execution
//       const executionId = await sdk.executions.start(workflowId!, { input: 'test' });

//       expect(executionId).toBeDefined();
//       expect(executionId).toMatch(/^exec_/);
//     });

//     it('should get test cases via mock provider', async () => {
//       const store = createStore();
//       const provider = createMockProvider({ speedMultiplier: 0.1 });

//       const sdk = createBAMLSDK(
//         {
//           mode: 'mock',
//           provider,
//         },
//         store
//       );

//       await sdk.initialize();

//       const testCases = await sdk.testCases.get('simpleWorkflow', 'fetchData');

//       expect(testCases).toBeDefined();
//       expect(Array.isArray(testCases)).toBe(true);
//     });
//   });

//   describe('VSCodeProvider Integration', () => {
//     it('should create SDK with VSCode provider', async () => {
//       const store = createStore();

//       // Mock WASM runtime
//       store.set(runtimeAtom, {
//         rt: {
//           // Placeholder - real runtime would have these methods
//         } as any,
//         diags: undefined,
//         lastValidRt: undefined,
//       });

//       const provider = createVSCodeProvider(store);

//       const sdk = createBAMLSDK(
//         {
//           mode: 'vscode',
//           provider,
//         },
//         store
//       );

//       await sdk.initialize();

//       // Should not throw
//       expect(sdk).toBeDefined();
//     });

//     it('should handle missing runtime gracefully', async () => {
//       const store = createStore();

//       // No runtime set
//       const provider = createVSCodeProvider(store);

//       const sdk = createBAMLSDK(
//         {
//           mode: 'vscode',
//           provider,
//         },
//         store
//       );

//       await sdk.initialize();

//       const workflows = sdk.workflows.getAll();
//       expect(workflows).toEqual([]);
//     });

//     it('should watch files via VSCode provider', async () => {
//       const store = createStore();
//       const provider = createVSCodeProvider(store);

//       const sdk = createBAMLSDK(
//         {
//           mode: 'vscode',
//           provider,
//         },
//         store
//       );

//       await sdk.initialize();

//       // File watching should work
//       let callbackCalled = false;
//       const unsubscribe = (provider as any).watchFiles(() => {
//         callbackCalled = true;
//       });

//       // Update files atom
//       store.set(filesAtom, { 'test.baml': 'content' });

//       // Give it a moment to propagate
//       await new Promise((resolve) => setTimeout(resolve, 10));

//       expect(callbackCalled).toBe(true);

//       unsubscribe();
//     });
//   });

//   describe('Provider Factory', () => {
//     it('should create mock provider by default', () => {
//       const store = createStore();
//       const provider = createDataProvider({ mode: 'mock' }, store);

//       expect(provider).toBeDefined();
//       expect((provider as any).constructor.name).toContain('Mock');
//     });

//     it('should create VSCode provider when requested', () => {
//       const store = createStore();
//       const provider = createDataProvider({ mode: 'vscode' }, store);

//       expect(provider).toBeDefined();
//       expect((provider as any).constructor.name).toContain('VSCode');
//     });

//     it('should throw for unimplemented server mode', () => {
//       const store = createStore();

//       expect(() => {
//         createDataProvider({ mode: 'server' }, store);
//       }).toThrow('Server provider not implemented');
//     });
//   });

//   describe('Backward Compatibility', () => {
//     it('should work with legacy MockDataProvider', async () => {
//       const store = createStore();
//       const { DefaultMockProvider } = await import('../../mock');

//       const sdk = createBAMLSDK(
//         {
//           mode: 'mock',
//           mockData: new DefaultMockProvider({ speedMultiplier: 0.1 }),
//         },
//         store
//       );

//       await sdk.initialize();

//       const workflows = sdk.workflows.getAll();
//       expect(workflows).toBeDefined();
//       expect(workflows.length).toBeGreaterThan(0);
//     });
//   });
// });
