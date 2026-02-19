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
import { sleep } from 'src/utils/some';
import { getVdrToken } from 'src/utils/rest';
import { URL } from 'src/router/routes';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { WINDOW_LABEL_OVERLAY, WINDOW_LABEL_SNACKBAR } from 'src/consts/generated_constants';

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

    // サーバーの疎通確認（ポーリング / リトライ ロジック）
    // 起動直後は Rust 側のサーバー (Mode::RT) がまだ bind 中である可能性があるため、
    // 10秒間 (20回 * 500ms) かけてヘルスチェックを行います。
    let ready = false;
    for (let i = 0; i < 20; i++) {
        try {
            // 標準の fetch による HTTP チェックではなく、Tauri コマンド (check_server_health) を
            // 使用することで、ネットワークスタックが未準備でもプロセスレベルの疎通を確認できます。
            const isHealthy = await invoke<boolean>('check_server_health');
            if (isHealthy) {
                console.log("Server is ready.");
                ready = true;
                break;
            }
        } catch (e) {
             console.warn("Health check command failed:", e);
        }
        await sleep(500);
    }

    if (!ready) {
        console.error("Backend server did not respond. Initiating Fail-Safe Shutdown.");
        // フェイルセーフ：バックエンドとの接続が確立できない場合、ゾンビプロセスを防ぐために
        // アプリケーション全体を道連れにして強制終了する（Fate-Sharing）。
        await invoke('force_shutdown');
        return; // ナビゲーションを防止
    }

    /**
     * 【業務データの初期化 (Phase 8.17)】
     * インフラ（サーバー）の準備が整ったので、VDR トークンの取得とストアの初期化を行います。
     * KEYS.V (VDR-KEY) はシステム全体の基盤となる認証であり、
     * KEYS.T (JWT) は個別のユーザーログインセッションです。
     */
    const vdrKey = get<string>(KEYS.V);

    const label = await getCurrentWindow().label;

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
                
                if (userJwt) {
                    switch (label) {
                        case WINDOW_LABEL_OVERLAY: router.replace(URL.OVERLAY); break;
                        case WINDOW_LABEL_SNACKBAR: router.replace(URL.SNACKBAR); break;
                        default: router.replace(URL.HOME); break;
                    }
                } else { router.replace(URL.LOGIN); }
                return;
            }
        } catch (e) {
            console.error("Failed to initialize VDR context:", e);
        }
    }

    // VDR-KEY が存在しない、または取得に失敗した場合は初期設定（ログイン/登録）へ
    console.log("No valid VDR-KEY found. Redirecting to login/setup.");
    store.setIsLoaderOn(false);

    switch (label) {
        case WINDOW_LABEL_OVERLAY: router.replace(URL.OVERLAY); break;
        case WINDOW_LABEL_SNACKBAR: router.replace(URL.SNACKBAR); break;
        default: router.replace(URL.LOGIN); break;
    }
});
</script>
