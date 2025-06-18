'use client'

import React from 'react'
import { useWasm } from '../contexts/runtime-context'

export function VersionDisplay() {
  const wasm = useWasm();
  
  const version = wasm?.version ? wasm.version() : 'Loading...';
  
  return (
    <span className='text-muted-foreground text-[10px]'>
      VSCode Runtime Version: {version}
    </span>
  );
}