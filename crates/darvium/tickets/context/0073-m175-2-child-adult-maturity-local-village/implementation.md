# 実装サマリ: M1.75-2 Child/Adult maturity 判定器および Local Village 構成ロジック

## 変更したファイル

### src/constants.rs（修正）
末尾に4定数を追加:
- MIN_SURVIVAL_EXPERIENCE = 5
- E_ADULT_THRESHOLD = 20
- T_ADULT_THRESHOLD = 0.70
- R_ADULT_THRESHOLD = 0.70

### src/village.rs（新規作成）
- WorkflowMaturity enum（Child/Adult）
- AdultCandidate 構造体
- LocalVillage 構造体
- classify_maturity() 純粋関数
- filter_adult_candidates() フィルタ関数
- compute_centroid() 内部関数
- build_local_village_topk() TopK 方式
- build_local_village_radius() 半径方式
- 21テスト（T-1〜T-20 + T-E1）

### src/lib.rs（修正）
- pub mod village 追加
- pub use で7つの公開 API をエクスポート

## 検証結果
- cargo test: 764 tests ALL PASS
- cargo clippy -- -D warnings: PASS
- RFC §41B.3 交叉参照: 矛盾なし
