<template>
  <q-dialog v-model="isOpen" persistent>
    <q-card class="bg-dark text-white" style="border-radius: 12px; max-width: 450px;">
      <q-card-section class="row items-center q-pb-none">
        <q-avatar color="negative" text-color="white" icon="warning" />
        <span class="q-ml-sm text-h6">{{ t('app.settings.expireExceedsCaTitle') }}</span>
      </q-card-section>

      <q-card-section class="q-py-md">
        <div class="text-body2 text-grey-4 style-relaxed">
          {{ t('app.settings.expireExceedsCaMessage', { caExpire: caExpireStr, maxHours: maxHours }) }}
        </div>
      </q-card-section>

      <q-card-actions align="right" class="q-pb-md q-px-md">
        <q-btn outline :label="t('app.common.cancel')" color="grey-6" @click="isOpen = false" />
        <q-btn
          color="negative"
          unelevated
          :label="t('app.common.ok')"
          @click="onConfirm"
        />
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { t, getDateStr } from 'src/utils/some'

const props = defineProps<{
  modelValue: boolean
  caExpireAt: number
  maxHours: number
}>()

const emit = defineEmits(['update:modelValue', 'confirm'])

const isOpen = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
})

const caExpireStr = computed(() => {
  if (!props.caExpireAt) return '---'
  return getDateStr(props.caExpireAt)
})

function onConfirm() {
  emit('confirm')
  isOpen.value = false
}
</script>

<style scoped>
.style-relaxed {
  line-height: 1.6;
  white-space: pre-wrap;
}
</style>
