import {
    SCHEME_PREFIX_HTTP,
    SCHEME_PREFIX_HTTPS,
    MYCUTE_PROXY_SUFFIX,
    MYCUTE_SCHEME_HTTP,
    MYCUTE_SCHEME_HTTPS,
    MYCUTE_PROXY_PORT
} from 'src/consts/generated_constants';

import { encodeHost } from './url_encoder';

/**
 * 標準の HTTP/HTTPS URL を Tauri プロキシ用の URL（ドメインサフィックス方式）に変換します。
 * 
 * @param url 変換対象の標準 URL (https://... または http://...)
 * @param isTauri Tauri 環境下で実行されているかどうか
 * @returns 変換後の URL。Tauri 環境でない場合は元の URL を返します。
 */
export function getUrlForProxy(url: string, isTauri: boolean): string {
    if (!isTauri) return url;

    // すでにカスタムスキームになっている場合はそのまま返す (Legacy support)
    if (url.startsWith(`${MYCUTE_SCHEME_HTTPS}://`) || url.startsWith(`${MYCUTE_SCHEME_HTTP}://`)) {
        return url;
    }

    try {
        const urlObj = new URL(url, window.location.origin);

        // すでにサフィックスがついている場合はそのまま
        if (urlObj.hostname.endsWith(MYCUTE_PROXY_SUFFIX)) {
            return url;
        }

        // サフィックスを適用 (ダブルハイフンエンコーディングを含む)
        // 例: google.com -> google--com.mc.shyme.net:58300
        urlObj.hostname = encodeHost(urlObj.hostname);
        urlObj.port = MYCUTE_PROXY_PORT.toString(); // Direct Hosting Architecture (Phase 8.20)
        return urlObj.toString();
    } catch (e) {
        // パースに失敗した場合は文字列ベースのフォールバックを試みます
        const fallbackUrl = applyStringFallback(url);
        if (fallbackUrl !== url) {
            return fallbackUrl;
        }
        console.warn('Failed to convert to proxy URL (parsing and fallback failed):', url, e);
        return url;
    }
}

/**
 * URLのパースに失敗した場合の文字列ベースのフォールバック処理です。
 */
function applyStringFallback(url: string): string {
    if (url.includes('://')) {
        const [proto, rest] = url.split('://');
        if (rest) {
            const parts = rest.split('/');
            if (parts[0] && !parts[0].endsWith(MYCUTE_PROXY_SUFFIX)) {
                parts[0] = `${encodeHost(parts[0])}:${MYCUTE_PROXY_PORT}`;
                return `${proto}://${parts.join('/')}`;
            }
        }
    }
    return url;
}
