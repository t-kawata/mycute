<template>
  <div v-if="IS_TAURI_DESKTOP" class="__harunohi-handle-left"></div>
  <div v-if="IS_TAURI_DESKTOP" class="__harunohi-handle-right"></div>
  <div v-if="IS_TAURI_DESKTOP" class="__harunohi-handle-bottom-dec" :class="{ 'is-active': isBottomHovered }"></div>
  <div class="__harunohi-handle-bottom"></div>

  <q-layout view="lHr lpR fFf" class="__harunohi-layout" style="min-height: none !important;">
    <!-- LEFT Drawer -->
    <q-drawer v-model="leftDrawerOpen" :width="leftDrawerWidth" side="left" behavior="mobile"></q-drawer>

    <!-- RIGHT Drawer -->
    <q-drawer v-model="rightDrawerOpen" :width="rightDrawerWidth" side="right" behavior="mobile"></q-drawer>

    <q-page-container class="__harunohi-page">
      <!-- key 属性を URL にすることで、別アプリ（別URL）への切り替え時に確実にコンポーネントを再生成する -->
      <!-- これにより、前のサイトの残留データやスクロール位置をリセットし、クリーンな状態で表示できる -->
      <WebFrame v-if="activeWebUrl" :key="activeWebUrl" :url="activeWebUrl" />
      <router-view v-else />
    </q-page-container>
  </q-layout>

  <!-- Bottom Sheet -->
  <SpringBottomSheet ref="sheetRef" v-model="isBottomSheetOpen" @closed="onBottomSheetClosed">
    <div class="q-pt-sm __harunohi-os-desktop">
      <q-tab-panels v-model="desktopPanel" animated swipeable infinite class="bg-transparent">
        <q-tab-panel v-for="(page, pageIndex) in desktopPages" :key="pageIndex" :name="'p' + pageIndex" class="q-pa-none">
          <div class="row q-col-gutter-md">
            <!-- 4x4 Grid = 16 slots -->
            <div v-for="slotIndex in 16" :key="slotIndex" class="col-3 flex flex-center">
              <!-- Slot Index is 0-based for logic, so slotIndex - 1 -->
              <div v-if="page[slotIndex - 1]" class="__harunohi-os-app-item" @click="launchApp(page[slotIndex - 1])">
                <div class="__harunohi-os-app-icon flex flex-center">
                  <component :is="page[slotIndex - 1].app.icon" width="60" height="60" />
                </div>
                <div class="__harunohi-os-app-caption">{{ page[slotIndex - 1].app.name }}</div>
              </div>
              <div v-else class="__harunohi-os-app-slot-empty"></div>
            </div>
          </div>
        </q-tab-panel>
      </q-tab-panels>
      
      <!-- Pagination Dots -->
      <div class="row justify-center q-mt-md">
        <div 
          v-for="(_, pageIndex) in desktopPages" 
          :key="pageIndex"
          class="__harunohi-os-dot"
          :class="{ active: desktopPanel === 'p' + pageIndex }"></div>
      </div>
    </div>
  </SpringBottomSheet>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { URL } from 'src/router/routes'
import { useMainStore } from 'src/stores/main-store'
import { onMounted, ref, computed, onUnmounted, watch } from 'vue'
import { useQuasar } from 'quasar'
import { isTauriDesktop, sleep } from "src/utils/some"
import SpringBottomSheet from "@douxcode/vue-spring-bottom-sheet"
import "@douxcode/vue-spring-bottom-sheet/dist/style.css"
import { APP_TYPE } from 'src/models/app'
import { invoke } from '@tauri-apps/api/core'

// Icon Imports
import Microphone3Icon from 'src/components/icons/Microphone3Icon.vue'
import BotIcon from 'src/components/icons/BotIcon.vue'
import CreditCardMultipleIcon from 'src/components/icons/CreditCardMultipleIcon.vue'
import GearIcon from 'src/components/icons/GearIcon.vue'
import InstagramIcon from 'src/components/icons/InstagramIcon.vue'
import CalendarIcon from 'src/components/icons/CalendarIcon.vue'
import MedicineBottole1Icon from 'src/components/icons/MedicineBottole1Icon.vue'

// Page 2 Icons
import HomeIcon from 'src/components/icons/HomeIcon.vue'
import SearchIcon from 'src/components/icons/SearchIcon.vue'
import SendIcon from 'src/components/icons/SendIcon.vue'
import PenIcon from 'src/components/icons/PenIcon.vue'
import TiktokIcon from 'src/components/icons/TiktokIcon.vue'
import FacebookIcon from 'src/components/icons/FacebookIcon.vue'
import GoogleIcon from 'src/components/icons/GoogleIcon.vue'
import GroupIcon from 'src/components/icons/GroupIcon.vue'
import HeartIcon from 'src/components/icons/HeartIcon.vue'
import BadgeIcon from 'src/components/icons/BadgeIcon.vue'
import LetterBlocksIcon from 'src/components/icons/LetterBlocksIcon.vue'

