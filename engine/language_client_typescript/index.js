export * from './safe_imports.js';
export * from './errors.js';
export * from './logging.js';
// Detect if we're in a Node.js environment
const isNode = typeof process !== 'undefined' && process.versions != null && process.versions.node != null;
if (!isNode) {
    const browserError = (name) => {
        throw new Error(`Cannot import ${name} from '@boundaryml/baml' in browser environment. Please import from '@boundaryml/baml/browser' instead.`);
    };
    // Provide helpful error messages for browser imports
    Object.defineProperty(exports, 'Image', {
        get: () => browserError('Image'),
        enumerable: true,
    });
    Object.defineProperty(exports, 'Audio', {
        get: () => browserError('Audio'),
        enumerable: true,
    });
    Object.defineProperty(exports, 'Pdf', {
        get: () => browserError('Pdf'),
        enumerable: true,
    });
    Object.defineProperty(exports, 'Video', {
        get: () => browserError('Video'),
        enumerable: true,
    });
}
export { BamlRuntime, FunctionResult, FunctionResultStream, BamlImage as Image, BamlAudio as Audio, BamlPdf as Pdf, BamlVideo as Video, invoke_runtime_cli, ClientRegistry, Collector, FunctionLog, LlmCall, LlmStreamCall, Usage, HTTPRequest, HTTPResponse, SSEResponse, StreamTiming, Timing, TraceStats, } from './native.js';
export { BamlStream } from './stream.js';
export { BamlCtxManager } from './async_context_vars.js';
