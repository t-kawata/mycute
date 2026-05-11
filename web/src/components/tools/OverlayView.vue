<template>
  <!-- 音声認識中のテキストを表示するオーバーレイコンポーネント -->
  <Transition name="__mycute-overlay">
    <div
      v-show="mainStore.isOverlayVisible"
      class="__mycute-overlay-container"
      :class="{ '__mycute-overlay-correcting': isCorrecting }"
      :style="{ fontSize: fontSize + 'px' }"
      @mouseenter="!isHovered && (isHovered = true)"
      @mouseleave="isHovered && (isHovered = false)"
      @mousemove="!isHovered && (isHovered = true)"
    >
      <WaterRipple />
      <!-- テキスト表示領域: 常に最新（最下部）が見えるようにスクロール制御 -->
      <div ref="textAreaRef" class="__mycute-overlay-text-area">
        <span
          v-for="(char, index) in chars"
          :key="index"
          :style="char.style"
          class="__mycute-overlay-char-span"
        >{{ char.text }}</span>
      </div>
      
      <!-- フォントサイズ調整ボタン（マウスオーバーでフェードイン） -->
      <div class="__mycute-overlay-controls" :style="{ opacity: isHovered ? 1 : 0 }">
        <q-btn flat round dense icon="add" size="sm" color="dark" @click="changeFontSize(1)" />
        <q-btn flat round dense icon="remove" size="sm" color="dark" @click="changeFontSize(-1)" />
        <q-btn flat round dense icon="history" size="sm" color="dark" @click="toggleHistory()" />
        <q-btn flat round dense icon="close" size="sm" color="dark" @click="closeOverlay()" />
      </div>

      <!-- STT 履歴パネル -->
      <Transition name="__mycute-overlay-history">
        <div v-show="showHistory" class="__mycute-overlay-history-panel" @click.stop>
          <div class="__mycute-overlay-history-header">
            <span>認識履歴</span>
            <div class="__mycute-overlay-history-header-actions">
              <q-btn flat round dense icon="delete" size="sm" color="negative" @click="clearHistoryData()" />
              <q-btn flat round dense icon="close" size="sm" color="dark" @click="showHistory = false" />
            </div>
          </div>
          <div v-if="historyItems.length > 0" class="__mycute-overlay-history-list">
            <div
              v-for="(item, index) in historyItems"
              :key="item.id"
              class="__mycute-overlay-history-item"
              @pointerdown="onPointerDown(index)"
              @pointerup="onPointerUp"
              @pointerleave="onPointerUp"
              @click="onHistoryItemClick(item.text)"
            >
              {{ item.text }}
            </div>
          </div>
          <div v-else class="__mycute-overlay-history-empty">
            履歴はありません
          </div>
        </div>
      </Transition>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { EVENT_STT_UPDATE, EVENT_STT_COMMIT, EVENT_APP_STATUS, EVENT_APP_OVERLAY_VISIBILITY } from 'src/consts/generated_constants';
import { get, set, KEYS } from 'src/utils/ldb';
import { getSttHistory, clearSttHistory } from 'src/utils/rest';
import WaterRipple from 'src/components/effects/WaterRipple.vue';
import { useMainStore } from 'src/stores/main-store';

const mainStore = useMainStore()
const opacityThreshold = 0.65

// 文字単位のデータ構造
interface CharData {
  text: string;
  style: string; // opacity を直接指定
}

// 表示中のテキスト（文字分解後）
const chars = ref<CharData[]>([]);
// オリジナルのテキストデータ
const rawText = ref('');
// フォントサイズ (初期値 18px)
const fontSize = ref(get<number>(KEYS.FS) || 18);
// マウスホバー状態
const isHovered = ref(false);
// テキスト表示領域の DOM 参照
const textAreaRef = ref<HTMLDivElement | null>(null);
// 最終補正レイヤー実行中フラグ（背景色変化用）
const isCorrecting = ref(false);

