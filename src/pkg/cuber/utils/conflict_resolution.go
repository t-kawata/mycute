// Package utils は、Cuberシステム全体で使用されるユーティリティ関数を提供します。
package utils

import (
	"context"
	"encoding/json"
	"fmt"
	"sort"
	"strings"

	"github.com/cloudwego/eino/components/model"
	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"go.uber.org/zap"
)

// ========================================
// Stage 1: 決定論的矛盾解決（静的ルールによる排除）
// ========================================

// ExclusiveRelationType は、排他的な関係タイプを定義します。
// これらの関係は (SourceID, RelationType) で一意であるべきです（同じソースに対して最新の1つのみ有効）。
var ExclusiveRelationType = map[string]bool{
	// === 人物の基本情報 ===
	"is_status":          true, // 状態（生存/死亡等）は一つ
	"lives_in":           true, // 現在の居住地は一つ
	"current_address":    true, // 現住所は一つ
	"resides_at":         true, // 居住地は一つ
	"works_at":           true, // 現在の勤務先は一つ
	"employed_by":        true, // 雇用主は一つ
	"current_occupation": true, // 現在の職業は一つ
	"current_job":        true, // 現在の仕事は一つ
	"married_to":         true, // 配偶者は一つ（一夫多妻制は別途考慮）
	"spouse":             true, // 配偶者
	"age":                true, // 年齢は一つ
	"current_age":        true, // 現在の年齢

	// === 組織・エンティティの属性 ===
	"capital_of":          true, // 首都は一つ
	"capital":             true, // 首都
	"headquarters_in":     true, // 本社所在地は一つ
	"headquartered_at":    true, // 本拠地
	"ceo_of":              true, // CEOは通常一人
	"current_ceo":         true, // 現在のCEO
	"president_of":        true, // 社長/大統領は一人
	"current_president":   true, // 現在の社長/大統領
	"leader_of":           true, // リーダーは一人
	"current_leader":      true, // 現在のリーダー
	"chairperson_of":      true, // 議長は一人
	"current_chairperson": true, // 現在の議長

	// === 技術・ソフトウェア ===
	"current_version":           true, // 現バージョンは一つ
	"latest_version":            true, // 最新バージョンは一つ
	"version":                   true, // バージョン
	"released_on":               true, // リリース日は一つ（最新）
	"last_updated":              true, // 最終更新日は一つ
	"deprecated_by":             true, // 非推奨化元は一つ
	"replaced_by":               true, // 置き換え先は一つ
	"superseded_by":             true, // 後継は一つ
	"default_value":             true, // デフォルト値は一つ
	"current_value":             true, // 現在値は一つ
	"primary_language":          true, // 主言語は一つ
	"main_programming_language": true, // 主要プログラミング言語

	// === 場所・地理 ===
	"located_in":         true, // 所在地（最も具体的な1つ）
	"located_at":         true, // 位置
	"country":            true, // 所属国は一つ
	"belongs_to_country": true, // 所属国
	"timezone":           true, // タイムゾーンは一つ
	"in_timezone":        true, // タイムゾーン

	// === 関係・階層 ===
	"parent_of":         true, // 親（生物学的には2人だが、組織では1つ）
	"parent_company":    true, // 親会社は一つ
	"owned_by":          true, // 所有者は一つ（主要）
	"primary_owner":     true, // 主要所有者
	"reports_to":        true, // 報告先（直属上司）は一人
	"direct_supervisor": true, // 直属上司
	"successor_of":      true, // 後継者は一人
	"predecessor_of":    true, // 前任者は一人
	"replaces":          true, // 置き換え対象は一つ

	// === 数値・測定 ===
	"population":         true, // 人口は一つ（最新）
	"current_population": true, // 現在の人口
	"area":               true, // 面積は一つ
	"size":               true, // サイズは一つ
	"height":             true, // 高さは一つ
	"weight":             true, // 重さは一つ
	"price":              true, // 価格は一つ（最新）
	"current_price":      true, // 現在価格
	"market_cap":         true, // 時価総額は一つ
	"revenue":            true, // 収益は一つ（最新）
	"annual_revenue":     true, // 年間収益

	// === 日時・期間 ===
	"founded_on":      true, // 設立日は一つ
	"established_on":  true, // 設立日
	"birth_date":      true, // 誕生日は一つ
	"born_on":         true, // 誕生日
	"death_date":      true, // 死亡日は一つ
	"died_on":         true, // 死亡日
	"start_date":      true, // 開始日は一つ（最新のイベント）
	"end_date":        true, // 終了日は一つ
	"expiration_date": true, // 有効期限は一つ
	"valid_until":     true, // 有効期限

	// === 識別子 ===
	"official_name":    true, // 正式名称は一つ
	"legal_name":       true, // 法的名称は一つ
	"primary_email":    true, // 主要メールアドレスは一つ
	"main_email":       true, // メインメール
	"primary_phone":    true, // 主要電話番号は一つ
	"main_phone":       true, // メイン電話
	"website":          true, // 公式ウェブサイトは一つ
	"official_website": true, // 公式サイト
	"primary_url":      true, // 主要URLは一つ
}

