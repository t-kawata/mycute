# 実装サマリ: M1.5-R3 StoredInteraction → InteractionRecord<HitlPayload> 型エイリアス移行

## 変更したファイル

| ファイル | 変更内容 |
|---|---|
| `src/types.rs` | TC-4 クロス型 JSON ラウンドトリップテスト `cross_type_json_roundtrip_n1000` 追加（n=1000） |
| `Darvium-v2.3-final-table-and-struct-definition-spec.md` | 旧 `struct StoredInteraction` 独立定義を `type StoredInteraction = InteractionRecord<HitlPayload>` に更新。`HitlPayload` 構造体定義と `InteractionPayload` 実装を追加 |

## 既存状態（変更なし）

型エイリアス `pub type StoredInteraction = InteractionRecord<HitlPayload>` は M1.5-R1 の段階で `src/types.rs:5194` に既に定義済み。後方互換アクセサ（`request()` / `outcome()`）も同様に実装済み。

## 検証結果

- `cargo test` — 全テスト PASS
- `cargo test cross_type_json_roundtrip_n1000 -- --nocapture` — 1000/1000 (100.0%) PASS
- `cargo check` — PASS
- RFC §12B.2 との実装一致確認 — 全フィールド一致
