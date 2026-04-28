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
        label-position="left"
        external-label
        :disable="mainStore.isOverlayVisible"
        :style="{ 'margin-right': IS_TAURI_WINDOWS ? '20px' : '10px' }"
      >
        <q-fab-action v-if="mainStore.isLoggedIn" external-label label-position="left" color="app" text-color="app" @click="toggleOverlay" icon="article" :class="{ 'to-hide': mainStore.isOverlayVisible }" :label="mainStore.isOverlayVisible ? $t('app.fab.overlay.on') : $t('app.fab.overlay.off')" />
        <q-fab-action v-if="mainStore.isLoggedIn" external-label label-position="left" color="app" text-color="app" @click="toggleAlwaysOnTop" icon="smartphone" :class="{ 'to-turn-off': mainStore.isAlwaysOnTop }" :label="mainStore.isAlwaysOnTop ? $t('app.fab.alwaysOnTop.on') : $t('app.fab.alwaysOnTop.off')" />
        <q-fab-action v-if="mainStore.isLoggedIn" external-label label-position="left" color="app" text-color="app" @click="logout" icon="logout" :label="$t('app.fab.logout')" />
        <q-fab-action v-if="mainStore.isLoggedIn" external-label label-position="left" color="app" text-color="app" @click="restartMycute" icon="restore" :label="$t('app.fab.restart')" />
        <q-fab-action external-label label-position="left" color="app" text-color="app" @click="shutdownMycute" icon="power_settings_new" class="to-turn-off" :label="$t('app.fab.shutdown')" />
      </q-fab>
    </div>
  </div>
  <router-view />

  <div v-if="mainStore.isLoaderOn" class="__harunohi-loader">
    <q-linear-progress v-if="!IS_TAURI_DESKTOP" indeterminate color="primary" />
    <q-spinner-puff color="primary" size="300px" class="fixed-center" />
    <q-linear-progress v-if="!IS_TAURI_DESKTOP" color="primary" query class="fixed-bottom" />
  </div>
  <ResetConfirmDialog />
  <OwnerConfirmDialog />
  <GenCaTokenDialog />
  <VerifyCaTokenDialog />
  <RegisterCaTokenDialog />
  <UnregisterCaTokenConfirmDialog />
  <RegisterLicenseDialog />
  <VerifyLicenseDialog />
  <GenLicenseDialog />
  <UnregisterLicenseConfirmDialog />
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useMeta } from 'quasar'
import { useRouter } from 'vue-router';
import { listen } from '@tauri-apps/api/event'
import { exit, relaunch } from "@tauri-apps/plugin-process";
import { useMainStore } from "src/stores/main-store"
import { useLlmStore } from "src/stores/llm-store"
import { LANG, useLangSetter, isTauriDesktop, isTauriMac, isTauriWindows, t, sleep } from "src/utils/some"
import { showNotify } from 'src/utils/notify'
import { APP_NAME } from 'src/configs/settings'
import { EVENT_APP_LOCALE_CHANGED, EVENT_APP_STT_ENGINE_CHANGED, EVENT_APP_OWNER_STATUS_CHANGED, EVENT_APP_CA_STATUS_CHANGED, EVENT_APP_LICENSES_CHANGED, EVENT_APP_LMGW_PROVIDERS_CHANGED } from 'src/consts/generated_constants'
import { URL } from 'src/router/routes';
import { initVdrContext } from 'src/utils/auth';
import ResetConfirmDialog from 'src/components/dialogs/ResetConfirmDialog.vue'
import OwnerConfirmDialog from 'src/components/dialogs/OwnerConfirmDialog.vue'
import GenCaTokenDialog from 'src/components/dialogs/GenCaTokenDialog.vue'
import VerifyCaTokenDialog from 'src/components/dialogs/VerifyCaTokenDialog.vue'
import RegisterCaTokenDialog from 'src/components/dialogs/RegisterCaTokenDialog.vue'
import UnregisterCaTokenConfirmDialog from 'src/components/dialogs/UnregisterCaTokenConfirmDialog.vue'
import RegisterLicenseDialog from 'src/components/dialogs/RegisterLicenseDialog.vue'
import VerifyLicenseDialog from 'src/components/dialogs/VerifyLicenseDialog.vue'
import GenLicenseDialog from 'src/components/dialogs/GenLicenseDialog.vue'
import UnregisterLicenseConfirmDialog from 'src/components/dialogs/UnregisterLicenseConfirmDialog.vue'

