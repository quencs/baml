import { defineConfig } from 'vitest/config';
import path from 'node:path';

const engineRoot = path.resolve(__dirname, '../../engine/language_client_typescript');

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    testTimeout: 60_000,
    hookTimeout: 60_000,
    maxConcurrency: 1,
    isolate: false,
    poolOptions: {
      threads: {
        singleThread: true,
      },
    },
  },
  resolve: {
    alias: {
      '@boundaryml/baml': path.join(engineRoot, 'index.js'),
      '@boundaryml/baml/native': path.join(engineRoot, 'native.js'),
      '@boundaryml/baml/type_builder': path.join(engineRoot, 'type_builder.js'),
      '@boundaryml/baml/logging': path.join(engineRoot, 'logging.js'),
    },
  },
});
