// Package utils は、Cuber システム全体で使用されるユーティリティ関数を提供します。
package utils

import (
	"math"
)

// ========================================
// Temporal Decay (時間減衰) 計算関数
// ========================================
// これらの関数は、知識グラフのエッジに対する時間ベースの減衰計算を統一します。
// 複数の場所（Query, Metabolism）で同じ計算式が使用されることを保証し、
// 計算ロジックの不一致による重大なバグを防止します。

// CalculateLambda は、指数減衰の減衰定数 λ を計算します。
//
// 数学的背景:
//   - λ（ラムダ）は指数減衰 N(t) = N₀ × e^(-λt) における減衰定数です。
//   - 半減期 T_{1/2} との関係: λ = ln(2) / T_{1/2}
//   - ln(2) ≈ 0.693147 は自然対数の2の値で、半減期で値が50%になるための定数です。
//
// 引数:
//   - halfLifeDays: 半減期（日数）。例: 30.0 = 30日で価値が半減
//
// 返り値:
//   - float64: 減衰定数λ（ミリ秒単位）
//
// 使用例:
//
//	lambda := CalculateLambda(30.0) // 30日の半減期
//	decay := math.Exp(-lambda * deltaT) // deltaT ミリ秒後の減衰率
func CalculateLambda(halfLifeDays float64) float64 {
	halfLifeMillis := DaysToMillis(halfLifeDays)
	if halfLifeMillis == 0 {
		return 0
	}
	return math.Ln2 / halfLifeMillis
}

// CalculateThickness は、エッジの Thickness（重要度）を計算します。
//
// Thickness は以下の式で計算されます:
//
//	Thickness = Weight × Confidence × e^(-λΔt)
//
// ここで:
//   - Weight: エッジの重み（0.0〜1.0）
//   - Confidence: 信頼度（0.0〜1.0）
//   - λ: 減衰定数（CalculateLambda で計算）
//   - Δt: 最新エッジからの経過時間（ミリ秒）= maxUnix - edgeUnix
//
// 引数:
//   - weight: エッジの重み
//   - confidence: エッジの信頼度
//   - edgeUnix: エッジの作成/更新時刻（UnixミリMs）
//   - maxUnix: グラフ内の最新エッジ時刻（UnixミリMs）
//   - lambda: 減衰定数（CalculateLambda の結果）
//
// 返り値:
//   - float64: Thickness 値（0.0〜1.0 の範囲）
func CalculateThickness(weight, confidence float64, edgeUnix, maxUnix int64, lambda float64) float64 {
	deltaT := float64(maxUnix - edgeUnix)
	if deltaT < 0 {
		deltaT = 0
	}
	decay := math.Exp(-lambda * deltaT)
	return weight * confidence * decay
}

// ========================================
// 時間単位変換関数
// ========================================

// DaysToMillis は、日数をミリ秒に変換します。
func DaysToMillis(days float64) float64 {
	return days * 24.0 * 60.0 * 60.0 * 1000.0
}

// HoursToMillis は、時間をミリ秒に変換します。
func HoursToMillis(hours float64) float64 {
	return hours * 60.0 * 60.0 * 1000.0
}
