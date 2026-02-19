# 元のメッセージ及び情報

OKです。sherpa02の音声認識を改めてテストしました。

マイクから喋ったセリフは以下です。
```
はい、お世話になります。川田です。今日はいい天気ですね。こんな日は、お散歩に行きたくなりますね。
```

この時、テキスト入力のカーソルの位置に入力された内容は以下の通りです。
```
はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
```

そしてログに出力された結果が以下の通りです。
```
26-01-14_14:40:57 mycute.stt.recognizer     [DEBUG] Speech recognition started (engine: Sherpa02)
26-01-14_14:40:58 tools.pseudo_asr_streamer [DEBUG] Speech START
26-01-14_14:40:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.36s, queue_len=0, current_chunk_len=5760
26-01-14_14:40:58 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:40:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_14:40:58 tools.pseudo_asr_streamer [DEBUG] Result: はい
26-01-14_14:40:58 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はい
26-01-14_14:40:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.62s, queue_len=0, current_chunk_len=9856
26-01-14_14:40:58 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:40:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_14:40:58 tools.pseudo_asr_streamer [DEBUG] Result: はい
26-01-14_14:40:58 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はい
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.90s, queue_len=0, current_chunk_len=14464
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "あ"
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はいあ"
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] Result: はいあ
26-01-14_14:40:59 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあ
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.16s, queue_len=0, current_chunk_len=18560
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はいあはい"
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] Result: はいあはい
26-01-14_14:40:59 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはい
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.45s, queue_len=0, current_chunk_len=23168
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お世話になります"
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はいあはいお世話になります"
26-01-14_14:40:59 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。
26-01-14_14:40:59 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.77s, queue_len=0, current_chunk_len=28288
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お世話になります"
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。
26-01-14_14:41:00 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.06s, queue_len=0, current_chunk_len=32896
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お世話になります川田です"
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: "田です"
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "田です"
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。
26-01-14_14:41:00 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.25s, queue_len=0, current_chunk_len=35968
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お世話になります河田です"
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。
26-01-14_14:41:00 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.57s, queue_len=0, current_chunk_len=41088
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お世話になります河田です"
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:00 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。
26-01-14_14:41:00 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.76s, queue_len=0, current_chunk_len=44160
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お世話になります川田です"
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。
26-01-14_14:41:01 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Speech END
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Chunk detected: ID=0, duration=2.95s
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.95s, queue_len=1, current_chunk_len=0
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お世話になります河田です"
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。
26-01-14_14:41:01 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.95s, queue_len=1, current_chunk_len=0
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お世話になります河田です"
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。
26-01-14_14:41:01 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Speech START
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.28s, queue_len=1, current_chunk_len=5248
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はい
26-01-14_14:41:01 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はい
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.47s, queue_len=1, current_chunk_len=8320
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_14:41:01 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はい
26-01-14_14:41:01 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はい
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.70s, queue_len=1, current_chunk_len=11904
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はい
26-01-14_14:41:02 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はい
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.92s, queue_len=1, current_chunk_len=15488
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お世話になります"
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はいお世話になります"
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。
26-01-14_14:41:02 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.22s, queue_len=0, current_chunk_len=19584
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "天気ですね"
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "天気ですね"
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。
26-01-14_14:41:02 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.48s, queue_len=0, current_chunk_len=23680
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "今日はいい天気ですね"
26-01-14_14:41:02 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:02 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.80s, queue_len=0, current_chunk_len=28800
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:03 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] Speech END
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Chunk detected: ID=1, duration=1.93s
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:03 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:03 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:03 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:04 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:04 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:04 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:04 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:04 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:05 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:05 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:05 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:05 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:05 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:06 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:06 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:06 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:06 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.93s, queue_len=1, current_chunk_len=0
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いい天気ですね"
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:07 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] Speech START
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.19s, queue_len=1, current_chunk_len=4224
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気です"
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "今日はいい天気です"
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。
26-01-14_14:41:07 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.49s, queue_len=0, current_chunk_len=7808
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "は"
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "は"
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。は
26-01-14_14:41:07 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。は
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.74s, queue_len=0, current_chunk_len=11904
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_14:41:07 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はい
26-01-14_14:41:07 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はい
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.00s, queue_len=0, current_chunk_len=16000
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はい
26-01-14_14:41:08 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はい
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.29s, queue_len=0, current_chunk_len=20608
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はい
26-01-14_14:41:08 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はい
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.58s, queue_len=0, current_chunk_len=25216
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はい
26-01-14_14:41:08 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はい
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.86s, queue_len=0, current_chunk_len=29824
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_14:41:08 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はい
26-01-14_14:41:08 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はい
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.18s, queue_len=0, current_chunk_len=34944
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お散歩に行きたくなります"
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はいお散歩に行きたくなります"
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。
26-01-14_14:41:09 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.38s, queue_len=0, current_chunk_len=38016
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "お散歩に行きたくなりますね"
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。
26-01-14_14:41:09 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.66s, queue_len=0, current_chunk_len=42624
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんな日はお散歩に行きたくなりますね"
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんな日はお散歩に行きたくなりますね"
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:09 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.89s, queue_len=0, current_chunk_len=46208
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんな日はお散歩に行きたくなりますね"
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:09 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:09 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] Speech END
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Chunk detected: ID=2, duration=2.98s
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.98s, queue_len=1, current_chunk_len=0
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんな日はお散歩に行きたくなりますね"
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:10 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.98s, queue_len=1, current_chunk_len=0
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんな日はお散歩に行きたくなりますね"
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:10 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.98s, queue_len=1, current_chunk_len=0
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんな日はお散歩に行きたくなりますね"
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:10 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.98s, queue_len=1, current_chunk_len=0
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんな日はお散歩に行きたくなりますね"
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:10 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:10 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.98s, queue_len=1, current_chunk_len=0
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんな日はお散歩に行きたくなりますね"
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:11 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.98s, queue_len=1, current_chunk_len=0
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんな日はお散歩に行きたくなりますね"
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] Result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:11 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: はいあはいお世話になります。田です。はいお世話になります。天気ですね。今日はいい天気ですね。今日はいい天気です。はいお散歩に行きたくなります。こんな日は、お散歩に行きたくなりますね。
26-01-14_14:41:11 tools.pseudo_asr_streamer [DEBUG] Recognition stopped
26-01-14_14:41:11 mycute.stt.recognizer     [DEBUG] Speech recognition stopped (Sherpa02)
^C26-01-14_14:41:11 mycute.mode.vp.main_of_vp [DEBUG] Received Ctrl+C, cleaning up...
26-01-14_14:41:11 mycute.mode.vp.main_of_vp [DEBUG] Cleanup complete. Exiting.
```

