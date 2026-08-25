use std::sync::atomic::Ordering;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::CreateBitmap;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, ChangeWindowMessageFilterEx, CreateIconIndirect, CreatePopupMenu,
    CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetMessageW, LoadCursorW, LoadImageW, MSGFLT_ALLOW, PostMessageW, PostQuitMessage,
    RegisterClassW, SetForegroundWindow, ShowWindow, TranslateMessage, TrackPopupMenuEx, HICON,
    HMENU, ICONINFO, IDC_ARROW, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR, MF_CHECKED,
    MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG, SW_HIDE, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_DESTROY, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

use crate::confetti;
use crate::sound;
use crate::ENABLED;

// ---- IPC protocol (WM_USER based, see docs/IPC.md) --------------------
const PM_PING: u32 = 0x0400; // WM_USER + 0
const PM_GET_STATE: u32 = 0x0400 + 1;
const PM_SET_ENABLED: u32 = 0x0400 + 2;
const PM_SET_SOUND: u32 = 0x0400 + 3;
const PM_SET_MINANIM: u32 = 0x0400 + 4;
const PM_TEST_POP: u32 = 0x0400 + 5;
const PM_GET_VERSION: u32 = 0x0400 + 6;
const PM_MAGIC: usize = 0x504C_4F50; // 'PLOP'

const PMF_ENABLED: u32 = 0x1;
const PMF_SOUND: u32 = 0x2;
const PMF_MINANIM_OFF: u32 = 0x4;

const ID_VERSION: isize = 0;
const ID_TOGGLE: isize = 100;
const ID_TEST: isize = 101;
const ID_EXIT: isize = 102;
const ID_MINANIM: isize = 103;
const ID_SOUND: isize = 104;
const CALLBACK_MSG: u32 = 0x8000 + 7; // WM_APP + 7
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONUP: u32 = 0x0204;

