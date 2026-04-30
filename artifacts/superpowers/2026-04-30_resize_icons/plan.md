# 画像リサイズおよびメタデータ削除計画

## 概要
`web/public/neco-asovi/neco-asovi-icon02.png` および `neco-asovi-icon03.png` を幅 512px にリサイズし、不要なメタデータ（プロファイル等）を削除します。

## ユーザーレビューが必要な事項
ありません。

## 実施手順

### [Component Name] 画像処理

#### [MODIFY] [neco-asovi-icon02.png](file:///Users/kawata/shyme/mycute/web/public/neco-asovi/neco-asovi-icon02.png)
- `convert web/public/neco-asovi/neco-asovi-icon02.png -resize 512 -strip web/public/neco-asovi/neco-asovi-icon02.png` を実行。

#### [MODIFY] [neco-asovi-icon03.png](file:///Users/kawata/shyme/mycute/web/public/neco-asovi/neco-asovi-icon03.png)
- `convert web/public/neco-asovi/neco-asovi-icon03.png -resize 512 -strip web/public/neco-asovi/neco-asovi-icon03.png` を実行。

## 検証計画

### 手動検証
- `identify` コマンドを使用して、画像サイズが 512px（幅）になっていること、およびメタデータが削除されていることを確認します。