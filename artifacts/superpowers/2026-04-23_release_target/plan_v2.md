# Makefileへのall-releaseターゲット追加計画

ビルド（ `make all` ）が成功した後に、自動的に成果物ディレクトリを特定してGitHubにリリースを行う `all-release` ターゲットを追加します。

## ユーザーレビューが必要な事項
- このターゲットは、現在のOSに合わせて `dist/mac/vX.X.X` または `dist/win/vX.X.X` を自動的に選択します。
- `all` ターゲットに依存するため、ビルドが失敗した場合はリリース処理は実行されません。

## Proposed Changes

### [Component] Makefile

#### [MODIFY] [Makefile](file:///Users/shyme01/shyme/mycute/Makefile)

`.PHONY` に `all-release` を追加し、 `release` ターゲットの後に実装を追記します。

```make
# ============================================================
# ターゲット: all-release (ビルド完了後に自動リリース)
# ============================================================
all-release: all
ifeq ($(OS),Windows_NT)
	@V=$$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); \
	echo "Releasing Windows version $$V..."; \
	make release dist/win/v$$V
else
	@V=$$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); \
	echo "Releasing Mac version $$V..."; \
	make release dist/mac/v$$V
endif
```

## 検証計画

### 自動テスト・検証
- `make all-release` を実行し、 `make all` が正常に終了した後に正しいディレクトリパスで `make release` が呼び出されることを確認します。
- バージョン重複エラーなどで `make all` が失敗した場合に、 `make release` が実行されないことを確認します。

### 手動確認
- 実際に開発環境で `make all-release` を実行し、ビルドからアップロードまでが自動で行われることを確認します。