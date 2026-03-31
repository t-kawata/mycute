import { decodeJwt } from 'jose'
import { get, KEYS } from 'src/utils/ldb'
import { getVdrToken } from 'src/utils/rest'

/**
 * ローカルストレージ内の VDR-KEY (KEYS.V) に基づいて、VDR トークンと ID をストアに初期化します。
 * この関数は VDR 基盤の初期化のみを担当し、ユーザーセッションの復元は行いません。
 * 
 * @param store useMainStore() のインスタンス
 * @returns VDR コンテキストの初期化に成功した場合は true
 */
export async function initVdrContext(store: any): Promise<boolean> {
  try {
    const vdrKey = get<string>(KEYS.V)
    if (!vdrKey) return false

    const vdrToken = await getVdrToken(vdrKey)
    if (!vdrToken) return false

    const vPayload = decodeJwt(vdrToken)
    store.setVdrToken(vdrToken)
    store.setApxID(Number(vPayload.apx_id))
    store.setVdrID(Number(vPayload.usr_id))

    return true
  } catch (e) {
    console.error('Failed to initialize VDR context:', e)
    return false
  }
}
