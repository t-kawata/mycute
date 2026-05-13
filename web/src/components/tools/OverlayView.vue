<template>
  <!-- 音声認識中のテキストを表示するオーバーレイコンポーネント -->
  <Transition name="__mycute-overlay">
    <div
      v-show="mainStore.isOverlayVisible"
      class="__mycute-overlay-container"
      :class="{ '__mycute-overlay-correcting': isCorrecting }"
      :style="{ fontSize: fontSize + 'px' }"
      @mouseenter="!isHovered && !showHistory && (isHovered = true)"
      @mouseleave="isHovered && (isHovered = false)"
      @mousemove="!isHovered && !showHistory && (isHovered = true)"
    >
      <WaterRipple />
      <!-- テキスト表示領域: 履歴表示中はDOMから除去 -->
      <div
        ref="textAreaRef"
        v-if="!showHistory"
        class="__mycute-overlay-text-area"
      >
        <span
          v-for="(char, index) in chars"
          :key="index"
          :style="char.style"
          class="__mycute-overlay-char-span"
          >{{ char.text }}</span
        >
      </div>

      <!-- フォントサイズ調整ボタン（履歴表示中はDOMから除去） -->
      <div
        v-if="!showHistory"
        class="__mycute-overlay-controls"
        :style="{ opacity: isHovered ? 1 : 0 }"
      >
        <q-btn
          flat
          round
          dense
          icon="add"
          size="sm"
          color="dark"
          @click="changeFontSize(1)"
        />
        <q-btn
          flat
          round
          dense
          icon="remove"
          size="sm"
          color="dark"
          @click="changeFontSize(-1)"
        />
        <q-btn
          flat
          round
          dense
          icon="history"
          size="sm"
          color="dark"
          @click="toggleHistory()"
        />
        <q-btn
          flat
          round
          dense
          icon="close"
          size="sm"
          color="dark"
          @click="closeOverlay()"
        />
      </div>

      <!-- STT 履歴パネル -->
      <Transition name="__mycute-overlay-history">
        <div
          v-show="showHistory"
          class="__mycute-overlay-history-panel"
          @click.stop
        >
          <WaterRipple />
          <div class="__mycute-overlay-history-toolbar">
            <q-btn
              flat
              round
              dense
              icon="close"
              size="sm"
              color="dark"
              @click="showHistory = false"
            />
          </div>
          <div
            v-if="historyItems.length > 0"
            class="__mycute-overlay-history-list"
          >
            <q-card
              v-for="(item, index) in historyItems"
              :key="item.id"
              class="__mycute-overlay-history-item"
              @pointerdown="onPointerDown(index)"
              @pointerup="onPointerUp"
              @pointerleave="onPointerUp"
              @click="onHistoryItemClick(item.text)"
            >
              <q-card-section class="__mycute-overlay-history-item-section">
                <span class="__mycute-overlay-history-item-text">{{
                  item.text
                }}</span>
              </q-card-section>
            </q-card>
          </div>
          <div v-else class="__mycute-overlay-history-empty">
            {{ t("app.fab.overlay.sttHistoryEmpty") }}
          </div>
        </div>
      </Transition>
    </div>
  </Transition>

  <!-- 履歴アイテム操作用ダイアログ（QDialog scale アニメーション使用） -->
  <q-dialog
    v-model="dialogVisible"
    transition-show="scale"
    transition-hide="scale"
    :style="{ zIndex: 99999 }"
    @hide="detailItem = null"
  >
    <q-card class="__mycute-stt-dialog-card bg-dark text-white">
      <q-card-section class="__mycute-stt-dialog-card-body">
        <pre class="__mycute-stt-dialog-card-pre">{{ detailItem?.text }}</pre>
      </q-card-section>
      <q-card-actions align="right">
        <q-btn
          outline
          :label="t('app.common.close')"
          color="grey-6"
          @click="closeDetail()"
        />
        <q-btn
          :label="t('app.common.delete')"
          color="negative"
          icon="delete"
          @click="deleteDetailItem()"
        />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
defineOptions({ inheritAttrs: false })
import { ref, nextTick, watch, onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  EVENT_STT_UPDATE,
  EVENT_STT_COMMIT,
  EVENT_APP_STATUS,
  EVENT_APP_OVERLAY_VISIBILITY,
} from "src/consts/generated_constants";
import { get, set, KEYS } from "src/utils/ldb";
import { getSttHistory, deleteSttHistoryItem } from "src/utils/rest";
import { t } from "src/utils/some";
import { showNotify } from "src/utils/notify";
import WaterRipple from "src/components/effects/WaterRipple.vue";
import { useMainStore } from "src/stores/main-store";

