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
}
