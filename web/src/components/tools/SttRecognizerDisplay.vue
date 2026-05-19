<template>
  <Transition
    name="stt-recognizer"
    @after-enter="emit('enterDone')"
    @before-leave="emit('leaveStart')"
  >
    <div v-if="active" class="stt-recognizer-layer">
      <WaterRipple />
      <div ref="textAreaRef" class="stt-recognizer-text-area">
        <span
          v-for="(c, index) in chars"
          :key="index"
          :style="c.style"
          class="stt-recognizer-char"
          >{{ c.text }}</span
        >
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { EVENT_STT_UPDATE } from "src/consts/generated_constants";
import WaterRipple from "src/components/effects/WaterRipple.vue";

defineOptions({ inheritAttrs: false });

const props = defineProps<{
  active: boolean;
}>();
const emit = defineEmits<{
  (e: 'enterDone'): void
  (e: 'leaveStart'): void
}>();

const opacityThreshold = 0.65;
const FONT_SIZE = 18;

interface CharData {
  text: string;
  style: string;
}

const chars = ref<CharData[]>([]);
const textAreaRef = ref<HTMLDivElement | null>(null);
let unlistenUpdate: UnlistenFn | null = null;

// テキスト領域を最下部にスクロールし、文字スタイルを更新する
const updateView = () => {
  const el = textAreaRef.value;
  if (!el) return;

  // 1. 文字スタイルの更新 (行判定と透明度計算)
  const spans = el.querySelectorAll(".stt-recognizer-char");
  if (spans.length > 0) {
    const lineHeight = FONT_SIZE * 1.65;
    const containerHeight = el.clientHeight;
    const maxLines = Math.max(2, Math.floor(containerHeight / lineHeight));

    const tops = Array.from(spans).map((s) => (s as HTMLElement).offsetTop);
    const uniqueRowTops = tops
      .reduce((acc, t) => {
        if (!acc.some((exist) => Math.abs(exist - t) < 5)) acc.push(t);
        return acc;
      }, [] as number[])
      .sort((a, b) => b - a);

    let needsUpdate = false;
    const newChars = chars.value.map((c, i) => {
      const top = tops[i] ?? 0;
      const rowIndex = uniqueRowTops.findIndex(
        (rowTop) => Math.abs(rowTop - top) < 5,
      );
      const safeRowIndex = rowIndex === -1 ? uniqueRowTops.length : rowIndex;

      let opacity = 0.0;
      if (safeRowIndex === 0 || safeRowIndex === 1) {
        opacity = 1.0;
      } else if (safeRowIndex === 2) {
        opacity = opacityThreshold;
      } else {
        const progress = (safeRowIndex - 2) / Math.max(1, maxLines - 2);
        opacity = opacityThreshold * (1.0 - progress);
      }
      opacity = Math.max(0, Math.min(1.0, opacity));

      const newStyle = `opacity: ${opacity.toFixed(3)}`;
      if (c.style !== newStyle) {
        needsUpdate = true;
        return { ...c, style: newStyle };
      }
      return c;
    });

    if (needsUpdate) {
      chars.value = newChars;
    }
  }

  // 2. 最下部スクロール
  el.scrollTop = el.scrollHeight;
};

// 生テキストから文字配列への変換
const updateChars = (newText: string) => {
  chars.value = newText.split("").map((c) => ({ text: c, style: "" }));
  nextTick(updateView);
};

// リサイズ時にも再計算
const onResize = () => {
  updateView();
};

onMounted(async () => {
  unlistenUpdate = await listen<{ text: string }>(EVENT_STT_UPDATE, (event) => {
    if (!props.active) return;
    updateChars(event.payload.text);
  });
  window.addEventListener("resize", onResize);
});

onUnmounted(() => {
  if (unlistenUpdate) unlistenUpdate();
  window.removeEventListener("resize", onResize);
});

// 開始時と終了時の両方で念のため表示をクリアする
watch(
  () => props.active,
  () => {
    chars.value = [];
  },
);
</script>

<style scoped>
@font-face {
  font-family: "MPLUSRounded1c";
  src: url("../../fonts/MPLUSRounded1c-Regular.ttf") format("truetype");
}

.stt-recognizer-layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 5;
  display: flex;
  flex-direction: column;
  font-family: "MPLUSRounded1c", sans-serif;
  font-weight: bold;
  font-size: 18px;
  overflow: hidden;
  backdrop-filter: blur(15px);
  -webkit-backdrop-filter: blur(15px);
}

.stt-recognizer-text-area {
  flex: 1;
  padding: 24px;
  overflow-y: auto;
  word-break: break-all;
  line-height: 1.65;
  display: flex;
  flex-wrap: wrap;
  align-content: center;
  justify-content: flex-start;

  scrollbar-width: none;
  &::-webkit-scrollbar {
    display: none;
  }
}

.stt-recognizer-char {
  transition: opacity 0.2s ease;
  white-space: pre-wrap;
  color: var(--q-dark);
}

.stt-recognizer-enter-active {
  animation: recognizer-in 0.3s ease-out;
}
.stt-recognizer-leave-active {
  animation: recognizer-in 0.3s ease-in reverse;
}

@keyframes recognizer-in {
  0% {
    opacity: 0;
    border-radius: 50%;
    transform: scale(0);
  }
  100% {
    opacity: 1;
    border-radius: 50px;
    transform: scale(1);
  }
}
</style>
