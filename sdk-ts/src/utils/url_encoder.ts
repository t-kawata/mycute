
import { MYCUTE_PROXY_SUFFIX } from '../generated_constants';

/**
 * ドットをダブルハイフンに置換し、プロキシサフィックスを付与することでホスト名をエンコードします。
 * 例: google.com -> google--com.mc.shyme.net
 * @param host 元のホスト名 (例: "google.com")
 * @returns エンコードされたホスト名 (例: "google--com.mc.shyme.net")
 */
export function encodeHost(host: string): string {
    if (!host) return host;
    // すでにサフィックスがある場合は二重エンコードを防ぐ
    if (host.endsWith(MYCUTE_PROXY_SUFFIX)) {
        return host;
    }

    // すべてのドットをダブルハイフンに置換します
    // 既存のハイフンには触れません
    // example-api.com -> example-api--com.mc.shyme.net
    const encodedBase = host.replace(/\./g, '--');
    return `${encodedBase}${MYCUTE_PROXY_SUFFIX}`;
}

/**
 * プロキシサフィックスを除去し、ダブルハイフンをドットに戻すことでホスト名をデコードします。
 * 例: google--com.mc.shyme.net -> google.com
 * @param host エンコードされたホスト名
 * @returns デコードされた元のホスト名
 */
export function decodeHost(host: string): string {
    if (!host || !host.endsWith(MYCUTE_PROXY_SUFFIX)) {
        return host;
    }

    const suffixLen = MYCUTE_PROXY_SUFFIX.length;
    const base = host.slice(0, -suffixLen);

    // Replace double hyphens with dots
    return base.replace(/--/g, '.');
}
