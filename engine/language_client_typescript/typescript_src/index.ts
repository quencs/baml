export * from './safe_imports.js'

export * from './errors.js'

export * from './logging.js'

export {
  BamlRuntime,
  FunctionResult,
  FunctionResultStream,
  invoke_runtime_cli,
  ClientRegistry,
  BamlLogEvent,
  Collector,
  FunctionLog,
  LlmCall,
  LlmStreamCall,
  Usage,
  HttpRequest as HTTPRequest,
  HttpResponse as HTTPResponse,
  SseResponse as SSEResponse,
  StreamTiming,
  Timing,
  TraceStats,
} from './native.js'

export { BamlStream } from './stream.js'
export { BamlCtxManager } from './async_context_vars.js'

import { BamlAudio, BamlImage, BamlPdf, BamlVideo } from './native.js'

const isNode = typeof process !== 'undefined' && !!process.versions?.node

const browserErrorMessage = (name: string) =>
  `Cannot import ${name} from '@boundaryml/baml' in browser environment. Please import from '@boundaryml/baml/browser' instead.`

function createBrowserGuard<T extends object>(name: string, target: T): T {
  if (isNode) {
    return target
  }

  const throwBrowserError = (): never => {
    throw new Error(browserErrorMessage(name))
  }

  const handler: ProxyHandler<any> = {
    get() {
      return throwBrowserError()
    },
    apply() {
      return throwBrowserError()
    },
    construct() {
      return throwBrowserError()
    },
  }

  return new Proxy(target as unknown as object, handler) as T
}

export const Image = createBrowserGuard('Image', BamlImage)
export const Audio = createBrowserGuard('Audio', BamlAudio)
export const Pdf = createBrowserGuard('Pdf', BamlPdf)
export const Video = createBrowserGuard('Video', BamlVideo)

export type Image = BamlImage
export type Audio = BamlAudio
export type Pdf = BamlPdf
export type Video = BamlVideo
