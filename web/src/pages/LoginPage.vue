<template>
  <div id="__harunohi-login-page-topbox-01" class="__harunohi-login-page-topbox-01 relative-position">
    <BoundingLogo :imageUrl="LOGO_IMG_SRC" color="#ffffff"/>
  </div>
  <BottomCurve01 color="#C7EFEF" />
  <div class="__harunohi-login-page-form-area relative-position">
    <span class="block bg-yellow-light __harunohi-dec-circle-right"></span>
    <span class="block bg-primary-light __harunohi-dec-circle-right-small"></span>
    <div class="absolute full-width" style="top: -20px;">
      <p class="text-h6 text-center q-mb-xs">{{ APP_NAME }}</p>
      <p class="text-caption text-center text-grey-6" style="position: relative; top: -5px;">{{ APP_CAPTION }}</p>
    </div>
    <div class="absolute-center full-width">
      <q-btn rounded no-caps unelevated color="purple" :label="t('page.login.signin')" class="q-mx-xl q-mb-sm" style="width: calc(100% - 96px); height: 40px;" @click="isSignInOpen = true"><template v-slot:default><ArrowCircleRightIcon class="q-ml-sm btn-svg" /></template></q-btn>
      <q-btn rounded no-caps unelevated color="secondary" :label="t('page.login.signup')" class="q-mx-xl q-mb-sm" style="width: calc(100% - 96px); height: 40px;" @click="isSignUpOpen = true"><template v-slot:default><PenIcon class="q-ml-sm btn-svg" /></template></q-btn>
      <q-btn rounded outline no-caps unelevated color="negative" :label="t('page.login.reset')" class="q-mx-xl" style="width: calc(100% - 96px); height: 40px;" @click="mainStore.setIsResetConfirmOpen(true)"><template v-slot:default><q-icon name="restore" class="q-ml-sm" /></template></q-btn>
    </div>
  </div>
  <SignInDialog v-model="isSignInOpen" />
  <SignUpDialog v-model="isSignUpOpen" />
  <VdrKeyDialog v-model="isVdrKeyOpen" />
</template>
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { t } from 'src/utils/some'
import * as ldb from 'src/utils/ldb'
import BottomCurve01 from 'src/components/decorations/BottomCurve01.vue'
import BoundingLogo from 'src/components/decorations/BoundingLogo.vue'
import SignInDialog from 'src/components/dialogs/SignInDialog.vue'
import SignUpDialog from 'src/components/dialogs/SignUpDialog.vue'
import VdrKeyDialog from 'src/components/dialogs/VdrKeyDialog.vue'
import ArrowCircleRightIcon from 'src/components/icons/ArrowCircleRightIcon.vue'
import PenIcon from 'src/components/icons/PenIcon.vue'
import { useMainStore } from "src/stores/main-store"
import { APP_NAME, APP_CAPTION, LOGO_IMG_SRC } from 'src/configs/settings'

const mainStore = useMainStore()

const isSignInOpen = ref(false)
const isSignUpOpen = ref(false)
const isVdrKeyOpen = ref(false)

onMounted(() => {
  const vdrKey = ldb.get<string>(ldb.KEYS.V)
  if (!vdrKey) {
    isVdrKeyOpen.value = true
  }
})
</script>
