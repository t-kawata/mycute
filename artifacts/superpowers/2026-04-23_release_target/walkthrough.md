# Walkthrough - Makefileへのreleaseターゲット追加

`scripts/release.sh` を `make` から直接呼び出せるように `Makefile` を更新しました。

## 変更内容

### [Makefile](file:///Users/shyme01/shyme/mycute/Makefile)
- `.PHONY` に `release` を追加しました。
- `release` ターゲットを追加しました。
- `MAKECMDGOALS` を利用して、コマンドライン引数をスクリプトに渡す仕組みを実装しました。
- ディレクトリ名が別のターゲットとして解釈されないように、ダミーのキャッチオールターゲット（ `%:` ）を導入しました。

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
```bash
$ mkdir -p tmp_test_release && touch tmp_test_release/test.txt && make release tmp_test_release
To get started with GitHub CLI, please run:  gh auth login
...
make: *** [release] Error 4
```
※ `gh` コマンドが認証エラーで停止しているのは、環境に `GH_TOKEN` が設定されていないためであり、 `Makefile` からの呼び出し自体は成功しています。

## 使い方
今後、ビルド済みの成果物をリリースする際は、以下のコマンドを使用してください：

```bash
make release dist/mac/v1.2.3
```
（ `dist/mac/v1.2.3` の部分は、実際にリリースしたいファイルが入っているディレクトリに置き換えてください）