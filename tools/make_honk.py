"""Synthesizes royalty-free clown-honk / party-horn WAVs into ../assets/.

Pure stdlib. Classic double-burst honk: harmonic-rich buzzy tone with fast
attack, pitch bend, breath noise and soft clipping for brassiness.
"""

import math
import os
import random
import struct
import wave

SR = 44100


def synth_honk(base_hz: float, seed: int) -> bytes:
    total = 0.45
    n = int(SR * total)
    buf = [0.0] * n
    rnd = random.Random(seed)
    bursts = [
        (0.00, 0.13, base_hz),
        (0.20, 0.16, base_hz * 1.07),
    ]
    for start, dur, f0 in bursts:
        s0 = int(start * SR)
        ln = int(dur * SR)
        phase = 0.0
        for i in range(ln):
            t = i / ln
            bend = 1.0 + 0.04 * math.sin(t * math.pi) - 0.12 * max(0.0, (t - 0.65) / 0.35)
            f = f0 * bend
            phase += 2.0 * math.pi * f / SR
            v = 0.0
            for h in range(1, 17):
                amp = 1.0 / (h ** 1.22)
                if h % 2 == 0:
                    amp *= 0.55
                v += amp * math.sin(phase * h)
            v /= 9.0
            v += 0.06 * (rnd.random() * 2.0 - 1.0)
            attack = min(1.0, i / (0.007 * SR))
            release = min(1.0, (ln - i) / (0.045 * SR))
            env = attack * release
            env *= 0.85 + 0.15 * math.sin(2.0 * math.pi * 27.0 * i / SR)
            buf[s0 + i] += v * env
    out = bytearray()
    for v in buf:
        v = math.tanh(v * 1.7) * 0.88
        out += struct.pack("<h", int(max(-1.0, min(1.0, v)) * 32000))
    return bytes(out)


def write_wav(path: str, pcm: bytes) -> None:
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm)


def main() -> None:
    here = os.path.dirname(os.path.abspath(__file__))
    assets = os.path.normpath(os.path.join(here, "..", "assets"))
    os.makedirs(assets, exist_ok=True)
    for i, hz in enumerate((295.0, 340.0, 392.0)):
        path = os.path.join(assets, f"honk_{i}.wav")
        write_wav(path, synth_honk(hz, seed=1000 + i))
        print(f"{path}  ({hz:.0f} Hz)")


if __name__ == "__main__":
    main()
