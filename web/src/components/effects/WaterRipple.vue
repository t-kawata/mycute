<template>
  <!-- 波紋エフェクト用コンテナ -->
  <div ref="containerRef" class="water-ripple-container">
    <div class="ripple ripple-1" @animationiteration="onRippleIteration"></div>
    <div class="ripple ripple-2" @animationiteration="onRippleIteration"></div>
    <div class="ripple ripple-3" @animationiteration="onRippleIteration"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'

const containerRef = ref<HTMLElement | null>(null)

// ============================================================
// 波紋エフェクトの制御
// ============================================================
/** 波紋の位置をランダムに更新する */
const randomizeElementPosition = (el: HTMLElement) => {
  el.style.top = `${Math.random() * 80 + 10}%` // 端に寄りすぎないよう10-90%の範囲
  el.style.left = `${Math.random() * 80 + 10}%`
}

/** アニメーションの1サイクル終了ごとに呼ばれる */
const onRippleIteration = (e: AnimationEvent) => {
  const el = e.target as HTMLElement
  if (el) randomizeElementPosition(el)
}

/** 初回の位置をセット */
const initRipplePositions = () => {
  if (!containerRef.value) return
  const ripples = containerRef.value.querySelectorAll('.ripple')
  ripples.forEach(el => randomizeElementPosition(el as HTMLElement))
}

onMounted(() => {
  initRipplePositions()
})
</script>

<style scoped>
/* ===== 水の波紋エフェクト ===== */
.water-ripple-container {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  border-radius: inherit;
  overflow: hidden;
  pointer-events: none;
  z-index: 0;
}

.ripple {
  position: absolute;
  border-radius: 50%;
  transform: translate(-50%, -50%) scale(0);
  opacity: 0;
  /* 立体感を出すためのハイライトとシャドウ（強すぎないように調整） */
  box-shadow: 
    inset 0 0 10px rgba(255, 255, 255, 0.4),
    inset 0 0 4px rgba(255, 255, 255, 0.6),
    0 4px 10px rgba(0, 0, 0, 0.05);
}

.ripple-1 {
  width: 120px; height: 120px;
  animation: drop-ripple 8s infinite cubic-bezier(0.1, 0.8, 0.3, 1);
}

.ripple-2 {
  width: 150px; height: 150px;
  animation: drop-ripple 11s infinite cubic-bezier(0.1, 0.8, 0.3, 1) 3s;
}

.ripple-3 {
  width: 130px; height: 130px;
  animation: drop-ripple 9s infinite cubic-bezier(0.1, 0.8, 0.3, 1) 6s;
}

@keyframes drop-ripple {
  0% {
    transform: translate(-50%, -50%) scale(0.1);
    opacity: 0;
  }
  5% {
    opacity: 0.7;
  }
  100% {
    transform: translate(-50%, -50%) scale(3.5);
    opacity: 0;
  }
}
</style>
