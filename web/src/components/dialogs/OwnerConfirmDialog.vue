<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width" style="border-radius: 12px;">
      <q-card-section class="row items-center">
        <q-avatar color="warning" text-color="white">
          <q-icon name="vpn_key" />
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.ownerActivation') }}</span>
      </q-card-section>
      <q-card-section class="q-pt-none">
        <q-input
          v-model="passphrase"
          type="password"
          :label="t('app.settings.ownerPassphrase')"
          outlined
          dense
          autofocus
          @keyup.enter="onSubmit"
        />
      </q-card-section>
      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" @click="onCancel" />
        <q-btn :label="t('app.settings.activate')" color="warning" icon="vpn_key" :loading="isLoading" @click="onSubmit" />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useQuasar } from 'quasar'
import { useMainStore } from 'src/stores/main-store'
import { t } from 'src/utils/some'

const $q = useQuasar()
const mainStore = useMainStore()

const passphrase = ref('')
const isLoading = ref(false)

// Pinia ストアの isOwnerActivateConfirmOpen にバインド（ResetConfirmDialog と同一パターン）
const isOpen = computed({
  get: () => mainStore.isOwnerActivateConfirmOpen,
  set: (val) => mainStore.setIsOwnerActivateConfirmOpen(val)
})

function onCancel() {
  passphrase.value = ''
  isOpen.value = false
}

async function onSubmit() {
  if (!passphrase.value) return
  isLoading.value = true
  const success = await mainStore.activateOwner(passphrase.value)
  isLoading.value = false
  if (success) {
    $q.notify({ type: 'positive', message: t('app.settings.ownerModeActivated') })
    passphrase.value = ''
    isOpen.value = false
  } else {
    $q.notify({ type: 'negative', message: t('app.settings.invalidPassphrase') })
  }
}
</script>
