<template>
  <q-layout view="lHr lpR fFf" class="__harunohi-layout" style="min-height: none !important;">
    <q-page-container class="__harunohi-page">
      <router-view />
    </q-page-container>
  </q-layout>
</template>

<script setup lang="ts">
import { del, KEYS } from 'src/utils/ldb';
import { invoke } from '@tauri-apps/api/core';
import { useMainStore } from 'src/stores/main-store';

defineOptions({
  async preFetch() {
    del(KEYS.T)
    const store = useMainStore()
    store.setToken('')
    try {
      await invoke('disable_hotkey_standby');
    } catch (e) {
      console.error("Failed to disable hotkey standby:", e);
    }
  }
})
</script>
