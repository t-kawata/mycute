<template>
<div v-if="!!props.color" style="position: relative; margin-bottom: 55px;">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" preserveAspectRatio="none" class="__harunohi-bottom-curve-01-svg">
    <defs>
      <linearGradient :id="`fadeUp-${id}`" x1="0" y1="1" x2="0" y2="0" gradientUnits="objectBoundingBox">
        <stop offset="0%" :stop-color="props.color" stop-opacity="1" />
        <stop offset="30%" :stop-color="props.color" stop-opacity="1" />
        <stop offset="100%" :stop-color="props.color" stop-opacity="0" />
      </linearGradient>
    </defs>
    <path
      d="
        M0,0
        H100
        V90
        C80,100 65,65 50,65
        C35,65 20,100 0,90
        Z
      "
      :fill="`url(#fadeUp-${id})`"
    />
  </svg>
</div>
</template>

<script setup lang="ts">
import { randStr } from 'src/utils/common'

interface Props {
  color?: string
}

const props = defineProps<Props>()

const id = randStr(10)
</script>

<style lang="scss" scoped>
/* はみ出る曲線の高さが 55px になる */
.__harunohi-bottom-curve-01-svg {
  width: 100%;
  height: 150px;
  position: absolute;
  top: -95px;
}
</style>
