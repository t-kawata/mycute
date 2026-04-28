<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <Bot2ErrorIcon style="width: 28px; height: 28px;"/>
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.resetApplication') }}</span>
      </q-card-section>
      <q-card-section class="q-pt-none">
        {{ t("app.settings.resetConfirm") }}
      </q-card-section>
      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" v-close-popup />
        <q-btn :label="t('app.common.reset')" color="negative" icon="restore" @click="onConfirmReset" v-close-popup />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useMainStore } from 'src/stores/main-store'
import { t } from 'src/utils/some'
import { showWarn } from 'src/utils/notify'
import Bot2ErrorIcon from 'src/components/icons/Bot2ErrorIcon.vue'

const mainStore = useMainStore()

const isOpen = computed({
  get: () => mainStore.isResetConfirmOpen,
  set: (val) => mainStore.setIsResetConfirmOpen(val)
})

async function onConfirmReset() {
  try {
    await invoke('reset_application')
    localStorage.clear()
    window.location.reload()
  } catch (e) {
    console.error('Failed to reset application', e)
    showWarn(t('app.settings.resetFailed'), 2000)
  }
}
</script>
