// Central types file for playground-common

export interface TestCase {
	functionName: string;
	testName: string;
}

export interface TestResult {
	functionName: string;
	testName: string;
	status: 'running' | 'done' | 'error';
	response?: any;
	message?: string;
	latency?: number;
	timestamp: number;
}

export interface TestHistoryRun {
	id: string;
	timestamp: number;
	tests: TestResult[];
}

export interface FlashRange {
	filePath: string;
	startLine: number;
	startCol: number;
	endLine: number;
	endCol: number;
}

export interface Cursor {
	fileName: string;
	fileText: string;
	line: number;
	column: number;
}

export interface VSCodeMessage {
	command: string;
	content?: any;
}

export interface BamlConfig {
	enablePlaygroundProxy?: boolean;
	[key: string]: any;
}

export type ICodeBlock = {
	code: string;
	language: 'python' | 'typescript' | 'baml';
	id: string;
};

// Event message types for VSCode communication
export type VSCodeEventMessage =
	| {
		command: 'modify_file';
		content: {
			root_path: string;
			name: string;
			content: string | undefined;
		};
	}
	| {
		command: 'add_project';
		content: {
			root_path: string;
			files: Record<string, string>;
		};
	}
	| {
		command: 'remove_project';
		content: {
			root_path: string;
		};
	}
	| {
		command: 'set_flashing_regions';
		content: {
			spans: {
				file_path: string;
				start_line: number;
				start: number;
				end_line: number;
				end: number;
			}[];
		};
	}
	| {
		command: 'select_function';
		content: {
			root_path: string;
			function_name: string;
		};
	}
	| {
		command: 'update_cursor';
		content: {
			cursor: Cursor;
		};
	}
	| {
		command: 'port_number';
		content: {
			port: number;
		};
	}
	| {
		command: 'baml_cli_version';
		content: string;
	}
	| {
		command: 'baml_settings_updated';
		content: BamlConfig;
	}
	| {
		command: 'run_test';
		content: { test_name: string };
	};

// View types
export type TestViewType = 'simple' | 'tabular' | 'card';

// Error count interface
export interface ErrorCount {
	errors: number;
	warnings: number;
}

// Test status summary
export interface TestStatus {
	total: number;
	completed: number;
	running: number;
	errors: number;
}