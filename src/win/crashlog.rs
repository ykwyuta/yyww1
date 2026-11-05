//! 異常終了を調べるためのログ（Windows）
//!
//! GUI アプリなので標準エラーは捨てられ、パニックの文面もどこにも残らない。そこで起動ごとに
//! %APPDATA%\yyww1\logs\run-日時-pid.log を作り、1行ごとにディスクへ書く（落ちても残る）。
//!
//! | 落ち方 | 残るもの |
//! |---|---|
//! | Rust のパニック | `[PANIC]` 文面・発生箇所・バックトレース・直前の状態と操作、対局中なら同名の .yysav |
//! | アクセス違反・スタック溢れなど | `[CRASH]` 例外コード・モジュールと位置・直前の状態と操作、同名の .dmp |
//! | abort・メモリ不足・外からの強制終了・電源断 | `[END]` が無いまま切れる。次の起動時に `[PREV]` で知らせる |
//! | 応答なし | `[STALL]` 止まった時間（1分ごとの `[STAT]` でメモリの推移も追える） |

use crate::platform;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, TryLockError};
use std::time::{Duration, Instant};
use windows::core::{w, HSTRING};
use windows::Wdk::System::SystemServices::RtlGetVersion;
use windows::Win32::Foundation::{HANDLE, HMODULE};
use windows::Win32::System::Diagnostics::Debug::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::SystemInformation::*;
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::*;

/// 残すファイルの数（古いものから消す）
const KEEP_LOGS: usize = 30;
const KEEP_DUMPS: usize = 5;
const KEEP_SAVES: usize = 5;
/// 直前の操作を覚えておく数
const CRUMBS: usize = 32;
/// これより長くフレームが進まなければ応答なしとして書く
const STALL_SECS: u64 = 15;
const STAT_SECS: u64 = 60;
/// 同じ種類の警告を書く上限（毎フレーム失敗してもログを埋めない）
const WARN_LIMIT: u32 = 5;
const END_OK: &str = "[END] 正常終了";

static LOG: Mutex<Option<File>> = Mutex::new(None);
static STEM: OnceLock<PathBuf> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();
static STATE: Mutex<String> = Mutex::new(String::new());
static TRAIL: Mutex<VecDeque<(f32, String)>> = Mutex::new(VecDeque::new());
static WARNED: Mutex<Vec<(&'static str, u32)>> = Mutex::new(Vec::new());
static FRAMES: AtomicU64 = AtomicU64::new(0);
static CRASH_TEST: OnceLock<Option<String>> = OnceLock::new();

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// 落ちる途中で使う：別のところが握っていても待たない
fn try_lock<T>(m: &Mutex<T>) -> Option<MutexGuard<'_, T>> {
    match m.try_lock() {
        Ok(g) => Some(g),
        Err(TryLockError::Poisoned(e)) => Some(e.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    }
}

fn write_raw(bytes: &[u8]) {
    if let Some(g) = try_lock(&LOG) {
        if let Some(mut f) = g.as_ref() {
            let _ = f.write_all(bytes);
        }
    }
}

/// ヒープを使わない文字バッファ（壊れたヒープやスタック溢れの中でも書けるように）
struct Fixed {
    buf: [u8; 1024],
    len: usize,
}

impl Fixed {
    fn new() -> Fixed {
        Fixed { buf: [0; 1024], len: 0 }
    }
    fn flush(&mut self) {
        write_raw(&self.buf[..self.len]);
        self.len = 0;
    }
}

impl std::fmt::Write for Fixed {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for c in s.chars() {
            let mut b = [0; 4];
            let e = c.encode_utf8(&mut b).as_bytes();
            if self.len + e.len() > self.buf.len() {
                self.flush();
            }
            self.buf[self.len..self.len + e.len()].copy_from_slice(e);
            self.len += e.len();
        }
        Ok(())
    }
}

fn stamp(out: &mut impl std::fmt::Write) {
    let t = unsafe { GetLocalTime() };
    let _ = write!(out, "{}-{:02}-{:02} {:02}:{:02}:{:02}.{:03} ", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds);
}

/// 1行書く（init 前やテストでは何もしない）
pub fn line(s: &str) {
    let mut out = Fixed::new();
    stamp(&mut out);
    let _ = writeln!(out, "{}", s);
    out.flush();
}

/// 同じ種類の警告は WARN_LIMIT 回まで書く
pub fn warn_limited(kind: &'static str, s: &str) {
    let n = {
        let mut w = lock(&WARNED);
        let i = w.iter().position(|e| e.0 == kind).unwrap_or_else(|| {
            w.push((kind, 0));
            w.len() - 1
        });
        w[i].1 += 1;
        w[i].1
    };
    if n <= WARN_LIMIT {
        line(&format!("[WARN] {}: {}", kind, s));
    }
}

