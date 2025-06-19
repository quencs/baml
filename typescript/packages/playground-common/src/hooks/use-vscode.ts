import { useCallback, useEffect } from 'react';
import { vscode } from '../shared/baml-project-panel/vscode';

export interface VSCodeMessage {
	command: string;
	content?: any;
}

export type MessageHandler = (message: VSCodeMessage) => void;

export function useVSCode() {
	const postMessage = useCallback((message: VSCodeMessage) => {
		try {
			if (vscode.isVscode()) {
				vscode.postMessage(message);
			} else {
				console.log('VSCode not available, message not sent:', message);
			}
		} catch (error) {
			console.error('Error posting message to VSCode:', error);
		}
	}, []);

	const useMessageHandler = useCallback((handler: MessageHandler) => {
		useEffect(() => {
			const listener = (event: MessageEvent) => {
				try {
					if (event.data && typeof event.data.command === 'string') {
						handler(event.data);
					}
				} catch (error) {
					console.error('Error handling VSCode message:', error);
				}
			};

			window.addEventListener('message', listener);
			return () => window.removeEventListener('message', listener);
		}, [handler]);
	}, []);

	const markInitialized = useCallback(() => {
		try {
			vscode.markInitialized();
		} catch (error) {
			console.error('Error marking VSCode as initialized:', error);
		}
	}, []);

	const getPlaygroundPort = useCallback(async () => {
		try {
			return await vscode.getPlaygroundPort();
		} catch (error) {
			console.error('Error getting playground port:', error);
			return 2780144; // Default port
		}
	}, []);

	const setState = useCallback((state: any) => {
		try {
			vscode.setState(state);
		} catch (error) {
			console.error('Error setting VSCode state:', error);
		}
	}, []);

	const getState = useCallback(() => {
		try {
			return vscode.getState();
		} catch (error) {
			console.error('Error getting VSCode state:', error);
			return null;
		}
	}, []);

	return {
		postMessage,
		useMessageHandler,
		markInitialized,
		getPlaygroundPort,
		setState,
		getState,
		isVscode: vscode.isVscode(),
	};
}