import wave
import struct
import math
import sys

def smart_process(path, padding_ms=200, threshold_db=-50):
    try:
        w = wave.open(path, 'rb')
        params = w.getparams()
        n_frames = w.getnframes()
        sample_rate = w.getframerate()
        n_channels = w.getnchannels()
        sampwidth = w.getsampwidth()
        frames = w.readframes(n_frames)
        w.close()
        
        max_val = 2**(sampwidth*8-1)
        fmt = f"<{n_frames * n_channels}{'h' if sampwidth == 2 else 'b'}"
        samples = list(struct.unpack(fmt, frames))
        
        # 閾値を超える最初のインデックスと最後のインデックスを探す
        first_idx = len(samples)
        last_idx = 0
        threshold_val = max_val * (10**(threshold_db/20.0))
        
        for i in range(len(samples)):
            if abs(samples[i]) > threshold_val:
                if i < first_idx: first_idx = i
                if i > last_idx: last_idx = i
        
        if last_idx < first_idx:
            print(f"Skipping {path}: No signal above {threshold_db}dB found.")
            return

        # パディング分のフレーム数
        pad_frames = int((padding_ms / 1000.0) * sample_rate)
        pad_samples = pad_frames * n_channels
        
        start_idx = max(0, first_idx - pad_samples)
        end_idx = min(len(samples), last_idx + pad_samples)
        
        refined = samples[start_idx:end_idx]
        
        # パディング部分の音圧をさらに下げる (-60dB程度を狙う)
        # 実音以外の部分を 0.05 倍にする
        actual_start_in_refined = first_idx - start_idx
        actual_end_in_refined = last_idx - start_idx
        
        for i in range(len(refined)):
            if i < actual_start_in_refined or i > actual_end_in_refined:
                refined[i] = int(refined[i] * 0.1) # 音圧をさらに減衰
                
        # 書き戻し
        out = wave.open(path, 'wb')
        out.setparams(params)
        out.setnframes(len(refined) // n_channels)
        out.writeframes(struct.pack(f"<{len(refined)}{'h' if sampwidth == 2 else 'b'}", *refined))
        out.close()
        print(f"Successfully optimized {path}: Extracted signal with {padding_ms}ms padding.")
        
    except Exception as e:
        print(f"Error processing {path}: {e}")

if __name__ == "__main__":
    for p in sys.argv[1:]:
        smart_process(p)
