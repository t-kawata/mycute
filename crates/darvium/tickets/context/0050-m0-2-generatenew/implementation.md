# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 変更内容 |
|---|---|---|
| `src/types.rs` | 修正 | 3 つの新規型を追加: `SideEffectSet` (6 fields + 2 methods), `PlaneKind` (3 variants), `SafeSandboxScope` (3 fields) |
| `src/guard.rs` | **新規** | 2 関数 + 12 ユニットテスト (T1-T12) + 3 観測テスト (OTS-1/2/3) |
| `src/lib.rs` | 修正 | `pub mod guard;` 追加、`guard_new_proposal_or_review` 公開APIエクスポート、3 型の再エクスポート |

## 実装の概要

### RFC 交叉参照

- **RFC §6.1**: `SideEffectSet` をフィールド・メソッド (`contains`, `is_safe_for_auto_approval`) 含めて完全実装
- **RFC §13.6**: ガード条件「UnsafeSearchTransition として拒否」を `DarviumError::SearchValidation` + `NeedsHumanReview` ルーティングで実装
- **RFC §16A v2.3**: Auto-Approval Exception Policy の scope boundary を `SafeSandboxScope` で表現

### 公開 API

```rust
// GenerateNew 選択後の安全ガード
pub fn guard_new_proposal_or_review(
    proposal: WorkflowGraph,
    side_effects: &SideEffectSet,
    plane: PlaneKind,
    scope: Option<&SafeSandboxScope>,
) -> Result<SearchOutcome, DarviumError>;
```

### テスト結果

- 402 テスト PASS（既存 390 + 新規 12）
- 5 integration tests PASS
- OTS-1: Production 閉包率 1.0 (352/352)
- OTS-2: Training auto-approval 率 0.25 (理論値 8/32 = 25% と一致)
- OTS-3: SafeSandbox scope 境界一致率 1.0 (320/320)
- コンパイル警告: 新規コードに警告なし（既存 composition.rs の deprecated RNG メソッド 7件のみ）
