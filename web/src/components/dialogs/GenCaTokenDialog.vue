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
          input-class="text-white"
          class="q-mb-md"
          spellcheck="false"
          autocorrect="off"
          autocapitalize="off"
          autocomplete="off"
        />
        <q-input
          v-model.number="expireHours"
          type="number"
          :label="t('app.settings.expireHours')"
          dense
          bg-color="black"
          label-color="white"
          standout="bg-black text-white"
          input-class="text-white"
          class="q-mb-md"
        />
        <div class="text-caption q-mb-sm text-grey-5">{{ t('app.settings.permissions') }} (JSON)</div>
        <q-input
          v-model="permissionsJson"
          type="textarea"
          :label="t('app.settings.permissions')"
          dense
          bg-color="black"
          label-color="white"
          standout="bg-black text-white"
          input-class="text-white"
          autogrow
          style="font-size: 0.7rem;"
          spellcheck="false"
          autocorrect="off"
          autocapitalize="off"
          autocomplete="off"
          @keyup.enter="onSubmit"
        />
      </q-card-section>

      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" @click="onCancel" />
        <q-btn :label="t('app.settings.issueAndCopy')" color="negative" icon="send" :loading="isLoading" :disable="!pubkeyHex.trim() || !expireHours || expireHours <= 0" @click="onSubmit" />
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
import CreditCardAIIcon from 'src/components/icons/CreditCardAIIcon.vue'

const defaultExpireHours = 336 // 初期値は14日
const defaultPermissionsJson = '{\n    "all": true\n}'

const mainStore = useMainStore()
const pubkeyHex = ref('')
const expireHours = ref(defaultExpireHours)
const permissionsJson = ref(defaultPermissionsJson)
const isLoading = ref(false)

const isOpen = computed({
  get: () => mainStore.isGenCaTokenDialogOpen,
  set: (val) => mainStore.setIsGenCaTokenDialogOpen(val)
})

function onCancel() {
  pubkeyHex.value = ''
  expireHours.value = defaultExpireHours
  permissionsJson.value = defaultPermissionsJson
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

  let permissionsObj: any
  try {
    permissionsObj = JSON.parse(permissionsJson.value)
  } catch (e) {
    showWarn('Invalid JSON for permissions')
    return
  }

  isLoading.value = true
  try {
    const token = await genCaToken(pubkeyHex.value, expireHours.value, permissionsObj)
    if (token) {
      await writeText(token)
      showNotify(t('app.settings.genCaTokenSuccess'))
      onCancel()
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
