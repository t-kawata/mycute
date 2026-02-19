<template>
  <!-- オーバーレイ/スナックバー専用の最小レイアウト -->
  <!-- 背景は完全に透明で、装飾やナビゲーションは一切持たない -->
  <q-layout :class="className" view="lHr lpR fFf" style="min-height: none !important; background: transparent;">
    <q-page-container style="background: transparent;">
      <router-view />
    </q-page-container>
  </q-layout>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { WINDOW_LABEL_OVERLAY, WINDOW_LABEL_SNACKBAR } from 'src/consts/generated_constants';

const className = ref('')

onMounted(async () => {
    const label = await getCurrentWindow().label;
    switch (label) {
        case WINDOW_LABEL_OVERLAY: className.value = '__harunohi-overlay-layout'; break;
        case WINDOW_LABEL_SNACKBAR: className.value = '__harunohi-snackbar-layout'; break;
    }
});
</script>