use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, GetMessageW, GetWindowRect, IsIconic, IsWindowVisible,
    EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE, EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_SHOW,
    EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, OBJID_WINDOW, TranslateMessage, MSG,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};

use crate::confetti;
use crate::config;
use crate::filter;
use crate::sound;

/// How long a window may have been hidden before its destroy stops popping
/// (covers hide-then-destroy close sequences; tray apps hide much earlier).
const HIDE_GRACE: Duration = Duration::from_secs(2);

/// Self-healing sweep interval. Also the max extra delay for pops from
/// elevated processes, whose WinEvents never reach us (UIPI).
const SWEEP_MS: u32 = 500;

struct Entry {
    rect: RECT,
    hidden_at: Option<Instant>,
    /// Already popped for the current minimized state; cleared on any
    /// observation of the window being restored (non-iconic).
    minimized: bool,
}

/// Last known state of eligible top-level windows, keyed by HWND value.
static CACHE: LazyLock<Mutex<HashMap<isize, Entry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn run() {
    unsafe {
        // Narrow hooks instead of one wide range: less event traffic, fewer
        // dropped events.
        let hooks = [
            SetWinEventHook(
                EVENT_SYSTEM_MINIMIZESTART,
                EVENT_SYSTEM_MINIMIZEEND,
                None,
                Some(on_winevent),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            ),
            SetWinEventHook(
                EVENT_OBJECT_DESTROY, // 0x8001 .. 0x8003 covers DESTROY, SHOW, HIDE
                EVENT_OBJECT_HIDE,
                None,
                Some(on_winevent),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            ),
            SetWinEventHook(
                EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_OBJECT_LOCATIONCHANGE,
                None,
                Some(on_winevent),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            ),
        ];
        if hooks.iter().all(|h| h.is_invalid()) {
            return;
        }

        snapshot();

        // Self-healing sweep on its own thread: re-syncs the cache every
        // SWEEP_MS regardless of missed/undelivered events (UIPI, drops).
        std::thread::Builder::new()
            .name("plop-sweep".into())
            .spawn(|| loop {
                std::thread::sleep(Duration::from_millis(SWEEP_MS as u64));
                sweep();
            })
            .expect("failed to spawn sweep thread");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn on_winevent(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    idobject: i32,
    idchild: i32,
    _thread: u32,
    _time: u32,
) {
    unsafe {
        if idobject != OBJID_WINDOW.0 || idchild != 0 || hwnd.is_invalid() {
            return;
        }
        let key = hwnd.0 as isize;
        if event == EVENT_OBJECT_DESTROY {
            let entry = CACHE.lock().unwrap().remove(&key);
            if let Some(e) = entry {
                // Windows hidden long ago (tray apps) vanish silently.
                let recent = e.hidden_at.map_or(true, |t| t.elapsed() < HIDE_GRACE);
                if recent {
                    pop("destroy-event", Some(hwnd), e.rect);
                }
            }
        } else if event == EVENT_SYSTEM_MINIMIZESTART {
            let entry = CACHE
                .lock()
                .unwrap()
                .get(&key)
                .map(|e| (e.rect, e.hidden_at.is_none(), e.minimized));
            if let Some((r, visible, already)) = entry {
                if visible && !already {
                    if let Some(e) = CACHE.lock().unwrap().get_mut(&key) {
                        e.minimized = true;
                    }
                    pop("minimize-event", Some(hwnd), r);
                }
            }
        } else if event == EVENT_SYSTEM_MINIMIZEEND {
            if let Some(e) = CACHE.lock().unwrap().get_mut(&key) {
                if !IsIconic(hwnd).as_bool() {
                    e.minimized = false;
                }
                if let Some(r) = get_rect(hwnd) {
                    e.rect = r;
                }
            }
        } else if event == EVENT_OBJECT_HIDE {
            if config::is_hide_as_close(hwnd) {
                // Close-to-tray app: hiding counts as closing.
                let entry = CACHE.lock().unwrap().remove(&key);
                if let Some(e) = entry {
                    pop("hide-as-close", Some(hwnd), e.rect);
                }
            } else if let Some(e) = CACHE.lock().unwrap().get_mut(&key) {
                e.hidden_at = Some(Instant::now());
            }
        } else if event == EVENT_OBJECT_SHOW {
            let mut map = CACHE.lock().unwrap();
            if let Some(e) = map.get_mut(&key) {
                // Re-shown after a temporary hide.
                e.hidden_at = None;
                e.minimized = false;
                if let Some(r) = get_rect(hwnd) {
                    e.rect = r;
                }
            } else if filter::eligible(hwnd)
                && !config::is_ignored(hwnd)
                && let Some(r) = get_rect(hwnd)
            {
                map.insert(key, Entry { rect: r, hidden_at: None, minimized: false });
            }
        } else if event == EVENT_OBJECT_LOCATIONCHANGE {
            let mut map = CACHE.lock().unwrap();
            if let Some(e) = map.get_mut(&key) {
                // Keep last good (non-minimized, non-hidden) rect fresh.
                if IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() {
                    e.hidden_at = None;
                    e.minimized = false;
                    if let Some(r) = get_rect(hwnd) {
                        e.rect = r;
                    }
                }
            }
        }
    }
}

/// Periodic reconciliation. Fixes everything the event stream missed:
/// windows that never sent SHOW (started minimized / restored silently),
/// elevated processes (UIPI hides their events entirely), stale hidden
/// flags and dropped DESTROY/MINIMIZESTART events.
fn sweep() {
    unsafe {
        let mut wins: Vec<HWND> = Vec::new();
        let _ = EnumWindows(Some(enum_collect), LPARAM(&mut wins as *mut _ as isize));
        let now = Instant::now();
        let mut pops: Vec<(&'static str, Option<HWND>, RECT)> = Vec::new();

        {
            let mut map = CACHE.lock().unwrap();
            let live: HashSet<isize> = wins.iter().map(|h| h.0 as isize).collect();

            // Destroyed windows: pop unless hidden for a while (tray apps).
            map.retain(|key, e| {
                if live.contains(key) {
                    true
                } else {
                    let recent = e.hidden_at.map_or(true, |t| now.duration_since(t) < HIDE_GRACE);
                    if recent {
                        pops.push(("sweep-destroy", None, e.rect));
                    }
                    false
                }
            });

            for h in &wins {
                let key = h.0 as isize;
                let Some(e) = map.get_mut(&key) else { continue };
                let visible = IsWindowVisible(*h).as_bool();
                let iconic = IsIconic(*h).as_bool();
                if !visible {
                    e.hidden_at.get_or_insert(now);
                } else if iconic {
                    if !e.minimized {
                        e.minimized = true;
                        pops.push(("sweep-minimize", Some(*h), e.rect));
                    }
                } else {
                    e.minimized = false;
                    e.hidden_at = None;
                    if let Some(r) = get_rect(*h) {
                        e.rect = r;
                    }
                }
            }

            // Windows we never saw an event for (missed SHOW, elevated apps,
            // minimized at startup). Insert minimized ones with their restore
            // rect so a later minimize still pops at the right place.
            for h in &wins {
                let key = h.0 as isize;
                if map.contains_key(&key) {
                    continue;
                }
                if !filter::eligible_lenient(*h) || config::is_ignored(*h) {
                    continue;
                }
                let iconic = IsIconic(*h).as_bool();
                let rect = if iconic {
                    match filter::normal_rect(*h) {
                        Some(r) => r,
                        None => continue,
                    }
                } else {
                    match get_rect(*h) {
                        Some(r) => r,
                        None => continue,
                    }
                };
                let minimized = iconic;
                map.insert(key, Entry { rect, hidden_at: None, minimized });
            }
        }

        for (reason, hwnd, rect) in pops {
            pop(reason, hwnd, rect);
        }
    }
}

fn pop(reason: &'static str, hwnd: Option<HWND>, rect: RECT) {
    if !crate::enabled() {
        return;
    }
    crate::debuglog::log(reason, hwnd);
    if crate::sound_enabled() {
        sound::play_honk();
    }
    confetti::spawn_burst(rect);
}

fn get_rect(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let mut r = RECT::default();
        match GetWindowRect(hwnd, &mut r) {
            Ok(()) => Some(r),
            Err(_) => None,
        }
    }
}

unsafe extern "system" fn enum_collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let list = &mut *(lparam.0 as *mut Vec<HWND>);
        list.push(hwnd);
        true.into()
    }
}

/// Seed the cache with windows that already exist at startup so closing them pops too.
fn snapshot() {
    sweep();
}
