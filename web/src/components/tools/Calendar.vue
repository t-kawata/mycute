<template>
  <div class="__mycute-calendar-wrapper" ref="calendarRef">
    <!-- Header -->
    <div class="__mycute-calendar-header">
      <q-btn flat round color="primary" class="__mycute-calendar-prev-next-btn q-ml-sm" @click="previousPeriod">
        <template v-slot:default><BackwardIcon /></template>
      </q-btn>
      <h2 class="__mycute-calendar-title text-primary">{{ headerTitle }}</h2>
      <q-btn flat round color="primary" class="__mycute-calendar-prev-next-btn q-mr-sm" @click="nextPeriod">
        <template v-slot:default><ForwardIcon /></template>
      </q-btn>
    </div>

    <!-- Calendar Container -->
    <div
      ref="calendarContainer"
      class="__mycute-calendar-container"
      @touchstart="handleTouchStart"
      @touchmove="handleTouchMove"
      @touchend="handleTouchEnd"
      @mousedown="handleMouseDown"
      @mousemove="handleMouseMove"
      @mouseup="handleMouseUp"
      @mouseleave="handleMouseUp"
    >
      <Transition :name="slideDirection">
        <div :key="currentKey" class="__mycute-calendar-content">
          <!-- Month View -->
          <div v-if="viewMode === 'month'" class="__mycute-calendar-month-view">
            <div class="__mycute-calendar-weekday-header">
              <div v-for="day in weekDayNames" :key="day" class="__mycute-calendar-weekday">
                {{ day }}
              </div>
            </div>
            <div class="__mycute-calendar-days-grid">
              <div
                v-for="(day, index) in monthDays"
                :key="index"
                :class="['__mycute-calendar-day-cell', ...getDayClasses(day)]"
                @click="handleDayClick(day)"
                v-ripple
              >
                <span class="__mycute-calendar-day-number">{{ day.date ? day.date.getDate() : '' }}</span>
                <div v-if="day.hasEvents && !day.hasFixedEvents" class="__mycute-calendar-tmp-event-indicator"></div>
              </div>
            </div>
          </div>
          <!-- Week View -->
          <div v-else class="__mycute-calendar-week-view">
            <div class="__mycute-calendar-week-header">
              <div class="__mycute-calendar-time-column"></div>
              <div
                v-for="(day, i) in weekDays"
                :key="day.key"
                class="__mycute-calendar-week-day-header"
              >
                <div class="__mycute-calendar-week-day-name">{{ day.dayName }}</div>
                <div
                  :class="['__mycute-calendar-week-day-date', { '__mycute-calendar-is-today': day.isToday }]"
                >{{ day.date ? day.date.getDate() : '' }}</div>
              </div>
            </div>
            <div class="__mycute-calendar-week-grid">
              <div class="__mycute-calendar-time-labels">
                <div v-for="hour in 24" :key="hour" class="__mycute-calendar-time-label">
                  {{ formatHour(hour - 1) }}
                </div>
              </div>
              <div class="__mycute-calendar-week-days">
                <div
                  v-for="day in weekDays"
                  :key="day.key"
                  class="__mycute-calendar-week-day-column"
                  @click="handleDayClick(day)"
                >
                  <div class="__mycute-calendar-hour-slots">
                    <div v-for="hour in 24" :key="hour" class="__mycute-calendar-hour-slot"></div>
                  </div>
                  <div class="__mycute-calendar-week-events">
                    <div
                      v-for="event in day.events"
                      :key="event.id"
                      :style="getEventStyle(event)"
                      class="__mycute-calendar-week-event"
                    >
                      <div class="__mycute-calendar-event-time">{{ formatEventTime(event) }}</div>
                      <div class="__mycute-calendar-event-title">{{ event.title }}</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useResizeObserver } from '@vueuse/core'
import ForwardIcon from 'src/components/icons/ForwardIcon.vue'
import BackwardIcon from 'src/components/icons/BackwardIcon.vue'
import { CalendarEvent } from 'src/models/main'

interface DayInfo {
  date: Date | null
  isCurrentMonth: boolean
  isToday: boolean
  hasEvents: boolean
  hasFixedEvents: boolean
  events: CalendarEvent[]
  key: string
  dayName?: string
}

