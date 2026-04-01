<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px; max-width: 500px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <BookmarkPlusCircleIcon style="width: 24px; height: 24px;" />
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.regLicense') }}</span>
      </q-card-section>

      <q-card-section class="q-pt-none">
        <q-input
          v-model="licenseInputText"
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
      </q-card-section>

      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" @click="onCancel" />
        <q-btn
          color="negative"
          icon="how_to_reg"
          :label="t('app.settings.register')"
          :disable="!licenseInputText.trim()"
          :loading="isRegistering"
          @click="onRegister"
        />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useMainStore } from 'src/stores/main-store'
import { t } from 'src/utils/some'
import { showNotify } from 'src/utils/notify'
import BookmarkPlusCircleIcon from 'src/components/icons/BookmarkPlusCircleIcon.vue'

const mainStore = useMainStore()
const licenseInputText = ref('')
const isRegistering = ref(false)

const isOpen = computed({
  get: () => mainStore.isRegisterLicenseDialogOpen,
  set: (val) => mainStore.setIsRegisterLicenseDialogOpen(val)
})

function onCancel() {
  licenseInputText.value = ''
  isOpen.value = false
}

async function onRegister() {
  if (isRegistering.value) return
  isRegistering.value = true
  try {
    const res = await mainStore.registerLicense(licenseInputText.value.trim())
    if (res?.success) {
      showNotify(t('app.settings.licenseRegistered'))
      licenseInputText.value = ''
      isOpen.value = false
    } else {
      showNotify(res?.message || t('app.settings.licenseRegisterFailed'))
    }
  } finally {
    isRegistering.value = false
  }
}
</script>
