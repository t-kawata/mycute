<template>
  <q-tab-panels v-model="mainStore.tab" animated class="__harunohi-tabpanels">
    <q-tab-panel class="__harunohi-tabpanel" :name="TAB.BADGE"><IndexBadgePanel /></q-tab-panel>
    <q-tab-panel class="__harunohi-tabpanel" :name="TAB.CALENDAR"><IndexCalendarPanel /></q-tab-panel>
    <q-tab-panel class="__harunohi-tabpanel" :name="TAB.SEARCH"><IndexSearchPanel /></q-tab-panel>
    <q-tab-panel class="__harunohi-tabpanel" :name="TAB.FRIENDS"><IndexFriendsPanel /></q-tab-panel>
    <q-tab-panel class="__harunohi-tabpanel" :name="TAB.SETTINGS"><IndexSettingsPanel /></q-tab-panel>
  </q-tab-panels>
  <div :class="!IS_TAURI_DESKTOP ? 'fixed-bottom' : ''">
    <div class="__harunohi-tabs">
      <div class="__harunohi-tabs-tab" @click="onClickTab(TAB.BADGE as TabType)">
        <BadgeIcon :class="{ active: mainStore.tab === TAB.BADGE }" />
      </div>
      <div class="__harunohi-tabs-tab" @click="onClickTab(TAB.CALENDAR as TabType)">
        <CalendarIcon :class="{ active: mainStore.tab === TAB.CALENDAR }" />
      </div>
      <div class="__harunohi-tabs-tab" @click="onClickTab(TAB.SEARCH as TabType)">
        <SearchIcon :class="{ active: mainStore.tab === TAB.SEARCH }" />
      </div>
      <div class="__harunohi-tabs-tab" @click="onClickTab(TAB.FRIENDS as TabType)">
        <GroupIcon :class="{ active: mainStore.tab === TAB.FRIENDS }" />
      </div>
      <div class="__harunohi-tabs-tab" @click="onClickTab(TAB.SETTINGS as TabType)">
        <GearIcon :class="{ active: mainStore.tab === TAB.SETTINGS }" />
      </div>
    </div>
  </div>
</template>
<script setup lang="ts">
import { useQuasar } from 'quasar'
import TAB, { type TabType } from 'src/enums/TAB'
import { useMainStore } from 'src/stores/main-store'
import { isTauriDesktop } from "src/utils/some"
import IndexBadgePanel from 'src/components/panels/index/IndexBadgePanel.vue'
import IndexCalendarPanel from 'src/components/panels/index/IndexCalendarPanel.vue'
import IndexSearchPanel from 'src/components/panels/index/IndexSearchPanel.vue'
import IndexFriendsPanel from 'src/components/panels/index/IndexFriendsPanel.vue'
import IndexSettingsPanel from 'src/components/panels/index/IndexSettingsPanel.vue'
import BadgeIcon from "src/components/icons/BadgeIcon.vue"
import CalendarIcon from "src/components/icons/CalendarIcon.vue"
import SearchIcon from "src/components/icons/SearchIcon.vue"
import GroupIcon from "src/components/icons/GroupIcon.vue"
import GearIcon from "src/components/icons/GearIcon.vue"

const $q = useQuasar()
const mainStore = useMainStore()
const IS_TAURI_DESKTOP = isTauriDesktop()

const onClickTab = (tabName: TabType) => {
  mainStore.setTab(tabName)
}
</script>
