import { Notify } from 'quasar'

/**
 * ネガティブカラーの警告通知を表示
 * @param message 表示するメッセージ
 * @param timeout タイムアウト時間（ミリ秒）デフォルト: 5000
 */
export function showWarn(message: string, timeout: number = 5000): void {
    const dismiss = Notify.create({
        type: 'warning',
        color: 'negative',
        textColor: 'white',
        iconColor: 'white',
        message,
        position: 'bottom',
        timeout,
        html: true,
        attrs: {
            role: 'alert',
            onclick: () => dismiss()
        },
        classes: 'cursor-pointer full-width'
    })
}

/**
 * infoカラーの情報通知を表示
 * @param message 表示するメッセージ
 * @param timeout タイムアウト時間（ミリ秒）デフォルト: 3000
 */
export function showInfo(message: string, timeout: number = 3000): void {
    const dismiss = Notify.create({
        type: 'info',
        color: 'info',
        textColor: 'white',
        iconColor: 'white',
        message,
        position: 'bottom',
        timeout,
        html: true,
        attrs: {
            role: 'alert',
            onclick: () => dismiss()
        },
        classes: 'cursor-pointer full-width'
    })
}
