import { toProxyUrl } from '../utils/url';
import { HEADER_X_MYCUTE_SCHEME, PROTOCOL_HTTP, PROTOCOL_HTTPS, SCHEME_PREFIX_HTTP, SCHEME_PREFIX_HTTPS } from '../generated_constants';

/**
 * window.fetch および XMLHttpRequest のインターセプターを初期化します。
 * HTTP/HTTPSのURLを、自動的にMYCUTEプロキシ対応の形式へと書き換えます。
 */
export function initFetchInterceptors() {
    hookFetch();
    hookXhr();
}

/**
 * 元のURLからスキーム（http/https）を判定します。
 */
function detectScheme(input: string | URL): string {
    const urlStr = input.toString();
    if (urlStr.startsWith(SCHEME_PREFIX_HTTP)) return PROTOCOL_HTTP; // "http"
    if (urlStr.startsWith(SCHEME_PREFIX_HTTPS)) return PROTOCOL_HTTPS; // "https"
    // デフォルトはhttpsとするが、明示的に判定できる場合は正確に返す
    return PROTOCOL_HTTPS;
}

function hookFetch() {
    const originalFetch = window.fetch;

    window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
        let proxiedInput = input;
        let originalScheme = PROTOCOL_HTTPS;

        if (typeof input === 'string') {
            originalScheme = detectScheme(input);
            proxiedInput = toProxyUrl(input);
        } else if (input instanceof URL) {
            originalScheme = detectScheme(input);
            proxiedInput = toProxyUrl(input.toString());
        } else if (input instanceof Request) {
            originalScheme = detectScheme(input.url);
            // 新しいURLでリクエストを複製
            const newUrl = toProxyUrl(input.url);
            proxiedInput = new Request(newUrl, input);
        }

        // ヘッダーにスキーム情報を付与
        // Requestオブジェクトの場合はヘッダーを操作、initオブジェクトの場合はheadersを拡張
        if (proxiedInput instanceof Request) {
            proxiedInput.headers.set(HEADER_X_MYCUTE_SCHEME, originalScheme);
        } else {
            // inputがURL文字列の場合、initがなければ作成、あればheadersを編集
            if (!init) {
                init = { headers: { [HEADER_X_MYCUTE_SCHEME]: originalScheme } };
            } else {
                // headersの型が HeadersInit (Headers | string[][] | Record<string, string>) なので少し複雑
                const headers = new Headers(init.headers);
                headers.set(HEADER_X_MYCUTE_SCHEME, originalScheme);
                init.headers = headers;
            }
        }

        return originalFetch(proxiedInput, init);
    };
}

function hookXhr() {
    const originalOpen = XMLHttpRequest.prototype.open;

    XMLHttpRequest.prototype.open = function (
        method: string,
        url: string | URL,
        async: boolean = true,
        username?: string | null,
        password?: string | null
    ) {
        const urlStr = url.toString();
        const proxiedUrl = toProxyUrl(urlStr);
        const originalScheme = detectScheme(urlStr);

        // XHRはopen後にsetRequestHeaderを呼ぶ必要があるため、ここではURL変換のみ行う。
        // ヘッダー付与は setRequestHeader をフックするか、send 前に挿入する必要があるが、
        // XHR の仕様上 open 後 send 前なら setRequestHeader が使える。
        // ただし、ユーザーコードが open -> setRequestHeader -> send とする間に
        // 我々が割り込むのは難しい (prototype.open 内ではまだヘッダー操作できない)。

        // 解決策: open を呼んだ直後に、自前で setRequestHeader を呼んでおく。
        // これにより、後続のユーザーコードの setRequestHeader とマージされる。
        const result = originalOpen.call(this, method, proxiedUrl, async, username, password);

        try {
            this.setRequestHeader(HEADER_X_MYCUTE_SCHEME, originalScheme);
        } catch (e) {
            // 同期XHRや特定ステートでは失敗する可能性があるのでガード
            console.warn('[MYCUTE SDK] Failed to set scheme header for XHR', e);
        }

        return result;
    };
}
