<template>
  <Transition name="__mycute-overlay">
    <div
      v-show="orchStore.isVisible"
      class="__orchestrator-overlay-container"
      :class="{
        '__orchestrator-overlay-container--recording': containerBlurDisabled,
      }"
      @mouseenter="isHovered = true"
      @mouseleave="isHovered = false"
      @mousemove="isHovered = true"
    >
      <!-- Controls (absolute positioned, matching legacy overlay) -->
      <div
        class="__orchestrator-overlay-controls"
        :style="{ opacity: isHovered ? 1 : 0 }"
      >
        <q-btn
          flat
          round
          dense
          icon="close"
          size="sm"
          color="dark"
          @click="orchStore.closeOverlay()"
        />
      </div>

      <!-- Recording STT Recognizer Layer -->
      <SttRecognizerDisplay
        :active="orchStore.isRecording"
        @enter-done="containerBlurDisabled = true"
        @leave-start="containerBlurDisabled = false"
      />

      <!-- Messages Area -->
      <div ref="messagesRef" class="__orchestrator-messages">
        <div
          v-for="(msg, index) in orchStore.messages"
          :key="index"
          class="__orchestrator-message"
          :class="'__orchestrator-message--' + msg.role"
        >
          <div
            v-if="msg.role === 'assistant'"
            v-html="renderMarkdown(msg.text)"
          />
          <div v-else class="__orchestrator-message-bubble">
            {{ msg.text }}
          </div>
        </div>

        <!-- Processing: skeleton loading (while waiting for AI) -->
        <div
          v-if="orchStore.isProcessing && !orchStore.streamingText"
          class="__orchestrator-message __orchestrator-message--assistant __orchestrator-message--skeleton"
        >
          <q-skeleton type="text" animation="fade" />
          <q-skeleton type="text" animation="fade" />
          <q-skeleton type="text" width="60%" animation="fade" />
        </div>

        <!-- Streaming response -->
        <div
          v-if="orchStore.streamingText"
          class="__orchestrator-message __orchestrator-message--assistant"
          v-html="renderMarkdown(orchStore.streamingText)"
        />
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
defineOptions({ inheritAttrs: false });
import { ref, nextTick, watch, onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useOrchestratorStore } from "src/stores/orchestrator-store";
import SttRecognizerDisplay from "src/components/tools/SttRecognizerDisplay.vue";
import MarkdownIt from "markdown-it";

