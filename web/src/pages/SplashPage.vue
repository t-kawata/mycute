<template>
  <q-page class="flex flex-center">
    <!-- ローダーは App.vue によって store を経由してグローバルに制御される -->
  </q-page>
</template>

<script setup lang="ts">
import { onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { decodeJwt } from 'jose';
import { useMainStore } from 'src/stores/main-store';
import { get, KEYS } from 'src/utils/ldb';
import { LANG, useLangSetter } from 'src/utils/some';
import { User } from 'src/models/main';
import { waitForServer, waitForWs } from 'src/utils/status';
import { initVdrContext } from 'src/utils/auth';
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
    // 最大30秒間 (120回 * 250ms) かけてヘルスチェックを行います。
    const ready = await waitForServer(120, 250);

    if (!ready) {
        console.error("Backend server did not respond. Initiating Fail-Safe Shutdown.");
        // フェイルセーフ：バックエンドとの接続が確立できない場合、ゾンビプロセスを防ぐために
        // アプリケーション全体を道連れにして強制終了する（Fate-Sharing）。
        await invoke('force_shutdown');
        return; // ナビゲーションを防止
    }
    
    // 2. CLとRT間の WebSocket ハンドシェイク完了の確認待ち
    // 最大30秒間 (120回 * 250ms) かけてハンドシェイク完了を待つ
    const wsReady = await waitForWs(120, 250);

    if (!wsReady) {
        console.error("WebSocket Handshake was not completed. Initiating Fail-Safe Shutdown.");
        await invoke('force_shutdown');
        return;
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
     * インフラ（サーバー）の準備が整ったので、セッション情報を復元します。
     * VDR-KEY (KEYS.V) に基づく VDR トークンの取得と、
     * ユーザーセッション (KEYS.T) の復元を一括で行います。
     */
    const vdrReady = await initVdrContext(store);

    if (vdrReady) {
        // VDR コンテキストの復元に成功した場合
        const userJwt = get<string>(KEYS.T);
        if (userJwt) { 
            try {
                const uPayload = decodeJwt(userJwt);
                store.setToken(userJwt);
                store.setUser({
                    id: Number(uPayload.usr_id),
                    first_name: '',
                    last_name: '',
                    apx_id: store.apxID,
                    vdr_id: store.vdrID,
                    type: Number(uPayload.type),
                    email: String(uPayload.email),
                    exp: Number(uPayload.exp),
                    is_staff: Boolean(uPayload.is_staff),
                } as User);
                console.log("Session restored. Redirecting to home.");
                store.setIsLoaderOn(false);
                router.replace(URL.HOME); 
                return;
            } catch (e) {
                console.error("Failed to decode user JWT:", e);
            }
        }
        console.log("VDR context initialized, but no user session. Redirecting to login.");
        store.setIsLoaderOn(false);
        router.replace(URL.LOGIN); 
        return;
    }

    // VDR-KEY が存在しない、または取得に失敗した場合は初期設定（ログイン/登録）へ
    console.log("No valid VDR-KEY found or initialization failed. Redirecting to login/setup.");
    store.setIsLoaderOn(false);
    router.replace(URL.LOGIN);
});
</script>
