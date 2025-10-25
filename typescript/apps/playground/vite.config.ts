import { normalizePath } from 'vite'

import * as path from 'node:path';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import { viteStaticCopy } from 'vite-plugin-static-copy'
import { wasmHmr } from './plugins/vite-plugin-wasm-hmr';

const isWatchMode = process.argv.includes('--watch');
const srcPath = normalizePath(path.resolve(__dirname, './dist/'));
const destPath = normalizePath(path.resolve(__dirname, '../vscode-ext/dist/playground'));

// Path to WASM source and local copy
const wasmSourcePath = path.resolve(__dirname, '../../../engine/baml-schema-wasm/web/dist');
const wasmLocalPath = path.resolve(__dirname, './baml-schema-wasm-web/dist');

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react({
      babel: {
        presets: ['jotai/babel/preset'],
      },
    }),
    wasm(),
    wasmHmr({
      wasmPackagePath: path.resolve(__dirname, '../../../engine/baml-schema-wasm/web'),
      watchPath: 'dist',
      localCopyPath: wasmLocalPath,
    }),
    viteStaticCopy({
      targets: [
        {
          src: srcPath,
          dest: destPath
        },
        {
          src: normalizePath(wasmSourcePath) + '/*',
          dest: normalizePath(wasmLocalPath)
        }
      ]
    })
    // topLevelAwait(),
  ],
  // root: path.resolve(process.cwd(), './src'),
  server: {
    strictPort: true, // Allow fallback to next available port
    port: 3030,
    host: true,
    cors: {
      origin: '*',
    },
    headers: {
      'Access-Control-Allow-Origin': '*',
      // Prevent caching of WASM files during development
      // TODO: idk if this actually does anything.
      'Cache-Control': 'no-store',
    },
    hmr: {
      // This is needed for HMR to work in VSCode webviews
      protocol: 'ws',
      host: 'localhost',
    },
    watch: {
      usePolling: true,
      interval: 100,
      ignored: ['../../../engine/baml-schema-wasm/web/dist/**/*.wasm'],
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '~': path.resolve(__dirname, './src'),
      '@gloo-ai/baml-schema-wasm-web': wasmLocalPath,
      baml_wasm_web: wasmLocalPath,
    },
  },
  mode: isWatchMode ? 'development' : 'production',
  build: {
    target: 'esnext',
    minify: isWatchMode ? false : 'esbuild',
    sourcemap: isWatchMode ? 'inline' : false,
    rollupOptions: {
      external: ['baml_wasm_web/rpc'],
      output: {
        format: 'es',
        entryFileNames: 'assets/[name].js',
        chunkFileNames: 'assets/[name].js',
        assetFileNames: 'assets/[name].[ext]',
      },
    },
  },
  optimizeDeps: {
    esbuildOptions: {
      target: 'esnext',
    },

    // exclude: ['@gloo-ai/baml-schema-wasm-web'],
  },
});