const mainStore = useMainStore()
const router = useRouter();
const shutdownMycute = async () => {
  mainStore.setIsLoaderOn(true);
  await sleep(300);
  await exit(0);
}
const restartMycute = async () => { await relaunch(); }
const toggleOverlay = async () => { isFabOpen.value = false; mainStore.setIsOverlayVisible(!mainStore.isOverlayVisible) }
const toggleAlwaysOnTop = async () => {
  isFabOpen.value = true
  mainStore.setIsAlwaysOnTop(!mainStore.isAlwaysOnTop)
  await toggleAlwaysOnTopOnTauri(mainStore.isAlwaysOnTop)
}
const logout = () => { router.push(URL.LOGIN) }
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

// Tauri API のプリロード
let preloadedTauriWindow: any = null
if (IS_TAURI_DESKTOP) {
  import('@tauri-apps/api/window').then((m) => {
    preloadedTauriWindow = m.getCurrentWindow()
  }).catch(e => console.error('Failed to preload Tauri API:', e))
}

async function initApp() {
  const langSetter = useLangSetter()

  // Tauri環境の場合はバックエンドとの同期やリスナーのセットを行う
  if (IS_TAURI_DESKTOP) {
    // 他プロセスからの変更を受信するためのリスナーをセット
    await listen(EVENT_APP_LOCALE_CHANGED, async (event: any) => {
      console.log(`Received ${EVENT_APP_LOCALE_CHANGED}:`, event.payload)
      const localeToken = event.payload.locale
      let ok = false
      if (localeToken === LANG.EN.SHORT) ok = await langSetter.setLangEN()
      else if (localeToken === LANG.JA.SHORT) ok = await langSetter.setLangJA()
      if (!ok) console.error(`Failed to apply locale change from other process: ${localeToken}`)
    })

    await listen(EVENT_APP_STT_ENGINE_CHANGED, async (event: any) => {
      console.log(`Received ${EVENT_APP_STT_ENGINE_CHANGED}:`, event.payload)
      const newEngine = event.payload.engine
      if (mainStore.sttEngine !== newEngine) await mainStore.setSttEngine(newEngine)
    })

    // オーナーモードのステータス変更イベントを購読
    await listen(EVENT_APP_OWNER_STATUS_CHANGED, (event: any) => {
      console.log(`Received ${EVENT_APP_OWNER_STATUS_CHANGED}:`, event.payload)
      const isActive = !!event.payload.is_active
      
      // 状態が実際に変化した場合のみ通知を表示（初期化時の反映と区別するため）
      if (mainStore.isOwnerActive !== isActive) {
        showNotify(isActive ? t('app.settings.ownerModeActivated') : t('app.settings.ownerModeDeactivated'))
      }
      mainStore.setIsOwnerActive(isActive)
    })

    // CAステータスの変更イベントを購読
    await listen(EVENT_APP_CA_STATUS_CHANGED, (event: any) => {
      console.log(`Received ${EVENT_APP_CA_STATUS_CHANGED}:`, event.payload)
      const caToken = event.payload.ca_token || null
      mainStore.setCaToken(caToken)
    })
    
    // ライセンス変更イベントを購読
    await listen(EVENT_APP_LICENSES_CHANGED, (event: any) => {
      console.log(`Received ${EVENT_APP_LICENSES_CHANGED}:`, event.payload)
      mainStore.setLicenses(event.payload.licenses || [])
    })

    // LMGW プロバイダー変更イベントを購読
    await listen(EVENT_APP_LMGW_PROVIDERS_CHANGED, (event: any) => {
      console.log(`Received ${EVENT_APP_LMGW_PROVIDERS_CHANGED}:`, event.payload)
      const llmStore = useLlmStore()
      llmStore.setProvidersFromRaw(event.payload.providers || [])
    })

    // 最前面表示状態の復元
    if (mainStore.isAlwaysOnTop) await toggleAlwaysOnTopOnTauri(true)

    // ライセンス一覧を初期取得
    await mainStore.fetchLicenses()

    // オーナーモードの初期ステータスをバックエンドから取得して初期化
    await mainStore.fetchOwnerStatus()

    // CAステータスの初期状態を取得して初期化
    await mainStore.fetchCaStatus()

    // STTエンジンの設定をバックエンドに同期
    await mainStore.setSttEngine(mainStore.sttEngine)

    // 自分の公開鍵をバックエンドから取得
    await mainStore.fetchMyPubKey()
  }

  // VDR コンテキストの復元 (Splash 以外での起動やリロードに対応)
  await initVdrContext(mainStore)
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