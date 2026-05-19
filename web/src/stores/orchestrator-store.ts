import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useMainStore } from "src/stores/main-store";

export interface OrchestratorMessage {
  role: "user" | "assistant";
  text: string;
}

export const useOrchestratorStore = defineStore("orchestrator", () => {
  const isVisible = ref(false);
  const isRecording = ref(false);
  const isProcessing = ref(false);
  const messages = ref<OrchestratorMessage[]>([]);
  const streamingText = ref("");

  // ================================================================
  // ストリーミング制御
  // ================================================================
  //
  // 【ダミー実装（現在）】
  //   確定した全文を startStreaming() に渡し、40ms ごとに 2 文字ずつ
  //   プログレッシブリビールすることで、ストリーミングのように見せる。
  //
  // 【本番実装（リアルAIストリーミング）】
  //   下記の startStreaming / stopStreaming / streamingText はそのまま流用。
  //   変更が必要なのは trigger() 内の「★本番差し替え位置」のみ。
  //
  //   本番では orchestrator_process を呼ばず、代わりに以下の手順を踏む：
  //
  //   1. Tauri イベントリスナーを登録する
  //        const unlisten = await listen<string>('orchestrator-stream-chunk', (e) => {
  //          streamingText.value += e.payload  // ← chunk を逐次追加
  //        })
  //
  //   2. ストリーミング開始をバックエンドに要求する
  //        await invoke('orchestrator_start_stream', { text, sessionId: '' })
  //
  //   3. バックエンドが完了時に発火するイベントを受け取る
  //        listen('orchestrator-stream-end', () => {
  //          unlisten()
  //          messages.value.push({ role: 'assistant', text: streamingText.value })
  //          streamingText.value = ''
  //          isProcessing.value = false
  //        })
  //
  //   テンプレートや closeOverlay の後片付けは一切変更不要。
  // ================================================================

  let streamTimer: ReturnType<typeof setInterval> | null = null;

  const startStreaming = (fullText: string, onDone?: () => void) => {
    streamingText.value = "";
    let pos = 0;
    const CHUNK_SIZE = 3;
    const INTERVAL_MS = 40;

    streamTimer = setInterval(() => {
      pos = Math.min(pos + CHUNK_SIZE, fullText.length);
      streamingText.value = fullText.slice(0, pos);
      if (pos >= fullText.length) {
        if (streamTimer) {
          clearInterval(streamTimer);
          streamTimer = null;
        }
        onDone?.();
      }
    }, INTERVAL_MS);
  };

  const stopStreaming = () => {
    if (streamTimer) {
      clearInterval(streamTimer);
      streamTimer = null;
    }
  };

  /** ホットキートリガー（Ctrl+Option/Ctrl+Alt）の状態遷移マシン。
   *
   * Hidden → Recording (overlay表示 + 録音開始)
   * Recording → Waiting (録音停止 → テキスト取得 → orchestrator_process)
   * Waiting/Idle → Recording (録音開始: 次のターン)
   * Any → closeOverlay() (閉じるボタンでHidden)
   */
  const trigger = async () => {
    if (isProcessing.value) {
      return;
    }

    if (!isVisible.value) {
      // Hidden → Recording
      const mainStore = useMainStore();
      if (mainStore.isOverlayVisible) {
        mainStore.setIsOverlayVisible(false);
      }
      isVisible.value = true;
      isRecording.value = true;
      try {
        await invoke("create_orchestrator_session");
        await invoke("start_recording", { mode: "buffered" });
      } catch (e) {
        console.error("Orchestrator: failed to start recording", e);
        isRecording.value = false;
      }
    } else if (isRecording.value) {
      // Recording → Waiting: 録音を停止してテキストを取得
      isRecording.value = false;
      isProcessing.value = true;
      try {
        const text: string = await invoke("stop_orchestrator_recording");
        if (text.trim()) {
          messages.value.push({ role: "user", text });
          const result = (await invoke("orchestrator_process", {
            text,
            sessionId: "",
          })) as { response_text: string };

          // ★★★ 本番差し替え位置 ★★★
          //
          // 【ダミー】擬似待機時間（800ms）＋ プログレッシブリビール
          // 【本番】下記ブロック全体を削除し、代わりに orchestrator API の
          //        ストリーミングハンドリングに差し替える。
          //        詳細は startStreaming のコメントを参照。
          // ============================
          // スケルトン表示のための擬似スリープ
          await new Promise((resolve) => setTimeout(resolve, 800));
          startStreaming(result.response_text, () => {
            messages.value.push({
              role: "assistant",
              text: result.response_text,
            });
            streamingText.value = "";
            isProcessing.value = false;
          });
          // ============================
        } else {
          isProcessing.value = false;
        }
      } catch (e) {
        console.error("Orchestrator: process failed", e);
        messages.value.push({
          role: "assistant",
          text: `エラーが発生しました: ${e}`,
        });
        isProcessing.value = false;
      }
    } else {
      // Idle → Recording: 次のターンの録音を開始
      isRecording.value = true;
      try {
        await invoke("start_recording", { mode: "buffered" });
      } catch (e) {
        console.error("Orchestrator: failed to start recording", e);
        isRecording.value = false;
      }
    }
  };

  const closeOverlay = async () => {
    stopStreaming();
    streamingText.value = "";
    const wasRecording = isRecording.value;
    isVisible.value = false;
    isRecording.value = false;
    isProcessing.value = false;
    messages.value = [];
    try {
      if (wasRecording) {
        await invoke("stop_recording");
        await invoke("play_commit_sound");
      }
      await invoke("destroy_orchestrator_session");
      const mainStore = useMainStore();
      if (mainStore.pinDuringVoiceInput) {
        await invoke("toggle_always_on_top", { alwaysOnTop: false });
      }
    } catch (e) {
      console.error("Failed to destroy orchestrator session:", e);
    }
  };

  return {
    isVisible,
    isRecording,
    isProcessing,
    messages,
    streamingText,
    trigger,
    closeOverlay,
  };
});
