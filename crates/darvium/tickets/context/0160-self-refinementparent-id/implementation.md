# 実装サマリ: 本番 self-refinement での parent_id 設定漏れ修正

## 変更ファイル（2ファイル）

### src/self_refinement.rs
- register_abstracted_subworkflow で子グラフ登録後、registry.resolve_mut で parent_id を設定
- WorkflowGraphId ("wf-graph-{:016x}") から数値部分を hex パースして設定

### src/lib.rs
- parent_alive_map の ID パースを修正: parse::<usize>() → strip_prefix + from_str_radix(16)
- "wf-graph-{:016x}" 形式の hex ID を正しくパース可能に

## 検証結果
- cargo test --features server → 1394 passed, 0 failed
