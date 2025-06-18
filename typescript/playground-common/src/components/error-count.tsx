'use client'

import React from 'react'
import { useDiagnostics } from '../contexts/runtime-context'
import { CheckCircle, AlertTriangle, XCircle } from 'lucide-react'

export function ErrorCount() {
  const diagnostics = useDiagnostics();
  
  const warningCount = diagnostics.filter((e) => e.type === 'warning').length;
  const errorCount = diagnostics.length - warningCount;

  if (errorCount === 0 && warningCount === 0) {
    return (
      <div className='flex flex-row gap-1 items-center text-green-600'>
        <CheckCircle size={12} />
      </div>
    );
  }
  
  if (errorCount === 0) {
    return (
      <div className='flex flex-row gap-1 items-center text-yellow-600'>
        {warningCount} <AlertTriangle size={12} />
      </div>
    );
  }
  
  return (
    <div className='flex flex-row gap-1 items-center text-red-600'>
      {errorCount} <XCircle size={12} /> {warningCount} <AlertTriangle size={12} />
    </div>
  );
}