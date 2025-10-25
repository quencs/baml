import type { Plugin, ViteDevServer } from 'vite';
import { watch } from 'fs';
import * as path from 'path';
import * as fs from 'fs';

interface WasmHmrOptions {
  /** Path to the WASM package web directory */
  wasmPackagePath: string;
  /** Directory to watch for changes (the dist output) */
  watchPath: string;
  /** Local path to copy WASM files to for HMR */
  localCopyPath?: string;
}

export function wasmHmr(options: WasmHmrOptions): Plugin {
  let server: ViteDevServer;
  let diagnosticsWatcher: ReturnType<typeof watch> | null = null;

  const diagnosticsFile = path.resolve(options.wasmPackagePath, '.wasm-build-status');

  let debounceTimer: NodeJS.Timeout | null = null;
  const DEBOUNCE_MS = 3000;

  return {
    name: 'vite-plugin-wasm-hmr',
    enforce: 'pre',

    configureServer(_server) {
      server = _server;

      console.log('[wasm-hmr] Watching diagnostics:', diagnosticsFile);

      // Watch the bacon diagnostics file
      diagnosticsWatcher = watch(diagnosticsFile, async (eventType) => {
        if (eventType !== 'change') return;

        // Debounce: only process if no more changes in 100ms
        if (debounceTimer) {
          clearTimeout(debounceTimer);
        }

        debounceTimer = setTimeout(() => {
          processDiagnostics();
        }, DEBOUNCE_MS);
      });

      const processDiagnostics = () => {
        console.log('[wasm-hmr] Processing diagnostics...');

        // Read diagnostics file
        if (!fs.existsSync(diagnosticsFile)) {
          return;
        }

        const diagnostics = fs.readFileSync(diagnosticsFile, 'utf-8').trim();

        // Handle refreshing state
        if (diagnostics === 'refreshing') {
          console.log('[wasm-hmr] Build in progress...');
          server.ws.send({
            type: 'custom',
            event: 'wasm-build-status',
            data: { status: 'refreshing' },
          });
          // server.ws.send({
          //   type: 'full-reload',
          //   path: '*',

          // })
          return;
        }

        // Handle cancelled state (build was interrupted)
        if (diagnostics === 'cancelled') {
          console.log('[wasm-hmr] Build cancelled (likely restarted)');
          // Don't send anything to browser, just wait for next build
          return;
        }

        // Handle success state
        if (diagnostics === 'success' || diagnostics === '') {
          console.log('[wasm-hmr] Build succeeded, waiting for file system sync...');

          // Small delay to ensure files are fully written and flushed to disk
          // before triggering reload
          setTimeout(() => {
            // Copy WASM files to local path if configured
            if (options.localCopyPath) {
              const sourcePath = path.resolve(options.wasmPackagePath, options.watchPath);
              const destPath = options.localCopyPath;

              console.log('[wasm-hmr] Copying WASM files from', sourcePath, 'to', destPath);

              try {
                // Ensure destination directory exists
                if (!fs.existsSync(destPath)) {
                  fs.mkdirSync(destPath, { recursive: true });
                }

                // Copy all files from source to destination
                const files = fs.readdirSync(sourcePath);
                for (const file of files) {
                  const srcFile = path.join(sourcePath, file);
                  const destFile = path.join(destPath, file);

                  if (fs.statSync(srcFile).isFile()) {
                    fs.copyFileSync(srcFile, destFile);
                    console.log('[wasm-hmr] Copied', file);
                  }
                }
              } catch (error) {
                console.error('[wasm-hmr] Error copying WASM files:', error);
              }
            }

            console.log('[wasm-hmr] Triggering WASM reload');
            server.ws.send({
              type: 'custom',
              event: 'wasm-hard-reload',
              data: { timestamp: Date.now() },
            });
          }, 500);

          return;
        }

        // Handle error state
        const hasErrors = diagnostics.split('\n').some(line =>
          line.trim().toLowerCase().includes('error')
        );

        if (hasErrors) {
          console.log('[wasm-hmr] Build failed with errors');
          server.ws.send({
            type: 'error',
            err: {
              message: 'Rust compilation failed',
              stack: formatBuildErrors(diagnostics),
              plugin: 'vite-plugin-wasm-hmr',
            },
          });
        } else {
          // Has content but no errors - probably warnings, treat as success
          console.log('[wasm-hmr] Build succeeded with warnings, triggering reload');
          // server.ws.send({
          //   type: 'full-reload',
          //   path: '*',
          // });
        }
      };

      server.httpServer?.on('close', () => {
        if (debounceTimer) {
          clearTimeout(debounceTimer);
        }
        diagnosticsWatcher?.close();
      });
    },

    // handleHotUpdate({ file }) {
    //   // If a .rs file changes, we don't handle it directly
    //   // (bacon handles the rebuild), but we can notify the user
    //   if (file.endsWith('.rs')) {
    //     console.log('[wasm-hmr] Rust file changed:', file);
    //     console.log('[wasm-hmr] Waiting for bacon to rebuild...');
    //   }
    //   return [];
    // },

    transformIndexHtml() {
      return [
        {
          tag: 'script',
          injectTo: 'head',
          attrs: {
            type: 'module',
          },
          children: `
if (import.meta.hot) {
  let buildStatusOverlay = null;

  import.meta.hot.on('wasm-build-status', (data) => {
    if (data.status === 'refreshing') {
      showBuildStatus('Rebuilding WASM...');
    }
  });

  import.meta.hot.on('wasm-hard-reload', async (data) => {
    console.log('[wasm-hmr] WASM rebuild complete, invalidating module...');
    showBuildStatus('WASM rebuilt, hot reloading...');

    // Invalidate the WASM module to force re-import with cache busting
    // const wasmModulePath = '@gloo-ai/baml-schema-wasm-web/baml_schema_build';

    // Use import.meta.hot.invalidate() to trigger HMR for this module
    // import.meta.hot.invalidate();
  });

  function showBuildStatus(message) {
    if (!buildStatusOverlay) {
      buildStatusOverlay = document.createElement('div');
      buildStatusOverlay.id = 'wasm-build-status';
      buildStatusOverlay.style.cssText = 'position:fixed;top:10px;right:10px;background:#1a1a1a;color:#4ade80;padding:8px 16px;border-radius:6px;font-family:monospace;font-size:12px;z-index:9999;box-shadow:0 2px 8px rgba(0,0,0,0.3);border:1px solid #4ade80;';
      document.body.appendChild(buildStatusOverlay);
    }
    buildStatusOverlay.textContent = message;
    buildStatusOverlay.style.display = 'block';
  }
}
          `,
        },
      ];
    },
  };
}

/**
 * Format build errors for Vite's error overlay
 */
function formatBuildErrors(diagnostics: string): string {
  const lines = diagnostics.split('\n').filter(line => line.trim());

  if (lines.length === 0) {
    return 'Build failed with unknown errors';
  }

  const formatted: string[] = ['Rust Compilation Failed:\n'];

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;

    // Highlight error lines
    if (trimmed.toLowerCase().includes('error[e') || trimmed.toLowerCase().includes('error:')) {
      formatted.push(`\n❌ ${trimmed}\n`);
    } else if (trimmed.includes('-->')) {
      formatted.push(`   ${trimmed}`);
    } else if (trimmed.startsWith('|')) {
      formatted.push(`   ${trimmed}`);
    } else if (trimmed.toLowerCase().includes('help:') || trimmed.toLowerCase().includes('note:')) {
      formatted.push(`   💡 ${trimmed}`);
    } else {
      formatted.push(`   ${trimmed}`);
    }
  }

  return formatted.join('\n');
}
