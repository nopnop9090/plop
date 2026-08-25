use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetAncestor, GetClassNameW, GetWindowLongPtrW, GetWindowPlacement, GetWindowThreadProcessId,
    GetWindowRect, IsIconic, IsWindowVisible, GA_ROOT, GWL_EXSTYLE, GWL_STYLE, WINDOWPLACEMENT,
    WS_CAPTION, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

/// Heuristic for "normal app window": visible top-level root window with a
/// taskbar-ish presence (caption or WS_EX_APPWINDOW), not cloaked (UWP
/// suspended shells), not a tool/shell window.
pub unsafe fn eligible(hwnd: HWND) -> bool {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            return false;
        }
        eligible_lenient(hwnd)
    }
}

/// Same as `eligible` but also accepts minimized windows (for the
/// reconciliation sweep; the rect then comes from GetWindowPlacement).
pub unsafe fn eligible_lenient(hwnd: HWND) -> bool {
    unsafe {
        if hwnd.is_invalid() {
            return false;
        }
        if GetAncestor(hwnd, GA_ROOT) != hwnd {
            return false;
        }
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        if cloaked(hwnd) {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return false;
        }
        if is_shell_class(hwnd) {
            return false;
        }

        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let exstyle = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if exstyle & WS_EX_TOOLWINDOW.0 != 0 {
            return false;
        }
        let caption = style & WS_CAPTION.0 != 0;
        let appwin = exstyle & WS_EX_APPWINDOW.0 != 0;
        if !(caption || appwin) {
            return false;
        }

        let mut r = RECT::default();
        if GetWindowRect(hwnd, &mut r).is_err() {
            return false;
        }
        let (w, h) = if IsIconic(hwnd).as_bool() {
            // Minimized: GetWindowRect is meaningless, use the restore rect.
            match normal_rect(hwnd) {
                Some(nr) => (nr.right - nr.left, nr.bottom - nr.top),
                None => (0, 0),
            }
        } else {
            (r.right - r.left, r.bottom - r.top)
        };
        w > 80 && h > 60
    }
}

/// The window's normal (restored) rectangle, valid even while minimized.
pub unsafe fn normal_rect(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let mut wp = WINDOWPLACEMENT::default();
        wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
        match GetWindowPlacement(hwnd, &mut wp) {
            Ok(()) if wp.rcNormalPosition.right > wp.rcNormalPosition.left => {
                Some(wp.rcNormalPosition)
            }
            _ => None,
        }
    }
}

pub fn class_name(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut buf = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut buf);
        if n <= 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..n as usize]))
    }
}

/// Process file name (e.g. "spotify.exe") that owns the window, lowercase.
pub unsafe fn process_exe_name(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => h,
            Err(_) => return None,
        };
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let r = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        r.ok()?;
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        Some(
            full.rsplit(['\\', '/'])
                .next()
                .unwrap_or("")
                .to_lowercase(),
        )
    }
}

unsafe fn cloaked(hwnd: HWND) -> bool {
    unsafe {
        let mut v = 0u32;
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut v as *mut u32).cast(),
            std::mem::size_of::<u32>() as u32,
        );
        hr.is_ok() && v != 0
    }
}

unsafe fn is_shell_class(hwnd: HWND) -> bool {
    unsafe {
        let mut buf = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut buf);
        if n <= 0 {
            return false;
        }
        let cls = String::from_utf16_lossy(&buf[..n as usize]);
        matches!(
            cls.as_str(),
            "Progman"
                | "WorkerW"
                | "Shell_TrayWnd"
                | "Shell_SecondaryTrayWnd"
                | "Windows.UI.Core.CoreWindow"
                | "XamlExplorerHostIslandWindow"
                | "TopLevelWindowForOverflowXamlIsland"
        )
    }
}
