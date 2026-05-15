<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <Bot2ErrorIcon style="width: 28px; height: 28px;" />
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.resetAndExit') }}</span>
      </q-card-section>
      <q-card-section class="q-pt-none" style="white-space: pre-line; line-height: 1.6;">
        {{ t('app.settings.resetAndExitConfirm') }}
      </q-card-section>
      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" v-close-popup />
        <q-btn :label="t('app.settings.resetAndExit')" color="negative" icon="delete_forever" @click="onConfirmResetAndExit" v-close-popup />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useMainStore } from 'src/stores/main-store'
import { sleep, t } from 'src/utils/some'
import { showWarn } from 'src/utils/notify'
import Bot2ErrorIcon from 'src/components/icons/Bot2ErrorIcon.vue'

const mainStore = useMainStore()

const isOpen = computed({
  get: () => mainStore.isResetAndExitConfirmOpen,
  set: (val) => mainStore.setIsResetAndExitConfirmOpen(val)
})

async function onConfirmResetAndExit() {
  mainStore.setIsLoaderOn(true)
  await sleep(300)
  try {
    await invoke('reset_and_exit')
  } catch (e) {
    console.error('Failed to reset and exit application', e)
    showWarn(t('app.settings.resetAndExitFailed'), 2000)
    mainStore.setIsLoaderOn(false)
  }
}
</script>
