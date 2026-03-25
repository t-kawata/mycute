import { invoke } from '@tauri-apps/api/core'
import { sleep } from 'src/utils/some'
import { getWsStatus } from 'src/utils/rest'

/**
 * バックエンドサーバーが準備完了（Healthy）になるまで待機します。
 * @param maxRetries 最大リトライ回数（デフォルト 60回 = 30秒）
 * @param interval 待機間隔（ms, デフォルト 500ms）
 * @returns 準備完了なら true, タイムアウトなら false
 */
export async function waitForServer(maxRetries = 60, interval = 500): Promise<boolean> {
    console.log(`Waiting for server to be healthy... (max ${maxRetries} retries)`)

    for (let i = 0; i < maxRetries; i++) {
        try {
            // Tauri コマンド check_server_health を使用
            const isHealthy = await invoke<boolean>('check_server_health')
            if (isHealthy) {
                console.log('Server is healthy.')
                return true
            }
        } catch (e) {
            console.warn(`Server health check failed (attempt ${i + 1}):`, e)
        }
        await sleep(interval)
    }

    console.error('Server health check timed out.')
    return false
}

/**
 * CL と RT 間の WebSocket ハンドシェイクが完了するまで待機します。
 * @param maxRetries 最大リトライ回数（デフォルト 60回 = 30秒）
 * @param interval 待機間隔（ms, デフォルト 500ms）
 * @returns 完了なら true, タイムアウトなら false
 */
export async function waitForWs(maxRetries = 60, interval = 500): Promise<boolean> {
    console.log(`Waiting for WebSocket handshake... (max ${maxRetries} retries)`)

    for (let i = 0; i < maxRetries; i++) {
        try {
            const status = await getWsStatus()
            if (status && status.is_cl_connected) {
                console.log('WebSocket handshake confirmed.')
                return true
            }
        } catch (e) {
            console.warn(`WS Status check failed (attempt ${i + 1}):`, e)
        }
        await sleep(interval)
    }

    console.error('WebSocket handshake timed out.')
    return false
}
