# 開発ルーチン（複数PC間の同期運用）

このドキュメントは複数のPCで開発する際の同期ルールを定める。

## 基本サイクル

```
作業開始 → make sync → 作業 → make push（リモートに反映）
  ↑                                        ↓
  └──────── 別のPC で make sync ──────────┘
```

## コマンド一覧

| コマンド | 役割 | 安全度 |
|----------|------|--------|
| `make sync` | リモートの変更だけを取り込む（ローカルの変更は stash で保護） | 高い |
| `make pull` | `make sync` と同じ（エイリアス） | 高い |
| `make push` | バージョン自動インクリメント → コミット → プッシュ（事前にリモートチェックあり） | 中 |

## 運用ルール

### PC を切り替えたら最初に `make sync`

別のPCで push された変更を取り込んでから作業を開始する。

```bash
cd /Users/kawata/shyme/mycute
make sync
```

### push する前に `make sync`

ローカルでコミットを積んだ後、push する前に必ず sync して競合を予防する。

```bash
make sync    # リモートの最新を取り込む（競合があればここで解決）
make push    # バージョン自動インクリメント → プッシュ
```

### `make push` が ABORT したら

```
[ABORT] Remote has new changes that are not merged yet.
Run 'make sync' first, then try 'make push' again.
```

→ `make sync` を実行してから、再度 `make push` する。

```bash
make sync
make push
```

### 競合が発生したら

`make sync` 実行中に rebase で競合が発生した場合：

```bash
# 1. 競合ファイルを確認
git status

# 2. 競合を手動で解決（エディタで編集）

# 3. 解決したファイルをステージング
git add <解決したファイル>

# 4. rebase を継続
git rebase --continue

# 5. stash していた変更があれば復元
git stash pop
```

### こまめなコミット

`make push` はバージョン番号を自動インクリメントするため、1 push = 1 バージョン となる。
細かい単位でコミットしたい場合は、手動で `git add` + `git commit` してからまとめて `make push` する。

```bash
# 細かいコミットを積む
git add src/some_file.rs
git commit -m "fix: 〇〇のバグを修正"
git add src/another_file.rs
git commit -m "feat: △△を追加"

# これらを含めて push（バージョンインクリメント + 全コミットをプッシュ）
make push
```

## 緊急時対応

### ローカルを完全にリモートの状態にリセットしたい場合

```bash
git fetch origin master
git reset --hard origin/master
```

注意: 未コミットの変更は全て消失する。通常は `make sync` を使うこと。

### 直前のコミットをやり直したい場合

```bash
git commit --amend    # コミットメッセージの修正
# または
git reset --soft HEAD~1  # コミットを取り消し（変更は保持）
```

amend した場合は次回 push 時に --force が必要になる可能性があるため注意。

## 用語

| 用語 | 説明 |
|------|------|
| rebase | ローカルのコミットを、リモートの最新コミットの上に積み直す操作。履歴が一直線になる。 |
| stash | 作業中の変更を一時的に退避する機能。 |
| fetch | リモートの履歴を取得するが、ワーキングツリーには反映しない操作。 |
| origin/master | リモートリポジトリの master ブランチの状態を示す参照。 |
