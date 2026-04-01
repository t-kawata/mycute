<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px; max-width: 500px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <CreditCardPlusCircleIcon style="width: 24px; height: 24px;" />
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.regCaTokenDialogTitle') }}</span>
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
              {{ result.success ? t('app.settings.registerCaTokenSuccess') : t('app.settings.registerCaTokenFail') }}
            </span>
          </div>
          <div v-if="result.message" class="q-mt-sm text-grey-5 text-caption">
            {{ result.message }}
          </div>
          <div v-if="result.success && result.permissions" class="q-mt-md">
            <div class="text-caption text-grey-5 q-mb-xs">{{ t('app.settings.grantedPermissions') }}</div>
            <pre class="bg-dark q-pa-sm text-caption text-grey-3" style="border-radius: 4px; overflow: auto; max-height: 120px; font-family: monospace;">{{ formattedPermissions }}</pre>
          </div>
        </div>
      </q-card-section>

      <q-card-actions align="right">
        <q-btn outline :label="result?.success ? t('app.common.close') : t('app.common.cancel')" color="grey-6" @click="onCancel" />
        <q-btn v-if="!result?.success" :label="t('app.settings.register')" color="negative" icon="how_to_reg" :loading="isLoading" :disable="!caToken.trim()" @click="onSubmit" />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useMainStore } from 'src/stores/main-store'
import { t } from 'src/utils/some'
import { showNotify, showWarn } from 'src/utils/notify'
import { registerCaToken } from 'src/utils/rest'
import { type RegisterCaTokenRes } from 'src/models/rtres'
import CreditCardPlusCircleIcon from 'src/components/icons/CreditCardPlusCircleIcon.vue'

const mainStore = useMainStore()
const caToken = ref('')
const isLoading = ref(false)
const result = ref<RegisterCaTokenRes | null>(null)

const isOpen = computed({
  get: () => mainStore.isRegisterCaTokenDialogOpen,
  set: (val) => mainStore.setIsRegisterCaTokenDialogOpen(val)
})

const formattedPermissions = computed(() => {
  if (!result.value?.permissions) return ''
  return JSON.stringify(result.value.permissions, null, 4)
})

// ダイアログが開かれた時に状態をリセットする
watch(isOpen, (newVal) => {
  if (newVal) {
    caToken.value = ''
    result.value = null
  }
})

function onCancel() {
  isOpen.value = false
}

async function onSubmit() {
  if (!caToken.value) {
    showWarn(t('app.settings.enterCaToken'))
    return
  }

  isLoading.value = true
  try {
    const res = await registerCaToken(mainStore.token, caToken.value)
    if (res) {
      result.value = res
      if (res.success) {
        showNotify(t('app.settings.registerCaTokenSuccess'))
        if (res.ca_token) {
          mainStore.setCaToken(res.ca_token)
        }
        // 成功時は少し待ってからダイアログを閉じる
        setTimeout(() => {
          onCancel()
        }, 5000)
      } else {
        showWarn(t('app.settings.registerCaTokenFail'))
      }
    } else {
      showWarn(t('app.settings.registerCaTokenFail'))
    }
  } catch (e: any) {
    showWarn(t('app.settings.registerCaTokenFail') + ": " + (e.message || "Unknown error"))
  } finally {
    isLoading.value = false
  }
}
</script>

<style scoped>
.break-all {
  word-break: break-all;
}
</style>