/// いまのゲームの状態（1秒ごとに差し替える。落ちたときにログへ書く）
pub fn set_state(s: String) {
    *lock(&STATE) = s;
}

pub fn state() -> String {
    lock(&STATE).clone()
}

/// 直前の操作として覚える（落ちたときにログへ書く）
pub fn crumb(s: String) {
    let at = START.get().map_or(0.0, |t| t.elapsed().as_secs_f32());
    let mut v = lock(&TRAIL);
    if v.len() >= CRUMBS {
        v.pop_front();
    }
    v.push_back((at, s));
}

/// 落ちる直前の状態と操作を書く
fn write_context(out: &mut impl std::fmt::Write) {
    if let Some(s) = try_lock(&STATE) {
        let _ = writeln!(out, "  状態: {}", s);
    }
    if let Some(v) = try_lock(&TRAIL) {
        let _ = writeln!(out, "  直前の操作（起動からの秒）:");
        for (at, s) in v.iter() {
            let _ = writeln!(out, "    {:8.2} {}", at, s);
        }
    }
}

fn exception_name(code: u32) -> &'static str {
    match code {
        0xC0000005 => "アクセス違反",
        0xC00000FD => "スタック溢れ",
        0xC0000374 => "ヒープ破壊",
        0xC0000409 => "スタック破壊／即時終了",
        0xC000001D => "不正な命令",
        0xC0000094 => "整数のゼロ除算",
        0xC00000FE | 0xC0000194 => "待機の打ち切り",
        0xC0000017 => "メモリ不足",
        _ => "その他",
    }
}

fn write_module(out: &mut impl std::fmt::Write, addr: usize) {
    unsafe {
        let mut m = HMODULE::default();
        let flags = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
        if GetModuleHandleExW(flags, windows::core::PCWSTR(addr as *const u16), &mut m).is_err() {
            let _ = write!(out, "（モジュール不明）");
            return;
        }
        let mut buf = [0u16; 260];
        let n = GetModuleFileNameW(Some(m), &mut buf) as usize;
        let name = buf[..n].rsplit(|&c| c == '\\' as u16).next().unwrap_or(&[]);
        for c in char::decode_utf16(name.iter().copied()) {
            let _ = out.write_char(c.unwrap_or('?'));
        }
        let _ = write!(out, "+0x{:x}", addr.wrapping_sub(m.0 as usize));
    }
}

/// 別スレッドで書く（スタック溢れでは自分のスタックに余裕がない）
fn write_dump(info: *const EXCEPTION_POINTERS, thread: u32) {
    let Some(stem) = STEM.get() else { return };
    let path = stem.with_extension("dmp");
    let Ok(f) = File::create(&path) else { return };
    let param = MINIDUMP_EXCEPTION_INFORMATION { ThreadId: thread, ExceptionPointers: info as *mut _, ClientPointers: false.into() };
    let kind = MINIDUMP_TYPE(MiniDumpWithThreadInfo.0 | MiniDumpWithIndirectlyReferencedMemory.0 | MiniDumpWithUnloadedModules.0);
    let r = unsafe { MiniDumpWriteDump(GetCurrentProcess(), GetCurrentProcessId(), HANDLE(f.as_raw_handle()), kind, Some(&param), None, None) };
    drop(f);
    match r {
        Ok(()) => line(&format!("[CRASH] ダンプ: {}", path.display())),
        Err(e) => {
            let _ = std::fs::remove_file(&path);
            line(&format!("[CRASH] ダンプを書けなかった: {}", e));
        }
    }
}

/// どこにも捕まらなかった例外（アクセス違反・スタック溢れなど）
unsafe extern "system" fn on_exception(info: *const EXCEPTION_POINTERS) -> i32 {
    unsafe {
        let rec = &*(*info).ExceptionRecord;
        let code = rec.ExceptionCode.0 as u32;
        let addr = rec.ExceptionAddress as usize;
        let mut out = Fixed::new();
        stamp(&mut out);
        let _ = write!(out, "[CRASH] 例外 0x{:08X}（{}） 場所 ", code, exception_name(code));
        write_module(&mut out, addr);
        if code == 0xC0000005 && rec.NumberParameters >= 2 {
            let what = match rec.ExceptionInformation[0] {
                0 => "読み出し",
                1 => "書き込み",
                _ => "実行",
            };
            let _ = write!(out, "（0x{:x} の{}）", rec.ExceptionInformation[1], what);
        }
        let _ = writeln!(out);
        write_context(&mut out);
        out.flush();
        let (ptr, thread) = (info as usize, GetCurrentThreadId());
        if let Ok(t) = std::thread::Builder::new().spawn(move || write_dump(ptr as *const EXCEPTION_POINTERS, thread)) {
            let _ = t.join();
        }
    }
    // Windows 側の記録（イベントログ）にも残るよう、処理は続けさせる
    EXCEPTION_CONTINUE_SEARCH
}

