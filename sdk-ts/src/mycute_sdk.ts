import { initFetchInterceptors } from './interceptors/fetch';
import { initNavigationInterceptor } from './interceptors/navigation';
import { initWebSocketInterceptor } from './interceptors/websocket';
import { initEventSourceInterceptor } from './interceptors/eventsource';
import { initDomInterceptors } from './interceptors/dom';
import { initWorkerInterceptor } from './interceptors/worker';
import { registerServiceWorker } from './service-worker/register';
import { isMycuteEnvironment } from './utils/url';
import { MYCUTE_SCHEME_HTTPS, MYCUTE_ORIGIN } from './generated_constants';

export * from './utils/url';
export * from './interceptors/fetch';
export * from './interceptors/navigation';
export * from './interceptors/websocket';
export * from './interceptors/eventsource';
export * from './service-worker/register';

export interface MycuteSdkOptions {
    /**
     * Path to the service worker file.
     * @default '/sw.js'
     */
    swPath?: string;

    /**
     * Whether to enable the Service Worker.
     * @default true
     */
    enableServiceWorker?: boolean;
}

/**
 * Initializes the MYCUTE Access SDK.
 * This sets up all interceptors (Fetch, XHR, Navigation, WebSocket, SSE) and registers the Service Worker.
 * 
 * @param options Configuration options
 */
export async function initMycute(options: MycuteSdkOptions = {}) {
    // Prevent double initialization
    if ((window as any).__MYCUTE_SDK_INITIALIZED_INTERNAL__) {
        return;
    }

    // Only initialize if running inside MYCUTE
    if (!isMycuteEnvironment()) {
        // We avoid logging here to be quiet on standard web pages if somehow loaded
        return;
    }

    (window as any).__MYCUTE_SDK_INITIALIZED_INTERNAL__ = true;
    console.log('[MYCUTE SDK] Initializing...');

    // proxyOriginの決定: swPathから抽出、またはデフォルトのプロキシドメインを使用
    let proxyOrigin = `${MYCUTE_SCHEME_HTTPS}://${MYCUTE_ORIGIN.replace('https://', '')}`; // フォールバック用のデフォルト値
    if (options.swPath) {
        try {
            const url = new URL(options.swPath, window.location.href);
            proxyOrigin = url.origin;
        } catch (e) {
            console.warn('[MYCUTE SDK] Failed to parse swPath, using default proxy origin.', e);
        }
    }

    // 1. Hook Fetch & XHR
    initFetchInterceptors();
    console.log('[MYCUTE SDK] Fetch & XHR interceptors active.');

    // 2. Hook Navigation
    initNavigationInterceptor();
    console.log('[MYCUTE SDK] Navigation interceptor active.');

    // 3. Hook WebSocket
    initWebSocketInterceptor(proxyOrigin);
    console.log('[MYCUTE SDK] WebSocket interceptor active.');

    // 4. Hook EventSource (SSE)
    initEventSourceInterceptor(proxyOrigin);
    console.log('[MYCUTE SDK] EventSource interceptor active.');

    // 5. Hook DOM (Image, etc. + Shadow DOM)
    initDomInterceptors();

    // 6. Hook Web Worker
    initWorkerInterceptor();

    // 7. Register Service Worker
    if (options.enableServiceWorker !== false) {
        await registerServiceWorker(options.swPath);
    }

    console.log('[MYCUTE SDK] Initialization complete.');
}

// --- Auto-Initialization (Plan B) ---
// When the script is loaded via <script>, it automatically starts.
if (typeof window !== 'undefined') {
    // Use a small delay to ensure the window is ready (optional but safer for module scripts)
    // Actually, type="module" executes after parsing.
    initMycute({
        swPath: '/mycute_sw.js',
        enableServiceWorker: true
    }).catch(err => {
        console.error('[MYCUTE SDK] Auto-init failed:', err);
    });
}
