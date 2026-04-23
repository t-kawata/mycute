# Walkthrough - Makefileへのreleaseターゲット追加

`scripts/release.sh` を `make` から直接呼び出せるように `Makefile` を更新しました。

## 変更内容

### [Makefile](file:///Users/shyme01/shyme/mycute/Makefile)
- `.PHONY` に `release` を追加しました。
- `release` ターゲットを追加しました。
- `MAKECMDGOALS` を利用して、コマンドライン引数をスクリプトに渡す仕組みを実装しました。
- ディレクトリ名が別のターゲットとして解釈されないように、ダミーのキャッチオールターゲット（ `%:` ）を導入しました。

### [Makefile](file:///Users/shyme01/shyme/mycute/Makefile) (追加分)
- `all-release` ターゲットを追加しました。
- このターゲットは `make all` に依存し、ビルド成功後に OS ごとの `dist` ディレクトリ（ `dist/mac/vX.X.X` または `dist/win/vX.X.X` ）を自動的に引数として `make release` を呼び出します。

## 検証結果

### 1. 引数なしでの実行
`make release` を実行した際、ディレクトリの指定が必要である旨のエラーメッセージが表示されることを確認しました。
```bash
$ make release
Error: Directory is required. (e.g. make release dist/mac/v1.2.3)
make: *** [release] Error 1
```

### 2. ディレクトリ指定での実行
ダミーのディレクトリを作成して実行し、 `scripts/release.sh` が正しく呼び出され、最終的に `gh` コマンド（GitHub CLI）の実行まで到達することを確認しました。

### 3. all-release のドライラン
`make -n all-release` を実行し、 `make all` の後に正しいディレクトリパスで `make release` が呼び出される一連のコマンド列が表示されることを確認しました。
```bash
...
V=$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); \
	echo "Releasing Mac version $V..."; \
	make release dist/mac/v$V
```

## 使い方
今後、ビルドからリリースまでを一気に行う場合は、以下のコマンドを使用してください：

```bash
make all-release
```
これにより、ビルド済みのインストーラーやバイナリが自動的に GitHub にアップロードされます。