const EXCEPTION_CONTINUE_SEARCH: i32 = 0;

fn on_panic(info: &std::panic::PanicHookInfo) {
    let thread = std::thread::current();
    let msg = info.payload_as_str().unwrap_or("（文字列でないパニック）");
    let at = info.location().map_or("（場所不明）".to_string(), |l| l.to_string());
    let mut out = Fixed::new();
    stamp(&mut out);
    let _ = writeln!(out, "[PANIC] スレッド {}: {}", thread.name().unwrap_or("?"), msg);
    let _ = writeln!(out, "  場所: {}", at);
    write_context(&mut out);
    out.flush();
    // 先に上を書いておく：バックトレースの解決で失敗しても文面は残る
    let bt = std::backtrace::Backtrace::force_capture();
    let _ = writeln!(out, "  バックトレース:\n{}", bt);
    out.flush();
}

/// 前回のログの中身から、異常終了の知らせを作る（正常終了なら None）
fn previous_run_note(text: &str) -> Option<String> {
    if text.contains(END_OK) {
        return None;
    }
    if text.contains("[PANIC]") || text.contains("[CRASH]") {
        return Some("前回はクラッシュした（詳細はそのログ）".to_string());
    }
    let last = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("（空）");
    Some(format!("前回は終了の記録がない（abort・メモリ不足・強制終了・電源断など）。最後の行: {}", last))
}

fn files_with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map_or(Vec::new(), |d| d.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == ext)).collect());
    // 名前が日時で始まるので、名前順が時刻順
    v.sort();
    v
}

/// 新しいものを keep 個残して消す
fn prune(dir: &Path, ext: &str, keep: usize) {
    let v = files_with_ext(dir, ext);
    let n = v.len().saturating_sub(keep);
    for p in &v[..n] {
        let _ = std::fs::remove_file(p);
    }
}

fn mem_stat() -> String {
    unsafe {
        let mut pm = PROCESS_MEMORY_COUNTERS { cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32, ..Default::default() };
        let _ = K32GetProcessMemoryInfo(GetCurrentProcess(), &mut pm, pm.cb);
        let mut sys = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
        let _ = GlobalMemoryStatusEx(&mut sys);
        const MB: usize = 1024 * 1024;
        format!("使用 {}MB（最大 {}MB） 実メモリ {}MB 空き {}/{}MB", pm.PagefileUsage / MB, pm.PeakPagefileUsage / MB, pm.WorkingSetSize / MB, sys.ullAvailPhys as usize / MB, sys.ullTotalPhys as usize / MB)
    }
}

/// フレームが止まっていないかを見張り、メモリの推移を書く
fn watchdog() {
    let (mut last, mut last_change) = (0, Instant::now());
    let (mut stat_at, mut stat_frames, mut stalled) = (Instant::now(), 0, false);
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let f = FRAMES.load(Ordering::Relaxed);
        if f != last {
            if stalled {
                line(&format!("[STALL] 再開（{:.0}秒止まっていた）", last_change.elapsed().as_secs_f32()));
                stalled = false;
            }
            last = f;
            last_change = Instant::now();
        } else if !stalled && last_change.elapsed().as_secs() >= STALL_SECS {
            // ファイルダイアログを開いている間やスリープ復帰でも出る
            stalled = true;
            line(&format!("[STALL] {}秒フレームが進まない {} {}", STALL_SECS, mem_stat(), state()));
        }
        if stat_at.elapsed().as_secs() >= STAT_SECS {
            let fps = (f - stat_frames) as f32 / stat_at.elapsed().as_secs_f32();
            line(&format!("[STAT] {:.0}fps {} {}", fps, mem_stat(), state()));
            stat_at = Instant::now();
            stat_frames = f;
        }
    }
}

fn os_version() -> String {
    let mut v = OSVERSIONINFOW { dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32, ..Default::default() };
    let _ = unsafe { RtlGetVersion(&mut v) };
    format!("{}.{}.{}", v.dwMajorVersion, v.dwMinorVersion, v.dwBuildNumber)
}

