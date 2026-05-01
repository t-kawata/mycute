# 再起動ボタンへのローディング処理追加 ウォークスルー

再起動ボタン（`restartMycute`）をクリックした際に、終了ボタンと同様のローディング処理が行われるように修正を完了しました。

## 変更内容

### Frontend

#### [App.vue](file:///Users/shyme01/shyme/mycute/web/src/App.vue)

再起動ボタンのクリックハンドラを修正し、終了ボタン（`shutdownMycute`）と完全に一貫した実装に変更しました。

```diff
-const restartMycute = async () => { await relaunch(); }
+const restartMycute = async () => {
+  mainStore.setIsLoaderOn(true);
+  await sleep(300);
+  await relaunch();
+}
```

## 検証結果

- **コードの一貫性**: `restartMycute` 関数が `shutdownMycute` 関数と同じ構造（ローディングON -> 300ms待機 -> 実行）になっていることを確認しました。
- **UXの向上**: これにより、再起動ボタンを押した瞬間に画面がフリーズしたように見える現象が解消され、ユーザーに適切なフィードバックが提供されるようになります。
