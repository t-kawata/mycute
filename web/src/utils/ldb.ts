import { LocalStorage, WebStorageGetMethodReturnType } from 'quasar'

export const KEYS = {
  T: 'T',
  L: 'L',
  V: 'V',
  FS: 'FS'
}

export const set = (key: string, value: any) => { LocalStorage.set(key, value) }

export const get = <T extends WebStorageGetMethodReturnType>(key: string): T | null => { return LocalStorage.getItem<T>(key) }

export const del = (key: string) => { LocalStorage.removeItem(key) }
