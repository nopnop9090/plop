use std::ffi::c_void;
use std::f32::consts::TAU;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, PeekMessageW, RegisterClassW,
    ShowWindow, TranslateMessage, UpdateLayeredWindow, MSG, PM_REMOVE, SW_SHOWNOACTIVATE,
    ULW_ALPHA, WM_QUIT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

pub static ACTIVE: AtomicUsize = AtomicUsize::new(0);
const MAX_ACTIVE: usize = 10;
const DURATION: f32 = 1.35;

const PALETTE: [[u8; 3]; 9] = [
    [232, 29, 54],   // red
    [255, 196, 12],  // yellow
    [46, 204, 64],   // green
    [41, 128, 237],  // blue
    [255, 105, 180], // pink
    [255, 127, 14],  // orange
    [171, 92, 255],  // purple
    [0, 214, 214],   // cyan
    [255, 250, 245], // white
];

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x2545F4914F6CDD1D);
        Rng(t ^ ((std::process::id() as u64) << 32) ^ 0x9E3779B97F4A7C15)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn f01(&mut self) -> f32 {
        ((self.next() >> 40) as f32) / ((1u64 << 24) as f32)
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f01()
    }
}

#[derive(Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r: f32,
    age: f32,
    ttl: f32,
    col: [u8; 3],
    shape: u8, // 0 = disc, 1 = spinning strip
    ang: f32,
    spin: f32,
    phase: f32,
}

