<template>
  <div class="full-width full-height q-pb-xl">
    <div class="q-ma-xl gallery">
      <div class="gallery__item gallery__item-with-camera">
        <img :src="`${PUBLIC_PATH}/capture/merii/01.png`">
      </div>
      <div class="gallery__item gallery__item-with-camera">
        <img :src="`${PUBLIC_PATH}/capture/merii/02.png`">
      </div>
      <div class="gallery__item gallery__item-with-camera">
        <img :src="`${PUBLIC_PATH}/capture/merii/03.png`">
      </div>
      <div class="gallery__item gallery__item-with-camera">
        <img :src="`${PUBLIC_PATH}/capture/merii/04.png`">
      </div>
      <div class="gallery__item">
        <img :src="`${PUBLIC_PATH}/capture/05.png`">
      </div>
      <div class="gallery__item">
        <img :src="`${PUBLIC_PATH}/capture/06.png`">
      </div>
      <div class="gallery__item gallery__item-with-camera">
        <img :src="`${PUBLIC_PATH}/capture/07.png`">
      </div>
      <div class="gallery__item">
        <img :src="`${PUBLIC_PATH}/capture/08.png`">
      </div>
      <div class="gallery__item">
        <img :src="`${PUBLIC_PATH}/capture/09.png`">
      </div>
      <div class="gallery__item">
        <img :src="`${PUBLIC_PATH}/capture/10.png`">
      </div>
      <div class="gallery__item">
        <img :src="`${PUBLIC_PATH}/capture/11.png`">
      </div>
      <div class="gallery__item">
        <img :src="`${PUBLIC_PATH}/capture/12.png`">
      </div>
      <div class="gallery__item">
        <img :src="`${PUBLIC_PATH}/capture/13.png`">
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { PUBLIC_PATH } from 'src/configs/settings'
</script>

<style scoped lang="scss">
$gallery-gap: 40px;
$gallery-min-width: 200px;
$frame-inset: 20px;
$base-width: 652px;
$base-height: 1366px;
$div: 3;
$radius: 120px;

.gallery {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax($gallery-min-width, 1fr));
  gap: $gallery-gap;

  &__item {
    position: relative;
    aspect-ratio: calc($base-width / $base-height);
    width: calc($base-width / $div);
    height: calc($base-height / $div);
    border-radius: calc($radius / $div);
    box-shadow: 0px 5px 15px 0px rgba(0, 0, 0, 0.1);
    img {
      position: relative;
      top: calc($frame-inset / $div);
      left: calc($frame-inset / $div);
      width: calc($base-width / $div - ($frame-inset / $div) * 2);
      height: calc($base-height / $div - ($frame-inset / $div) * 2);
      border-radius: calc($radius / $div);
      display: block;
      padding: 0;
      box-sizing: border-box;
    }

    &-with-camera {
      &::before {
        content: '';
        position: absolute;
        width: calc(($base-width / $div) * 0.38);
        height: calc(($base-width / $div) * 0.12);
        background-color: #fff;
        top: 0;
        left: 0;
        right: 0;
        margin: 0 auto;
        border-bottom-left-radius: calc(($base-width / $div) * 0.05);
        border-bottom-right-radius: calc(($base-width / $div) * 0.05);
        z-index: 100;
      }
    }

    &::after {
      content: "";
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      background-image: url('/${PUBLIC_PATH}/smart-phone01.png');
      background-size: 100% 100%;
      background-repeat: no-repeat;
      pointer-events: none;
      opacity: 0.3;
    }
  }
}
</style>