pub fn run() {
    unsafe {
        let hinstance = GetModuleHandleW(None).unwrap();

        let cls = w!("PlopTrayHost");
        let mut wc = WNDCLASSW::default();
        wc.lpfnWndProc = Some(tray_wndproc);
        wc.hInstance = hinstance.into();
        wc.hCursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        wc.lpszClassName = cls;
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            Default::default(),
            cls,
            None,
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("tray host window");

        let _ = ShowWindow(hwnd, SW_HIDE);

        add_tray_icon(hwnd);
        allow_ipc_messages(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Small hand-drawn confetti-dots tray icon (16x16, opaque dark background).
unsafe fn make_icon() -> HICON {
    unsafe {
        let mut buf = [0u8; 16 * 16 * 4];
        for i in 0..(16 * 16) {
            buf[i * 4] = 0x30; // B
            buf[i * 4 + 1] = 0x24; // G
            buf[i * 4 + 2] = 0x20; // R
            buf[i * 4 + 3] = 255;
        }
        let dots: [(usize, usize, [u8; 3]); 10] = [
            (2, 2, [236, 28, 54]),
            (7, 1, [255, 199, 12]),
            (12, 3, [41, 128, 237]),
            (1, 7, [46, 204, 64]),
            (6, 6, [255, 105, 180]),
            (11, 8, [255, 127, 14]),
            (3, 12, [0, 214, 214]),
            (9, 12, [171, 92, 255]),
            (13, 13, [255, 250, 245]),
            (5, 9, [232, 29, 54]),
        ];
        for (dx, dy, col) in dots {
            for oy in 0..2 {
                for ox in 0..2 {
                    let x = dx + ox;
                    let y = dy + oy;
                    if x < 16 && y < 16 {
                        let i = (y * 16 + x) * 4;
                        buf[i] = col[2];
                        buf[i + 1] = col[1];
                        buf[i + 2] = col[0];
                    }
                }
            }
        }
        let color = CreateBitmap(16, 16, 1, 32, Some(buf.as_ptr().cast()));
        let mask = CreateBitmap(16, 16, 1, 1, None);

        let mut ii = ICONINFO::default();
        ii.fIcon = true.into();
        ii.xHotspot = 0;
        ii.yHotspot = 0;
        ii.hbmMask = mask;
        ii.hbmColor = color;
        match CreateIconIndirect(&ii) {
            Ok(icon) => return icon,
            Err(_) => {}
        }

        // fallback: stock application icon
        match LoadImageW(
            None,
            IDI_APPLICATION,
            IMAGE_ICON,
            16,
            16,
            LR_DEFAULTCOLOR,
        ) {
            Ok(obj) => HICON(obj.0),
            Err(_) => HICON::default(),
        }
    }
}

unsafe fn add_tray_icon(hwnd: HWND) {
    unsafe {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = CALLBACK_MSG;
        nid.hIcon = make_icon();
        set_tip(&mut nid, &format!("Plop! {} \u{2013} Fenster-Konfetti", crate::FULL_VERSION));
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

unsafe fn modify_tip(hwnd: HWND, text: &str) {
    unsafe {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_TIP;
        set_tip(&mut nid, text);
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

fn set_tip(nid: &mut NOTIFYICONDATAW, s: &str) {
    let v: Vec<u16> = s.encode_utf16().take(127).collect();
    nid.szTip[..v.len()].copy_from_slice(&v);
    nid.szTip[v.len()] = 0;
}

unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if (0x0400..=0x0410).contains(&msg) {
            return handle_ipc(msg, wparam);
        }
        if msg == CALLBACK_MSG {
            let mouse = (lparam.0 & 0xFFFF) as u32;
            if mouse == WM_LBUTTONUP {
                toggle_enabled(hwnd);
            } else if mouse == WM_RBUTTONUP {
                show_menu(hwnd);
            }
            return LRESULT(0);
        }
        if msg == WM_DESTROY {
            crate::minanim::restore();
            let mut nid = NOTIFYICONDATAW::default();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            PostQuitMessage(0);
            return LRESULT(0);
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

/// External control interface: any process can FindWindowW("PlopTrayHost")
/// and SendMessage the PM_* messages (docs/IPC.md).
unsafe fn handle_ipc(msg: u32, wparam: WPARAM) -> LRESULT {
    unsafe {
        match msg {
            PM_PING => LRESULT(PM_MAGIC as isize),
            PM_GET_STATE => {
                let mut st = 0u32;
                if crate::enabled() {
                    st |= PMF_ENABLED;
                }
                if crate::sound_enabled() {
                    st |= PMF_SOUND;
                }
                if crate::minanim::is_active() {
                    st |= PMF_MINANIM_OFF;
                }
                LRESULT(st as isize)
            }
            PM_SET_ENABLED => {
                let prev = crate::enabled() as isize;
                crate::ENABLED.store(wparam.0 != 0, Ordering::Relaxed);
                LRESULT(prev)
            }
            PM_SET_SOUND => {
                let prev = crate::sound_enabled() as isize;
                crate::SOUND_ENABLED.store(wparam.0 != 0, Ordering::Relaxed);
                LRESULT(prev)
            }
            PM_SET_MINANIM => {
                let prev = crate::minanim::is_active() as isize;
                crate::minanim::set_enabled(wparam.0 != 0);
                LRESULT(prev)
            }
            PM_TEST_POP => {
                test_pop();
                LRESULT(1)
            }
            PM_GET_VERSION => {
                let major: u32 = env!("PLOP_MAJOR").parse().unwrap_or(0);
                let minor: u32 = env!("PLOP_MINOR").parse().unwrap_or(0);
                let build: u32 = env!("PLOP_BUILD_NO").parse().unwrap_or(0);
                LRESULT((((major << 24) | (minor << 16) | (build & 0xFFFF)) as u32) as isize)
            }
            _ => LRESULT(0),
        }
    }
}

/// Allow these messages even if plop ever runs elevated (UIPI).
unsafe fn allow_ipc_messages(hwnd: HWND) {
    unsafe {
        for m in [
            PM_PING,
            PM_GET_STATE,
            PM_SET_ENABLED,
            PM_SET_SOUND,
            PM_SET_MINANIM,
            PM_TEST_POP,
            PM_GET_VERSION,
        ] {
            let _ = ChangeWindowMessageFilterEx(hwnd, m, MSGFLT_ALLOW, None);
        }
    }
}

unsafe fn toggle_enabled(hwnd: HWND) {
    let now = !crate::enabled();
    ENABLED.store(now, Ordering::Relaxed);
    unsafe {
        if now {
            modify_tip(hwnd, "Plop! \u{2013} Fenster-Konfetti (aktiv)");
        } else {
            modify_tip(hwnd, "Plop! \u{2013} pausiert");
        }
    }
}

unsafe fn test_pop() {
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let r = RECT {
            left: pt.x - 170,
            top: pt.y - 130,
            right: pt.x + 170,
            bottom: pt.y + 130,
        };
        crate::debuglog::log("test-pop", None);
        if crate::sound_enabled() {
            sound::play_honk();
        }
        confetti::spawn_burst(r);
    }
}

/// NUL-terminated UTF-16 for dynamic menu labels.
fn pcw(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn show_menu(hwnd: HWND) {
    unsafe {
        let menu: HMENU = match CreatePopupMenu() {
            Ok(m) => m,
            Err(_) => return,
        };

        let version_label = pcw(&format!("Plop! v{}", crate::FULL_VERSION));
        let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, ID_VERSION as usize, PCWSTR(version_label.as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0usize, PCWSTR::null());
        let active_flag = if crate::enabled() { MF_CHECKED } else { Default::default() };
        let _ = AppendMenuW(menu, active_flag | MF_STRING, ID_TOGGLE as usize, w!("Aktiv"));
        let sound_flag = if crate::sound_enabled() { MF_CHECKED } else { Default::default() };
        let _ = AppendMenuW(menu, sound_flag | MF_STRING, ID_SOUND as usize, w!("Ton"));
        let minanim_flag = if crate::minanim::is_active() { MF_CHECKED } else { Default::default() };
        let _ = AppendMenuW(menu, minanim_flag | MF_STRING, ID_MINANIM as usize, w!("Minimier-Animation aus"));
        let _ = AppendMenuW(menu, MF_STRING, ID_TEST as usize, w!("Test-Pop"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0usize, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_EXIT as usize, w!("Beenden"));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);

        // TPM_RETURNCMD makes this return the chosen command instead of posting WM_COMMAND.
        let picked = TrackPopupMenuEx(
            menu,
            (TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY).0,
            pt.x,
            pt.y,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), 0 /* WM_NULL */, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);

        match picked.0 as isize {
            ID_TOGGLE => toggle_enabled(hwnd),
            ID_SOUND => {
                let now = !crate::sound_enabled();
                crate::SOUND_ENABLED.store(now, Ordering::Relaxed);
            }
            ID_MINANIM => crate::minanim::set_enabled(!crate::minanim::is_active()),
            ID_TEST => test_pop(),
            ID_EXIT => {
                let _ = DestroyWindow(hwnd);
            }
            _ => {}
        }
    }
}
