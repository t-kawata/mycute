<template>
  <q-dialog class="__harunohi-dialog" v-model="internalModel" persistent maximized transition-show="slide-right" transition-hide="slide-left">
    <q-card class="bg-white relative-position">
      <div class="__harunohi-badge-window-topbox-01">
         <div class="__harunohi-badge-window-topbox-01-box">
          <div class="full-width full-height relative-position">
            <MedicineBottole3Icon class="__harunohi-badge-window-topbox-01-box-logo-img" />
          </div>
        </div>
      </div>
      <BottomCurve01 color="#FBC5DF" />
      <div class="__harunohi-badge-window-form-area relative-position">
        <span class="block bg-yellow-light __harunohi-dec-circle-right"></span>
        <span class="block bg-primary-light __harunohi-dec-circle-right-small"></span>
        <div class="absolute full-width" style="top: -20px;">
          <p class="text-h6 text-center q-mb-xs">{{ badge?.name || 'バッジタイトル' }}</p>
          <p class="text-caption text-center text-grey-6" style="position: relative; top: -5px;">You were awarded this for your excellent work.</p>
        </div>
        <div class="full-width full-height q-px-lg q-pt-xl">
          <div class="__harunohi-badge-window-form-area-list-area">
            <q-list class="rounded-borders">
              <!----------------- 1件のリスト bgn ------------------->
              <q-item clickable v-ripple v-for="(ub, i) in usrBadges">
                <q-item-section avatar top>
                  <q-avatar>
                    <img :src="`https://cdn.quasar.dev/img/avatar${ub.from - 4}.jpg`">
                  </q-avatar>
                </q-item-section>
                <q-item-section>
                  <q-item-label class="text-primary text-weight-bold"><span class="text-secondary">{{ formatToMMDD(ub.createdAt) }} : </span>{{ ub.title }}</q-item-label>
                  <q-item-label caption>
                    <span class="text-weight-bold">{{ SAMPLE_USERS_BY_ID[ub.from - 4] ? `${SAMPLE_USERS_BY_ID[ub.from - 4]?.last_name}${SAMPLE_USERS_BY_ID[ub.from - 4]?.first_name}` : 'Name' }}</span>
                    -- {{ ub.message }}
                  </q-item-label>
                </q-item-section>
              </q-item>
              <!----------------- 1件のリスト end ------------------->
            </q-list>
          </div>
        </div>
      </div>
      <q-btn v-close-popup round flat color="secondary" class="__harunohi-control-btn-back">
        <template v-slot:default><ArrowLeftIcon /></template>
      </q-btn>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import BottomCurve01 from 'src/components/decorations/BottomCurve01.vue'
import ArrowLeftIcon from 'src/components/icons/ArrowLeftIcon.vue'
import MedicineBottole3Icon from 'src/components/icons/MedicineBottole3Icon.vue'
import { type Badge, type UsrBadge } from "src/models/main"
import { SAMPLE_USERS_BY_ID } from 'src/consts/data'

interface Props {
  modelValue?: boolean,
  badge?: Badge,
  usrBadges?: UsrBadge[]
}

/* ----------------- v-model 作成 bgn ----------------- */
const props = withDefaults(defineProps<Props>(), {
  modelValue: false,
  badge: () => { return {} as Badge },
  usrBadges: () => [] as UsrBadge[],
})
const emit = defineEmits<{ (e: 'update:modelValue', value: boolean): void }>()
const internalModel = computed({ get() { return props.modelValue }, set(val: boolean) { emit('update:modelValue', val) } })
/* ----------------- v-model 作成 end ----------------- */

const formatToMMDD = (date: Date | string): string => {
  const d = typeof date === 'string' ? new Date(date) : date;
  const month = (d.getMonth() + 1).toString().padStart(2, '0');
  const day = d.getDate().toString().padStart(2, '0');
  return `${month}/${day}`;
}
</script>
