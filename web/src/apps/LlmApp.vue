<template>
  <div class="__mycute-glass-app-container q-pa-sm">
    <div class="__mycute-glass-panel-inner">
      <WaterRipple />

      <!-- ローディング -->
      <div v-if="llmStore.isFetchingProviders" class="flex flex-center q-pa-xl">
        <q-spinner-dots size="40px" color="primary" />
      </div>

      <!-- エラー表示 -->
      <q-banner v-if="llmStore.lastError && !llmStore.isFetchingProviders" dense class="bg-negative text-white q-mb-md" rounded>
        <template #avatar><q-icon name="error" /></template>
        {{ llmStore.lastError }}
      </q-banner>

      <template v-if="!llmStore.isFetchingProviders">
        <!-- プロバイダー選択ステッパー -->
        <q-stepper
          id="llm-provider-stepper"
          v-model="settingsStep"
          vertical
          flat
          animated
          header-nav
          class="__llm-stepper"
          @click="onStepperClick"
        >
          <!-- Step 1: プロバイダー選択 -->
          <q-step
            name="select"
            :title="t('app.llm.stepProviderSelect')"
            icon="category"
            color="dark"
            :done="!!editingProviderName"
          >
            <div class="q-mt-sm">
              <q-select
                id="llm-provider-select"
                v-model="editingProviderName"
                :options="SUPPORTED_PROVIDERS"
                option-label="label"
                option-value="name"
                emit-value
                map-options
                color="dark"
              >
                <template #option="scope">
                  <q-item v-bind="scope.itemProps" @click="onSelectProvider(scope.opt.name)">
                    <q-item-section>
                      <q-item-label>{{ scope.opt.label }}</q-item-label>
                    </q-item-section>
                  </q-item>
                </template>
              </q-select>
            </div>
          </q-step>

          <!-- Step 2: APIキー管理 -->
          <q-step
            name="apikeys"
            :title="t('app.llm.stepApiKey')"
            icon="vpn_key"
            color="dark"
          >
            <div v-if="editingEntry">
              <!-- APIキーのリスト -->
              <q-list v-if="editingEntry.config.keys.length > 0" class="q-mb-md">
                <q-item
                  v-for="(key, idx) in editingEntry.config.keys"
                  :key="idx"
                  class="q-px-none"
                  style="min-height: 60px;"
                >
                  <q-item-section>
                    <div class="row items-center no-wrap q-mb-xs">
                      <q-icon :name="key.is_new ? 'fiber_new' : 'lock'" color="dark" size="xs" class="q-mr-xs" />
                      <input
                        v-model="key.name"
                        class="__llm-key-name-input text-caption text-bold text-dark"
                      />
                    </div>
                    
                    <q-input
                      v-if="key.is_new"
                      v-model="key.value"
                      dense
                      borderless
                      type="password"
                      :placeholder="t('app.llm.labelApiKeyPlain')"
                      class="full-width"
                      input-style="font-size: 13px;"
                    />
                    <q-input
                      v-else
                      :model-value="maskedValue"
                      dense
                      borderless
                      readonly
                      class="full-width"
                      input-style="font-size: 13px; color: #ffffff;"
                    />
                  </q-item-section>

                  <q-item-section side>
                    <div class="row items-center no-wrap">
                      <div class="column items-center q-mr-sm">
                        <div class="text-caption text-dark" style="font-size: 9px; line-height: 1;">{{ t('app.llm.labelWeight') }}</div>
                        <q-input
                          v-model.number="key.weight"
                          dense
                          borderless
                          type="number"
                          style="width:40px"
                          input-class="text-center no-spinner"
                          input-style="font-size: 13px; padding: 0;"
                        />
                      </div>
                      <q-btn
                        flat
                        round
                        dense
                        icon="delete"
                        color="negative"
                        size="sm"
                        @click="llmStore.removeKey(editingProviderName, idx)"
                      />
                    </div>
                  </q-item-section>
                </q-item>
              </q-list>

              <!-- 新規追加エリア -->
              <div class="row no-wrap items-center q-gutter-sm q-mt-md">
                <q-input
                  id="llm-new-key-input"
                  v-model="newKeyValue"
                  type="password"
                  :placeholder="t('app.llm.labelApiKey')"
                  class="col"
                  color="dark"
                  filled
                  borderless
                  input-style="font-size: 13px;"
                  @keydown.enter="onAddKey"
                  @paste="onPasteKey"
                />
              </div>
            </div>

          </q-step>

          <!-- Step 3: 保存・反映 -->
          <q-step
            name="save"
            :title="t('app.llm.stepSync')"
            icon="sync"
            color="dark"
          >
            <p class="text-caption text-white" style="text-shadow: 1px 1px 3px rgba(0, 0, 0, 0.4); font-weight: bold;">
              {{ t('app.llm.syncDescription') }}
            </p>
            <div class="row justify-end q-mt-md">
              <q-btn
                id="llm-save-btn"
                :loading="llmStore.isSaving"
                :label="t('app.llm.btnSync')"
                icon="sync"
                color="app"
                unelevated
                rounded
                :disable="!canSave"
                @click="onSaveAndSync"
              />
            </div>
          </q-step>

          <!-- Step 4: 通信テスト -->
          <q-step
            name="test"
            :title="t('app.llm.stepTest')"
            icon="science"
            color="dark"
          >
            <p class="text-caption text-white" style="text-shadow: 1px 1px 3px rgba(0, 0, 0, 0.4); font-weight: bold;">
              {{ t('app.llm.testDescription') }}
            </p>
            <div v-if="editingEntry && editingEntry.config.keys.length > 0" class="row q-col-gutter-sm q-mt-sm">
              <div v-for="msg in testMessages" :key="msg" class="col-12">
                <q-btn
                  outline
                  color="white"
                  text-color="white"
                  class="full-width"
                  style="font-size: 12px; text-shadow: 1px 1px 3px rgba(0, 0, 0, 0.4); font-weight: bold; background: rgba(255, 255, 255, 0.1); border-radius: 8px; justify-content: flex-start; text-transform: none;"
                  @click="onTestMessage(msg)"
                >
                  <div class="ellipsis">{{ msg }}</div>
                </q-btn>
              </div>
            </div>

            <!-- 結果表示エリア -->
            <div v-if="testResult"
                 class="q-mt-md q-pa-sm"
                 :class="testSuccess ? 'bg-app' : ''"
                 :style="{
                   background: testSuccess ? undefined : 'color-mix(in srgb, var(--q-negative), transparent 80%)',
                   borderRadius: '10px',
                   color: '#fff',
                   textShadow: '1px 1px 3px rgba(0, 0, 0, 0.5)'
                 }">
              <div v-if="!testSuccess" class="text-bold q-mb-xs">
                {{ t('app.llm.testFail') }}
              </div>
              <div v-if="testSuccess" class="text-caption q-mb-xs">{{ t('app.llm.testResult') }}</div>
              <div class="text-body2" style="white-space: pre-wrap; word-break: break-word; font-weight: bold;">
                {{ testResult }}
              </div>
            </div>
          </q-step>
        </q-stepper>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { t, tm } from 'src/utils/some'
