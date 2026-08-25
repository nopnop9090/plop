"""Mixes the game-extracted pop + confetti sounds into combined WAVs for plop.

Input : assets/src_*.wav  (16-bit PCM stereo, extracted via vgmstream)
Output: assets/plop_sound_{0,1,2,3}.wav
        0-2: pop layered with one confetti variant each
        3:   confetti only (no pop) — so not every window-pop honks
"""

import os
import struct
import wave

ASSETS = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets"))

POP = "src_0186_PLR_ConfettiPop_01.wav"
CONFETTI = [
    "src_0033_PLR_Confetti_01.wav",
    "src_0072_PLR_Confetti_02.wav",
    "src_0775_confetti01.wav",
]


def load(name: str):
    with wave.open(os.path.join(ASSETS, name), "rb") as w:
        assert w.getsampwidth() == 2, f"{name}: 16-bit erwartet"
        sr = w.getframerate()
        ch = w.getnchannels()
        raw = w.readframes(w.getnframes())
    samples = struct.unpack(f"<{len(raw) // 2}h", raw)
    return samples, sr, ch


def write(name: str, samples: list, sr: int, ch: int) -> None:
    peak = max(1, max(abs(s) for s in samples))
    if peak > 32000:
        k = 32000 / peak
        samples = [int(s * k) for s in samples]
    with wave.open(os.path.join(ASSETS, name), "wb") as w:
        w.setnchannels(ch)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(struct.pack(f"<{len(samples)}h", *samples))


def main() -> None:
    pop, sr_p, ch_p = load(POP)
    for i, conf_name in enumerate(CONFETTI):
        conf, sr_c, ch_c = load(conf_name)
        assert sr_p == sr_c and ch_p == ch_c, "Format-Mismatch"
        n = max(len(pop), len(conf))
        mixed = [0] * n
        for j, s in enumerate(pop):
            mixed[j] += s
        for j, s in enumerate(conf):
            mixed[j] += s
        out = f"plop_sound_{i}.wav"
        write(out, mixed, sr_p, ch_p)
        print(f"{out}  ({n / sr_p:.2f}s, pop={len(pop)/sr_p:.2f}s + confetti={len(conf)/sr_p:.2f}s)")

    # variant without pop: plain confetti
    conf, sr_c, ch_c = load(CONFETTI[2])
    out = f"plop_sound_{len(CONFETTI)}.wav"
    write(out, list(conf), sr_c, ch_c)
    print(f"{out}  ({len(conf) / sr_c:.2f}s, confetti only)")


if __name__ == "__main__":
    main()
