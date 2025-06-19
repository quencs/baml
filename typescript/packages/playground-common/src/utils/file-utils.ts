// Utility functions for file operations

export function filterBamlFiles(files: Record<string, string>): [string, string][] {
	return Object.entries(files).filter(([path]) => path.endsWith('.baml'));
}

export function isValidFileName(name: string): boolean {
	return name.length > 0 && !name.includes('..') && name !== '.' && name !== '/';
}

export function getFileExtension(fileName: string): string {
	const parts = fileName.split('.');
	return parts.length > 1 ? (parts[parts.length - 1] ?? '') : '';
}

export function createFileMap(entries: [string, string][]): Record<string, string> {
	return Object.fromEntries(entries);
}

export function normalizeFilePath(path: string): string {
	return path.replace(/\\/g, '/').replace(/\/+/g, '/');
}

export function getDirectoryFromPath(filePath: string): string {
	const normalized = normalizeFilePath(filePath);
	const lastSlash = normalized.lastIndexOf('/');
	return lastSlash > 0 ? normalized.substring(0, lastSlash) : '';
}

export function getFileNameFromPath(filePath: string): string {
	const normalized = normalizeFilePath(filePath);
	const lastSlash = normalized.lastIndexOf('/');
	return lastSlash >= 0 ? normalized.substring(lastSlash + 1) : normalized;
}