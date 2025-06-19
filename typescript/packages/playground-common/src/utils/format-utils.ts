// Utility functions for formatting and display

export function formatLatency(latencyMs: number): string {
	if (latencyMs < 1000) {
		return `${Math.round(latencyMs)}ms`;
	}
	return `${(latencyMs / 1000).toFixed(1)}s`;
}

export function formatTimestamp(timestamp: number): string {
	return new Date(timestamp).toLocaleTimeString();
}

export function formatFileSize(bytes: number): string {
	const units = ['B', 'KB', 'MB', 'GB'];
	let size = bytes;
	let unitIndex = 0;

	while (size >= 1024 && unitIndex < units.length - 1) {
		size /= 1024;
		unitIndex++;
	}

	return `${size.toFixed(1)} ${units[unitIndex]}`;
}

export function truncateString(str: string, maxLength: number): string {
	if (str.length <= maxLength) return str;
	return str.slice(0, maxLength - 3) + '...';
}

export function capitalizeFirst(str: string): string {
	if (str.length === 0) return str;
	return str.charAt(0).toUpperCase() + str.slice(1);
}

export function camelToKebab(str: string): string {
	return str.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
}

export function kebabToCamel(str: string): string {
	return str.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
}

export function pluralize(count: number, singular: string, plural?: string): string {
	if (count === 1) return `${count} ${singular}`;
	return `${count} ${plural || singular + 's'}`;
}