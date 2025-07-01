import { lezer } from '@lezer/generator/rollup';
import typescript from '@rollup/plugin-typescript';

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
      outDir: 'dist',
      module: 'ESNext',
      target: 'ES2020',
      moduleResolution: 'node'
    })
  ],
};
