import * as vscode from 'vscode';

export type DebugAdapterCommand = {
  command: string;
  args: string[];
};

export function buildBamlDebugAdapterCommand(cliPath: string): DebugAdapterCommand {
  return {
    command: cliPath,
    args: ['dap'],
  };
}

export class BamlDebugAdapterFactory implements vscode.DebugAdapterDescriptorFactory {
  constructor(private readonly cliPath: string) {}

  createDebugAdapterDescriptor(
    _session: vscode.DebugSession,
    _executable: vscode.DebugAdapterExecutable | undefined,
  ): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
    const command = buildBamlDebugAdapterCommand(this.cliPath);
    return new vscode.DebugAdapterExecutable(command.command, command.args);
  }
}
