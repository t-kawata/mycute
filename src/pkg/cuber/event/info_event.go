package event

import (
	"sync/atomic"

	"github.com/t-kawata/mycute/lib/eventbus"
)

const (
	EVENT_INFO_CONFLICT_RESOLUTION_1_START EventName = "INFO_CONFLICT_RESOLUTION_1_START" // 矛盾解決（Stage 1）が開始された時に発火する
	EVENT_INFO_CONFLICT_RESOLUTION_1_END   EventName = "INFO_CONFLICT_RESOLUTION_1_END"   // 矛盾解決（Stage 1）が完了した時に発火する
	EVENT_INFO_CONFLICT_RESOLUTION_2_START EventName = "INFO_CONFLICT_RESOLUTION_2_START" // 矛盾解決（Stage 2）が開始された時に発火する
	EVENT_INFO_CONFLICT_RESOLUTION_2_END   EventName = "INFO_CONFLICT_RESOLUTION_2_END"   // 矛盾解決（Stage 2）が完了した時に発火する
	EVENT_INFO_CONFLICT_DISCARDED          EventName = "INFO_CONFLICT_DISCARDED"          // 矛盾解決によりエッジが破棄された時に発火する
)

type InfoConflictResolution1StartPayload struct {
	BasePayload
	BeforeTriplesCount int
}

type InfoConflictResolution1EndPayload struct {
	BasePayload
	BeforeTriplesCount int
	AfterTriplesCount  int
}

type InfoConflictResolution2StartPayload struct {
	BasePayload
	BeforeTriplesCount int
}

type InfoConflictResolution2EndPayload struct {
	BasePayload
	BeforeTriplesCount int
	AfterTriplesCount  int
}

type InfoConflictDiscardedPayload struct {
	BasePayload
	SourceID     string
	RelationType string
	TargetID     string
	Stage        int    // 1 or 2
	Reason       string // 破棄された理由
}

