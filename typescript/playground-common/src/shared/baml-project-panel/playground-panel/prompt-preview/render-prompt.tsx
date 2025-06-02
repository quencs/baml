import type { WasmPrompt, WasmTestCase } from '@gloo-ai/baml-schema-wasm-web'
import { RenderPart } from './render-part'
import { cn } from '@/lib/utils'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { ChevronDown, ChevronsUpDown } from 'lucide-react'
import { useState } from 'react'
import he from 'he'

export const RenderPrompt: React.FC<{
  prompt: WasmPrompt
  testCase?: WasmTestCase
}> = ({ prompt, testCase }) => {
  const chat = prompt.as_chat() ?? []

  const getFirstLine = (parts: any[]) => {
    for (const part of parts) {
      if (part.is_text()) {
        const text = part.as_text()
        if (text) {
          const decodedText = he.decode(text)
          const lines = decodedText.split('\n')
          if (lines.length > 0 && lines[0].trim()) {
            return lines[0].trim()
          }
        }
      }
    }
    return ''
  }

  return (
    <div className='h-full space-y-4'>
      {chat.map((p, partIndex) => {
        const [isOpen, setIsOpen] = useState(true)
        const firstLine = getFirstLine(p.parts)
        return (
          <Collapsible
            key={partIndex}
            open={isOpen}
            onOpenChange={setIsOpen}
            className={cn('border-l-4 pl-2 rounded', {
              'border-[var(--vscode-charts-blue)]': p.role === 'assistant',
              'border-[var(--vscode-charts-green)]': p.role === 'user',
              'border-[var(--vscode-charts-gray)]': p.role === 'system',
              'border-[var(--vscode-charts-yellow)]': p.role !== 'assistant' && p.role !== 'user' && p.role !== 'system',
            })}
          >
            <CollapsibleTrigger
              className={cn(
                'flex w-full items-center justify-between px-2 py-2 transition-colors',
                'data-[state=closed]:bg-card data-[state=closed]:rounded-md'
              )}
            >
              <div className='flex flex-col items-start gap-1'>
                <div className='text-xs text-muted-foreground'>{p.role.charAt(0).toUpperCase() + p.role.slice(1)}</div>
                {firstLine && !isOpen && (
                  <div className='text-sm truncate'>{firstLine} ...</div>
                )}
              </div>
              <ChevronsUpDown className={'size-4'} />
            </CollapsibleTrigger>
            <CollapsibleContent className='space-y-3'>
              {p.parts.map((part, index) => (
                <RenderPart key={`${partIndex}-${index}`} part={part} testCase={testCase} />
              ))}
            </CollapsibleContent>
          </Collapsible>
        )
      })}
    </div>
  )
}
