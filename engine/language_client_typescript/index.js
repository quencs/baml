export * from './safe_imports.js';
export * from './errors.js';
export * from './logging.js';
export { BamlRuntime, FunctionResult, FunctionResultStream, invoke_runtime_cli, ClientRegistry, Collector, FunctionLog, LlmCall, LlmStreamCall, Usage, HttpRequest as HTTPRequest, HttpResponse as HTTPResponse, SseResponse as SSEResponse, StreamTiming, Timing, TraceStats, } from './native.js';
export { BamlStream } from './stream.js';
export { BamlCtxManager } from './async_context_vars.js';
import { BamlAudio, BamlImage, BamlPdf, BamlVideo } from './native.js';
const isNode = typeof process !== 'undefined' && !!process.versions?.node;
const browserErrorMessage = (name) => `Cannot import ${name} from '@boundaryml/baml' in browser environment. Please import from '@boundaryml/baml/browser' instead.`;
function createBrowserGuard(name, target) {
    if (isNode) {
        return target;
    }
    const throwBrowserError = () => {
        throw new Error(browserErrorMessage(name));
    };
    const handler = {
        get() {
            return throwBrowserError();
        },
        apply() {
            return throwBrowserError();
        },
        construct() {
            return throwBrowserError();
        },
    };
    return new Proxy(target, handler);
}
export const Image = createBrowserGuard('Image', BamlImage);
export const Audio = createBrowserGuard('Audio', BamlAudio);
export const Pdf = createBrowserGuard('Pdf', BamlPdf);
export const Video = createBrowserGuard('Video', BamlVideo);
