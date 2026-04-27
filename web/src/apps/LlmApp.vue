<template>
  <!-- HarunohiApp と同じ構造: タブパネル + ボトムタブバー -->
  <q-tab-panels v-model="currentTab" animated class="__harunohi-tabpanels">

    <!-- ===== チャットテスト パネル ===== -->
    <q-tab-panel class="__harunohi-tabpanel __llm-panel" name="CHAT">
      <div class="__llm-panel-inner">
        <!-- ヘッダー: 目的を明示する -->
        <div class="__llm-chat-header">
          <div class="__llm-chat-header-title">
            <BrainAI1Icon style="width:20px;height:20px;" class="q-mr-xs" />
            {{ t('app.llm.chatSubtitle') }}
          </div>
          <div class="__llm-chat-header-sub">
            {{ t('app.llm.chatHint') }}
          </div>
        </div>

        <!-- プロバイダー / モデル 選択 -->
        <div class="__llm-chat-selectors row q-gutter-sm q-mb-sm">
          <q-select
            id="llm-provider-select"
            v-model="llmStore.selectedProviderName"
            :options="providerOptions"
            emit-value
            map-options
            dense
            outlined
            :label="t('app.llm.provider')"
            class="col"
            @update:model-value="onProviderChange"
          />
          <q-select
            id="llm-model-select"
            v-model="llmStore.selectedModel"
            :options="llmStore.availableModels"
            dense
            outlined
            :label="t('app.llm.model')"
            class="col"
            :disable="!llmStore.selectedProviderName"
          />
          <q-btn
            id="llm-clear-chat-btn"
            flat
            dense
            round
            icon="delete_sweep"
            color="grey-6"
            @click="llmStore.clearChat()"
          >
            <q-tooltip>{{ t('app.llm.clearChat') }}</q-tooltip>
          </q-btn>
        </div>

        <!-- メッセージ表示エリア -->
        <div ref="chatScrollEl" class="__llm-chat-messages">
          <div
            v-for="msg in llmStore.chatMessages"
            :key="msg.id"
            :class="['__llm-chat-bubble', msg.role === 'user' ? '__llm-bubble-user' : '__llm-bubble-assistant']"
          >
            <!-- ストリーミング中のカーソル表示 -->
            <span>{{ msg.content }}<span v-if="msg.isStreaming" class="__llm-cursor">▋</span></span>
          </div>
          <!-- メッセージがない場合のプレースホルダー -->
          <div v-if="llmStore.chatMessages.length === 0" class="__llm-chat-empty">
            <BotIcon style="width:48px;height:48px;opacity:0.3;" />
            <div class="q-mt-sm text-grey-5 text-caption">{{ t('app.llm.selectProviderHint') }}</div>
          </div>
        </div>

        <!-- 入力エリア -->
        <div class="__llm-chat-input-wrap">
          <q-input
            id="llm-chat-input"
            v-model="chatInput"
            outlined
            dense
            autogrow
            :placeholder="t('app.llm.messagePlaceholder')"
            class="__llm-chat-input"
            :disable="llmStore.isChatStreaming"
            @keydown.enter.exact.prevent="onSendChat"
          />
          <q-btn
            id="llm-send-btn"
            :loading="llmStore.isChatStreaming"
            :disable="!chatInput.trim() || !llmStore.selectedProviderName || !llmStore.selectedModel"
            round
            color="primary"
            icon="send"
            class="q-ml-sm"
            @click="onSendChat"
          />
        </div>
      </div>
    </q-tab-panel>

    <!-- ===== プロバイダー設定 パネル ===== -->
    <q-tab-panel class="__harunohi-tabpanel __llm-panel" name="SETTINGS">
      <div class="__llm-panel-inner">
        <div class="__llm-settings-header">
          <KeyAI1Icon style="width:20px;height:20px;" class="q-mr-xs" />
          <span>{{ t('app.llm.settingsTitle') }}</span>
        </div>

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
            class="__llm-stepper"
          >
            <!-- Step 1: プロバイダー選択 -->
            <q-step
              name="select"
              :title="t('app.llm.stepProviderSelect')"
              icon="category"
              :done="!!editingProviderName"
            >
              <div class="row q-gutter-sm q-mt-xs">
                <q-btn
                  v-for="sp in SUPPORTED_PROVIDERS"
                  :id="`llm-provider-btn-${sp.name}`"
                  :key="sp.name"
                  :label="sp.label"
                  :icon="sp.icon"
                  :color="editingProviderName === sp.name ? 'primary' : 'grey-3'"
                  :text-color="editingProviderName === sp.name ? 'white' : 'dark'"
                  unelevated
                  rounded
                  size="sm"
                  @click="onSelectProvider(sp.name)"
                />
              </div>
              <q-stepper-navigation>
                <q-btn
                  id="llm-step-next-btn"
                  :label="t('app.common.next')"
                  color="primary"
                  :disable="!editingProviderName"
                  @click="settingsStep = 'apikeys'"
                />
              </q-stepper-navigation>
            </q-step>

            <!-- Step 2: APIキー管理 -->
            <q-step
              name="apikeys"
              :title="t('app.llm.stepApiKey')"
              icon="vpn_key"
              :done="settingsStep === 'save'"
            >
              <div v-if="editingEntry">
                <!-- 既存キーの一覧 -->
                <div
                  v-for="(key, idx) in editingEntry.config.keys"
                  :key="idx"
                  class="__llm-key-row q-mb-sm"
                >
                  <!-- 既存キーは「設定済み」と表示し、値は復号不可なので見せない -->
                  <div class="__llm-key-badge">
                    <q-icon v-if="!key.is_new" name="lock" size="14px" class="q-mr-xs text-grey-6" />
                    <q-icon v-else name="fiber_new" size="14px" class="q-mr-xs text-positive" />
                    <span class="text-caption text-grey-7">
                      {{ key.is_new ? t('app.llm.newKey') : t('app.llm.storedKey') }}
                    </span>
                  </div>

                  <!-- 新規キーの場合は実際に値を編集可能 -->
                  <q-input
                    v-if="key.is_new"
                    :id="`llm-key-input-${idx}`"
                    v-model="key.value"
                    dense
                    outlined
                    type="password"
                    :label="t('app.llm.labelApiKeyPlain')"
                    class="q-mt-xs"
                  />
                  <!-- 既存キーの場合はマスク表示のみ（復号不可）-->
                  <q-input
                    v-else
                    :id="`llm-key-masked-${idx}`"
                    :model-value="maskedValue"
                    dense
                    outlined
                    readonly
                    :label="t('app.llm.labelApiKeyEncrypted')"
                    class="q-mt-xs"
                  />

                  <div class="row items-center q-mt-xs q-gutter-xs">
                    <q-input
                      :id="`llm-key-weight-${idx}`"
                      v-model.number="key.weight"
                      dense
                      outlined
                      type="number"
                      :label="t('app.llm.labelWeight')"
                      style="width:80px"
                      :min="1"
                    />
                    <q-btn
                      :id="`llm-key-delete-${idx}`"
                      flat
                      round
                      dense
                      icon="delete"
                      color="negative"
                      @click="llmStore.removeKey(editingProviderName, idx)"
                    >
                      <q-tooltip>{{ t('app.llm.deleteKeyTooltip') }}</q-tooltip>
                    </q-btn>
                  </div>
                </div>

                <!-- 新規キー追加フォーム -->
                <div class="__llm-add-key-form q-mt-md">
                  <q-input
                    id="llm-new-key-input"
                    v-model="newKeyValue"
                    dense
                    outlined
                    type="password"
                    :label="t('app.llm.labelApiKey')"
                    class="q-mb-xs"
                  />
                  <q-btn
                    id="llm-add-key-btn"
                    flat
                    :label="t('app.llm.btnAddKey')"
                    icon="add"
                    color="primary"
                    :disable="!newKeyValue.trim()"
                    @click="onAddKey"
                  />
                </div>
              </div>

              <q-stepper-navigation>
                <q-btn
                  id="llm-step-back-btn"
                  flat
                  :label="t('app.common.back')"
                  color="grey"
                  class="q-mr-sm"
                  @click="settingsStep = 'select'"
                />
                <q-btn
                  id="llm-step-save-btn"
                  :label="t('app.common.next')"
                  color="primary"
                  @click="settingsStep = 'save'"
                />
              </q-stepper-navigation>
            </q-step>

            <!-- Step 3: 保存・Bifrost同期 -->
            <q-step
              name="save"
              :title="t('app.llm.stepSync')"
              icon="sync"
            >
              <p class="text-caption text-grey-7">
                {{ t('app.llm.syncDescription') }}
              </p>
              <q-stepper-navigation>
                <q-btn
                  flat
                  :label="t('app.common.back')"
                  color="grey"
                  class="q-mr-sm"
                  @click="settingsStep = 'apikeys'"
                />
                <q-btn
                  id="llm-save-btn"
                  :loading="llmStore.isSaving"
                  :label="t('app.llm.btnSync')"
                  icon="cloud_upload"
                  color="positive"
                  @click="onSaveAndSync"
                />
              </q-stepper-navigation>
            </q-step>
          </q-stepper>
        </template>
      </div>
    </q-tab-panel>

  </q-tab-panels>

  <!-- ボトムタブバー（HarunohiApp と同じ構造） -->
  <div :class="!IS_TAURI_DESKTOP ? 'fixed-bottom' : ''">
    <div class="__harunohi-tabs">
      <div class="__harunohi-tabs-tab" @click="currentTab = 'CHAT'">
        <BotIcon :class="{ active: currentTab === 'CHAT' }" />
      </div>
      <div class="__harunohi-tabs-tab" @click="onClickSettingsTab">
        <KeyAI1Icon :class="{ active: currentTab === 'SETTINGS' }" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted } from 'vue'
