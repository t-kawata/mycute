/**
 * アプリケーションのタイプを定義する定数。
 */
export const APP_TYPE = {
    MYCUTE: 'mycute',
    WEB: 'web'
} as const;

/**
 * アプリケーションのタイプ。
 * APP_TYPE の値のいずれかであることが保証されます。
 */
export type AppType = typeof APP_TYPE[keyof typeof APP_TYPE];

/**
 * Mycuteシステム内で共通して扱われるアプリケーションの構造体インターフェース。
 * 内部実装のページか、外部のウェブサイトかに関わらず、
 * この型に従うことで一貫したユーザ体験を提供します。
 */
export interface MycuteApp {
    /** アプリケーションを一意識別するためのID */
    id: string;

    /** ユーザーに表示されるアプリケーション名 */
    name: string;

    /**
     * 表示されるアイコン。
     * 現状はVueコンポーネントを想定していますが、将来的に文字列での指定も
     * 考慮できるように柔軟な型を持たせています。
     */
    icon: any;

    /** アプリケーションの種別 */
    type: AppType;

    /**
     * 起動先パス。
     * type が APP_TYPE.MYCUTE の場合は Vue Router のパス、
     * type が APP_TYPE.WEB の場合は http(s) から始まるフルURLを想定します。
     */
    url: string;
}

/**
 * ボトムシート上でのアプリケーションの配置情報を管理するための設定インターフェース。
 * アプリケーションの実体（MycuteApp）に、「どこに表示するか」という
 * コンテキストを付与します。
 */
export interface MycuteAppConfig {
    /** ターゲットとなるアプリケーションの実体 */
    app: MycuteApp;

    /** 
     * 表示されるページ番号（0開始）。
     * ボトムシートが複数のページを持つ場合に使用します。
     */
    page: number;

    /**
     * ページ内でのスロット位置（0〜15）。
     * 4x4のグリッドレイアウト上の配置場所を決定します。
     */
    slot: number;
}
