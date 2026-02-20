<template>
  <!-- 音声認識中のテキストを表示するオーバーレイ -->
  <div 
    class="__mycute-overlay-container" 
    data-tauri-drag-region 
    :style="{ fontSize: fontSize + 'px' }"
    @mouseenter="!isHovered && (isHovered = true)"
    @mouseleave="isHovered && (isHovered = false)"
    @mousemove="!isHovered && (isHovered = true)"
  >
    <!-- テキスト表示領域: 常に最新（最下部）が見えるようにスクロール制御 -->
    <div ref="textAreaRef" class="__mycute-overlay-text-area" data-tauri-drag-region>
      <span
        v-for="(char, index) in chars"
        :key="index"
        :style="char.style"
        class="char-span"
      >{{ char.text }}</span>
    </div>
    
    <!-- フォントサイズ調整ボタン（マウスオーバーでフェードイン） -->
    <div class="__font-size-controls no-drag" :style="{ opacity: isHovered ? 1 : 0 }">
      <q-btn flat round dense icon="add" size="sm" @click="changeFontSize(1)" />
      <q-btn flat round dense icon="remove" size="sm" @click="changeFontSize(-1)" />
      <q-btn flat round dense icon="close" size="sm" @click="closeWindow" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { EVENT_STT_UPDATE, EVENT_STT_COMMIT } from 'src/consts/generated_constants';
import { get, set, KEYS } from 'src/utils/ldb';

// 文字単位のデータ構造
interface CharData {
  text: string;
  style: string; // opacity を直接指定
}

// 表示中のテキスト（文字分解後）
const chars = ref<CharData[]>([]);
// オリジナルのテキストデータ
const rawText = ref('');
// フォントサイズ (初期値 14px)
const fontSize = ref(get<number>(KEYS.FS) || 14);
// マウスホバー状態
const isHovered = ref(false);
// テキスト表示領域の DOM 参照
const textAreaRef = ref<HTMLDivElement | null>(null);

// テキスト領域を最下部にスクロールし、文字スタイルを更新する
const updateView = () => {
  const el = textAreaRef.value;
  if (!el) return;

  // 1. 文字スタイルの更新 (行判定と透明度計算)
  const spans = el.querySelectorAll('.char-span');
  if (spans.length > 0) {
    // コンテナの高さから最大表示可能行数を概算
    // line-height: 1.5 なので、1行の高さ = fontSize * 1.5
    const lineHeight = fontSize.value * 1.5;
    const containerHeight = el.clientHeight;
    // ヘッダーやパディングの影響を考慮しつつ、実際に表示可能な行数を算出
    const maxLines = Math.max(2, Math.floor(containerHeight / lineHeight));

    // 各文字の top 位置を取得
    const tops = Array.from(spans).map(s => (s as HTMLElement).offsetTop);
    
    // ユニークな top 位置（＝行の Y 座標）を収集し、降順（下から順）にソート
    // 多少の誤差(2px)を許容してグループ化
    const uniqueRowTops = tops.reduce((acc, t) => {
      if (!acc.some(exist => Math.abs(exist - t) < 5)) {
        acc.push(t);
      }
      return acc;
    }, [] as number[]).sort((a, b) => b - a); // 降順: [最下行Y, 下から2行目Y, ...]

    let needsUpdate = false;
    const newChars = chars.value.map((c, i) => {
      const top = tops[i] ?? 0;
      
      // この文字が「下から何行目」にいるか判定 (0-indexed)
      // 見つからない場合は最古の行扱い
      const rowIndex = uniqueRowTops.findIndex(rowTop => Math.abs(rowTop - top) < 5);
      const safeRowIndex = rowIndex === -1 ? uniqueRowTops.length : rowIndex;

      // 透明度計算ルール:
      // Index 0 (最下行): 1.0
      // Index 1 (下から2行目): 0.7
      // Index 2以降: 0.7 から 0 へ、maxLines に応じて減衰
      // 式: 0.7 * (1 - (rowIndex - 1) / (maxLines - 1))
      
      let opacity = 0.0;
      if (safeRowIndex === 0) {
        opacity = 1.0;
      } else if (safeRowIndex === 1) {
        opacity = 0.7;
      } else {
        // 残りの行数で 0.7 -> 0 へグラデーション
        // 分母が 0 にならないよう Math.max(1, ...)
        const progress = (safeRowIndex - 1) / Math.max(1, maxLines - 1);
        opacity = 0.7 * (1.0 - progress);
      }
      
      // 負の値や範囲外を防ぐ
      opacity = Math.max(0.1, Math.min(1.0, opacity)); 
      // 完全に0にすると消えてしまうので、あえて 0.1 程度を残すか、0にするか。
      // 「消えていく」なら 0 でOKだが、デバッグや誤読防止で 0.1 にしておくのも手。
      // ここでは要望通り「順次透明度が上がっていく（見えなくなる）」なので 0 を許容するが、
      // 完全に消えると違和感がある場合もあるため、計算結果をそのまま使う（負なら0）。
      opacity = Math.max(0, opacity);

      // 小数点3桁程度に丸める
      const opacityStr = opacity.toFixed(3);
      const newStyle = `opacity: ${opacityStr}`;
      
      if (c.style !== newStyle) {
        needsUpdate = true;
        return { ...c, style: newStyle };
      }
      return c;
    });

    if (needsUpdate) {
      chars.value = newChars;
    }
  }

  // 2. 最下部スクロール
  el.scrollTop = el.scrollHeight;
};