import { isTauriDesktop, t } from 'src/utils/some'
import { useLlmStore, SUPPORTED_PROVIDERS, type ChatMessage } from 'src/stores/llm-store'
import { useMainStore } from 'src/stores/main-store'
import { API_BASE_URL } from 'src/configs/settings'
import { PATH_LMGW_OPENAI_V1 } from 'src/consts/generated_constants'
import { useQuasar } from 'quasar'
import BrainAI1Icon from 'src/components/icons/BrainAI1Icon.vue'
import BotIcon from 'src/components/icons/BotIcon.vue'
import KeyAI1Icon from 'src/components/icons/KeyAI1Icon.vue'

const $q = useQuasar()
const IS_TAURI_DESKTOP = isTauriDesktop()
const llmStore = useLlmStore()
const mainStore = useMainStore()

// ============================================================
// タブ管理
// ============================================================
const currentTab = ref<'CHAT' | 'SETTINGS'>('CHAT')

const onClickSettingsTab = async () => {
  currentTab.value = 'SETTINGS'
  // 設定タブを開くたびに最新データをフェッチする
  await llmStore.fetchProviders()
}

// ============================================================
// チャットパネル
// ============================================================
const chatInput = ref('')
const chatScrollEl = ref<HTMLElement | null>(null)

/** ドロップダウン用のプロバイダーオプション（設定済みのみ表示） */
const providerOptions = computed(() =>
  llmStore.configuredProviders.map(sp => ({ label: sp.label, value: sp.name }))
)

