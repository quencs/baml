# Hot-Reload for Rust/WASM in Vite Playground

## Overview

This document outlines the architecture and implementation for hot-reloading Rust code changes in the BAML Playground Vite app. The goal is to automatically rebuild the WASM module when Rust source files change and display compilation errors in the browser UI.

### Current Setup

- **WASM Package**: `engine/baml-schema-wasm`
- **Build Command**: `pnpm build` from `baml-schema-wasm/web` (runs `wasm-pack build ../ --target bundler --out-dir ./web/dist --release`)
- **Vite App**: `typescript/apps/playground`
- **Current Plugin**: `vite-plugin-wasm` for loading WASM modules
- **Requirement**: Must use `--release` flag even in development for acceptable performance

### Goals

1. Auto-rebuild WASM when Rust files change
2. Display Cargo compilation errors in browser UI overlay
3. Fast feedback loop for development
4. Minimal dependencies (prefer direct wasm-pack over rsw-rs layer)

## Architecture Recommendation

### Recommended Approach: Custom Vite Plugin + Bacon

**Why not rsw-rs?**
- Adds an extra dependency layer between your build and wasm-pack
- You already have bacon configured (`baml-schema-wasm/bacon.toml`)
- rsw-rs is primarily useful for multi-crate monorepos with complex npm linking needs
- Direct wasm-pack integration is simpler and more maintainable for a single WASM package

**Why Bacon exports over stderr redirection?**
- Bacon's export system is designed for this use case
- Cleaner configuration with no shell scripting needed
- Automatically formats diagnostics in a structured format
- Works reliably across different platforms and shells
- Can be extended with custom export formats if needed

**Architecture Components:**

```
┌─────────────────┐
│  Rust Files     │
│  (.rs)          │
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│  Bacon Watch    │  ← Monitors .rs files
│  (background)   │    Runs wasm-pack on changes
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│  wasm-pack      │  ← Build with --release
│  build          │    Outputs to web/dist
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│  Bacon Exports  │  ← Writes diagnostics to
│  (auto)         │    .bacon-diagnostics file
└─────┬───────┬───┘
      │       │
      ↓       ↓
  dist/   .bacon-diagnostics
      │       │
      └───┬───┘
          ↓
┌─────────────────┐
│  Custom Vite    │  ← Watches both files:
│  Plugin         │    - dist/ for successful builds
│                 │    - .bacon-diagnostics for errors
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│  Browser        │  ← Reloads on success
│  (HMR/Reload)   │    Shows overlay on errors
└─────────────────┘
```

## Implementation Guide

### Step 1: Set Up Bacon for WASM Building

You already have a `build-wasm` job in `engine/baml-schema-wasm/bacon.toml`. Update it to use the release flag:

```toml
[jobs.build-wasm]
command = [
   "sh", "-c", "cd web && pnpm build --release"
]
need_stdout = true
allow_warnings = true
background = false
on_change_strategy = "kill_then_restart"
watch = ["src"]
ignore = ["web/dist", "target"]
```

Or create a dedicated watch job:

```toml
[jobs.watch-wasm]
command = [
   "sh", "-c", "cd web && wasm-pack build ../ --target bundler --out-dir ./web/dist --release"
]
need_stdout = true
allow_warnings = false  # We want to capture errors
background = false
on_change_strategy = "kill_then_restart"
watch = ["src"]
ignore = ["web/dist", "target"]
```

### Step 2: Create Custom Vite Plugin

Create a new plugin at `typescript/apps/playground/plugins/vite-plugin-wasm-hmr.ts`:

```typescript
import type { Plugin, ViteDevServer } from 'vite';
import { watch } from 'fs';
import { exec } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

interface WasmHmrOptions {
  /** Path to the WASM package web directory */
  wasmPackagePath: string;
  /** Directory to watch for changes (the dist output) */
  watchPath: string;
  /** Optional: bacon config path if you want to start bacon automatically */
  baconConfig?: string;
}

export function wasmHmr(options: WasmHmrOptions): Plugin {
  let server: ViteDevServer;
  let distWatcher: ReturnType<typeof watch> | null = null;
  let diagnosticsWatcher: ReturnType<typeof watch> | null = null;

  const wasmDistPath = path.resolve(options.wasmPackagePath, options.watchPath);
  const diagnosticsFile = path.resolve(options.wasmPackagePath, '.bacon-diagnostics');

  let lastBuildHadErrors = false;

  return {
    name: 'vite-plugin-wasm-hmr',
    enforce: 'pre',

    configureServer(_server) {
      server = _server;

      console.log('[wasm-hmr] Watching WASM output:', wasmDistPath);
      console.log('[wasm-hmr] Watching diagnostics:', diagnosticsFile);

      // Watch the bacon diagnostics file
      diagnosticsWatcher = watch(diagnosticsFile, async (eventType) => {
        if (eventType !== 'change') return;

        console.log('[wasm-hmr] Build completed, checking diagnostics...');

        // Read diagnostics file
        if (!fs.existsSync(diagnosticsFile)) {
          lastBuildHadErrors = false;
          return;
        }

        const diagnostics = fs.readFileSync(diagnosticsFile, 'utf-8');
        const hasErrors = diagnostics.split('\n').some(line => line.trim().startsWith('error'));

        if (hasErrors) {
          lastBuildHadErrors = true;
          console.log('[wasm-hmr] Build failed with errors');

          // Send error overlay to browser
          server.ws.send({
            type: 'error',
            err: {
              message: 'Rust compilation failed',
              stack: formatBaconDiagnostics(diagnostics),
              plugin: 'vite-plugin-wasm-hmr',
            },
          });
        } else {
          // No errors - if we previously had errors, clear them
          if (lastBuildHadErrors) {
            console.log('[wasm-hmr] Build succeeded, clearing previous errors');
            lastBuildHadErrors = false;
          }
          // Success will trigger reload when dist files change
        }
      });

      // Watch the WASM dist folder for changes
      distWatcher = watch(wasmDistPath, { recursive: true }, async (eventType, filename) => {
        if (!filename) return;

        // Only trigger on .js or .wasm file changes
        if (filename.endsWith('.js') || filename.endsWith('.wasm')) {
          console.log('[wasm-hmr] WASM package rebuilt:', filename);

          // Only reload if we don't have errors
          if (!lastBuildHadErrors) {
            server.ws.send({
              type: 'full-reload',
              path: '*',
            });
            console.log('[wasm-hmr] Triggering browser reload');
          }
        }
      });

      server.httpServer?.on('close', () => {
        distWatcher?.close();
        diagnosticsWatcher?.close();
      });
    },

    handleHotUpdate({ file }) {
      // If a .rs file changes, we don't handle it directly
      // (bacon handles the rebuild), but we can notify the user
      if (file.endsWith('.rs')) {
        console.log('[wasm-hmr] Rust file changed:', file);
        console.log('[wasm-hmr] Waiting for bacon to rebuild...');
      }
      return [];
    },
  };
}

/**
 * Format bacon diagnostics output for Vite's error overlay
 */
function formatBaconDiagnostics(diagnostics: string): string {
  const lines = diagnostics.split('\n').filter(line => line.trim());

  if (lines.length === 0) {
    return 'Build failed with unknown errors';
  }

  const formatted: string[] = ['Rust Compilation Errors:\n'];

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;

    // Parse bacon format: {kind} {path}:{line}:{column} {message}
    if (trimmed.startsWith('error')) {
      formatted.push(`\n❌ ${trimmed}\n`);
    } else if (trimmed.startsWith('warning')) {
      formatted.push(`\n⚠️  ${trimmed}\n`);
    } else {
      formatted.push(`   ${trimmed}`);
    }
  }

  return formatted.join('\n');
}
```

### Step 3: Configure Bacon to Export Diagnostics

Update your bacon config to use bacon's built-in exports feature to capture build diagnostics:

```toml
[jobs.watch-wasm-dev]
command = [
   "sh", "-c",
   "cd web && wasm-pack build ../ --target bundler --out-dir ./web/dist --release"
]
need_stdout = true
allow_warnings = true
background = false
on_change_strategy = "kill_then_restart"
watch = ["src"]
ignore = ["web/dist", "target"]

# Export build diagnostics for Vite plugin to display
[exports.wasm-build-status]
auto = true
path = "web/.bacon-diagnostics"
line_format = "{kind} {path}:{line}:{column} {message}"
```

This uses bacon's automatic exports to write diagnostics to a file that the Vite plugin watches.

### Step 4: Update Vite Config

Modify `typescript/apps/playground/vite.config.ts`:

```typescript
import { wasmHmr } from './plugins/vite-plugin-wasm-hmr';

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
    }),
    // ... other plugins
  ],
  // ... rest of config
});
```

### Step 5: Development Workflow

1. **Terminal 1** - Start Bacon watcher:
   ```bash
   cd engine/baml-schema-wasm
   bacon watch-wasm-dev
   ```

2. **Terminal 2** - Start Vite dev server:
   ```bash
   cd typescript/apps/playground
   pnpm dev
   ```

Now when you edit Rust files:
1. Bacon detects the change and triggers `wasm-pack build --release`
2. On success: New WASM files are written to `web/dist/`
3. Vite plugin detects dist changes and triggers browser reload
4. On error: Error is written to file, plugin reads it and shows overlay

## Alternative Approaches

### Option 2: Use rsw-rs (Not Recommended)

If you decide to use rsw-rs despite the extra layer:

1. Install rsw-rs globally: `cargo install rsw`
2. Install vite-plugin: `pnpm add -D vite-plugin-rsw`
3. Create `rsw.toml` in project root:
   ```toml
   [[crates]]
   name = "baml-schema-build"
   path = "engine/baml-schema-wasm"
   target = "bundler"
   out-dir = "engine/baml-schema-wasm/web/dist"
   profile = "release"  # Even for dev mode
   ```
4. Update vite.config.ts:
   ```typescript
   import ViteRsw from 'vite-plugin-rsw';

   export default defineConfig({
     plugins: [
       ViteRsw({
         crates: ["baml-schema-build"],
         profile: "release",
       }),
       // ... other plugins
     ],
   });
   ```

**Tradeoffs:**
- ✅ Automatic error overlay (built-in)
- ✅ No need to run bacon separately
- ❌ Extra dependency to maintain
- ❌ Less control over build process
- ❌ May not work well with existing bacon setup

### Option 3: Manual Watch Script

Create a Node.js watch script that directly watches Rust files:

```typescript
// scripts/watch-wasm.ts
import { watch } from 'chokidar';
import { exec } from 'child_process';
import debounce from 'lodash.debounce';

const watcher = watch('engine/baml-schema-wasm/src/**/*.rs', {
  ignored: /target/,
  persistent: true,
});

const rebuild = debounce(() => {
  console.log('Rebuilding WASM...');
  exec(
    'cd engine/baml-schema-wasm/web && pnpm build',
    (error, stdout, stderr) => {
      if (error) {
        console.error('Build failed:', stderr);
      } else {
        console.log('Build successful!');
      }
    }
  );
}, 300);

watcher.on('change', rebuild);
```

**Tradeoffs:**
- ✅ Simple, direct control
- ✅ No extra rust dependencies
- ❌ Manual error overlay implementation needed
- ❌ Another process to run
- ❌ Less robust than bacon

## Optimizations

### 1. Incremental Builds

Ensure cargo uses incremental compilation for faster rebuilds:

```toml
# In Cargo.toml or .cargo/config.toml
[profile.release]
incremental = true
```

### 2. Parallel Compilation

Set cargo to use more CPU cores:

```bash
# In your shell profile or .envrc
export CARGO_BUILD_JOBS=8
```

### 3. Cache wasm-pack artifacts

Make sure the `target/` directory is preserved between builds (not cleaned).

