<template>
  <q-dialog class="__harunohi-dialog" v-model="internalModel" persistent maximized transition-show="slide-right" transition-hide="slide-left">
    <q-card class="bg-white relative-position">
      <div class="__harunohi-login-page-topbox-02">
        <BoundingLogo :imageUrl="LOGO_IMG_SRC" color="#ffffff"/>
      </div>
      <BottomCurve01 color="#cbb0ff" />
      <div class="__harunohi-login-page-form-area relative-position">
        <span class="block bg-yellow-light __harunohi-dec-circle-right"></span>
        <span class="block bg-primary-light __harunohi-dec-circle-right-small"></span>
        <div class="absolute full-width" style="top: -20px;">
          <p class="text-h6 text-center q-mb-xs">{{ APP_NAME }}</p>
          <p class="text-caption text-center text-grey-6" style="position: relative; top: -5px;">{{ APP_CAPTION }}</p>
        </div>
        <div class="absolute-center full-width">
          <q-input
            v-model="email"
            label="Email"
            rounded
            dense
            class="q-mx-xl q-mb-sm"
            :color="!emailOutlined ? 'primary' : 'purple'"
            :outlined="emailOutlined"
            :standout="emailOutlined ? false : 'bg-purple text-white'"
            @focus="onFocusEmail"
            @blur="onBlurEmail"
          ><template v-slot:prepend><UserOutlineIcon class="btn-svg-dark" /></template></q-input>

          <q-input
            v-model="password"
            label="Password"
            type="password"
            rounded
            dense
            class="q-mx-xl q-mb-sm"
            :color="!passwordOutlined ? 'primary' : 'purple'"
            :outlined="passwordOutlined"
            :standout="passwordOutlined ? false : 'bg-purple text-white'"
            @focus="onFocusPassword"
            @blur="onBlurPassword"
          ><template v-slot:prepend><KeyHeartOutlineIcon class="btn-svg-dark" /></template></q-input>

          <q-btn @click="onClickSubmit" rounded no-caps unelevated color="purple" :label="t('page.login.login')" class="q-mx-xl" style="width: calc(100% - 96px); height: 40px;"><template v-slot:default><ArrowCircleRightIcon class="q-ml-sm btn-svg" /></template></q-btn>

          <div class="q-px-xl">
            <div class="row">
              <div class="text-center col">
                <q-btn round flat color="purple" @click="onClickSSOGoogle">
                  <template v-slot:default><GoogleIcon class="btn-svg-dark" /></template>
                </q-btn>
              </div>
              <div class="text-center col">
                <q-btn round flat color="purple" @click="onClickSSOInstagram">
                  <template v-slot:default><InstagramIcon class="btn-svg-dark" /></template>
                </q-btn>
              </div>
              <div class="text-center col">
                <q-btn round flat color="purple" @click="onClickSSOTikTok">
                  <template v-slot:default><TiktokIcon class="btn-svg-dark" /></template>
                </q-btn>
              </div>
              <div class="text-center col">
                <q-btn round flat color="purple" @click="onClickSSOFacebook">
                  <template v-slot:default><FacebookIcon class="btn-svg-dark" /></template>
                </q-btn>
              </div>
            </div>
          </div>
        </div>
      </div>
      <q-btn v-close-popup round flat color="purple" class="__harunohi-control-btn-back">
        <template v-slot:default><ArrowLeftIcon /></template>
      </q-btn>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useMainStore } from 'src/stores/main-store'
import BottomCurve01 from 'src/components/decorations/BottomCurve01.vue'
import BoundingLogo from 'src/components/decorations/BoundingLogo.vue'
import UserOutlineIcon from 'src/components/icons/UserOutlineIcon.vue'
import KeyHeartOutlineIcon from 'src/components/icons/KeyHeartOutlineIcon.vue'
import ArrowLeftIcon from 'src/components/icons/ArrowLeftIcon.vue'
import ArrowCircleRightIcon from 'src/components/icons/ArrowCircleRightIcon.vue'
import GoogleIcon from 'src/components/icons/GoogleIcon.vue'
import InstagramIcon from 'src/components/icons/InstagramIcon.vue'
import TiktokIcon from 'src/components/icons/TiktokIcon.vue'
import FacebookIcon from 'src/components/icons/FacebookIcon.vue'
import { APP_NAME, APP_CAPTION, LOGO_IMG_SRC, TOKEN_EXPIRE_HOURS } from 'src/configs/settings'
import { URL } from 'src/router/routes'
import { sleep, t } from 'src/utils/some'
import { usrsAuth } from 'src/utils/rest'
import { showWarn } from 'src/utils/notify'

const router = useRouter()
const mainStore = useMainStore()

const email = ref('')
const emailOutlined = ref(false)
const password = ref('')
const passwordOutlined = ref(false)

interface Props { modelValue?: boolean }

/* ----------------- v-model 作成 bgn ----------------- */
const props = withDefaults(defineProps<Props>(), { modelValue: false })
const emit = defineEmits<{ (e: 'update:modelValue', value: boolean): void }>()
const internalModel = computed({ get() { return props.modelValue }, set(val: boolean) { emit('update:modelValue', val) } })
/* ----------------- v-model 作成 end ----------------- */

const onFocusEmail = () => { emailOutlined.value = true }
const onBlurEmail = () => { emailOutlined.value = false }
const onFocusPassword = () => { passwordOutlined.value = true }
const onBlurPassword = () => { passwordOutlined.value = false }

const onClickSubmit = async () => {
  const { body, code, err } = await usrsAuth(mainStore.apxID, mainStore.vdrID, email.value, password.value, TOKEN_EXPIRE_HOURS)
  if (err !== '' || code !== 200 || body === '') {
    showWarn(t('page.login.error.failedToSignIn'))
    return
  }
  try {
    const bodyObj = JSON.parse(body) as { data: { token: string } }
    const { token } = bodyObj as unknown as { token: string }
    if (!token) { showWarn(t('page.login.error.failedToSignIn')); return }
    mainStore.setToken(token)
    mainStore.setIsLoaderOn(true)
    await sleep(1500)
    mainStore.setIsLoaderOn(false)
    router.push(URL.HOME)
  } catch (e) { showWarn(t('page.login.error.failedToSignIn')); return }
}

const onClickSSOGoogle = () => { showWarn(t('page.login.error.comingSoon')) }
const onClickSSOInstagram = () => { showWarn(t('page.login.error.comingSoon')) }
const onClickSSOTikTok = () => { showWarn(t('page.login.error.comingSoon')) }
const onClickSSOFacebook = () => { showWarn(t('page.login.error.comingSoon')) }
</script>