// テキスト領域を最下部にスクロールし、文字スタイルを更新する
const updateView = () => {
  const el = textAreaRef.value;
  if (!el) return;

  // 1. 文字スタイルの更新 (行判定と透明度計算)
  const spans = el.querySelectorAll('.__mycute-overlay-char-span');
  if (spans.length > 0) {
    const lineHeight = fontSize.value * 1.65;
    const containerHeight = el.clientHeight;
    const maxLines = Math.max(2, Math.floor(containerHeight / lineHeight));

    const tops = Array.from(spans).map(s => (s as HTMLElement).offsetTop);
    
    const uniqueRowTops = tops.reduce((acc, t) => {
      if (!acc.some(exist => Math.abs(exist - t) < 5)) {
        acc.push(t);
      }
      return acc;
    }, [] as number[]).sort((a, b) => b - a);

    let needsUpdate = false;
    const newChars = chars.value.map((c, i) => {
      const top = tops[i] ?? 0;
      const rowIndex = uniqueRowTops.findIndex(rowTop => Math.abs(rowTop - top) < 5);
      const safeRowIndex = rowIndex === -1 ? uniqueRowTops.length : rowIndex;

      let opacity = 0.0;
      if (safeRowIndex === 0 || safeRowIndex === 1) {
        opacity = 1.0;
      } else if (safeRowIndex === 2) {
        opacity = opacityThreshold;
      } else {
        const progress = (safeRowIndex - 2) / Math.max(1, maxLines - 2);
        opacity = opacityThreshold * (1.0 - progress);
      }
      
      opacity = Math.max(0, Math.min(1.0, opacity));
      const opacityStr = opacity.toFixed(3);
      const newStyle = `opacity: ${opacityStr}`;
      
      if (c.style !== newStyle) {
        needsUpdate = true;
        return { ...c, style: newStyle };
      }
      return c;
    });

    if (needsUpdate) {
      chars.value = newChars;
    }
  }

  // 2. 最下部スクロール
  el.scrollTop = el.scrollHeight;
};

// 生テキストから文字配列への変換
const updateChars = (newText: string) => {
  rawText.value = newText;
  chars.value = newText.split('').map(c => ({ text: c, style: '' }));
  nextTick(updateView);
};

// リサイズ時にも再計算
const onResize = () => {
  updateView();
};

let unlistenUpdate: UnlistenFn | null = null;
let unlistenCommit: UnlistenFn | null = null;
let unlistenAppStatus: UnlistenFn | null = null;
let unlistenOverlayVisibility: UnlistenFn | null = null;

onMounted(async () => {
  unlistenUpdate = await listen<{ text: string }>(EVENT_STT_UPDATE, (event) => {
    updateChars(event.payload.text);
  });

  unlistenCommit = await listen(EVENT_STT_COMMIT, () => {
    updateChars('');
  });

  unlistenAppStatus = await listen<{ status: string }>(EVENT_APP_STATUS, (event) => {
    isCorrecting.value = event.payload.status === 'correcting';
  });

  unlistenOverlayVisibility = await listen<{ visible: boolean }>(EVENT_APP_OVERLAY_VISIBILITY, (event) => {
    mainStore.setIsOverlayVisible(event.payload.visible);
  });

  window.addEventListener('resize', onResize);
});

onUnmounted(() => {
  if (unlistenUpdate) unlistenUpdate();
  if (unlistenCommit) unlistenCommit();
  if (unlistenAppStatus) unlistenAppStatus();
  if (unlistenOverlayVisibility) unlistenOverlayVisibility();
  window.removeEventListener('resize', onResize);
});

const closeOverlay = async () => {
  mainStore.setIsOverlayVisible(false);
  try {
    await invoke('stop_recording');
    await invoke('toggle_always_on_top', { alwaysOnTop: false });
  } catch (e) {
    console.error('Failed to cleanup recording state:', e);
  }
};

const changeFontSize = (delta: number) => {
  fontSize.value = Math.max(8, fontSize.value + delta);
  set(KEYS.FS, fontSize.value);
  nextTick(updateView);
};

// ============================================================
// STT 履歴パネル
// ============================================================

/** 履歴パネルの表示状態 */
const showHistory = ref(false);
/** 履歴データ */
const historyItems = ref<{ id: number; text: string; created_at: string }[]>([]);
/** 長押し検出用タイマー */
const longPressTimer = ref<ReturnType<typeof setTimeout> | null>(null);
/** 長押しが確定したかどうか */
const longPressTriggered = ref(false);

/** 履歴パネルの開閉をトグルし、開くときはデータをロードする */
const toggleHistory = async () => {
  showHistory.value = !showHistory.value;
  if (showHistory.value) {
    historyItems.value = await getSttHistory();
  }
};

/** 履歴データを全て削除する */
const clearHistoryData = async () => {
  const ok = await clearSttHistory();
  if (ok) {
    historyItems.value = [];
  }
};

/** 長押しダミーハンドラ（将来：カスタムコマンドダイアログ表示用） */
const dummyLongPressHandler = (_text: string) => {
  alert('長押しされました');
};

