import { createI18n } from 'vue-i18n';
import messages from 'src/i18n';

export type MessageLanguages = keyof typeof messages;
export type MessageSchema = typeof messages['en-US'];

declare module 'vue-i18n' {
    export interface DefineLocaleMessage extends MessageSchema { }
    export interface DefineDateTimeFormat { }
    export interface DefineNumberFormat { }
}

export const i18n = createI18n<{ message: MessageSchema }, MessageLanguages>({
    locale: 'ja-JP',
    legacy: false,
    messages,
});
