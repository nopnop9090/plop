use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use windows::core::BOOL;
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, ANIMATIONINFO, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_GETANIMATION,
    SPI_SETANIMATION,
};

/// Disables the system-wide minimize/restore animation ("MinAnimate" via
/// SPI_GET/SETANIMATION) so windows vanish instantly and the confetti
/// effect reads better. The user's original setting is captured and
/// restored on exit.

static ORIGINAL: AtomicI32 = AtomicI32::new(-1);
static ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn init() {
    let mut ai = ANIMATIONINFO {
        cbSize: std::mem::size_of::<ANIMATIONINFO>() as u32,
        iMinAnimate: 0,
    };
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETANIMATION,
            std::mem::size_of::<ANIMATIONINFO>() as u32,
            Some((&mut ai as *mut ANIMATIONINFO).cast()),
            Default::default(),
        );
    }
    ORIGINAL.store(ai.iMinAnimate, Ordering::Relaxed);
    apply(true);
}

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

pub fn set_enabled(on: bool) {
    apply(on);
}

pub fn restore() {
    if ACTIVE.load(Ordering::Relaxed) {
        apply(false);
    }
}

fn apply(on: bool) {
    let target = if on {
        0
    } else {
        ORIGINAL.load(Ordering::Relaxed).max(0)
    };
    let mut ai = ANIMATIONINFO {
        cbSize: std::mem::size_of::<ANIMATIONINFO>() as u32,
        iMinAnimate: target,
    };
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_SETANIMATION,
            std::mem::size_of::<ANIMATIONINFO>() as u32,
            Some((&mut ai as *mut ANIMATIONINFO).cast()),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        );
    }
    ACTIVE.store(on, Ordering::Relaxed);
}

// BOOL import kept for potential future SPI calls; silence unused warning.
#[allow(dead_code)]
type _BoolAlias = BOOL;