音声認識の速度も、実際にモデルによって行われている音声認識の精度そのものも、おそらく全く問題ないほどに高いということが見て取れます。しかし、最終出力前の処理に問題があることにより、実際に出力される内容は、実用的なレベルには達していません。まずは、上記の情報より、音声認識結果を最終出力に加工するための各処理ステップの中にどのような問題がある可能性があるかを網羅的にリストして私に説明してください。説明だけを行いまだ作業には入らないでください。

# 問題点の洗い出し

1. 重なり検知（Overlap Detection）の不正確さと「表記揺れ」への弱さ
ログを見ると、お世話になります川田です が お世話になります河田です（河 vs 川）と変化した際に、重なり検知が失敗している形跡があります。

問題点: 
find_overlap_len
 が完全一致（または厳密な前方一致）に依存している場合、漢字の変換候補が変わるだけで「全く別の新しいテキスト」と判定されてしまいます。
具体例:
前のチャンクの末尾：河田です
次のウィンドウの先頭：お世話になります川田です
ここで 河 と 川 が違うため、お世話になります 以降が重複して結合されてしまった可能性があります。また、田です だけが残る（川 が削られる）といった奇妙な挙動も、不完全な重なり検知が原因です。
2. チャンク境界での「多重コミット」現象
VAD（無音検知）によって Speech END -> Speech START が短時間に繰り返されています。この境界での状態遷移がスムーズではありません。

問題点: 
PseudoAsrStreamer
 は session_history（確定済み）と active_text（現在認識中）を組み合わせて display_text を作ります。
具体例: ログの Result: を見ると、はいあはいお世話になります。 という履歴に、さらに はいお世話になります。 が追加されています。これは、新しく始まった認識ウィンドウの中に、既に session_history に入れたはずの古い音声データがまだ残っており、それが「新しい発話」として再度認識・追加されてしまっている（二重計上）ことを示唆しています。
3. input_diff（差分入力）と累積テキストの不整合
main_of_vp
 側で行っている「差分入力」のロジックが、ストリーマーから送られてくる累積テキストの「巻き戻り」や「書き換え」に対応しきれていない可能性があります。

問題点: 認識が進むにつれて、途中の漢字が変わったり句読点が打たれたりして累積テキストが「数文字分短くなる」または「数文字前が書き換わる」ことがあります。
具体例: KeyboardInjector::input_diff は、新旧のテキストを比較して足りない分を打ち込みますが、「既に打ち込んだ文字が実は間違っていた（変換が変わった）」場合、適切にバックスペースで消去して打ち直す必要があります。この挙動が不完全だと、古い変換を残したまま新しい変換が追記され、結果として「はいあはいお世話...」のようなカオスな入力になります。
4. VAD（発話検知）の感度とチャンク化の粒度
Speech END と Speech START が非常に頻繁に発生しています（0.3秒〜0.5秒程度の短い沈黙で切れている）。

問題点: チャンクが細切れになりすぎると、上記（1, 2）の「境界問題」が発生する回数が増え、エラーが累積しやすくなります。もう少し「一つの発話」として粘り強く待つ（silence toleranceを調整する）か、あるいはチャンクを跨いでも過去の音響データを引きずらないような厳密なフラッシュが必要です。
まとめ：発生していることの推測
ログから推測される現状は、**「ASRエンジンは正しく喋り出しているが、過去の履歴と今の認識を繋ぎ合わせる『糊（のり）』の部分が、漢字の変換違いやデータの重複によってガタガタになっている」**状態です。

特に、Sherpa02 (Streaming ASR) は逐次結果を書き換える特性があるため、従来の「確定したものを継ぎ足す」という単純なモデルではなく、**「常に最新の状態に上書き（必要なら過去を消して打ち直し）」**というロジックへの徹底が必要であると考えられます。

# さらに私がした質問
わかりました。では追加で質問です。差分を比較する時に、その比較対象は句読点が入っていない状態のテキストで行われるべきだと思っています。なぜなら、マイクでしゃべったユーザがしゃべっている途中で言い淀んだりして中途半端な認識結果が出てきたとき、それに対して句読点挿入を行った場合に、本来句読点が挿入されるべきではない場所に対して句読点挿入が行われている状態のテキストが出来上がる可能性があるからです。そのような句読点の誤りがあるテキストに対して、差分の検査を行うと句読点が入っているせいで、正確な検査ができないのではないかと思うのです。現在の実装では、差分の検査は句読点が入っていない状態のテキストに対して行われるよう実装されていますか？質問にだけ答えて作業は行わないでください。

# さらに私がした質問に対するあなたの回答
ご質問ありがとうございます。現状の実装を確認したところ、結論から申し上げますと 「現在は残念ながら『句読点が入った状態』で差分（重なり）の検査を行ってしまっている」 状態です。

具体的な理由は以下の通りです。

現状の実装（src/tools/pseudo_asr_streamer.rs）

1. 句読点挿入のタイミング: transcribe メソッド内で ASR 変換（Rawテキスト）を取得した直後に、句読点マシン（PunctuationMachine）を通して、句読点が含まれたテキストを変数 window_text に格納しています。
2. 重なり検査のタイミング: その後、find_overlap_len を呼び出して session_history と window_text の重なりを調べていますが、この時点での window_text は既に句読点が挿入された後のものです。
3. 検査の問題点: ご指摘の通り、言い淀みなどで「こんにちは（沈黙）...」となった際に「こんにちは。」と句録点が入ってしまうと、次に「こんにちは、今日は...」という正しい続きが来た時に、末尾の 。 が邪魔をしてしまい、
find_overlap_len
 が正しい重なり（「こんにちは」部分）を検出できなくなります。

まとめ
現在の実装は、「加工済みテキスト（句読点あり）」同士を比較してしまっているため、ユーザー様が懸念されている 「句読点の誤挿入による検査精度の低下」がダイレクトに発生している 状態だと断定できます。

