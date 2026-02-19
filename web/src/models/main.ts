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

export interface BadgeCandidate { // Card と多くのフィールドが一致するが、データ型としては分ける
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
  // -------- BadgeCandidate のみのフィールド bgn
  from: number
  fromUsrName: string
  to: number
  toUsrName: string
  badgeID: number
  badgeName: string
  message: string
  // -------- BadgeCandidate のみのフィールド end
}

// BadgeCandidate と多くのフィールドが一致するが、データ型としては分ける
// 本番実装では UsrBadge と統合する
// GaveBadge は、Friends ページのポイント計算用であり、
// アマギフによるポイント還元済みの授与バッジは含まないことに注意
export interface GaveBadge {
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
  // -------- BadgeCandidate のみのフィールド bgn
  from: number
  fromUsrName: string
  to: number
  toUsrName: string
  badgeID: number
  badgeName: string
  message: string
  // -------- BadgeCandidate のみのフィールド end
}

export interface Badge {
  id: number
  usrID: number
  name: string
  shortName: string // バッジを fe で表示する時の短い名前
  description: string
  apxID: number
  vdrID: number
  simID: number
  createdAt: Date | string
}

export interface UsrBadge {
  badgeID: number
  usrID: number // Badgeを作った法人の UsrID
  from: number
  to: number
  title: string
  message: string
  type: string // 1: 法人による授与, 2: 個人による授与
  apxID: number
  vdrID: number
  simID: number
  createdAt: Date | string
}
