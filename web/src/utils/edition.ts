import { APP_SLUG } from 'src/configs/settings'
import { Edition } from 'src/enums/Edition'

/**
 * 現在のエディションが NECO-ASOVI かどうかを判定します。
 */
export const isNecoAsovi = () => (APP_SLUG as string) === Edition.NECO_ASOVI

/**
 * 現在のエディションが MYCUTE かどうかを判定します。
 */
export const isMycute = () => (APP_SLUG as string) === Edition.MYCUTE