interface Props {
  locale?: 'ja' | 'en'
  viewMode?: 'month' | 'week'
  events?: CalendarEvent[]
  onDayClick?: (date: Date, events: CalendarEvent[]) => void
}

const props = withDefaults(defineProps<Props>(), {
  locale: 'ja',
  viewMode: 'month',
  events: () => [],
  onDayClick: () => {}
});

const emit = defineEmits<{
  (e: 'change-height', height: number): void
}>()

const calendarRef = ref<HTMLElement | null>(null)
const height = ref(0)

const viewMode = computed(() => props.viewMode);

const currentDate = ref(new Date());
const currentKey = ref(0);
const slideDirection = ref<'__mycute-calendar-slide-left' | '__mycute-calendar-slide-right'>('__mycute-calendar-slide-left');
const calendarContainer = ref<HTMLElement | null>(null);

let touchStartX = 0;
let touchStartY = 0;
let isMouseDown = false;
const SWIPE_THRESHOLD = 50;

const locales = {
  ja: {
    weekDays: ['日', '月', '火', '水', '木', '金', '土'],
    months: ['1月', '2月', '3月', '4月', '5月', '6月', '7月', '8月', '9月', '10月', '11月', '12月']
  },
  en: {
    weekDays: ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'],
    months: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
  }
};

const weekDayNames = computed(() => locales[props.locale].weekDays);

const headerTitle = computed(() => {
  const year = currentDate.value.getFullYear();
  const month = currentDate.value.getMonth();
  if (props.locale === 'ja') {
    return `${year}年${month + 1}月`;
  }
  return `${locales.en.months[month]} ${year}`;
});

const createDayInfo = (date: Date, isCurrentMonth: boolean): DayInfo => {
  const today = new Date();
  const isToday = isSameDay(date, today);
  const dayEvents = getDayEvents(date);
  const hasFixedEvents = dayEvents.length > 0 && dayEvents.filter((e: CalendarEvent) => e.isFixed).length > 0
  return {
    date,
    isCurrentMonth,
    isToday,
    hasEvents: dayEvents.length > 0,
    hasFixedEvents: hasFixedEvents,
    events: dayEvents,
    key: date.toISOString()
  };
};

const getDayEvents = (date: Date): CalendarEvent[] => {
  return props.events.filter(event => {
    const eventStart = new Date(event.start);
    const eventEnd = new Date(event.end);
    const targetDate = new Date(date);
    targetDate.setHours(0, 0, 0, 0);
    return (
      targetDate >= new Date(eventStart.setHours(0, 0, 0, 0)) &&
      targetDate <= new Date(eventEnd.setHours(0, 0, 0, 0))
    );
  });
};

const isSameDay = (date1: Date, date2: Date): boolean => {
  return date1.getFullYear() === date2.getFullYear() &&
         date1.getMonth() === date2.getMonth() &&
         date1.getDate() === date2.getDate();
};

const getStartOfWeek = (date: Date): Date => {
  const d = new Date(date);
  const day = d.getDay();
  d.setDate(d.getDate() - day);
  d.setHours(0, 0, 0, 0);
  return d;
};

const getDayClasses = (day: DayInfo): string[] => {
  const classes: string[] = [];
  if (!day.isCurrentMonth) classes.push('__mycute-calendar-other-month');
  if (day.isToday) classes.push('__mycute-calendar-today');
  if (day.hasEvents) classes.push('__mycute-calendar-has-events');
  if (day.hasFixedEvents) classes.push('__mycute-calendar-has-fixed-events');
  return classes;
};

const handleDayClick = (day: DayInfo) => {
  if (day.date) {
    props.onDayClick?.(day.date, day.events);
  }
};

const previousPeriod = () => {
  slideDirection.value = '__mycute-calendar-slide-right';
  if (viewMode.value === 'month') {
    currentDate.value = new Date(currentDate.value.getFullYear(), currentDate.value.getMonth() - 1, 1);
  } else {
    currentDate.value = new Date(currentDate.value);
    currentDate.value.setDate(currentDate.value.getDate() - 7);
  }
  currentKey.value++;
};

