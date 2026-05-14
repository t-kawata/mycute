<template>
  <Calendar ref="calendarRef" :events="mainStore.events" @change-height="onCalendarHeightChange" />
  <Tinder :cards="mainStore.cards" :height="300" :prevHeight="prevHeight" :onSwipe="onSwipeCard" />
  <div class="__mycute-tabpanel-container">

  </div>
</template>
<script setup lang="ts">
import { ref } from 'vue'
import Calendar from 'src/components/tools/Calendar.vue'
import Tinder from 'src/components/tools/Tinder.vue'
import { useMainStore } from 'src/stores/main-store'
import { SWIPE_DIRECTION, type Card } from 'src/models/main'
import { sleep, isTauriDesktop } from 'src/utils/some'

type CalendarInstance = InstanceType<typeof Calendar>

const mainStore = useMainStore()
const IS_TAURI_DESKTOP = isTauriDesktop()
const calendarRef = ref<CalendarInstance | null>(null)
const prevHeight = ref(IS_TAURI_DESKTOP ? 440 : 400)

const generateDateRangeStrs = (start: Date | string, end: Date | string): string[] => {
  const startDate = new Date(start);
  const endDate = new Date(end);
  // 同じ日付かどうかを確認
  const isSameDay =
    startDate.getFullYear() === endDate.getFullYear() &&
    startDate.getMonth() === endDate.getMonth() &&
    startDate.getDate() === endDate.getDate();
  // YYYY-MM-DDThh:mm:ss形式にフォーマット
  const formatDate = (date: Date): string => {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    const seconds = String(date.getSeconds()).padStart(2, '0');
    return `${year}-${month}-${day}T${hours}:${minutes}:${seconds}`;
  };
  // 同じ日付の場合
  if (isSameDay) {
    return [formatDate(startDate)];
  }
  // 違う日付の場合、日ごとにループ
  const result: string[] = [];
  const current = new Date(startDate);
  while (current <= endDate) {
    result.push(formatDate(new Date(current)));
    current.setDate(current.getDate() + 1);
  }
  return result;
}

const onCalendarHeightChange = (newHeight: number) => { prevHeight.value = newHeight + (IS_TAURI_DESKTOP ? 40 : 0) }

/**
 * CardのstartフィールドからjumpToに渡すための年と月を抽出する
 * @param card - Cardオブジェクト
 * @returns { year: number, month: number } - 月は1-12の形式
 */
const getYearMonthFromCard = (card: Card): { year: number; month: number } => {
  const date = card.start instanceof Date ? card.start : new Date(card.start);
  return {
    year: date.getFullYear(),
    month: date.getMonth() + 1 // JavaScriptのgetMonth()は0-11なので+1して1-12に変換
  };
};

const onSwipeCard = async (card: Card, direction: string) => {
  if (direction !== SWIPE_DIRECTION.LIKE) return
  if (!card || !card.start || !card.end) return
  const { year, month } = getYearMonthFromCard(card)
  if (calendarRef.value?.jumpTo(year, month)) await sleep(450)
  generateDateRangeStrs(card.start, card.end).forEach(datetimeStr => {
    const amimationDuration = 2000
    calendarRef.value?.highlightDate(datetimeStr, amimationDuration)
    setTimeout(() => { mainStore.pushEventByCard(card); }, amimationDuration)
  })
}
</script>
