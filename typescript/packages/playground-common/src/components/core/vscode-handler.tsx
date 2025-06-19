'use client';

import { useDebounceCallback } from '@react-hook/debounce';
import { useAtom, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { type BamlConfigAtom, bamlConfig } from '../baml_wasm_web/bamlConfig';
import {
	filesAtom,
} from '../shared/baml-project-panel/atoms';
import {
	flashRangesAtom,
	selectedFunctionAtom,
	selectedTestcaseAtom,
	updateCursorAtom,
} from '../shared/baml-project-panel/playground-panel/atoms';
import { orchIndexAtom } from '../shared/baml-project-panel/playground-panel/atoms-orch-graph';
import { useRunBamlTests } from '../shared/baml-project-panel/playground-panel/prompt-preview/test-panel/test-runner';
import type { VSCodeEventMessage } from '../types';
import { bamlCliVersionAtom } from './event-listener';

export function VSCodeHandler() {
	const updateCursor = useSetAtom(updateCursorAtom);
	const setFiles = useSetAtom(filesAtom);
	const debouncedSetFiles = useDebounceCallback(setFiles, 50, true);
	const setFlashRanges = useSetAtom(flashRangesAtom);

	const [selectedFunc, setSelectedFunction] = useAtom(selectedFunctionAtom);
	const setSelectedTestcase = useSetAtom(selectedTestcaseAtom);
	const setBamlConfig = useSetAtom(bamlConfig);
	const setBamlCliVersion = useSetAtom(bamlCliVersionAtom);
	const runBamlTests = useRunBamlTests();
	const setOrchestratorIndex = useSetAtom(orchIndexAtom);

	// Reset orchestrator index when function changes
	useEffect(() => {
		if (selectedFunc) {
			setOrchestratorIndex(0);
		}
	}, [selectedFunc, setOrchestratorIndex]);

	useEffect(() => {
		console.log('Setting up VSCode message listener');

		const messageHandler = (event: MessageEvent<VSCodeEventMessage>) => {
			const { command, content } = event.data;
			console.log('VSCode command received:', command);

			switch (command) {
				case 'add_project':
					if (content && content.root_path) {
						console.log('Adding project:', content.root_path);
						debouncedSetFiles(
							Object.fromEntries(
								Object.entries(content.files).map(([name, fileContent]) => [
									name,
									fileContent,
								]),
							),
						);
					}
					break;

				case 'set_flashing_regions':
					console.log('Setting flashing regions:', content);
					setFlashRanges(
						content.spans.map((span) => ({
							filePath: span.file_path,
							startLine: span.start_line,
							startCol: span.start,
							endLine: span.end_line,
							endCol: span.end,
						})),
					);
					break;

				case 'select_function':
					console.log('Selecting function:', content);
					setSelectedFunction(content.function_name);
					break;

				case 'update_cursor':
					if ('cursor' in content) {
						updateCursor(content.cursor);
					}
					break;

				case 'baml_settings_updated':
					console.log('BAML settings updated:', content);
					setBamlConfig(content as BamlConfigAtom);
					break;

				case 'baml_cli_version':
					console.log('BAML CLI version:', content);
					setBamlCliVersion(content);
					break;

				case 'remove_project':
					console.log('Removing project');
					setFiles({});
					break;

				case 'run_test':
					if (selectedFunc) {
						setSelectedTestcase(content.test_name);
						runBamlTests([
							{ functionName: selectedFunc, testName: content.test_name },
						]);
					} else {
						console.error('No function selected for test run');
					}
					break;

				default:
					console.warn('Unknown VSCode command:', command);
			}
		};

		window.addEventListener('message', messageHandler);

		return () => {
			console.log('Cleaning up VSCode message listener');
			window.removeEventListener('message', messageHandler);
		};
	}, [
		selectedFunc,
		runBamlTests,
		updateCursor,
		debouncedSetFiles,
		setFlashRanges,
		setSelectedFunction,
		setSelectedTestcase,
		setBamlConfig,
		setBamlCliVersion,
		setFiles,
		setOrchestratorIndex,
	]);

	// This is a pure side-effect component, no UI
	return null;
}