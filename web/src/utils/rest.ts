import { API_BASE_URL } from 'src/configs/settings'
import { get, post, patch, put, del, type ApiResponse } from 'src/utils/hc'
import { type CreateUsrReq } from 'src/models/rtreq'
import {
    type CreateBdHashRes,
    type AuthUsrRes,
    type CreateUsrRes,
    type GetVdrTokenRes,
    type DecryptRes,
    type CreateVdrTokenRes
} from 'src/models/rtres'

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
    }
}

const cryptoVdr = async (key: string): Promise<ApiResponse> => {
    return await get(`${API_BASE_URL}${REST_EP.CRYPTO.VDR}/${key}`)
}

const cryptoDec = async (text: string): Promise<ApiResponse> => {
    return await get(`${API_BASE_URL}${REST_EP.CRYPTO.DEC}?text=${text}`)
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