### 4. Debounce File Changes

The Vite plugin example above already handles rapid file changes, but you can adjust the debounce timing if needed.

## Troubleshooting

### WASM module fails to reload

**Symptom**: Changes to Rust code don't appear in browser even after rebuild.

**Solutions**:
1. Check browser console for WASM loading errors
2. Hard refresh (Cmd+Shift+R) to clear WASM cache
3. Verify dist files are actually updating: `ls -la engine/baml-schema-wasm/web/dist/`
4. Check that Vite alias points to correct dist folder (see vite.config.ts lines 57-64)

### Build errors not showing in overlay

**Symptom**: Rust compilation fails but no error overlay appears.

**Solutions**:
1. Check that error file is being written: `cat engine/baml-schema-wasm/target/wasm-build-error.txt`
2. Verify Vite plugin is loaded: check console for `[wasm-hmr]` messages
3. Check WebSocket connection in browser DevTools Network tab
4. Try sending a manual error via Vite's WebSocket to test overlay

### Slow rebuild times

**Symptom**: Each Rust change takes >30 seconds to rebuild.

**Solutions**:
1. Ensure incremental compilation is enabled (see Optimizations)
2. Use `--timings` flag to see where time is spent: `wasm-pack build --timings`
3. Consider using `--dev` for non-release builds (but note performance impact)
4. Check if you have enough RAM (WASM builds can be memory-intensive)
5. Use `sccache` or `cargo-chef` for better caching

### Bacon not detecting changes

**Symptom**: Editing Rust files doesn't trigger bacon rebuild.

**Solutions**:
1. Check `watch = ["src"]` paths in bacon.toml
2. Verify files aren't in `ignore` list
3. Try running `bacon --debug` to see what's being watched
4. Ensure your editor saves files properly (some save to temp files)

## Testing

### Test the Complete Flow

1. Start bacon: `cd engine/baml-schema-wasm && bacon watch-wasm-dev`
2. Start Vite: `cd typescript/apps/playground && pnpm dev`
3. Open browser to `http://localhost:3030`
4. Edit a Rust file in `engine/baml-schema-wasm/src/`
5. Verify:
   - [ ] Bacon detects change and starts rebuild
   - [ ] wasm-pack completes successfully
   - [ ] Vite plugin detects dist change
   - [ ] Browser automatically reloads
   - [ ] Changes appear in the app

### Test Error Handling

1. Introduce a syntax error in a Rust file (e.g., remove a semicolon)
2. Save the file
3. Verify:
   - [ ] Bacon shows compilation error
   - [ ] Error is written to file
   - [ ] Vite plugin reads error
   - [ ] Browser shows error overlay with formatted Rust error
   - [ ] Error overlay is readable and helpful

### Test Recovery

1. After introducing an error (above), fix it
2. Save the file
3. Verify:
   - [ ] Bacon rebuilds successfully
   - [ ] Error file is removed
   - [ ] Browser automatically reloads
   - [ ] Error overlay disappears
   - [ ] App works correctly

## Future Improvements

1. **Faster Builds**: Explore wasm-pack alternatives like `wasm-bindgen-cli` directly
2. **Granular HMR**: Investigate if partial WASM module reloading is possible
3. **Build Notifications**: Add desktop notifications for build completion/errors
4. **Metrics**: Track and display build times in the UI
5. **Auto-recovery**: Automatically retry failed builds on next file change
6. **Source Maps**: Improve Rust source map support for better debugging

## Resources

- [wasm-pack documentation](https://rustwasm.github.io/wasm-pack/)
- [Bacon configuration guide](https://dystroy.org/bacon/config/)
- [Vite Plugin API](https://vitejs.dev/guide/api-plugin.html)
- [Vite HMR API](https://vitejs.dev/guide/api-hmr.html)
- [rsw-rs repository](https://github.com/rwasm/rsw-rs) (for reference)
- [vite-plugin-rsw](https://github.com/rwasm/vite-plugin-rsw) (for reference)
