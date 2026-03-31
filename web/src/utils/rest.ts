import { API_BASE_URL } from 'src/configs/settings'
import { get, post, put, type ApiResponse } from 'src/utils/hc'
import { PATH_MYCUTE_WS_STATUS, PATH_MYCUTE_LANG, PATH_MYCUTE_LLMS_GET, PATH_MYCUTE_LLMS_SET, PATH_OWNER_ACTIVATE, PATH_OWNER_STATUS, PATH_OWNER_DEACTIVATE, PATH_IDENTITIES_PUBKEY } from 'src/consts/generated_constants'
import { type CreateUsrReq } from 'src/models/rtreq'
import {
    type CreateBdHashRes,
    type AuthUsrRes,
    type CreateUsrRes,
    type GetVdrTokenRes,
    type DecryptRes,
    type CreateVdrTokenRes,
    type VerifyCaTokenRes,
    type CaStatusRes,
    type RegisterCaTokenRes,
    type UnregisterCaTokenRes
} from 'src/models/rtres'

// ============================================================
// REST エンドポイント定数
// ============================================================
export const REST_EP = {
    BDS: {
        CREATE: '/v1/bds/create'
    },
    CRYPTO: {
        VDR: '/v1/crypto/vdr',
        DEC: '/v1/crypto/dec'
    },
    USRS: {
        AUTH: '/v1/usrs/auth',
        CREATE: '/v1/usrs'
    },
    MYCUTE: {
        LANG: PATH_MYCUTE_LANG,
        WS_STATUS: PATH_MYCUTE_WS_STATUS,
        GET_LLMS: PATH_MYCUTE_LLMS_GET,
        SET_LLMS: PATH_MYCUTE_LLMS_SET,
        VERIFY_CA_TOKEN: '/v1/mycute/catoken/verify',
    },
    NODE: {
        PUBKEY: PATH_IDENTITIES_PUBKEY
    },
    OWNER: {
        ACTIVATE: PATH_OWNER_ACTIVATE,
        STATUS: PATH_OWNER_STATUS,
        DEACTIVATE: PATH_OWNER_DEACTIVATE,
        GEN_CA_TOKEN: '/v1/owner/gencatoken'
    },
    CA: {
        STATUS_LOCAL: '/v1/ca/status/local',
        REGISTER: '/v1/ca/token/register',
        UNREGISTER: '/v1/ca/token/unregister'
    }
}

// ============================================================
// LLM エンドポイント型定義（バックエンドの LlmEndpoint と対称）
// ============================================================
export interface LlmEndpoint {
    name: string
    base_url: string
    api_key?: string | null
    model: string
}

// ============================================================
// 内部ユーティリティ（非公開）
// ============================================================

const cryptoVdr = async (key: string): Promise<ApiResponse> => {
    return await get(`${API_BASE_URL}${REST_EP.CRYPTO.VDR}/${key}`)
}

const cryptoDec = async (text: string): Promise<ApiResponse> => {
    return await get(`${API_BASE_URL}${REST_EP.CRYPTO.DEC}?text=${text}`)
}

// ============================================================
// 公開 API 関数
// ============================================================

export const getWsStatus = async (): Promise<{ is_cl_connected: boolean, active_clients: number } | null> => {
    const { body, code, err } = await get(`${API_BASE_URL}${REST_EP.MYCUTE.WS_STATUS}`)
    if (err !== '' || code !== 200 || body === '') { return null }
    try {
        return JSON.parse(body)
    } catch (e) { return null }
}