const nextPeriod = () => {
  slideDirection.value = '__mycute-calendar-slide-left';
  if (viewMode.value === 'month') {
    currentDate.value = new Date(currentDate.value.getFullYear(), currentDate.value.getMonth() + 1, 1);
  } else {
    currentDate.value = new Date(currentDate.value);
    currentDate.value.setDate(currentDate.value.getDate() + 7);
  }
  currentKey.value++;
};

const jumpTo = (year: number | string, month: number | string): boolean => {
  // string型をnumber型に変換
  const targetYear = typeof year === 'string' ? parseInt(year, 10) : year;
  const targetMonth = typeof month === 'string' ? parseInt(month, 10) : month;

  // 月は1-12で受け取るが、Dateオブジェクトは0-11なので-1する
  const normalizedMonth = targetMonth - 1;

  // 現在表示されている年月を取得
  const currentYear = currentDate.value.getFullYear();
  const currentMonth = currentDate.value.getMonth();

  // 現在と同じ年月の場合は何もしない（早期リターン）
  if (currentYear === targetYear && currentMonth === normalizedMonth) {
    return false;
  }

  // 現在の年月と比較してスライド方向を決定
  const current = new Date(currentYear, currentMonth);
  const target = new Date(targetYear, normalizedMonth);

  slideDirection.value = target > current ? '__mycute-calendar-slide-left' : '__mycute-calendar-slide-right';

  // 新しい日付に移動
  currentDate.value = new Date(targetYear, normalizedMonth, 1);
  currentKey.value++;
  return true
};

const handleTouchStart = (e: TouchEvent) => {
  if (!e.touches || !e.touches[0]) return;
  touchStartX = e.touches[0].clientX;
  touchStartY = e.touches[0].clientY;
};

const handleTouchMove = (e: TouchEvent) => {
  if (!e.touches || !e.touches[0]) return;
  const deltaX = Math.abs(e.touches[0].clientX - touchStartX);
  const deltaY = Math.abs(e.touches[0].clientY - touchStartY);
  if (deltaX > deltaY) {
    e.preventDefault();
  }
};

const handleTouchEnd = (e: TouchEvent) => {
  if (!e.changedTouches || !e.changedTouches[0]) return;
  const touchEndX = e.changedTouches[0].clientX;
  const touchEndY = e.changedTouches[0].clientY;
  processSwipe(touchEndX, touchEndY);
};

const handleMouseDown = (e: MouseEvent) => {
  isMouseDown = true;
  touchStartX = e.clientX;
  touchStartY = e.clientY;
};

const handleMouseMove = (e: MouseEvent) => {
  if (!isMouseDown) return;
  const deltaX = Math.abs(e.clientX - touchStartX);
  const deltaY = Math.abs(e.clientY - touchStartY);
  if (deltaX > deltaY) {
    e.preventDefault();
  }
};

const handleMouseUp = (e: MouseEvent) => {
  if (!isMouseDown) return;
  isMouseDown = false;
  processSwipe(e.clientX, e.clientY);
};

const processSwipe = (endX: number, endY: number) => {
  const deltaX = endX - touchStartX;
  const deltaY = endY - touchStartY;
  if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > SWIPE_THRESHOLD) {
    if (deltaX > 0) {
      previousPeriod();
    } else {
      nextPeriod();
    }
  }
};

const formatHour = (hour: number): string => {
  return `${hour.toString().padStart(2, '0')}:00`;
};

const formatEventTime = (event: CalendarEvent): string => {
  const start = new Date(event.start);
  const end = new Date(event.end);
  return `${start.getHours()}:${start.getMinutes().toString().padStart(2, '0')} - ${end.getHours()}:${end.getMinutes().toString().padStart(2, '0')}`;
};

const getEventStyle = (event: CalendarEvent): Record<string, string> => {
  const start = new Date(event.start);
  const end = new Date(event.end);
  const startMinutes = start.getHours() * 60 + start.getMinutes();
  const endMinutes = end.getHours() * 60 + end.getMinutes();
  const duration = endMinutes - startMinutes;
  return {
    top: `${(startMinutes / 1440) * 100}%`,
    height: `${(duration / 1440) * 100}%`
  };
};

