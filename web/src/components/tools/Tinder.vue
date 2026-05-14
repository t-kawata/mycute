<template>
  <div class="__mycute-tinder" :style="{ height: `calc(100dvh - 32px - 36px - ${prevHeight}px)` }">
    <div class="__mycute-tinder__container">
      <!-- スワイプインジケーター -->
      <div
        class="__mycute-tinder__indicator __mycute-tinder__indicator--like"
        :style="getIndicatorStyle(SWIPE_DIRECTION.LIKE as SwipeDirection)"
      >
        {{ t('page.index.calendar.ok') }}
      </div>
      <div
        class="__mycute-tinder__indicator __mycute-tinder__indicator--nope"
        :style="getIndicatorStyle(SWIPE_DIRECTION.NOPE as SwipeDirection)"
      >
        {{ t('page.index.calendar.ng') }}
      </div>
      <!-- カードスタック -->
      <div class="__mycute-tinder__stack">
        <div
          v-for="(card, index) in visibleCards"
          :key="card.id ?? index"
          class="__mycute-tinder__card"
          :class="{
            '__mycute-tinder__card--front': index === 0,
            '__mycute-tinder__card--animating': index === 0 && isAnimating
          }"
          :style="getCardStyle(index)"
          @mousedown="index === 0 ? startDrag($event) : null"
          @touchstart="index === 0 ? startDrag($event) : null"
        >
          <slot :card="card" :index="index + mainStore.tinderCurrentIndex">
            <div class="__mycute-tinder__card-content" :style="`background-image: url(${card.img})`">
              <div class="__mycute-tinder__card-content-title">{{ card.title }}</div>
              <div class="__mycute-tinder__card-content-subtitle"><span>{{ card.subtitle }}</span></div>
              <div class="__mycute-tinder__card-content-datetime">{{ formatDateRange(card.start, card.end, false) }}</div>
              <div class="__mycute-tinder__card-content-price">{{ calcHourlyWageStr(card.start, card.end, card.hourPrice, false) }}</div>
            </div>
          </slot>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useMainStore } from 'src/stores/main-store'
import { calcHourlyWageStr, formatDateRange, t } from 'src/utils/some'
import { SWIPE_DIRECTION, type Card, type SwipeDirection } from 'src/models/main'

interface Props {
  cards: Card[]
  prevHeight?: number
  swipeThreshold?: number
  onSwipe?: (card: Card, direction: SwipeDirection) => void
}

const props = withDefaults(defineProps<Props>(), {
  prevHeight: 400,
  swipeThreshold: 120
})

const mainStore = useMainStore()

const isDragging = ref(false)
const isAnimating = ref(false)
const startX = ref(0)
const startY = ref(0)
const currentX = ref(0)
const currentY = ref(0)
const swipeDirection = ref<SwipeDirection | null>(null)

const visibleCards = computed(() => {
  return props.cards.slice(mainStore.tinderCurrentIndex, mainStore.tinderCurrentIndex + 3)
})

const dragDistance = computed(() => currentX.value - startX.value)
const dragRotation = computed(() => (dragDistance.value / 20))

const getCardStyle = (index: number) => {
  // アニメーション時は方向ごとに正しくフレームアウト
  if (index === 0 && isAnimating.value) {
    const direction = swipeDirection.value
    const outX = direction === SWIPE_DIRECTION.LIKE ? '150vw' : direction === SWIPE_DIRECTION.NOPE ? '-150vw' : '0'
    return {
      transform: `translateX(${outX})`,
      transition: 'transform 0.35s cubic-bezier(.18,.89,.32,1.28)',
      zIndex: 30 - index
    }
  }

  // ドラッグ時は動的に制御
  if (index === 0 && isDragging.value) {
    return {
      transform: `translate(${dragDistance.value}px, ${currentY.value - startY.value}px) rotate(${dragRotation.value}deg)`,
      transition: 'none',
      zIndex: 30 - index
    }
  }

  // 背後カードは奇数は右5度, 偶数は左5度, フロントはまっすぐ
  if (index > 0) {
    const isOdd = index % 5 === 1
    const rotation = isOdd ? 5 : -5
    const scale = 1 - (index * 0.05)
    const translateY = index * 10
    return {
      transform: `translateY(${translateY}px) scale(${scale}) rotate(${rotation}deg)`,
      zIndex: 30 - index,
      opacity: 1 - (index * 0.18)
    }
  }

  // フロント静止
  return {
    transform: 'translate(0, 0) rotate(0deg)',
    zIndex: 30 - index
  }
}