const md = new MarkdownIt({ html: false, linkify: true });
const renderMarkdown = (text: string): string => {
  // リンク参照定義 [label]: URL として解釈され行全体が出力から消えるのを防ぐため、
  // 行頭の [ を \[ にエスケープする
  const escaped = text.replace(/^\[/gm, "\\[");
  // 末尾の空行を除去してからレンダリングする
  return md.render(escaped).trimEnd();
};

const EVENT_ORCHESTRATOR_DISPLAY = "orchestrator-display";
const EVENT_APP_OVERLAY_VISIBILITY = "app-overlay-visibility";

const orchStore = useOrchestratorStore();
const messagesRef = ref<HTMLDivElement | null>(null);
const isHovered = ref(false);
const containerBlurDisabled = ref(false);

// オーバーレイ再表示時に blur リセット（前回の close で leave が完了しない場合に備える）
watch(
  () => orchStore.isVisible,
  (visible) => {
    if (visible) containerBlurDisabled.value = false;
  },
);

let unlistenDisplay: UnlistenFn | null = null;
let unlistenLegacyOverlay: UnlistenFn | null = null;

// メッセージ更新時に最下部へスクロール
watch(
  () => orchStore.messages.length,
  () => {
    nextTick(scrollToBottom);
  },
);
watch(
  () => orchStore.isProcessing,
  () => {
    nextTick(scrollToBottom);
  },
);
// ストリーミング中も文字が増えるたびに最下部へスクロール
watch(
  () => orchStore.streamingText,
  () => {
    nextTick(scrollToBottom);
  },
);

const scrollToBottom = () => {
  const el = messagesRef.value;
  if (el) el.scrollTop = el.scrollHeight;
};

onMounted(async () => {
  // orchestrator-display イベントをトリガーとして状態遷移を実行する
  unlistenDisplay = await listen(EVENT_ORCHESTRATOR_DISPLAY, () => {
    orchStore.trigger();
  });

  // 排他: 従来の音声入力オーバーレイが開かれたらオーケストレーターを閉じる
  unlistenLegacyOverlay = await listen<{ visible: boolean }>(
    EVENT_APP_OVERLAY_VISIBILITY,
    (event) => {
      if (event.payload.visible && orchStore.isVisible) {
        orchStore.closeOverlay();
      }
    },
  );
});

onUnmounted(() => {
  if (unlistenDisplay) unlistenDisplay();
  if (unlistenLegacyOverlay) unlistenLegacyOverlay();
});
</script>

<style lang="scss" scoped>
@font-face {
  font-family: "MPLUSRounded1c";
  src: url("../../fonts/MPLUSRounded1c-Regular.ttf") format("truetype");
}

.__orchestrator-overlay-container {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 9990;
  display: flex;
  flex-direction: column;
  font-family: "MPLUSRounded1c", sans-serif;
  backdrop-filter: blur(15px);
  -webkit-backdrop-filter: blur(15px);
  background: rgba(255, 255, 255, 0.4);
  overflow: hidden;
  transform-origin: center center;

  &--recording {
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  &.__mycute-overlay-enter-active {
    animation: overlay-in 0.3s ease-out;
  }
  &.__mycute-overlay-leave-active {
    animation: overlay-in 0.3s ease-in reverse;
  }
}

.__orchestrator-overlay-controls {
  position: absolute;
  top: 20px;
  right: 24px;
  display: flex;
  align-items: center;
  gap: 8px;
  z-index: 10;
  transition: opacity 0.3s ease;

  :deep(.q-btn) {
    background: rgba(255, 255, 255, 0.7);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }
}

.__orchestrator-messages {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 10px;

  scrollbar-width: none;
  &::-webkit-scrollbar {
    display: none;
  }
}

.__orchestrator-message {
  display: flex;

  &--user {
    align-self: flex-end;
    max-width: 85%;
  }

  &--assistant {
    width: 100%;
    flex-direction: column;
    margin: 12px 0;
    text-shadow: 0 0 4px rgba(255, 255, 255, 0.9);
  }
}

.__orchestrator-message-bubble {
  padding: 10px 14px;
  border-radius: 14px;
  font-size: 13px;
  line-height: 1.5;
  word-break: break-word;
  white-space: pre-wrap;

  .__orchestrator-message--user & {
    background: color-mix(in srgb, var(--q-primary) 15%, transparent);
    color: var(--q-dark);
    border-bottom-right-radius: 4px;
  }
}

.__orchestrator-message--skeleton {
  gap: 8px;
  min-height: 50px;

  :deep(.q-skeleton) {
    border-radius: 4px;
  }
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

/* markdown-it レンダリング出力のスタイル調整（アシスタントメッセージ） */
.__orchestrator-message--assistant :deep(p) {
  margin: 0 0 6px;
  &:last-child {
    margin-bottom: 0;
  }
}

.__orchestrator-message--assistant :deep(code) {
  background: rgba(0, 0, 0, 0.08);
  border-radius: 3px;
  padding: 1px 4px;
  font-size: 12px;
}

.__orchestrator-message--assistant :deep(pre) {
  background: rgba(0, 0, 0, 0.08);
  border-radius: 6px;
  padding: 8px 10px;
  overflow-x: auto;
  margin: 6px 0;
}

.__orchestrator-message--assistant :deep(pre code) {
  background: none;
  padding: 0;
}
</style>
