<template>
  <div class="__harunohi-swipe-list-container">
    <div
      v-for="(card, index) in props.cards"
      :key="card.id"
      :class="{ '__harunohi-swipe-item-wrapper': true, 'collapsing': isCollapsing(card.id) }"
      :style="getWrapperStyle(card.id)"
    >
      <div
        class="__harunohi-swipe-actions __harunohi-swipe-actions-left bg-secondary"
        :style="{ opacity: swipeStates[card.id]?.leftOpacity || 0 }"
      >
        <slot
          name="left"
          :card="card"
          :swipeDistance="getSwipeDistance(card.id, 'nope')"
        />
      </div>
      <div
        class="__harunohi-swipe-actions __harunohi-swipe-actions-right bg-primary"
        :style="{ opacity: swipeStates[card.id]?.rightOpacity || 0 }"
      >
        <slot
          name="right"
          :card="card"
          :swipeDistance="getSwipeDistance(card.id, 'like')"
        />
      </div>

      <div
        class="__harunohi-swipe-item-content"
        :style="getItemStyle(card.id)"
        @mousedown="onDragStart($event, card)"
        @touchstart="onDragStart($event, card)"
      >
        <div class="__harunohi-swipe-item-content-item __harunohi-swipe-item-content-item-image">
          <div class="__harunohi-swipe-item-content-item-image-handle __harunohi-swipe-item-content-item-image-handle-left"><AngleDoubleLeftIcon class="__harunohi-icon-white" /></div>
          <img :src="card.imgSmall" />
          <div class="__harunohi-swipe-item-content-item-image-handle __harunohi-swipe-item-content-item-image-handle-right"><AngleDoubleRightIcon class="__harunohi-icon-white" /></div>
          <div class="__harunohi-swipe-item-content-item-image-title">{{ card.title || `Item ${card.id}` }}</div>
          <div class="__harunohi-swipe-item-content-item-image-subtitle"><span>{{ card.subtitle }}</span></div>
          <div class="__harunohi-swipe-item-content-item-image-datetime">{{ formatDateRange(card.start, card.end, false) }}</div>
          <div class="__harunohi-swipe-item-content-item-image-price">{{ calcHourlyWageStr(card.start, card.end, card.hourPrice, false) }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, CSSProperties } from 'vue';
import { type Card, type SwipeDirection } from 'src/models/main'
import { calcHourlyWageStr, formatDateRange } from 'src/utils/some'
import AngleDoubleLeftIcon from 'src/components/icons/AngleDoubleLeftIcon.vue'
import AngleDoubleRightIcon from 'src/components/icons/AngleDoubleRightIcon.vue'

// --- Types ---

interface SwipeState {
  startX: number;
  currentX: number;
  translateX: number;
  isDragging: boolean;
  isAnimating: boolean;
  isCollapsing: boolean;
  leftOpacity: number;
  rightOpacity: number;
}

// --- Props ---

const props = withDefaults(
  defineProps<{
    cards: Card[];
    threshold?: number;
    collapseDelay?: number;
    collapseDuration?: number;
  }>(),
  {
    threshold: 120,
    collapseDelay: 0,
    collapseDuration: 300,
  }
);

// --- Emits ---
const emit = defineEmits<{
  (e: 'swipe-out', payload: { card: Card; direction: SwipeDirection }): void;
}>();

// --- State ---
const swipeStates = reactive<Record<string | number, SwipeState>>({});
const activeCardId = ref<string | number | null>(null);

// --- Watchers ---
watch(
  () => props.cards,
  (newItems) => {
    newItems.forEach((item) => {
      if (!swipeStates[item.id]) {
        swipeStates[item.id] = {
          startX: 0,
          currentX: 0,
          translateX: 0,
          isDragging: false,
          isAnimating: false,
          isCollapsing: false,
          leftOpacity: 0,
          rightOpacity: 0,
        };
      }
    });
  },
  { immediate: true, deep: true }
);

// --- Helpers ---
const isCollapsing = (id: string | number): boolean => {
  const state = swipeStates[id];
  return state ? state.isCollapsing : false;
};

const getWrapperStyle = (id: string | number): CSSProperties => {
  const state = swipeStates[id];
  if (!state || !state.isCollapsing) return {};
  return {
    maxHeight: '0px',
    marginBottom: '0px',
    paddingTop: '0px',
    paddingBottom: '0px',
    borderBottomWidth: '0px',
    transition: `all ${props.collapseDuration}ms cubic-bezier(0.4, 0, 0.2, 1)`,
  };
};