const mainStore = useMainStore();
const opacityThreshold = 0.65;

// 文字単位のデータ構造
interface CharData {
  text: string;
  style: string; // opacity を直接指定
}

// 表示中のテキスト（文字分解後）
const chars = ref<CharData[]>([]);
// オリジナルのテキストデータ
const rawText = ref("");
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
  const spans = el.querySelectorAll(".__mycute-overlay-char-span");
  if (spans.length > 0) {
    const lineHeight = fontSize.value * 1.65;
    const containerHeight = el.clientHeight;
    const maxLines = Math.max(2, Math.floor(containerHeight / lineHeight));

    const tops = Array.from(spans).map((s) => (s as HTMLElement).offsetTop);

    const uniqueRowTops = tops
      .reduce((acc, t) => {
        if (!acc.some((exist) => Math.abs(exist - t) < 5)) {
          acc.push(t);
        }
        return acc;
      }, [] as number[])
      .sort((a, b) => b - a);

    let needsUpdate = false;
    const newChars = chars.value.map((c, i) => {
      const top = tops[i] ?? 0;
      const rowIndex = uniqueRowTops.findIndex(
        (rowTop) => Math.abs(rowTop - top) < 5,
      );
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
  chars.value = newText.split("").map((c) => ({ text: c, style: "" }));
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
    updateChars("");
  });

  unlistenAppStatus = await listen<{ status: string }>(
    EVENT_APP_STATUS,
    (event) => {
      isCorrecting.value = event.payload.status === "correcting";
    },
  );

  unlistenOverlayVisibility = await listen<{ visible: boolean }>(
    EVENT_APP_OVERLAY_VISIBILITY,
    (event) => {
      mainStore.setIsOverlayVisible(event.payload.visible);
      if (event.payload.visible) {
        showHistory.value = false;
      }
    },
  );

  window.addEventListener("resize", onResize);

  // ストアからの履歴自動表示要求を監視
  watch(() => mainStore.isOverlayHistoryRequested, (val) => {
    if (val) {
      mainStore.setIsOverlayHistoryRequested(false);
      openHistory();
    }
  });
});

onUnmounted(() => {
  if (unlistenUpdate) unlistenUpdate();
  if (unlistenCommit) unlistenCommit();
  if (unlistenAppStatus) unlistenAppStatus();
  if (unlistenOverlayVisibility) unlistenOverlayVisibility();
  window.removeEventListener("resize", onResize);
});

