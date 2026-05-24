# 実装計画: M1.5-R11 Event Architecture 較正候補定数 + プロパティベース不変条件ファジング

## RFC 既存実装状態検証

### RFC §12C 定数表 vs チケット仕様 — 不一致解決

RFC §12C (L5854-5861) が絶対正本。以下の方針で実装する:
- RFC §12C の6定数を最優先で実装（EVENTBUS_ プレフィックス）
- チケット仕様の定数は RFC 命名規則に合わせ EVEBTBUS_ プレフィックスで追加
- EVENT_BUS_MAX_RETRY_COUNT は RFC の EVENTBUS_MAX_RECONNECT_RETRIES と統合
- EVENT_REPLAY_BATCH_SIZE は RFC の EVENTBUS_REPLAY_BATCH_SIZE と統合（値は RFC の 100）

### 変更ファイル一覧
| ファイル | 種別 | 内容 |
|----------|------|------|
| src/constants.rs | 編集 | RFC §12C 定数6件 + 追加定数5件 |
| src/event.rs | 編集 | proptest 戦略3種 + invariant suite 9テスト + 定数確認テスト C-1〜C-7 + 極端値テスト E-1〜E-3 |

### 実装手順
1. constants.rs に全11定数を分類付きで追加
2. event.rs mod tests に定数確認テスト C-1〜C-7
3. event.rs に proptest 戦略群 (P-1〜P-3)
4. event.rs に invariant suite (P-4〜P-9)
5. event.rs に極端値テスト (E-1〜E-3)
6. cargo test 全通過確認
7. 観察レポート保存
8. 品質チェック
