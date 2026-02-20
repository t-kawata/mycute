//! 常駐型オーディオプレイヤー (Actor Pattern)
//!
//! 専用スレッド（Actor）を起動し、そのスレッド内で Audio OutputStream を保持し続けます。
//! 外部からは Channel 経由で再生リクエストを送信することで、
//! OutputStream の Send/Sync 制約を回避しつつ、デバイスの常駐化（低遅延再生）を実現します。

use lazy_static::lazy_static;
use rodio::{Decoder, OutputStreamBuilder, Sink};
use std::io::Cursor;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::thread;

/// 埋め込み音声データ
static READY_WAV: &[u8] = include_bytes!("../wav/piro.wav");
static COMMIT_WAV: &[u8] = include_bytes!("../wav/commit.wav");

/// 再生リクエスト
enum AudioCommand {
    PlayReady,
    PlayCommit,
}

/// 擬似無音ソース
/// デジタル的な 0 ではなく、人間には聞こえないレベル (-120dB) の
/// 極微細なゆらぎを生成することで OS やハードウェアのサスペンドを回避する。
struct PseudoSilence {
    channels: u16,
    sample_rate: u32,
    seed: u32,
}

impl PseudoSilence {
    fn new(channels: u16, sample_rate: u32) -> Self {
        Self {
            channels,
            sample_rate,
            seed: 12345,
        }
    }
}

impl Iterator for PseudoSilence {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        // シンプルな偽似乱数生成 (LCG)
        self.seed = self.seed.wrapping_mul(1103515245).wrapping_add(12345);
        // 極小振幅のノイズ。OS に「活動中」と思わせるのに十分、かつ人間には聞こえない。
        Some(((self.seed as f32 / u32::MAX as f32) - 0.5) * 0.0005)
    }
}

impl rodio::Source for PseudoSilence {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

/// オーディオスレッドへの送信チャンネルを保持する
struct AudioHandle {
    sender: Sender<AudioCommand>,
}

impl AudioHandle {
    fn new() -> Self {
        let (tx, rx) = channel();

        // 専用スレッドを起動
        thread::Builder::new()
            .name("mycute-audio-actor".to_string())
            .spawn(move || {
                run_audio_actor(rx);
            })
            .expect("Failed to spawn audio actor thread");

        Self { sender: tx }
    }

    fn send(&self, cmd: AudioCommand) {
        if let Err(e) = self.sender.send(cmd) {
            log::warn!("[Audio] Failed to send play command: {}", e);
        }
    }
}

/// オーディオActor（専用スレッド内で実行）
fn run_audio_actor(rx: Receiver<AudioCommand>) {
    log::info!("[AUDIO-TRC] Actor thread started. Attempting to open device...");
    // スレッド内でデバイスを開く（ここでのみ保持）
    let stream = match OutputStreamBuilder::open_default_stream() {
        Ok(s) => {
            log::info!("[AUDIO-TRC] Output stream opened successfully.");
            s
        }
        Err(e) => {
            log::error!(
                "[AUDIO-TRC] CRITICAL: Failed to open output stream: {}. Audio actor will exit.",
                e
            );
            return;
        }
    };

    // 現在再生中の Sink を保持する（新しい音が来たら古い方を破棄して停止するため）
    let mut current_sink: Option<Sink> = None;

    // メッセージループ
    while let Ok(cmd) = rx.recv() {
        // 1. もし現在再生中の音があれば、即座に停止して破棄する (割り込み)
        if let Some(sink) = current_sink.take() {
            log::debug!("[AUDIO-TRC] Stopping previous playback for interruption.");
            sink.stop();
        }

        match cmd {
            AudioCommand::PlayReady => {
                log::info!("[AUDIO-TRC] Received PlayReady command (Stable Mode)")
            }
            AudioCommand::PlayCommit => {
                log::info!("[AUDIO-TRC] Received PlayCommit command (Stable Mode)")
            }
        }

        let wav_data = match cmd {
            AudioCommand::PlayReady => READY_WAV,
            AudioCommand::PlayCommit => COMMIT_WAV,
        };

        let cursor = Cursor::new(wav_data);
        match Decoder::new(cursor) {
            Ok(source) => {
                use rodio::Source;
                let sample_rate = source.sample_rate();
                let channels = source.channels();

                log::debug!(
                    "[AUDIO-TRC] Decoding successful (rate={}, ch={}). Creating sink...",
                    sample_rate,
                    channels
                );
                let mixer = stream.mixer();
                match Sink::connect_new(&mixer) {
                    sink => {
                        // 1. 本編を再生
                        log::debug!("[AUDIO-TRC] Appending main audio source.");
                        sink.append(source);

                        // 2. 擬似無音ポストロール (500ms)
                        // デジタル無音ではなく擬似信号（-70dB程度）を最後に流すことで、
                        // OSのサスペンドを回避し、物理バッファを最後まで確実にフラッシュさせる。
                        log::info!("[AUDIO-TRC] Appending post-roll pseudo-silence (500ms) to flush hardware buffer...");
                        let post_silence = PseudoSilence::new(channels, sample_rate)
                            .take_duration(std::time::Duration::from_millis(500));
                        sink.append(post_silence);

                        // 非ブロッキングで保持（次のコマンドによる割り込み停止を可能にする）
                        current_sink = Some(sink);
                        log::debug!("[AUDIO-TRC] Playback chain started (with post-roll).");
                    }
                }
            }
            Err(e) => log::error!("[AUDIO-TRC] Failed to decode WAV: {}", e),
        }
    }
    log::info!("[AUDIO-TRC] Actor thread exiting normally.");
}

lazy_static! {
    static ref AUDIO_HANDLE: Mutex<AudioHandle> = Mutex::new(AudioHandle::new());
}

/// 録音準備完了音（piro.wav）を再生する
pub fn play_ready_sound() {
    log::info!("[AUDIO-TRC] play_ready_sound() called");
    if let Ok(handle) = AUDIO_HANDLE.lock() {
        handle.send(AudioCommand::PlayReady);
    }
}

/// 録音終了・コミット音（commit.wav）を再生する
pub fn play_commit_sound() {
    log::info!("[AUDIO-TRC] play_commit_sound() called");
    if let Ok(handle) = AUDIO_HANDLE.lock() {
        handle.send(AudioCommand::PlayCommit);
    }
}

/// オーディオシステムを初期化する
pub fn init() {
    // lazy_static の初期化をトリガー
    let _guard = AUDIO_HANDLE.lock();
    log::debug!("[Audio] Initialized (Actor thread spawned).");
}
