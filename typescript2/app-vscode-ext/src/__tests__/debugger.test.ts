import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({
  DebugAdapterExecutable: class DebugAdapterExecutable {
    constructor(
      public readonly command: string,
      public readonly args: string[] = [],
    ) {}
  },
}));

import { buildBamlDebugAdapterCommand } from '../debugger';

describe('buildBamlDebugAdapterCommand', () => {
  it('launches baml-cli in dap mode', () => {
    expect(buildBamlDebugAdapterCommand('/custom/bin/baml-cli')).toEqual({
      command: '/custom/bin/baml-cli',
      args: ['dap'],
    });
  });
});

describe('package debugger contribution', () => {
  it('declares the baml debugger contribution', () => {
    const packageJsonPath = path.resolve(__dirname, '../../package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));

    const debuggerContribution = packageJson?.contributes?.debuggers?.find(
      (entry: { type?: string }) => entry?.type === 'baml',
    );

    expect(debuggerContribution).toBeDefined();
    expect(debuggerContribution.languages).toContain('baml');
    expect(debuggerContribution.initialConfigurations?.[0]?.type).toBe('baml');
    expect(
      packageJson?.contributes?.breakpoints?.some(
        (entry: { language?: string }) => entry?.language === 'baml',
      ),
    ).toBe(true);
  });
});