export const getVdrToken = async (key: string): Promise<string> => {
    const { body, code, err } = await cryptoVdr(key)
    if (err !== '' || code !== 200 || body === '') { return '' }
    let bodyObj = {}
    try {
        bodyObj = JSON.parse(body)
    } catch (e) { return '' }
    // Rust側の型: GetVdrTokenRes { key, value }
    const { value } = bodyObj as GetVdrTokenRes
    if (!value) { return '' }
    const { body: decBody, code: decCode, err: decErr } = await cryptoDec(value)
    if (decErr !== '' || decCode !== 200 || decBody === '') { return '' }
    let decBodyObj = {}
    try {
        decBodyObj = JSON.parse(decBody)
    } catch (e) { return '' }
    // Rust側の型: DecryptRes { data } (旧 CryptoDecRes)
    const { data: token } = decBodyObj as DecryptRes
    if (!token) { return '' }
    return token
}

export const usrsAuth = async (apxID: number, vdrID: number, email: string, password: string, expire: number): Promise<ApiResponse> => {
    return await get(`${API_BASE_URL}${REST_EP.USRS.AUTH}/${apxID}/${vdrID}?email=${email}&password=${password}&expire=${expire}`)
}

export const createBD = async (bd: string): Promise<string> => {
    const { body, code, err } = await get(`${API_BASE_URL}${REST_EP.BDS.CREATE}?bd=${bd}`)
    if (err !== '' || code !== 200 || body === '') { return '' }
    let bodyObj = {}
    try {
        bodyObj = JSON.parse(body)
    } catch (e) { return '' }
    // Rust側の型: CreateBdHashRes { hash }
    const { hash } = bodyObj as CreateBdHashRes
    return hash || ''
}

export const authWithBD = async (bdPassphrase: string, expire: number): Promise<string> => {
    // BDでの認証時も、形として email/password は必要 (dummyでOK)
    const { body, code, err } = await get(`${API_BASE_URL}${REST_EP.USRS.AUTH}/0/0?email=dummy@dummy.com&password=dummy&expire=${expire}`, {
        headers: { 'X-BD': bdPassphrase }
    })
    if (err !== '' || code !== 200 || body === '') { return '' }
    let bodyObj = {}
    try {
        bodyObj = JSON.parse(body)
    } catch (e) { return '' }
    // Rust側の型: AuthUsrRes { token }
    const { token } = bodyObj as AuthUsrRes
    return token || ''
}

export const createUser = async (token: string, payload: CreateUsrReq): Promise<number> => {
    const { body, code, err } = await post(`${API_BASE_URL}${REST_EP.USRS.CREATE}`, payload, {
        headers: { 'Authorization': `Bearer ${token}` }
    })
    if (err !== '' || code !== 200 || body === '') { return 0 }
    let bodyObj = {}
    try {
        bodyObj = JSON.parse(body)
    } catch (e) { return 0 }
    // Rust側の型: CreateUsrRes { id }
    const { id } = bodyObj as CreateUsrRes
    return id || 0
}

export const createVdr100YearToken = async (token: string, key: string, apxId: number, vdrId: number): Promise<string> => {
    const { body, code, err } = await put(`${API_BASE_URL}${REST_EP.CRYPTO.VDR}/${key}/${apxId}/${vdrId}`, {}, {
        headers: { 'Authorization': `Bearer ${token}` }
    })
    if (err !== '' || code !== 200 || body === '') { return '' }
    let bodyObj = {}
    try {
        bodyObj = JSON.parse(body)
    } catch (e) { return '' }
    // Rust側の型: CreateVdrTokenRes { key, value }
    const { value } = bodyObj as CreateVdrTokenRes
    return value || ''
}

// 言語を設定する
export const setMycuteLang = async (lang: string): Promise<boolean> => {
    const { code, err } = await post(`${API_BASE_URL}${REST_EP.MYCUTE.LANG}`, { locale: lang })
    if (err !== '' || code !== 200) { return false }
    return true
}

// バックエンドから現在の LLM 設定一覧を取得する
export const getMycuteLlms = async (): Promise<{ llms: LlmEndpoint[] } | null> => {
    const { body, code, err } = await get(`${API_BASE_URL}${REST_EP.MYCUTE.GET_LLMS}`)
    if (err !== '' || code !== 200 || body === '') { return null }
    try {
        return JSON.parse(body)
    } catch (e) { return null }
}

