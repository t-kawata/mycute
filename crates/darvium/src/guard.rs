// GenerateNew 安全ガードロジック
//
// RFC §13.6 ガード条件に基づき、GenerateNew 選択時に副作用プロファイルと
// 実行平面種別に応じて human review 強制または auto-approval を決定する。
//
// # ガード条件 (RFC §13.6)
//
// - Production plane: 全 GenerateNew は human review 必須
//   → SearchOutcome::NeedsHumanReview へルーティング
// - Training plane: safe-scoped なもののみ auto-approval 例外を許容
// - SafeSandbox plane: scope boundary 内の包含チェックで許可

use crate::error::DarviumError;
use crate::types::*;

/// GenerateNew の安全性を検査する内部ガード関数。
///
/// # 引数
///
/// * `side_effects` - 対象ミッションの副作用プロファイル
/// * `plane` - 実行平面種別
/// * `scope` - SafeSandbox のスコープ境界（SafeSandbox plane 時は必須）
///
/// # エラー
///
/// 安全でないと判断された場合、`DarviumError::SearchValidation` で
/// "UnsafeSearchTransition: ..." メッセージを返す。
pub fn check_generate_new_safety(
    side_effects: &SideEffectSet,
    plane: PlaneKind,
    scope: Option<&SafeSandboxScope>,
) -> Result<(), DarviumError> {
    match plane {
        PlaneKind::Production => Err(DarviumError::SearchValidation(
            "UnsafeSearchTransition: GenerateNew in production requires human review".into(),
        )),
        PlaneKind::Training => {
            if side_effects.is_safe_for_auto_approval() {
                Ok(())
            } else {
                Err(DarviumError::SearchValidation(format!(
                    "UnsafeSearchTransition: GenerateNew in training plane with unsafe side effects \
                     (writes_external_api={}, irreversible={})",
                    side_effects.writes_external_api, side_effects.irreversible,
                )))
            }
        }
        PlaneKind::SafeSandbox => {
            match scope {
                Some(s) => {
                    if side_effects.contains(&s.allowed_side_effects) {
                        Ok(())
                    } else {
                        Err(DarviumError::SearchValidation(
                            "UnsafeSearchTransition: GenerateNew in SafeSandbox exceeds scope boundary"
                                .into(),
                        ))
                    }
                }
                None => Err(DarviumError::SearchValidation(
                    "UnsafeSearchTransition: GenerateNew in SafeSandbox requires a scope definition"
                        .into(),
                )),
            }
        }
    }
}

