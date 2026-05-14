import { Notify } from 'quasar'

/**
 * 共通デザインの通知を表示します。
 * @param message 表示するメッセージ
 * @param timeout タイムアウト時間（ミリ秒）デフォルト: 2000
 * @param color 背景色（quasarのカラー名またはカスタムカラー）デフォルト: 'app'
 * @param icon アイコン名 デフォルト: 'check'
 */
export function showNotify(
    message: string, 
    timeout: number = 2000, 
    color: string = 'app', 
    icon: string = 'check'
): void {
    const dismiss = Notify.create({
        position: 'top',
        message,
        timeout,
        color,
        textColor: 'white',
        icon,
        classes: 'text-bold __mycute-snackbar',
        attrs: {
            role: 'alert',
            onclick: () => dismiss()
        }
    })
}

/**
 * ネガティブカラーの警告通知を表示
 * @param message 表示するメッセージ
 * @param timeout タイムアウト時間（ミリ秒）デフォルト: 5000
 */
export function showWarn(message: string, timeout: number = 5000): void {
    showNotify(message, timeout, 'negative', 'warning')
}

/**
 * infoカラーの情報通知を表示
 * @param message 表示するメッセージ
 * @param timeout タイムアウト時間（ミリ秒）デフォルト: 3000
 */
export function showInfo(message: string, timeout: number = 3000): void {
    showNotify(message, timeout, 'info', 'info')
}
