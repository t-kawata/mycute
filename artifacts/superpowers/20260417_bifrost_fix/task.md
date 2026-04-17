# タスク: Bifrost 起動エラー（JSONパスエスケープ）の修正

- [x] `task.md` の作成
- [x] `src/bifrost/installer.rs` の修正
    - [x] `generate_config_json` 関数内でパスのバックスラッシュをスラッシュに置換する処理を追加
    - [x] ユニットテストの追加
- [x] 修正の検証
    - [x] `make check-be` (または `make check-be`) でコンパイル確認
    - [x] ユニットテストの実行 (パスのエスケープ確認)
    - [x] `config.json` の不備による起動失敗の再現確認 (調査フェーズで実施済み)
- [x] 完了報告とウォークスルーの作成