const monthDays = computed((): DayInfo[] => {
  const year = currentDate.value.getFullYear();
  const month = currentDate.value.getMonth();
  const firstDay = new Date(year, month, 1);
  const lastDay = new Date(year, month + 1, 0);
  const startDay = firstDay.getDay();
  const days: DayInfo[] = [];
  // 前月
  const prevMonthLast = new Date(year, month, 0);
  for (let i = startDay - 1; i >= 0; i--) {
    const date = new Date(year, month - 1, prevMonthLast.getDate() - i);
    days.push(createDayInfo(date, false));
  }
  // 当月
  for (let i = 1; i <= lastDay.getDate(); i++) {
    const date = new Date(year, month, i);
    days.push(createDayInfo(date, true));
  }
  // 翌月
  const remaining = 42 - days.length;
  for (let i = 1; i <= remaining; i++) {
    const date = new Date(year, month + 1, i);
    days.push(createDayInfo(date, false));
  }
  return days;
});

const weekDays = computed(() => {
  const days: (DayInfo & { dayName: string })[] = [];
  const startOfWeek = getStartOfWeek(currentDate.value);
  for (let i = 0; i < 7; i++) {
    const date = new Date(startOfWeek);
    date.setDate(startOfWeek.getDate() + i);
    const dInfo = createDayInfo(date, true);
    days.push({
      ...dInfo,
      dayName: weekDayNames.value[date.getDay()] as string
    });
  }
  return days;
});

const highlightDate = (targetDate: Date | string, duration: number = 1000) => {
  if (!calendarRef.value) return
  const smallClassName = '__mycute-calendar-wrapper-small'
  calendarRef.value.classList.add(smallClassName)
  setTimeout(() => { calendarRef.value?.classList.remove(smallClassName) }, 150)
  const target = typeof targetDate === 'string' ? new Date(targetDate) : targetDate;
  target.setHours(0, 0, 0, 0);
  let dayElement: HTMLElement | null = null;
  if (viewMode.value === 'month') {
    // 月表示の場合、monthDaysから該当日を検索
    const dayIndex = monthDays.value.findIndex(day => {
      if (!day.date) return false;
      const compareDate = new Date(day.date);
      compareDate.setHours(0, 0, 0, 0);
      return compareDate.getTime() === target.getTime();
    });
    if (dayIndex === -1) return; // 表示されていない日付
    // DOMから該当要素を取得
    const dayCells = calendarContainer.value?.querySelectorAll('.__mycute-calendar-day-cell');
    if (dayCells && dayCells[dayIndex]) {
      dayElement = dayCells[dayIndex] as HTMLElement;
    }
  } else {
    // 週表示の場合、weekDaysから該当日を検索
    const dayIndex = weekDays.value.findIndex(day => {
      if (!day.date) return false;
      const compareDate = new Date(day.date);
      compareDate.setHours(0, 0, 0, 0);
      return compareDate.getTime() === target.getTime();
    });
    if (dayIndex === -1) return; // 表示されていない日付
    // DOMから該当要素を取得
    const dayColumns = calendarContainer.value?.querySelectorAll('.__mycute-calendar-week-day-column');
    if (dayColumns && dayColumns[dayIndex]) {
      dayElement = dayColumns[dayIndex] as HTMLElement;
    }
  }
  if (dayElement) {
    dayElement.classList.add('__mycute-calendar-highlight-pulse');
    setTimeout(() => {
      dayElement?.classList.remove('__mycute-calendar-highlight-pulse');
    }, duration);
  }
};

watch(height, (newHeight) => { emit('change-height', newHeight) })

onMounted(() => {
  if (!calendarRef.value) return
  height.value = calendarRef.value.offsetHeight
  useResizeObserver(calendarRef, (entries) => {
      if (!entries || entries.length === 0) return
      const entry = entries[0] as ResizeObserverEntry
      const { height: newHeight } = entry.contentRect
      height.value = newHeight
    })
})

// defineExposeで公開
defineExpose({　highlightDate, jumpTo　})
</script>

