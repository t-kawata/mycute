<template>
  <q-layout view="lHr lpR fFf" class="__harunohi-layout" style="min-height: none !important;">
    <q-page-container class="__harunohi-page">
      <router-view />
    </q-page-container>
  </q-layout>
</template>

<script setup lang="ts">
import { decodeJwt } from 'jose'
import { get, del, KEYS } from 'src/utils/ldb';
import { invoke } from '@tauri-apps/api/core';
import { useMainStore } from 'src/stores/main-store';
import { getVdrToken } from 'src/utils/rest';

defineOptions({
  async preFetch() {
    del(KEYS.T)
    const mainStore = useMainStore()
    mainStore.setToken('')
    try {
      await invoke('disable_hotkey_standby');
    } catch (e) {
      console.error("Failed to disable hotkey standby:", e);
    }

    const vdrKey = get<string>(KEYS.V)
    if (!vdrKey) throw new Error('No VDR key')
    const vdrToken = await getVdrToken(vdrKey);
    const vPayload = decodeJwt(vdrToken)
    mainStore.setVdrToken(vdrToken)
    mainStore.setApxID(Number(vPayload.apx_id))
    mainStore.setVdrID(Number(vPayload.usr_id))
  }
})
</script>
