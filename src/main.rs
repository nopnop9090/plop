#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod confetti;
mod config;
mod debuglog;
mod filter;
mod minanim;
mod sound;
mod tray;
mod watch;

use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::w;
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::Media::timeBeginPeriod;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};

pub static ENABLED: AtomicBool = AtomicBool::new(true);
pub static SOUND_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn sound_enabled() -> bool {
    SOUND_ENABLED.load(Ordering::Relaxed)
}

/// e.g. "1.0.260825.b12"
pub const FULL_VERSION: &str = concat!(
    env!("PLOP_VERSION"),
    ".",
    env!("PLOP_DATE_CODE"),
    ".b",
    env!("PLOP_BUILD_NO")
);

fn main() {
    unsafe {
        // Physical-pixel coordinates everywhere (multi-monitor / scaling safe).
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // ~1ms sleep granularity for smooth confetti animation.
        let _ = timeBeginPeriod(1);

        // Single instance guard; leak the handle so it lives for the process lifetime.
        match CreateMutexW(None, false, w!("Local\\Plop.Toy.Single.Instance")) {
            Ok(h) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    return;
                }
                let _ = h; // never closed: mutex lives for the process lifetime
            }
            Err(_) => return,
        }

        // Kill the minimize/restore animation (restored on exit).
        minanim::init();

        std::thread::Builder::new()
            .name("plop-watch".into())
            .spawn(watch::run)
            .expect("failed to spawn watch thread");

        tray::run();
    }
}