<style lang="scss">
// Variables
$__mycute-calendar-color-primary: #0d6efd;
$__mycute-calendar-color-primary-light: #e7f3ff;
$__mycute-calendar-color-primary-bg: #cfe2ff;
$__mycute-calendar-color-primary-dark: #084298;
$__mycute-calendar-color-text: $dark;
$__mycute-calendar-color-text-muted: #6c757d;
$__mycute-calendar-color-text-light: #adb5bd;
$__mycute-calendar-color-bg: #ffffff;
$__mycute-calendar-color-bg-light: #f8f9fa;
$__mycute-calendar-color-border: #e9ecef;
$__mycute-calendar-color-border-light: #dee2e6;

$__mycute-calendar-spacing-xs: 4px;
$__mycute-calendar-spacing-sm: 8px;
$__mycute-calendar-spacing-md: 12px;
$__mycute-calendar-spacing-lg: 16px;

$__mycute-calendar-border-radius: 8px;
$__mycute-calendar-border-radius-sm: 4px;

$__mycute-calendar-transition-speed: 0.2s;
$__mycute-calendar-transition-speed-slow: 0.3s;

.__mycute-calendar {
  &-wrapper {
    width: 100%;
    max-width: 100%;
    background: $__mycute-calendar-color-bg;
    overflow: hidden;
    transition: all 0.15s ease;
    transform: scale(1);
    &-small {
      transform: scale(0.9);
    }
  }
  &-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  &-prev-next-btn {
    & svg {
      width: 24px;
      height: 24px;
      & path {
        fill: $primary;
      }
    }
  }

  &-title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: $__mycute-calendar-color-text;
  }

  &-container {
    position: relative;
    overflow: hidden;
    touch-action: pan-y;
  }

  &-content {
    width: 100%;
  }

  // Transitions
  &-slide-left,
  &-slide-right {
    &-enter-active,
    &-leave-active {
      transition: transform $__mycute-calendar-transition-speed-slow ease-out;
    }
  }

  &-slide-left {
    &-enter-from {
      transform: translateX(100%);
    }

    &-leave-to {
      transform: translateX(-100%);
    }

    &-leave-active {
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
    }
  }

  &-slide-right {
    &-enter-from {
      transform: translateX(-100%);
    }

    &-leave-to {
      transform: translateX(100%);
    }

    &-leave-active {
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
    }
  }

  // Month View
  &-month-view {
    padding: $__mycute-calendar-spacing-sm;
  }

  &-weekday-header {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: $__mycute-calendar-spacing-xs;
    margin-bottom: $__mycute-calendar-spacing-sm;
  }

  &-weekday {
    text-align: center;
    font-size: 12px;
    font-weight: 600;
    color: $__mycute-calendar-color-text-muted;
    padding: $__mycute-calendar-spacing-sm $__mycute-calendar-spacing-xs;
  }

  &-days-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: $__mycute-calendar-spacing-xs;
  }

  &-day-cell {
    aspect-ratio: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    position: relative;
    border-radius: $__mycute-calendar-border-radius;
    cursor: pointer;
    transition: background $__mycute-calendar-transition-speed;
    min-height: 44px;

    &:active {
      background: $__mycute-calendar-color-bg-light;
    }

    &.__mycute-calendar-other-month {
      .__mycute-calendar-day-number {
        color: $__mycute-calendar-color-text-light;
      }
    }

    &.__mycute-calendar-today {
      border: 2px solid $primary-light;
      /* .__mycute-calendar-day-number {
        color: #ffffff;
        font-weight: 700;
        text-shadow: 1px 1px 3px rgba(0, 0, 0, 0.2);
      } */
    }
    &.__mycute-calendar-has-fixed-events {
      background-color: $purple-light;
      .__mycute-calendar-day-number {
        color: #ffffff;
        font-weight: 700;
        text-shadow: 1px 1px 3px rgba(0, 0, 0, 0.2);
      }
    }
  }

  &-day-number {
    font-size: 14px;
    color: $__mycute-calendar-color-text;
    font-weight: 500;
  }

  &-tmp-event-indicator {
    width: 10px;
    height: 10px;
    background: $secondary;
    border-radius: 50%;
    margin-top: 2px;
  }

  // Week View
  &-week-view {
    overflow-x: auto;
  }

  &-week-header {
    display: grid;
    grid-template-columns: 50px repeat(7, 1fr);
    border-bottom: 1px solid $__mycute-calendar-color-border;
    position: sticky;
    top: 0;
    background: $__mycute-calendar-color-bg;
    z-index: 10;
  }

  &-time-column {
    width: 50px;
  }

  &-week-day-header {
    text-align: center;
    padding: $__mycute-calendar-spacing-md $__mycute-calendar-spacing-xs;
    border-left: 1px solid $__mycute-calendar-color-border;
  }

  &-week-day-name {
    font-size: 11px;
    color: $__mycute-calendar-color-text-muted;
    font-weight: 600;
    margin-bottom: $__mycute-calendar-spacing-xs;
  }

  &-week-day-date {
    font-size: 18px;
    font-weight: 600;
    color: $__mycute-calendar-color-text;
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;

    &.__mycute-calendar-is-today {
      background: $__mycute-calendar-color-primary;
      color: $__mycute-calendar-color-bg;
    }
  }

  &-week-grid {
    display: grid;
    grid-template-columns: 50px 1fr;
    position: relative;
    min-height: 600px;
  }

  &-time-labels {
    border-right: 1px solid $__mycute-calendar-color-border;
  }

  &-time-label {
    height: 60px;
    font-size: 11px;
    color: $__mycute-calendar-color-text-muted;
    padding: $__mycute-calendar-spacing-xs $__mycute-calendar-spacing-sm;
    text-align: right;
    border-top: 1px solid $__mycute-calendar-color-bg-light;
  }

  &-week-days {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
  }

  &-week-day-column {
    position: relative;
    border-left: 1px solid $__mycute-calendar-color-border;
  }

  &-hour-slots {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
  }

  &-hour-slot {
    height: 60px;
    border-top: 1px solid $__mycute-calendar-color-bg-light;
  }

  &-week-events {
    position: relative;
    height: 1440px; // 24 hours * 60px
  }

  &-week-event {
    position: absolute;
    left: 2px;
    right: 2px;
    background: $__mycute-calendar-color-primary-bg;
    border-left: 3px solid $__mycute-calendar-color-primary;
    border-radius: $__mycute-calendar-border-radius-sm;
    padding: $__mycute-calendar-spacing-xs 6px;
    font-size: 11px;
    overflow: hidden;
    cursor: pointer;
    transition: opacity $__mycute-calendar-transition-speed;

    &:hover {
      opacity: 0.9;
    }
  }

  &-event-time {
    font-weight: 600;
    color: $__mycute-calendar-color-primary-dark;
    margin-bottom: 2px;
  }

  &-event-title {
    color: #052c65;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  &-highlight-pulse {
    background-color: $secondary !important;
    animation: pulse-scale 0.3s ease-in-out infinite alternate;
    border-radius: 50%;
  }
}