// RegisterInfoStreamer subscribes to neutral informational events and forwards them to the provided channel.
func RegisterInfoStreamer(eb *eventbus.EventBus, ch chan<- StreamEvent) {
	send := func(name EventName, p any) {
		ch <- StreamEvent{EventName: name, Payload: p}
	}
	eventbus.Subscribe(eb, string(EVENT_INFO_CONFLICT_RESOLUTION_1_START), func(p InfoConflictResolution1StartPayload) error {
		send(EVENT_INFO_CONFLICT_RESOLUTION_1_START, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_INFO_CONFLICT_RESOLUTION_1_END), func(p InfoConflictResolution1EndPayload) error {
		send(EVENT_INFO_CONFLICT_RESOLUTION_1_END, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_INFO_CONFLICT_RESOLUTION_2_START), func(p InfoConflictResolution2StartPayload) error {
		send(EVENT_INFO_CONFLICT_RESOLUTION_2_START, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_INFO_CONFLICT_RESOLUTION_2_END), func(p InfoConflictResolution2EndPayload) error {
		send(EVENT_INFO_CONFLICT_RESOLUTION_2_END, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_INFO_CONFLICT_DISCARDED), func(p InfoConflictDiscardedPayload) error {
		send(EVENT_INFO_CONFLICT_DISCARDED, p)
		return nil
	})
}

var stage1ReasonCounter uint64

// GetStage1ExclusiveReason returns one of 25 variations of the exclusive conflict reason.
func GetStage1ExclusiveReason(isEn bool) string {
	idx := atomic.AddUint64(&stage1ReasonCounter, 1) % 25
	if isEn {
		return stage1ExclusiveReasonsEn[idx]
	}
	return stage1ExclusiveReasonsJa[idx]
}

// GetStage1DuplicateReason returns one of 25 variations of the duplicate triple reason.
func GetStage1DuplicateReason(isEn bool) string {
	idx := atomic.AddUint64(&stage1ReasonCounter, 1) % 25
	if isEn {
		return stage1DuplicateReasonsEn[idx]
	}
	return stage1DuplicateReasonsJa[idx]
}

var stage1ExclusiveReasonsEn = [25]string{
	"Higher thickness score was selected for this exclusive relation type.",
	"Prioritized the edge with better confidence score for this unique relation.",
	"This relation type is exclusive; the most reliable data was kept.",
	"Maintaining consistency: only the highest-scored edge is preserved here.",
	"Exclusive constraint applied: keeping the most prominent edge.",
	"Pruned lower-scored candidates for this unique relationship type.",
	"Consistency check: selected the edge with superior thickness value.",
	"Only one relationship of this type is allowed; highest thickness wins.",
	"Filtering exclusive conflicts: preserved the most certain link.",
	"The relation requires uniqueness; kept the edge with maximum weight.",
	"Discarded alternative edges for this exclusive property.",
	"Ensuring data integrity: only the best candidate was retained.",
	"Exclusive relation resolution: keeping the strongest semantic connection.",
	"Conflict resolved: prioritized the most recent and high-scored edge.",
	"Unique constraint enforced: lower thickness edges were eliminated.",
	"Pruning redundant facts: keeping the most validated relationship.",
	"Logical consistency: selected the dominant edge for this exclusive link.",
	"Data cleanup: removed less probable candidates for this unique type.",
	"Refining knowledge graph: only the highest-confidence edge survives.",
	"Exclusive property handling: preserved the link with best thickness.",
	"Maintaining single-truth: only the top-scored edge is included.",
	"Resolution logic: kept the most significant edge for this exclusive type.",
	"Exclusivity enforced: removing weaker contradictory relationship.",
	"Knowledge reliability: keeping the edge with best validation score.",
	"Structural integrity: prioritized the most robust unique relation.",
}

var stage1ExclusiveReasonsJa = [25]string{
	"同一の排他的関係タイプにおいて、より高い Thickness スコアを持つエッジが選択されました。",
	"排他的な関係性において、最も確信度の高い（Thicknessが高い）データが優先されました。",
	"この関係タイプは唯一性が求められるため、スコアの低い候補を排除しました。",
	"一貫性維持のため、このユニークな関係では最高スコアのエッジのみを保持しました。",
	"排他ルールの適用：最も重要度の高いエッジを残し、他を破棄しました。",
	"唯一の関係性において、最大の Thickness を有するものを採用しました。",
	"整合性チェックの結果、より優れた Thickness 値を持つエッジを選択しました。",
	"この種類の関係は1つのみ許可されるため、最高スコアのものが選ばれました。",
	"排他矛盾の解消：最も確実な接続を優先して維持しました。",
	"関係のユニーク制約により、最大重みを持つエッジを抽出しました。",
	"この排他的プロパティにおいて、代替となるエッジを破棄しました。",
	"データ整合性の確保：最良の候補のみがグラフに残されました。",
	"排他関係の解決：最も強力な意味的繋がりを保持しました。",
	"矛盾解消：最も最新かつ高スコアのエッジが優先されました。",
	"ユニーク制約の適用の結果、Thickness の低いエッジが排除されました。",
	"冗長な事実の整理：最も検証された関係性を維持しました。",
	"論理的一貫性：この排他リンクにおいて支配的なエッジを選択しました。",
	"データクリーンアップ：この唯一のタイプに対して可能性の低い候補を削除しました。",
	"知識グラフの洗練：最も信頼度の高いエッジのみが存続しました。",
	"排他プロパティの処理：最良の Thickness を持つリンクを保護しました。",
	"「唯一の事実」の維持：トップスコアのエッジのみが含まれました。",
	"解決ロジック：この排他的タイプに対して最も重要なエッジを維持しました。",
	"排他性の強制：より弱い矛盾した関係性を除去しました。",
	"知識の信頼性：最良の検証スコアを持つエッジを保持しました。",
	"構造的整合性：最も堅牢な固有の関係を優先しました。",
}

var stage1DuplicateReasonsEn = [25]string{
	"A duplicate triple with a higher thickness score was found.",
	"Removed duplicate entry in favor of a version with higher confidence.",
	"Multiple identical triples detected; keeping the one with maximum thickness.",
	"De-duplicating: prioritized the most reliable instance of this relationship.",
	"Filtering identical facts: preserved the edge with superior validation.",
	"Data normalization: only the most robust copy of this triple is kept.",
	"Merged duplicate information by retaining the highest-scored edge.",
	"Redundant triple detected: selecting the version with best thickness.",
	"Cleanup of identical edges: kept the one with the highest weight.",
	"Knowledge synthesis: unified duplicates by selecting the top score.",
	"Pruned redundant link: a stronger version already exist.",
	"Duplicate fact resolution: highest confidence edge was preserved.",
	"Refining graph: removed lower-scored identical relationships.",
	"Ensuring uniqueness: kept the most validated instance of this triple.",
	"Eliminated less reliable duplicate of this relationship.",
	"Fact check: keeping the strongest evidence for this duplicate triple.",
	"Integrity preservation: selected the most certain duplicate.",
	"Duplicate removal: lower thickness copies were discarded.",
	"Selecting the primary evidence among identical triples.",
	"Highest thickness edge retained among redundant entries.",
	"Logic consistency: filtered out less probable duplicate facts.",
	"Knowledge optimization: kept the best version of this triple.",
	"Structural cleanup: removed weaker duplicate of this edge.",
	"Validation check: highest thickness duplicate survives.",
	"Consistent knowledge: selected the most authoritative duplicate.",
}

var stage1DuplicateReasonsJa = [25]string{
	"同一のトリプルの中で、より高い Thickness を持つものが優先されました。",
	"重複データが検出され、スコアの低いエッジが排除されました。",
	"同じ内容の複数の候補から、最も信頼されるエッジを維持しました。",
	"同一関係の重複解消：最も確信度の高いインスタンスを優先しました。",
	"同一事実のフィルタリング：より優れた検証スコアを持つエッジを保護しました。",
	"データの正規化：このトリプルの最も堅牢なコピーのみが保持されます。",
	"最高スコアのエッジを残すことで重複情報を統合しました。",
	"冗長なトリプルを検出：Thickness の最も良いバージョンを選択しました。",
	"同一エッジのクリーンアップ：最も重みの大きいものを残しました。",
	"知識の統合：トップスコアを選択することで重複を統一しました。",
	"冗長なリンクの排除：より協力なバージョンが既に存在します。",
	"重複事実の解決：最高信頼度のエッジが維持されました。",
	"グラフの洗練：スコアの低い同一の関係を削除しました。",
	"唯一性の確保：このトリプルの最も検証されたインスタンスを保持しました。",
	"この関係性のより信頼性の低い重複を排除しました。",
	"ファクトチェック：この重複トリプルに対する最強の証拠を維持しました。",
	"整合性の維持：最も確実な重複データを選択しました。",
	"重複削除：Thickness の低いコピーが破棄されました。",
	"同一トリプルの中から主要な証拠を選択しました。",
	"冗長なエントリの中で最高 Thickness のエッジが保持されました。",
	"論理的一貫性：可能性の低い重複事実をフィルタリングしました。",
	"知識の最適化：このトリプルの最良のバージョンを維持しました。",
	"構造的クリーンアップ：このエッジのより弱い重複を削除しました。",
	"検証チェック：最高 Thickness の重複データが存続しました。",
	"一貫した知識：最も権威のある重複データを選択しました。",
}