const getItemStyle = (id: string | number): CSSProperties => {
  const state = swipeStates[id];
  if (!state) return {};
  const transition = state.isDragging
    ? 'none'
    : 'transform 0.3s cubic-bezier(0.2, 0, 0, 1)';
  return {
    transform: `translateX(${state.translateX}px)`,
    transition: transition,
    zIndex: activeCardId.value === id || state.isAnimating ? 10 : 1,
    pointerEvents: state.isAnimating ? 'none' : 'auto',
  };
};

const getSwipeDistance = (
  id: string | number,
  direction: SwipeDirection
) => {
  const state = swipeStates[id];
  if (!state) return 0;
  if (direction === 'nope' && state.translateX < 0) {
    return Math.abs(state.translateX);
  }
  if (direction === 'like' && state.translateX > 0) {
    return state.translateX;
  }
  return 0;
};

const getClientX = (event: MouseEvent | TouchEvent): number => {
  if ('touches' in event) {
    return event.touches[0]?.clientX || (event as TouchEvent).changedTouches[0]?.clientX || 0;
  }
  return (event as MouseEvent).clientX;
};

// --- Event Handlers ---
const onDragStart = (event: MouseEvent | TouchEvent, item: Card) => {
  if (activeCardId.value) return;
  const state = swipeStates[item.id];
  if (!state || state.isAnimating || state.isCollapsing) return;

  // 画像ドラッグなどのブラウザ標準動作を抑制（入力フォーム等でない場合のみ）
  const target = event.target as HTMLElement;
  if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
    event.preventDefault();
  }

  activeCardId.value = item.id;
  state.isDragging = true;
  state.startX = getClientX(event);
  state.currentX = state.startX;

  window.addEventListener('mousemove', onDragMove);
  window.addEventListener('touchmove', onDragMove, { passive: false });
  window.addEventListener('mouseup', onDragEnd);
  window.addEventListener('touchend', onDragEnd);
};

const onDragMove = (event: MouseEvent | TouchEvent) => {
  if (!activeCardId.value) return;
  const state = swipeStates[activeCardId.value];
  if (!state || !state.isDragging) return;

  const clientX = getClientX(event);
  const deltaX = clientX - state.currentX;
  const totalDeltaX = clientX - state.startX;

  if (Math.abs(totalDeltaX) > 10) {
    event.preventDefault();
  }

  state.currentX = clientX;
  state.translateX = totalDeltaX;

  if (state.translateX > 0) {
    const opacity = Math.min(state.translateX / props.threshold, 1);
    state.rightOpacity = opacity;
    state.leftOpacity = 0;
  } else if (state.translateX < 0) {
    const opacity = Math.min(Math.abs(state.translateX) / props.threshold, 1);
    state.leftOpacity = opacity;
    state.rightOpacity = 0;
  } else {
    state.rightOpacity = 0;
    state.leftOpacity = 0;
  }
};

const onDragEnd = () => {
  if (!activeCardId.value) return;
  const cardId = activeCardId.value;
  const state = swipeStates[cardId];
  const card = props.cards.find((i) => i.id === cardId);

  if (!state || !card) return;
  state.isDragging = false;
  window.removeEventListener('mousemove', onDragMove);
  window.removeEventListener('touchmove', onDragMove);
  window.removeEventListener('mouseup', onDragEnd);
  window.removeEventListener('touchend', onDragEnd);

  const finalTranslateX = state.translateX;
  let direction: SwipeDirection | null = null;
  let targetTranslateX = 0;

  if (finalTranslateX > props.threshold) {
    targetTranslateX = window.innerWidth;
    direction = 'like';
  } else if (finalTranslateX < -props.threshold) {
    targetTranslateX = -window.innerWidth;
    direction = 'nope';
  } else {
    targetTranslateX = 0;
  }

  state.isAnimating = true;
  state.translateX = targetTranslateX;

  state.rightOpacity = 0;
  state.leftOpacity = 0;

  setTimeout(() => {
    state.isAnimating = false;
    activeCardId.value = null;

    if (direction) {
      setTimeout(() => {
        state.isCollapsing = true;
        setTimeout(() => {
          emit('swipe-out', { card: card, direction });
        }, props.collapseDuration);
      }, props.collapseDelay);
    }
  }, 10);
};
</script>