@keyframes pulse-scale {
  0% {
    transform: scale(1);
    box-shadow: 0px 1px 1px 0px rgba(0, 0, 0, 0.1);
  }
  100% {
    transform: scale(1.10);
    box-shadow: 0px 1px 5px 0px rgba(0, 0, 0, 0.1);
  }
}

// Mobile Optimizations
@media (max-width: 768px) {
  .__mycute-calendar {
    &-title {
      font-size: 16px;
    }

    &-day-number {
      font-size: 13px;
    }

    &-week-day-header {
      padding: $__mycute-calendar-spacing-sm 2px;
    }

    &-time-label {
      font-size: 10px;
      padding: $__mycute-calendar-spacing-xs;
    }
  }
}

// Tablet Optimizations
@media (min-width: 769px) and (max-width: 1024px) {
  .__mycute-calendar {
    &-wrapper {
      max-width: 800px;
      margin: 0 auto;
    }
  }
}

// Desktop Optimizations
@media (min-width: 1025px) {
  .__mycute-calendar {
    &-wrapper {
      max-width: 1000px;
      margin: 0 auto;
    }

    &-day-cell {
      min-height: 60px;

      &:hover {
        background: rgba($__mycute-calendar-color-primary, 0.05);
      }
    }

    &-week-event {
      &:hover {
        transform: translateY(-1px);
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
      }
    }
  }
}
</style>
