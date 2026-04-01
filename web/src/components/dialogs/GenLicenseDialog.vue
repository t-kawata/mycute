<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px; max-width: 500px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <BookmarkPencilIcon style="width: 24px; height: 24px;" />
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.genLicenseDialogTitle') }}</span>
      </q-card-section>
      
      <q-card-section class="q-pt-none">
        <div class="text-caption q-mb-sm text-grey-5">{{ t('app.settings.targetPubKeyHint') }}</div>
        <q-input
          v-model="targetPubKey"
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
        <div class="text-caption q-mb-sm text-grey-5">{{ t('app.settings.expireHoursHint') }}</div>
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
        />
      </q-card-section>

      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" @click="onCancel" />
        <q-btn
          color="negative"
          icon="send"
          :label="t('app.settings.issueLicense')"
          :disable="!targetPubKey.trim() || expireHours <= 0"
          :loading="isIssuing"
          @click="onIssue"
        />
      </q-card-actions>
    </q-card>
  </q-dialog>

  <!-- Confirm Expiration Limit Dialog -->
  <GenLicenseExpireConfirmDialog
    v-model="isConfirming"
    :ca-expire-at="mainStore.caExpireAt || 0"
    :max-hours="maxPossibleHours"
    @confirm="onExpireConfirm"
  />
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { t } from 'src/utils/some'
import { useMainStore } from 'src/stores/main-store'
import { genLicense as apiGenLicense } from 'src/utils/rest'
import { showNotify } from 'src/utils/notify'
import GenLicenseExpireConfirmDialog from 'src/components/dialogs/GenLicenseExpireConfirmDialog.vue'
import BookmarkPencilIcon from 'src/components/icons/BookmarkPencilIcon.vue'

const defaultPermissionsJson = '{\n    "all": true\n}'
const mainStore = useMainStore()
const targetPubKey = ref('')
const expireHours = ref(336) // Default: 14 days
const permissionsJson = ref(defaultPermissionsJson)
const isIssuing = ref(false)
const isConfirming = ref(false)
const maxPossibleHours = ref(0)

const isOpen = computed({
  get: () => mainStore.isGenLicenseDialogOpen,
  set: (val) => mainStore.setIsGenLicenseDialogOpen(val)
})

function onCancel() {
  targetPubKey.value = ''
  expireHours.value = 336
  permissionsJson.value = defaultPermissionsJson
  isOpen.value = false
}

const checkExpirationLimit = () => {
  if (isConfirming.value || !mainStore.caExpireAt || !isOpen.value) return

  const now = Date.now()
  const maxPossibleMs = mainStore.caExpireAt - now
  const mHours = Math.floor(maxPossibleMs / (1000 * 60 * 60))
  maxPossibleHours.value = mHours

  if (expireHours.value > mHours) {
    isConfirming.value = true
  }
}

function onExpireConfirm() {
  expireHours.value = maxPossibleHours.value
}

watch(expireHours, () => {
  checkExpirationLimit()
})

watch(isOpen, (newVal) => {
  if (newVal) {
    // 確実にステータスが反映された後にチェック
    setTimeout(() => {
      checkExpirationLimit()
    }, 200)
  }
})

async function onIssue() {
  if (isIssuing.value) return
  
  let permissionsObj: any
  try {
    permissionsObj = JSON.parse(permissionsJson.value)
  } catch (e) {
    showNotify('Invalid JSON for permissions')
    return
  }

  isIssuing.value = true
  try {
    const res = await apiGenLicense(
      mainStore.token || '',
      targetPubKey.value.trim(),
      expireHours.value,
      permissionsObj
    )

    if (res?.license) {
      await writeText(res.license)
      showNotify(t('app.settings.genLicenseSuccess'))
      onCancel()
    } else {
      showNotify(t('app.settings.genLicenseFail'))
    }
  } catch (e) {
    showNotify(t('app.settings.genLicenseFail'))
  } finally {
    isIssuing.value = false
  }
}
</script>