<style scoped lang="scss">
.__harunohi {
  &-swipe {
    &-list-container {
      width: 100%;
    }
    &-item {
      &-wrapper {
        position: relative;
        max-height: 500px;
        transition: none;
        margin-bottom: 5px;
        &.collapsing {
          pointer-events: none;
        }
      }
      &-content {
        position: relative;
        display: flex;
        align-items: center;
        width: 100%;
        background-color: #ffffff;
        z-index: 2;
        cursor: grab;
        user-select: none;
        -webkit-user-select: none;
        -moz-user-select: none;
        -ms-user-select: none;
        -webkit-touch-callout: none;
        border-radius: 23px;
        &:active {
          cursor: grabbing;
        }
        &-item {
          display: flex;
          &-image {
            position: relative;
            flex-shrink: 0;
            width: calc(100dvw - 32px);
            height: 150px;
            border-radius: 23px;
            border: 2px solid #ffffff;
            box-shadow: 0px 1px 5px 0px rgba(0, 0, 0, 0.15);
            overflow: hidden;
            &-handle {
              width: 28px;
              height: 100%;
              display: flex;
              justify-content: center;
              align-items: center;
              &-left {
                background-color: $secondary-light;
                border-top-left-radius: 22px;
                border-bottom-left-radius: 22px;
                border-right: 2px solid #ffffff;
              }
              &-right {
                background-color: $primary-light;
                border-top-right-radius: 22px;
                border-bottom-right-radius: 22px;
                border-left: 2px solid #ffffff;
              }
            }
            img {
              width: calc(100dvw - 24px - 56px - 4px - 4px);
              height: 100%;
              object-fit: cover;
              display: block;
              -webkit-user-drag: none;
              user-select: none;
              -webkit-user-select: none;
              -moz-user-select: none;
              -ms-user-select: none;
            }
            &-title {
              position: absolute;
              top: 0;
              left: 0;
              right: 0;
              width: calc(100% - 60px);
              margin: 0 auto;
              height: 40px;
              line-height: 40px;
              text-align: center;
              font-size: clamp(14px, 4vw, 20px);
              white-space: nowrap;
              overflow: hidden;
              text-overflow: ellipsis;
              font-weight: bold;
              color: #ffffff;
              text-shadow: 1px 1px 5px rgba(0, 0, 0, 0.5);
              user-select: none;
              -webkit-user-select: none;
              -moz-user-select: none;
              -ms-user-select: none;
            }
            &-subtitle,
            &-datetime,
            &-price {
              position: absolute;
              left: 0;
              right: 0;
              margin: 0 auto;
              width: calc(100% - 60px);
              height: 20px;
              line-height: 20px;
              text-align: center;
              font-size: 13px;
              background-color: rgba(255, 255, 255, 0.45);
              border-radius: 5px;
              white-space: nowrap;
              overflow: hidden;
              text-overflow: ellipsis;
              user-select: none;
              -webkit-user-select: none;
              -moz-user-select: none;
              -ms-user-select: none;
              & span {
                display: block;
                width: calc(100% - 6px);
                white-space: nowrap;
                overflow: hidden;
                text-overflow: ellipsis;
                margin: 0 auto;
              }
            }
            &-subtitle { top: 40px; }
            &-datetime { top: 63px; }
            &-price {
              position: absolute;
              top: 86px;
              left: 0;
              right: 0;
              margin: 0 auto;
              width: calc(100% - 60px);
              height: 57px;
              line-height: 57px;
              text-align: center;
              font-size: 30px;
              font-weight: bold;
              color: #ffffff;
              border-radius: 5px;
              text-shadow: 1px 1px 5px rgba(0, 0, 0, 0.3);
              background-color: rgba(255, 255, 255, 0.45);
            }
          }
        }
      }
    }
    &-actions {
      border-radius: 23px;
      opacity: 0;
      pointer-events: none;
      transition: all 0.15s linear;
      &-left,
      &-right {
        position: absolute;
        top: 0;
        bottom: 0;
        width: 100%;
        display: flex;
        align-items: center;
        transform: scale(1);
      }
      &-left {
        left: 0;
        justify-content: flex-start;
      }
      &-right {
        right: 0;
        justify-content: flex-end;
      }
    }
  }
}
</style>