このため、本来は「生の結果（Rawテキスト）」同士で重なりを確認し、重なりを除去した「新しい純粋な発話」を確定させてから、最後に表示用・入力用として句読点を添える、という順序にする必要があります。

# 修正実装計画

## 問題1 & 5: 重なり検知精度と「句読点」干渉の解消

### 現状の悪影響と原因
- **ソース箇所**: `pseudo_asr_streamer.rs:1109` 周辺
    - `let history_overlap = Self::find_overlap_len(&self.session_history, &window_text);`
- **問題**: `self.session_history` には `。` や `？` が含まれており、`window_text`（生の結果）と文字列が完全一致しません。
    - 例: 履歴 "こんにちは。" vs 新規 "こんにちは今日..." -> 履歴末尾の `。` が邪魔で重なり 0 と判定され、結果が "こんにちは。こんにちは今日は..." と重複します。
- **表記揺れ**: `河田` と `川田` のように、ASRが途中で漢字を変えると重なり検知が失敗し、入力が重複または欠落します。

### 修正内容
1. `session_history_raw` を導入し、句読点のない純粋な認識結果の履歴を保持します。
2. 認識結果 (`window_text`) を加工（句読点挿入）する**前**に、生の状態同士で比較を行います。
3. `find_overlap_len` の内部で、文字種（ひらがな/カタカナ）の正規化を行い、軽微な揺れを許容します。この際の曖昧さ閾値は設定ファイル (`fuzzy_threshold`) から読み込みます。

```rust
// pseudo_asr_streamer.rs

// 1. 履歴をRaw(生)とDisplay(表示用)で分離保持
struct PseudoAsrStreamer<B: AsrBackend> {
    // ...
    session_history_raw: String,     // 句読点なしの生履歴
    session_history_display: String, // 句読点ありの表示履歴
}

// 2. 比較ロジックの修正
fn merge_text(&mut self, window_text_raw: &str) -> SttMergeResult {
    // 常に Raw 同士で重なりをチェック（句読点に邪魔されない）
    let history_overlap = Self::find_overlap_len(&self.session_history_raw, window_text_raw);
    
    // 重なりを除去した新しい部分を抽出
    let new_raw_part: String = window_text_raw.chars().skip(history_overlap).collect();
    
    // committed_text (未確定安定分) と new_raw_part を結合
    let current_active_raw = format!("{}{}", self.committed_text, new_raw_part);
    
    // UI表示用にのみ、その場で句読点を振る
    let punctuated = self.punctuation_machine.insert(&current_active_raw);
    
    // ...
}

// 3. 曖昧一致の強化と正規化関数の実装
fn find_overlap_len(a: &str, b: &str) -> usize {
    // 比較前に句読点を消し、ひらがなに統一することで「河田」と「川田」を同一視させる
    let a_norm = normalize_for_comparison(a);
    let b_norm = normalize_for_comparison(b);

    // a_norm の末尾 50文字程度と b_norm の冒頭を比較
    // 編集距離 (Levenshtein distance) を用いて、
    // (編集距離 / 一致長) < FUZZY_COST_THRESHOLD (0.3) なら重なりとみなす
}

fn normalize_for_comparison(text: &str) -> String {
    text.chars()
        .filter(|&c| !is_punctuation(c)) // 句読点、スペースを除去
        .map(|c| to_kana_one_way(c))     // 全てひらがな、または全てカタカナに統一
        .collect()
}
```

### 期待される結果
- 前の文が `。` で終わっていても、次の発話の冒頭部分が正しく重なりとして検出され、重複入力が完全に解消されます。
- 多少の漢字変換の揺れがあっても、論理的な接続が維持されます。
- 発話途中の言い淀みで「こんにちは。 ... 今日は」となった場合でも、Rawテキストベースでは「こんにちは 今日は」として綺麗に繋がります。

## 問題2: チャンク境界での二重コミット防止

### 現状の悪影響と原因
- **ソース箇所**: `pseudo_asr_streamer.rs:1095` 周辺
    - 文章が決定しても `self.current_chunk` がクリアされない。
- **問題**: 沈黙によって `Speech END` が発生し、それまでの音声が `Chunk` として確定しても、ASRの内部バッファ (`current_chunk`) にその音声が残り続けます。
- **結果**: 次に喋り出したとき、`current_chunk` には「過去に喋り終えた音声 + 今の音声」が混ざった状態で ASR に送られ、過去のセリフが「新しい発話」として二重に認識されます。

### 修正内容
1. 文章が句読点によって確定した (`SttMergeResult::WithFinal`) タイミングで、その「確定分」に相当する音声データを物理的に破棄します。
2. **安全性ガード**: 単なる `。` の出現だけでなく、後述する **「最終補正レイヤー」の確定条件（文数 or 長期沈黙）** と連動させ、認識が本当に安定したと判断された時のみ、`current_chunk` や `chunk_queue` をクリアします。これにより、言い淀み等による誤ったフラッシュを防ぎます。

```rust
// pseudo_asr_streamer.rs
fn process_window(&mut self) {
    // ... 認識実行 ...
    let result = self.merge_text(&window_text);
    
    if let SttMergeResult::WithFinal { finalized_raw_len, .. } = result {
        // 条件判定：文脈が十分に安定したか？ (後述の確定ロジックを参照)
        if self.should_flush_audio(finalized_raw_len) {
            self.chunk_queue.clear();
            self.current_chunk.clear(); 
            log::debug!("Finalized high-level: All audio buffers flushed safely.");
        }
    }
}
```

### 期待される結果
- ひとたび `。` で文章が終わった後は、以前の音声が再認識されることがなくなり、二重コミットが根絶されます。
- 安全性ガードにより、ユーザーが喋り続けている最中にバッファが誤ってクリアされることがなくなります。

## 問題3: input_diff（バックスペース）の安定化

### 現状の悪影響と原因
- **ソース箇所**: `keyboard.rs:144` (`input_diff`)
- **問題**: ASRが途中で認識（漢字変換など）を書き換えた場合、`old_text` と `new_text` の共通接頭辞が短くなります。このとき、適切にバックスペースを発行して「間違っていた古い文字列」を消す必要があります。
- **結果**: 削除処理が不完全、あるいはストリーマーから送られるテキストの不整合（前後関係の破綻）があると、古い変換の上に新しい変換が乗り、「河田川田です」のような重複入力が発生します。

