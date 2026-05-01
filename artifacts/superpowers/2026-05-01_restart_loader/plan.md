# 再起動ボタンへのローディング処理追加

再起動ボタン（`restartMycute`）をクリックした際、終了ボタンと同様にローディングマスクを一定時間表示してから再起動を実行するように修正します。これにより、ユーザーに対して「処理中であること」を明示し、画面が固まったような印象を与えるのを防ぎます。

## Proposed Changes

### Frontend

#### [MODIFY] [App.vue](file:///Users/shyme01/shyme/mycute/web/src/App.vue)
- `restartMycute` 関数を修正し、`mainStore.setIsLoaderOn(true)` と `sleep(300)` を追加します。

## Verification Plan

### Manual Verification
- 修正後のコードが `shutdownMycute` と完全に一貫していることを目視で確認します。
