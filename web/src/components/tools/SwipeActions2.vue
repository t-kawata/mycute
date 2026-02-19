<template>
  <div class="__harunohi-swipe2-list-container">
    <div
      v-for="(candidate, index) in props.candidates"
      :key="candidate.id"
      :class="{ '__harunohi-swipe2-item-wrapper': true, 'collapsing': isCollapsing(candidate.id) }"
      :style="getWrapperStyle(candidate.id)"
    >
      <div
        class="__harunohi-swipe2-actions __harunohi-swipe2-actions-left bg-grey-5"
        :style="{ opacity: swipeStates[candidate.id]?.leftOpacity || 0 }"
      >
        <slot
          name="left"
          :candidate="candidate"
          :swipeDistance="getSwipeDistance(candidate.id, 'nope')"
        />
      </div>
      <div
        class="__harunohi-swipe2-actions __harunohi-swipe2-actions-right bg-primary"
        :style="{ opacity: swipeStates[candidate.id]?.rightOpacity || 0 }"
      >
        <slot
          name="right"
          :candidate="candidate"
          :swipeDistance="getSwipeDistance(candidate.id, 'like')"
        />
      </div>

      <div
        class="__harunohi-swipe2-item-content"
        :style="getItemStyle(candidate.id)"
        @mousedown="onDragStart($event, candidate)"
        @touchstart="onDragStart($event, candidate)"
      >
        <div class="__harunohi-swipe2-item-content-item __harunohi-swipe2-item-content-item-image">
          <div class="__harunohi-swipe2-item-content-item-image-handle __harunohi-swipe2-item-content-item-image-handle-left"><AngleDoubleLeftIcon class="__harunohi-icon-white" /></div>
          <img :src="`${PUBLIC_PATH}/sample-face/0${candidate.to}-small.png`" />
          <div class="__harunohi-swipe2-item-content-item-image-handle __harunohi-swipe2-item-content-item-image-handle-right"><AngleDoubleRightIcon class="__harunohi-icon-white" /></div>
          <div class="__harunohi-swipe2-item-content-item-image-title">{{ candidate.toUsrName || `User ${candidate.id}` }}</div>
          <div class="__harunohi-swipe2-item-content-item-image-maintitle">{{ candidate.title || `Item ${candidate.id}` }}</div>
          <div class="__harunohi-swipe2-item-content-item-image-subtitle"><span>{{ candidate.subtitle }}</span></div>
          <div class="__harunohi-swipe2-item-content-item-image-datetime">{{ formatDateRange(candidate.start, candidate.end, false) }}</div>
          <div class="__harunohi-swipe2-item-content-item-image-badgename">認定バッジ: {{ candidate.badgeName }}</div>
          <div class="__harunohi-swipe2-item-content-item-image-price">
            <textarea
              class="__harunohi-swipe2-item-content-item-image-price-textarea"
              :rows="macLines" :placeholder="`${candidate.toUsrName}さんのこのお仕事について、スキル認定バッジと一緒に暖かいメッセージを送りましょう。`"
            >{{ candidate.message }}</textarea>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, CSSProperties } from 'vue';
import { type BadgeCandidate, type SwipeDirection } from 'src/models/main'
import { formatDateRange } from 'src/utils/some'
import AngleDoubleLeftIcon from 'src/components/icons/AngleDoubleLeftIcon.vue'
import AngleDoubleRightIcon from 'src/components/icons/AngleDoubleRightIcon.vue'
import { PUBLIC_PATH } from 'src/configs/settings'

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

// --- Consts ---
const macLines = 4

// --- Props ---

const props = withDefaults(
  defineProps<{
    candidates: BadgeCandidate[];
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
  (e: 'swipe-out', payload: { candidate: BadgeCandidate; direction: SwipeDirection }): void;
}>();

// --- State ---

const swipeStates = reactive<Record<string | number, SwipeState>>({});
const activeCandidateId = ref<string | number | null>(null);

// --- Watchers ---

watch(
  () => props.candidates,
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

// --- Computed & Helpers ---

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
    zIndex: activeCandidateId.value === id || state.isAnimating ? 10 : 1,
    pointerEvents: state.isAnimating ? 'none' : 'auto',
  };
};

