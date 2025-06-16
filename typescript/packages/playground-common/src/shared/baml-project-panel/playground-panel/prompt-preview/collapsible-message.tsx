import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@baml/ui/collapsible'
import { cn } from '@baml/ui/lib/utils'
import type { WasmTestCase } from '@gloo-ai/baml-schema-wasm-web'
import { ChevronsUpDown } from 'lucide-react'
import { useState } from 'react'
import { getFirstLine } from './highlight-utils'
import { PromptStats } from './prompt-stats'
import { RenderPart } from './render-part'

interface CollapsibleMessageProps {
  part: any
  partIndex: number
  testCase?: WasmTestCase
}

export const CollapsibleMessage: React.FC<CollapsibleMessageProps> = ({ part, partIndex, testCase }) => {
  const [open, setOpen] = useState(false)
  const firstLine = getFirstLine(part.parts)
  const statsText = part.parts.map((part: any) => part.as_text() ?? '').join('\n')

  return (
    <div
      className={cn('border-l-4 pl-2 rounded', {
        'border-[var(--vscode-charts-blue)]': part.role === 'assistant',
        'border-[var(--vscode-charts-green)]': part.role === 'user',
        'border-[var(--vscode-charts-gray)]': part.role === 'system',
        'border-[var(--vscode-charts-yellow)]':
          part.role !== 'assistant' && part.role !== 'user' && part.role !== 'system',
      })}
    >
      <Collapsible open={open} onOpenChange={setOpen}>
        <CollapsibleTrigger
          className={cn(
            'flex w-full items-center justify-between px-2 py-2 transition-colors',
            'data-[state=closed]:bg-card data-[state=closed]:rounded-t',
          )}
        >
          <div className='flex flex-col items-start gap-1'>
            <div className='text-xs text-muted-foreground'>
              {part.role.charAt(0).toUpperCase() + part.role.slice(1)}
            </div>
            {!open && firstLine && <div className='text-sm truncate'>{firstLine} ...</div>}
          </div>
          <ChevronsUpDown className={'size-4'} />
        </CollapsibleTrigger>
        <CollapsibleContent className='space-y-3'>
          {part.parts.map((part: any, index: number) => (
            <div key={`${partIndex}-${index}`}>
              <RenderPart part={part} testCase={testCase} />
            </div>
          ))}
        </CollapsibleContent>
      </Collapsible>
      <PromptStats text={statsText} />
    </div>
  )
}
