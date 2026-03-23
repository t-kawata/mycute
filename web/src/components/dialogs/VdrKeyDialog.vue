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
            v-model="vdrKey"
            label="VDR-KEY"
            rounded
            dense
            class="q-mx-xl q-mb-sm"
            :color="!vdrKeyOutlined ? 'primary' : 'purple'"
            :outlined="vdrKeyOutlined"
            :standout="vdrKeyOutlined ? false : 'bg-purple text-white'"
            @focus="onFocusEmail"
            @blur="onBlurEmail"
            @keydown.enter="onClickSubmit"
          ><template v-slot:prepend><KeyHeartOutlineIcon class="btn-svg-dark" /></template></q-input>

          <q-btn @click="onClickSubmit" rounded no-caps unelevated color="purple" :label="t('page.login.registerVdrKey')" class="q-mx-xl" style="width: calc(100% - 96px); height: 40px;"><template v-slot:default><ArrowCircleRightIcon class="q-ml-sm btn-svg" /></template></q-btn>
        </div>
      </div>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useMainStore } from 'src/stores/main-store'
import BottomCurve01 from 'src/components/decorations/BottomCurve01.vue'
import BoundingLogo from 'src/components/decorations/BoundingLogo.vue'
import KeyHeartOutlineIcon from 'src/components/icons/KeyHeartOutlineIcon.vue'
import ArrowCircleRightIcon from 'src/components/icons/ArrowCircleRightIcon.vue'
import { decodeJwt } from "jose"
import { APP_NAME, APP_CAPTION, LOGO_IMG_SRC } from 'src/configs/settings'
import { sleep, t } from 'src/utils/some'
import * as ldb from 'src/utils/ldb'
import { getVdrToken, createBD, authWithBD, createUser, usrsAuth, createVdr100YearToken } from 'src/utils/rest'
import { showWarn } from 'src/utils/notify'

const router = useRouter()
const mainStore = useMainStore()

const vdrKey = ref('')
const vdrKeyOutlined = ref(false)

interface Props { modelValue?: boolean }

/* ----------------- v-model 作成 bgn ----------------- */
const props = withDefaults(defineProps<Props>(), { modelValue: false })
const emit = defineEmits<{ (e: 'update:modelValue', value: boolean): void }>()
const internalModel = computed({ get() { return props.modelValue }, set(val: boolean) { emit('update:modelValue', val) } })
/* ----------------- v-model 作成 end ----------------- */

const onFocusEmail = () => { vdrKeyOutlined.value = true }
const onBlurEmail = () => { vdrKeyOutlined.value = false }

const onClickSubmit = async () => {
  if (vdrKey.value === '') {
    showWarn(t('page.login.error.requiredVdrKey'))
    return
  }
  mainStore.setIsLoaderOn(true)
  await sleep(1000)
  const token = await getVdrToken(vdrKey.value)
  if (token === '') {
    mainStore.setIsLoaderOn(false)
    showWarn(t('page.login.error.failedToValidateVdrKey'))
    return
  }
  // Valid key, store it and close dialog
  ldb.set(ldb.KEYS.V, vdrKey.value)
  // Inject details into store
  mainStore.setVdrToken(token)
  const payload = decodeJwt(token)
  if (payload.apx_id && payload.usr_id) {
    mainStore.setApxID(Number(payload.apx_id))
    mainStore.setVdrID(Number(payload.usr_id))
  }
  await sleep(500)
  mainStore.setIsLoaderOn(false)
  internalModel.value = false
}

// 自動セットアップロジック
onMounted(() => {
    // コンポーネントがマウントされた時点で、もし VDR-KEY がローカルストレージになければ
    // 自動セットアップを開始する
    // 親コンポーネント (LoginPage) ですでにチェックしているが、
    // ここでも念の為チェックし、自動実行する
    if (!vdrKey.value && !ldb.get(ldb.KEYS.V)) {
        runAutoSetup()
    }
})

const runAutoSetup = async () => {
    console.log("Starting VDR-KEY Auto Setup...")
    mainStore.setIsLoaderOn(true)
    await sleep(1000) // 視覚的な遅延

    try {
        // 1. バックドアの作成
        console.log("Creating Backdoor...")
        // クライアント側でパスフレーズを生成する
        const bdPass = `bd_pass_${Date.now()}_${Math.random().toString(36).substring(7)}`
        const bdHash = await createBD(bdPass)
        if (!bdHash) throw new Error('Failed to create Backdoor')

        // 2. バックドアによる認証
        console.log("Authenticating with Backdoor...")
        const bdToken = await authWithBD(bdPass, 1) // 1時間有効
        if (!bdToken) throw new Error('Failed to auth with Backdoor')

        // 3. APXの作成
        console.log("Creating APX...")
        const timestamp = Date.now()
        // ランダムだが有効なフォーマットで生成
        const apxEmail = `apx${timestamp}@auto.local`
        const apxPass = `pass${timestamp}`
        
        // APX作成. typeは指定しない(バックエンド側で自動判定される前提)
        // bgn_at, end_at は必須なので指定する
        const bgnAt = "2024-01-01T00:00:00"
        const endAt = "2100-12-31T23:59:59"
        
        const apxId = await createUser(bdToken, {
            name: `AutoAPX_${timestamp}`,
            email: apxEmail,
            password: apxPass,
            bgn_at: bgnAt,
            end_at: endAt,
        })
        if (!apxId) throw new Error('Failed to create APX')

        // 4. APX認証 (APXトークン取得)
        console.log(`Authenticating APX (ID: ${apxId})...`)
        const apxAuthRes = await usrsAuth(0, 0, apxEmail, apxPass, 1)
        if (apxAuthRes.code !== 200) throw new Error('Failed to auth APX')
        
        let apxTokenObj = {}
        try { apxTokenObj = JSON.parse(apxAuthRes.body) } catch (e) { throw new Error('Failed to parse APX token') }
        const apxToken = (apxTokenObj as { token: string }).token
        if (!apxToken) throw new Error('Failed to get APX token')

        // 5. VDRの作成
        console.log("Creating VDR...")
        const vdrEmail = `vdr${timestamp}@auto.local`
        const vdrPass = `pass${timestamp}`
        
        // VDR用パラメータとしてデフォルト値を入れておく。
        const vdrId = await createUser(apxToken, {
             name: `AutoVDR_${timestamp}`,
             email: vdrEmail,
             password: vdrPass,
             bgn_at: bgnAt,
             end_at: endAt,
             // VDR属性
             base_point: 1000,
             belong_rate: 0.1,
             max_works: 100,
             flush_fee_rate: 0.05
        })
        if (!vdrId) throw new Error('Failed to create VDR')

        // 6. VDRの100年トークン作成
        console.log(`Creating 100 Year Token for VDR (ID: ${vdrId})...`)
        // キーは 5-50文字, regex=^[a-zA-Z0-9-_]{5,50}$
        const newVdrKey = `vdr_key_${timestamp}` 
        const vdr100YearToken = await createVdr100YearToken(apxToken, newVdrKey, apxId, vdrId)
        if (!vdr100YearToken) throw new Error('Failed to create VDR 100 Year Token')

        // 7. 完了
        console.log("Auto Setup Completed. Registering key...")
        vdrKey.value = newVdrKey
        await onClickSubmit()

    } catch (e) {
        console.error("Auto Setup Failed:", e)
        showWarn(t('page.login.error.autoSetupFailed'))
        mainStore.setIsLoaderOn(false)
    }
}
</script>