const getSwipeDistance = (
  id: string | number,
  direction: SwipeDirection
) => {
  const state = swipeStates[id];
  if (!state) return 0;

  // 左方向スワイプ時（translateX < 0）
  if (direction === 'nope' && state.translateX < 0) {
    return Math.abs(state.translateX);
  }
  // 右方向スワイプ時（translateX > 0）
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

const onDragStart = (event: MouseEvent | TouchEvent, item: BadgeCandidate) => {
  if (activeCandidateId.value) return;

  const state = swipeStates[item.id];
  if (!state || state.isAnimating || state.isCollapsing) return;

  // 入力フォーム等でないなら標準動作を抑制（画像ドラッグ防止など）
  const target = event.target as HTMLElement;
  if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
    event.preventDefault();
  }

  activeCandidateId.value = item.id;
  state.isDragging = true;
  state.startX = getClientX(event);
  state.currentX = state.startX;

  window.addEventListener('mousemove', onDragMove);
  window.addEventListener('touchmove', onDragMove, { passive: false });
  window.addEventListener('mouseup', onDragEnd);
  window.addEventListener('touchend', onDragEnd);
};

const onDragMove = (event: MouseEvent | TouchEvent) => {
  if (!activeCandidateId.value) return;

  const state = swipeStates[activeCandidateId.value];
  if (!state || !state.isDragging) return;

  const clientX = getClientX(event);
  const deltaX = clientX - state.currentX;
  const totalDeltaX = clientX - state.startX;

  if (Math.abs(totalDeltaX) > 10) {
    event.preventDefault();
  }

  state.currentX = clientX;
  state.translateX = totalDeltaX;

  // ↓opacity制御を追加
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
  if (!activeCandidateId.value) return;

  const candidateId = activeCandidateId.value;
  const state = swipeStates[candidateId];
  const candidate = props.candidates.find((i) => i.id === candidateId);

  if (!state || !candidate) return;

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

  // opacity初期化
  state.rightOpacity = 0;
  state.leftOpacity = 0;

  setTimeout(() => {
    state.isAnimating = false;
    activeCandidateId.value = null;

    if (direction) {
      setTimeout(() => {
        state.isCollapsing = true;
        setTimeout(() => {
          emit('swipe-out', { candidate: candidate, direction });
        }, props.collapseDuration);
      }, props.collapseDelay);
    }
  }, 10);
};
</script>

<style scoped lang="scss">
.__harunohi {
  &-swipe2 {
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
            height: 246px;
            border-radius: 23px;
            border: 2px solid #ffffff;
            box-shadow: 0px 1px 5px 0px rgba(0, 0, 0, 0.15);
            &-handle {
              width: 28px;
              height: 100%;
              display: flex;
              justify-content: center;
              align-items: center;
              &-left {
                background-color: $grey-4;
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
              filter: blur(0.8px) brightness(1.1);
              opacity: 0.5;
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
              font-size: clamp(20px, 4vw, 24px);
              letter-spacing: 3px;
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
            &-maintitle,
            &-subtitle,
            &-datetime,
            &-badgename {
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
            &-maintitle { top: 40px; }
            &-subtitle { top: 63px; }
            &-datetime { top: 86px; }
            &-badgename { top: 109px; }
            &-price {
              position: absolute;
              top: 132px;
              left: 0;
              right: 0;
              margin: 0 auto;
              width: calc(100% - 60px);
              height: 107px;
              font-size: 12px;
              color: #ffffff;
              border-radius: 5px;
              background-color: rgba(255, 255, 255, 0.45);
              &-textarea {
                width: 100%;
                height: 100%;
                border: none;
                padding: 10px;
                border-radius: 5px;
                background-color: rgba(255, 255, 255, 0.75);
                color: $grey-8;
                vertical-align: top;
                resize: none;
                box-sizing: border-box;
                line-height: 1.44;
                user-select: text;
                -webkit-user-select: text;
                -moz-user-select: text;
                -ms-user-select: text;
                -webkit-touch-callout: default;
              }
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
