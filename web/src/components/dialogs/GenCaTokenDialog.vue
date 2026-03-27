<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <CreditCardAIIcon style="width: 24px; height: 24px;" />
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.genCaTokenDialogTitle') }}</span>
      </q-card-section>

      <q-card-section class="q-pt-none">
        <div class="text-caption q-mb-sm text-grey-5">{{ t('app.settings.targetPubKeyHint') }}</div>
        <q-input
          v-model="pubkeyHex"
          :label="t('app.settings.targetPubKey')"
          dense
          autofocus
          bg-color="black"
          label-color="white"
          standout="bg-black text-white"
          class="q-mb-md"
        />
        <div class="text-caption q-mb-sm text-grey-5">{{ t('app.settings.expireHoursHint') }}</div>
        <q-input
          v-model.number="expireHours"
          type="number"
          :label="t('app.settings.expireHours')"
          dense
          bg-color="black"
          label-color="white"
          standout="bg-black text-white"
          @keyup.enter="onSubmit"
        />
      </q-card-section>

      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" @click="onCancel" />
        <q-btn :label="t('app.settings.issueAndCopy')" color="negative" icon="send" :loading="isLoading" @click="onSubmit" />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { useMainStore } from 'src/stores/main-store'
import { t } from 'src/utils/some'
import { showNotify, showWarn } from 'src/utils/notify'
import { genCaToken } from 'src/utils/rest'
import KeyAI1Icon from 'src/components/icons/KeyAI1Icon.vue'
import CreditCardAIIcon from '../icons/CreditCardAIIcon.vue'

const defaultExpireHours = 336 // 初期値は14日

const mainStore = useMainStore()
const pubkeyHex = ref('')
const expireHours = ref(defaultExpireHours)
const isLoading = ref(false)

const isOpen = computed({
  get: () => mainStore.isGenCaTokenDialogOpen,
  set: (val) => mainStore.setIsGenCaTokenDialogOpen(val)
})

function onCancel() {
  pubkeyHex.value = ''
  expireHours.value = defaultExpireHours
  isOpen.value = false
}

async function onSubmit() {
  if (!pubkeyHex.value) {
    showWarn(t('app.settings.enterPubKey'))
    return
  }
  if (!expireHours.value || expireHours.value <= 0) {
    showWarn(t('app.settings.enterValidHours'))
    return
  }

  isLoading.value = true
  try {
    const token = await genCaToken(pubkeyHex.value, expireHours.value)
    if (token) {
      await writeText(token)
      showNotify(t('app.settings.genCaTokenSuccess'))
      pubkeyHex.value = ''
      expireHours.value = 336
      isOpen.value = false
    } else {
      showWarn(t('app.settings.genCaTokenFail'))
    }
  } catch (e: any) {
    showWarn(t('app.settings.genCaTokenFail') + ": " + (e.message || "Unknown error"))
  } finally {
    isLoading.value = false
  }
}
</script>
