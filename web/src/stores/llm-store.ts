/**
 * LlmStore — LMGW（Bifrost）管理に関する状態管理ストア
 *
 * 責務：
 * - DBから取得したプロバイダー設定の保持
 * - 設定の保存・同期処理の状態管理（ローディング、エラー）
 * - チャットテストパネルで使用するメッセージ履歴と状態の管理
 *
 * セキュリティポリシー：
 * - フロントエンドはAPIキーの実体を決して復号・表示しない
 * - バックエンドから受け取った暗号化済みの値はそのまま保持し、
 *   is_new=false としてそのままバックエンドへ送り返す
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getLmgwProviders, saveLmgwProviders } from 'src/utils/rest'
import { t } from 'src/utils/some'
import type {
  LmgwProviderEntry,
  LmgwProviderConfig,
  SupportedProvider,
  SaveLmgwProvidersReq,
} from 'src/models/lmgw'
import { useMainStore } from 'src/stores/main-store'

// ============================================================
// サポートするプロバイダー定義（順序が表示順になる）
// ============================================================
export const SUPPORTED_PROVIDERS: SupportedProvider[] = [
  {
    name: 'openai',
    label: 'OpenAI',
    icon: 'bolt',
    models: ['gpt-5.5-pro', 'gpt-5.5', 'gpt-5.4', 'gpt-5.4-mini', 'gpt-5.4-nano', 'gpt-4.1-nano'],
  },
  {
    name: 'anthropic',
    label: 'Anthropic (Claude)',
    icon: 'psychology',
    models: ['claude-opus-4-7', 'claude-sonnet-4-6', 'claude-haiku-4-5'],
  },
  {
    name: 'gemini',
    label: 'Google Gemini',
    icon: 'auto_awesome',
    models: ['gemini-3.1-pro', 'gemini-3-flash', 'gemini-3.1-flash-lite', 'gemini-2.5-pro', 'gemini-2.5-flash'],
  },
]

// ============================================================
// チャットメッセージ型
// ============================================================
export interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  isStreaming?: boolean // アシスタントの返信がストリーミング中かどうか
}

// ============================================================
// Pinia ストア定義
// ============================================================
export const useLlmStore = defineStore('llm', () => {
  // ----- プロバイダー設定 -----
  /** DBから取得したプロバイダーエントリの一覧（config_jsonをパース済み） */
  const providers = ref<LmgwProviderEntry[]>([])

  /** 設定の取得中かどうか */
  const isFetchingProviders = ref(false)

  /** 設定の保存・Bifrost同期中かどうか */
  const isSaving = ref(false)

  /** 最後に発生したエラーメッセージ */
  const lastError = ref<string | null>(null)

  // ----- チャット -----
  /** チャットテストパネルのメッセージ履歴 */
  const chatMessages = ref<ChatMessage[]>([])

  /** チャットが送信中（ストリーミング中）かどうか */
  const isChatStreaming = ref(false)

  /** チャットで選択中のプロバイダー名 */
  const selectedProviderName = ref<string>('')

  /** チャットで選択中のモデル名 */
  const selectedModel = ref<string>('')

  // ============================================================
  // Computed
  // ============================================================

  /**
   * 設定済みのプロバイダーを持つ SupportedProvider の一覧。
   * チャットパネルでプロバイダーを選ぶドロップダウンに使用する。
   */
  const configuredProviders = computed<SupportedProvider[]>(() =>
    SUPPORTED_PROVIDERS.filter(sp =>
      providers.value.some(p => p.provider_name === sp.name)
    )
  )

  /**
   * 現在選択中のプロバイダーのモデル一覧。
   * チャットパネルでモデルを選ぶドロップダウンに使用する。
   */
  const availableModels = computed<string[]>(() => {
    const sp = SUPPORTED_PROVIDERS.find(p => p.name === selectedProviderName.value)
    return sp?.models ?? []
  })

  // ============================================================
  // Actions
  // ============================================================

  /**
   * DBからプロバイダー設定を取得してストアに保持する。
   * config_json は JSON 文字列のためここでパースする。
   */
  const fetchProviders = async (): Promise<void> => {
    const mainStore = useMainStore()
    const token = mainStore.token
    if (!token) { lastError.value = t('page.login.error.failedToSignIn'); return }

    isFetchingProviders.value = true
    lastError.value = null
    try {
      const res = await getLmgwProviders(token)
      if (!res) { lastError.value = t('app.llm.fetchFailed'); return }

      providers.value = res.providers.map(p => {
        let config: LmgwProviderConfig = { keys: [] }
        try {
          config = JSON.parse(p.config_json) as LmgwProviderConfig
        } catch {
          // config_json のパースに失敗した場合は空のキー配列で初期化
          console.error(`Failed to parse config_json for provider: ${p.provider_name}`)
        }
        return { provider_name: p.provider_name, config }
      })
    } finally {
      isFetchingProviders.value = false
    }
  }

  /**
   * プロバイダー設定をバックエンドへ保存し、Bifrostに同期する。
   *
   * 重要なセキュリティポリシー：
   * - is_new=true のキーの value は平文。バックエンドが暗号化してDBに保存する。
   * - is_new=false のキーの value は暗号化済みの値。そのままバックエンドへ渡す。
   * - フロントエンドはいかなる場合も複号を行わない。
   */
  const saveProviders = async (): Promise<boolean> => {
    const mainStore = useMainStore()
    const token = mainStore.token
    if (!token) { lastError.value = t('page.login.error.failedToSignIn'); return false }

    isSaving.value = true
    lastError.value = null
    try {
      const req: SaveLmgwProvidersReq = {
        providers: providers.value.map(p => ({
          provider_name: p.provider_name,
          config_json: JSON.stringify(p.config),
        }))
      }
      const res = await saveLmgwProviders(token, req)
      if (!res || !res.success) { lastError.value = t('app.llm.saveFailed'); return false }
      // 保存成功後は最新の状態をDBから再取得して is_new フラグをリセットする
      await fetchProviders()
      return true
    } finally {
      isSaving.value = false
    }
  }

  /**
   * 指定プロバイダーの設定エントリを初期化する（初めて設定するとき）。
   * すでに存在する場合は何もしない。
   */
  const ensureProvider = (providerName: string): void => {
    if (!providers.value.some(p => p.provider_name === providerName)) {
      providers.value.push({ provider_name: providerName, config: { keys: [] } })
    }
  }

  /**
   * 指定プロバイダーに新しいAPIキー（平文）を追加する。
   * is_new=true をセットすることで、次回保存時にバックエンドが暗号化する。
   */
  const addNewKey = (providerName: string, plainValue: string, weight = 1): void => {
    ensureProvider(providerName)
    const entry = providers.value.find(p => p.provider_name === providerName)
    if (!entry) return
    entry.config.keys.push({ value: plainValue, weight, is_new: true })
  }

  /**
   * 指定プロバイダーの指定インデックスのキーを削除する。
   */
  const removeKey = (providerName: string, keyIndex: number): void => {
    const entry = providers.value.find(p => p.provider_name === providerName)
    if (!entry) return
    entry.config.keys.splice(keyIndex, 1)
  }

  /**
   * 指定プロバイダーの指定インデックスのキーの weight を更新する。
   */
  const updateKeyWeight = (providerName: string, keyIndex: number, weight: number): void => {
    const entry = providers.value.find(p => p.provider_name === providerName)
    if (!entry) return
    const key = entry.config.keys[keyIndex]
    if (key) key.weight = weight
  }

  /**
   * チャットメッセージ履歴をリセットする。
   */
  const clearChat = (): void => {
    chatMessages.value = []
  }

  return {
    // State
    providers,
    isFetchingProviders,
    isSaving,
    lastError,
    chatMessages,
    isChatStreaming,
    selectedProviderName,
    selectedModel,
    // Computed
    configuredProviders,
    availableModels,
    // Actions
    fetchProviders,
    saveProviders,
    ensureProvider,
    addNewKey,
    removeKey,
    updateKeyWeight,
    clearChat,
  }
})
