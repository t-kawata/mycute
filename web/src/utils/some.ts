import { Ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { i18n } from 'src/i18n/instance'
import { z, ZodRawShape } from 'zod'
import { useMainStore } from 'src/stores/main-store'
import { set, KEYS } from 'src/utils/ldb'
import { setMycuteLang } from 'src/utils/rest'

export const LANG = {
  EN: { SHORT: 'en', LONG: 'en-US' },
  JA: { SHORT: 'ja', LONG: 'ja-JP' }
}

export function useLangSetter() {

  const setLangEN = async (): Promise<boolean> => {
    const mainStore = useMainStore()
    if (mainStore.lang === LANG.EN.LONG) return true // 既に設定済みの場合はスキップ
    // 状態をローカルに反映
    mainStore.setLang(LANG.EN.LONG)
    set(KEYS.L, LANG.EN.LONG)
    // @ts-ignore
    i18n.global.locale.value = LANG.EN.LONG
    // バックエンドへ通知
    const ok = await setMycuteLang(LANG.EN.SHORT)
    if (!ok) console.error('Failed to sync EN lang change to backend.')
    return ok
  }

  const setLangJA = async (): Promise<boolean> => {
    const mainStore = useMainStore()
    if (mainStore.lang === LANG.JA.LONG) return true // 既に設定済みの場合はスキップ
    // 状態をローカルに反映
    mainStore.setLang(LANG.JA.LONG)
    set(KEYS.L, LANG.JA.LONG)
    // @ts-ignore
    i18n.global.locale.value = LANG.JA.LONG
    // バックエンドへ通知
    const ok = await setMycuteLang(LANG.JA.SHORT)
    if (!ok) console.error('Failed to sync JA lang change to backend.')
    return ok
  }

  return { setLangEN, setLangJA }
}

// @ts-ignore
export const t = (key: string, values?: Record<string, unknown>) => i18n.global.t(key, values)

export const validate = <T extends ZodRawShape>(schema: z.ZodObject<T>, data: Object, errors: Ref<T | {}>): boolean => {
  errors.value = {}
  const result = schema.safeParse(data)
  if (result.success) { errors.value = {}; return true; }
  for (let i in result.error.issues) {
    const err = result.error.issues[i]  // 型は自動的に推論される
    for (let k in data) {
      // @ts-ignore
      if (err.path[0] === k && !errors.value[k]) {
        // @ts-ignore
        errors.value[k] = err.message
      }
    }
  }
  return false
}

export const getToken = async (): Promise<string> => {
  const mainStore = useMainStore()
  const maxRetry = 30 // 3秒以内に取得できなければ 403 で終了する
  let token = mainStore.token
  if (!token) {
    for (let i = 0; i < maxRetry; i++) {
      await sleep(100)
      token = mainStore.token
      if (token) break
    }
    if (!token) return ''
  }
  return token
}

export const sleep = async (ms: number): Promise<void> => await new Promise(resolve => setTimeout(resolve, ms))

export const getDateStr = (ms: number): string => {
  const date = new Date(ms)
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  const hours = String(date.getHours()).padStart(2, '0')
  const minutes = String(date.getMinutes()).padStart(2, '0')
  return `${year}/${month}/${day} ${hours}:${minutes}`
}

export const calcHourlyWageStr = (start: Date | string, end: Date | string, hourPrice: number, isEn: boolean): string => {
  const wage = calcHourlyWage(start, end, hourPrice);
  const formatted = Math.round(wage).toLocaleString();
  if (isEn) {
    return `¥ ${formatted}`;
  } else {
    return `${formatted} 円`;
  }
}

export const calcHourlyWage = (start: Date | string, end: Date | string, hourPrice: number): number => {
  const s = new Date(start);
  const e = new Date(end);
  const diffMs = e.getTime() - s.getTime();
  const diffHours = diffMs / (1000 * 60 * 60);
  return diffHours * hourPrice;
}

export const numberToYenStr = (num: number): string => {
  if (num >= 10000) {
    const manValue = num / 10000;
    // 小数点以下がない場合
    if (Number.isInteger(manValue)) {
      return `${manValue}万円`;
    }
    // 小数点以下がある場合は小数第一位まで
    return `${manValue.toFixed(1)}万円`;
  }
  // 1万円未満の場合は従来通り
  const formatted = num.toLocaleString();
  return `¥${formatted}`;
}


export const numberToPointStr = (num: number): string => {
  if (num >= 10000) {
    const manValue = num / 10000;
    // 小数点以下がない場合
    if (Number.isInteger(manValue)) {
      return `${manValue}万`;
    }
    // 小数点以下がある場合は小数第一位まで
    return `${manValue.toFixed(1)}万`;
  }
  // 1万未満の場合は従来通り
  const formatted = num.toLocaleString();
  return `${formatted}`;
}

export const formatDateRange = (start: Date | string, end: Date | string, isEn: boolean): string => {
  const s = new Date(start);
  const e = new Date(end);
  const pad = (n: number) => n.toString().padStart(2, '0');
  const isSameDate =
    s.getMonth() === e.getMonth() &&
    s.getDate() === e.getDate();
  if (!isEn) {
    // 日本語
    if (isSameDate) {
      return `${s.getMonth() + 1}月${s.getDate()}日 ${pad(s.getHours())}:${pad(s.getMinutes())} ~ ${pad(e.getHours())}:${pad(e.getMinutes())}`;
    } else {
      return `${s.getMonth() + 1}月${s.getDate()}日 ${pad(s.getHours())}:${pad(s.getMinutes())} ~ ${e.getMonth() + 1}月${e.getDate()}日 ${pad(e.getHours())}:${pad(e.getMinutes())}`;
    }
  } else {
    // 英語
    const months = [
      'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'
    ];
    if (isSameDate) {
      return `${months[s.getMonth()]} ${s.getDate()} ${pad(s.getHours())}:${pad(s.getMinutes())} ~ ${pad(e.getHours())}:${pad(e.getMinutes())}`;
    } else {
      return `${months[s.getMonth()]} ${s.getDate()} ${pad(s.getHours())}:${pad(s.getMinutes())} to ${months[e.getMonth()]} ${e.getDate()} ${pad(e.getHours())}:${pad(e.getMinutes())}`;
    }
  }
}

// モバイルブラウザかどうかを判定
export const isMobileBrowser = (): boolean => {
  return useMainStore().platform.isMobileBrowser;
}

// Tauri環境（デスクトップ/モバイル問わず）にいるかどうかを判定
export const isTauri = (): boolean => {
  return useMainStore().platform.isTauri;
}

// 具体的に「Tauriのデスクトップアプリ」として動いているかを判定
export const isTauriDesktop = (): boolean => {
  return useMainStore().platform.isTauriDesktop;
}

// Windows環境かどうかを判定
export const isWindows = (): boolean => {
  return useMainStore().platform.isWindows;
}

// Mac環境かどうかを判定
export const isMac = (): boolean => {
  return useMainStore().platform.isMac;
}

// WindowsのTauri環境かどうかを判定
export const isTauriWindows = (): boolean => {
  return useMainStore().platform.isTauriWindows;
}

// MacのTauri環境かどうかを判定
export const isTauriMac = (): boolean => {
  return useMainStore().platform.isTauriMac;
}

// Tauriのモバイルアプリ（iOS/Android）として動いているかを判定
export const isTauriMobile = (): boolean => {
  return useMainStore().platform.isTauriMobile;
}
