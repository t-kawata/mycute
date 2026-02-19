<template>
  <q-toolbar :class="['text-primary', 'bg-white', { 'fixed-top': !IS_TAURI_DESKTOP }]" style="z-index: 2;">
    <BadgeIcon class="__harunohi-btn-icon __harunohi-header-btn-icon __harunohi-header-btn-icon-badge" />
    <BadgeCounter :total="mainStore.subBadgeTotal" :lines="lines" :item-width="segWidth" :width="counterBarWidth" :height="23" :active="isBadgeCounterActive" />
    <q-space />
    <q-btn flat round dense color="secondary">
      <SearchIcon class="__harunohi-btn-icon __harunohi-header-btn-icon __harunohi-header-btn-icon-search" />
    </q-btn>
  </q-toolbar>
  <div class="__harunohi-tabpanel-container" :style="`margin-top: ${IS_TAURI_DESKTOP ? 0 : 34}px; height: calc(100dvh - ${IS_TAURI_DESKTOP ? 136 : 70}px)`">
    <SwipeActions
      :cards="mainStore.extraCards"
      @swipe-out="handleSwipeOut"
      @long-press="handleLongPress"
    >
      <!-- 左スロット -->
      <template #left="{ card, swipeDistance }">
        <div style="width: calc(100% - 15px); text-align: right; color: #ffffff; font-size: 35px; font-weight: bold; padding-right: 15px;">
          {{ t('page.index.search.ng') }}
        </div>
      </template>

      <!-- 右スロット -->
      <template #right="{ card, swipeDistance }">
        <div style="width: calc(100% - 15px); text-align: left; color: #ffffff; font-size: 35px; font-weight: bold; padding-left: 15px;">
          {{ t('page.index.search.ok') }}
        </div>
      </template>
    </SwipeActions>
    <div class="absolute-bottom-right relative-position __harunohi-gamaguchi-wrapper">
      <GamaguchiIcon :label="t('page.index.search.salary')" :total="mainStore.subTotal" :class="[
        'relative-position',
        '__harunohi-gamaguchi-icon',
        '__harunohi-icon-yellow-strong',
        '__harunohi-icon-border-white',
        '__harunohi-balloon-element',
        (isGamaguchiPurun ? '__harunohi-balloon-element-purun' : ''),
      ]" />
    </div>
    <div v-if="isStarMoveStart" class="__harunohi-movestar-wrapper">
      <MovingStar v-model="isStarMoveStart" />
    </div>
  </div>
</template>
<script setup lang="ts">
import { ref, computed } from 'vue'
import { useQuasar } from 'quasar'
import { useMainStore } from 'src/stores/main-store'
import SwipeActions from 'src/components/tools/SwipeActions.vue'
import { type Card, type SwipeDirection } from 'src/models/main'
import SearchIcon from 'src/components/icons/SearchIcon.vue'
import BadgeIcon from 'src/components/icons/BadgeIcon.vue'
import GamaguchiIcon from 'src/components/icons/GamaguchiIcon.vue'
import MovingStar from 'src/components/decorations/MovingStar.vue'
import BadgeCounter from 'src/components/decorations/BadgeCounter.vue'
import { sleep, t, isTauriDesktop } from 'src/utils/some'

const $q = useQuasar()
const mainStore = useMainStore()
const IS_TAURI_DESKTOP = isTauriDesktop()

const isStarMoveStart = ref(false)
const windowWidth = computed(() => $q.screen.width)
const counterBarWidth = computed(() => windowWidth.value - 72 - 20)
const segWidth = 5
const maxSegmentsPerLine = computed(() => Math.floor(counterBarWidth.value / segWidth))
const lines = computed(() => Math.ceil(mainStore.subBadgeTotal / maxSegmentsPerLine.value))
const isBadgeCounterActive = ref(false)
const isGamaguchiPurun = ref(false)

const handleSwipeOut = async ({ card, direction }: { card: Card, direction: SwipeDirection }) => {
  console.log(`${card.title} を ${direction} にスワイプしました`);
  if (direction === 'like' as SwipeDirection) {
    isStarMoveStart.value = true;
    (async () => { // バッジカウンターのウェーブを制御
      await sleep(650)
      isBadgeCounterActive.value = true
      await sleep(2000)
      isBadgeCounterActive.value = false
    })();
    (async () => { // がま口財布アイコンのぷるんを制御
      await sleep(650)
      isGamaguchiPurun.value = true
      // ここでイベントカレンダーデータに「仮」で追加
      mainStore.pushEventByCard(card)
      await sleep(2000)
      isGamaguchiPurun.value = false
    })();

  }
};

const handleLongPress = ({ card }: { card: Card }) => {
  console.log(card)
}
</script>
<style scoped lang="scss">
.__harunohi {
  &-gamaguchi {
    &-wrapper {
      width: 200px;
      height: 200px;
      z-index: 10;
      border-radius: 50%;
      right: -65px;
      bottom: -65px;
      display: flex;
      justify-content: center; /* 横方向の中央揃え */
      align-items: center;     /* 縦方向の中央揃え */
    }
    &-icon {
      cursor: pointer;
      width: 80px !important;
      right: 22px;
      bottom: 28px;
      transform: rotate(45deg);
      filter: drop-shadow(0px 1px 5px rgba(0, 0, 0, 0.2));
    }
  }
  &-movestar-wrapper {
    position: fixed;
    top: 50px;
    left: 0;
    width: 100dvw;
    height: calc(100dvh - 50px - 36px);
    z-index: 10;
  }
}
.__harunohi-balloon-element {
  animation: balloon-float 5s ease-in-out infinite;
  &-purun {
    animation: purun 0.8s linear 0s 2;
  }
}

@keyframes balloon-float {
  0%, 100% {
    transform: scale(1) rotate(45deg);
  }
  50% {
    transform: scale(1.3) rotate(20deg);

  }
}

@keyframes purun {
  0%   { transform: scale(1.0, 1.0) translate(0%, 0%); }
  15%  { transform: scale(0.9, 0.9) translate(0%, 5%); }
  30%  { transform: scale(1.3, 0.8) translate(0%, 10%); }
  50%  { transform: scale(0.8, 1.3) translate(0%, -10%); }
  70%  { transform: scale(1.1, 0.9) translate(0%, 5%); }
  100% { transform: scale(1.0, 1.0) translate(0%, 0%); }
}
</style>
