<template>
  <div class="__harunohi-moving-star-container">
    <div
      v-if="show"
      :key="animationKey"
      class="__harunohi-moving-star-animation-stage"
      :class="{ '__harunohi-moving-star-animation-stage-tauri': IS_TAURI_DESKTOP }"
    >
      <div
        class="__harunohi-moving-star-left-wrapper"
        :class="leftClass"
        @animationend="onStarEnd('left')"
      >
        <div class="__harunohi-moving-star-left __harunohi-moving-star-star">
          <StarFatIcon class="__harunohi-icon-yellow" style="width: 50px"/>
        </div>
      </div>
      <div
        class="__harunohi-moving-star-right-wrapper"
        :class="rightClass"
        @animationend="onStarEnd('right')"
      >
        <div class="__harunohi-moving-star-right __harunohi-moving-star-star">
          <StarFatIcon class="__harunohi-icon-yellow" style="width: 50px"/>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { isTauriDesktop } from 'src/utils/some'
import StarFatIcon from 'src/components/icons/StarFatIcon.vue'

const IS_TAURI_DESKTOP = isTauriDesktop()

const props = defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void;
  (e: 'animation-done'): void;
}>();

const animationKey = ref(0);
const show = ref(false);
const doneSet = ref<{ left: boolean; right: boolean }>({ left: false, right: false });

const leftClass = computed(() => "__harunohi-moving-star-left-animated");
const rightClass = computed(() => "__harunohi-moving-star-right-animated");

watch(
  () => props.modelValue,
  async (nv, ov) => {
    if (nv) {
      animationKey.value += 1;
      doneSet.value = { left: false, right: false };
      show.value = false;
      await nextTick();
      show.value = true;
    } else {
      show.value = false;
    }
  },
  { immediate: true }
);

function onStarEnd(side: 'left' | 'right') {
  doneSet.value[side] = true;
  if (doneSet.value.left && doneSet.value.right) {
    emit('update:modelValue', false);
    emit('animation-done');
    show.value = false;
  }
}

// <transition :css="false"> を使わないため、これらのフックは不要
// function onBeforeEnter() {}
// function onAfterLeave() {}
</script>

<style scoped lang="scss">
.__harunohi-moving-star-container {
  position: relative;
  width: 100%;
  height: 100%;
  /* 動作確認用にコンテナサイズと背景色を指定（任意） */
  /* width: 200px; */
  /* height: 200px; */
  /* background-color: #eee; */
  overflow: hidden; /* 親要素の端に移動するため */
}

.__harunohi-moving-star-animation-stage {
  position: absolute;
  top: calc(50% - 25px); left: calc(50% - 25px);
  width: 0; height: 0;
  pointer-events: none;
  transform: translate(-50%, -50%);

  &-tauri {
    top: calc(50% - 25px + 40px);
  }
}

.__harunohi-moving-star-left-wrapper,
.__harunohi-moving-star-right-wrapper {
  position: absolute;
  top: 0; left: 0;
  width: 50px; height: 50px;
  /* 星の中心をアニメーションステージの中心に合わせる
    (transform-origin がデフォルトで center のため)
  */
}

.__harunohi-moving-star-star {
  width: 50px;
  height: 50px;
  opacity: 0;
  transform: scale(0.1);
}

/* --- アニメーション適用 --- */
.__harunohi-moving-star-left-animated .__harunohi-moving-star-star {
  animation: __harunohi-moving-star-left-fullmotion 0.8s linear forwards;
}

.__harunohi-moving-star-right-animated .__harunohi-moving-star-star {
  animation: __harunohi-moving-star-right-fullmotion 0.8s linear forwards;
}

/* ---
  左星 一連フルアニメーション
  --- */
@keyframes __harunohi-moving-star-left-fullmotion {
  0% {
    opacity: 0;
    transform: scale(0.1) translateX(0px) translateY(0px) rotate(0deg);
  }
  14.3% {
    opacity: 1;
    transform: scale(2) translateX(0px) translateY(0px) rotate(0deg);
  }
  28.6% {
    opacity: 1;
    transform: scale(2) translateX(-25px) translateY(0px) rotate(0deg);
  }
  35.7% { /* 90deg CW (下) */
    opacity: 1;
    transform: scale(2) translateX(0px) translateY(-25px) rotate(0deg);
  }
  42.8% { /* 180deg CW (右) */
    opacity: 1;
    transform: scale(2) translateX(25px) translateY(0px) rotate(0deg);
  }
  50.0% { /* 270deg CW (上) */
    opacity: 1;
    transform: scale(2) translateX(0px) translateY(25px) rotate(0deg);
  }
  /* 円運動 完了 (元の分離位置に戻る) */
  70.0% {
    opacity: 1;
    transform: scale(2) translateX(-25px) translateY(0px) rotate(0deg);
  }
  85.0% {
    opacity: 1;
    transform: scale(2) translateX(-130px) translateY(-250px) rotate(0deg);
  }
  100% {
    opacity: 0;
    transform: scale(2) translateX(-130px) translateY(-250px) rotate(0deg);
  }
}

/* ---
  右星 一連フルアニメーション (左星のミラー)
  --- */
@keyframes __harunohi-moving-star-right-fullmotion {
  0% {
    opacity: 0;
    transform: scale(0.1) translateX(0px) translateY(0px) rotate(0deg);
  }
  14.3% {
    opacity: 1;
    transform: scale(2) translateX(0px) translateY(0px) rotate(0deg);
  }
  28.6% {
    opacity: 1;
    transform: scale(2) translateX(25px) translateY(0px) rotate(0deg);
  }
  35.7% { /* 90deg CW (上) */
    opacity: 1;
    transform: scale(2) translateX(0px) translateY(25px) rotate(0deg);
  }
  42.8% { /* 180deg CW (左) */
    opacity: 1;
    transform: scale(2) translateX(-25px) translateY(0px) rotate(0deg);
  }
  50.0% { /* 270deg CW (下) */
    opacity: 1;
    transform: scale(2) translateX(0px) translateY(-25px) rotate(0deg);
  }
  /* 円運動 完了 */
  70.0% {
    opacity: 1;
    transform: scale(2) translateX(25px) translateY(0px) rotate(0deg);
  }
  /* 0.9s: 右下端に到達完了 */
  85.0% {
    opacity: 1;
    transform: scale(2) translateX(130px) translateY(250px) rotate(0deg);
  }
  /* 0.9s ~ 1.05s: 拡大・フェードアウト */
  100% {
    opacity: 0;
    transform: scale(2) translateX(130px) translateY(250px) rotate(0deg);
  }
}
</style>
