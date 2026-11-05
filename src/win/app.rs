//! Windows 版の窓とメッセージループ（Win32）。描画は win/gfx.rs が Direct3D11 / Direct2D で行う。
//! ゲームの中身（シミュレーション・AI・画面）はブラウザ版と共通。

use crate::{crashlog, game, gfx, lang, platform, Ev};
use std::cell::RefCell;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

thread_local! {
    static EVENTS: RefCell<Vec<Ev>> = const { RefCell::new(Vec::new()) };
}

fn push(e: Ev) {
    EVENTS.with(|v| v.borrow_mut().push(e));
}

fn lp_xy(lp: LPARAM) -> (f32, f32) {
    let x = (lp.0 & 0xffff) as i16 as f32;
    let y = ((lp.0 >> 16) & 0xffff) as i16 as f32;
    (x, y)
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_SIZE => {
                let w = (lp.0 & 0xffff) as u32;
                let h = ((lp.0 >> 16) & 0xffff) as u32;
                push(Ev::Resize(w, h));
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                let (x, y) = lp_xy(lp);
                push(Ev::MouseMove(x, y));
                LRESULT(0)
            }
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
                let (x, y) = lp_xy(lp);
                SetCapture(hwnd);
                let b = match msg {
                    WM_LBUTTONDOWN => 0,
                    WM_RBUTTONDOWN => 1,
                    _ => 2,
                };
                push(Ev::Down(b, x, y));
                LRESULT(0)
            }
            WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => {
                let (x, y) = lp_xy(lp);
                let _ = ReleaseCapture();
                let b = match msg {
                    WM_LBUTTONUP => 0,
                    WM_RBUTTONUP => 1,
                    _ => 2,
                };
                push(Ev::Up(b, x, y));
                LRESULT(0)
            }
            WM_MOUSEWHEEL => {
                let d = ((wp.0 >> 16) & 0xffff) as i16 as f32 / 120.0;
                push(Ev::Wheel(d));
                LRESULT(0)
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                if wp.0 as u32 == VK_F4.0 as u32 && msg == WM_SYSKEYDOWN {
                    return DefWindowProcW(hwnd, msg, wp, lp);
                }
                push(Ev::Key(wp.0 as u32, true));
                LRESULT(0)
            }
            WM_KEYUP | WM_SYSKEYUP => {
                push(Ev::Key(wp.0 as u32, false));
                LRESULT(0)
            }
            WM_CHAR => {
                if let Some(c) = char::from_u32(wp.0 as u32) {
                    push(Ev::Char(c));
                }
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

pub fn main() {
    crashlog::init();
    match std::panic::catch_unwind(run) {
        Ok(Ok(())) => crashlog::clean_exit(),
        Ok(Err(e)) => {
            crashlog::report_failure(&format!("{:?}", e));
            std::process::exit(1);
        }
        Err(_) => {
            crashlog::report_failure("パニック");
            std::process::exit(101);
        }
    }
}

fn run() -> Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        lang::set_lang(platform::initial_lang());
        let inst = GetModuleHandleW(None)?;
        let class = w!("YYWW1Wnd");
        // アプリのアイコン（build.rs が assets/app_icon.ico を資源番号1として埋め込む）
        let icon = |sx: i32, sy: i32| -> HICON {
            LoadImageW(Some(inst.into()), PCWSTR(1 as *const u16), IMAGE_ICON, sx, sy, LR_DEFAULTCOLOR)
                .map(|h| HICON(h.0))
                .unwrap_or_default()
        };
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: inst.into(),
            hIcon: icon(GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON)),
            hIconSm: icon(GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON)),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassExW(&wc);
        let (w, h) = (1600, 900);
        let mut rc = RECT { left: 0, top: 0, right: w, bottom: h };
        let _ = AdjustWindowRect(&mut rc, WS_OVERLAPPEDWINDOW, false);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            &HSTRING::from(lang::window_title()),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            rc.right - rc.left,
            rc.bottom - rc.top,
            None,
            None,
            Some(inst.into()),
            None,
        )?;
        let mut cr = RECT::default();
        GetClientRect(hwnd, &mut cr)?;
        let gfx = gfx::Gfx::new(hwnd, (cr.right - cr.left).max(1) as u32, (cr.bottom - cr.top).max(1) as u32)?;
        let mut game = game::Game::new(gfx);
        let mut last = std::time::Instant::now();
        let mut state_at = last;
        crashlog::set_state(game.state_text());
        let mut msg = MSG::default();
        'outer: loop {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    break 'outer;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let evs: Vec<Ev> = EVENTS.with(|v| std::mem::take(&mut *v.borrow_mut()));
            // ファイルから読み込んだ記録・保存の結果
            if let Some(data) = platform::take_loaded() {
                game.apply_loaded(&data);
            }
            for n in platform::take_notes() {
                game.notify(&n);
            }
            for e in evs.iter().filter(|e| !matches!(e, Ev::MouseMove(..))) {
                crashlog::crumb(format!("{:?}", e));
            }
            let now = std::time::Instant::now();
            let dt = (now - last).as_secs_f32().min(0.1);
            last = now;
            if (now - state_at).as_secs() >= 1 {
                state_at = now;
                crashlog::set_state(game.state_text());
            }
            let focused = GetForegroundWindow() == hwnd;
            let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crashlog::tick();
                game.frame(&evs, dt, focused)
            }));
            match done {
                Ok(true) => break,
                Ok(false) => {}
                Err(p) => {
                    game.crash_save();
                    std::panic::resume_unwind(p);
                }
            }
            if IsIconic(hwnd).as_bool() {
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
        }
    }
    Ok(())
}
