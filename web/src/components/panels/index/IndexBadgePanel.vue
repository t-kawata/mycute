<template>
  <div class="full-width full-height __harunohi-ruler">
    <BadgeBox :usrID="SAMPLE_USER_ID" :badges="mainStore.badges" :usrBadges="mainStore.usrBadges" @click-badge="onClickBadge" />
  </div>
</template>
<script setup lang="ts">
import { useMainStore } from 'src/stores/main-store'
import BadgeBox from 'src/components/decorations/BadgeBox.vue'
import { type Badge, type UsrBadge } from "src/models/main"
import { SAMPLE_USER_ID } from 'src/consts/data'

const mainStore = useMainStore()

const onClickBadge = (badge: Badge, usrBadges: UsrBadge[]) => {
  console.log({ badge, usrBadges })
}
</script>
