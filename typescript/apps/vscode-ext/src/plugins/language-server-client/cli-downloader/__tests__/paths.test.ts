import { describe, it, expect, vi, beforeEach } from 'vitest';
import os from 'node:os';
import path from 'node:path';
import { type ExtensionContext } from 'vscode';
import {
  getInstallPath,
  cliBinaryArtifactName,
  downloadedCliPath,
  checkIfDownloadedCliExists,
  ensureInstallPathExists,
} from '../paths';

vi.mock('node:os');
vi.mock('node:fs');

describe('paths', () => {
  const mockContext = {} as ExtensionContext;

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getInstallPath', () => {
    it('should return ~/.baml path', () => {
      vi.mocked(os.homedir).mockReturnValue('/home/user');
      expect(getInstallPath(mockContext)).toBe('/home/user/.baml');
    });
  });

  describe('cliBinaryArtifactName', () => {
    it('should generate correct artifact name', () => {
      const cliVersion = {
        architecture: 'x64',
        platform: 'darwin',
        version: '1.2.3',
      };
      
      expect(cliBinaryArtifactName(cliVersion)).toBe(
        'baml-cli-1.2.3-x86_64-apple-darwin'
      );
    });

    it('should handle Windows platform', () => {
      const cliVersion = {
        architecture: 'x64',
        platform: 'win32',
        version: '2.0.0',
      };
      
      expect(cliBinaryArtifactName(cliVersion)).toBe(
        'baml-cli-2.0.0-x86_64-pc-windows-msvc'
      );
    });
  });

  describe('downloadedCliPath', () => {
    it('should generate correct path for macOS', () => {
      vi.mocked(os.homedir).mockReturnValue('/Users/test');
      vi.mocked(os.platform).mockReturnValue('darwin' as any);
      
      const cliVersion = {
        architecture: 'arm64',
        platform: 'darwin',
        version: '1.0.0',
      };
      
      const result = downloadedCliPath(mockContext, cliVersion);
      expect(result).toBe(
        '/Users/test/.baml/baml-cli-1.0.0-aarch64-apple-darwin-baml-cli'
      );
    });

    it('should generate correct path for Windows', () => {
      vi.mocked(os.homedir).mockReturnValue('C:\\Users\\test');
      vi.mocked(os.platform).mockReturnValue('win32' as any);
      
      const cliVersion = {
        architecture: 'x64',
        platform: 'win32',
        version: '1.0.0',
      };
      
      const result = downloadedCliPath(mockContext, cliVersion);
      expect(result).toBe(
        'C:\\Users\\test\\.baml\\baml-cli-1.0.0-x86_64-pc-windows-msvc-baml-cli.exe'
      );
    });
  });
});