// 生テキストから文字配列への変換
const updateChars = (newText: string) => {
  rawText.value = newText;
  // 初期状態は不透明（styleなし）で生成し、描画後に updateView で opacity を計算・適用
  chars.value = newText.split('').map(c => ({ text: c, style: '' }));
  nextTick(updateView);
};

// リサイズ時にも再計算
const onResize = () => {
  updateView();
};

// イベントリスナーの解除関数
let unlistenUpdate: UnlistenFn | null = null;
let unlistenCommit: UnlistenFn | null = null;

onMounted(async () => {
  // STT テキスト更新イベントの受信
  unlistenUpdate = await listen<{ text: string }>(EVENT_STT_UPDATE, (event) => {
    updateChars(event.payload.text);
  });

  // コミットイベント受信時にテキストをクリア
  unlistenCommit = await listen(EVENT_STT_COMMIT, () => {
    updateChars('');
  });

  window.addEventListener('resize', onResize);
});

onUnmounted(() => {
  if (unlistenUpdate) unlistenUpdate();
  if (unlistenCommit) unlistenCommit();
  window.removeEventListener('resize', onResize);
});

// フォントサイズ変更と保存
const changeFontSize = (delta: number) => {
  fontSize.value = Math.max(8, fontSize.value + delta); // 最小値を 8px に制限
  set(KEYS.FS, fontSize.value);
  // フォントサイズ変更後も再計算
  nextTick(updateView);
};

// ウィンドウを閉じる
const closeWindow = async () => {
  isHovered.value = false;
  await getCurrentWindow().hide();
};
</script>

<style lang="scss" scoped>
@font-face {
  font-family: 'MPLUSRounded1c';
  src: url('../fonts/MPLUSRounded1c-Regular.ttf') format('truetype');
}

.__mycute-overlay-container {
  width: 100%;
  height: 100vh;
  display: flex;
  flex-direction: column;
  font-family: 'MPLUSRounded1c', sans-serif;
  font-weight: bold;
  cursor: grab;
  position: relative;
  transition: font-size 0.2s ease;
  overflow: hidden;

  // テキスト表示領域
  .__mycute-overlay-text-area {
    flex: 1;
    padding: 12px;
    overflow-y: auto;
    word-break: break-all; // 文字単位でspan区切りなので break-all が安全
    line-height: 1.5;
    
    // 中身が少ない時は中央寄せ、溢れたら通常スクロール（上部へ伸びる）を実現
    display: flex;
    flex-wrap: wrap;       // spanを折り返すために必須
    align-content: center; // 行単位で中央寄せ (flex-wrap時)
    justify-content: flex-start; // 左揃え
    
    // スクロール時の挙動補正: コンテンツが溢れたら align-content: flex-start に近い挙動になるよう制御したいが、
    // flexbox の align-content は溢れた場合に top-cropping を起こしやすい。
    // 安全のため、ここだけは "margin-top: auto" テクニックに切り替えるのが無難だが、
    // とりあえず既存の align-content で挙動を見る。
    
    // スクロールバー非表示
    scrollbar-width: none;
    -ms-overflow-style: none;
    &::-webkit-scrollbar {
      display: none;
    }
    
    // 文字単位のスタイル
    .char-span {
      transition: opacity 0.2s ease; // 透明度変化を滑らかに
      white-space: pre-wrap;  // 改行コード対応
      color: rgba(255, 255, 255, 1.0); // ベースは白
    }
  }

  .__font-size-controls {
    position: absolute;
    top: 8px;
    right: 8px;
    display: flex;
    gap: 4px;
    transition: opacity 0.3s ease;
    z-index: 10;
  }

  .no-drag {
    cursor: default;
  }
}
</style>