// LLM 設定一覧をバックエンドへ送信して永続保存する
export const setMycuteLlms = async (llms: LlmEndpoint[]): Promise<boolean> => {
    const { code, err } = await post(`${API_BASE_URL}${REST_EP.MYCUTE.SET_LLMS}`, { llms })
    if (err !== '' || code !== 200) { return false }
    return true
}

// オーナーモードを有効化する
export const activateOwner = async (passphrase: string): Promise<boolean> => {
    const { code, err } = await post(`${API_BASE_URL}${REST_EP.OWNER.ACTIVATE}`, { passphrase })
    // エラーが空文字でなくとも 200 なら成功だが、バックエンドの実装に合わせる。
    if (err !== '' || code !== 200) { return false }
    return true
}

// オーナーモードを無効化する
export const deactivateOwner = async (): Promise<boolean> => {
    const { code, err } = await post(`${API_BASE_URL}${REST_EP.OWNER.DEACTIVATE}`, {})
    if (err !== '' || code !== 200) { return false }
    return true
}

// オーナーモードのステータスを取得する
export const getOwnerStatus = async (): Promise<boolean> => {
    const { body, code, err } = await get(`${API_BASE_URL}${REST_EP.OWNER.STATUS}`)
    if (err !== '' || code !== 200 || !body) { return false }
    try {
        const res = JSON.parse(body)
        return !!res.is_active
    } catch { return false }
}

// 自身の公開鍵（My Public Key）を取得する
export const getMyPubKey = async (): Promise<string> => {
    const { body, code, err } = await get(`${API_BASE_URL}${REST_EP.NODE.PUBKEY}`)
    if (err !== '' || code !== 200 || !body) { return '' }
    try {
        const res = JSON.parse(body)
        return res.public_key || ''
    } catch { return '' }
}

/**
 * CAトークンを生成する
 */
export const genCaToken = async (pubkeyHex: string, expireHours: number): Promise<string | null> => {
    const { body, code, err } = await post(`${API_BASE_URL}${REST_EP.OWNER.GEN_CA_TOKEN}`, {
        pubkey_hex: pubkeyHex,
        expire_hours: expireHours
    })
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        const res = JSON.parse(body)
        return res.ca_token || null
    } catch { return null }
}

/**
 * CAトークンを検証する
 */
export const verifyCaToken = async (caToken: string): Promise<VerifyCaTokenRes | null> => {
    const { body, code, err } = await post(`${API_BASE_URL}${REST_EP.MYCUTE.VERIFY_CA_TOKEN}`, {
        ca_token: caToken
    })
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        return JSON.parse(body) as VerifyCaTokenRes
    } catch { return null }
}

/**
 * 自身のCAステータスを取得する
 */
export const getCaStatus = async (): Promise<string | null> => {
    const { body, code, err } = await get(`${API_BASE_URL}${REST_EP.CA.STATUS_LOCAL}`)
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        const res = JSON.parse(body) as CaStatusRes
        return res.ca_token ?? null
    } catch { return null }
}

/**
 * CAトークンを登録する
 */
export const registerCaToken = async (authToken: string, caToken: string): Promise<RegisterCaTokenRes | null> => {
    const { body, code, err } = await post(`${API_BASE_URL}${REST_EP.CA.REGISTER}`, {
        ca_token: caToken
    }, {
        headers: { 'Authorization': `Bearer ${authToken}` }
    })
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        return JSON.parse(body) as RegisterCaTokenRes
    } catch { return null }
}

/**
 * CAトークンを削除（登録解除）する
 */
export const unregisterCaToken = async (authToken: string): Promise<UnregisterCaTokenRes | null> => {
    const { body, code, err } = await post(`${API_BASE_URL}${REST_EP.CA.UNREGISTER}`, {}, {
        headers: { 'Authorization': `Bearer ${authToken}` }
    })
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        return JSON.parse(body) as UnregisterCaTokenRes
    } catch { return null }
}

