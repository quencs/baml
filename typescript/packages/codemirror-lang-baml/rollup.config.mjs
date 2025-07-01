import { lezer } from '@lezer/generator/rollup';
import typescript from '@rollup/plugin-typescript';
import { createRequire } from 'module';

// Create require function for ES modules
const require = createRequire(import.meta.url);

export default {
  input: 'src/index.ts',
  external: (id) => id != 'tslib' && !/^(\.?\/|\w:)/.test(id),
  output: [
    { file: 'dist/index.cjs', format: 'cjs', exports: 'named' },
    { file: 'dist/index.js', format: 'es' },
  ],
  plugins: [
    lezer(),
    typescript({
      tsconfig: './tsconfig.json',
      declaration: true,
      declarationMap: true,
      // Remove problematic outDir - let TypeScript handle it
      module: 'ESNext',
      target: 'ES2020',
      moduleResolution: 'node',
      allowSyntheticDefaultImports: true,
      esModuleInterop: true,
      isolatedModules: true,
      skipLibCheck: true,
      // Use the proper TypeScript instance
      typescript: require('typescript')
    })
  ],
};