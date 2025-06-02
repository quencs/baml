import { atom, useAtomValue } from 'jotai'
import { renderModeAtom } from '../preview-toolbar'
import { useMemo, useState } from 'react'
import { ChevronDown, ChevronUp } from 'lucide-react'
import { ScrollArea } from '@/components/ui/scroll-area'
import { TokenEncoderCache } from './render-tokens'
import React from 'react'
import { cn } from '@/lib/utils'
import { CopyButton } from '@/components/copy-button'
import { Button } from '@/components/ui/button'

export const isDebugModeAtom = atom((get) => get(renderModeAtom) === 'tokens')

export const RenderPromptPart: React.FC<{
  text: string
  highlightChunks?: string[]
  model?: string
  provider?: string
}> = ({ text, highlightChunks = [], model, provider }) => {
  const isDebugMode = useAtomValue(isDebugModeAtom)
  const isLongText = useMemo(() => text.split('\n').length > 18, [text])
  // const currentClient = useAtomValue(currentClientsAtom)
  // this causes weird scroll issues
  const [isFullTextVisible, setIsFullTextVisible] = useState(true)

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
    if (!highlightChunks?.length) return <>{text}</>;

    console.log('Debug - HighlightedText - Text:', text)
    console.log('Debug - HighlightedText - Highlight chunks:', highlightChunks)

    // Build a regex that matches any of the highlightChunks
    const regex = new RegExp(
      highlightChunks
        .filter(Boolean)
        .sort((a, b) => b.length - a.length)
        .map(chunk =>
          chunk
            .replace(/[.*+?^${}()|[\\]\\]/g, '\\$&')
            .replace(/ /g, '\\s+') // Replace literal spaces with \s+ for whitespace/newline tolerance
        )
        .join('|'),
      'gms' // Add 's' for dotAll
    );

    console.log('Debug - HighlightedText - Regex:', regex)

    const parts = [];
    let lastIndex = 0;
    let match: RegExpExecArray | null;

    while ((match = regex.exec(text)) !== null) {
      console.log('Debug - HighlightedText - Match found:', match[0], 'at index:', match.index)
      if (match.index > lastIndex) {
        parts.push(text.slice(lastIndex, match.index));
      }
      parts.push(
        <mark
          key={match.index}
          className={cn(
            "inline-flex items-center align-middle text-input rounded px-1 py-0.5 font-normal text-xs",
            match[0].trim() === ""
              ? "bg-[var(--vscode-charts-red)]/20"
              : "bg-[var(--vscode-charts-blue)]/20"
          )}
          style={{ whiteSpace: 'pre', wordBreak: 'keep-all' }}
        >
          {match[0]}
        </mark>
      );
      lastIndex = regex.lastIndex;
    }
    if (lastIndex < text.length) {
      parts.push(text.slice(lastIndex));
    }

    console.log('Debug - HighlightedText - Final parts:', parts.length)

    return <>{parts}</>;
  };

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
      {isDebugMode && (
        <div className='flex flex-row gap-4 justify-start items-center px-3 py-2 text-xs border-b border-border bg-muted text-muted-foreground'>
          <div className='flex items-center gap-1.5'>
            <span className='text-muted-foreground/60'>Characters:</span>
            <span className='font-medium'>{text.length}</span>
          </div>
          <div className='flex items-center gap-1.5'>
            <span className='text-muted-foreground/60'>Words:</span>
            <span className='font-medium'>{text.split(/\s+/).filter(Boolean).length}</span>
          </div>
          <div className='flex items-center gap-1.5'>
            <span className='text-muted-foreground/60'>Lines:</span>
            <span className='font-medium'>{text.split('\n').length}</span>
          </div>
          <div className='flex items-center gap-1.5'>
            <span className='text-muted-foreground/60'>Tokens (est.):</span>
            <span className='font-medium'>{Math.ceil(text.length / 4)}</span>
          </div>
        </div>
      )}
      <div className='relative flex-1'>
        <ScrollArea className='relative p-2 rounded bg-card group' type='always'>
          <div className='absolute right-2 top-1 opacity-0 group-hover:opacity-100 transition-opacity z-10'>
            <CopyButton text={text} size="sm" variant="ghost" />
          </div>
          <pre
            className={cn(
              'whitespace-pre-wrap text-xs leading-relaxed transition-all',
              isLongText && !isFullTextVisible ? 'max-h-48 overflow-hidden' : 'max-h-[600px]'
            )}
          >
            {renderContent}
          </pre>
        </ScrollArea>
        {/* {isLongText && (
          <Button
            size='xs'
            variant='outline'
            onClick={() => setIsFullTextVisible(!isFullTextVisible)}
            className='flex absolute left-1/2 -translate-x-1/2 bottom-[-12px] gap-1 items-center rounded-full z-10 px-3 text-muted-foreground border-border '
          >
            {isFullTextVisible ? (
              <>
                Show less
                <ChevronUp className='w-3 h-3' />
              </>
            ) : (
              <>
                Show more
                <ChevronDown className='w-3 h-3' />
              </>
            )}
          </Button>
        )} */}
      </div>
    </div>
  )
}
