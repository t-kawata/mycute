# SettingsApp のデザイン共通化 - 完了報告

## 概要
`SettingsApp.vue` のデザインを `LlmApp.vue` と完全に同期させました。
背景画像、すりガラスのボーダー丸み、および波紋エフェクトを両アプリで共通して使用できるように設計を見直しました。

## 実施内容
1.  **共通クラスと定数 (app.scss)**:
    - 角丸をSCSS変数 `$mycute-glass-border-radius: 45px;` として定義。
    - コンテナ全体用とすりガラスパネル用の2つの共通クラス `.__mycute-glass-app-container`, `.__mycute-glass-panel-inner` を追加。
2.  **LlmApp.vue**:
    - 新しい共通クラスへ移行し、重複していた scoped CSS を削除。
3.  **SettingsApp.vue**:
    - `WaterRipple` コンポーネントを配置。
    - 全体を共通クラスでラップし、内部の元のコンテンツの padding やスクロール領域が破綻しないよう、インラインで安全にスタイリングを調整。

## 検証結果
`make check-fe` によるフロントエンドビルドが正常に完了しました。既存のコンテンツに悪影響を与えずに、統一感のあるデザインが適用されています。
