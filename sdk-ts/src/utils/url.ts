import { MYCUTE_SCHEME_HTTP, MYCUTE_SCHEME_HTTPS, MYCUTE_PROXY_SUFFIX, MYCUTE_PROXY_PORT } from '../generated_constants';

/**
 * 標準的なHTTP/HTTPS URLをMYCUTEプロキシスキームに変換します。
 * 
 * @param url 入力URL文字列
 * @returns プロキシされたURL文字列
 */
import { encodeHost } from './url_encoder';

/**
 * 標準的なHTTP/HTTPS URLをMYCUTEプロキシスキームに変換します。
 * 
 * @param url 入力URL文字列
 * @returns プロキシされたURL文字列
 */
export function toProxyUrl(url: string): string {
    try {
        // 安全にロジックを処理するためにURLを解析します
        const urlObj = new URL(url, window.location.origin);

        // すでにプロキシされたホスト（サフィックスで終わる）の場合は、そのまま返します
        if (urlObj.hostname.endsWith(MYCUTE_PROXY_SUFFIX)) {
            return url;
        }

        // localhost や IP アドレスを含め、すべてのホストに対して一貫性を持たせるために
        // 例外なくサフィックス（.mc.local）を付加してプロキシ経由で処理します。

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
        console.warn('[MYCUTE SDK] Failed to convert to proxy URL:', url, e);
        return url;
    }
}

/**
 * URLがすでにプロキシスキーム（あるいはプロキシドメイン）を使用しているか判定します。
 * @param urlToCheck 判定対象のURL
 */
export function isProxyUrl(urlToCheck: string): boolean {
    if (!urlToCheck) return false;
    try {
        const urlObj = new URL(urlToCheck, window.location.href);
        return urlObj.hostname.endsWith(MYCUTE_PROXY_SUFFIX);
    } catch (e) {
        // 文字列チェック (フォールバック)
        return urlToCheck.includes(MYCUTE_PROXY_SUFFIX);
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
                // Ensure encodeHost is available or replicate logic if strict dependency issue exists.
                // But simplified fallback might just be dangerous if incorrect.
                // Let's rely on basic string replacement here as fallback of fallback.
                // But better to use encodeHost if possible.
                // encodeHost is pure string manipulation so safe.
                parts[0] = `${encodeHost(parts[0])}:${MYCUTE_PROXY_PORT}`;
                return `${proto}://${parts.join('/')}`;
            }
        }
    }
    return url;
}

/**
 * 現在の環境がMYCUTE (Tauri) 内で実行されているかチェックします。
 * これは、Tauriによって注入された特定のウィンドウプロパティを確認することで改善できます。
 */
export function isMycuteEnvironment(): boolean {
    if (typeof window === 'undefined') return false;

    // 1. MYCUTEカスタムプロトコルをチェック (ゲストに対して最も信頼性が高い)
    const protocol = window.location.protocol;
    if (protocol === `${MYCUTE_SCHEME_HTTP}:` || protocol === `${MYCUTE_SCHEME_HTTPS}:`) {
        return true;
    }

    // 2. ドメインサフィックスをチェック (フェーズ 8)
    const hostname = window.location.hostname;
    if (hostname.endsWith(MYCUTE_PROXY_SUFFIX)) {
        return true;
    }

    // 3. Tauriの内部変数をチェック (シェルコンテキストのフォールバック)
    return typeof (window as any).__TAURI_INTERNALS__ !== 'undefined';
}
