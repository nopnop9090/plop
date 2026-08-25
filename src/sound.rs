use std::sync::atomic::{AtomicU64, Ordering};

use windows::core::PCWSTR;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

const SOUNDS: [&[u8]; 4] = [
    include_bytes!("../assets/plop_sound_0.wav"),
    include_bytes!("../assets/plop_sound_1.wav"),
    include_bytes!("../assets/plop_sound_2.wav"),
    include_bytes!("../assets/plop_sound_3.wav"),
];

static LAST_MS: AtomicU64 = AtomicU64::new(0);

/// Play a random sound variant (3x pop+confetti, 1x confetti-only).
/// SND_ASYNC returns immediately; PlaySound cuts off the previous sound,
/// so we throttle rapid-fire closes a little.
pub fn play_honk() {
    let now = now_ms();
    let last = LAST_MS.load(Ordering::Relaxed);
    if now.wrapping_sub(last) < 60 {
        return;
    }
    LAST_MS.store(now, Ordering::Relaxed);

    let idx = (now % SOUNDS.len() as u64) as usize;
    let data = SOUNDS[idx];
    unsafe {
        // PlaySoundW takes PCWSTR but with SND_MEMORY it is really a pointer
        // to an in-memory WAV image.
        let _ = PlaySoundW(
            PCWSTR(data.as_ptr().cast::<u16>()),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
