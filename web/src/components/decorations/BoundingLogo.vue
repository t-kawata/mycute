<template>
  <div class="full-width full-height animation-container">
  <div
    class="ball"
    :style="{
      left: `${ballState.x}px`,
      top: `${ballState.y}px`,
      width: `${ballState.size}px`,
      height: `${ballState.size}px`,
      backgroundColor: hexToRgba(props.color, ballState.colorOpacity),
      opacity: ballState.opacity
    }"
  >
    <img
      v-if="ballState.showImage"
      :src="imageUrl"
      :style="{
        opacity: ballState.imageOpacity,
        width: '100%',
      }"
    />
  </div>
</div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted } from 'vue'

interface Props {
  imageUrl: string;
  color?: string;
}

interface BallState {
  x: number;
  y: number;
  size: number;
  opacity: number;
  colorOpacity: number;
  imageOpacity: number;
  showImage: boolean;
  velocity?: number;
  targetY?: number;
  isPhysicsActive?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  color: '#FF69B4'
})

const ballState = reactive<BallState>({
  x: 0,
  y: 0,
  size: 30,
  opacity: 1,
  colorOpacity: 1,
  imageOpacity: 0,
  showImage: false
})

let animationId: number | null = null
let startTime: number | null = null
let containerWidth = 0
let containerHeight = 0

// 16進カラーコードをRGBAに変換する関数
const hexToRgba = (hex: string, alpha: number): string => {
  hex = hex.replace('#', '');
  const r = parseInt(hex.length === 3 ? hex.slice(0, 1).repeat(2) : hex.slice(0, 2), 16);
  const g = parseInt(hex.length === 3 ? hex.slice(1, 2).repeat(2) : hex.slice(2, 4), 16);
  const b = parseInt(hex.length === 3 ? hex.slice(2, 3).repeat(2) : hex.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

const easeInOutCubic = (t: number): number => {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

const animate = (currentTime: number) => {
  if (!startTime) startTime = currentTime;
  const elapsed = currentTime - startTime;
  const sizeRatio = 1.2

  // ステージ1: 物理演算による落下とバウンド (0-3000ms)
  if (elapsed < 3000) {
    // 初回のみ物理変数を初期化
    if (!ballState.velocity) {
      ballState.velocity = 0;
      ballState.targetY = containerHeight * 0.75;
      ballState.isPhysicsActive = true;
    }

    if (ballState.isPhysicsActive) {
      // 重力加速度（ピクセル/フレーム^2）
      const gravity = 0.45;
      // 減衰係数（バウンド時のエネルギー損失）
      const damping = 0.65;
      // 停止判定の閾値
      const stopThreshold = 0.5;

      // 速度に重力を加算
      ballState.velocity += gravity;
      // 位置を更新
      ballState.y += ballState.velocity;

      // 地面（targetY）に到達したらバウンド
      if (ballState.y >= Number(ballState.targetY)) {
        ballState.y = Number(ballState.targetY);
        ballState.velocity = -ballState.velocity * damping;

        // 速度が十分小さくなったら停止
        if (Math.abs(ballState.velocity) < stopThreshold) {
          ballState.velocity = 0;
          ballState.isPhysicsActive = false;
        }
      }
    }

    ballState.x = containerWidth / 2 - ballState.size / 2;
  }
  // ステージ2: 中央へ移動 (3000-3700ms)
  else if (elapsed < 3700) {
    const moveProgress = (elapsed - 3000) / 700;
    const easedProgress = easeInOutCubic(moveProgress);
    ballState.showImage = true;
    ballState.colorOpacity = 1 - easedProgress;
    ballState.imageOpacity = easedProgress;
    ballState.y = Number(ballState.targetY) + easedProgress * (containerHeight / 2 - Number(ballState.targetY) - ballState.size / 2);
    ballState.x = containerWidth / 2 - ballState.size / 2;
  }
  // ステージ3: 拡大 & 画像フェードイン (3700-5200ms)
  else if (elapsed < 5200) {
    const expandProgress = (elapsed - 3700) / 1500;
    const easedProgress = easeInOutCubic(expandProgress);

    ballState.size = 30 + easedProgress * (containerWidth * sizeRatio - 30);
    ballState.x = containerWidth / 2 - ballState.size / 2;
    ballState.y = containerHeight / 2 - ballState.size / 2;
    ballState.opacity = 1 - easedProgress;

    // 画像フェードイン
    // ballState.imageOpacity = easedProgress;
  }
  // アニメーション終了
  else {
    if (animationId) {
      cancelAnimationFrame(animationId);
      animationId = null;
    }
    return;
  }

  animationId = requestAnimationFrame(animate);
};

onMounted(() => {
  const container = document.querySelector('.animation-container') as HTMLElement;
  if (container) {
    containerWidth = container.offsetWidth;
    containerHeight = container.offsetHeight;

    // アニメーション開始
    animationId = requestAnimationFrame(animate);
  }
});

onUnmounted(() => {
  if (animationId) {
    cancelAnimationFrame(animationId);
  }
});
</script>

<style scoped lang="scss">
.animation-container {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;

  .ball {
    position: absolute;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: none;
    will-change: transform, opacity;

    img {
      object-fit: cover;
      pointer-events: none;
    }
  }
}
</style>