const closeOverlay = async () => {
  mainStore.setIsOverlayVisible(false);
  try {
    await invoke("stop_recording");
    await invoke("toggle_always_on_top", { alwaysOnTop: false });
  } catch (e) {
    console.error("Failed to cleanup recording state:", e);
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
const historyItems = ref<{ id: number; text: string; created_at: string }[]>(
  [],
);
/** 長押し検出用タイマー */
const longPressTimer = ref<ReturnType<typeof setTimeout> | null>(null);
/** 長押しが確定したかどうか */
const longPressTriggered = ref(false);
/** インライン詳細パネルの表示対象 */
const detailItem = ref<{ id: number; text: string; index: number } | null>(
  null,
);
/** QDialog の表示状態 */
const dialogVisible = ref(false);

/** 履歴パネルを開き、データをロードする（トグル不要の強制オープン用） */
const openHistory = async () => {
  if (!showHistory.value) {
    showHistory.value = true;
    isHovered.value = false;
    historyItems.value = await getSttHistory();
  }
};

/** 履歴パネルの開閉をトグルし、開くときはデータをロードする */
const toggleHistory = async () => {
  showHistory.value = !showHistory.value;
  if (showHistory.value) {
    isHovered.value = false;
    historyItems.value = await getSttHistory();
  }
};

/** 長押しハンドラ：履歴項目の詳細パネルを開く */
const dummyLongPressHandler = (_text: string) => {
  const item = historyItems.value.find((i) => i.text === _text);
  if (!item) return;
  const index = historyItems.value.indexOf(item);
  detailItem.value = { id: item.id, text: item.text, index };
  dialogVisible.value = true;
};

/** 詳細パネルを閉じる */
const closeDetail = () => {
  dialogVisible.value = false;
};

/** 詳細パネルから履歴項目を削除する */
const deleteDetailItem = async () => {
  if (!detailItem.value) return;
  const ok = await deleteSttHistoryItem(detailItem.value.id);
  if (ok) {
    historyItems.value.splice(detailItem.value.index, 1);
    detailItem.value = null;
    dialogVisible.value = false;
  }
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
    await invoke("set_clipboard", { text });
    showNotify(t("app.fab.overlay.sttHistoryCopied"));
  } catch (e) {
    console.error("Failed to copy to clipboard:", e);
  }
};
</script>

<style lang="scss" scoped>
@font-face {
  font-family: "MPLUSRounded1c";
  src: url("../../fonts/MPLUSRounded1c-Regular.ttf") format("truetype");
}

.__mycute-overlay-container {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 9990;
  display: flex;
  flex-direction: column;
  font-family: "MPLUSRounded1c", sans-serif;
  font-weight: bold;
  transition:
    font-size 0.2s ease,
    transform 0.3s ease,
    opacity 0.3s ease;
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
      text-shadow: 0 0 4px rgba(255, 255, 255, 1); // ライトテーマでは不要なためコメントアウト
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

  // STT 履歴パネル — オーバーレイと同一デザイン
  .__mycute-overlay-history-panel {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 20;
    display: flex;
    flex-direction: column;
    background: rgba(255, 255, 255, 0.4);
    backdrop-filter: blur(15px);
    -webkit-backdrop-filter: blur(15px);
    overflow: hidden;
    user-select: none;
    transform-origin: center center;

    .__mycute-overlay-history-toolbar {
      position: absolute;
      top: 20px;
      right: 24px;
      display: flex;
      gap: 8px;
      z-index: 10;
      cursor: pointer;
    }

    .__mycute-overlay-history-list {
      flex: 1;
      overflow-y: auto;
      padding: 60px 0 8px;

      .__mycute-overlay-history-item {
        margin: 0 12px 8px;
        border-radius: 12px;
        cursor: pointer;
        user-select: none;
        background: rgba(255, 255, 255, 0.5) !important;
        backdrop-filter: blur(10px);
        -webkit-backdrop-filter: blur(10px);
        box-shadow: 0 1px 4px rgba(0, 0, 0, 0.06);
        transition: transform 0.2s ease;
        will-change: transform;

        &:hover {
          transform: scale(1.03);
        }

        .__mycute-overlay-history-item-section {
          padding: 12px 16px;
          font-size: 13px;
          font-weight: normal;
          line-height: 1.5;
        }

        .__mycute-overlay-history-item-text {
          display: block;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
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
}

// 履歴パネル Transition — オーバーレイと同一アニメーション（アニメーションクラスは
// Transition の直接子要素＝ .__mycute-overlay-history-panel に付与されるため、
// .__mycute-overlay-container のネスト外で定義する）
.__mycute-overlay-history-enter-active {
  animation: overlay-in 0.3s ease-out;
}
.__mycute-overlay-history-leave-active {
  animation: overlay-in 0.3s ease-in reverse;
}
.__mycute-overlay-history-enter-from {
  opacity: 0;
  border-radius: 50%;
  transform: scale(0);
}

@keyframes overlay-in {
  0% {
    opacity: 0;
    border-radius: 50%;
    transform: scale(0);
  }
  100% {
    opacity: 1;
    border-radius: 50px;
    transform: scale(1);
  }
}

// 最終補正レイヤー実行中は背景色をわずかに青みに変化させる
.__mycute-overlay-correcting {
  background: rgba(220, 235, 255, 0.45) !important;
}
</style>

// 履歴アイテム操作用ダイアログ — QDialog は Teleport で body
直下にレンダリングされるため // scoped
スタイルが適用されない。そのため別ブロックで unscoped として定義する。
<style lang="scss">
.__mycute-stt-dialog-card {
  max-width: 90vw;
  max-height: 80vh;
  min-width: 360px;
  border-radius: 12px !important;

  .__mycute-stt-dialog-card-body {
    padding: 0;
  }

  .__mycute-stt-dialog-card-pre {
    font-size: 12px;
    font-family: inherit;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
    padding: 12px 16px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 0;
    color: inherit;
  }
}
</style>
