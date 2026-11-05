//! Windows 版の実行ファイル（yyww1-win.exe）。ゲームの中身は lib（src/lib.rs）にある。
//! Windows 以外では何もしない（ブラウザ版は lib を WebAssembly にして使う）。
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    yyww1::windows_main();
    #[cfg(not(windows))]
    eprintln!("yyww1-win は Windows 専用。ブラウザ版は `bash build.sh` でビルドする");
}