import WaterRipple from 'src/components/effects/WaterRipple.vue'
import { useLlmStore, SUPPORTED_PROVIDERS } from 'src/stores/llm-store'
import { useMainStore } from 'src/stores/main-store'
import { useQuasar } from 'quasar'
import { showNotify, showWarn } from 'src/utils/notify'
import { testLlmCommunication } from 'src/utils/rest'

const $q = useQuasar()
const llmStore = useLlmStore()
const mainStore = useMainStore()
const DEFAULT_PROVIDER = 'openai'

// ============================================================
// 初期化
// ============================================================
onMounted(async () => {
  await llmStore.fetchProviders()
  // 初期値のプロバイダーをストア側に反映させておく
  llmStore.ensureProvider(DEFAULT_PROVIDER)
  // メッセージの初期生成
  generateTestMessages()
})





// ============================================================
// プロバイダー設定パネル
// ============================================================
const settingsStep = ref<'select' | 'apikeys' | 'save' | 'test'>('select')

watch(settingsStep, (newStep) => {
  if (newStep === 'test') {
    generateTestMessages()
  }
})

/** 言語設定が変わったらテストメッセージを再生成する */
watch(() => mainStore.lang, () => {
  if (settingsStep.value === 'test') {
    generateTestMessages()
  }
})

/** ステッパーのヘッダータイトル（既存UI）がクリックされた時の処理 */
const onStepperClick = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  const titleText = t('app.llm.stepTest')
  
  // ステッパーのタブ全体（ヘッダー領域）を特定する
  // 垂直ステッパーの場合、各ステップのクリック可能な領域は .q-stepper__tab クラスを持つ
  const tab = target.closest('.q-stepper__tab')
  
  if (tab && tab.textContent?.includes(titleText)) {
    // 既にテストステップにいる場合のみリフレッシュを実行
    if (settingsStep.value === 'test') {
      generateTestMessages()
    }
  }
}
const editingProviderName = ref(DEFAULT_PROVIDER)
const newKeyValue = ref('')

