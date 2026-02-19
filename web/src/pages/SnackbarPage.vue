<template>
  <!-- スナックバー: 通知メッセージのポップアップ表示 -->
  <div class="__mycute-snackbar-container" @click="dismissNow">
    <transition name="__mycute-snackbar-fade">
      <div v-if="visible" class="__mycute-snackbar-toast">
        {{ message }}
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { EVENT_SHOW_SNACKBAR } from 'src/consts/generated_constants';

// 表示状態とメッセージ
const visible = ref(false);
const message = ref('');

// オートハイド用タイマー
let hideTimer: ReturnType<typeof setTimeout> | null = null;

// イベントリスナーの解除関数
let unlistenSnackbar: UnlistenFn | null = null;

// オートハイド秒数（ミリ秒）
const AUTO_HIDE_MS = 3000;

// スナックバーの現在のウィンドウ参照
const appWindow = getCurrentWindow();

/**
 * スナックバーを表示する。
 * 既にタイマーが動いている場合はリセットし、新しいメッセージで上書きする。
 */
function showSnackbar(msg: string) {
  // 既存タイマーをクリア
  if (hideTimer) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }

  message.value = msg;
  visible.value = true;

  // ウィンドウを表示状態にする
  appWindow.show();

  // オートハイドタイマーを設定
  hideTimer = setTimeout(() => {
    dismissNow();
  }, AUTO_HIDE_MS);
}

/**
 * クリックまたはタイマーによる即時消去。
 * フェードアウト完了後にウィンドウを非表示にする。
 */
function dismissNow() {
  if (hideTimer) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }
  visible.value = false;

  // フェードアウトアニメーション完了後にウィンドウを隠す（300ms = CSSアニメーション時間）
  setTimeout(() => {
    appWindow.hide();
  }, 300);
}

onMounted(async () => {
  // スナックバー表示イベントの受信
  // ペイロード構造: { message: string }
  unlistenSnackbar = await listen<{ message: string }>(EVENT_SHOW_SNACKBAR, (event) => {
    showSnackbar(event.payload.message);
  });
});

onUnmounted(() => {
  unlistenSnackbar?.();
  if (hideTimer) {
    clearTimeout(hideTimer);
  }
});
</script>

<style scoped>
.__mycute-snackbar-container {
  /* ウィンドウ全体をクリック可能な領域として使用 */
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  background: transparent;
  padding: 12px;
  box-sizing: border-box;
  cursor: pointer;
}

.__mycute-snackbar-toast {
  /* トースト通知のスタイル */
  color: #ffffff;
  font-size: 14px;
  font-weight: 500;
  font-family: 'M PLUS Rounded 1c', 'Hiragino Maru Gothic ProN', sans-serif;
  background: rgba(50, 50, 50, 0.9);
  border-radius: 8px;
  padding: 10px 20px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  max-width: 100%;
  text-align: center;
}

/* フェードイン/フェードアウト アニメーション */
.__mycute-snackbar-fade-enter-active {
  transition: opacity 0.2s ease-in, transform 0.2s ease-out;
}
.__mycute-snackbar-fade-leave-active {
  transition: opacity 0.3s ease-out, transform 0.3s ease-in;
}
.__mycute-snackbar-fade-enter-from {
  opacity: 0;
  transform: translateY(10px);
}
.__mycute-snackbar-fade-leave-to {
  opacity: 0;
  transform: translateY(10px);
}
</style>