// スワイプインジケーターの動作: opacity & X位置で外から現れる
const getIndicatorStyle = (type: SwipeDirection) => {
  const baseDistance = 82 // 指定したpx分だけ画面外からスライド
  const threshold = props.swipeThreshold
  let opacity = 0
  let x = 0
  if (type === SWIPE_DIRECTION.LIKE && dragDistance.value > 0) {
    opacity = Math.min(dragDistance.value / threshold, 1)
    x = Math.min(dragDistance.value / threshold, 1) * (baseDistance * -1) + Math.min(dragDistance.value, threshold)
  } else if (type === SWIPE_DIRECTION.NOPE && dragDistance.value < 0) {
    opacity = Math.min(Math.abs(dragDistance.value) / threshold, 1)
    x = Math.min(Math.abs(dragDistance.value) / threshold, 1) * baseDistance - Math.min(Math.abs(dragDistance.value), threshold)
  }
  if (!isDragging.value || opacity === 0) {
    x = type === SWIPE_DIRECTION.LIKE ? baseDistance * -1 : baseDistance
    opacity = 0
  }
  return {
    opacity,
    transform: `translateX(${x}px)`,
    transition: 'opacity 0.15s, transform 0.15s'
  }
}

const startDrag = (e: MouseEvent | TouchEvent) => {
  if (isAnimating.value || mainStore.tinderCurrentIndex >= props.cards.length) return
  isDragging.value = true
  const point = ('touches' in e && e.touches[0]) || (e as MouseEvent)
  if (!point) return
  startX.value = point.clientX
  startY.value = point.clientY
  currentX.value = point.clientX
  currentY.value = point.clientY
}
const onDrag = (e: MouseEvent | TouchEvent) => {
  if (!isDragging.value) return
  if ('touches' in e) e.preventDefault();
  const point = ('touches' in e && e.touches[0]) || (e as MouseEvent)
  if (!point) return
  currentX.value = point.clientX
  currentY.value = point.clientY
}
const endDrag = () => {
  if (!isDragging.value) return
  isDragging.value = false
  const distance = dragDistance.value
  if (Math.abs(distance) >= props.swipeThreshold) {
    swipeDirection.value = (distance > 0 ? SWIPE_DIRECTION.LIKE : SWIPE_DIRECTION.NOPE) as SwipeDirection
    isAnimating.value = true
    const direction = swipeDirection.value
    const card = props.cards[mainStore.tinderCurrentIndex] as Card
    setTimeout(() => {
      if (props.onSwipe) {
        props.onSwipe(card, direction!)
      }
      mainStore.setTinderCurrentIndex(mainStore.tinderCurrentIndex + 1)
      swipeDirection.value = null
      isAnimating.value = false
      resetPosition()
    }, 10)
  } else {
    swipeDirection.value = null
    resetPosition()
  }
}
const resetPosition = () => {
  currentX.value = startX.value
  currentY.value = startY.value
}

onMounted(() => {
  document.addEventListener('mousemove', onDrag)
  document.addEventListener('mouseup', endDrag)
  document.addEventListener('touchmove', onDrag, { passive: false })
  document.addEventListener('touchend', endDrag)
})
onUnmounted(() => {
  document.removeEventListener('mousemove', onDrag)
  document.removeEventListener('mouseup', endDrag)
  document.removeEventListener('touchmove', onDrag)
  document.removeEventListener('touchend', endDrag)
})
</script>

