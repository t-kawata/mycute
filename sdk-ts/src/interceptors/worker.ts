import { toProxyUrl, isProxyUrl } from '../utils/url';
import { MYCUTE_SDK_FILENAME } from '../generated_constants';

/**
 * Web Workerのコンストラクタをフックし、Worker内でもプロキシが有効になるようにします。
 * Workerスクリプトの先頭にSDKのインポートを強制的に注入します。
 */
export function initWorkerInterceptor() {
    const originalWorker = window.Worker;

    // @ts-ignore
    window.Worker = function (scriptURL: string | URL, options?: WorkerOptions) {
        let urlStr = scriptURL.toString();

        // 1. Resolve to absolute URL
        try {
            urlStr = new URL(urlStr, window.location.href).href;
        } catch (e) {
            // keep original if fails
        }

        // 2. Proxy the script URL itself (to avoid 502/CORS on load)
        if (!isProxyUrl(urlStr)) {
            urlStr = toProxyUrl(urlStr);
        }

        console.log(`[MYCUTE SDK] Intercepting Worker creation for: ${urlStr}`);

        // 3. Inject SDK via Blob (Bootstrapping)
        // We want the worker to load our SDK first, then the original script.
        // importScripts() is synchronous.

        // SDK URL must be absolute and proxied (or local)
        const sdkUrl = new URL(`/${MYCUTE_SDK_FILENAME}`, window.location.origin).href;

        // Create a bootstrap script
        const bootstrapCode = `
            try {
                importScripts('${sdkUrl}');
                // console.log('[MYCUTE Worker] SDK injected successfully.');
            } catch (e) {
                console.error('[MYCUTE Worker] Failed to inject SDK:', e);
            }
            importScripts('${urlStr}');
        `;

        const blob = new Blob([bootstrapCode], { type: 'application/javascript' });
        const blobUrl = URL.createObjectURL(blob);

        // Call original constructor with the Blob URL
        // We lose valid 'options' (like type: module) support if we use importScripts...
        // If options.type === 'module', importScripts is invalid.
        // But most legacy workers use classic scripts.
        // For module workers, we might need 'import ...' syntax and Blob with type 'application/javascript' might still work if we use dynamic import?
        // Let's check options.

        if (options?.type === 'module') {
            // Module worker injection is harder. 
            // We can try to use standard proxy url and hope the SW catches the sub-requests.
            // Or we construct a module blob: "import ...; import ..."
            const moduleBootstrap = `
                import '${sdkUrl}';
                import '${urlStr}';
            `;
            const moduleBlob = new Blob([moduleBootstrap], { type: 'application/javascript' });
            const moduleBlobUrl = URL.createObjectURL(moduleBlob);
            return new originalWorker(moduleBlobUrl, options);
        }

        return new originalWorker(blobUrl, options);
    };

    // Copy prototype to ensure instanceof works (mostly)
    window.Worker.prototype = originalWorker.prototype;

    console.log('[MYCUTE SDK] Worker interceptor active.');
}
