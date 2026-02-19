<template>
  <div
    class="__harunohi-badge-counter"
    :style="{ width: width + 'px', height: height + 'px' }"
  >
    <div
      class="__harunohi-badge-counter-segments"
      :class="{ '__harunohi-badge-counter-segments--active': active }"
      :style="{ gap: '1px' }"
    >
      <div
        v-for="(lineCount, lineIndex) in segmentLines"
        :key="lineIndex"
        class="__harunohi-badge-counter-line"
        :style="lineStyle"
      >
        <div
          v-for="segIndex in lineCount"
          :key="`${lineIndex}-${segIndex}`"
          class="__harunohi-badge-counter-segment"
          :style="segmentStyle"
        />
      </div>
    </div>
    <div class="__harunohi-badge-counter-text">
      {{ total }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  total: number
  width: number
  height: number
  active: boolean
  itemWidth: number
  lines: number
}

const props = defineProps<Props>()

const maxSegmentsPerLine = computed(() => {
  return Math.floor(props.width / (props.itemWidth))
})

const segmentLines = computed(() => {
  const maxPerLine = maxSegmentsPerLine.value
  const lines: number[] = []
  let remaining = props.total

  for (let i = 0; i < props.lines; i++) {
    const count = Math.min(maxPerLine, Math.max(0, remaining))
    lines.push(count)
    remaining -= count
  }

  return lines
})

const lineHeight = computed(() => {
  return (props.height - (props.lines - 1)) / props.lines
})

const lineStyle = computed(() => {
  return {
    display: 'flex',
    height: `${lineHeight.value}px`,
    width: '100%'
  }
})

const segmentStyle = computed(() => {
  return {
    width: `${props.itemWidth}px`,
    height: `${lineHeight.value}px`
  }
})
</script>

<style scoped lang="scss">
.__harunohi-badge-counter {
  position: relative;
  background-color: $grey-2;
  /* border-radius: 4px; */
}

.__harunohi-badge-counter-segments {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;

  &--active {
    .__harunohi-badge-counter-segment {
      animation: __harunohi-badge-counter-wave 0.3s ease-in-out infinite;
      @for $i from 1 through 150 {
        &:nth-child(#{$i}) {
          animation-delay: #{$i * 0.05}s;
        }
      }
    }
  }
}

.__harunohi-badge-counter-line {
  display: flex;
  flex: 0 0 auto;
}

.__harunohi-badge-counter-segment {
  background: $secondary-light;
  border-right: 1px solid white;
  flex: 0 0 auto;
}

.__harunohi-badge-counter-text {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-weight: bold;
  font-size: 16px;
  color: $grey-7;
  text-shadow: 0 0 4px rgba(255, 255, 255, 0.8);
  pointer-events: none;
  z-index: 10;
}

@keyframes __harunohi-badge-counter-wave {
  0%, 100% {
    transform: scaleY(1);
  }
  50% {
    transform: scaleY(1.5);
  }
}
</style>