// Page 3 Icons
import BackwardIcon from 'src/components/icons/BackwardIcon.vue'
import ForwardIcon from 'src/components/icons/ForwardIcon.vue'
import DoorIcon from 'src/components/icons/DoorIcon.vue'
import KeyHeartOutlineIcon from 'src/components/icons/KeyHeartOutlineIcon.vue'
import BadgesBoxIcon from 'src/components/icons/BadgesBoxIcon.vue'
import WebFrame from 'src/components/apps/WebFrame.vue'

const router = useRouter()
const mainStore = useMainStore()
const $q = useQuasar()
const IS_TAURI_DESKTOP = isTauriDesktop()

// ---------------------------------------------------------
// OS Desktop Logic
// ---------------------------------------------------------
const desktopPanel = ref('p0')
const sheetRef = ref<any>(null)

const pendingNavigationUrl = ref<string | null>(null)
const activeWebUrl = ref<string | null>(null)

const launchApp = (appConfig: any) => {
  mainStore.setIsLoaderOn(true)
  
  if (appConfig.app.type === APP_TYPE.MYCUTE) {
    pendingNavigationUrl.value = appConfig.app.url
  } else if (appConfig.app.type === APP_TYPE.WEB) {
    // 外部サイトの場合は内部フレームで開く
    activeWebUrl.value = appConfig.app.url
    pendingNavigationUrl.value = null
  }

  if (sheetRef.value) {
    sheetRef.value.close()
  } else {
    isBottomSheetOpen.value = false
  }
}

const onBottomSheetClosed = async () => {
  if (pendingNavigationUrl.value) {
    activeWebUrl.value = null // WebFrameを閉じる
    router.push(pendingNavigationUrl.value)
    pendingNavigationUrl.value = null
    await sleep(1000)
    mainStore.setIsLoaderOn(false)
  } else if (activeWebUrl.value) {
    await sleep(500)
    mainStore.setIsLoaderOn(false)
  }
}

// OS Desktop Logic
const desktopPages = computed(() => {
  if (!mainStore.apps.length) return []

  // 最大ページ数を特定
  const maxPage = Math.max(...mainStore.apps.map(a => a.page))
  const pages: any[][] = []

  for (let p = 0; p <= maxPage; p++) {
    const pageData = new Array(16).fill(null)
    const appsInPage = mainStore.apps.filter(a => a.page === p)
    appsInPage.forEach(appConfig => {
      if (appConfig.slot >= 0 && appConfig.slot < 16) {
        pageData[appConfig.slot] = appConfig
      }
    })
    pages.push(pageData)
  }
  return pages
})

/* 
// Temporarily commenting out multi-page dummy launcher
const desktopPagesLegacy = computed(() => {
  // Page 1: Core Apps (8 apps)
  const page1 = new Array(16).fill(null)
  page1[0] = { name: 'mycute', icon: Microphone3Icon }
  page1[1] = { name: 'buddy', icon: BotIcon }
  page1[2] = { name: 'harunohi', icon: CreditCardMultipleIcon }
  page1[3] = { name: 'Settings', icon: GearIcon }
  page1[4] = { name: 'Photos', icon: InstagramIcon }
  page1[6] = { name: 'Calendar', icon: CalendarIcon }
  page1[7] = { name: 'Health', icon: MedicineBottole1Icon }

  // Page 2: Social & Utilities (12 apps)
  const page2 = new Array(16).fill(null)
  page2[0] = { name: 'Home', icon: HomeIcon }
  page2[1] = { name: 'Search', icon: SearchIcon }
  page2[2] = { name: 'Messages', icon: SendIcon }
  page2[3] = { name: 'Notes', icon: PenIcon }
  page2[4] = { name: 'TikTok', icon: TiktokIcon }
  page2[5] = { name: 'Facebook', icon: FacebookIcon }
  page2[6] = { name: 'Google', icon: GoogleIcon }
  page2[7] = { name: 'Community', icon: GroupIcon }
  page2[8] = { name: 'Heart', icon: HeartIcon }
  page2[9] = { name: 'Badge', icon: BadgeIcon }
  page2[10] = { name: 'Dictionary', icon: LetterBlocksIcon }

  // Page 3: System & Others (5 apps)
  const page3 = new Array(16).fill(null)
  page3[0] = { name: 'Back', icon: BackwardIcon }
  page3[1] = { name: 'Next', icon: ForwardIcon }
  page3[2] = { name: 'Exit', icon: DoorIcon }
  page3[3] = { name: 'Key', icon: KeyHeartOutlineIcon }
  page3[4] = { name: 'Box', icon: BadgesBoxIcon }

  return [page1, page2, page3]
})
*/

// ---------------------------------------------------------
// Bottom Sheet Gesture Logic (Global Listener Implementation)
// ---------------------------------------------------------
let isTracking = false
let startY = 0
const isBottomHovered = ref(false)
const GESTURE_ZONE_HEIGHT = 36
const SWIPE_THRESHOLD = 20

