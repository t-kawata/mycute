export const WHO = {
  SYSTEM: 0,
  GENERAL_USER: 1
}

export const SWIPE_DIRECTION = {
  NOPE: 'nope',
  LIKE: 'like'
} as { [k: string]: SwipeDirection }

export type SwipeDirection = 'nope' | 'like'

export interface User {
  id: number
  first_name: string
  last_name: string
  apx_id: number
  vdr_id: number
  type: number // 1: 法人, 2: 個人
  email: string
  exp: number
  is_staff: boolean
}

export interface Card { // CalendarEvent と全てのフィールドが一致するが、データ型としては分ける
  id: number
  img: string
  imgSmall: string
  title: string
  subtitle: string
  description: string
  start: Date | string // start と end は、同一日付でなければならない
  end: Date | string // start と end は、同一日付でなければならない
  hourPrice: number
  requirements: string
  benefits: string
  location: string
  phone: string
  maxBadges: number, // このお仕事でもらえる可能性のあるバッジの最大数
  isFixed: boolean
}

export interface CalendarEvent { // Card と全てのフィールドが一致するが、データ型としては分ける
  id: number
  img: string
  imgSmall: string
  title: string
  subtitle: string
  description: string
  start: Date | string // start と end は、同一日付でなければならない
  end: Date | string // start と end は、同一日付でなければならない
  hourPrice: number
  requirements: string
  benefits: string
  location: string
  phone: string
  maxBadges: number, // このお仕事でもらえる可能性のあるバッジの最大数
  isFixed: boolean
}
