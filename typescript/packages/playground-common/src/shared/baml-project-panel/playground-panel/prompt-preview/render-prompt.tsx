import type { WasmPrompt, WasmTestCase } from '@gloo-ai/baml-schema-wasm-web'
import { CollapsibleMessage } from './collapsible-message'

export const RenderPrompt: React.FC<{
  prompt: WasmPrompt
  testCase?: WasmTestCase
}> = ({ prompt, testCase }) => {
  const chat = prompt.as_chat() ?? []

  return (
    <div className='h-full space-y-4'>
      {chat.map((p, partIndex) => (
        <CollapsibleMessage key={partIndex} part={p} partIndex={partIndex} testCase={testCase} />
      ))}
    </div>
  )
}
