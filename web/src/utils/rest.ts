import { API_BASE_URL } from 'src/configs/settings'
import { del, get, post, put, type ApiResponse } from 'src/utils/hc'
import { PATH_MYCUTE_WS_STATUS, PATH_MYCUTE_LANG, PATH_OWNER_ACTIVATE, PATH_OWNER_STATUS, PATH_OWNER_DEACTIVATE, PATH_IDENTITIES_PUBKEY, PATH_LMGW_OPENAI_V1 } from 'src/consts/generated_constants'
import { t } from 'src/utils/some'
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
    type UnregisterCaTokenRes,
    type LicenseSummary,
    type ListLicensesRes,
    type RegisterLicenseRes,
    type UnregisterLicenseRes,
    type VerifyLicenseRes,
    type GenLicenseRes,
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
        // GET_LLMS / SET_LLMS は LMGW 移行に伴い廃止済み
        VERIFY_CA_TOKEN: '/v1/mycute/catoken/verify',
        LICENSE_LIST: '/v1/mycute/license/list',
        LICENSE_REGISTER: '/v1/mycute/license/register',
        LICENSE_UNREGISTER: '/v1/mycute/license/unregister',
        LICENSE_VERIFY: '/v1/mycute/license/verify',
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
        UNREGISTER: '/v1/ca/token/unregister',
        GEN_LICENSE: '/v1/ca/genlicense',
    },
    STT: {
        HISTORY: '/v1/stt/history',
    }
}

// LlmEndpoint は LMGW 移行に伴い廃止済み
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

// getMycuteLlms / setMycuteLlms は LMGW 移行に伴い廃止済み

// STT 履歴を全件取得する
export const getSttHistory = async (): Promise<{ id: number; text: string; created_at: string }[]> => {
    const { body, code, err } = await get(`${API_BASE_URL}${REST_EP.STT.HISTORY}`)
    if (err !== '' || code !== 200 || body === '') { return [] }
    try {
        const { histories } = JSON.parse(body) as { histories: { id: number; text: string; created_at: string }[] }
        return histories || []
    } catch (e) { return [] }
}

// STT 履歴を全件削除する
export const clearSttHistory = async (): Promise<boolean> => {
    const { code, err } = await del(`${API_BASE_URL}${REST_EP.STT.HISTORY}`)
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
export const genCaToken = async (pubkeyHex: string, expireHours: number, permissions?: any): Promise<string | null> => {
    const payload: Record<string, unknown> = {
        pubkey_hex: pubkeyHex,
        expire_hours: expireHours
    }
    if (permissions) {
        payload.permissions = permissions
    }
    const { body, code, err } = await post(`${API_BASE_URL}${REST_EP.OWNER.GEN_CA_TOKEN}`, payload)
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

// ============================================================
// ライセンス管理 API
// ============================================================

/**
 * 登録済みライセンス一覧を取得する。
 * 認証不要のパブリック API。
 */
export const listLicenses = async (): Promise<LicenseSummary[]> => {
    const { body, code, err } = await get(`${API_BASE_URL}${REST_EP.MYCUTE.LICENSE_LIST}`)
    if (err !== '' || code !== 200 || !body) { return [] }
    try {
        const res = JSON.parse(body) as ListLicensesRes
        return res.licenses
    } catch { return [] }
}

/**
 * ライセンスを自身に登録する。
 * USR ロールのみ使用可能。
 */
export const registerLicense = async (authToken: string, license: string): Promise<RegisterLicenseRes | null> => {
    const { body, code, err } = await post(
        `${API_BASE_URL}${REST_EP.MYCUTE.LICENSE_REGISTER}`,
        { license },
        { headers: { Authorization: `Bearer ${authToken}` } }
    )
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        return JSON.parse(body) as RegisterLicenseRes
    } catch { return null }
}

/**
 * 指定した ID のライセンスを削除する。
 * USR ロールのみ使用可能。
 */
export const unregisterLicense = async (authToken: string, id: string): Promise<UnregisterLicenseRes | null> => {
    const { body, code, err } = await post(
        `${API_BASE_URL}${REST_EP.MYCUTE.LICENSE_UNREGISTER}`,
        { id },
        { headers: { Authorization: `Bearer ${authToken}` } }
    )
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        return JSON.parse(body) as UnregisterLicenseRes
    } catch { return null }
}

