/*
 * バックエンドのレスポンスモデル
 * src/mode/rt/rtres/*.rs と同期しています
 */

export interface CreateBdHashRes {
    hash: string
}

export interface AuthUsrRes {
    token: string
}

export interface CreateUsrRes {
    id: number
}

export interface DecryptRes {
    data: string
}

export interface CreateVdrTokenRes {
    key: string
    value: string
}

export interface GetVdrTokenRes {
    key: string
    value: string
}

export interface VerifyCaTokenRes {
    success: boolean
    message: string
    ca_pubkey?: string
    expire_at?: number
    permissions?: any
}

export interface CaStatusRes {
    ca_token: string | null
}

export interface RegisterCaTokenRes {
    success: boolean
    message: string
    ca_token?: string
    permissions?: any
}

export interface UnregisterCaTokenRes {
    success: boolean
    message: string
}

// ============================================================
// ライセンス管理（license_bl.rs と同期）
// ============================================================

/** 1 件のライセンスのパース済みサマリー。一覧・詳細表示に使用する。 */
export interface LicenseSummary {
    /** ライセンスの識別子（SHA-256 先頭 16 文字）*/
    id: string
    /** ライセンスを発行した CA の公開鍵 (Hex) */
    ca_pubkey: string
    /** ライセンスの有効期限（Unix TS ms）*/
    expire_at: number
    /** 権限内容 (JSON オブジェクト) */
    permissions: Record<string, unknown>
    /** ライセンスが現在有効かどうか */
    is_valid: boolean
    /** 元のライセンス文字列（登録・削除に使用）*/
    raw: string
}

/** GET /v1/mycute/license/list レスポンス */
export interface ListLicensesRes {
    licenses: LicenseSummary[]
}

/** POST /v1/mycute/license/register レスポンス */
export interface RegisterLicenseRes {
    success: boolean
    message: string
    summary?: LicenseSummary
}

/** POST /v1/mycute/license/unregister レスポンス */
export interface UnregisterLicenseRes {
    success: boolean
    message: string
}

/** POST /v1/mycute/license/verify レスポンス */
export interface VerifyLicenseRes {
    success: boolean
    message: string
    summary?: LicenseSummary
}

/** POST /v1/ca/genlicense レスポンス */
export interface GenLicenseRes {
    /** 発行されたライセンス文字列 (base64(payload).sig_hex) */
    license: string
}
