<template>
  <div class="__mycute-web-frame-container">
    <iframe
      class="__mycute-web-frame"
      :src="urlForProxy"
      frameborder="0"
      allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
      allowfullscreen
    ></iframe>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { useMainStore } from 'src/stores/main-store'
import { getUrlForProxy } from 'src/utils/url'
import { EVENT_PROXY_LEAK } from 'src/consts/generated_constants'

const props = defineProps<{
  url: string
}>()

const mainStore = useMainStore()
const iframeRef = ref<HTMLIFrameElement | null>(null)
let cleanupListener: (() => void) | null = null

/**
 * Shell Bridge (Mycute OS Event Relay)
 * カーネル(Rust/Tauri)からのシステムイベントをリッスンし、
 * 表示中のゲストアプリ(iframe)に対して postMessage で転送する。
 */
async function setupEventBridge() {
  if (!mainStore.platform.isTauri) return

  try {
    // システムイベント: プロキシ漏れ警告
    // 将来的にはイベント名のリストを動的に管理するか、ワイルドカード化する
    const EVENTS_TO_BRIDGE = [
      EVENT_PROXY_LEAK
    ]

    const unlisteners = await Promise.all(EVENTS_TO_BRIDGE.map(eventName => {
      return listen(eventName, (event: any) => {
        if (iframeRef.value && iframeRef.value.contentWindow) {
          // iframe 内の SDK (MycuteEventBus) に向けて転送
          iframeRef.value.contentWindow.postMessage({
            type: 'MYCUTE_SYSTEM_EVENT',
            event: event.event,
            payload: event.payload
          }, '*') // プロキシ環境下ではターゲットは自明なため '*' とするが、必要に応じて urlForProxy をパースしたオリジンを指定
        }
      })
    }))

    cleanupListener = () => {
      unlisteners.forEach((fn: () => any) => fn())
    }
    
    console.debug('[ShellBridge] Initialized for:', EVENTS_TO_BRIDGE)
  } catch (e) {
    console.error('[ShellBridge] Failed to setup event bridge:', e)
  }
}

onMounted(() => {
  setupEventBridge()
})

onUnmounted(() => {
  if (cleanupListener) {
    cleanupListener()
  }
})

/**
 * 表示用の URL。
 * Tauri 環境下では、iframe の制限を回避するために自動的にプロキシスキームへと変換されます。
 */
const urlForProxy = computed(() => {
  return getUrlForProxy(props.url, mainStore.platform.isTauri)
})
</script>

<style scoped lang="scss">
.__mycute-web-frame-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background-color: #fff;
}

.__mycute-web-frame {
  flex: 1;
  width: 100%;
  height: 100%;
  border: none;
  display: block;
  /* 念のため余白を完全排除 */
  margin: 0;
  padding: 0;
}
</style>
