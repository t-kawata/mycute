import { CalendarEvent, Card } from "src/models/main"

export const randStr = (length: number): string => {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
  return Array.from({ length }, () => chars[Math.floor(Math.random() * chars.length)]).join('')
}

export const truncate = (str: string, maxLength: number): string => {
  if (str.length > maxLength) return str.slice(0, maxLength - 1) + '...'
  return str
}

/**
 * 日付のみを取得（時刻を0:00:00にリセット）
 */
export const getDateOnly = (date: Date | string): Date => {
  const d = new Date(date);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

/**
 * 2つの日付期間が重複しているかをチェック（日付のみで判定）
 */
export const isDateRangeOverlap = (
  start1: Date | string,
  end1: Date | string,
  start2: Date | string,
  end2: Date | string
): boolean => {
  const s1 = getDateOnly(start1);
  const e1 = getDateOnly(end1);
  const s2 = getDateOnly(start2);
  const e2 = getDateOnly(end2);
  // 重複判定: start1 <= end2 AND end1 >= start2
  return s1 <= e2 && e1 >= s2;
}
/**
 * イベントの日付を未来に正規化する関数
 *
 * 配列内の全イベントのstart/end日付を検査し、最小日付が過去または今日の場合、
 * 全てのイベントの日付を明日以降に移動します。相対的な日付の関係性は保持されます。
 * 第二引数で指定されたイベントの日付と重複しないように自動調整されます。
 *
 * @template T - Card または CalendarEvent を継承する型
 * @param events - 正規化対象のイベント配列
 * @param excludedEvents - 重複を避けるべきイベント配列（既存イベント）
 * @returns 日付が正規化されたイベント配列。最小日付が既に未来の場合は元の配列を返す
 *
 * @example
 * const newEvents = [
 *   { id: 1, start: '2025-10-02T09:00:00', end: '2025-10-02T17:00:00', ... }
 * ];
 * const existingEvents = [
 *   { id: 99, start: '2025-10-31T09:00:00', end: '2025-10-31T17:00:00', ... }
 * ];
 * const normalized = normalizeEventsToFuture(newEvents, existingEvents);
 * // newEventsは2025-10-31を避けて2025-11-01以降に配置される
 */
export const normalizeEventsToFuture = <T extends Card | CalendarEvent>(
  events: T[],
  excludedEvents: T[] = []
): T[] => {
  if (events.length === 0) return events;

  // 全イベントの日付をDateオブジェクトに変換し、最小日付を見つける
  const dates = events.flatMap(event => [
    new Date(event.start),
    new Date(event.end)
  ]);
  const minDate = new Date(Math.min(...dates.map(d => d.getTime())));

  // 明日の日付(時刻を00:00:00に設定)
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  tomorrow.setHours(0, 0, 0, 0);

  // 最小日付も時刻を00:00:00に設定して比較
  const minDateNormalized = new Date(minDate);
  minDateNormalized.setHours(0, 0, 0, 0);

  // 最小日付が明日より前(過去または今日)の場合
  if (minDateNormalized < tomorrow) {
    // 初期オフセット日数を計算
    let daysDiff = Math.ceil(
      (tomorrow.getTime() - minDateNormalized.getTime()) / (1000 * 60 * 60 * 24)
    );

    // 除外イベントの日付範囲を日単位で正規化して収集
    const excludedDateRanges = excludedEvents.map(event => {
      const start = new Date(event.start);
      const end = new Date(event.end);
      start.setHours(0, 0, 0, 0);
      end.setHours(0, 0, 0, 0);
      return { start, end };
    });

    // 重複チェック関数（日単位で判定）
    const hasOverlap = (shiftedEvents: T[]): boolean => {
      for (const event of shiftedEvents) {
        const eventStart = new Date(event.start);
        const eventEnd = new Date(event.end);
        eventStart.setHours(0, 0, 0, 0);
        eventEnd.setHours(0, 0, 0, 0);

        // 除外リスト内のいずれかのイベントと重複しているかチェック
        for (const excluded of excludedDateRanges) {
          // 日付範囲の重複判定: end2 >= start1 && end1 >= start2
          if (excluded.end >= eventStart && eventEnd >= excluded.start) {
            return true; // 重複あり
          }
        }
      }
      return false; // 重複なし
    };

    // 日付をシフトして重複がなくなるまで調整
    let shiftedEvents: T[];
    let maxIterations = 365; // 無限ループ防止

    do {
      shiftedEvents = events.map(event => {
        const startDate = new Date(event.start);
        const endDate = new Date(event.end);

        startDate.setDate(startDate.getDate() + daysDiff);
        endDate.setDate(endDate.getDate() + daysDiff);

        return {
          ...event,
          start: startDate.toISOString(),
          end: endDate.toISOString()
        };
      });

      if (excludedEvents.length === 0 || !hasOverlap(shiftedEvents)) {
        return shiftedEvents;
      }

      daysDiff++; // 1日進めて再チェック
      maxIterations--;
    } while (maxIterations > 0);

    // 最大反復回数に達した場合は最後の結果を返す
    return shiftedEvents;
  }

  // すでに未来の日付の場合はそのまま返す
  return events;
}
