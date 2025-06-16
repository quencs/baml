import { CopyButton } from '@baml/ui/custom/copy-button'
import { cn } from '@baml/ui/lib/utils'
import { atom, useAtomValue } from 'jotai'
import { useMemo } from 'react'
import React from 'react'
import { displaySettingsAtom } from '../preview-toolbar'
import { getHighlightedParts } from './highlight-utils'
import { TokenEncoderCache } from './render-tokens'

export const showTokenCountsAtom = atom((get) => get(displaySettingsAtom).showTokenCounts)

export const RenderPromptPart: React.FC<{
  text: string
  highlightChunks?: string[]
  model?: string
  provider?: string
}> = ({ text, highlightChunks = [], model, provider }) => {
  const isDebugMode = useAtomValue(showTokenCountsAtom)
  const isLongText = useMemo(() => text.split('\n').length > 18, [text])
  // const currentClient = useAtomValue(currentClientsAtom)
  // this causes weird scroll issues

  const tokenizer = useMemo(() => {
    if (!isDebugMode) return undefined

    // TODO! Change this to the appropriate tokenizer!
    const encodingName = TokenEncoderCache.getEncodingNameForModel('baml-openai-chat', 'gpt-4o')
    console.log('encoding name', encodingName)
    if (!encodingName) return undefined

    const enc = TokenEncoderCache.INSTANCE.getEncoder(encodingName)
    return { enc, tokens: enc.encode(text) }
  }, [text, isDebugMode, model, provider])

  const HighlightedText: React.FC<{ text: string; highlightChunks: string[] }> = ({ text, highlightChunks }) => {
    const parts = getHighlightedParts(text, highlightChunks)
    return (
      <>
        {parts.map((part, i) =>
          part.highlight ? (
            <mark
              key={i}
              className={cn(
                'inline-flex items-center align-middle text-input rounded px-1 py-0.5 font-normal text-xs',
                part.text.trim() === '' ? 'bg-[var(--vscode-charts-red)]/20' : 'bg-[var(--vscode-charts-blue)]/20',
              )}
              style={{ whiteSpace: 'pre', wordBreak: 'keep-all' }}
            >
              {part.text}
            </mark>
          ) : (
            <React.Fragment key={i}>{part.text}</React.Fragment>
          ),
        )}
      </>
    )
  }

  // Only compute highlighted text if we're not tokenizing
  const renderContent = useMemo(() => {
    if (tokenizer) {
      const tokenized = Array.from(tokenizer.tokens).map((token) => tokenizer.enc.decode([token]))
      return (
        <>
          {tokenized.map((token, i) => (
            <span
              key={i}
              className={cn(
                'text-white',
                // Uncomment and use these classes if you want to color-code tokens
                // ['bg-fuchsia-800', 'bg-emerald-700', 'bg-yellow-600', 'bg-red-700', 'bg-cyan-700'][i % 5]
              )}
            >
              {token}
            </span>
          ))}
        </>
      )
    }

    // Only do highlighting if we're not tokenizing
    return <HighlightedText text={text} highlightChunks={highlightChunks} />
  }, [text, highlightChunks, tokenizer])

  return (
    <div className='flex flex-col'>
      <div className='relative p-2 rounded-b bg-card group max-h-[600px] overflow-y-auto'>
        <div className='absolute right-2 top-1 opacity-0 group-hover:opacity-100 transition-opacity z-10'>
          <CopyButton text={text} size='sm' variant='ghost' />
        </div>
        <pre className={cn('whitespace-pre-wrap text-xs leading-relaxed transition-all')}>{renderContent}</pre>
      </div>
    </div>
  )
}