### 修正内容
1. `input_diff` のロジックが、常に `old_text` の末尾から共通部分までを正確に削除することを確認します（現状は概ね正しいが、文字境界の扱いを再チェック）。
2. ストリーマー側 (`merge_text`) が、一度確定した `session_history` 以外の部分（未確定分）を自由に書き換えても良いようにし、`input_diff` がそれを常に正しくカーソル位置に反映するように同期させます。

### 期待される結果
- ASRが「河田」と言った直後に「あ、やっぱり川田だ」と認識を修正しても、エディタ上では「河田」がバックスペースで消え、即座に「川田」に書き換わるスムーズな挙動が得られます。

## 問題4: VAD (発話検知) パラメータの最適化

### 現状の悪影響と原因
- **ソース箇所**: `pseudo_asr_streamer.rs:209-213` (Default Config)
- **問題**: 500ms（0.5秒）の沈黙で発話終了とみなされるため、日本語特有の「溜め」や「言い淀み」で頻繁にチャンクが切れてしまいます。
- **結果**: チャンクが細切れになるたびに「重なり検知」が走り、そこで誤差が蓄積するリスクが高まります。

### 修正内容
1. `vad_silence_tolerance_ms` のデフォルト値を延長します。
2. `vad_min_speech_duration` を少し厳しめにし、一瞬のノイズが「発話」としてカウントされるのを防ぎます。

```rust
// pseudo_asr_streamer.rs
impl Default for StreamerConfig {
    fn default() -> Self {
        Self {
            // ...
            vad_min_speech_duration: 0.3,     // 0.25 -> 0.3 (少し慎重に)
            vad_silence_tolerance_ms: 800,    // 500 -> 800 (間を持たせる)
            // ...
        }
    }
}
```

### 期待される結果
- 「はい（小休止）お世話になります」という発話が、途中でブツ切りにならずに1つの文として認識されやすくなり、システム全体の安定性が向上します。

## 問題5: `input_diff` における文字数と長さの扱い

### 現状の悪影響と原因
- **ソース箇所**: `keyboard.rs:145`
- **問題**: 文字列の長さを `len()` (バイト長) で扱っている箇所があると、2バイト/3バイト文字である日本語の削除（バックスペース）でズレが生じます。
- **確認事項**: 現在は `chars().collect::<Vec<char>>().len()` を使っているため概ね安全ですが、`input_diff` に渡される文字列自体にゴミ（制御文字など）が混じっていないかを保証する必要があります。

### 修正内容
- デバッグログを強化し、`input_diff` が「何文字の共通部分を見つけ、何文字削除しようとしたか」を可視化します。

## 設定情報の管理 (`settings.json`)

本計画で導入・変更されるすべての閾値やタイムアウト時間は、ハードコードせず `Sherpa02Settings` 構造体に追加し、`settings.json` から動的に変更できるようにします。

### [MODIFY] `Sherpa02Settings` (`stt_config.rs`)
以下の項目を追加します：
- `fuzzy_threshold: f32` (重なり検知の曖昧さ閾値)
- `final_sentence_count: usize` (最終補正を起動する文数)
- `final_silence_duration_ms: u64` (最終補正を起動する長期沈黙時間)
- `normalize_kana: bool` (比較時にかな統一を行うか)

```rust
// stt_config.rs
pub struct Sherpa02Settings {
    // ... 既存項目 ...
    #[serde(default = "default_fuzzy_threshold")]
    pub fuzzy_threshold: f32,
    #[serde(default = "default_final_sentence_count")]
    pub final_sentence_count: usize,
    #[serde(default = "default_final_silence_duration_ms")]
    pub final_silence_duration_ms: u64,
}
```

## 新機能: 最終補正レイヤー (Final Correction Layer) と確定ロジックの高度化

逐次認識では解決できない「長文の整合性」を担保するため、後出しでの一括補正レイヤーを導入します。

### 補正バッファの構造 (`PostCorrectionBuffer`)
単なる履歴ではなく、文脈情報を保持する専用のコンテナを新設します。
```rust
struct PostCorrectionBuffer {
    raw_text: String,               // 句読点を含まない全生文字列
    punctuated_text: String,        // 逐次加工済みのテキスト
    punctuation_map: BTreeMap<usize, char>, // raw_text上のどの文字の後に、どの句読点があるか
    sentence_count: usize,          // 「。」や「？」の出現回数
}
```

### 最終補正関数 (`final_correct_layer`)
```rust
fn final_correct_layer(buffer: &PostCorrectionBuffer) -> String {
    // 1. 蓄積された raw_text を引き取る
    // 2. まとまった文脈に基づいて、形態素解析や将来のLLM変換を実行
    // 3. 全体として最も不自然さの少ない決定稿（句読点、表記補正済み）を返す
}
```

### 確定（フラッシュ & 補正）の起動条件
単純な文字数ではなく、以下の**文脈ベースの条件**が満たされた時に最終補正を実行し、オーディオをフラッシュします。

1. **文数（Sentence Count）による制限**:
   - 形態素解析の結果、意味の区切りが明確な `。` が一定数（例: 3〜5文）に達した場合。句読点マップを参照するため、意味の途中で切れるリスクを最小化できます。
2. **長期沈黙（VAD Long Silence）**:
   - 約1.5秒〜2.0秒の沈黙を検知した時。ユーザーが話を終えたとみなし、バッファを全清書します。
3. **強制終了**:
   - ホットキーによる認識停止時。未補正のバッファをすべて清書して出力します。

### 期待される結果
- リアルタイムでの「打ち込みの速さ」を維持しつつ、最終的には「文脈の通った正しい文章」に整うことが保証されます。
- オーディオバッファの破棄タイミングが「文の区切り」と一致するため、二重認識と認識漏れの両方を防ぐ堅牢な仕組みとなります。