const onProviderChange = () => {
  // プロバイダーが変わったらモデル選択をリセット
  llmStore.selectedModel = ''
  if (llmStore.availableModels.length > 0) {
    llmStore.selectedModel = llmStore.availableModels[0] ?? ''
  }
}

/** チャット欄を最下部にスクロールする */
const scrollToBottom = async () => {
  await nextTick()
  if (chatScrollEl.value) {
    chatScrollEl.value.scrollTop = chatScrollEl.value.scrollHeight
  }
}

/** メッセージが追加されるたびに自動スクロール */
watch(() => llmStore.chatMessages.length, scrollToBottom)

/**
 * チャットを送信し、Bifrost（/v1/lmgw/v1/chat/completions）へ
 * SSEストリームとしてリクエストする。
 *
 * Bifrostへは MYCUTE バックエンドのプロキシ（/v1/lmgw/*）経由でアクセスする。
 * 認証は MYCUTE の JWT を Bearer トークンとして付与し、
 * バックエンドが BIFROST_AUTH_SECRET に差し替えて転送する。
 */
const onSendChat = async () => {
  const text = chatInput.value.trim()
  if (!text || !llmStore.selectedProviderName || !llmStore.selectedModel) return
  if (llmStore.isChatStreaming) return

  chatInput.value = ''
  llmStore.isChatStreaming = true

  // ユーザーメッセージを追加
  const userMsg: ChatMessage = {
    id: `user-${Date.now()}`,
    role: 'user',
    content: text,
  }
  llmStore.chatMessages.push(userMsg)

  // アシスタントのプレースホルダーを追加（ストリーミング中はここに追記）
  const assistantMsg: ChatMessage = {
    id: `assistant-${Date.now()}`,
    role: 'assistant',
    content: '',
    isStreaming: true,
  }
  llmStore.chatMessages.push(assistantMsg)

  try {
    const token = mainStore.token
    // Bifrost の OpenAI互換エンドポイントをバックエンドプロキシ経由で呼ぶ
    const url = `${API_BASE_URL}${PATH_LMGW_OPENAI_V1}/chat/completions`
    const body = JSON.stringify({
      model: `${llmStore.selectedProviderName}/${llmStore.selectedModel}`,
      stream: true,
      messages: [{ role: 'user', content: text }],
      // プロバイダー指定は Bifrost のルーティングヘッダでも並行して行う
    })

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
        // Bifrostにどのプロバイダーを使うか指示するヘッダー
        'x-bifrost-routing': JSON.stringify({ provider: llmStore.selectedProviderName }),
      },
      body,
    })

    if (!response.ok || !response.body) {
      assistantMsg.content = `[${t('app.common.error')}] HTTP ${response.status}`
      assistantMsg.isStreaming = false
      return
    }

    // SSEストリームを読み込む
    const reader = response.body.getReader()
    const decoder = new TextDecoder()
    let accumulated = ''

    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      const chunk = decoder.decode(value, { stream: true })
      const lines = chunk.split('\n')

      for (const line of lines) {
        if (!line.startsWith('data: ')) continue
        const data = line.slice(6).trim()
        if (data === '[DONE]') break

        try {
          const parsed = JSON.parse(data)
          const delta = parsed?.choices?.[0]?.delta?.content ?? ''
          if (delta) {
            accumulated += delta
            assistantMsg.content = accumulated
          }
        } catch {
          // パース失敗は無視（SSEの空行など）
        }
      }
    }

  } catch (e) {
    assistantMsg.content = `[${t('app.common.error')}] ${String(e)}`
  } finally {
    assistantMsg.isStreaming = false
    llmStore.isChatStreaming = false
  }
}

