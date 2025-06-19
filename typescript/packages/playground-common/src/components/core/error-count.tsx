'use client';

import { cn } from '@baml/ui/lib/utils';
import { AlertTriangle, CheckCircle, XCircle } from 'lucide-react';
import { useAtomValue } from 'jotai';
import React from 'react';
import { diagnosticsAtom, numErrorsAtom } from '../../shared/baml-project-panel/atoms';

export function ErrorCount() {
	const diagnostics = useAtomValue(diagnosticsAtom);

	const warningCount = diagnostics.filter((e: any) => e.type === 'warning').length;
	const errorCount = diagnostics.length - warningCount;

	if (errorCount === 0 && warningCount === 0) {
		return (
			<div className="flex flex-row gap-1 items-center text-green-600">
				<CheckCircle size={12} />
			</div>
		);
	}

	if (errorCount === 0) {
		return (
			<div className="flex flex-row gap-1 items-center text-yellow-600">
				{warningCount} <AlertTriangle size={12} />
			</div>
		);
	}

	return (
		<div className="flex flex-row gap-1 items-center text-red-600">
			{errorCount} <XCircle size={12} /> {warningCount} <AlertTriangle size={12} />
		</div>
	);
}