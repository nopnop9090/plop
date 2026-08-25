use std::io::Write;

use windows::Win32::Foundation::HWND;

/// Optional pop logging for diagnostics: set PLOP_DEBUG=1 and watch
/// %TEMP%\plop_debug.log to attribute every pop to a window + reason.
pub fn log(reason: &str, hwnd: Option<HWND>) {
    if std::env::var("PLOP_DEBUG").unwrap_or_default().is_empty() {
        return;
    }
    let what = hwnd
        .and_then(|h| {
            unsafe { crate::filter::process_exe_name(h) }
        })
        .unwrap_or_else(|| "?".into());
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = std::env::temp_dir().join("plop_debug.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{ms} {reason} exe={what}");
    }
}
