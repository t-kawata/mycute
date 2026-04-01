<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px; max-width: 500px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <CreditCardSearchIcon style="width: 24px; height: 24px;" />
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.verifyCaTokenDialogTitle') }}</span>
      </q-card-section>

      <q-card-section class="q-pt-none">
        <q-input
          v-model="caToken"
          type="textarea"
          :label="t('app.settings.caTokenInputLabel')"
          dense
          autofocus
          bg-color="black"
          label-color="white"
          standout="bg-black text-white"
          input-class="text-white"
          rows="4"
          class="q-mb-md"
          spellcheck="false"
          autocorrect="off"
          autocapitalize="off"
          autocomplete="off"
        />

        <div v-if="result" class="q-mt-md q-pa-md bg-black shadow-5" style="border-radius: 8px;">
          <div class="row items-center">
            <q-icon 
              :name="result.success ? 'check_circle' : 'cancel'" 
              :color="result.success ? 'primary' : 'negative'" 
              size="24px" 
              class="q-mr-sm"
            />
            <span class="text-subtitle1 text-weight-bold" :class="result.success ? 'text-primary' : 'text-negative'">
              {{ result.success ? t('app.settings.tokenValid') : t('app.settings.tokenInvalid') }}
            </span>
          </div>
          
          <q-list dense dark v-if="result.success">
            <q-item class="q-px-none q-mt-sm" style="padding-left: 0px; padding-right: 0px;">
              <q-item-section>
                <q-item-label caption class="text-grey-5">{{ t('app.settings.caPubKey') }}</q-item-label>
                <q-item-label class="text-white break-all text-caption" style="word-break: break-all;">
                  {{ result.ca_pubkey }}
                  <q-btn flat round dense icon="content_copy" size="xs" color="grey-5" class="q-ml-xs" @click="copyPubKey" />
                </q-item-label>
              </q-item-section>
            </q-item>
            <q-item class="q-px-none q-mt-sm" style="padding-left: 0px; padding-right: 0px;">
              <q-item-section>
                <q-item-label caption class="text-grey-5">{{ t('app.settings.expireAt') }}</q-item-label>
                <q-item-label class="text-white">{{ formattedExpireAt }}</q-item-label>
              </q-item-section>
            </q-item>
            <div v-if="result.permissions" class="q-mt-md">
              <div class="text-caption text-grey-5 q-mb-xs">{{ t('app.settings.grantedPermissions') }}</div>
              <pre class="bg-dark q-pa-sm text-caption text-grey-3" style="border-radius: 4px; overflow: auto; max-height: 120px; font-family: monospace;">{{ formattedPermissions }}</pre>
            </div>
          </q-list>
        </div>
      </q-card-section>

      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" @click="onCancel" />
        <q-btn :label="t('app.settings.verify')" color="negative" icon="search" :loading="isLoading" :disable="!caToken.trim()" @click="onSubmit" />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { useMainStore } from 'src/stores/main-store'
import { t, getDateStr } from 'src/utils/some'
import { showNotify, showWarn } from 'src/utils/notify'
import { verifyCaToken } from 'src/utils/rest'
import { type VerifyCaTokenRes } from 'src/models/rtres'
import CreditCardSearchIcon from 'src/components/icons/CreditCardSearchIcon.vue'

const mainStore = useMainStore()
const caToken = ref('')
const isLoading = ref(false)
const result = ref<VerifyCaTokenRes | null>(null)

const isOpen = computed({
  get: () => mainStore.isVerifyCaTokenDialogOpen,
  set: (val) => mainStore.setIsVerifyCaTokenDialogOpen(val)
})

const formattedExpireAt = computed(() => {
  if (!result.value?.expire_at) return ''
  return getDateStr(result.value.expire_at)
})

const formattedPermissions = computed(() => {
  if (!result.value?.permissions) return ''
  return JSON.stringify(result.value.permissions, null, 4)
})

function onCancel() {
  caToken.value = ''
  result.value = null
  isOpen.value = false
}

async function onSubmit() {
  if (!caToken.value) {
    showWarn(t('app.settings.enterCaToken'))
    return
  }

  isLoading.value = true
  try {
    const res = await verifyCaToken(caToken.value)
    if (res) {
      result.value = res
      if (res.success) {
        showNotify(t('app.settings.verifyCaTokenSuccess'))
      } else {
        showWarn(t('app.settings.verifyCaTokenFail'))
      }
    } else {
      showWarn(t('app.settings.verifyCaTokenFail'))
    }
  } catch (e: any) {
    showWarn(t('app.settings.verifyCaTokenFail') + ": " + (e.message || "Unknown error"))
  } finally {
    isLoading.value = false
  }
}

async function copyPubKey() {
  if (result.value?.ca_pubkey) {
    await writeText(result.value.ca_pubkey)
    showNotify(t('app.settings.copyPubKeySuccess'))
  }
}
</script>

<style scoped>
.break-all {
  word-break: break-all;
}
</style>