/** マスク表示用の固定文字列（暗号化済みの値は見せない） */
const maskedValue = t('app.llm.apiKeyMask')

/** 現在編集中のプロバイダーエントリ（llmStoreから参照） */
const editingEntry = computed(() =>
  llmStore.providers.find(p => p.provider_name === editingProviderName.value) ?? null
)

/** 保存可能かどうかのバリデーション */
const canSave = computed(() => {
  // 追加された新規キー（is_new: true）がすべて空でないことのみをチェック
  // これにより、キーをすべて削除して「空」の状態に上書き保存することも可能になります
  return llmStore.providers.every(p =>
    p.config.keys.every(k => !k.is_new || k.value.trim().length > 0)
  )
})

const onSelectProvider = (name: string) => {
  editingProviderName.value = name
  llmStore.ensureProvider(name)
  // 選択したら自動的に次のステップへ進む
  settingsStep.value = 'apikeys'
}

const onAddKey = () => {
  const val = newKeyValue.value.trim()
  if (!val || !editingProviderName.value) return
  llmStore.addNewKey(editingProviderName.value, val)
  newKeyValue.value = ''
}

const onPasteKey = (e: ClipboardEvent) => {
  // ペーストされたテキストを直接取得
  const pastedText = e.clipboardData?.getData('text') || ''
  if (pastedText.trim() && editingProviderName.value) {
    // デフォルトのペースト挙動（入力欄への反映）を停止
    e.preventDefault()
    // ストアに直接追加
    llmStore.addNewKey(editingProviderName.value, pastedText.trim())
    // 入力欄を確実に空にする
    newKeyValue.value = ''
  }
}

const onSaveAndSync = async () => {
  const success = await llmStore.saveProviders()
  if (success) {
    showNotify(t('app.llm.syncSuccess'))
    generateTestMessages()
    settingsStep.value = 'test'
  } else {
    showWarn(llmStore.lastError ?? t('app.llm.saveFailed'))
  }
}

// ============================================================
// テストステップ
// ============================================================
const testMessages = ref<string[]>([])
const testResult = ref<string>('')
const testSuccess = ref<boolean>(false)

const generateTestMessages = () => {
  const presets = tm('app.llm.testPresets') as string[]
  if (!presets || presets.length === 0) return
  
  const shuffled = [...presets].sort(() => 0.5 - Math.random())
  testMessages.value = shuffled.slice(0, 3)
  testResult.value = ''
  testSuccess.value = false
}

const getTestModel = (provider: string) => {
  if (provider === 'openai') return 'openai/gpt-4.1-nano'
  if (provider === 'anthropic') return 'anthropic/claude-haiku-4-5'
  if (provider === 'gemini') return 'gemini/gemini-3.1-flash-lite'
  
  const p = SUPPORTED_PROVIDERS.find(x => x.name === provider)
  const modelName = p?.models[0] || 'unknown'
  return `${provider}/${modelName}`
}

const onTestMessage = async (msg: string) => {
  if (!mainStore.token) return
  
  mainStore.setIsLoaderOn(true)
  try {
    const model = getTestModel(editingProviderName.value)
    const res = await testLlmCommunication(mainStore.token, model, msg)
    testSuccess.value = res.success
    testResult.value = res.success ? (res.content || '') : (res.error || 'Unknown error')
  } catch (e) {
    testSuccess.value = false
    testResult.value = String(e)
  } finally {
    mainStore.setIsLoaderOn(false)
  }
}


</script>

<style scoped>
.__llm-key-name-input {
  background: transparent;
  border: none;
  outline: none;
  padding: 0;
  margin: 0;
  flex: 1;
  min-width: 0;
  font-size: 11px;
  font-weight: 700;
  color: var(--q-dark);
  line-height: 1;
}

/* ===== 設定パネル ===== */
.__llm-settings-header {
  display: flex;
  align-items: center;
  font-size: 15px;
  font-weight: 700;
  color: var(--q-primary);
  margin-bottom: 16px;
}
.__llm-stepper {
  border: none;
  background: transparent;
}
</style>
<style>
/* スピンボタン（上下矢印）を非表示にする */
.no-spinner[type=number] {
  -moz-appearance: textfield;
  appearance: textfield;
}
.no-spinner::-webkit-outer-spin-button,
.no-spinner::-webkit-inner-spin-button {
  -webkit-appearance: none;
  appearance: none;
  margin: 0;
}
</style>
