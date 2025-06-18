'use client';

import { useAtomValue } from 'jotai';
import { atom } from 'jotai';
import { wasmAtom } from '../shared/baml-project-panel/atoms';
import { ErrorCount } from './error-count';
import { bamlCliVersionAtom } from './event-listener';

const versionAtom = atom((get) => {
  const wasm = get(wasmAtom);
  if (wasm === undefined) {
    return 'Loading...';
  }
  return wasm.version();
});

export function StatusBar() {
  const version = useAtomValue(versionAtom);
  const bamlCliVersion = useAtomValue(bamlCliVersionAtom);

  return (
    <div className="flex absolute right-2 bottom-2 z-50 flex-row gap-2 text-xs bg-transparent">
      <div className="pr-4 whitespace-nowrap">
        {bamlCliVersion && `baml-cli ${bamlCliVersion}`}
      </div>
      <ErrorCount />
      <span className="text-muted-foreground text-[10px]">
        VSCode Runtime Version: {version}
      </span>
    </div>
  );
}