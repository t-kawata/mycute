<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width" style="border-radius: 12px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <Bot2ErrorIcon style="width: 28px; height: 28px;"/>
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.unregisterLicense') }}</span>
      </q-card-section>
      <q-card-section class="q-pt-none">
        {{ t("app.settings.unregisterLicenseConfirm") }}
      </q-card-section>
      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" v-close-popup />
        <q-btn :label="t('app.common.delete')" color="negative" icon="delete_forever" @click="onConfirmUnregister" v-close-popup />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useMainStore } from 'src/stores/main-store'
import { t } from 'src/utils/some'
import { showNotify } from 'src/utils/notify'
import Bot2ErrorIcon from 'src/components/icons/Bot2ErrorIcon.vue'

const mainStore = useMainStore()

const isOpen = computed({
  get: () => mainStore.isUnregisterLicenseConfirmOpen,
  set: (val) => mainStore.setIsUnregisterLicenseConfirmOpen(val)
})

async function onConfirmUnregister() {
  const id = mainStore.licenseIdToUnregister
  if (!id) return
  
  const res = await mainStore.unregisterLicense(id)
  if (res?.success) {
    showNotify(t('app.settings.licenseUnregistered'))
  }
}
</script>
