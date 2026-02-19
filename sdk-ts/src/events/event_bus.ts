/**
 * MycuteEventBus: The OS Neural Receptor (OS神経系レセプター)
 * Rust Kernel (Mycute OS) からのシステムイベントを統一的に受信・分配するクラス。
 * 内部アプリ(Tauriネイティブ)と外部アプリ(iframe/Proxy)の違いを吸収し、
 * アプリケーション開発者に透明なイベント購読インターフェースを提供する。
 */

// イベントハンドラの型定義
type EventHandler = (payload: any) => void;

interface TauriEventPayload<T> {
    event: string;
    payload: T;
}

export class MycuteEventBus {
    private static instance: MycuteEventBus;
    private listeners: Map<string, Set<EventHandler>> = new Map();
    private isInitialized = false;
    private isNative = false; // true = 内部アプリ(Quasar), false = 外部アプリ(iframe)

    private constructor() { }

    /**
     * シングルトンインスタンスを取得
     */
    public static getInstance(): MycuteEventBus {
        if (!MycuteEventBus.instance) {
            MycuteEventBus.instance = new MycuteEventBus();
        }
        return MycuteEventBus.instance;
    }

    /**
     * レセプターの初期化
     * 環境を自動判別し、適切な下位レイヤーのリスナー(Tauri Event or postMessage)をセットアップする。
     */
    public init() {
        if (this.isInitialized) return;

        // 環境判定ロジック: window.__TAURI__ の有無
        // @ts-ignore
        this.isNative = !!(window.__TAURI__);

        if (this.isNative) {
            console.log("[MycuteSDK/EventBus] Initializing Internal Mode (Native MYCUTE API)");
            // ネイティブモードは .on() 時に遅延登録する戦略をとる（Tauri v1の仕様上、ワイルドカードリッスンが難しいため）
        } else {
            console.log("[MycuteSDK/EventBus] Initializing External Mode (Bridged via postMessage)");
            this.setupBridgedListener();
        }

        this.isInitialized = true;
    }

    /**
     * システムイベントを購読する
     * @param event イベント名 (例: EVENT_PROXY_LEAK "mycute://kernel/proxy-leak")
     * @param callback コールバック関数
     */
    public on(event: string, callback: EventHandler) {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, new Set());

            // ネイティブモードの場合、初めてそのイベントが購読されたタイミングで Tauri Event Listener を張る
            if (this.isNative) {
                this.listenNativeLazy(event);
            }
        }
        this.listeners.get(event)?.add(callback);
    }

    /**
     * イベント購読を解除する
     */
    public off(event: string, callback: EventHandler) {
        const handlers = this.listeners.get(event);
        if (handlers) {
            handlers.delete(callback);
            // オプティマイズ: リスナーがゼロになったら Tauri unlisten する処理も将来的に追加可能
        }
    }

    // --- Internal Implementation ---

    /**
     * [Native Mode] Tauri イベントリスナーの遅延登録
     */
    private async listenNativeLazy(event: string) {
        // @ts-ignore
        if (window.__TAURI__ && window.__TAURI__.event) {
            try {
                // @ts-ignore
                await window.__TAURI__.event.listen(event, (evt: TauriEventPayload<any>) => {
                    // Tauri から直接受信したイベントをローカルバスに流す
                    this.dispatchLocal(evt.event, evt.payload);
                });
                console.debug(`[MycuteSDK/EventBus] Attached native listener for: ${event}`);
            } catch (e) {
                console.error(`[MycuteSDK/EventBus] Failed to attach native listener for ${event}:`, e);
            }
        }
    }

    /**
     * [External Mode] postMessage ブリッジリスナーのセットアップ
     */
    private setupBridgedListener() {
        // シェル(親ウィンドウ)から転送されてくるメッセージを待ち受ける
        window.addEventListener("message", (event) => {
            // セキュリティ: オリジン検証は本来必要だが、プロキシ環境下では親＝Mycute Shellであることが保証される前提。
            // ただし、データ構造のチェックは必須。

            const data = event.data;

            // Mycute System Event プロトコルに従ったメッセージのみを処理
            if (data && data.type === "MYCUTE_SYSTEM_EVENT" && data.event && data.payload) {
                // console.debug(`[MycuteSDK/EventBus] Bridged event received: ${data.event}`);
                this.dispatchLocal(data.event, data.payload);
            }
        });
    }

    /**
     * ローカルリスナーへのディスパッチ (共通処理)
     */
    private dispatchLocal(event: string, payload: any) {
        const handlers = this.listeners.get(event);
        if (handlers) {
            handlers.forEach(fn => {
                try {
                    fn(payload);
                } catch (e) {
                    console.error(`[MycuteSDK/EventBus] Error in handler for ${event}:`, e);
                }
            });
        }
    }
}

export const eventBus = MycuteEventBus.getInstance();
