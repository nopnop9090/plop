use std::collections::HashSet;
use std::sync::LazyLock;

use crate::filter;

/// Optional config file (plop.ini next to plop.exe):
///
///   [ignore]        -> windows that never pop (class=Foo / exe=app.exe)
///   [hide_as_close] -> apps that hide instead of closing (close-to-tray):
///                      hiding counts as closing and pops (class=Foo / exe=app.exe)
///
/// Keys are `class=` (window class, case-insensitive) or `exe=` (process
/// file name, case-insensitive). `#`/`;` start comments.

pub struct Config {
    pub ignore_class: HashSet<String>,
    pub ignore_exe: HashSet<String>,
    pub hide_close_class: HashSet<String>,
    pub hide_close_exe: HashSet<String>,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(load);

fn load() -> Config {
    let mut c = Config {
        ignore_class: HashSet::new(),
        ignore_exe: HashSet::new(),
        hide_close_class: HashSet::new(),
        hide_close_exe: HashSet::new(),
    };
    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("plop.ini")));
    let Some(path) = path else { return c };
    let Ok(text) = std::fs::read_to_string(&path) else { return c };

    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_lowercase();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let key = k.trim().to_lowercase();
        let val = v.trim().to_lowercase();
        if val.is_empty() {
            continue;
        }
        match section.as_str() {
            "ignore" => match key.as_str() {
                "class" => {
                    c.ignore_class.insert(val);
                }
                "exe" => {
                    c.ignore_exe.insert(val);
                }
                _ => {}
            },
            "hide_as_close" => match key.as_str() {
                "class" => {
                    c.hide_close_class.insert(val);
                }
                "exe" => {
                    c.hide_close_exe.insert(val);
                }
                _ => {}
            },
            _ => {}
        }
    }
    c
}

pub unsafe fn is_ignored(hwnd: windows::Win32::Foundation::HWND) -> bool {
    unsafe {
        let c = &*CONFIG;
        if c.ignore_class.is_empty() && c.ignore_exe.is_empty() {
            return false;
        }
        if let Some(cls) = filter::class_name(hwnd) {
            if c.ignore_class.contains(&cls.to_lowercase()) {
                return true;
            }
        }
        if let Some(exe) = filter::process_exe_name(hwnd) {
            if c.ignore_exe.contains(&exe) {
                return true;
            }
        }
        false
    }
}

pub unsafe fn is_hide_as_close(hwnd: windows::Win32::Foundation::HWND) -> bool {
    unsafe {
        let c = &*CONFIG;
        if c.hide_close_class.is_empty() && c.hide_close_exe.is_empty() {
            return false;
        }
        if let Some(cls) = filter::class_name(hwnd) {
            if c.hide_close_class.contains(&cls.to_lowercase()) {
                return true;
            }
        }
        if let Some(exe) = filter::process_exe_name(hwnd) {
            if c.hide_close_exe.contains(&exe) {
                return true;
            }
        }
        false
    }
}
