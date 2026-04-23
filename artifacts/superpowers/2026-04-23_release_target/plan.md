# Makefileへのreleaseターゲット追加計画

`scripts/release.sh` を利用して、ビルド済みファイルをGitHubにリリースするための `release` ターゲットを `Makefile` に追加します。

## ユーザーレビューが必要な事項
- `make release /path/to/dir` という構文を実現するため、 `Makefile` にキャッチオールターゲット（ `%:` ）を追加します。これにより、ディレクトリ名がターゲットとして誤認されるのを防ぎます。

## Proposed Changes

### [Component] Makefile

#### [MODIFY] [Makefile](file:///Users/shyme01/shyme/mycute/Makefile)

`Makefile` の適当な位置（ビルド・インストーラー系の後）に、以下のターゲットを追加します。

```make
# ============================================================
# ターゲット: release (GitHub リリースの作成とアップロード)
# ============================================================
# 使用方法: make release <directory>
# 例: make release dist/mac/v1.2.3
# ============================================================
release:
	@if [ -z "$(filter-out $@,$(MAKECMDGOALS))" ]; then \
		echo "\033[1;31mError: Directory is required. (e.g. make release dist/mac/v1.2.3)\033[0m"; \
		exit 1; \
	fi
	@bash scripts/release.sh $(filter-out $@,$(MAKECMDGOALS))

# make release <dir> において、<dir> をターゲットとして扱わないためのダミー
%:
	@:
```

## 検証計画

### 自動テスト・検証
- `make release` （引数なし）を実行し、エラーメッセージが表示されることを確認します。
- `make release tests/` （テスト用のダミーディレクトリ）を実行し、 `gh` コマンドが呼ばれることを確認します（実際にリリースを作成しないように、必要に応じて `scripts/release.sh` の動作をモックするか、慎重に確認します）。

### 手動確認
- 実際にビルド済みのディレクトリを指定して実行し、GitHub上にリリースが作成され、ファイルがアップロードされていることを確認します。