/** ポインター押下: 600ms 後に長押し確定 */
const onPointerDown = (index: number) => {
  longPressTriggered.value = false;
  longPressTimer.value = setTimeout(() => {
    longPressTriggered.value = true;
    dummyLongPressHandler(historyItems.value[index].text);
  }, 600);
};

/** ポインター解放: タイマーをキャンセル */
const onPointerUp = () => {
  if (longPressTimer.value) {
    clearTimeout(longPressTimer.value);
    longPressTimer.value = null;
  }
};

/** 履歴アイテムをクリック → クリップボードにコピー（長押し確定後は無視） */
const onHistoryItemClick = async (text: string) => {
  if (longPressTriggered.value) {
    longPressTriggered.value = false;
    return;
  }
  try {
    await invoke('set_clipboard', { text });
  } catch (e) {
    console.error('Failed to copy to clipboard:', e);
  }
};
</script>

<style lang="scss" scoped>
@font-face {
  font-family: 'MPLUSRounded1c';
  src: url('../../fonts/MPLUSRounded1c-Regular.ttf') format('truetype');
}

.__mycute-overlay-container {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  font-family: 'MPLUSRounded1c', sans-serif;
  font-weight: bold;
  transition: font-size 0.2s ease, transform 0.3s ease, opacity 0.3s ease;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.4); // ライトテーマのすりガラス背景
  backdrop-filter: blur(15px);
  -webkit-backdrop-filter: blur(15px);
  transform-origin: center center;

  &.__mycute-overlay-enter-active {
    animation: overlay-in 0.3s ease-out;
  }
  &.__mycute-overlay-leave-active {
    animation: overlay-in 0.3s ease-in reverse;
  }

  .__mycute-overlay-text-area {
    flex: 1;
    padding: 24px;
    overflow-y: auto;
    word-break: break-all;
    line-height: 1.65;
    display: flex;
    flex-wrap: wrap;
    align-content: center;
    justify-content: flex-start;
    
    scrollbar-width: none;
    &::-webkit-scrollbar {
      display: none;
    }
    
    .__mycute-overlay-char-span {
      transition: opacity 0.2s ease;
      white-space: pre-wrap;
      color: var(--q-dark);
      text-shadow: 0 0 4px rgba(255, 255, 255, 1.0); // ライトテーマでは不要なためコメントアウト
    }
  }

  .__mycute-overlay-controls {
    position: absolute;
    top: 20px;
    right: 24px;
    display: flex;
    gap: 8px;
    transition: opacity 0.3s ease;
    z-index: 10;
    cursor: pointer;
  }

  // STT 履歴パネル
  .__mycute-overlay-history-panel {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 20;
    display: flex;
    flex-direction: column;
    background: rgba(255, 255, 255, 0.85);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    overflow: hidden;
    user-select: none;

    .__mycute-overlay-history-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 12px 16px;
      font-size: 14px;
      font-weight: bold;
      border-bottom: 1px solid rgba(0, 0, 0, 0.1);

      .__mycute-overlay-history-header-actions {
        display: flex;
        gap: 4px;
      }
    }

    .__mycute-overlay-history-list {
      flex: 1;
      overflow-y: auto;
      padding: 8px 0;

      .__mycute-overlay-history-item {
        padding: 10px 16px;
        font-size: 13px;
        font-weight: normal;
        line-height: 1.5;
        border-bottom: 1px solid rgba(0, 0, 0, 0.05);
        cursor: pointer;
        transition: background 0.15s ease;
        user-select: none;

        &:hover {
          background: rgba(0, 0, 0, 0.06);
        }

        &:active {
          background: rgba(0, 0, 0, 0.1);
        }
      }
    }

    .__mycute-overlay-history-empty {
      flex: 1;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 14px;
      color: #999;
    }
  }

  // 履歴パネル Transition
  &.__mycute-overlay-history-enter-active {
    transition: opacity 0.2s ease;
  }
  &.__mycute-overlay-history-leave-active {
    transition: opacity 0.15s ease;
  }
  &.__mycute-overlay-history-enter-from,
  &.__mycute-overlay-history-leave-to {
    opacity: 0;
  }
}

@keyframes overlay-in {
  0% {
    opacity: 0;
    border-radius: 50%;
    transform: scale(0.0);
  }
  100% {
    opacity: 1;
    border-radius: 50px;
    transform: scale(1.0);
  }
}

// 最終補正レイヤー実行中は背景色をわずかに青みに変化させる
.__mycute-overlay-correcting {
  background: rgba(220, 235, 255, 0.45) !important;
}
</style>
