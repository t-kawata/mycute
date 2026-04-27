/*
 * LMGW（Bifrost）管理に関する型定義
 * バックエンドの rtreq/lmgws_req.rs および rtres/lmgws_res.rs と同期しています。
 */

// ============================================================
// config_json 内部の構造（Bifrostプロバイダー設定）
// ============================================================

/**
 * プロバイダーが持つ個々のAPIキーオブジェクト。
 * config_json 内の keys 配列の要素。
 *
 * - value: 暗号化済みの値（バックエンドから取得時）または平文（新規登録時）
 * - weight: Bifrostが複数キーを使う際の重み付け（1以上の整数）
 * - is_new: true のとき、value は平文としてバックエンドに送信され暗号化される
 *           false のとき、value は暗号化済みとしてそのままDBに保存される
 */
export interface LmgwKey {
  value: string
  weight: number
  is_new: boolean
}

/**
 * config_json 全体の構造（Bifrostが期待するプロバイダー設定JSON）。
 * provider_name に対応するプロバイダーの全設定を持つ。
 */
export interface LmgwProviderConfig {
  keys: LmgwKey[]
  [key: string]: unknown // Bifrost が将来追加するフィールドへの拡張性
}

// ============================================================
// バックエンド API のリクエスト / レスポンス型
// ============================================================

/**
 * GET /v1/lmgw/manage/providers レスポンス内の1プロバイダー。
 * config_json は JSON 文字列のまま返却されるため、パース後は LmgwProviderConfig 型になる。
 */
export interface ManageLmgwProviderRes {
  provider_name: string
  config_json: string // JSON 文字列: LmgwProviderConfig をシリアライズしたもの
}

/** GET /v1/lmgw/manage/providers のレスポンス全体 */
export interface GetLmgwProvidersRes {
  providers: ManageLmgwProviderRes[]
}

/**
 * POST /v1/lmgw/manage/providers のリクエスト内の1プロバイダー。
 * config_json は LmgwProviderConfig を JSON 文字列にシリアライズして送信する。
 */
export interface ManageLmgwProviderReq {
  provider_name: string
  config_json: string
}

/** POST /v1/lmgw/manage/providers のリクエスト全体 */
export interface SaveLmgwProvidersReq {
  providers: ManageLmgwProviderReq[]
}

/** POST /v1/lmgw/manage/providers のレスポンス */
export interface SaveLmgwProvidersRes {
  success: boolean
}

// ============================================================
// フロントエンド内部で使用する管理用状態型
// ============================================================

/**
 * フロントエンド内で編集対象として保持するプロバイダーエントリ。
 * バックエンドから取得した ManageLmgwProviderRes を、
 * config_json をパースして扱いやすい形に変換したもの。
 */
export interface LmgwProviderEntry {
  provider_name: string
  config: LmgwProviderConfig
}

/**
 * サポートするプロバイダーの定義。
 * UIで選択可能なプロバイダー一覧を管理するための定数定義に使用。
 */
export interface SupportedProvider {
  name: string       // Bifrostが認識するプロバイダー名 (例: "openai")
  label: string      // UIに表示する名前 (例: "OpenAI")
  icon: string       // Quasar アイコン名
  models: string[]   // そのプロバイダーが提供するモデル名の一覧
}
