import { defineStore, acceptHMRUpdate } from 'pinia'
import { SAMPLE_CARDS, SAMPLE_EXTRA_CARDS, SAMPLE_EVENTS, SAMPLE_BADGES, SAMPLE_USR_BADGES, SAMPLE_BADGE_CANDIDATES, SAMPLE_GAVE_BADGES } from 'src/consts/data';
import { CalendarEvent, Card, User, Badge, UsrBadge, BadgeCandidate, GaveBadge } from 'src/models/main'
import { MycuteAppConfig } from 'src/models/app'
import { isDateRangeOverlap, normalizeEventsToFuture } from 'src/utils/common';
import { calcHourlyWage, LANG } from 'src/utils/some'
import TAB, { type TabType } from 'src/enums/TAB'
import { get, KEYS, set } from 'src/utils/ldb';
import { ENGINE_OS } from 'src/consts/generated_constants';
import { type LlmEndpoint } from 'src/utils/rest';

export interface PlatformState {
  isMobileBrowser: boolean;
  isTauri: boolean;
  isTauriDesktop: boolean;
  isWindows: boolean;
  isMac: boolean;
  isTauriWindows: boolean;
  isTauriMac: boolean;
  isTauriMobile: boolean;
}

const POINT_PER_GAVE_BADGE = 13795
const dummyEvents = normalizeEventsToFuture(SAMPLE_EVENTS, [])
const dummyCards = normalizeEventsToFuture(SAMPLE_CARDS, dummyEvents)
const dummyExtraCards = normalizeEventsToFuture(SAMPLE_EXTRA_CARDS, [...dummyCards, ...dummyEvents])

const detector = (): PlatformState => {
  if (typeof window === 'undefined') {
    return {
      isMobileBrowser: false, isTauri: false, isTauriDesktop: false,
      isWindows: false, isMac: false, isTauriWindows: false,
      isTauriMac: false, isTauriMobile: false
    };
  }

  // isMobileBrowser logic
  let isMobileBrowser = false;
  const ua = navigator.userAgent;
  if ((navigator as any).userAgentData?.mobile !== undefined) {
    isMobileBrowser = (navigator as any).userAgentData.mobile;
  } else {
    const isMobi = /Mobi|Android/i.test(ua);
    const isIPad = /Macintosh/i.test(ua) && (navigator.maxTouchPoints > 0);
    isMobileBrowser = isMobi || isIPad;
  }

  // isTauri logic
  const isTauri = Boolean((window as any).__TAURI_INTERNALS__);

  // isWindows logic
  let isWindows = (window as any).__MYCUTE_PLATFORM__ === 'windows';
  if (!isWindows && typeof navigator !== 'undefined') {
    const platform = (navigator as any).userAgentData?.platform || navigator.platform || '';
    isWindows = /Win/i.test(platform) || /Win/i.test(ua);
  }

  // isMac logic
  let isMac = (window as any).__MYCUTE_PLATFORM__ === 'macos';
  if (!isMac && typeof navigator !== 'undefined') {
    const platform = (navigator as any).userAgentData?.platform || navigator.platform || '';
    const isMacPlatform = /Mac/i.test(platform) || /Mac/i.test(ua);
    isMac = isMacPlatform && !isMobileBrowser;
  }

  return {
    isMobileBrowser,
    isTauri,
    isTauriDesktop: isTauri && !isMobileBrowser,
    isWindows,
    isMac,
    isTauriWindows: isTauri && isWindows,
    isTauriMac: isTauri && isMac,
    isTauriMobile: isTauri && isMobileBrowser,
  };
};