/// 起動の最初に呼ぶ
pub fn init() {
    let _ = START.set(Instant::now());
    let _ = CRASH_TEST.set(std::env::var("YY_CRASH_TEST").ok());
    let dir = platform::app_dir().join("logs");
    let _ = std::fs::create_dir_all(&dir);
    let prev = files_with_ext(&dir, "log").pop();
    prune(&dir, "log", KEEP_LOGS - 1);
    prune(&dir, "dmp", KEEP_DUMPS - 1);
    prune(&dir, "yysav", KEEP_SAVES - 1);
    let t = unsafe { GetLocalTime() };
    let stem = dir.join(format!("run-{}{:02}{:02}-{:02}{:02}{:02}-{}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, std::process::id()));
    *lock(&LOG) = File::create(stem.with_extension("log")).ok();
    let _ = STEM.set(stem);
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    line(&format!("[INFO] yyww1 {} ({} {})", env!("CARGO_PKG_VERSION"), env!("YY_GIT_HASH"), profile));
    let cpus = std::thread::available_parallelism().map_or(0, |n| n.get());
    line(&format!("[INFO] Windows {} CPU {} {}", os_version(), cpus, mem_stat()));
    if let Some(p) = prev {
        let name = p.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
        let text = String::from_utf8_lossy(&std::fs::read(&p).unwrap_or_default()).into_owned();
        if let Some(note) = previous_run_note(&text) {
            line(&format!("[PREV] {}: {}", name, note));
        }
    }
    std::panic::set_hook(Box::new(on_panic));
    unsafe {
        SetUnhandledExceptionFilter(Some(on_exception));
    }
    let _ = std::thread::Builder::new().name("watchdog".into()).spawn(watchdog);
}

/// フレームごとに呼ぶ（見張りと、YY_CRASH_TEST の発動）
pub fn tick() {
    let f = FRAMES.fetch_add(1, Ordering::Relaxed);
    if f == 120 {
        if let Some(Some(kind)) = CRASH_TEST.get() {
            crash_test(kind);
        }
    }
}

/// 落ち方ごとにログが残るか確かめるための、わざとの異常（YY_CRASH_TEST=panic など）
fn crash_test(kind: &str) {
    extern "system" fn no_unwind() {
        panic!("YY_CRASH_TEST: 巻き戻せない関数の中のパニック");
    }
    #[allow(unconditional_recursion)]
    fn recurse(n: u64) -> u64 {
        let a = [n; 512];
        recurse(std::hint::black_box(a[(n % 512) as usize] + 1)) + a[0]
    }
    match kind {
        "panic" => panic!("YY_CRASH_TEST: パニック"),
        "wndproc" => no_unwind(),
        "av" => unsafe { std::ptr::write_volatile(std::ptr::null_mut::<u32>().wrapping_add(4), 1) },
        "stack" => {
            std::hint::black_box(recurse(0));
        }
        "abort" => std::process::abort(),
        "hang" => std::thread::sleep(Duration::from_secs(STALL_SECS + 3)),
        _ => line(&format!("[WARN] YY_CRASH_TEST の値が不明: {}", kind)),
    }
}

/// パニックした対局を記録に残す（読み込めば再現を試せる）
pub fn write_crash_save(data: &[u8]) {
    let Some(stem) = STEM.get() else { return };
    let path = stem.with_extension("yysav");
    match std::fs::write(&path, data) {
        Ok(()) => line(&format!("[PANIC] 対局の記録: {}", path.display())),
        Err(e) => line(&format!("[PANIC] 対局の記録を書けなかった: {}", e)),
    }
}

pub fn clean_exit() {
    line(END_OK);
}

/// 異常終了を知らせて、ログの場所を教える
pub fn report_failure(what: &str) {
    line(&format!("[END] 異常終了: {}", what));
    if std::env::var_os("YY_AUTO").is_some() {
        return;
    }
    let path = STEM.get().map_or(String::new(), |s| s.with_extension("log").display().to_string());
    let text = tf!("問題が起きたため終了します。\n原因の調査には次のログが役立ちます。\n\n{}", "A problem occurred and the game must close.\nThis log can help find the cause:\n\n{}", "发生问题，游戏即将关闭。\n以下日志有助于查明原因：\n\n{}", path);
    unsafe {
        MessageBoxW(None, &HSTRING::from(text), w!("yyww1"), MB_OK | MB_ICONERROR | MB_TOPMOST | MB_SETFOREGROUND);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous_run_is_judged_by_its_last_words() {
        assert_eq!(previous_run_note("a\n2026 [END] 正常終了\n"), None);
        assert!(previous_run_note("2026 [PANIC] x\n").unwrap().contains("クラッシュ"));
        assert!(previous_run_note("a\nb\n\n").unwrap().ends_with("最後の行: b"));
    }

    #[test]
    fn fixed_buffer_spills_instead_of_cutting_characters() {
        let mut f = Fixed::new();
        // ログが無いので書き出しは捨てられるが、文字の途中で切らずに続けられる
        write!(f, "{}", "あ".repeat(1000)).unwrap();
        assert!(std::str::from_utf8(&f.buf[..f.len]).is_ok());
    }
}