// ScoredTriple は、Thickness スコアを持つトリプルを表します。
type ScoredTriple struct {
	Triple    *storage.Triple
	Thickness float64
}

// ConflictGroup は、矛盾する可能性のあるエッジのグループを表します。
type ConflictGroup struct {
	SourceID     string
	RelationType string
	Edges        []ScoredTriple
}

// Stage1ConflictResolution は、決定論的ルールに基づいて矛盾を解決します。
// 排他的関係リストに含まれる関係タイプについて、同一 (SourceID, RelationType) ペアの中で
// 最高 Thickness スコアのエッジのみを残します。
//
// 引数:
//   - triples: スコア付きトリプルのリスト
//   - logger: ロガー
//
// 戻り値:
//   - resolved: Stage 1 で解決されたトリプルのリスト
//   - discarded: Stage 1 で矛盾と判断され、削除対象となったトリプルのリスト
//   - remainingConflicts: Stage 2 で解決が必要な矛盾グループ
func Stage1ConflictResolution(triples []ScoredTriple, logger *zap.Logger) (resolved []ScoredTriple, discarded []ScoredTriple, remainingConflicts []ConflictGroup) {
	if len(triples) == 0 {
		return triples, nil, nil
	}

	// (SourceID, RelationType) でグループ化
	groupMap := make(map[string][]ScoredTriple)
	for _, st := range triples {
		key := st.Triple.Edge.SourceID + "|" + st.Triple.Edge.Type
		if _, ok := groupMap[key]; !ok {
			groupMap[key] = []ScoredTriple{}
		}
		groupMap[key] = append(groupMap[key], st)
	}

	resolved = make([]ScoredTriple, 0, len(triples))
	discarded = make([]ScoredTriple, 0)
	remainingConflicts = make([]ConflictGroup, 0)

	for key, group := range groupMap {
		parts := strings.SplitN(key, "|", 2)
		sourceID := parts[0]
		relationType := parts[1]

		if len(group) == 1 {
			// 一つのソースが特定の関係性にて一つのエッジしか持たない場合は矛盾なし
			resolved = append(resolved, group[0])
			continue
		}

		// 一つのソースが特定の関係性にて複数のエッジを持つ場合
		if ExclusiveRelationType[relationType] { // ステージ1の明示的排他対象関係だった場合
			// 最高スコアのエッジのみを残す
			var best ScoredTriple
			for _, st := range group {
				if st.Thickness > best.Thickness {
					best = st
				}
			}
			resolved = append(resolved, best)
			// best 以外のエッジを discarded に追加
			for _, st := range group {
				if st.Triple.Edge.TargetID != best.Triple.Edge.TargetID {
					discarded = append(discarded, st)
				}
			}
			LogDebug(logger, "Stage1: Resolved exclusive conflict",
				zap.String("source", sourceID),
				zap.String("relation", relationType),
				zap.Int("candidates", len(group)),
				zap.String("selected_target", best.Triple.Edge.TargetID),
				zap.Float64("thickness", best.Thickness))
		} else { // ステージ1の明示的排他対象関係ではなかった場合
			targetMap := make(map[string]ScoredTriple)
			for _, st := range group {
				targetKey := st.Triple.Edge.TargetID
				if existing, ok := targetMap[targetKey]; ok {
					// 同一ターゲットに対する同じ関係のトリプルがあったら
					// 最高スコアのものだけにする
					if st.Thickness > existing.Thickness {
						discarded = append(discarded, existing) // 低スコアのものを破棄
						targetMap[targetKey] = st
					} else {
						discarded = append(discarded, st) // 今のエッジの方が低スコアなら破棄
					}
				} else {
					targetMap[targetKey] = st
				}
			}

			// 特定のソースから特定の関係で複数のユニークターゲットに対するトリプルを集める
			uniqueTargets := make([]ScoredTriple, 0, len(targetMap))
			for _, st := range targetMap {
				uniqueTargets = append(uniqueTargets, st)
			}

			// 異なるターゲットへの同じ関係での複数エッジがある場合は Stage 2 へ
			if len(uniqueTargets) > 1 {
				// 潜在的な矛盾（複業、複数スキル等を除く一般的な矛盾）
				// → Stage 2 で LLM が判定
				remainingConflicts = append(remainingConflicts, ConflictGroup{
					SourceID:     sourceID,
					RelationType: relationType,
					Edges:        uniqueTargets,
				})
				// とりあえず全て resolved に追加（必要に応じて Stage 2 で絞り込み）
				resolved = append(resolved, uniqueTargets...)
			} else {
				resolved = append(resolved, uniqueTargets...)
			}
		}
	}

	// Thickness 降順でソート
	sort.Slice(resolved, func(i, j int) bool {
		return resolved[i].Thickness > resolved[j].Thickness
	})

	return resolved, discarded, remainingConflicts
}

