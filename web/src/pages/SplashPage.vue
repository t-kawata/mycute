<template>
  <q-page class="flex flex-center">
    <!-- ローダーは App.vue によって store を経由してグローバルに制御される -->
  </q-page>
</template>

<script setup lang="ts">
import { onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { decodeJwt } from 'jose';
import { useMainStore } from 'stores/main-store';
import { get, KEYS } from 'src/utils/ldb';
import { LANG, useLangSetter } from 'src/utils/some';
import { getVdrToken, getMycuteLlms } from 'src/utils/rest';
import { waitForServer, waitForWs } from 'src/utils/status';
import { URL } from 'src/router/routes';

const store = useMainStore();
const router = useRouter();

onMounted(async () => {
    console.log("SplashPage mounted. Starting orchestration...");
    store.setIsLoaderOn(true);
    
    // 【ブートストラップ・ロジックの意図】
    // SplashPage はアプリ起動時の最初のエントリポイントです。
    // この時点では Tauri の JS ブリッジ (invoke等) が完全に初期化されていない、
    // あるいは自動インポートが失敗するリスクがあるため、以下の防衛的なチェックを行います。
    // @ts-ignore
    let invoke = window.__TAURI__?.core?.invoke;
    if (!invoke) {
        try {
            // グローバルなユーティリティ（some.ts 等）を介さず、
            // 動的インポートを試みることで、依存関係の解決を強制します。
            const tauriApi = await import('@tauri-apps/api/core');
            invoke = tauriApi.invoke;
        } catch (e) {
            console.error("Failed to load Tauri invoke:", e);
        }
    }

    // 1. サーバーの疎通確認（共通ユーティリティを使用）
    // 起動直後は Rust 側のサーバー (Mode::RT) がまだ bind 中である可能性があるため、
    // 最大30秒間 (60回 * 500ms) かけてヘルスチェックを行います。
    const ready = await waitForServer(60, 500);

    if (!ready) {
        console.error("Backend server did not respond. Initiating Fail-Safe Shutdown.");
        // フェイルセーフ：バックエンドとの接続が確立できない場合、ゾンビプロセスを防ぐために
        // アプリケーション全体を道連れにして強制終了する（Fate-Sharing）。
        await invoke('force_shutdown');
        return; // ナビゲーションを防止
    }
    
    // 2. CLとRT間の WebSocket ハンドシェイク完了の確認待ち
    // 最大30秒間 (60回 * 500ms) かけてハンドシェイク完了を待つ
    const wsReady = await waitForWs(60, 500);

    if (!wsReady) {
        console.error("WebSocket Handshake was not completed. Initiating Fail-Safe Shutdown.");
        await invoke('force_shutdown');
        return;
    }

    // 2.5 バックエンドから LLM 設定を取得して初期化（サーバー準備完了後に行う）
    console.log("Syncing initial LLM settings...");
    try {
        const settings = await getMycuteLlms();
        if (settings) {
            store.setLlms(settings.llms);
            console.log("LLM settings synced successfully.");
        }
    } catch (e) {
        console.warn("Failed to sync LLM settings during splash:", e);
        // LLM設定の取得失敗は致命的ではないため、続行する
    }

    // 3. CLとRT間の WebSocket ハンドシェイク完了の確認待ち後、初期言語設定を同期
    console.log("Syncing initial language settings...");
    const lang = get<string>(KEYS.L);
    const langSetter = useLangSetter();
    let ok = false;

    if (!lang) ok = await langSetter.setLangJA(); // デフォルトは日本語
    else if (lang === LANG.EN.LONG) ok = await langSetter.setLangEN();
    else ok = await langSetter.setLangJA(); // 英語でないならひとまず日本語

    if (!ok) {
        console.error("Failed to sync initial language. Initiating Fail-Safe Shutdown.");
        await invoke('force_shutdown');
        return;
    }
    console.log("Initial language synced successfully.");

    /**
     * 【業務データの初期化 (Phase 8.17)】
     * インフラ（サーバー）の準備が整ったので、VDR トークンの取得とストアの初期化を行います。
     * KEYS.V (VDR-KEY) はシステム全体の基盤となる認証であり、
     * KEYS.T (JWT) は個別のユーザーログインセッションです。
     */
    const vdrKey = get<string>(KEYS.V);

    if (vdrKey) {
        try {
            // サーバーから VDR トークンを取得
            const vdrToken = await getVdrToken(vdrKey);
            if (vdrToken) {
                // ストアに VDR 情報を同期（同期処理を確実に行う）
                store.setVdrToken(vdrToken);
                const payload = decodeJwt(vdrToken);
                if (payload.apx_id && payload.usr_id) {
                    store.setApxID(Number(payload.apx_id));
                    store.setVdrID(Number(payload.usr_id));
                }
                console.log("VDR context initialized successfully.");

                // VDR の準備が整った後、ユーザーセッション (JWT) の有無で遷移先を決定
                const userJwt = get<string>(KEYS.T);
                store.setIsLoaderOn(false);
                
                if (userJwt) { router.replace(URL.HOME); }
                else { router.replace(URL.LOGIN); }
                return;
            }
        } catch (e) {
            console.error("Failed to initialize VDR context:", e);
        }
    }

    // VDR-KEY が存在しない、または取得に失敗した場合は初期設定（ログイン/登録）へ
    console.log("No valid VDR-KEY found. Redirecting to login/setup.");
    store.setIsLoaderOn(false);

    router.replace(URL.LOGIN);
});
</script>
