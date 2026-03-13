<template>
  <div class="__harunohi-tabpanel-container __harunohi-tabpanel-container-settings">
    <q-list>
      <!-------------- 一行 bgn ---------------->
      <q-item class="q-px-none">
        <q-item-section avatar>
          <q-avatar color="primary" text-color="white">
            <q-icon name="translate" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label>{{ t('app.settings.englishMode') }}</q-item-label>
          <q-item-label caption>{{ t('app.settings.englishModeDescription') }}</q-item-label>
        </q-item-section>
        <q-item-section side >
          <q-toggle color="primary" v-model="isEn" val="battery" />
        </q-item-section>
      </q-item>
      <!-------------- 一行 end ---------------->
    </q-list>
    <!-- <div class="q-my-xl"></div>
    <p>- バッジを受け取る体験を華やかに</p>
    <p>- 各カードに、もらえるバッジ最大数をデザイン</p>
    <p>- ポイントがアマギフや景品に変わる様子</p>
    <p>- ポイントでもらえる景品等カタログ</p>
    <p>- ポイント還元分配元のプール額推移</p>
    <p>- ポイント上位者ランキング</p>
    <p>- ポイント上位者の実際の受け取り額</p> -->
  </div>
</template>
<script setup lang="ts">
import { computed } from 'vue'
import { useMainStore } from 'src/stores/main-store'
import { LANG, useLangSetter, t } from "src/utils/some"
import LetterBlocksIcon from 'src/components/icons/LetterBlocksIcon.vue'

const mainStore = useMainStore()
const langSetter = useLangSetter()

const isEn = computed({
  get() { return mainStore.lang === LANG.EN.LONG },
  set(newValue: boolean) {
    if (newValue) langSetter.setLangEN()
    else langSetter.setLangJA()
  }
})
</script>