// ========================================
// Stage 2: LLM による矛盾解決
// ========================================

// ConflictEdgeInfo は、LLM に渡す矛盾エッジの情報です。
type ConflictEdgeInfo struct {
	SourceID     string  `json:"source_id"`
	RelationType string  `json:"relation_type"`
	TargetID     string  `json:"target_id"`
	Score        float64 `json:"score"`
	Unix         int64   `json:"unix"`
}

// LLMConflictResolution は、Stage 2 のレスポンス構造体です。
type LLMConflictResolution struct {
	Resolution []struct {
		SourceID     string `json:"source_id"`
		RelationType string `json:"relation_type"`
		TargetID     string `json:"target_id"`
		Reason       string `json:"reason"`
	} `json:"resolution"`
	Discarded []struct {
		SourceID     string `json:"source_id"`
		RelationType string `json:"relation_type"`
		TargetID     string `json:"target_id"`
		Reason       string `json:"reason"`
	} `json:"discarded"`
}

// Stage2ConflictResolution は、LLM を使用して矛盾を解決します。
// Stage 1 で解決できなかった文脈依存の矛盾について、LLM の判断を仰ぎ、不要なエッジを削ぎ落とします。
//
// 引数:
//   - ctx: コンテキスト
//   - llm: LLM インスタンス
//   - modelName: モデル名
//   - triples: Stage 1 で解決されたトリプルリストへのポインタ。Stage 2 の判断により、インプレースで削除されます。
//   - conflicts: 矛盾グループのリスト
//   - isEn: 英語出力フラグ
//   - logger: ロガー
//
// 戻り値:
//   - discarded: Stage 2 の判断により削除対象となったトリプルのリスト
//   - usage: トークン使用量
//   - error: エラー
func Stage2ConflictResolution(
	ctx context.Context,
	llm model.ToolCallingChatModel,
	modelName string,
	triples *[]ScoredTriple,
	conflicts []ConflictGroup,
	isEn bool,
	logger *zap.Logger,
) (discarded []ScoredTriple, usage types.TokenUsage, err error) {
	// 矛盾がなければ何もしない
	if len(conflicts) == 0 {
		return nil, usage, nil
	}

	// プロンプトの選択
	var systemPrompt string
	if isEn {
		systemPrompt = prompts.ARBITRATE_CONFLICT_SYSTEM_EN_PROMPT
	} else {
		systemPrompt = prompts.ARBITRATE_CONFLICT_SYSTEM_JA_PROMPT
	}

	// 矛盾情報を JSON 形式で構築
	conflictInfos := make([]ConflictEdgeInfo, 0)
	for _, cg := range conflicts {
		for _, st := range cg.Edges {
			conflictInfos = append(conflictInfos, ConflictEdgeInfo{
				SourceID:     cg.SourceID,
				RelationType: cg.RelationType,
				TargetID:     st.Triple.Edge.TargetID,
				Score:        st.Thickness,
				Unix:         st.Triple.Edge.Unix,
			})
		}
	}

	conflictDataJSON, err := json.MarshalIndent(conflictInfos, "", "  ")
	if err != nil {
		return nil, usage, fmt.Errorf("Stage2: Failed to marshal conflict data: %w", err)
	}

	// メッセージ構築（SystemプロンプトとUserプロンプトを分離）
	userPrompt := fmt.Sprintf(prompts.ARBITRATE_CONFLICT_USER_PROMPT, string(conflictDataJSON))

	// LLM 呼び出し（Eino Callback によるトークン使用量自動集計）
	responseContent, usage, err := GenerateWithUsage(ctx, llm, modelName, systemPrompt, userPrompt)
	if err != nil {
		return nil, usage, fmt.Errorf("Stage2: LLM generation failed: %w", err)
	}

	// レスポンスを解析
	var resolution LLMConflictResolution
	// JSON 部分を抽出（```json ... ``` 内）
	jsonStart := strings.Index(responseContent, "{")
	jsonEnd := strings.LastIndex(responseContent, "}")
	if jsonStart >= 0 && jsonEnd > jsonStart {
		responseContent = responseContent[jsonStart : jsonEnd+1]
	}

	if err := json.Unmarshal([]byte(responseContent), &resolution); err != nil {
		LogWarn(logger, "Stage2: Failed to parse LLM response, keeping all edges", zap.Error(err), zap.String("response", responseContent))
		return nil, usage, nil
	}

	// Discarded リストを検索用のマップにする
	discardedMap := make(map[string]bool)
	for _, d := range resolution.Discarded {
		key := d.SourceID + "|" + d.RelationType + "|" + d.TargetID
		discardedMap[key] = true
	}

	// インプレース・フィルタリング
	n := 0
	discarded = make([]ScoredTriple, 0)
	for _, st := range *triples {
		key := st.Triple.Edge.SourceID + "|" + st.Triple.Edge.Type + "|" + st.Triple.Edge.TargetID
		if !discardedMap[key] {
			(*triples)[n] = st
			n++
		} else {
			discarded = append(discarded, st)
			LogDebug(logger, "Stage2: Edge discarded by LLM",
				zap.String("source", st.Triple.Edge.SourceID),
				zap.String("relation", st.Triple.Edge.Type),
				zap.String("target", st.Triple.Edge.TargetID))
		}
	}
	*triples = (*triples)[:n]

	// 解決されたログも一応出力
	for _, r := range resolution.Resolution {
		LogDebug(logger, "Stage2: LLM resolved/kept edge",
			zap.String("source", r.SourceID),
			zap.String("relation", r.RelationType),
			zap.String("target", r.TargetID),
			zap.String("reason", r.Reason))
	}

	return discarded, usage, nil
}
