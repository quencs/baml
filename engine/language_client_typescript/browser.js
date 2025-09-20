/**
 * @warning This file is intended for browser usage only.
 * For Node.js environments, import Image and Audio directly from '@boundaryml/baml'.
 * Example:
 * ```ts
 * // ✅ Browser usage
 * import { Image, Audio } from '@boundaryml/baml/browser'
 *
 * // ❌ Don't import these from '@boundaryml/baml' in browser environments
 * import { Image, Audio } from '@boundaryml/baml'
 * ```
 */
// Import actual implementations
import { BamlAudio } from "./audio.js";
import { BamlImage } from "./image.js";
import { BamlVideo } from "./video.js";
import { BamlPdf } from "./pdf.js";
// Detect if we're in server-side rendering environment
const isSSR = typeof window === "undefined";
// Create a proxy handler that logs warnings in SSR environment
function createSSRProxyHandler(name) {
    return {
        get: (target, prop) => {
            if (isSSR) {
                console.warn(`Using ${name} from '@boundaryml/baml/browser' in a server-side environment. This will not function properly in SSR.`);
            }
            return target[prop];
        },
    };
}
// Create proxied versions that will work in both environments but warn in SSR
const ImageImpl = new Proxy(BamlImage, createSSRProxyHandler("Image"));
const AudioImpl = new Proxy(BamlAudio, createSSRProxyHandler("Audio"));
const PdfImpl = new Proxy(BamlPdf, createSSRProxyHandler("Pdf"));
const VideoImpl = new Proxy(BamlVideo, createSSRProxyHandler("Video"));
// Then export the implementations
export { ImageImpl as Image, AudioImpl as Audio, PdfImpl as Pdf, VideoImpl as Video, };
