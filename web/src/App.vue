<template>
  <div v-if="IS_TAURI_DESKTOP && barClassName === ''" data-tauri-drag-region :class="['__harunohi-windows-title-bar', IS_TAURI_MAC ? '__harunohi-windows-title-bar-mac' : '', barClassName]">
    {{ APP_NAME }}
    <div class="__harunohi-title-bar-actions no-drag">
      <q-btn
        flat
        round
        dense
        size="sm"
        icon="article"
        :class="{ 'to-hide': isOverlayVisible }"
        @click="toggleOverlay"
      />
    </div>
  </div>
  <router-view />

  <div v-if="mainStore.isLoaderOn" class="__harunohi-loader">
    <q-linear-progress v-if="!IS_TAURI_DESKTOP" indeterminate color="primary" />
    <q-spinner-puff color="primary" size="300px" class="fixed-center" />
    <q-linear-progress v-if="!IS_TAURI_DESKTOP" color="primary" query class="fixed-bottom" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useMeta } from 'quasar'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useMainStore } from "src/stores/main-store"
import { get, KEYS } from "src/utils/ldb"
import { LANG, useLangSetter, isTauriDesktop, isTauriMac } from "src/utils/some"
import { APP_NAME } from 'src/configs/settings'
import { WINDOW_LABEL_OVERLAY, WINDOW_LABEL_SNACKBAR, EVENT_OVERLAY_VISIBILITY } from 'src/consts/generated_constants'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

const barClassName = ref('')
const isOverlayVisible = ref(false)

const toggleOverlay = async () => {
  try {
    isOverlayVisible.value = await invoke<boolean>('toggle_overlay_visibility')
  } catch(e) {
    console.error("Failed to toggle overlay:", e)
  }
}

const IS_TAURI_DESKTOP = isTauriDesktop()
const IS_TAURI_MAC = isTauriMac()

useMeta({
  title: APP_NAME,
})

// Tauri API のプリロード (デスクトップ環境での応答性向上のため)
let preloadedTauriWindow: any = null
if (IS_TAURI_DESKTOP) {
  import('@tauri-apps/api/window').then((m) => {
    preloadedTauriWindow = m.getCurrentWindow()
  }).catch(e => console.error('Failed to preload Tauri API:', e))
}

const mainStore = useMainStore()

async function initApp() {
  const langSetter = useLangSetter()
  const lang = get(KEYS.L)
  if (!lang) langSetter.setLangJA() // デフォルトは日本語
  else if (lang === LANG.EN) langSetter.setLangEN()
  else langSetter.setLangJA() // 英語でないならひとまず日本語
}
initApp()

onMounted(async () => {
  if (IS_TAURI_DESKTOP) {
    document.documentElement.classList.add('is-tauri-desktop')
    const label = await getCurrentWindow().label;
    switch (label) {
        case WINDOW_LABEL_OVERLAY: barClassName.value = '__harunohi-overlay-title-bar'; break;
        case WINDOW_LABEL_SNACKBAR: barClassName.value = '__harunohi-snackbar-title-bar'; break;
    }

    // オーバーレイの状態同期イベントを購読
    listen(EVENT_OVERLAY_VISIBILITY, (event) => {
      console.log("Overlay Visibility Event Received:", event.payload)
      isOverlayVisible.value = event.payload as boolean
    })
  }
})
</script>
<style scoped lang="scss">

</style>