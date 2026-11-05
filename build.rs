use std::env;
use std::fs;
use std::path::PathBuf;

fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn pad4(out: &mut Vec<u8>) {
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

fn write_res_header(out: &mut Vec<u8>, data_size: u32, res_type: u16, res_id: u16) {
    push_u32(out, data_size);
    push_u32(out, 32); // HeaderSize
    push_u16(out, 0xFFFF);
    push_u16(out, res_type);
    push_u16(out, 0xFFFF);
    push_u16(out, res_id);
    push_u32(out, 0); // DataVersion
    push_u16(out, 0x1030); // MemoryFlags
    push_u16(out, 0x0411); // LanguageId (ja-JP)
    push_u32(out, 0); // Version
    push_u32(out, 0); // Characteristics
}

fn main() {
    println!("cargo:rerun-if-changed=assets/app_icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    // ログに書く版（クラッシュの行番号をソースと突き合わせるため）
    println!("cargo:rerun-if-changed=.git/logs/HEAD");
    let git = std::process::Command::new("git").args(["rev-parse", "--short", "HEAD"]).output();
    let hash = git.ok().filter(|o| o.status.success()).map_or("unknown".to_string(), |o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    println!("cargo:rustc-env=YY_GIT_HASH={}", hash);
    // アイコンを埋め込むのは Windows（MSVC）向けの実行ファイルだけ。ブラウザ版（wasm）では何もしない
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let tenv = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os != "windows" || tenv != "msvc" {
        return;
    }
    let ico = fs::read("assets/app_icon.ico").expect("failed to read assets/app_icon.ico");
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;

    let mut res = Vec::new();
    // 1. 32-byte null resource header required at start of .res file
    push_u32(&mut res, 0);
    push_u32(&mut res, 32);
    push_u16(&mut res, 0xFFFF);
    push_u16(&mut res, 0);
    push_u16(&mut res, 0xFFFF);
    push_u16(&mut res, 0);
    push_u32(&mut res, 0);
    push_u16(&mut res, 0);
    push_u16(&mut res, 0);
    push_u32(&mut res, 0);
    push_u32(&mut res, 0);

    // 2. RT_ICON (type = 3) entries
    let mut grp = Vec::with_capacity(6 + count * 14);
    grp.extend_from_slice(&ico[0..6]); // idReserved, idType, idCount

    for i in 0..count {
        let ent = 6 + i * 16;
        let bytes_in_res = u32::from_le_bytes([ico[ent + 8], ico[ent + 9], ico[ent + 10], ico[ent + 11]]);
        let img_off = u32::from_le_bytes([ico[ent + 12], ico[ent + 13], ico[ent + 14], ico[ent + 15]]) as usize;
        let icon_id = (i + 1) as u16;

        write_res_header(&mut res, bytes_in_res, 3, icon_id);
        res.extend_from_slice(&ico[img_off..img_off + bytes_in_res as usize]);
        pad4(&mut res);

        // GRPICONDIRENTRY (12 bytes from ICONDIRENTRY + 2-byte nID)
        grp.extend_from_slice(&ico[ent..ent + 12]);
        push_u16(&mut grp, icon_id);
    }

    // 3. RT_GROUP_ICON (type = 14, id = 1)
    write_res_header(&mut res, grp.len() as u32, 14, 1);
    res.extend_from_slice(&grp);
    pad4(&mut res);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let res_path = out_dir.join("app_icon.res");
    fs::write(&res_path, &res).expect("failed to write app_icon.res");

    println!("cargo:rustc-link-arg-bins={}", res_path.display());
}