// ボトムエリアにマウスがある時、bodyにクラスを付与してカーソルを制御する (Desktop Only for visual feedback)
watch(isBottomHovered, (val) => {
  if (!IS_TAURI_DESKTOP) return
  if (val) {
    document.body.classList.add('is-bottom-grabbing-ready')
  } else {
    document.body.classList.remove('is-bottom-grabbing-ready')
  }
})

const onGlobalPointerDown = (e: PointerEvent) => {
  // 画面最下部 15px 以内でのみジェスチャ開始
  if (e.clientY < window.innerHeight - GESTURE_ZONE_HEIGHT) return

  isTracking = true
  startY = e.clientY
  
  window.addEventListener('pointermove', onGlobalPointerMove)
  window.addEventListener('pointerup', onGlobalPointerUp)
  window.addEventListener('pointercancel', onGlobalPointerUp)
}

const onGlobalPointerMove = (e: PointerEvent) => {
  if (!isTracking) return

  const deltaY = startY - e.clientY
  if (deltaY > SWIPE_THRESHOLD) {
    mainStore.setIsBottomSheetOpen(true)
    stopTracking()
  }
}

const onGlobalPointerUp = () => {
  stopTracking()
}

const stopTracking = () => {
  isTracking = false
  window.removeEventListener('pointermove', onGlobalPointerMove)
  window.removeEventListener('pointerup', onGlobalPointerUp)
  window.removeEventListener('pointercancel', onGlobalPointerUp)
}

// Drawer visibility bound to global state
const leftDrawerOpen = computed({
  get: () => mainStore.leftDrawerOpen,
  set: (val) => mainStore.setLeftDrawerOpen(val)
})
const rightDrawerOpen = computed({
  get: () => mainStore.rightDrawerOpen,
  set: (val) => mainStore.setRightDrawerOpen(val)
})
const isBottomSheetOpen = computed({
  get: () => mainStore.isBottomSheetOpen,
  set: (val) => mainStore.setIsBottomSheetOpen(val)
})

// Responsive width logic
const isMobile = computed(() => $q.screen.lt.sm)
const windowWidth = ref(window.innerWidth)
const leftDrawerWidth = computed(() => !isMobile.value ? 350 : windowWidth.value - 30)
const rightDrawerWidth = computed(() => !isMobile.value ? 500 : windowWidth.value - 30)

defineOptions({
  inheritAttrs: false,
  preFetch({ redirect }) {
    const mainStore = useMainStore()
    if (!mainStore.token) {
      redirect(URL.LOGIN)
    }
  }
})

const onResize = () => {
  windowWidth.value = window.innerWidth
}

const onGlobalPointerMoveForHover = (e: PointerEvent) => {
  isBottomHovered.value = e.clientY >= window.innerHeight - GESTURE_ZONE_HEIGHT
}

onMounted(() => {
  if (!mainStore.token) {
    router.push(URL.LOGIN)
  }

  // アプリケーションの初期化（本来はAPIから取得するなどの処理がここに入る）
  mainStore.setApps([
    {
      page: 0,
      slot: 0,
      app: { id: 'harunohi', name: 'Harunohi', icon: CreditCardMultipleIcon, type: APP_TYPE.MYCUTE, url: URL.HOME }
    },
    {
      page: 0,
      slot: 1,
      app: { id: 'music', name: 'Music', icon: HeartIcon, type: APP_TYPE.MYCUTE, url: URL.MUSIC }
    },
    {
      page: 0,
      slot: 2,
      app: { id: 'settings', name: 'Settings', icon: GearIcon, type: APP_TYPE.MYCUTE, url: URL.SETTINGS }
    },
    {
      page: 0,
      slot: 3,
      app: { id: 'remosell', name: 'Remosell', icon: GroupIcon, type: APP_TYPE.WEB, url: 'https://agent-network.com/remosell/' }
    }
  ])

  window.addEventListener('resize', onResize)

  // ジェスチャ検知は全プラットフォームで capture: true で登録し、伝播が止まる前に補足する
  window.addEventListener('pointerdown', onGlobalPointerDown, true)

  if (IS_TAURI_DESKTOP) {
    // 装飾用のホバー検知のみデスクトップ限定
    window.addEventListener('pointermove', onGlobalPointerMoveForHover, { passive: true })
    
    // ホットキー監視の待機状態を開始 (二重起動はバックエンドで防止済み)
    invoke('enable_hotkey_standby').catch(e => console.error("Failed to enable hotkey standby:", e));
  }
})

onUnmounted(() => {
  window.removeEventListener('resize', onResize)
  window.removeEventListener('pointerdown', onGlobalPointerDown, true)
  
  if (IS_TAURI_DESKTOP) {
    window.removeEventListener('pointermove', onGlobalPointerMoveForHover)
  }
  stopTracking()
})
</script>
