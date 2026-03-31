<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="full-width bg-dark text-white" style="border-radius: 12px;">
      <q-card-section class="row items-center">
        <q-avatar color="negative" text-color="white">
          <CreditCardTrashIcon style="width: 24px; height: 24px;" />
        </q-avatar>
        <span class="q-ml-sm text-h6">{{ t('app.settings.unregCaToken') }}</span>
      </q-card-section>
      <q-card-section class="q-pt-none">
        {{ t("app.settings.unregCaTokenConfirm") }}
      </q-card-section>
      <q-card-actions align="right">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" v-close-popup />
        <q-btn :label="t('app.common.delete')" color="negative" icon="delete_forever" @click="onConfirmUnreg" v-close-popup />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useMainStore } from 'src/stores/main-store'
import { t } from 'src/utils/some'
import { showNotify } from 'src/utils/notify'
import CreditCardTrashIcon from '../icons/CreditCardTrashIcon.vue'

const mainStore = useMainStore()

const isOpen = computed({
  get: () => mainStore.isUnregisterCaTokenConfirmOpen,
  set: (val) => mainStore.setIsUnregisterCaTokenConfirmOpen(val)
})

async function onConfirmUnreg() {
  try {
    const res = await mainStore.unregisterCaToken();
    if (res && res.success) {
      showNotify(t("app.settings.unregCaTokenSuccess"));
    } else {
      const msg = res?.message || "Failed to unregister CA token.";
      showNotify(msg, 5000, "negative", "warning");
    }
  } catch (e) {
    console.error('Failed to unregister CA token', e)
    showNotify("Failed to unregister CA token.", 5000, "negative", "warning");
  }
}
</script>
