import wave
import struct
import math
import sys

def analyze(path):
    try:
        w = wave.open(path, 'rb')
        params = w.getparams()
        n_frames = w.getnframes()
        sample_rate = w.getframerate()
        n_channels = w.getnchannels()
        sampwidth = w.getsampwidth()
        
        print(f"File: {path}")
        print(f"Sample Rate: {sample_rate} Hz")
        print(f"Channels: {n_channels}")
        print(f"Duration: {n_frames/sample_rate:.3f} s")
        
        frames = w.readframes(n_frames)
        # Assuming 16-bit (width=2) or 8-bit
        fmt = f"<{n_frames * n_channels}{'h' if sampwidth == 2 else 'b'}"
        samples = struct.unpack(fmt, frames)
        
        def get_rms_db(s_chunk):
            if not s_chunk: return -100.0
            ms = sum(float(s)**2 for s in s_chunk) / len(s_chunk)
            rms = math.sqrt(ms)
            # Max amplitude for 16-bit is 32768, 8-bit is 128
            max_val = 2**(sampwidth*8-1)
            db = 20 * math.log10(rms / max_val) if rms > 0 else -100.0
            return db

        # Analyze first 500ms and last 500ms
        pre_frames = int(0.5 * sample_rate)
        post_frames = int(0.5 * sample_rate)
        
        pre_chunk = samples[:pre_frames * n_channels]
        post_chunk = samples[-(post_frames * n_channels):]
        
        print(f"Pre-padding (0-500ms) RMS: {get_rms_db(pre_chunk):.2f} dB")
        print(f"Post-padding (last 500ms) RMS: {get_rms_db(post_chunk):.2f} dB")
        
        # Also check Peak to see if there is ANY signal
        peak = max(abs(float(s)) for s in samples)
        peak_db = 20 * math.log10(peak / (2**(sampwidth*8-1))) if peak > 0 else -100.0
        print(f"Peak level: {peak_db:.2f} dB")
        
        w.close()
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    analyze(sys.argv[1])
