import { toProxyUrl } from '../utils/url';
import { MYCUTE_SSE_PROXY_PATH } from '../generated_constants';

/**
 * EventSource (SSE) インターセプターを初期化します。
 * window.EventSourceをオーバーライドすることで、リクエストを自動的に
 * ローカルプロキシサーバーのエンドポイントへと書き換えます。
 * 
 * @param proxyOrigin ローカルプロキシサーバーのオリジン (例: "http://localhost:8889")
 */
export function initEventSourceInterceptor(proxyOrigin: string) {
    const OriginalEventSource = window.EventSource;

    class MycuteEventSource extends OriginalEventSource {
        constructor(url: string | URL, eventSourceInitDict?: EventSourceInit) {
            const originalUrlStr = url.toString();
            const proxyUrl = `${proxyOrigin}${MYCUTE_SSE_PROXY_PATH}?target=${encodeURIComponent(originalUrlStr)}`;

            // console.log(`[MYCUTE SDK] EventSourceプロキシ実行: ${originalUrlStr} -> ${proxyUrl}`);

            super(proxyUrl, eventSourceInitDict);
        }
    }

    window.EventSource = MycuteEventSource;
}
