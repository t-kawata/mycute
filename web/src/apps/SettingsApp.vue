<template>
  <div class="__harunohi-tabpanel-container __harunohi-tabpanel-container-settings">
    <q-list>
      <!-------------- 一行 bgn ---------------->
      <q-item class="q-px-none">
        <q-item-section avatar>
          <q-avatar color="primary" text-color="white">
            <LetterBlocksIcon class="__harunohi-icon-for-settings"/>
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label>{{ t('page.index.settings.englishMode') }}</q-item-label>
          <q-item-label caption>{{ t('page.index.settings.englishModeDescription') }}</q-item-label>
        </q-item-section>
        <q-item-section side >
          <q-toggle color="primary" v-model="isEn" val="battery" />
        </q-item-section>
      </q-item>
      <!-------------- 一行 end ---------------->
      <!-------------- 一行 bgn ---------------->
      <q-item class="q-px-none" style="margin-top: 10px;">
        <q-item-section avatar>
          <q-avatar color="primary" text-color="white">
            <q-icon name="mic" class="__harunohi-icon-for-settings"/>
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label>{{ t('page.index.settings.sttEngine') }}</q-item-label>
          <q-item-label caption>{{ t('page.index.settings.sttEngineDescription') }}</q-item-label>
        </q-item-section>
        <q-item-section side>
          <q-select
            filled
            v-model="sttEngine"
            :options="sttEngineOptions"
            emit-value
            map-options
            dense
            style="margin-top: 8px;"
          />
        </q-item-section>
      </q-item>
      <!-------------- 一行 end ---------------->
    </q-list>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useMainStore } from 'src/stores/main-store'
import { LANG, useLangSetter, t } from "src/utils/some"
import LetterBlocksIcon from 'src/components/icons/LetterBlocksIcon.vue'
import { ENGINE_OPENAI, ENGINE_OS } from 'src/consts/generated_constants'

const mainStore = useMainStore()
const langSetter = useLangSetter()

const isEn = computed({
  get() { return mainStore.lang === LANG.EN.LONG },
  set(newValue: boolean) {
    if (newValue) langSetter.setLangEN()
    else langSetter.setLangJA()
  }
})

const sttEngine = computed({
  get() { return mainStore.sttEngine },
  set(newValue: string) {
    mainStore.setSttEngine(newValue)
  }
})

const sttEngineOptions = computed(() => [
  { label: t('page.index.settings.sttEngineOs'), value: ENGINE_OS },
  { label: t('page.index.settings.sttEngineOpenAI'), value: ENGINE_OPENAI }
])
</script>