## 修正の順序
1. `pseudo_asr_streamer.rs` の内部状態 (`session_history_raw`) と補正バッファ (`PostCorrectionBuffer`) の追加。
2. `merge_text` のフロー修正（Raw優先）および句読点位置のトラッキング実装。
3. 文数ベース及び長期沈黙ベースの「確定確定（Final Finalization）」ロジックの実装。
4. 確定確定時のオーディオバッファ・クリア処理。
5. `final_correct_layer` のガワ（初期実装は既存のPunctuationMachineの再適用）の作成。
6. VAD パラメータのデフォルト値変更および `settings.json` への反映。
7. `input_diff` の動作検証。

# 以上の修正を完了させた後に行なったテスト結果報告

「こんにちは。今日はいい天気ですね。」と言った後、少し間を置いて「今日は買い物に行きます。」と続けるというテストを行いました。

テキスト入力のカーソルの位置に入力されたのは以下です。
```
こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
```

サーバーのコンソールのログには以下のように出力されました。
```
26-01-14_15:58:39 mycute.stt.recognizer     [DEBUG] Speech recognition started (engine: Sherpa02)
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] Speech START
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.36s, queue_len=0, current_chunk_len=5760
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] Result: こんにちは
26-01-14_15:58:41 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちは
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.65s, queue_len=0, current_chunk_len=10368
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはい"
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。
26-01-14_15:58:41 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.94s, queue_len=0, current_chunk_len=14976
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_15:58:41 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは
26-01-14_15:58:41 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.22s, queue_len=0, current_chunk_len=19584
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは
26-01-14_15:58:42 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] Speech END
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Chunk detected: ID=0, duration=1.29s
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.29s, queue_len=1, current_chunk_len=0
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは
26-01-14_15:58:42 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.29s, queue_len=1, current_chunk_len=0
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは
26-01-14_15:58:42 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは
26-01-14_15:58:42 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.29s, queue_len=1, current_chunk_len=0
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは
26-01-14_15:58:43 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.29s, queue_len=1, current_chunk_len=0
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは
26-01-14_15:58:43 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.29s, queue_len=1, current_chunk_len=0
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは
26-01-14_15:58:43 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] Speech START
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.55s, queue_len=1, current_chunk_len=4224
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_15:58:43 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは
26-01-14_15:58:43 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.84s, queue_len=1, current_chunk_len=8832
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは
26-01-14_15:58:44 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.16s, queue_len=1, current_chunk_len=13952
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは今日はいい天気ですね"
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:44 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.35s, queue_len=1, current_chunk_len=17024
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:44 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.64s, queue_len=1, current_chunk_len=21632
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:44 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:44 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.86s, queue_len=1, current_chunk_len=25216
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:45 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.09s, queue_len=1, current_chunk_len=28800
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:45 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] Speech END
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Chunk detected: ID=1, duration=1.86s
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.15s, queue_len=2, current_chunk_len=0
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:45 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.15s, queue_len=2, current_chunk_len=0
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:45 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:45 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.15s, queue_len=2, current_chunk_len=0
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:46 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.15s, queue_len=2, current_chunk_len=0
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:46 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.15s, queue_len=2, current_chunk_len=0
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:46 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.15s, queue_len=2, current_chunk_len=0
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:46 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.15s, queue_len=2, current_chunk_len=0
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:46 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:46 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.15s, queue_len=2, current_chunk_len=0
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:47 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.15s, queue_len=2, current_chunk_len=0
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:47 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] Speech START
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.51s, queue_len=2, current_chunk_len=5760
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい
26-01-14_15:58:47 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.74s, queue_len=2, current_chunk_len=9344
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい
26-01-14_15:58:47 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい
26-01-14_15:58:47 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.96s, queue_len=2, current_chunk_len=12928
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい
26-01-14_15:58:48 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=4.18s, queue_len=2, current_chunk_len=16512
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい"
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい
26-01-14_15:58:48 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=4.44s, queue_len=2, current_chunk_len=20608
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "はい今日は買い物に行きます"
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:48 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=4.76s, queue_len=2, current_chunk_len=25728
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:48 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:48 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=2, current_chunk_len=30848
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:49 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] Speech END
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Chunk detected: ID=2, duration=1.96s
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:49 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:49 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:49 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:50 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:50 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:50 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:50 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:50 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:51 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:51 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:51 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:51 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:51 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:52 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:52 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:52 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:52 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:52 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:53 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:53 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:53 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:53 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:53 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:54 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=5.00s, queue_len=3, current_chunk_len=0
26-01-14_15:58:54 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日は買い物に行きます"
26-01-14_15:58:54 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Text after history stripping: ""
26-01-14_15:58:54 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: ""
26-01-14_15:58:54 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
26-01-14_15:58:54 tools.pseudo_asr_streamer [DEBUG] Recognition stopped
26-01-14_15:58:54 mycute.stt.recognizer     [DEBUG] Speech recognition stopped (Sherpa02)
26-01-14_15:58:54 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは今日はいい天気ですね。はい今日は買い物に行きます。
```

確かに、修正を行う前よりも改善しています。
でも完璧ではありませんね。
このログからわかる問題点を洗い出し、私に報告し、まだ作業には入らないでください。

# 更なる修正案：テキスト確定と音声クリアの同期

分析の結果、重複の主な原因は**「テキスト層の勝手な確定」と「音声層の慎重な保持」の不整合**にあることが分かりました。

### 1. 問題の再定義（なぜ「い。こんにちは」になったか）
1. **15:58:41**: ASRが `こんにちはい`。末尾に句読点がついたため、`merge_text` は即座に `session_history`（確定済み履歴）へ移動。
2. **15:58:42**: しかし、音声層は「安全ガード（文数閾値）」により、まだ音声データを保持。
3. **直後**: ASRが音声を再評価し、より正しい `こんにちは` を出力。
4. **不整合**: テキスト層は既に前回の誤入力を「確定した過去」として扱っているため、新しい `こんにちは` を重なりとして検知できず、二重に出力されてしまった。

### 2. 解決策：タイミングの完全同期
テキストの確定タイミングを、音声バッファのクリアタイミングと完全に同期させます。

*   **句読点による即時確定の廃止**:
    `merge_text` 内で句読点を見つけても、`should_flush_audio()` が `true` にならない限りは `session_history` に push しない。
*   **「修正可能な Partial」としての維持**:
    `input_diff` ロジックは、以前送った `Partial` の内容が変われば、バックスペースで自動的に修正（例：「い。」を消して「こんにちは」に書き換え）してくれます。テキストを履歴に移動させず「生きた文字列」として保持し続けることで、ASRの後追いの修正を許容します。