<style lang="scss">
.__mycute-tinder {
  width: 100vw;
  position: relative;
  background: transparent;

  &__container {
    width: 100%;
    height: 100%;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  &__stack {
    width: 94%;
    max-width: 460px;
    min-height: 70%;
    height: 100%;
    position: relative;
    margin: 0 auto;
  }

  &__card {
    position: absolute;
    width: 100%;
    height: 100%;
    background: white;
    border-radius: 20px;
    box-shadow: 0 4px 24px rgba(0,0,0,0.10), 0 1.5px 3px rgba(0,0,0,0.08);
    cursor: grab;
    transition: transform 0.35s cubic-bezier(.18,.89,.32,1.28), opacity 0.17s;
    user-select: none;
    -webkit-user-select: none;
    will-change: transform, opacity;
    &--front {
      cursor: grab;
      &:active {
        cursor: grabbing;
      }
    }
    &--animating {
      pointer-events: none;
    }
    &-content {
      width: 100%;
      height: 100%;
      font-size: 22px;
      border-radius: 20px;
      border: 3px solid #fff;
      background-size: cover;
      background-position: center;
      &-title {
        width: 100%;
        height: 25%;
        display: flex;
        justify-content: center;
        align-items: center;
        border-top-left-radius: 20px;
        border-top-right-radius: 20px;
        font-size: 26px;
        font-weight: bold;
        color: #ffffff;
        text-shadow: 1px 1px 5px rgba(0, 0, 0, 0.5);
        /* background-color: rgba(255, 0, 0, 0.3); */
      }
      &-subtitle {
        width: calc(100% - 20px);
        height: 15%;
        margin: 0 auto;
        display: flex;
        justify-content: center;
        align-items: center;
        border-radius: 10px;
        background-color: rgba(255, 255, 255, 0.45);
        // 内側のテキスト要素
        span {
          width: calc(100% - 10px);
          font-size: 16px;
          color: $dark;
          text-shadow: 1px 1px 5px rgba(255, 255, 255, 0.7);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          text-align: center;
        }
      }
      &-datetime {
        width: calc(100% - 20px);
        height: 15%;
        margin: 5px auto 0;
        display: flex;
        justify-content: center;
        align-items: center;
        font-size: 16px;
        /* font-weight: bold; */
        color: $dark;
        border-radius: 10px;
        text-shadow: 1px 1px 5px rgba(255, 255, 255, 0.7);
        background-color: rgba(255, 255, 255, 0.45);
      }
      &-price {
        width: calc(100% - 20px);
        height: calc(45% - 20px);
        margin: 5px auto 0;
        display: flex;
        justify-content: center;
        align-items: center;
        font-size: 45px;
        font-weight: bold;
        color: #ffffff;
        border-radius: 10px;
        text-shadow: 1px 1px 5px rgba(0, 0, 0, 0.3);
        background-color: rgba(255, 255, 255, 0.45);
      }
    }
  }

  &__indicator {
    position: absolute;
    top: 14%;
    font-size: 30px;
    font-weight: bold;
    height: 70px;
    line-height: 70px;
    padding: 0px 20px;
    vertical-align: middle;
    pointer-events: none;
    z-index: 60;
    transition: opacity 0.15s, transform 0.15s;
    box-shadow: 0 4px 24px rgba(0,0,0,0.10), 0 1.5px 3px rgba(0,0,0,0.08);
    &--like {
      left: -10%;
      background: $primary;
      color: #fff;
      border-top-right-radius: 35px;
      border-bottom-right-radius: 35px;
    }
    &--nope {
      right: -10%;
      background: $secondary;
      color: #fff;
      border-top-left-radius: 35px;
      border-bottom-left-radius: 35px;
    }
  }
}
</style>
