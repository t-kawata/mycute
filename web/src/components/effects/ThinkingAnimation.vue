<template>
  <div class="thinking-animation" :class="sizeClass">
    <span
      v-for="n in dotCount"
      :key="n"
      class="thinking-dot"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  size?: 'sm' | 'md' | 'lg'
  dotCount?: number
}>(), {
  size: 'md',
  dotCount: 3,
})

const sizeClass = computed(() => `thinking-animation--${props.size}`)
</script>

<style lang="scss" scoped>
.thinking-animation {
  display: inline-flex;
  align-items: center;
  gap: 4px;

  .thinking-dot {
    border-radius: 50%;
    background: currentColor;
    animation: think-bounce 1.4s ease-in-out infinite;
  }

  @for $i from 1 through 5 {
    .thinking-dot:nth-child(#{$i}) {
      animation-delay: #{($i - 1) * 0.16}s;
    }
  }

  &--sm .thinking-dot {
    width: 5px;
    height: 5px;
  }

  &--md .thinking-dot {
    width: 8px;
    height: 8px;
  }

  &--lg .thinking-dot {
    width: 12px;
    height: 12px;
  }
}

@keyframes think-bounce {
  0%, 80%, 100% {
    transform: scale(0.6);
    opacity: 0.4;
  }
  40% {
    transform: scale(1);
    opacity: 1;
  }
}
</style>
