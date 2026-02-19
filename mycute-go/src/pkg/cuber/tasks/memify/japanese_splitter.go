package memify

// JapaneseSentenceEnders は、日本語の文末として扱われる文字のセットです。
var JapaneseSentenceEnders = map[rune]bool{
	'。':  true,
	'！':  true,
	'？':  true,
	'．':  true,
	'\n': true, // 改行も文の区切りとみなす
	'!':  true,
	'?':  true,
}

// isSentenceEnder は、指定された文字が文末文字かどうかを判定します。
func isSentenceEnder(r rune) bool {
	return JapaneseSentenceEnders[r]
}

// SplitAtNaturalBoundary は、指定された文字数付近の自然な境界（文末）でテキストを分割します。
//
// 引数:
//   - text: 分割対象のテキスト
//   - targetChars: 目標とする分割位置（文字数）
//   - maxSearchRangePercent: 目標位置から前後どれくらいの範囲を探索するか（%）
//
// 戻り値:
//   - int: 分割すべき位置（文字数インデックス）
func SplitAtNaturalBoundary(text string, targetChars int, maxSearchRangePercent int) int {
	runes := []rune(text)
	textLen := len(runes)

	if targetChars >= textLen {
		return textLen
	}

	// 探索範囲を計算
	rangeChars := targetChars * maxSearchRangePercent / 100
	minPos := targetChars - rangeChars
	maxPos := targetChars + rangeChars

	if minPos < 0 {
		minPos = 0
	}
	if maxPos > textLen {
		maxPos = textLen
	}

	// 1. 優先度高: 文末記号（。！？\n）を探す
	// targetChars から近い順に探すため、外側に向かって探索
	for offset := 0; offset <= rangeChars; offset++ {
		// 後ろ方向
		backPos := targetChars + offset
		if backPos < maxPos && isSentenceEnder(runes[backPos]) {
			return backPos + 1 // 記号の直後で切る
		}

		// 前方向
		frontPos := targetChars - offset
		if frontPos > minPos && isSentenceEnder(runes[frontPos]) {
			return frontPos + 1 // 記号の直後で切る
		}
	}

	// 2. 優先度中: 読点（、）やスペースを探す
	// 文末が見つからない場合のフォールバック
	secondaryBreakers := map[rune]bool{'、': true, ' ': true, '　': true, ',': true}
	for offset := 0; offset <= rangeChars; offset++ {
		backPos := targetChars + offset
		if backPos < maxPos && secondaryBreakers[runes[backPos]] {
			return backPos + 1
		}
		frontPos := targetChars - offset
		if frontPos > minPos && secondaryBreakers[runes[frontPos]] {
			return frontPos + 1
		}
	}

	// 3. 自然な境界が見つからない場合は、targetChars で強制分割
	return targetChars
}

func abs(x int) int {
	if x < 0 {
		return -x
	}
	return x
}

// SplitTextWithOverlap は、テキストを自然境界で分割し、オーバーラップを適用します。
//
// 【処理フロー】
// 1. テキストを batchSize 文字ごとに自然境界で分割
// 2. 各バッチに overlapPercent% のオーバーラップを追加
// 3. オーバーラップ部分も自然境界で調整
//
// 【引数】
// - text: 分割対象のテキスト（全チャンク結合済み）
// - batchSize: 1バッチの目標文字数
// - overlapPercent: オーバーラップ割合（0-100）
//
// 【戻り値】
// - []string: 分割されたバッチのスライス（オーバーラップ含む）
func SplitTextWithOverlap(text string, batchSize int, overlapPercent int) []string {
	runes := []rune(text)
	textLen := len(runes)

	if textLen <= batchSize {
		return []string{text} // 分割不要
	}

	var batches []string
	overlapChars := batchSize * overlapPercent / 100

	// 分割時に自然境界を探す範囲（前後20%）
	const searchRangePercent = 20

	currentStart := 0

	for currentStart < textLen {
		// このバッチの終了位置を計算
		targetEnd := currentStart + batchSize
		if targetEnd >= textLen {
			// 最後のバッチ
			batches = append(batches, string(runes[currentStart:]))
			break
		}

		// 自然境界で分割位置を調整
		searchText := string(runes[currentStart:])
		splitPos := SplitAtNaturalBoundary(searchText, batchSize, searchRangePercent)

		actualEnd := currentStart + splitPos
		if actualEnd > textLen {
			actualEnd = textLen
		}

		batches = append(batches, string(runes[currentStart:actualEnd]))

		// 次のバッチの開始位置（オーバーラップを考慮）
		nextStart := actualEnd - overlapChars
		if nextStart <= currentStart {
			// オーバーラップが大きすぎる場合は少なくとも1文字進める
			nextStart = currentStart + 1
		}

		// オーバーラップ開始位置も自然境界に調整
		if overlapChars > 0 && nextStart > 0 {
			// overlapChars分戻った位置から、前方向に自然境界を探す
			for i := nextStart; i < actualEnd; i++ {
				if isSentenceEnder(runes[i]) {
					nextStart = i + 1
					break
				}
			}
		}

		currentStart = nextStart
	}

	return batches
}