*   **同期した Finalization**:
    `process_window` で `should_flush_audio()` が `true` を返したときだけ、`PostCorrectionBuffer` に溜まった内容を `session_history` および `session_history_raw` に移動し、音声バッファをクリアする。

### 3. 具体的なコード変更方針
1.  `merge_text` のロジックを変更：
    - 句読点で `session_history` を更新する処理を削除。
    - 常に「現在構築中の文字列」として `SttMergeResult::Partial` を返すように整理する。
2.  `process_window` の更新：
    - `self.should_flush_audio()` が真の場合、`self.post_correction_buffer.punctuated_text` の全体（または確定部分）を `session_history` に移動する。
    - `session_history_raw` も同期して更新。
3.  `input_diff` への影響：
    - テキストが履歴（不動の文字列）にならないため、ASRの迷いによる「書き換え」がそのままエディタ上での「バックスペースと再入力」として安全に実行される。

この方針であれば、ASRが一時的に誤った句読点を出しても、音声バッファクリアまでの猶予期間中に正しく修正されることが期待できます。

# 以上の修正を完了させた後に行なったテスト結果報告

「こんにちは。今日はいい天気ですね。今日は買い物に行きます。」と発話してテストしました。

テキスト入力のカーソルの位置に入力されたのは以下です。
```
こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
```

サーバーのコンソールにログされたのは以下です。
```
26-01-14_16:15:52 mycute.stt.recognizer     [DEBUG] Speech recognition started (engine: Sherpa02)
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] Speech START
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.39s, queue_len=0, current_chunk_len=6272
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちは"
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] Result: こんにちは
26-01-14_16:15:54 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちは
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.65s, queue_len=0, current_chunk_len=10368
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはい"
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。
26-01-14_16:15:54 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.94s, queue_len=0, current_chunk_len=14976
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはい"
26-01-14_16:15:54 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。
26-01-14_16:15:54 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.19s, queue_len=0, current_chunk_len=19072
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはい"
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。
26-01-14_16:15:55 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] Speech END
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Chunk detected: ID=0, duration=1.26s
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.26s, queue_len=1, current_chunk_len=0
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはい"
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。
26-01-14_16:15:55 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] Speech START
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.55s, queue_len=1, current_chunk_len=4736
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "こんにちは"
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちは"
26-01-14_16:15:55 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちは
26-01-14_16:15:55 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちは
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.84s, queue_len=1, current_chunk_len=9344
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はい"
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい"
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい。
26-01-14_16:15:56 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい。
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.06s, queue_len=1, current_chunk_len=12928
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気"
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気"
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気
26-01-14_16:15:56 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.35s, queue_len=1, current_chunk_len=17536
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気です"
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気です"
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気です。
26-01-14_16:15:56 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気です。
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.58s, queue_len=1, current_chunk_len=21120
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですね"
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:56 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:56 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=2.77s, queue_len=1, current_chunk_len=24192
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですね"
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:57 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.02s, queue_len=1, current_chunk_len=28288
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですね"
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:57 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Speech END
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Chunk detected: ID=1, duration=1.93s
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.18s, queue_len=2, current_chunk_len=0
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですね"
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:57 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.18s, queue_len=2, current_chunk_len=0
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですね"
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:57 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.18s, queue_len=2, current_chunk_len=0
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですね"
26-01-14_16:15:57 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:57 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.18s, queue_len=2, current_chunk_len=0
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですね"
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:58 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.18s, queue_len=2, current_chunk_len=0
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですね"
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:58 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.18s, queue_len=2, current_chunk_len=0
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですね"
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:58 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.18s, queue_len=2, current_chunk_len=0
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですね"
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:58 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] Speech START
26-01-14_16:15:58 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.58s, queue_len=2, current_chunk_len=6272
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "はいこう"
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですねはいこう"
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですねはいこう。
26-01-14_16:15:59 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですねはいこう。
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=3.80s, queue_len=2, current_chunk_len=9856
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "今日はいい天気ですね"
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "こんにちはいこんにちはい今日はいい天気ですねはいこう今日はいい天気ですね"
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] Finalized: Text and audio buffers flushed (sentence_count >= threshold). session_history: "こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。"
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。
26-01-14_16:15:59 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.26s, queue_len=0, current_chunk_len=4096
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "二"
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "二"
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二
26-01-14_16:15:59 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.51s, queue_len=0, current_chunk_len=8192
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "九"
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "二九"
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九
26-01-14_16:15:59 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九
26-01-14_16:15:59 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=0.77s, queue_len=0, current_chunk_len=12288
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "きます"
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "二九きます"
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きます。
26-01-14_16:16:00 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きます。
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.06s, queue_len=0, current_chunk_len=16896
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "いきます"
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "二九きますいきます"
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきます。
26-01-14_16:16:00 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきます。
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] Speech END
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Chunk detected: ID=2, duration=1.31s
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.31s, queue_len=1, current_chunk_len=0
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "きま"
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "二九きますいきますきま"
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:00 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.31s, queue_len=1, current_chunk_len=0
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "きま"
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "二九きますいきますきま"
26-01-14_16:16:00 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:00 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.31s, queue_len=1, current_chunk_len=0
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "きま"
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "二九きますいきますきま"
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:01 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.31s, queue_len=1, current_chunk_len=0
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "きま"
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "二九きますいきますきま"
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:01 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.31s, queue_len=1, current_chunk_len=0
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "きま"
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "二九きますいきますきま"
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:01 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:01 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Recognizing window: duration=1.31s, queue_len=1, current_chunk_len=0
26-01-14_16:16:02 tools.pseudo_asr_streamer [DEBUG] Raw transcription: "きま"
26-01-14_16:16:02 tools.pseudo_asr_streamer [DEBUG] [PseudoAsrStreamer] Cumulative active text: "二九きますいきますきま"
26-01-14_16:16:02 tools.pseudo_asr_streamer [DEBUG] Result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:02 mycute.mode.vp.main_of_vp [DEBUG] Received STT result: こんにちはい。こんにちはい今日はいい天気ですね。はいこう今日はいい天気ですね。二九きますいきますきま
26-01-14_16:16:02 tools.pseudo_asr_streamer [DEBUG] Recognition stopped
26-01-14_16:16:02 mycute.stt.recognizer     [DEBUG] Speech recognition stopped (Sherpa02)
^C26-01-14_16:16:02 mycute.mode.vp.main_of_vp [DEBUG] Received Ctrl+C, cleaning up...
26-01-14_16:16:02 mycute.mode.vp.main_of_vp [DEBUG] Cleanup complete. Exiting.
```