/**
 * ライセンスの妥当性を検証する（登録不要）。
 * 認証不要のパブリック API。
 */
export const verifyLicense = async (license: string): Promise<VerifyLicenseRes | null> => {
    const { body, code, err } = await post(`${API_BASE_URL}${REST_EP.MYCUTE.LICENSE_VERIFY}`, { license })
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        return JSON.parse(body) as VerifyLicenseRes
    } catch { return null }
}

/**
 * CA としてユーザーにライセンスを発行する。
 * JWT 認証（USR ロール）が必要。
 */
export const genLicense = async (
    authToken: string,
    pubkeyHex: string,
    expireHours: number,
    permissions?: any
): Promise<GenLicenseRes | null> => {
    const payload: Record<string, unknown> = {
        pubkey_hex: pubkeyHex,
        expire_hours: expireHours,
    }
    if (permissions) {
        payload.permissions = permissions
    }
    const { body, code, err } = await post(`${API_BASE_URL}${REST_EP.CA.GEN_LICENSE}`, payload, {
        headers: { 'Authorization': `Bearer ${authToken}` }
    })
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        return JSON.parse(body) as GenLicenseRes
    } catch { return null }
}

// ============================================================
// LMGW プロバイダー管理 API
// ============================================================
import type { GetLmgwProvidersRes, SaveLmgwProvidersReq, SaveLmgwProvidersRes } from 'src/models/lmgw'

const EP_LMGW_PROVIDERS = '/v1/lmgw/manage/providers'

/**
 * DBに保存されているLMGWプロバイダー設定の一覧を取得する。
 * APIキーは暗号化された状態で返却される。
 */
export const getLmgwProviders = async (authToken: string): Promise<GetLmgwProvidersRes | null> => {
    const { body, code, err } = await get(`${API_BASE_URL}${EP_LMGW_PROVIDERS}`, {
        headers: { 'Authorization': `Bearer ${authToken}` }
    })
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        return JSON.parse(body) as GetLmgwProvidersRes
    } catch { return null }
}

/**
 * LMGWプロバイダー設定を保存し、Bifrostへ同期する。
 * is_new=true のキーはバックエンドで暗号化される。
 * is_new=false のキーは暗号化済みのままDBに保存される。
 */
export const saveLmgwProviders = async (
    authToken: string,
    req: SaveLmgwProvidersReq
): Promise<SaveLmgwProvidersRes | null> => {
    const { body, code, err } = await post(`${API_BASE_URL}${EP_LMGW_PROVIDERS}`, req, {
        headers: { 'Authorization': `Bearer ${authToken}` }
    })
    if (err !== '' || code !== 200 || !body) { return null }
    try {
        return JSON.parse(body) as SaveLmgwProvidersRes
    } catch { return null }
}

export interface LmgwChatCompletionsRes {
    choices: Array<{
        message: {
            content: string
        }
    }>
}

export const EP_LMGW_CHAT_COMPLETIONS = `${PATH_LMGW_OPENAI_V1}/chat/completions`

/**
 * Bifrost透過プロキシを経由して、LLMプロバイダーへチャットコンプリーションのテストリクエストを送信する。
 */
export const testLlmCommunication = async (
    authToken: string,
    model: string,
    message: string
): Promise<{ success: boolean; content?: string; error?: string }> => {
    const payload = {
        model,
        messages: [{ role: 'user', content: message }]
    }
    
    const { body, code, err } = await post(`${API_BASE_URL}${EP_LMGW_CHAT_COMPLETIONS}`, payload, {
        headers: { 'Authorization': `Bearer ${authToken}` }
    })
    
    if (err !== '' || code !== 200 || !body) {
        let errMsg = err || t('app.llm.testCommFail')
        try {
            const parsed = JSON.parse(body || '{}')
            if (parsed.error?.message) errMsg = parsed.error.message
        } catch { /* ignore */ }
        return { success: false, error: errMsg }
    }
    
    try {
        const parsed = JSON.parse(body) as LmgwChatCompletionsRes
        if (parsed.choices && parsed.choices.length > 0) {
            return { success: true, content: parsed.choices[0]!.message.content }
        }
        return { success: false, error: t('app.llm.testInvalidFormat') }
    } catch {
        return { success: false, error: t('app.llm.testParseFail') }
    }
}
