<template>
  <Transition name="__mycute-overlay">
    <div
      v-show="orchStore.isVisible"
      class="__orchestrator-overlay-container"
    >
      <!-- Header -->
      <div class="__orchestrator-overlay-header">
        <span class="__orchestrator-overlay-title">Orchestrator</span>
        <div class="__orchestrator-overlay-header-right">
          <span
            v-if="orchStore.isRecording"
            class="__orchestrator-recording-indicator"
          >
            <span class="__orchestrator-recording-dot" />
            Recording
          </span>
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
      </div>

      <!-- Messages Area -->
      <div
        ref="messagesRef"
        class="__orchestrator-messages"
      >
        <div
          v-for="(msg, index) in orchStore.messages"
          :key="index"
          class="__orchestrator-message"
          :class="'__orchestrator-message--' + msg.role"
        >
          <div class="__orchestrator-message-bubble">
            {{ msg.text }}
          </div>
        </div>

        <!-- Thinking indicator -->
        <div
          v-if="orchStore.isProcessing"
          class="__orchestrator-message __orchestrator-message--assistant"
        >
          <div class="__orchestrator-message-bubble __orchestrator-thinking-bubble">
            <ThinkingAnimation size="sm" />
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
defineOptions({ inheritAttrs: false })
import { ref, nextTick, watch, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useOrchestratorStore } from 'src/stores/orchestrator-store'
import ThinkingAnimation from 'src/components/effects/ThinkingAnimation.vue'
import { EVENT_STT_FINAL } from 'src/consts/generated_constants'

const EVENT_ORCHESTRATOR_DISPLAY = 'orchestrator-display'
const EVENT_ORCHESTRATOR_RESPONSE = 'orchestrator-response'
const EVENT_ORCHESTRATOR_TASK_COMPLETED = 'orchestrator-task-completed'

const orchStore = useOrchestratorStore()
const messagesRef = ref<HTMLDivElement | null>(null)

let unlistenDisplay: UnlistenFn | null = null
let unlistenResponse: UnlistenFn | null = null
let unlistenTaskCompleted: UnlistenFn | null = null
let unlistenSttFinal: UnlistenFn | null = null

// メッセージ更新時に最下部へスクロール
watch(() => orchStore.messages.length, () => {
  nextTick(scrollToBottom)
})
watch(() => orchStore.isProcessing, () => {
  nextTick(scrollToBottom)
})

const scrollToBottom = () => {
  const el = messagesRef.value
  if (el) el.scrollTop = el.scrollHeight
}

onMounted(async () => {
  unlistenDisplay = await listen<{ visible: boolean }>(EVENT_ORCHESTRATOR_DISPLAY, (event) => {
    if (event.payload.visible) {
      orchStore.startSession()
    } else {
      orchStore.closeOverlay()
    }
  })

  unlistenResponse = await listen<string>(EVENT_ORCHESTRATOR_RESPONSE, (event) => {
    orchStore.addAssistantMessage(event.payload)
  })

  unlistenTaskCompleted = await listen<boolean>(EVENT_ORCHESTRATOR_TASK_COMPLETED, () => {
    // タスク完了 — 必要に応じて通知表示
  })

  // STT 確定テキストをオーケストレーターに送信
  unlistenSttFinal = await listen<{ text: string; seq: number }>(EVENT_STT_FINAL, (event) => {
    if (orchStore.isVisible && event.payload.text.trim()) {
      orchStore.sendText(event.payload.text)
    }
  })
})

onUnmounted(() => {
  if (unlistenDisplay) unlistenDisplay()
  if (unlistenResponse) unlistenResponse()
  if (unlistenTaskCompleted) unlistenTaskCompleted()
  if (unlistenSttFinal) unlistenSttFinal()
})
</script>

<style lang="scss" scoped>
@font-face {
  font-family: 'MPLUSRounded1c';
  src: url('../../fonts/MPLUSRounded1c-Regular.ttf') format('truetype');
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
  font-family: 'MPLUSRounded1c', sans-serif;
  background: rgba(255, 255, 255, 0.4);
  backdrop-filter: blur(15px);
  -webkit-backdrop-filter: blur(15px);
  overflow: hidden;
  transform-origin: center center;

  &.__mycute-overlay-enter-active {
    animation: overlay-in 0.3s ease-out;
  }
  &.__mycute-overlay-leave-active {
    animation: overlay-in 0.3s ease-in reverse;
  }
}

.__orchestrator-overlay-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
  flex-shrink: 0;
}

.__orchestrator-overlay-title {
  font-size: 15px;
  font-weight: bold;
  color: var(--q-dark);
}

.__orchestrator-overlay-header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.__orchestrator-recording-indicator {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  color: #e53935;
  font-weight: bold;
}

.__orchestrator-recording-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #e53935;
  animation: recording-pulse 1.2s ease-in-out infinite;
}

@keyframes recording-pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.4; transform: scale(0.8); }
}

.__orchestrator-messages {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
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
  max-width: 85%;

  &--user {
    align-self: flex-end;
  }

  &--assistant {
    align-self: flex-start;
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
    background: rgba(25, 118, 210, 0.15);
    color: var(--q-dark);
    border-bottom-right-radius: 4px;
  }

  .__orchestrator-message--assistant & {
    background: rgba(0, 0, 0, 0.06);
    color: var(--q-dark);
    border-bottom-left-radius: 4px;
  }
}

.__orchestrator-thinking-bubble {
  display: flex;
  align-items: center;
  min-height: 32px;
  padding: 8px 16px;
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
</style>