ここから何がわかるか説明してください。説明だけして作業には入らないでください。

# 更なる修正案：多層重複防止アーキテクチャ

## 1. 問題の根本原因

直前のテストで判明した問題は2つの層で発生している：

### 問題A: バッファ内での重複蓄積
```
Raw="こんにちは" → committed_text="こんにちは"
Raw="はい"       → committed_text="こんにちはい"  ← 誤追加
```
ASRが短い音声区間で「はい」と誤認識した際、差分マージロジックがそのまま `committed_text` に追加してしまう。

### 問題B: 履歴との重複検知失敗
```
Raw="こんにちは" → cumulative="こんにちはいこんにちは"  ← 重複
```
`session_history_raw` が空のため、2回目の「こんにちは」を重複として検知できない。

### 問題C: 音声保持による認識精度の劣化
3文蓄積を待つ間に音声バッファが巨大化し、ASRが以前の音声コンテキストに引きずられて、後半の認識精度が著しく低下する。
（例：ログの最後で「今日は買い物に行きます」が「二九きます」に壊れた現象）

## 2. 解決策：多層防御アーキテクチャ

重複の防止（Layer 1 & 2）と、認識精度の維持（Layer 2 によるバッファの適時クリア）を両立させる。

### Layer 1: バッファ追加時の重複チェック（Option 3）

**目的**: `committed_text` に新しいテキストを追加する前に、既存テキストとの重複をチェック

**実装場所**: `merge_text` 関数の Differential Merge 部分

**ロジック**:
```rust
// Differential Merge（既存）
if !self.last_window_text.is_empty() {
    let overlap_len = Self::find_overlap_len(&self.last_window_text, &window_text);
    let last_chars: Vec<char> = self.last_window_text.chars().collect();

    if overlap_len < last_chars.len() {
        let shifted_out: String = last_chars[..last_chars.len() - overlap_len]
            .iter()
            .collect();
        
        // 【新規】shifted_out が committed_text の末尾と重複していないかチェック
        let buffer_overlap = Self::find_overlap_len(&self.committed_text, &shifted_out);
        if buffer_overlap < shifted_out.chars().count() {
            // 重複していない部分のみ追加
            let non_overlap: String = shifted_out.chars().skip(buffer_overlap).collect();
            self.committed_text.push_str(&non_overlap);
        }
        // 完全に重複している場合は追加しない
    }
}
```

### Layer 2: 履歴との重複検知の復活（Option 2）

**目的**: `session_history_raw` を句読点で即時更新し、重複検知を機能させる

**実装場所**: `merge_text` 関数の末尾

**ロジック**:
```rust
// 句読点検知（UI確定とは分離）
let sentence_end_pos = active_text_display.rfind(|c| c == '。' || c == '？');

if let Some(pos) = sentence_end_pos {
    // 【重要】session_history_raw を即時更新（重複検知用）
    let raw_sentence_end = Self::find_raw_position_for_display_position(
        &active_text_raw, &active_text_display, pos
    );
    let raw_to_finalize: String = active_text_raw.chars().take(raw_sentence_end + 1).collect();
    
    if !self.session_history_raw.ends_with(&raw_to_finalize) {
        self.session_history_raw.push_str(&raw_to_finalize);
    }
    
    // committed_text から確定済み部分を削除
    let remaining_raw: String = active_text_raw.chars().skip(raw_sentence_end + 1).collect();
    self.committed_text = remaining_raw;
    self.last_window_text.clear();
}

// session_history（UI用）は should_flush_audio() の場合のみ更新（process_window で処理）
```

### Layer 3: UI確定の遅延（既存）

**目的**: ASRの訂正を反映するため、UIへの確定送信を遅延させる

**実装場所**: `process_window` 関数

**ロジック**:
```rust
if self.should_flush_audio() {
    // PostCorrectionBuffer から確定テキストを取得してUIに送信
    self.session_history.push_str(&self.post_correction_buffer.punctuated_text);
    self.post_correction_buffer.clear();
    self.chunk_queue.clear();
    self.current_chunk.clear();
}
```

## 3. データフロー図

```
[ASR Raw Output]
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│ Layer 2: History Stripping                                      │
│ session_history_raw との重複をチェックし、重複部分を除去        │
└─────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│ Layer 1: Buffer Overlap Check                                   │
│ committed_text への追加前に、末尾との重複をチェック            │
└─────────────────────────────────────────────────────────────────┘
       │
       ▼
[committed_text + window_text = active_text_raw]
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│ Punctuation Machine                                             │
│ 句読点を挿入して active_text_display を生成                    │
└─────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│ Sentence Detection                                              │
│ 句読点を検知したら session_history_raw を即時更新              │
│ （UI用 session_history は更新しない）                          │
└─────────────────────────────────────────────────────────────────┘
       │
       ▼
[session_history + active_text_display → UI へ送信]
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│ Layer 3: Delayed Finalization (process_window)                  │
│ should_flush_audio() == true の場合のみ                        │
│ session_history を更新し、バッファをクリア                     │
└─────────────────────────────────────────────────────────────────┘
```

## 4. 実装ステップ

### Step 1: Layer 1 の実装（バッファ追加時の重複チェック）
- `merge_text` の Differential Merge 部分を修正
- `committed_text` への追加前に `find_overlap_len` で重複チェック
- 重複部分を除いた文字列のみ追加

### Step 2: Layer 2 の実装（履歴更新の復活）
- `merge_text` の末尾で句読点検知時に `session_history_raw` を即時更新
- `session_history`（UI用）は更新しない（Layer 3 で処理）
- `committed_text` から確定済み部分を削除

### Step 3: 動作確認
- `cargo check` でコンパイル確認
- テスト実行：「こんにちは。今日はいい天気ですね。今日は買い物に行きます。」