export const useMainStore = defineStore('counter', {
  state: () => ({
    tab: TAB.BADGE as TabType,
    user: {} as User,
    apxID: 0,
    vdrID: 0,
    vdrToken: '',
    token: get<string>(KEYS.T) || '',
    isLoaderOn: false,
    lang: LANG.JA.LONG,
    tinderCurrentIndex: 0,
    events: dummyEvents as CalendarEvent[],
    cards: dummyCards as Card[], // extraCards と絶対に被らないようにサーバサイドで制御（events のうち isFixed = true の event の日付と被るものがあってはならない）
    extraCards: dummyExtraCards as Card[], // cards と絶対に被らないようにサーバサイドで制御（events のうち isFixed = true の event の日付と被るものがあってはならない）
    badges: SAMPLE_BADGES as Badge[],
    usrBadges: SAMPLE_USR_BADGES as UsrBadge[],
    candidates: SAMPLE_BADGE_CANDIDATES as BadgeCandidate[],
    gaveBadges: SAMPLE_GAVE_BADGES as GaveBadge[],
    leftDrawerOpen: false,
    rightDrawerOpen: false,
    isBottomSheetOpen: false,
    platform: detector(),
    apps: [] as MycuteAppConfig[],
    isOverlayVisible: false,
    isAlwaysOnTop: get<boolean>(KEYS.AT) || false,
    sttEngine: get<string>(KEYS.SE) || ENGINE_OS,
    llms: [] as LlmEndpoint[],  // バックエンドを Source of Truth とするため、起動時に GET /v1/mycute/llms/get で初期化
  }),
  getters: {
    assignableCards: (state) => state.cards.filter(card => {
      const hasOverlap = state.events.some(event => isDateRangeOverlap(card.start, card.end, event.start, event.end))
      return !hasOverlap
    }),
    subTotal: (state): number => { // 仮 & 確定 の events の、「明日以降」における報酬合計
      // 今日の日付を取得して明日の0時0分0秒に設定
      const tomorrow = new Date();
      tomorrow.setHours(0, 0, 0, 0);
      tomorrow.setDate(tomorrow.getDate() + 1);
      // 明日以降のイベントをフィルタリングして報酬合計を計算
      return state.events
        .filter(event => {
          const eventStart = typeof event.start === 'string' ? new Date(event.start) : event.start;
          return eventStart >= tomorrow;
        })
        .reduce((total, event) => {
          return total + calcHourlyWage(event.start, event.end, event.hourPrice);
        }, 0);
    },
    subBadgeTotal: (state): number => { // 仮 & 確定 の events の、「明日以降」における最大バッジ合計
      // 今日の日付を取得して明日の0時0分0秒に設定
      const tomorrow = new Date();
      tomorrow.setHours(0, 0, 0, 0);
      tomorrow.setDate(tomorrow.getDate() + 1);
      // 明日以降のイベントをフィルタリングして報酬合計を計算
      return state.events
        .filter(event => {
          const eventStart = typeof event.start === 'string' ? new Date(event.start) : event.start;
          return eventStart >= tomorrow;
        })
        .reduce((total, event) => {
          return total + event.maxBadges;
        }, 0);
    },
    subGaveBadgeTotal: (state): number => {
      return state.gaveBadges.length
    },
    subPointTotal: (state): number => { // アマギフによるポイント還元未済の授与バッジで計算される未償還ポイント合計
      return state.gaveBadges.length * POINT_PER_GAVE_BADGE
    },
    isLoggedIn: (state): boolean => {
      return !!state.vdrToken && !!state.token
    }
  },
  actions: {
    setTab(tab: TabType) { this.tab = tab },
    setUser(user: User) { this.user = user },
    setApxID(apxID: number) { this.apxID = apxID },
    setVdrID(vdrID: number) { this.vdrID = vdrID },
    setVdrToken(vdrToken: string) { this.vdrToken = vdrToken },
    setToken(token: string) {
      set(KEYS.T, token)
      this.token = token
    },
    setIsLoaderOn(isLoaderOn: boolean) { this.isLoaderOn = isLoaderOn },
    setLang(lang: string) { this.lang = lang },
    setTinderCurrentIndex(tinderCurrentIndex: number) { this.tinderCurrentIndex = tinderCurrentIndex },
    setEvents(events: CalendarEvent[]) { this.events = events },
    setCards(cards: Card[]) { this.cards = cards },
    setExtraCards(extraCards: Card[]) { this.extraCards = extraCards },
    setBadges(badges: Badge[]) { this.badges = badges },
    setUsrBadges(usrBadges: UsrBadge[]) { this.usrBadges = usrBadges },
    setCandidates(candidates: BadgeCandidate[]) { this.candidates = candidates },
    setGaveBadges(gaveBadges: GaveBadge[]) { this.gaveBadges = gaveBadges },
    pushEventByCard(card: Card) { this.events.push(card as CalendarEvent) },
    pushGaveBadgeByCandidate(candidate: BadgeCandidate) { this.gaveBadges.push(candidate as GaveBadge) },
    setLeftDrawerOpen(open: boolean) { this.leftDrawerOpen = open },
    setRightDrawerOpen(open: boolean) { this.rightDrawerOpen = open },
    setIsBottomSheetOpen(open: boolean) { this.isBottomSheetOpen = open },
    setApps(apps: MycuteAppConfig[]) { this.apps = apps },
    pushApp(app: MycuteAppConfig) { this.apps.push(app) },
    removeApp(appId: string) { this.apps = this.apps.filter(a => a.app.id !== appId) },
    setIsOverlayVisible(isOverlayVisible: boolean) { this.isOverlayVisible = isOverlayVisible },
    setIsAlwaysOnTop(isAlwaysOnTop: boolean) {
      set(KEYS.AT, isAlwaysOnTop)
      this.isAlwaysOnTop = isAlwaysOnTop
    },
    async setSttEngine(sttEngine: string) {
      set(KEYS.SE, sttEngine)
      this.sttEngine = sttEngine
      const { invoke } = await import('@tauri-apps/api/core')
      await invoke('switch_stt_engine', { engine: sttEngine })
    },
    setLlms(llms: LlmEndpoint[]) { this.llms = llms },
  },
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useMainStore, import.meta.hot));
}
