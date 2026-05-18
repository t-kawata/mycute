<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px; max-width: 420px;">
      <q-card-section>
        <div class="text-h6 text-warning">
          <q-icon name="warning" class="q-mr-sm" />
          {{ t('app.windowsHealth.title') }}
        </div>
        <div class="text-body2 text-grey-4 q-mt-sm">
          {{ t('app.windowsHealth.description') }}
        </div>
      </q-card-section>

      <q-card-section class="q-pt-none">
        <q-list dense>
          <!-- 音声認識モデル未インストール -->
          <q-item v-if="hasIssue(1)">
            <q-item-section avatar>
              <q-icon name="mic_off" color="negative" />
            </q-item-section>
            <q-item-section>
              <q-item-label class="text-body2">
                {{ t('app.windowsHealth.noModel') }}
              </q-item-label>
              <q-item-label caption>
                <q-btn
                  flat dense no-caps
                  color="primary"
                  size="sm"
                  icon="open_in_new"
                  :label="t('app.windowsHealth.openSpeechModel')"
                  @click="openSettings('ms-settings:regionlanguage-languageoptions')"
                />
              </q-item-label>
            </q-item-section>
          </q-item>

          <!-- 音声認識プライバシートグル OFF -->
          <q-item v-if="hasIssue(2)">
            <q-item-section avatar>
              <q-icon name="lock" color="negative" />
            </q-item-section>
            <q-item-section>
              <q-item-label class="text-body2">
                {{ t('app.windowsHealth.privacyOff') }}
              </q-item-label>
              <q-item-label caption>
                <q-btn
                  flat dense no-caps
                  color="primary"
                  size="sm"
                  icon="open_in_new"
                  :label="t('app.windowsHealth.openSpeechPrivacy')"
                  @click="openSettings('ms-settings:privacy-speech')"
                />
              </q-item-label>
            </q-item-section>
          </q-item>

          <!-- マイク権限なし -->
          <q-item v-if="hasIssue(4)">
            <q-item-section avatar>
              <q-icon name="mic" color="negative" />
            </q-item-section>
            <q-item-section>
              <q-item-label class="text-body2">
                {{ t('app.windowsHealth.noMic') }}
              </q-item-label>
              <q-item-label caption>
                <q-btn
                  flat dense no-caps
                  color="primary"
                  size="sm"
                  icon="open_in_new"
                  :label="t('app.windowsHealth.openMicSettings')"
                  @click="openSettings('ms-settings:privacy-microphone')"
                />
              </q-item-label>
            </q-item-section>
          </q-item>
        </q-list>
      </q-card-section>

      <q-card-actions align="right">
        <q-btn
          outline
          :label="t('app.windowsHealth.dismiss')"
          color="grey-6"
          @click="onDismiss"
        />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { t } from 'src/utils/some'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
  modelValue: boolean
  issues: number
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const isOpen = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

function hasIssue(bit: number): boolean {
  return (props.issues & bit) !== 0
}

function onDismiss() {
  // 確認済みとしてバックエンドに通知
  invoke('acknowledge_windows_health').catch((e: unknown) => {
    console.error('Failed to acknowledge health check:', e)
  })
  isOpen.value = false
}

function openSettings(uri: string) {
  invoke('open_windows_settings', { uri }).catch((e: unknown) => {
    console.error('Failed to open Windows settings:', e)
  })
}
</script>