## 5. 期待される結果

| 入力 | 期待出力 |
|------|----------|
| こんにちは。今日はいい天気ですね。今日は買い物に行きます。 | こんにちは。今日はいい天気ですね。今日は買い物に行きます。 |

- Layer 1 により「はい」などの誤追加が防止される
- Layer 2 により2回目の「こんにちは」が重複として検知・除去される
- Layer 2 による定期的なバッファクリアにより、ASRの認識精度が維持される（「二九きます」問題を解消）
- Layer 3 によりUI確定が遅延され、ASRの最終的な訂正が反映される

## 6. リスクと対策

| リスク | 対策 |
|--------|------|
| 過剰な重複検知で正当なテキストが削除される | `find_overlap_len` の閾値を調整可能にする |
| `session_history_raw` が誤ったテキストを含む | ASR訂正をある程度待ってから更新する |
| パフォーマンス低下 | 比較対象を末尾50文字に限定（既存） |

# 更なる修正案

これまでの修正により、「はいはい」という過剰な重複はある程度抑制されましたが、ASR（ReazonSpeech/Sherpa-ONNX）特有の「認識結果の激しい書き換え（フリッカ）」と「窓の端に残った残響による幻聴」という、より深層の問題が浮き彫りになりました。

本セクションでは、これらの課題を「対処療法」ではなく「頑健なアーキテクチャ」として解決するための、更なる修正案を提示します。

## 1. 背景と課題の再定義

### A. ASRの「迷い」による文字消失と重複
ASR は、窓の中に新しい音声が入ってくるたびに、過去の認識結果も遡って書き換えます。
- **消失**: 「こんにちは」と言った後、窓がスライドして「は」だけが残った際、ASRが「あ、今のやっぱり『はい』だった」と判断を変えると、確定前の「こんにちは」がバッファから消えてしまいます。
- **重複**: 「窓から消えたら確定」というルールだと、ASRが一時的に内容を削った際に「消えた」と誤認して固定してしまい、その後ASRが自信を取り戻して再び同じ文を出したとき、固定分＋窓分で二重になります。

### B. 窓の切れ端による「はい（幻聴）」
ASRの窓がスライドしていく過程で、「こんにちは」の末尾の母音の余韻などが窓の先頭に僅かに残ることがあります。ASRは「意味のない音」を無理やり日本語として解釈しようとするため、最も確率の高い「はい」「は」「え」といった幻聴を出力し続けます。

---

## 2. 根本対策案

### 案1：時間軸ベースのプレフィックス安定化 (Time-based Stability)
「推論回数」ではなく、**「そのテキストが窓内で一貫して存在し続けた累積時間（ms）」**を確定基準にします。

- **ロジック**:
    1. 窓の先頭部分にあるテキスト X が、複数の推論フレームにわたって不変であることを追跡する。
    2. テキスト X が窓内に現れてから **N ミリ秒（例: 400ms）** 経過し、かつその間一度も内容が書き換わらなかった場合のみ、「安定」とみなして `committed_text` に固定する。
- **効果**: `interim_interval_ms` の設定値に左右されず、ASRの「迷い（フリッカ）」が収まるのを物理的な時間で待つことができるようになります。

### 案2：ノイズ除去基盤 (GTCRN) の導入
将来的なノイズ環境（カフェ、オフィス等）への耐性を高めるため、ASRの前段にデノイザーを配置します。

- **選定理由**: **GTCRN (Group-Transformer Convolutional Recurrent Network)** を採用。
    - DeepFilterNet3 に匹敵する高いノイズ抑制能力を持ちつつ、演算量は 1/10 程度と極めて軽量。
    - 現代のコンシューマーPCにおいて、数パーセント以下のCPU負荷でリアルタイム動作が可能。
- **目的**: 信号から「机を叩く音」「鼻をすする音」「窓の端に残った不自然な残響」を物理的に除去し、ASRが幻聴を起こすための「種」を未然に排除します。

### 案3：軽量 Pre-ASR 信号フィルタ
デノイズ後、さらに ASR に投げるに値するオーディオデータかどうかを軽量な計算で判定します。

- **実質発話長チェック**: ウィンドウ全体に対し、音圧（RMS）が基準を超えている区間が**一定の割合（例: 10〜15%）**に満たない場合は、ASR に投げずに無視する。
- **効果**: 窓サイズ（1.3秒等）の変更に柔軟に対応しつつ、意味をなさない極小の音の切れ端が ASR に到達することを防ぎ、「はい」攻撃を未然に防ぎます。
- **将来性**: マシンの性能や ASR の特性に合わせ、この「占有率閾値」を非固定のパラメータとして調整可能にします（settings.jsonで設定可能なパラメーターとして実装します）。

---

## 3. 実装イメージ

### 安定性トラッカーの導入
```rust
struct StabilityTracker {
    candidate_text: String,
    first_seen: std::time::Instant,
}

// 窓の前方（古い部分）から順に、安定時間をチェック
fn check_stability(&mut self, current_window: &str) {
    let common_prefix = find_common_prefix(&self.last_stable_candidate, current_window);
    
    if common_prefix == self.last_stable_candidate {
        if self.first_seen.elapsed().as_millis() > 400 {
            self.commit(common_prefix);
        }
    } else {
        self.last_stable_candidate = common_prefix;
        self.first_seen = std::time::Instant::now();
    }
}
```

### 信号品質フィルタ
```rust
fn is_worthy_of_asr(samples: &[f32]) -> bool {
    let rms = (samples.iter().map(|s| s*s).sum::<f32>() / samples.len() as f32).sqrt();
    let energy_threshold = 0.01; // 要調整
    let active_samples = samples.iter().filter(|&&s| s.abs() > energy_threshold).count();
    let effective_duration_ms = (active_samples as f32 / 16000.0) * 1000.0;

    // 150ms以下の微小な音や、極小音量は無視
    effective_duration_ms >= 150.0 && rms >= 0.005
}
```

## 4. 将来の展望
この設計により、`mycute` は単なる「STT 表示ツール」から、**「過酷な音響環境下でも言葉のニュアンスを壊さず、正確にテキスト化する頑健な音声入力エンジン」**へと進化します。ノイズ除去を基盤に置くことで、今後の屋外利用や多人数環境への対応も容易になります。
