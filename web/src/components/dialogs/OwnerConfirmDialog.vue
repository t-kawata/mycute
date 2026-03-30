<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <q-icon name="vpn_key" />
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.ownerActivation') }}</span>
      </q-card-section>
      <q-card-section class="q-pt-none">
        <q-input
          v-model="passphrase"
          type="password"
          :label="t('app.settings.ownerPassphrase')"
          dense
          autofocus
          bg-color="black"
          label-color="white"
          input-class="text-white"
          standout="bg-black text-white"
          @keyup.enter="onSubmit"
        />
      </q-card-section>
      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" @click="onCancel" />
        <q-btn :label="t('app.settings.activate')" color="negative" icon="vpn_key" :loading="isLoading" @click="onSubmit" />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useMainStore } from 'src/stores/main-store'
import { t } from 'src/utils/some'
import { showWarn } from 'src/utils/notify'

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
    passphrase.value = ''
    isOpen.value = false
  } else {
    showWarn(t('app.settings.invalidPassphrase'), 2000)
  }
}
</script>