/// Spawn a confetti burst overlay over `rect` (physical px). Non-blocking.
pub fn spawn_burst(rect: RECT) {
    if ACTIVE.load(Ordering::Relaxed) >= MAX_ACTIVE {
        return;
    }
    ACTIVE.fetch_add(1, Ordering::Relaxed);
    let spawned = std::thread::Builder::new()
        .name("plop-fx".into())
        .spawn(move || unsafe { run_overlay(rect) })
        .is_ok();
    if !spawned {
        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}

unsafe extern "system" fn stub_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn register_class() {
    static DONE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    DONE.get_or_init(|| unsafe {
        let hinstance = GetModuleHandleW(None).unwrap();
        let mut wc = WNDCLASSW::default();
        wc.lpfnWndProc = Some(stub_wndproc);
        wc.hInstance = hinstance.into();
        wc.lpszClassName = w!("PlopConfettiOverlay");
        let _ = RegisterClassW(&wc);
    });
}

unsafe fn run_overlay(rect: RECT) {
    unsafe {
        let rw = (rect.right - rect.left).max(1);
        let rh = (rect.bottom - rect.top).max(1);
        let cx = rect.left + rw / 2;
        let cy = rect.top + rh / 2;

        // Overlay covers the window plus generous fly-out margin, capped for perf.
        let ow = ((rw + 480).min(1500)).max(360);
        let oh = ((rh + 420).min(1150)).max(320);
        let ox = cx - ow / 2;
        let oy = cy - oh / 2;

        register_class();
        let hinstance = GetModuleHandleW(None).unwrap();

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW
                | WS_EX_TOPMOST,
            w!("PlopConfettiOverlay"),
            None,
            WS_POPUP,
            ox,
            oy,
            ow,
            oh,
            None,
            None,
            Some(hinstance.into()),
            None,
        );
        let hwnd = match hwnd {
            Ok(h) => h,
            Err(_) => {
                ACTIVE.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        };

        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(None);
        if hdc_screen.is_invalid() || hdc_mem.is_invalid() {
            ACTIVE.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = ow;
        bmi.bmiHeader.biHeight = -oh; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0; // BI_RGB == 0

        let mut bits: *mut c_void = std::ptr::null_mut();
        let hbmp = match CreateDIBSection(Some(hdc_mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(h) => h,
            Err(_) => {
                cleanup(hwnd, hdc_screen, hdc_mem, None);
                return;
            }
        };
        let old = SelectObject(hdc_mem, hbmp.into());

        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        // --- particles -------------------------------------------------
        let mut rng = Rng::new();
        let count = (((rw as i64) * (rh as i64) / 9000).clamp(130, 320)) as usize;
        let start_x = (cx - ox) as f32;
        let start_y = (cy - oy) as f32;
        let mut parts: Vec<Particle> = Vec::with_capacity(count);
        for _ in 0..count {
            let ang = rng.range(0.0, TAU);
            let speed = rng.range(140.0, 720.0);
            let shape = if rng.f01() < 0.45 { 1 } else { 0 };
            parts.push(Particle {
                x: start_x + rng.range(-14.0, 14.0),
                y: start_y + rng.range(-10.0, 10.0),
                vx: ang.cos() * speed,
                vy: ang.sin() * speed - 140.0, // slight upward bias
                r: rng.range(3.0, 7.0),
                age: 0.0,
                ttl: rng.range(0.85, DURATION),
                col: PALETTE[(rng.next() % PALETTE.len() as u64) as usize],
                shape,
                ang: rng.range(0.0, TAU),
                spin: rng.range(3.0, 11.0) * if rng.f01() < 0.5 { -1.0 } else { 1.0 },
                phase: rng.range(0.0, TAU),
            });
        }

        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let size = SIZE { cx: ow, cy: oh };
        let src_pt = POINT { x: 0, y: 0 };

        let started = Instant::now();
        let mut last = started;
        let mut quit = false;

        loop {
            // pump messages so the window behaves
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    quit = true;
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if quit {
                break;
            }

            let now = Instant::now();
            let dt = (now - last).as_secs_f32().min(0.033);
            last = now;
            let elapsed = (now - started).as_secs_f32();

            if bits.is_null() {
                break;
            }
            let buf =
                std::slice::from_raw_parts_mut(bits.cast::<u8>(), (ow * oh * 4) as usize);
            buf.fill(0);

            let mut alive = false;
            for p in &mut parts {
                p.age += dt;
                if p.age >= p.ttl {
                    continue;
                }
                alive = true;
                p.vy += 1500.0 * dt; // gravity
                p.vx += (p.age * 9.0 + p.phase).sin() * 26.0 * dt; // flutter
                p.vx *= 1.0 - 0.35 * dt;
                p.x += p.vx * dt;
                p.y += p.vy * dt;
                p.ang += p.spin * dt;

                let t = p.age / p.ttl;
                let alpha = ((1.0 - t) * (1.0 - t) * 255.0) as u8;
                let shrink = 1.0 - 0.55 * t;
                draw_particle(buf, ow, oh, p, alpha, shrink);
            }

            if !alive || elapsed > DURATION {
                break;
            }

            let _ = UpdateLayeredWindow(
                hwnd,
                Some(hdc_screen),
                None,
                Some(&size),
                Some(hdc_mem),
                Some(&src_pt),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            std::thread::sleep(Duration::from_millis(9));
        }
        // final fade-out frame
        let _ = UpdateLayeredWindow(
            hwnd,
            Some(hdc_screen),
            None,
            Some(&size),
            Some(hdc_mem),
            Some(&src_pt),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        cleanup(hwnd, hdc_screen, hdc_mem, Some((hbmp, old)));
    }
}

unsafe fn cleanup(
    hwnd: HWND,
    hdc_screen: windows::Win32::Graphics::Gdi::HDC,
    hdc_mem: windows::Win32::Graphics::Gdi::HDC,
    bmp: Option<(windows::Win32::Graphics::Gdi::HBITMAP, windows::Win32::Graphics::Gdi::HGDIOBJ)>,
) {
    unsafe {
        if let Some((h, old)) = bmp {
            SelectObject(hdc_mem, old);
            let _ = DeleteObject(h.into());
        }
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);
        let _ = DestroyWindow(hwnd);
        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}

fn draw_particle(buf: &mut [u8], w: i32, h: i32, p: &Particle, alpha: u8, shrink: f32) {
    let a = alpha as f32 / 255.0;
    let put = |buf: &mut [u8], px: i32, py: i32| {
        if px < 0 || py < 0 || px >= w || py >= h {
            return;
        }
        let idx = ((py * w + px) * 4) as usize;
        if idx + 3 >= buf.len() {
            return;
        }
        // premultiplied BGRA
        buf[idx] = (p.col[2] as f32 * a) as u8;
        buf[idx + 1] = (p.col[1] as f32 * a) as u8;
        buf[idx + 2] = (p.col[0] as f32 * a) as u8;
        buf[idx + 3] = alpha;
    };

    match p.shape {
        0 => {
            let r = (p.r * shrink).max(1.2);
            let rr = r * r;
            let cx = p.x as i32;
            let cy = p.y as i32;
            let ri = r.ceil() as i32;
            for dy in -ri..=ri {
                for dx in -ri..=ri {
                    let fx = dx as f32;
                    let fy = dy as f32;
                    if fx * fx + fy * fy <= rr {
                        put(buf, cx + dx, cy + dy);
                    }
                }
            }
        }
        _ => {
            // spinning strip: sample along main axis, small perpendicular width
            let half = (p.r * 2.3 * shrink).max(2.0);
            let ca = p.ang.cos();
            let sa = p.ang.sin();
            let steps = (half * 2.0) as i32;
            let bx = p.x - half * ca;
            let by = p.y - half * sa;
            for s in 0..=steps {
                let t = s as f32;
                let px = bx + t * ca;
                let py = by + t * sa;
                put(buf, px as i32, py as i32);
                put(buf, px as i32 + sa as i32, py as i32 - ca as i32);
            }
        }
    }
}
