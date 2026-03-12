import { toProxyUrl } from '../utils/url';
import { MYCUTE_WS_PROXY_PATH } from '../generated_constants';

/**
 * WebSocketインターセプターを初期化します。
 * window.WebSocketをオーバーライドすることで、ws:// および wss:// のURLを
 * ローカルプロキシサーバーのエンドポイントへと自動的に書き換えます。
 * 
 * @param proxyOrigin ローカルプロキシサーバーのオリジン (例: "http://localhost:3911")
 */
export function initWebSocketInterceptor(proxyOrigin: string) {
    const OriginalWebSocket = window.WebSocket;

    // オリジナルのWebSocketを継承したプロキシクラスを作成
    class MycuteWebSocket extends OriginalWebSocket {
        constructor(url: string | URL, protocols?: string | string[]) {
            const originalUrlStr = url.toString();

            // プロキシURLの構築
            // 例: ws://target -> ws://localhost:3911/mycute_proxy_ws?target=ws://target

            // 1. プロキシのオリジンに基づいてWSスキームを決定 (http -> ws, https -> wss)
            let proxyWsScheme = 'ws:';
            if (proxyOrigin.startsWith('https:')) {
                proxyWsScheme = 'wss:';
            }

            // 2. proxyOriginからホスト部分を抽出
            const proxyHost = proxyOrigin.replace(/^https?:\/\//, '').replace(/^[a-z]+:\/\//, '');

            // 3. プロキシ用URLを生成
            const proxyUrl = `${proxyWsScheme}//${proxyHost}${MYCUTE_WS_PROXY_PATH}?target=${encodeURIComponent(originalUrlStr)}`;

            // console.log(`[MYCUTE SDK] WebSocketプロキシ実行: ${originalUrlStr} -> ${proxyUrl}`);

            super(proxyUrl, protocols);
        }
    }

    // グローバルのWebSocketオブジェクトを自作のクラスで上書き
    window.WebSocket = MycuteWebSocket;
}