// ============================================================
// プロバイダー設定パネル
// ============================================================
const settingsStep = ref<'select' | 'apikeys' | 'save'>('select')
const editingProviderName = ref('')
const newKeyValue = ref('')

/** マスク表示用の固定文字列（暗号化済みの値は見せない） */
const maskedValue = t('app.llm.apiKeyMask')

/** 現在編集中のプロバイダーエントリ（llmStoreから参照） */
const editingEntry = computed(() =>
  llmStore.providers.find(p => p.provider_name === editingProviderName.value) ?? null
)

const onSelectProvider = (name: string) => {
  editingProviderName.value = name
  llmStore.ensureProvider(name)
}

const onAddKey = () => {
  const val = newKeyValue.value.trim()
  if (!val || !editingProviderName.value) return
  llmStore.addNewKey(editingProviderName.value, val)
  newKeyValue.value = ''
}

const onSaveAndSync = async () => {
  const success = await llmStore.saveProviders()
  if (success) {
    $q.notify({ type: 'positive', message: t('app.llm.syncSuccess'), position: 'top' })
    settingsStep.value = 'select'
    editingProviderName.value = ''
  } else {
    $q.notify({
      type: 'negative',
      message: llmStore.lastError ?? t('app.llm.saveFailed'),
      position: 'top',
    })
  }
}

// ============================================================
// 初期化
// ============================================================
onMounted(async () => {
  // 最初にプロバイダー一覧をフェッチ（チャットのプロバイダー選択に使用するため）
  await llmStore.fetchProviders()
})
</script>

<style scoped>
/* ===== パネル共通 ===== */
.__llm-panel {
  padding: 0;
}
.__llm-panel-inner {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 16px 16px 80px; /* ボトムタブバー分の余白 */
  overflow-y: auto;
  box-sizing: border-box;
}

/* ===== チャットパネル ===== */
.__llm-chat-header {
  margin-bottom: 12px;
}
.__llm-chat-header-title {
  display: flex;
  align-items: center;
  font-size: 14px;
  font-weight: 700;
  color: var(--q-primary);
}
.__llm-chat-header-sub {
  font-size: 11px;
  color: #9e9e9e;
  margin-top: 2px;
  padding: 4px 8px;
  border-left: 3px solid var(--q-primary);
  background: rgba(0, 0, 0, 0.03);
  border-radius: 0 4px 4px 0;
}
.__llm-chat-selectors {
  flex-shrink: 0;
}
.__llm-chat-messages {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 8px 0;
  min-height: 0;
}
.__llm-chat-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  padding: 40px 0;
}
.__llm-chat-bubble {
  max-width: 85%;
  padding: 10px 14px;
  border-radius: 16px;
  line-height: 1.5;
  font-size: 14px;
  white-space: pre-wrap;
  word-break: break-word;
}
.__llm-bubble-user {
  align-self: flex-end;
  background: var(--q-primary);
  color: white;
  border-bottom-right-radius: 4px;
}
.__llm-bubble-assistant {
  align-self: flex-start;
  background: #f5f5f5;
  color: #212121;
  border-bottom-left-radius: 4px;
}
.__llm-cursor {
  animation: llm-blink 1s step-start infinite;
  display: inline-block;
}
@keyframes llm-blink {
  0%, 100% { opacity: 1; }
  50%       { opacity: 0; }
}
.__llm-chat-input-wrap {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  flex-shrink: 0;
  padding-top: 8px;
}
.__llm-chat-input {
  flex: 1;
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
}
.__llm-key-row {
  background: #fafafa;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  padding: 10px 12px;
}
.__llm-key-badge {
  display: flex;
  align-items: center;
}
.__llm-add-key-form {
  background: #f5f5f5;
  border-radius: 8px;
  padding: 12px;
}
</style>
