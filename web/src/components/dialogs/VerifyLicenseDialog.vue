<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px; max-width: 500px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <BookmarkSearchIcon style="width: 24px; height: 24px;" />
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.verifyLicense') }}</span>
      </q-card-section>

      <q-card-section class="q-pt-none">
        <q-input
          v-model="verifyInputText"
          type="textarea"
          :label="t('app.settings.licenseInputLabel')"
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

        <div v-if="verifyResult" class="q-mt-md q-pa-md bg-black shadow-5" style="border-radius: 8px;">
          <div class="row items-center">
            <q-icon 
              :name="verifyResult.success ? 'check_circle' : 'cancel'" 
              :color="verifyResult.success ? 'primary' : 'negative'" 
              size="24px" 
              class="q-mr-sm"
            />
            <span class="text-subtitle1 text-weight-bold" :class="verifyResult.success ? 'text-primary' : 'text-negative'">
              {{ verifyResult.success ? t('app.settings.verifyLicenseSuccess') : t('app.settings.verifyLicenseFail') }}
            </span>
          </div>
          
          <q-list dense dark v-if="verifyResult.success && verifyResult.summary">
            <q-item class="q-px-none q-mt-sm" style="padding-left: 0px; padding-right: 0px;">
              <q-item-section>
                <q-item-label caption class="text-grey-5">ID</q-item-label>
                <q-item-label class="text-white break-all text-caption" style="word-break: break-all;">
                  {{ verifyResult.summary.id }}
                </q-item-label>
              </q-item-section>
            </q-item>
            <q-item class="q-px-none q-mt-sm" style="padding-left: 0px; padding-right: 0px;">
              <q-item-section>
                <q-item-label caption class="text-grey-5">{{ t('app.settings.expireAt') }}</q-item-label>
                <q-item-label class="text-white">{{ formatExpireDate(verifyResult.summary.expire_at) }}</q-item-label>
              </q-item-section>
            </q-item>
            <div v-if="verifyResult.summary.permissions" class="q-mt-md">
              <div class="text-caption text-grey-5 q-mb-xs">{{ t('app.settings.grantedPermissions') }}</div>
              <pre class="bg-dark q-pa-sm text-caption text-grey-3" style="border-radius: 4px; overflow: auto; max-height: 120px; font-family: monospace;">{{ JSON.stringify(verifyResult.summary.permissions, null, 4) }}</pre>
            </div>
          </q-list>
        </div>
      </q-card-section>

      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" @click="onClose" />
        <q-btn
          color="negative"
          icon="check"
          :label="t('app.settings.verify')"
          :disable="!verifyInputText.trim()"
          :loading="isVerifying"
          @click="onVerify"
        />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { t, formatExpireDate } from 'src/utils/some'
import { useMainStore } from 'src/stores/main-store'
import { verifyLicense as apiVerifyLicense } from 'src/utils/rest'
import { type VerifyLicenseRes } from 'src/models/rtres'
import BookmarkSearchIcon from 'src/components/icons/BookmarkSearchIcon.vue'

const mainStore = useMainStore()
const verifyInputText = ref('')
const isVerifying = ref(false)
const verifyResult = ref<VerifyLicenseRes | null>(null)

const isOpen = computed({
  get: () => mainStore.isVerifyLicenseDialogOpen,
  set: (val) => mainStore.setIsVerifyLicenseDialogOpen(val)
})

function onClose() {
  verifyResult.value = null
  verifyInputText.value = ''
  isOpen.value = false
}

async function onVerify() {
  if (isVerifying.value) return
  isVerifying.value = true
  try {
    const res = await apiVerifyLicense(verifyInputText.value.trim())
    verifyResult.value = res
  } finally {
    isVerifying.value = false
  }
}
</script>

<style scoped>
.break-all {
  word-break: break-all;
}
</style>
