/// <reference lib="webworker" />
// @ts-ignore
import {
    SCHEME_PREFIX_HTTP, SCHEME_PREFIX_HTTPS,
    PROTOCOL_HTTP, PROTOCOL_HTTPS,
    DOMAIN_LOCALHOST, IP_LOCALHOST,
    PATH_PROXY_LEAK_SW,
    MYCUTE_PROXY_SUFFIX,
    HEADER_X_MYCUTE_SCHEME
} from '../generated_constants';
import { encodeHost } from '../utils/url_encoder';

// This script runs in the Service Worker context.
// It intercepts all fetch events and rewrites requests to use the proxy scheme.

const SW_VERSION = '0.24.66';

// Cast self to ServiceWorkerGlobalScope to access SW-specific properties and events
const sw = self as unknown as ServiceWorkerGlobalScope;

sw.addEventListener('install', (event) => {
    sw.skipWaiting();
});

sw.addEventListener('activate', (event) => {
    event.waitUntil(sw.clients.claim());
});

sw.addEventListener('fetch', (event) => {
    const request = event.request;
    const url = request.url;

    // 自己参照（通報API自体）は無視する（無限ループ防止）
    // PATH_PROXY_LEAK_SW は "/v1/..." なので部分一致で確認
    if (url.includes(PATH_PROXY_LEAK_SW)) {
        return;
    }

    // http/https リクエストをチェック
    // 標準的な http/https スキームであり、かつローカルホスト（開発環境/SWサーバー）でないものを対象とする
    if (url.startsWith(SCHEME_PREFIX_HTTP) || url.startsWith(SCHEME_PREFIX_HTTPS)) {
        // 既にプロキシ対象（サフィックス付き）のドメインであれば何もしない
        // URL オブジェクトでの判定が安全だが、SW 内でのオーバーヘッドを考慮し文字列判定を行う
        // ホスト名の末尾判定を行うには URL パースが必要だが、ここでは簡易的に文字列末尾チェックで代用せず、
        // URL オブジェクトを使って安全に判定する。
        let isAlreadyProxied = false;
        try {
            const urlObj = new URL(url);
            if (urlObj.hostname.endsWith(MYCUTE_PROXY_SUFFIX)) {
                isAlreadyProxied = true;
            }
        } catch (e) {
            // URL パースエラー時は何もしない（プロキシ対象外とみなす）
            return;
        }

        if (isAlreadyProxied) {
            return;
        }

        // ここに来たということは「プロキシ対象外の生の http/https リクエスト」である
        // = プロキシ漏れ検出
        // Phase 8: スキーム置換ではなく、ダブルハイフンエンコーディングを行う
        let proxiedUrl = url;
        let originalScheme = PROTOCOL_HTTPS;

        try {
            const urlObj = new URL(url);

            // スキーム判定
            if (urlObj.protocol === `${PROTOCOL_HTTP}:`) {
                originalScheme = PROTOCOL_HTTP;
            } else if (urlObj.protocol === `${PROTOCOL_HTTPS}:`) {
                originalScheme = PROTOCOL_HTTPS;
            }

            // Encode hostname: google.com -> google--com.mc.shyme.net
            urlObj.hostname = encodeHost(urlObj.hostname);
            proxiedUrl = urlObj.toString();
        } catch (e) {
            console.error('[MYCUTE SW] Failed to construct config url:', e);
            return;
        }

        if (proxiedUrl !== url) {
            // 非同期で通報 (Fire and Forget)
            const reportUrl = `${self.location.origin}${PATH_PROXY_LEAK_SW}`;

            // 重要: 通報処理がメインのリクエストをブロックしないようにする
            // catch でエラーを握りつぶす
            fetch(reportUrl, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    url: url,
                    message: `Proactive SW Intercept: Redirecting raw request to ${proxiedUrl}`
                })
            }).catch(e => console.error('[MYCUTE SW] Failed to report leak:', e));

            // 元のリクエストヘッダーを複製し、新しいヘッダーを追加
            const newRequestHeaders = new Headers(request.headers);
            newRequestHeaders.set(HEADER_X_MYCUTE_SCHEME, originalScheme);

            // 自動修正して送信
            console.log(`[MYCUTE SW] Intercepted raw request: ${url} -> ${proxiedUrl}`);

            event.respondWith(
                fetch(proxiedUrl, {
                    method: request.method,
                    headers: newRequestHeaders, // 修正済みヘッダーを使用
                    body: request.body,
                    mode: request.mode,
                    credentials: request.credentials,
                    cache: request.cache,
                    redirect: request.redirect,
                    referrer: request.referrer,
                    integrity: request.integrity,
                }).then(response => {
                    if (response.status >= 400) {
                        console.warn(`[MYCUTE SW] Proxy response error ${response.status} for ${proxiedUrl}`);
                    }
                    // Re-create the response with modified headers to fix CORS issues
                    const newHeaders = new Headers(response.headers);

                    // Force allow origin to the current page's origin (transparent mode)
                    newHeaders.set('Access-Control-Allow-Origin', '*');
                    newHeaders.set('Access-Control-Allow-Methods', '*');
                    newHeaders.set('Access-Control-Allow-Headers', '*');
                    newHeaders.set('Access-Control-Allow-Credentials', 'true');

                    return new Response(response.body, {
                        status: response.status,
                        statusText: response.statusText,
                        headers: newHeaders,
                    });
                }).catch(err => {
                    console.error('[MYCUTE SW] CRITICAL: Proxy fetch failed for', proxiedUrl, 'Error:', err);
                    return new Response(`Proxy Error: ${err}`, { status: 502 });
                })
            );
        }
    }
});
