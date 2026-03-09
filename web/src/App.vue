<template>
  <div v-if="IS_TAURI_DESKTOP" data-tauri-drag-region :class="['__harunohi-windows-title-bar', IS_TAURI_MAC ? '__harunohi-windows-title-bar-mac' : '']">
    {{ APP_NAME }}
    <div class="__harunohi-title-bar-actions no-drag">
      <q-fab
        flat
        round
        persistent
        v-model="isFabOpen"
        padding="2px"
        color="white"
        icon="keyboard_arrow_down"
        direction="down"
        :disable="mainStore.isOverlayVisible"
        :style="{ 'margin-right': IS_TAURI_WINDOWS ? '20px' : '10px' }"
      >
        <q-fab-action color="app" text-color="app" @click="toggleOverlay" icon="article" :class="{ 'to-hide': mainStore.isOverlayVisible }" />
        <q-fab-action color="app" text-color="app" @click="toggleAlwaysOnTop" icon="smartphone" :class="{ 'to-turn-off': mainStore.isAlwaysOnTop }" />
        <q-fab-action color="app" text-color="app" @click="shutdownMycute" icon="power_settings_new" class="to-turn-off" />
      </q-fab>
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
import { listen } from '@tauri-apps/api/event'
import { exit } from "@tauri-apps/plugin-process";
import { useMainStore } from "src/stores/main-store"
import { LANG, useLangSetter, isTauriDesktop, isTauriMac, isTauriWindows } from "src/utils/some"
import { APP_NAME } from 'src/configs/settings'
import { EVENT_APP_LOCALE_CHANGED } from './consts/generated_constants'

const mainStore = useMainStore()
const shutdownMycute = async () => { await exit(0); }
const toggleOverlay = async () => { isFabOpen.value = false; mainStore.setIsOverlayVisible(!mainStore.isOverlayVisible) }
const toggleAlwaysOnTop = async () => {
  isFabOpen.value = true
  mainStore.setIsAlwaysOnTop(!mainStore.isAlwaysOnTop)
  await toggleAlwaysOnTopOnTauri(mainStore.isAlwaysOnTop)
}
const toggleAlwaysOnTopOnTauri = async (isAlwaysOnTop: boolean) => {
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('toggle_always_on_top', { alwaysOnTop: isAlwaysOnTop })
}
const IS_TAURI_DESKTOP = isTauriDesktop()
const IS_TAURI_MAC = isTauriMac()
const IS_TAURI_WINDOWS = isTauriWindows()
const isFabOpen = ref(false)

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

async function initApp() {
  const langSetter = useLangSetter()

  // Tauri環境の場合は他プロセスからの変更を受信するためのリスナーをセット
  if (IS_TAURI_DESKTOP) {
    await listen(EVENT_APP_LOCALE_CHANGED, async (event: any) => {
      console.log(`Received ${EVENT_APP_LOCALE_CHANGED}:`, event.payload)
      const localeToken = event.payload.locale
      let ok = false
      if (localeToken === LANG.EN.SHORT) ok = await langSetter.setLangEN()
      else if (localeToken === LANG.JA.SHORT) ok = await langSetter.setLangJA()
      if (!ok) console.error(`Failed to apply locale change from other process: ${localeToken}`)
    })
  }

  // 最前面表示状態の復元
  if (IS_TAURI_DESKTOP && mainStore.isAlwaysOnTop) {
    await toggleAlwaysOnTopOnTauri(true)
  }

  // STTエンジンの設定をバックエンドに同期
  if (IS_TAURI_DESKTOP) {
    await mainStore.setSttEngine(mainStore.sttEngine)
  }
}
initApp()

onMounted(async () => {
  if (IS_TAURI_DESKTOP) {
    document.documentElement.classList.add('is-tauri-desktop')
  }
})
</script>
<style scoped lang="scss">

</style>