/// GenerateNew 選択後、安全性に応じて human review または auto-approval へ振り分ける。
///
/// 安全と判断された場合は `SearchOutcome::GenerateNew` を返す。
/// 不安全と判断された場合は `SearchOutcome::NeedsHumanReview` を返す。
/// プログラム的誤用（scope 未指定等）は `DarviumError::SearchValidation` のエラーとする。
///
/// # 引数
///
/// * `proposal` - 生成されたワークフローグラフ
/// * `side_effects` - 対象ミッションの副作用プロファイル
/// * `plane` - 実行平面種別
/// * `scope` - SafeSandbox のスコープ境界（SafeSandbox plane 時は必須）
pub fn guard_new_proposal_or_review(
    proposal: WorkflowGraph,
    side_effects: &SideEffectSet,
    plane: PlaneKind,
    scope: Option<&SafeSandboxScope>,
) -> Result<SearchOutcome, DarviumError> {
    match check_generate_new_safety(side_effects, plane, scope) {
        Ok(()) => Ok(SearchOutcome::GenerateNew { proposal }),
        Err(DarviumError::SearchValidation(reason)) => {
            Ok(SearchOutcome::NeedsHumanReview { reason })
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── T1: Production plane — 全副作用パターンでブロック ──

    #[test]
    fn t1_production_plane_all_side_effects_blocked() {
        let proposal = WorkflowGraph::new();
        let patterns = [
            SideEffectSet {
                writes_external_api: false,
                sends_notification: false,
                has_hitl_communicate: false,
                modifies_persistent_state: false,
                irreversible: false,
                risk_score: 0.0,
            },
            SideEffectSet {
                writes_external_api: true,
                sends_notification: false,
                has_hitl_communicate: false,
                modifies_persistent_state: false,
                irreversible: false,
                risk_score: 0.5,
            },
            SideEffectSet {
                writes_external_api: false,
                sends_notification: true,
                has_hitl_communicate: false,
                modifies_persistent_state: false,
                irreversible: false,
                risk_score: 0.3,
            },
            SideEffectSet {
                writes_external_api: false,
                sends_notification: false,
                has_hitl_communicate: false,
                modifies_persistent_state: true,
                irreversible: false,
                risk_score: 0.7,
            },
            SideEffectSet {
                writes_external_api: false,
                sends_notification: false,
                has_hitl_communicate: false,
                modifies_persistent_state: false,
                irreversible: true,
                risk_score: 1.0,
            },
            SideEffectSet {
                writes_external_api: true,
                sends_notification: true,
                has_hitl_communicate: false,
                modifies_persistent_state: true,
                irreversible: false,
                risk_score: 0.8,
            },
            SideEffectSet {
                writes_external_api: false,
                sends_notification: true,
                has_hitl_communicate: false,
                modifies_persistent_state: true,
                irreversible: true,
                risk_score: 0.9,
            },
            SideEffectSet {
                writes_external_api: true,
                sends_notification: true,
                has_hitl_communicate: false,
                modifies_persistent_state: true,
                irreversible: true,
                risk_score: 1.0,
            },
        ];

        for (i, effects) in patterns.iter().enumerate() {
            let result =
                guard_new_proposal_or_review(proposal.clone(), effects, PlaneKind::Production, None);
            assert!(
                matches!(result, Ok(SearchOutcome::NeedsHumanReview { .. })),
                "T1 failed for pattern {}: expected NeedsHumanReview, got {:?}",
                i,
                result
            );
        }
    }

    // ── T2: Training plane — 安全な副作用で auto-approval ──

    #[test]
    fn t2_training_plane_safe_side_effects_auto_approval() {
        let proposal = WorkflowGraph::new();
        let safe_effects = SideEffectSet {
            writes_external_api: false,
            sends_notification: false,
            has_hitl_communicate: false,
            modifies_persistent_state: false,
            irreversible: false,
            risk_score: 0.2,
        };

        let result =
            guard_new_proposal_or_review(proposal, &safe_effects, PlaneKind::Training, None);
        assert!(
            matches!(result, Ok(SearchOutcome::GenerateNew { .. })),
            "T2 failed: expected GenerateNew, got {:?}",
            result
        );
    }

    // ── T3: Training plane — writes_external_api でブロック ──

    #[test]
    fn t3_training_plane_writes_external_api_blocked() {
        let proposal = WorkflowGraph::new();
        let unsafe_effects = SideEffectSet {
            writes_external_api: true,
            sends_notification: false,
            has_hitl_communicate: false,
            modifies_persistent_state: false,
            irreversible: false,
            risk_score: 0.5,
        };

        let result =
            guard_new_proposal_or_review(proposal, &unsafe_effects, PlaneKind::Training, None);
        assert!(
            matches!(result, Ok(SearchOutcome::NeedsHumanReview { .. })),
            "T3 failed: expected NeedsHumanReview, got {:?}",
            result
        );
    }

    // ── T4: Training plane — irreversible でブロック ──

    #[test]
    fn t4_training_plane_irreversible_blocked() {
        let proposal = WorkflowGraph::new();
        let unsafe_effects = SideEffectSet {
            writes_external_api: false,
            sends_notification: false,
            has_hitl_communicate: false,
            modifies_persistent_state: false,
            irreversible: true,
            risk_score: 1.0,
        };

        let result =
            guard_new_proposal_or_review(proposal, &unsafe_effects, PlaneKind::Training, None);
        assert!(
            matches!(result, Ok(SearchOutcome::NeedsHumanReview { .. })),
            "T4 failed: expected NeedsHumanReview, got {:?}",
            result
        );
    }

    // ── T5: SafeSandbox — 許可範囲内で auto-approval ──

    #[test]
    fn t5_safe_sandbox_within_scope_auto_approval() {
        let proposal = WorkflowGraph::new();
        let scope = SafeSandboxScope {
            namespace: "test-ns".into(),
            artifact_kind: "proposal".into(),
            allowed_side_effects: SideEffectSet {
                writes_external_api: false,
                sends_notification: true,
                has_hitl_communicate: false,
                modifies_persistent_state: true,
                irreversible: false,
                risk_score: 0.0,
            },
        };

        let effects = SideEffectSet {
            writes_external_api: false,
            sends_notification: true,
            has_hitl_communicate: false,
            modifies_persistent_state: true,
            irreversible: false,
            risk_score: 0.5,
        };

        let result = guard_new_proposal_or_review(
            proposal,
            &effects,
            PlaneKind::SafeSandbox,
            Some(&scope),
        );
        assert!(
            matches!(result, Ok(SearchOutcome::GenerateNew { .. })),
            "T5 failed: expected GenerateNew, got {:?}",
            result
        );
    }

    // ── T6: SafeSandbox — 許可範囲外でブロック ──

    #[test]
    fn t6_safe_sandbox_exceeds_scope_blocked() {
        let proposal = WorkflowGraph::new();
        let scope = SafeSandboxScope {
            namespace: "test-ns".into(),
            artifact_kind: "proposal".into(),
            allowed_side_effects: SideEffectSet {
                writes_external_api: false,
                sends_notification: true,
                has_hitl_communicate: false,
                modifies_persistent_state: false,
                irreversible: false,
                risk_score: 0.0,
            },
        };

        // writes_external_api=true は scope の許可範囲を超過
        let effects = SideEffectSet {
            writes_external_api: true,
            sends_notification: false,
            has_hitl_communicate: false,
            modifies_persistent_state: false,
            irreversible: false,
            risk_score: 0.5,
        };

        let result = guard_new_proposal_or_review(
            proposal,
            &effects,
            PlaneKind::SafeSandbox,
            Some(&scope),
        );
        assert!(
            matches!(result, Ok(SearchOutcome::NeedsHumanReview { .. })),
            "T6 failed: expected NeedsHumanReview, got {:?}",
            result
        );
    }

    // ── T7: SideEffectSet::contains 包含関係の網羅的テスト ──

    #[test]
    fn t7_side_effect_set_contains() {
        let self_true_write = SideEffectSet {
            writes_external_api: true,
            ..Default::default()
        };
        let self_false_write = SideEffectSet {
            writes_external_api: false,
            ..Default::default()
        };

        // required.writes_external_api=true, self=true → true
        assert!(self_true_write.contains(&SideEffectSet {
            writes_external_api: true,
            ..Default::default()
        }));
        // required.writes_external_api=true, self=false → false
        assert!(!self_false_write.contains(&SideEffectSet {
            writes_external_api: true,
            ..Default::default()
        }));
        // required.writes_external_api=false → true (チェック不要)
        assert!(self_false_write.contains(&SideEffectSet {
            writes_external_api: false,
            ..Default::default()
        }));

        // 複合: self が一部しか満たさない
        let self_partial = SideEffectSet {
            writes_external_api: true,
            sends_notification: false,
            ..Default::default()
        };
        assert!(!self_partial.contains(&SideEffectSet {
            writes_external_api: true,
            sends_notification: true,
            ..Default::default()
        }));
        assert!(self_partial.contains(&SideEffectSet {
            writes_external_api: true,
            sends_notification: false,
            ..Default::default()
        }));

        // 空の required は常に true
        let empty_required = SideEffectSet::default();
        assert!(self_false_write.contains(&empty_required));
        assert!(self_true_write.contains(&empty_required));

        // 全フィールド包含
        let self_all = SideEffectSet {
            writes_external_api: true,
            sends_notification: true,
            has_hitl_communicate: true,
            modifies_persistent_state: true,
            ..Default::default()
        };
        let all_required = SideEffectSet {
            writes_external_api: true,
            sends_notification: true,
            has_hitl_communicate: true,
            modifies_persistent_state: true,
            ..Default::default()
        };
        assert!(self_all.contains(&all_required));
    }

    // ── T8: SideEffectSet::is_safe_for_auto_approval の網羅的テスト ──

    #[test]
    fn t8_is_safe_for_auto_approval() {
        // 安全: 外部書き込みなし + 不可逆なし
        assert!(SideEffectSet {
            writes_external_api: false,
            irreversible: false,
            ..Default::default()
        }
        .is_safe_for_auto_approval());

        // 不安全: 外部 API 書き込みあり
        assert!(!SideEffectSet {
            writes_external_api: true,
            irreversible: false,
            ..Default::default()
        }
        .is_safe_for_auto_approval());

        // 不安全: 不可逆副作用あり
        assert!(!SideEffectSet {
            writes_external_api: false,
            irreversible: true,
            ..Default::default()
        }
        .is_safe_for_auto_approval());

        // 安全: 永続状態変更は sandbox 内では許容
        assert!(SideEffectSet {
            writes_external_api: false,
            modifies_persistent_state: true,
            irreversible: false,
            ..Default::default()
        }
        .is_safe_for_auto_approval());

        // 安全: 通知送信は許容
        assert!(SideEffectSet {
            writes_external_api: false,
            sends_notification: true,
            irreversible: false,
            ..Default::default()
        }
        .is_safe_for_auto_approval());
    }

    // ── T9: PlaneKind のデバッグ表現 ──

    #[test]
    fn t9_plane_kind_debug_representation() {
        assert!(!format!("{:?}", PlaneKind::Production).is_empty());
        assert!(!format!("{:?}", PlaneKind::Training).is_empty());
        assert!(!format!("{:?}", PlaneKind::SafeSandbox).is_empty());
    }

    // ── T10: 空の SideEffectSet デフォルト ──

    #[test]
    fn t10_side_effect_set_default_values() {
        let default = SideEffectSet::default();
        assert_eq!(default.writes_external_api, false);
        assert_eq!(default.sends_notification, false);
        assert_eq!(default.has_hitl_communicate, false);
        assert_eq!(default.modifies_persistent_state, false);
        assert_eq!(default.irreversible, false);
        assert_eq!(default.risk_score, 0.0);
    }

    // ── OTS-1: 副作用ベクトル空間の全軌道閉包性 ──

    /// 5 bool × 11 risk_score = 352 の全組合せを Production plane で投入し、
    /// 100% NeedsHumanReview にルーティングされることを確認する。
    #[test]
    fn ots1_production_closure() {
        let proposal = WorkflowGraph::new();
        let bool_values = [false, true];
        let risk_scores: [f32; 11] = [
            0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
        ];

        println!("=== OTS-1: Production Closure Test ===");
        println!("pattern,writes_api,sends_notif,has_hitl,modifies_db,irreversible,risk_score,outcome");

        let mut review_count = 0u32;
        let mut total_count = 0u32;

        for &wea in &bool_values {
            for &sn in &bool_values {
                for &hhc in &bool_values {
                    for &mps in &bool_values {
                        for &irr in &bool_values {
                            for &rs in &risk_scores {
                                let effects = SideEffectSet {
                                    writes_external_api: wea,
                                    sends_notification: sn,
                                    has_hitl_communicate: hhc,
                                    modifies_persistent_state: mps,
                                    irreversible: irr,
                                    risk_score: rs,
                                };
                                let result = guard_new_proposal_or_review(
                                    proposal.clone(),
                                    &effects,
                                    PlaneKind::Production,
                                    None,
                                );
                                let is_review =
                                    matches!(result, Ok(SearchOutcome::NeedsHumanReview { .. }));
                                if is_review {
                                    review_count += 1;
                                }
                                total_count += 1;
                                println!(
                                    "{},{},{},{},{},{},{},{}",
                                    total_count, wea, sn, hhc, mps, irr, rs,
                                    if is_review { "review" } else { "auto-approval" }
                                );
                            }
                        }
                    }
                }
            }
        }

        println!("\n--- Summary ---");
        println!("total={}, review={}, auto_approval={}", total_count, review_count, total_count - review_count);
        println!("closure_rate={}", review_count as f64 / total_count as f64);

        assert_eq!(
            review_count, total_count,
            "OTS-1 FAILED: expected 100% closure to NeedsHumanReview, but {}/{} were not",
            total_count - review_count, total_count
        );
        println!("=== OTS-1 PASS ===");
    }

    // ── OTS-2: Training plane 通過率とリスクスコアの関係 ──

    /// 32 パターン × 11 risk_score を Training plane で投入し、
    /// is_safe_for_auto_approval 条件と実際の auto-approval 率の一致を確認する。
    #[test]
    fn ots2_training_auto_approval_rate() {
        let proposal = WorkflowGraph::new();
        let bool_values = [false, true];
        let risk_scores: [f32; 11] = [
            0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
        ];

        println!("=== OTS-2: Training Auto-Approval Rate ===");
        println!("pattern,writes_api,sends_notif,has_hitl,modifies_db,irreversible,risk_score,safe_flag,outcome");

        let mut approval_count = 0u32;
        let mut total_count = 0u32;

        for &wea in &bool_values {
            for &sn in &bool_values {
                for &hhc in &bool_values {
                    for &mps in &bool_values {
                        for &irr in &bool_values {
                            for &rs in &risk_scores {
                                let effects = SideEffectSet {
                                    writes_external_api: wea,
                                    sends_notification: sn,
                                    has_hitl_communicate: hhc,
                                    modifies_persistent_state: mps,
                                    irreversible: irr,
                                    risk_score: rs,
                                };
                                let safe_flag = effects.is_safe_for_auto_approval();
                                let result = guard_new_proposal_or_review(
                                    proposal.clone(),
                                    &effects,
                                    PlaneKind::Training,
                                    None,
                                );

                                let is_approval =
                                    matches!(result, Ok(SearchOutcome::GenerateNew { .. }));
                                if is_approval {
                                    approval_count += 1;
                                }
                                total_count += 1;
                                println!(
                                    "{},{},{},{},{},{},{},{},{}",
                                    total_count, wea, sn, hhc, mps, irr, rs, safe_flag,
                                    if is_approval { "auto-approval" } else { "review" }
                                );
                            }
                        }
                    }
                }
            }
        }

        let approval_rate = approval_count as f64 / total_count as f64;
        println!("\n--- Summary ---");
        println!("total={}, auto_approval={}, review={}", total_count, approval_count, total_count - approval_count);
        println!("approval_rate={:.4}", approval_rate);

        // is_safe_for_auto_approval が true のパターン数 / 全パターン数
        // 5 bool 中 writes_external_api と irreversible のみが条件なので:
        // 全体 32 パターン中、wea=false AND irr=false は 8 パターン (25%)
        assert!(
            (approval_rate - 0.25).abs() < 0.001,
            "OTS-2 FAILED: expected ~25% approval rate (8/32 safe patterns), got {:.4}",
            approval_rate
        );
        println!("=== OTS-2 PASS ===");
    }

    // ── OTS-3: SafeSandbox scope 境界感度 ──

    /// 5 次元 bool space で scope 境界を sweep し、
    /// 包含判定の一致率が 1.0 であることを確認する。
    #[test]
    fn ots3_safe_sandbox_boundary_sensitivity() {
        let proposal = WorkflowGraph::new();
        let bool_values = [false, true];

        println!("=== OTS-3: SafeSandbox Boundary Sensitivity ===");

        // 各次元を 1 つずつ反転した 10 通りの scope
        let scope_defs = [
            // scope が全許可
            (
                "scope_all_allow",
                SideEffectSet {
                    writes_external_api: true,
                    sends_notification: true,
                    has_hitl_communicate: true,
                    modifies_persistent_state: true,
                    irreversible: true,
                    risk_score: 0.0,
                },
            ),
            // scope が全拒否（空）
            (
                "scope_all_deny",
                SideEffectSet::default(),
            ),
            // 各次元のみ許可
            (
                "scope_only_wea",
                SideEffectSet {
                    writes_external_api: true,
                    ..Default::default()
                },
            ),
            (
                "scope_only_sn",
                SideEffectSet {
                    sends_notification: true,
                    ..Default::default()
                },
            ),
            (
                "scope_only_hhc",
                SideEffectSet {
                    has_hitl_communicate: true,
                    ..Default::default()
                },
            ),
            (
                "scope_only_mps",
                SideEffectSet {
                    modifies_persistent_state: true,
                    ..Default::default()
                },
            ),
            (
                "scope_only_irr",
                SideEffectSet {
                    irreversible: true,
                    ..Default::default()
                },
            ),
            // 複合: 一部許可
            (
                "scope_wea_sn",
                SideEffectSet {
                    writes_external_api: true,
                    sends_notification: true,
                    ..Default::default()
                },
            ),
            (
                "scope_hhc_mps",
                SideEffectSet {
                    has_hitl_communicate: true,
                    modifies_persistent_state: true,
                    ..Default::default()
                },
            ),
            (
                "scope_no_wea",
                SideEffectSet {
                    writes_external_api: false,
                    sends_notification: true,
                    has_hitl_communicate: true,
                    modifies_persistent_state: true,
                    irreversible: true,
                    risk_score: 0.0,
                },
            ),
        ];

        let mut match_count = 0u32;
        let mut total_checks = 0u32;

        for (scope_name, scope_allowed) in &scope_defs {
            let scope = SafeSandboxScope {
                namespace: "test".into(),
                artifact_kind: scope_name.to_string(),
                allowed_side_effects: scope_allowed.clone(),
            };

            for &wea in &bool_values {
                for &sn in &bool_values {
                    for &hhc in &bool_values {
                        for &mps in &bool_values {
                            for &irr in &bool_values {
                                let effects = SideEffectSet {
                                    writes_external_api: wea,
                                    sends_notification: sn,
                                    has_hitl_communicate: hhc,
                                    modifies_persistent_state: mps,
                                    irreversible: irr,
                                    risk_score: 0.5,
                                };

                                let expect_contain = effects.contains(scope_allowed);
                                let result = guard_new_proposal_or_review(
                                    proposal.clone(),
                                    &effects,
                                    PlaneKind::SafeSandbox,
                                    Some(&scope),
                                );
                                let actual_contain =
                                    matches!(result, Ok(SearchOutcome::GenerateNew { .. }));

                                let is_match = expect_contain == actual_contain;
                                if is_match {
                                    match_count += 1;
                                }
                                total_checks += 1;

                                println!(
                                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                                    scope_name, total_checks,
                                    scope_allowed.writes_external_api,
                                    scope_allowed.sends_notification,
                                    scope_allowed.has_hitl_communicate,
                                    scope_allowed.modifies_persistent_state,
                                    scope_allowed.irreversible,
                                    wea, sn, hhc, mps, irr,
                                    expect_contain, actual_contain, is_match
                                );
                            }
                        }
                    }
                }
            }
        }

        let match_rate = match_count as f64 / total_checks as f64;
        println!("\n--- Summary ---");
        println!("total_checks={}, match_count={}", total_checks, match_count);
        println!("match_rate={:.6}", match_rate);

        assert_eq!(
            match_count, total_checks,
            "OTS-3 FAILED: expected 100% boundary match, but {}/{} mismatched",
            total_checks - match_count, total_checks
        );
        println!("=== OTS-3 PASS ===");
    }

    // ── T11: SafeSandbox scope 未指定時のエラーハンドリング ──

    #[test]
    fn t11_safe_sandbox_missing_scope_error() {
        let proposal = WorkflowGraph::new();
        let effects = SideEffectSet::default();

        let result =
            guard_new_proposal_or_review(proposal, &effects, PlaneKind::SafeSandbox, None);
        assert!(
            matches!(
                result,
                Ok(SearchOutcome::NeedsHumanReview { ref reason })
                    if reason.contains("requires a scope definition")
            ),
            "T11 failed: expected NeedsHumanReview about missing scope, got {:?}",
            result
        );
    }

    // ── T12: Training plane HITL communicate は許容される ──

    #[test]
    fn t12_training_safe_effects_has_hitl_communicate_allowed() {
        let proposal = WorkflowGraph::new();
        let effects = SideEffectSet {
            writes_external_api: false,
            has_hitl_communicate: true,
            irreversible: false,
            ..Default::default()
        };

        let result =
            guard_new_proposal_or_review(proposal, &effects, PlaneKind::Training, None);
        assert!(
            matches!(result, Ok(SearchOutcome::GenerateNew { .. })),
            "T12 failed: has_hitl_communicate should be allowed in Training, got {:?}",
            result
        );
    }
}
