//! 第一次大戦の司令部を基調としたUI・各国の軍装・建物の描画

use crate::gfx::Gfx;
use crate::rng::{fbm, hash2};
use crate::world::{BKind, Era, UnitKind, N_KINDS};
use crate::gfx::Bitmap as ID2D1Bitmap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResIcon {
    Food = 0,
    Wood = 1,
    Bronze = 2,
    Worker = 3,
    Soldier = 4,
    Upkeep = 5,
    Era = 6,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CmdIcon {
    Worker = 0,
    Infantry = 1,
    Artillery = 2,
    Cavalry = 3,
    Tank = 4,
    Storm = 5,
    Alpine = 6,
    Submarine = 7,
    Destroyer = 8,
    Transport = 9,
    Farm = 10,
    Barracks = 11,
    Town = 12,
    Capital = 13,
    Upgrade = 14,
    Convert = 15,
    Stop = 16,
    AttackMove = 17,
    Demolish = 18,
    Diplo = 19,
    Help = 20,
    GatherAuto = 21,
    GatherWood = 22,
    GatherBronze = 23,
    Defend = 24,
    AllArmy = 25,
    Fort = 26,
    Dock = 27,
}

const N_CMD: usize = 28;

impl CmdIcon {
    /// 兵種のアイコン（並びは UnitKind と同じ）
    pub fn of_unit(k: UnitKind) -> CmdIcon {
        CMD_ALL[k as usize]
    }
}

const CMD_ALL: [CmdIcon; N_CMD] = [
    CmdIcon::Worker,
    CmdIcon::Infantry,
    CmdIcon::Artillery,
    CmdIcon::Cavalry,
    CmdIcon::Tank,
    CmdIcon::Storm,
    CmdIcon::Alpine,
    CmdIcon::Submarine,
    CmdIcon::Destroyer,
    CmdIcon::Transport,
    CmdIcon::Farm,
    CmdIcon::Barracks,
    CmdIcon::Town,
    CmdIcon::Capital,
    CmdIcon::Upgrade,
    CmdIcon::Convert,
    CmdIcon::Stop,
    CmdIcon::AttackMove,
    CmdIcon::Demolish,
    CmdIcon::Diplo,
    CmdIcon::Help,
    CmdIcon::GatherAuto,
    CmdIcon::GatherWood,
    CmdIcon::GatherBronze,
    CmdIcon::Defend,
    CmdIcon::AllArmy,
    CmdIcon::Fort,
    CmdIcon::Dock,
];

const UNIT_SPRITE_SZ: usize = 160;
const RAW_UNIT_SPRITES: [&[u8]; N_KINDS] = [
    include_bytes!("../assets/units/worker.bgra"),
    include_bytes!("../assets/units/infantry.bgra"),
    include_bytes!("../assets/units/artillery.bgra"),
    include_bytes!("../assets/units/cavalry.bgra"),
    include_bytes!("../assets/units/tank.bgra"),
    include_bytes!("../assets/units/storm.bgra"),
    include_bytes!("../assets/units/alpine.bgra"),
    include_bytes!("../assets/units/submarine.bgra"),
    include_bytes!("../assets/units/destroyer.bgra"),
    include_bytes!("../assets/units/transport.bgra"),
];

const RAW_NATIONAL_INFANTRY: [&[u8]; 20] = [
    include_bytes!("../assets/nations/ger_infantry.bgra"),
    include_bytes!("../assets/nations/auh_infantry.bgra"),
    include_bytes!("../assets/nations/ott_infantry.bgra"),
    include_bytes!("../assets/nations/bul_infantry.bgra"),
    include_bytes!("../assets/nations/gbr_infantry.bgra"),
    include_bytes!("../assets/nations/fra_infantry.bgra"),
    include_bytes!("../assets/nations/rus_infantry.bgra"),
    include_bytes!("../assets/nations/srb_infantry.bgra"),
    include_bytes!("../assets/nations/bel_infantry.bgra"),
    include_bytes!("../assets/nations/ita_infantry.bgra"),
    include_bytes!("../assets/nations/rou_infantry.bgra"),
    include_bytes!("../assets/nations/gre_infantry.bgra"),
    include_bytes!("../assets/nations/por_infantry.bgra"),
    include_bytes!("../assets/nations/usa_infantry.bgra"),
    include_bytes!("../assets/nations/ned_infantry.bgra"),
    include_bytes!("../assets/nations/sui_infantry.bgra"),
    include_bytes!("../assets/nations/esp_infantry.bgra"),
    include_bytes!("../assets/nations/swe_infantry.bgra"),
    include_bytes!("../assets/nations/nor_infantry.bgra"),
    include_bytes!("../assets/nations/den_infantry.bgra"),
];
const RAW_LATE_INFANTRY: [&[u8]; 4] = [
    include_bytes!("../assets/nations/ger_infantry_late.bgra"),
    include_bytes!("../assets/nations/gbr_infantry_late.bgra"),
    include_bytes!("../assets/nations/fra_infantry_late.bgra"),
    include_bytes!("../assets/nations/bel_infantry_late.bgra"),
];
const RAW_FRENCH_TANK: &[u8] = include_bytes!("../assets/units/tank_french.bgra");

const BLD_SPRITE_SZ: usize = 192;
const N_BLD: usize = 6;
const RAW_BLD_SPRITES: [&[u8]; N_BLD] = [
    include_bytes!("../assets/map/capital.bgra"),
    include_bytes!("../assets/map/town.bgra"),
    include_bytes!("../assets/map/barracks.bgra"),
    include_bytes!("../assets/map/farm.bgra"),
    include_bytes!("../assets/map/fort.bgra"),
    include_bytes!("../assets/map/dock.bgra"),
];

const PROP_SPRITE_SZ: usize = 160;
const RAW_PROP_SPRITES: [&[u8]; 3] = [
    include_bytes!("../assets/map/mountain.bgra"),
    include_bytes!("../assets/map/forest.bgra"),
    include_bytes!("../assets/map/mine.bgra"),
];

const RAW_DIVINE_UNIT_SPRITES: [&[u8]; 2] = [
    include_bytes!("../assets/divine/divine_infantry.bgra"),
    include_bytes!("../assets/divine/divine_cavalry.bgra"),
];

const RAW_DIVINE_BLD_SPRITES: [&[u8]; 2] = [
    include_bytes!("../assets/divine/divine_capital.bgra"),
    include_bytes!("../assets/divine/divine_town.bgra"),
];

const PLANTING_SPRITE_SIZE: usize = 96;
const PLANTING_SPRITE_BYTES: usize = PLANTING_SPRITE_SIZE * PLANTING_SPRITE_SIZE * 4;
const RAW_PLANTING_SPRITES: &[u8; PLANTING_SPRITE_BYTES * 3] = include_bytes!("../assets/map/planting.bgra");

pub struct Art {
    planting_sprites: [Option<ID2D1Bitmap>; 3],
    gen: u32,
    pub hud_stone: Option<ID2D1Bitmap>,
    pub panel_wood: Option<ID2D1Bitmap>,
    pub title_crest: Option<ID2D1Bitmap>,
    /// 英語の題字用（扁額を広げたもの）
    pub title_crest_wide: Option<ID2D1Bitmap>,
    pub btn_norm: Option<ID2D1Bitmap>,
    pub btn_hover: Option<ID2D1Bitmap>,
    pub btn_dis: Option<ID2D1Bitmap>,
    pub res_icons: [Option<ID2D1Bitmap>; 7],
    pub cmd_icons: [Option<ID2D1Bitmap>; N_CMD],
    national_infantry: Vec<Option<ID2D1Bitmap>>,
    national_infantry_flip: Vec<Option<ID2D1Bitmap>>,
    french_tank: [Option<ID2D1Bitmap>; 2],
    late_infantry: Vec<[Option<ID2D1Bitmap>; 2]>,
    pub unit_sprites: [Option<ID2D1Bitmap>; N_KINDS],
    pub unit_sprites_flip: [Option<ID2D1Bitmap>; N_KINDS],
    pub bld_sprites: [Option<ID2D1Bitmap>; N_BLD],
    pub bld_sprites_fog: [Option<ID2D1Bitmap>; N_BLD],
    pub prop_sprites: [Option<ID2D1Bitmap>; 3],
    pub prop_sprites_flip: [Option<ID2D1Bitmap>; 3],
    pub prop_sprites_fog: [Option<ID2D1Bitmap>; 3],
    pub prop_sprites_fog_flip: [Option<ID2D1Bitmap>; 3],
    pub divine_unit_sprites: [Option<ID2D1Bitmap>; 2],
    pub divine_unit_sprites_flip: [Option<ID2D1Bitmap>; 2],
    pub divine_bld_sprites: [Option<ID2D1Bitmap>; 2],
    pub divine_bld_sprites_fog: [Option<ID2D1Bitmap>; 2],
}

struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[f32; 4]>,
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        Self { w, h, px: vec![[0.0; 4]; w * h] }
    }

    #[inline]
    fn blend(&mut self, x: i32, y: i32, c: [f32; 4]) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let a = c[3].clamp(0.0, 1.0);
        if a <= 1e-4 {
            return;
        }
        let d = &mut self.px[y as usize * self.w + x as usize];
        let inv = 1.0 - a;
        let out_a = a + d[3] * inv;
        if out_a > 1e-5 {
            d[0] = (c[0] * a + d[0] * d[3] * inv) / out_a;
            d[1] = (c[1] * a + d[1] * d[3] * inv) / out_a;
            d[2] = (c[2] * a + d[2] * d[3] * inv) / out_a;
            d[3] = out_a;
        }
    }

    fn circle(&mut self, cx: f32, cy: f32, r: f32, c: [f32; 4]) {
        let x0 = (cx - r - 1.5).floor() as i32;
        let x1 = (cx + r + 1.5).ceil() as i32;
        let y0 = (cy - r - 1.5).floor() as i32;
        let y1 = (cy + r + 1.5).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                let cov = (r + 0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, [c[0], c[1], c[2], c[3] * cov]);
                }
            }
        }
    }

    fn ring(&mut self, cx: f32, cy: f32, r: f32, w: f32, c: [f32; 4]) {
        let x0 = (cx - r - w - 1.5).floor() as i32;
        let x1 = (cx + r + w + 1.5).ceil() as i32;
        let y0 = (cy - r - w - 1.5).floor() as i32;
        let y1 = (cy + r + w + 1.5).ceil() as i32;
        let hw = w * 0.5;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                let dist = (d - r).abs();
                let cov = (hw + 0.5 - dist).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, [c[0], c[1], c[2], c[3] * cov]);
                }
            }
        }
    }

    fn capsule(&mut self, ax: f32, ay: f32, bx: f32, by: f32, r: f32, c: [f32; 4]) {
        let x0 = (ax.min(bx) - r - 1.5).floor() as i32;
        let x1 = (ax.max(bx) + r + 1.5).ceil() as i32;
        let y0 = (ay.min(by) - r - 1.5).floor() as i32;
        let y1 = (ay.max(by) + r + 1.5).ceil() as i32;
        let dx = bx - ax;
        let dy = by - ay;
        let l2 = dx * dx + dy * dy;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let t = if l2 > 1e-6 { ((px - ax) * dx + (py - ay) * dy) / l2 } else { 0.0 }.clamp(0.0, 1.0);
                let qx = ax + dx * t;
                let qy = ay + dy * t;
                let d = ((px - qx).powi(2) + (py - qy).powi(2)).sqrt();
                let cov = (r + 0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, [c[0], c[1], c[2], c[3] * cov]);
                }
            }
        }
    }

    fn round_rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, c: [f32; 4]) {
        let ix0 = (x0 - 1.0).floor() as i32;
        let ix1 = (x1 + 1.0).ceil() as i32;
        let iy0 = (y0 - 1.0).floor() as i32;
        let iy1 = (y1 + 1.0).ceil() as i32;
        let cx = (x0 + x1) * 0.5;
        let cy = (y0 + y1) * 0.5;
        let hx = (x1 - x0) * 0.5 - r;
        let hy = (y1 - y0) * 0.5 - r;
        for y in iy0..=iy1 {
            for x in ix0..=ix1 {
                let px = (x as f32 + 0.5 - cx).abs() - hx;
                let py = (y as f32 + 0.5 - cy).abs() - hy;
                let d = (px.max(0.0).powi(2) + py.max(0.0).powi(2)).sqrt() + px.max(py).min(0.0) - r;
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, [c[0], c[1], c[2], c[3] * cov]);
                }
            }
        }
    }

    fn poly(&mut self, pts: &[(f32, f32)], c: [f32; 4]) {
        if pts.len() < 3 {
            return;
        }
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &(x, y) in pts {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let ix0 = (min_x - 1.0).floor() as i32;
        let ix1 = (max_x + 1.0).ceil() as i32;
        let iy0 = (min_y - 1.0).floor() as i32;
        let iy1 = (max_y + 1.0).ceil() as i32;
        // 2x2 オーバーサンプリング
        for y in iy0..=iy1 {
            for x in ix0..=ix1 {
                let mut hit = 0;
                for sy in 0..2 {
                    for sx in 0..2 {
                        let px = x as f32 + (sx as f32 + 0.5) * 0.5;
                        let py = y as f32 + (sy as f32 + 0.5) * 0.5;
                        let mut inside = false;
                        let mut j = pts.len() - 1;
                        for i in 0..pts.len() {
                            let (xi, yi) = pts[i];
                            let (xj, yj) = pts[j];
                            if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
                                inside = !inside;
                            }
                            j = i;
                        }
                        if inside {
                            hit += 1;
                        }
                    }
                }
                if hit > 0 {
                    self.blend(x, y, [c[0], c[1], c[2], c[3] * (hit as f32 * 0.25)]);
                }
            }
        }
    }

    fn to_bgra_premul(&self) -> Vec<u32> {
        self.px
            .iter()
            .map(|p| {
                let a = p[3].clamp(0.0, 1.0);
                let r = (p[0].clamp(0.0, 1.0) * a * 255.0).round() as u32;
                let g = (p[1].clamp(0.0, 1.0) * a * 255.0).round() as u32;
                let b = (p[2].clamp(0.0, 1.0) * a * 255.0).round() as u32;
                let ai = (a * 255.0).round() as u32;
                b | (g << 8) | (r << 16) | (ai << 24)
            })
            .collect()
    }

    fn blit_raw_sprite(
        &mut self,
        raw: &[u8],
        sz: usize,
        dst: (f32, f32, f32, f32),
        src_uv: (f32, f32, f32, f32),
        circle_clip: Option<(f32, f32, f32)>,
    ) {
        let sw = sz as f32;
        let sh = sz as f32;
        let (dx0, dy0, dx1, dy1) = dst;
        let (u0, v0, u1, v1) = src_uv;
        let ix0 = dx0.floor().max(0.0) as i32;
        let iy0 = dy0.floor().max(0.0) as i32;
        let ix1 = dx1.ceil().min(self.w as f32 - 1.0) as i32;
        let iy1 = dy1.ceil().min(self.h as f32 - 1.0) as i32;
        let dw = (dx1 - dx0).max(1.0);
        let dh = (dy1 - dy0).max(1.0);

        let sample_px = |sx: i32, sy: i32| -> [f32; 4] {
            let cx = sx.clamp(0, sz as i32 - 1) as usize;
            let cy = sy.clamp(0, sz as i32 - 1) as usize;
            let off = (cy * sz + cx) * 4;
            let b = raw[off] as f32 / 255.0;
            let g = raw[off + 1] as f32 / 255.0;
            let r = raw[off + 2] as f32 / 255.0;
            let a = raw[off + 3] as f32 / 255.0;
            if a > 1e-4 {
                [(r / a).clamp(0.0, 1.0), (g / a).clamp(0.0, 1.0), (b / a).clamp(0.0, 1.0), a]
            } else {
                [0.0; 4]
            }
        };

        for y in iy0..=iy1 {
            for x in ix0..=ix1 {
                let mut clip_a = 1.0;
                if let Some((ccx, ccy, cr)) = circle_clip {
                    let d = ((x as f32 + 0.5 - ccx).powi(2) + (y as f32 + 0.5 - ccy).powi(2)).sqrt();
                    clip_a = (cr + 0.5 - d).clamp(0.0, 1.0);
                    if clip_a <= 0.0 {
                        continue;
                    }
                }
                let tx = (x as f32 + 0.5 - dx0) / dw;
                let ty = (y as f32 + 0.5 - dy0) / dh;
                let su = (u0 + (u1 - u0) * tx) * sw - 0.5;
                let sv = (v0 + (v1 - v0) * ty) * sh - 0.5;
                let sx0 = su.floor() as i32;
                let sy0 = sv.floor() as i32;
                let fx = su - sx0 as f32;
                let fy = sv - sy0 as f32;
                let c00 = sample_px(sx0, sy0);
                let c10 = sample_px(sx0 + 1, sy0);
                let c01 = sample_px(sx0, sy0 + 1);
                let c11 = sample_px(sx0 + 1, sy0 + 1);
                let mut c = [0.0f32; 4];
                let w00 = (1.0 - fx) * (1.0 - fy);
                let w10 = fx * (1.0 - fy);
                let w01 = (1.0 - fx) * fy;
                let w11 = fx * fy;
                let a = c00[3] * w00 + c10[3] * w10 + c01[3] * w01 + c11[3] * w11;
                if a > 1e-3 {
                    for k in 0..3 {
                        c[k] = (c00[k] * c00[3] * w00 + c10[k] * c10[3] * w10 + c01[k] * c01[3] * w01 + c11[k] * c11[3] * w11) / a;
                    }
                    c[3] = a * clip_a;
                    self.blend(x, y, c);
                }
            }
        }
    }

    fn blit_unit_sprite(
        &mut self,
        unit_idx: usize,
        dst: (f32, f32, f32, f32),
        src_uv: (f32, f32, f32, f32),
        circle_clip: Option<(f32, f32, f32)>,
    ) {
        self.blit_raw_sprite(RAW_UNIT_SPRITES[unit_idx.min(N_KINDS - 1)], UNIT_SPRITE_SZ, dst, src_uv, circle_clip);
    }

    fn blit_bld_sprite(
        &mut self,
        bld_idx: usize,
        dst: (f32, f32, f32, f32),
        src_uv: (f32, f32, f32, f32),
    ) {
        self.blit_raw_sprite(RAW_BLD_SPRITES[bld_idx.min(N_BLD - 1)], BLD_SPRITE_SZ, dst, src_uv, None);
    }

    fn blit_prop_sprite(
        &mut self,
        prop_idx: usize,
        dst: (f32, f32, f32, f32),
        src_uv: (f32, f32, f32, f32),
    ) {
        self.blit_raw_sprite(RAW_PROP_SPRITES[prop_idx.min(2)], PROP_SPRITE_SZ, dst, src_uv, None);
    }
}

fn load_raw_sprite_bgra(raw: &[u8], sz: usize, flip_x: bool, mul: f32) -> Vec<u32> {
    let mut out = vec![0u32; sz * sz];
    for y in 0..sz {
        for x in 0..sz {
            let sx = if flip_x { sz - 1 - x } else { x };
            let off = (y * sz + sx) * 4;
            let b = ((raw[off] as f32 * mul).round() as u32).min(255);
            let g = ((raw[off + 1] as f32 * mul).round() as u32).min(255);
            let r = ((raw[off + 2] as f32 * mul).round() as u32).min(255);
            let a = raw[off + 3] as u32;
            out[y * sz + x] = b | (g << 8) | (r << 16) | (a << 24);
        }
    }
    out
}

fn load_unit_sprite_bgra(idx: usize, flip_x: bool) -> Vec<u32> {
    load_raw_sprite_bgra(RAW_UNIT_SPRITES[idx.min(N_KINDS - 1)], UNIT_SPRITE_SZ, flip_x, 1.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

// Field headquarters: ink-blue enamel, drafting grids and restrained brass rules.
fn gen_hud_stone(w: usize, h: usize) -> Vec<u32> {
    let mut cv = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let grain = (hash2(x as i32, y as i32, 101) - 0.5) * 0.018;
            let grid = if x % 32 == 0 || y % 32 == 0 { 0.012 } else { 0.0 };
            cv.px[y * w + x] = [0.075 + grain + grid, 0.115 + grain + grid, 0.135 + grain + grid, 0.98];
        }
    }
    cv.to_bgra_premul()
}

fn gen_panel_wood(w: usize, h: usize) -> Vec<u32> {
    let mut cv = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let grain = (hash2(x as i32, y as i32, 303) - 0.5) * 0.012;
            cv.px[y * w + x] = [0.10 + grain, 0.145 + grain, 0.16 + grain, 0.98];
        }
    }
    cv.to_bgra_premul()
}

fn gen_button_tex(w: usize, h: usize, state: u8) -> Vec<u32> {
    let mut cv = Canvas::new(w, h);
    let (base, rule) = match state {
        1 => ([0.24, 0.32, 0.34, 1.0], [0.80, 0.73, 0.53, 1.0]),
        2 => ([0.13, 0.16, 0.17, 1.0], [0.27, 0.31, 0.31, 1.0]),
        _ => ([0.15, 0.22, 0.24, 1.0], [0.48, 0.56, 0.55, 1.0]),
    };
    cv.round_rect(0.0, 0.0, w as f32, h as f32, 0.0, rule);
    cv.round_rect(1.0, 1.0, w as f32 - 1.0, h as f32 - 1.0, 0.0, base);
    cv.to_bgra_premul()
}

pub const TITLE_PLAQUE_W: f32 = 292.0;
pub const TITLE_PLAQUE_W_WIDE: f32 = 292.0;
fn gen_title_crest(_wide: bool) -> Vec<u32> {
    let mut cv = Canvas::new(340, 124);
    let rule = [0.69, 0.66, 0.49, 0.9];
    cv.capsule(26.0, 12.0, 314.0, 12.0, 0.6, rule);
    cv.capsule(26.0, 112.0, 314.0, 112.0, 0.6, rule);
    for x in [26.0, 314.0] {
        cv.capsule(x, 8.0, x, 16.0, 0.6, rule);
        cv.capsule(x, 108.0, x, 116.0, 0.6, rule);
    }
    cv.to_bgra_premul()
}

/// 64x64 アイコンに共通の Age of Empires 風額縁と背景グラデーションを描く
fn icon_backdrop(cv: &mut Canvas, top: [f32; 3], bot: [f32; 3], glow: [f32; 3]) {
    let (w, h) = (cv.w, cv.h);
    for y in 0..h {
        let ty = y as f32 / (h - 1) as f32;
        for x in 0..w {
            let dx = (x as f32 - w as f32 * 0.5) / (w as f32 * 0.5);
            let dy = (y as f32 - h as f32 * 0.45) / (h as f32 * 0.5);
            let r2 = (dx * dx + dy * dy).min(1.2);
            let mut c = lerp3(top, bot, ty);
            c = lerp3(glow, c, r2.sqrt().clamp(0.0, 1.0));
            cv.px[y * w + x] = [c[0], c[1], c[2], 1.0];
        }
    }
}

fn icon_frame(cv: &mut Canvas) {
    let (w, h) = (cv.w as f32, cv.h as f32);
    let c = [0.55, 0.62, 0.60, 0.9];
    cv.capsule(1.0, 1.0, w - 1.0, 1.0, 0.5, c);
    cv.capsule(1.0, h - 1.0, w - 1.0, h - 1.0, 0.5, c);
    cv.capsule(1.0, 1.0, 1.0, h - 1.0, 0.5, c);
    cv.capsule(w - 1.0, 1.0, w - 1.0, h - 1.0, 0.5, c);
}

/// 上部バー用の資源・ステータスアイコン（48x48、背景透過の立体エンブレム）
fn gen_res_icon(kind: ResIcon) -> Vec<u32> {
    let mut cv = Canvas::new(48, 48);
    // 背後の丸いメダル台座
    cv.circle(24.0, 25.0, 21.0, [0.0, 0.0, 0.0, 0.55]);
    cv.circle(24.0, 24.0, 20.0, [0.16, 0.13, 0.09, 0.95]);
    cv.ring(24.0, 24.0, 19.5, 2.2, [0.78, 0.64, 0.34, 1.0]);

    match kind {
        ResIcon::Food => {
            // 背後の黄金の稲穂・粟穂
            let stalk = [0.85, 0.68, 0.22, 1.0];
            let grain = [1.0, 0.88, 0.35, 1.0];
            for &(ax, ay, bx, by) in &[(20.0, 36.0, 12.0, 10.0), (24.0, 36.0, 24.0, 7.0), (28.0, 36.0, 36.0, 10.0)] {
                cv.capsule(ax, ay, bx, by, 1.5, stalk);
                for k in 0..4 {
                    let t = 0.45 + k as f32 * 0.14;
                    let gx = ax + (bx - ax) * t;
                    let gy = ay + (by - ay) * t;
                    cv.circle(gx - 2.2, gy, 2.4, grain);
                    cv.circle(gx + 2.2, gy, 2.4, grain);
                }
            }
            // 手前の山盛り穀物鉢（陶器の高坏）
            cv.circle(24.0, 27.0, 10.5, [0.98, 0.86, 0.38, 1.0]);
            cv.poly(&[(12.0, 27.0), (36.0, 27.0), (31.0, 38.0), (17.0, 38.0)], [0.68, 0.32, 0.18, 1.0]);
            cv.capsule(12.0, 27.0, 36.0, 27.0, 2.0, [0.88, 0.52, 0.28, 1.0]);
            cv.capsule(18.0, 39.0, 30.0, 39.0, 2.0, [0.52, 0.24, 0.14, 1.0]);
        }
        ResIcon::Wood => {
            // 三本の丸太（樹皮と年輪）＋斧の刃
            let bark = [0.42, 0.25, 0.13, 1.0];
            let bark_hi = [0.56, 0.35, 0.18, 1.0];
            let cut = [0.92, 0.76, 0.48, 1.0];
            let ring_c = [0.68, 0.48, 0.24, 1.0];
            for &(x0, y0, x1, y1) in &[(11.0, 31.0, 30.0, 34.0), (18.0, 31.0, 37.0, 34.0), (14.0, 22.0, 34.0, 25.0)] {
                cv.capsule(x0, y0, x1, y1, 5.8, bark);
                cv.capsule(x0, y0 - 1.5, x1, y1 - 1.5, 2.8, bark_hi);
                cv.circle(x0, y0, 5.2, cut);
                cv.ring(x0, y0, 2.8, 1.0, ring_c);
            }
            // 若葉のアクセント
            cv.circle(35.0, 17.0, 4.2, [0.32, 0.72, 0.28, 1.0]);
            cv.circle(38.0, 20.0, 3.2, [0.48, 0.85, 0.35, 1.0]);
        }
        ResIcon::Bronze => {
            // 鉄のインゴット（鋼材）と石炭
            let dark = [0.30, 0.32, 0.36, 1.0];
            let mid = [0.56, 0.60, 0.66, 1.0];
            let hi = [0.86, 0.90, 0.96, 1.0];
            for &ox in &[16.0, 31.0] {
                cv.poly(&[(ox - 8.0, 35.0), (ox + 8.0, 35.0), (ox + 5.5, 27.0), (ox - 5.5, 27.0)], dark);
                cv.poly(&[(ox - 7.0, 34.0), (ox + 7.0, 34.0), (ox + 4.8, 28.0), (ox - 4.8, 28.0)], mid);
                cv.capsule(ox - 4.0, 28.5, ox + 4.0, 28.5, 1.2, hi);
            }
            let ox = 23.5;
            cv.poly(&[(ox - 8.5, 27.5), (ox + 8.5, 27.5), (ox + 6.0, 19.0), (ox - 6.0, 19.0)], dark);
            cv.poly(&[(ox - 7.5, 26.5), (ox + 7.5, 26.5), (ox + 5.0, 20.0), (ox - 5.0, 20.0)], mid);
            cv.capsule(ox - 4.5, 20.5, ox + 4.5, 20.5, 1.4, hi);
            cv.circle(14.0, 15.0, 4.0, [0.10, 0.10, 0.12, 1.0]);
            cv.circle(33.0, 14.0, 3.2, [0.12, 0.12, 0.14, 1.0]);
        }
        ResIcon::Worker => {
            cv.blit_unit_sprite(0, (4.0, 4.0, 44.0, 44.0), (0.16, 0.0, 0.84, 0.68), Some((24.0, 24.0, 18.5)));
            cv.ring(24.0, 24.0, 19.5, 2.0, [0.95, 0.80, 0.38, 1.0]);
        }
        ResIcon::Soldier => {
            cv.blit_unit_sprite(1, (4.0, 4.0, 44.0, 44.0), (0.14, 0.0, 0.86, 0.72), Some((24.0, 24.0, 18.5)));
            cv.ring(24.0, 24.0, 19.5, 2.0, [0.95, 0.80, 0.38, 1.0]);
        }
        ResIcon::Upkeep => {
            // 兵糧袋と炎
            cv.round_rect(14.0, 18.0, 34.0, 37.0, 6.0, [0.76, 0.62, 0.40, 1.0]);
            cv.capsule(18.0, 18.0, 30.0, 18.0, 2.2, [0.85, 0.22, 0.18, 1.0]);
            cv.poly(&[(17.0, 12.0), (31.0, 12.0), (28.0, 18.0), (20.0, 18.0)], [0.82, 0.68, 0.44, 1.0]);
            cv.circle(24.0, 28.0, 5.0, [0.95, 0.55, 0.20, 1.0]);
            cv.circle(24.0, 28.0, 2.8, [1.0, 0.88, 0.40, 1.0]);
        }
        ResIcon::Era => {
            // Printed staff calendar, replacing bamboo slips.
            cv.round_rect(12.0, 10.0, 36.0, 38.0, 1.0, [0.86, 0.85, 0.75, 1.0]);
            cv.round_rect(12.0, 10.0, 36.0, 17.0, 0.0, [0.46, 0.22, 0.20, 1.0]);
            for y in [22.0, 28.0, 34.0] {
                cv.capsule(17.0, y, 31.0, y, 0.7, [0.22, 0.29, 0.30, 1.0]);
            }
        }
    }
    cv.to_bgra_premul()
}

/// 64x64 コマンド＆ポートレートアイコン（Age of Empires 風の絵画調アイコン）
fn gen_cmd_icon(kind: CmdIcon) -> Vec<u32> {
    let mut cv = Canvas::new(64, 64);
    match kind {
        CmdIcon::Worker | CmdIcon::Infantry | CmdIcon::Artillery | CmdIcon::Cavalry | CmdIcon::Tank | CmdIcon::Storm | CmdIcon::Alpine | CmdIcon::Submarine | CmdIcon::Destroyer | CmdIcon::Transport => {
            let k = kind as usize;
            let (top, bot, glow) = match kind {
                CmdIcon::Worker => ([0.22, 0.38, 0.52], [0.16, 0.24, 0.14], [0.65, 0.72, 0.48]),
                CmdIcon::Infantry | CmdIcon::Storm | CmdIcon::Alpine => ([0.38, 0.30, 0.16], [0.16, 0.12, 0.06], [0.78, 0.66, 0.36]),
                CmdIcon::Artillery | CmdIcon::Tank => ([0.30, 0.30, 0.22], [0.12, 0.12, 0.08], [0.66, 0.64, 0.46]),
                CmdIcon::Cavalry => ([0.42, 0.28, 0.14], [0.20, 0.12, 0.06], [0.88, 0.66, 0.30]),
                _ => ([0.18, 0.32, 0.50], [0.08, 0.14, 0.26], [0.45, 0.65, 0.88]),
            };
            icon_backdrop(&mut cv, top, bot, glow);
            let uv = match kind {
                CmdIcon::Worker | CmdIcon::Infantry | CmdIcon::Storm | CmdIcon::Alpine => (0.12, 0.0, 0.88, 0.76),
                CmdIcon::Cavalry => (0.08, 0.06, 0.94, 0.92),
                _ => (0.02, 0.18, 0.98, 0.98),
            };
            cv.blit_unit_sprite(k, (3.0, 3.0, 61.0, 61.0), uv, None);
        }
        CmdIcon::Farm => {
            icon_backdrop(&mut cv, [0.28, 0.48, 0.68], [0.26, 0.42, 0.18], [0.75, 0.82, 0.52]);
            cv.blit_bld_sprite(3, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Barracks => {
            icon_backdrop(&mut cv, [0.38, 0.18, 0.14], [0.18, 0.12, 0.10], [0.72, 0.42, 0.26]);
            cv.blit_bld_sprite(2, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Town => {
            icon_backdrop(&mut cv, [0.22, 0.36, 0.52], [0.18, 0.24, 0.16], [0.58, 0.66, 0.52]);
            cv.blit_bld_sprite(1, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Fort => {
            icon_backdrop(&mut cv, [0.46, 0.34, 0.20], [0.20, 0.14, 0.10], [0.85, 0.62, 0.32]);
            cv.blit_bld_sprite(4, (2.0, 2.0, 62.0, 62.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Capital => {
            icon_backdrop(&mut cv, [0.48, 0.32, 0.14], [0.22, 0.12, 0.08], [0.98, 0.82, 0.38]);
            cv.blit_bld_sprite(0, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Dock => {
            icon_backdrop(&mut cv, [0.18, 0.32, 0.50], [0.08, 0.14, 0.26], [0.45, 0.65, 0.88]);
            cv.blit_bld_sprite(5, (2.0, 2.0, 62.0, 62.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Upgrade => {
            icon_backdrop(&mut cv, [0.18,0.25,0.27], [0.08,0.13,0.15], [0.40,0.46,0.43]);
            for y in [20.0, 32.0, 44.0] {
                cv.capsule(16.0,y+8.0,32.0,y-4.0,2.0,[0.85,0.81,0.63,1.0]);
                cv.capsule(32.0,y-4.0,48.0,y+8.0,2.0,[0.85,0.81,0.63,1.0]);
            }
        }
        CmdIcon::Convert => {
            icon_backdrop(&mut cv,[0.18,0.25,0.27],[0.08,0.13,0.15],[0.40,0.46,0.43]);
            cv.blit_unit_sprite(0,(2.0,8.0,30.0,58.0),(0.0,0.0,1.0,1.0),None);
            cv.blit_unit_sprite(1,(34.0,8.0,62.0,58.0),(0.0,0.0,1.0,1.0),None);
            cv.capsule(22.0,30.0,42.0,30.0,1.5,[0.87,0.84,0.68,1.0]);
            cv.poly(&[(42.0,30.0),(35.0,25.0),(35.0,35.0)],[0.87,0.84,0.68,1.0]);
        }
        CmdIcon::Stop => {
            icon_backdrop(&mut cv,[0.24,0.20,0.20],[0.10,0.12,0.13],[0.40,0.37,0.33]);
            cv.round_rect(17.0,17.0,47.0,47.0,0.0,[0.79,0.43,0.35,1.0]);
        }
        CmdIcon::AttackMove => {
            icon_backdrop(&mut cv,[0.26,0.21,0.19],[0.12,0.13,0.13],[0.49,0.37,0.29]);
            cv.capsule(14.0,49.0,47.0,16.0,3.0,[0.83,0.70,0.50,1.0]);
            cv.poly(&[(50.0,12.0),(31.0,17.0),(46.0,32.0)],[0.83,0.70,0.50,1.0]);
        }
        CmdIcon::Demolish => {
            icon_backdrop(&mut cv, [0.32, 0.16, 0.12], [0.14, 0.08, 0.06], [0.85, 0.36, 0.16]);
            // 砕ける石材とツルハシ・炎
            cv.round_rect(12.0, 36.0, 34.0, 52.0, 2.0, [0.55, 0.50, 0.44, 1.0]);
            cv.round_rect(32.0, 40.0, 52.0, 54.0, 2.0, [0.48, 0.44, 0.38, 1.0]);
            cv.capsule(14.0, 48.0, 48.0, 14.0, 2.8, [0.58, 0.36, 0.18, 1.0]);
            cv.capsule(36.0, 10.0, 54.0, 28.0, 3.2, [0.82, 0.85, 0.90, 1.0]);
            cv.circle(32.0, 34.0, 7.0, [1.0, 0.62, 0.18, 0.9]);
        }
        CmdIcon::Diplo => {
            icon_backdrop(&mut cv, [0.28, 0.22, 0.14], [0.14, 0.10, 0.06], [0.78, 0.64, 0.32]);
            // 玉璽（ぎょくじ）と盟約の竹簡
            cv.round_rect(12.0, 16.0, 52.0, 48.0, 3.0, [0.86, 0.72, 0.42, 1.0]);
            cv.circle(32.0, 32.0, 10.0, [0.82, 0.18, 0.14, 1.0]);
            cv.ring(32.0, 32.0, 7.0, 1.5, [1.0, 0.85, 0.55, 1.0]);
        }
        CmdIcon::Help => {
            icon_backdrop(&mut cv, [0.16, 0.26, 0.38], [0.08, 0.12, 0.18], [0.42, 0.64, 0.85]);
            // 兵法書（巻物）
            cv.round_rect(14.0, 12.0, 50.0, 52.0, 4.0, [0.90, 0.80, 0.55, 1.0]);
            cv.capsule(12.0, 12.0, 52.0, 12.0, 3.2, [0.68, 0.42, 0.20, 1.0]);
            cv.capsule(12.0, 52.0, 52.0, 52.0, 3.2, [0.68, 0.42, 0.20, 1.0]);
            cv.circle(32.0, 32.0, 9.0, [0.22, 0.38, 0.62, 1.0]);
        }
        CmdIcon::GatherAuto => {
            icon_backdrop(&mut cv, [0.22, 0.36, 0.24], [0.12, 0.18, 0.10], [0.58, 0.78, 0.42]);
            // 黄金の稲穂・丸太・青銅鉱石の3資源と鍬
            cv.circle(20.0, 42.0, 9.0, [0.95, 0.78, 0.25, 1.0]); // 糧
            cv.round_rect(26.0, 38.0, 46.0, 50.0, 3.5, [0.58, 0.36, 0.18, 1.0]); // 木
            cv.circle(46.0, 44.0, 6.0, [0.82, 0.64, 0.36, 1.0]);
            cv.poly(&[(34.0, 22.0), (52.0, 26.0), (46.0, 38.0), (30.0, 34.0)], [0.32, 0.85, 0.65, 1.0]); // 銅・孔雀石
            // 中央を横切る黄金の鍬
            cv.capsule(12.0, 50.0, 46.0, 14.0, 2.8, [0.68, 0.44, 0.20, 1.0]);
            cv.poly(&[(40.0, 10.0), (54.0, 20.0), (48.0, 26.0), (35.0, 16.0)], [0.95, 0.96, 1.0, 1.0]);
        }
        CmdIcon::GatherWood => {
            icon_backdrop(&mut cv, [0.16, 0.34, 0.18], [0.08, 0.16, 0.08], [0.38, 0.68, 0.32]);
            cv.blit_prop_sprite(1, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
            // 輝く伐採斧のオーバーレイ
            cv.capsule(16.0, 52.0, 48.0, 18.0, 2.6, [0.68, 0.44, 0.22, 1.0]);
            cv.poly(&[(40.0, 14.0), (56.0, 24.0), (46.0, 34.0), (36.0, 22.0)], [0.95, 0.96, 1.0, 1.0]);
        }
        CmdIcon::GatherBronze => {
            icon_backdrop(&mut cv, [0.32, 0.22, 0.14], [0.14, 0.10, 0.06], [0.85, 0.56, 0.24]);
            cv.blit_prop_sprite(2, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
            // ツルハシのオーバーレイ
            cv.capsule(14.0, 50.0, 46.0, 18.0, 2.6, [0.65, 0.42, 0.20, 1.0]);
            cv.capsule(34.0, 12.0, 54.0, 32.0, 2.8, [0.98, 0.90, 0.68, 1.0]);
        }
        CmdIcon::Defend => {
            icon_backdrop(&mut cv,[0.18,0.25,0.27],[0.08,0.13,0.15],[0.40,0.46,0.43]);
            cv.blit_bld_sprite(4,(3.0,3.0,61.0,61.0),(0.0,0.0,1.0,1.0));
        }
        CmdIcon::AllArmy => {
            icon_backdrop(&mut cv,[0.18,0.25,0.27],[0.08,0.13,0.15],[0.40,0.46,0.43]);
            for x in [3.0,20.0,37.0] {
                cv.blit_unit_sprite(1,(x,7.0,x+26.0,60.0),(0.0,0.0,1.0,1.0),None);
            }
        }
    }
    icon_frame(&mut cv);
    cv.to_bgra_premul()
}

impl Art {
    pub fn new() -> Self {
        Self {
            gen: u32::MAX,
            hud_stone: None,
            panel_wood: None,
            title_crest: None,
            title_crest_wide: None,
            btn_norm: None,
            btn_hover: None,
            btn_dis: None,
            res_icons: Default::default(),
            cmd_icons: Default::default(),
            national_infantry: Vec::new(),
            national_infantry_flip: Vec::new(),
            french_tank: Default::default(),
            late_infantry: Vec::new(),
            unit_sprites: Default::default(),
            unit_sprites_flip: Default::default(),
            bld_sprites: Default::default(),
            bld_sprites_fog: Default::default(),
            planting_sprites: Default::default(),
            prop_sprites: Default::default(),
            prop_sprites_flip: Default::default(),
            prop_sprites_fog: Default::default(),
            prop_sprites_fog_flip: Default::default(),
            divine_unit_sprites: Default::default(),
            divine_unit_sprites_flip: Default::default(),
            divine_bld_sprites: Default::default(),
            divine_bld_sprites_fog: Default::default(),
        }
    }

    pub fn ensure(&mut self, gfx: &Gfx) {
        if self.gen == gfx.d2d_gen && self.hud_stone.is_some() {
            return;
        }
        self.gen = gfx.d2d_gen;
        self.hud_stone = gfx.create_bitmap(256, 128, &gen_hud_stone(256, 128));
        self.panel_wood = gfx.create_bitmap(256, 128, &gen_panel_wood(256, 128));
        self.title_crest = gfx.create_bitmap(340, 124, &gen_title_crest(false));
        self.title_crest_wide = gfx.create_bitmap(340, 124, &gen_title_crest(true));
        self.btn_norm = gfx.create_bitmap(128, 48, &gen_button_tex(128, 48, 0));
        self.btn_hover = gfx.create_bitmap(128, 48, &gen_button_tex(128, 48, 1));
        self.btn_dis = gfx.create_bitmap(128, 48, &gen_button_tex(128, 48, 2));

        let res_all = [
            ResIcon::Food,
            ResIcon::Wood,
            ResIcon::Bronze,
            ResIcon::Worker,
            ResIcon::Soldier,
            ResIcon::Upkeep,
            ResIcon::Era,
        ];
        for r in res_all {
            self.res_icons[r as usize] = gfx.create_bitmap(48, 48, &gen_res_icon(r));
        }

        let cmd_all = CMD_ALL;
        for c in cmd_all {
            self.cmd_icons[c as usize] = gfx.create_bitmap(64, 64, &gen_cmd_icon(c));
        }
        for b in 0..N_KINDS {
            let sz = UNIT_SPRITE_SZ as u32;
            self.unit_sprites[b] = gfx.create_bitmap(sz, sz, &load_unit_sprite_bgra(b, false));
            self.unit_sprites_flip[b] = gfx.create_bitmap(sz, sz, &load_unit_sprite_bgra(b, true));
        }
        self.national_infantry.clear();
        self.national_infantry_flip.clear();
        for raw in RAW_NATIONAL_INFANTRY {
            self.national_infantry.push(gfx.create_bitmap(160, 160, &load_raw_sprite_bgra(raw, 160, false, 1.0)));
            self.national_infantry_flip.push(gfx.create_bitmap(160, 160, &load_raw_sprite_bgra(raw, 160, true, 1.0)));
        }
        self.late_infantry.clear();
        for raw in RAW_LATE_INFANTRY {
            self.late_infantry.push([
                gfx.create_bitmap(160, 160, &load_raw_sprite_bgra(raw, 160, false, 1.0)),
                gfx.create_bitmap(160, 160, &load_raw_sprite_bgra(raw, 160, true, 1.0)),
            ]);
        }
        for flip in 0..2 {
            self.french_tank[flip] = gfx.create_bitmap(160, 160, &load_raw_sprite_bgra(RAW_FRENCH_TANK, 160, flip == 1, 1.0));
        }
        let bsz = BLD_SPRITE_SZ as u32;
        for b in 0..N_BLD {
            self.bld_sprites[b] = gfx.create_bitmap(bsz, bsz, &load_raw_sprite_bgra(RAW_BLD_SPRITES[b], BLD_SPRITE_SZ, false, 1.0));
            self.bld_sprites_fog[b] = gfx.create_bitmap(bsz, bsz, &load_raw_sprite_bgra(RAW_BLD_SPRITES[b], BLD_SPRITE_SZ, false, 0.52));
        }
        let planting_size = PLANTING_SPRITE_SIZE as u32;
        for (bitmap, raw) in self.planting_sprites.iter_mut().zip(RAW_PLANTING_SPRITES.chunks_exact(PLANTING_SPRITE_BYTES)) {
            let pixels = load_raw_sprite_bgra(raw, PLANTING_SPRITE_SIZE, false, 1.0);
            *bitmap = gfx.create_bitmap(planting_size, planting_size, &pixels);
        }
        let psz = PROP_SPRITE_SZ as u32;
        for p in 0..3 {
            self.prop_sprites[p] = gfx.create_bitmap(psz, psz, &load_raw_sprite_bgra(RAW_PROP_SPRITES[p], PROP_SPRITE_SZ, false, 1.0));
            self.prop_sprites_flip[p] = gfx.create_bitmap(psz, psz, &load_raw_sprite_bgra(RAW_PROP_SPRITES[p], PROP_SPRITE_SZ, true, 1.0));
            self.prop_sprites_fog[p] = gfx.create_bitmap(psz, psz, &load_raw_sprite_bgra(RAW_PROP_SPRITES[p], PROP_SPRITE_SZ, false, 0.52));
            self.prop_sprites_fog_flip[p] = gfx.create_bitmap(psz, psz, &load_raw_sprite_bgra(RAW_PROP_SPRITES[p], PROP_SPRITE_SZ, true, 0.52));
        }
        let usz = UNIT_SPRITE_SZ as u32;
        for d in 0..2 {
            self.divine_unit_sprites[d] = gfx.create_bitmap(usz, usz, &load_raw_sprite_bgra(RAW_DIVINE_UNIT_SPRITES[d], UNIT_SPRITE_SZ, false, 1.0));
            self.divine_unit_sprites_flip[d] = gfx.create_bitmap(usz, usz, &load_raw_sprite_bgra(RAW_DIVINE_UNIT_SPRITES[d], UNIT_SPRITE_SZ, true, 1.0));
        }
        for d in 0..2 {
            self.divine_bld_sprites[d] = gfx.create_bitmap(bsz, bsz, &load_raw_sprite_bgra(RAW_DIVINE_BLD_SPRITES[d], BLD_SPRITE_SZ, false, 1.0));
            self.divine_bld_sprites_fog[d] = gfx.create_bitmap(bsz, bsz, &load_raw_sprite_bgra(RAW_DIVINE_BLD_SPRITES[d], BLD_SPRITE_SZ, false, 0.55));
        }
    }

    pub fn divine_unit_sprite(&self, k: UnitKind, flip_x: bool) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            UnitKind::Cavalry => 1,
            _ => 0,
        };
        if flip_x {
            self.divine_unit_sprites_flip[idx].as_ref()
        } else {
            self.divine_unit_sprites[idx].as_ref()
        }
    }

    pub fn divine_bld_sprite(&self, k: BKind, fogged: bool) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            BKind::Capital => 0,
            _ => 1,
        };
        if fogged {
            self.divine_bld_sprites_fog[idx].as_ref()
        } else {
            self.divine_bld_sprites[idx].as_ref()
        }
    }

    pub fn bld_sprite(&self, k: BKind, fogged: bool) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            BKind::Capital => 0,
            BKind::Town => 1,
            BKind::Barracks => 2,
            BKind::Farm => 3,
            BKind::Fort => 4,
            BKind::Dock => 5,
        };
        if fogged {
            self.bld_sprites_fog[idx].as_ref()
        } else {
            self.bld_sprites[idx].as_ref()
        }
    }

    pub fn mountain_sprite(&self, flip_x: bool, fogged: bool) -> Option<&ID2D1Bitmap> {
        match (fogged, flip_x) {
            (false, false) => self.prop_sprites[0].as_ref(),
            (false, true) => self.prop_sprites_flip[0].as_ref(),
            (true, false) => self.prop_sprites_fog[0].as_ref(),
            (true, true) => self.prop_sprites_fog_flip[0].as_ref(),
        }
    }

    pub fn planting_sprite(&self, stage: usize) -> Option<&ID2D1Bitmap> {
        self.planting_sprites[stage].as_ref()
    }

    pub fn forest_sprite(&self, flip_x: bool, fogged: bool) -> Option<&ID2D1Bitmap> {
        match (fogged, flip_x) {
            (false, false) => self.prop_sprites[1].as_ref(),
            (false, true) => self.prop_sprites_flip[1].as_ref(),
            (true, false) => self.prop_sprites_fog[1].as_ref(),
            (true, true) => self.prop_sprites_fog_flip[1].as_ref(),
        }
    }

    pub fn mine_sprite(&self, fogged: bool) -> Option<&ID2D1Bitmap> {
        if fogged {
            self.prop_sprites_fog[2].as_ref()
        } else {
            self.prop_sprites[2].as_ref()
        }
    }

    pub fn national_unit_sprite(&self, k: UnitKind, era: Era, nation: usize, year: f32, flip_x: bool) -> Option<&ID2D1Bitmap> {
        if k == UnitKind::Infantry {
            if let Some(idx) = late_infantry_index(nation, year) {
                if let Some(pair) = self.late_infantry.get(idx) {
                    return pair[flip_x as usize].as_ref();
                }
            }
            let sprites = if flip_x { &self.national_infantry_flip } else { &self.national_infantry };
            if let Some(Some(bmp)) = sprites.get(nation) { return Some(bmp); }
        }
        if k == UnitKind::Tank && (nation == crate::geo::FRA || nation == crate::geo::USA) {
            return self.french_tank[flip_x as usize].as_ref();
        }
        self.unit_sprite(k, era, flip_x)
    }

    pub fn unit_sprite(&self, k: UnitKind, _era: Era, flip_x: bool) -> Option<&ID2D1Bitmap> {
        let idx = k as usize;
        if flip_x {
            self.unit_sprites_flip[idx].as_ref()
        } else {
            self.unit_sprites[idx].as_ref()
        }
    }

    pub fn bld_icon(&self, k: BKind) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            BKind::Farm => CmdIcon::Farm,
            BKind::Barracks => CmdIcon::Barracks,
            BKind::Town => CmdIcon::Town,
            BKind::Capital => CmdIcon::Capital,
            BKind::Fort => CmdIcon::Fort,
            BKind::Dock => CmdIcon::Dock,
        };
        self.cmd_icons[idx as usize].as_ref()
    }

    /// Age of Empires 風の石彫・金枠パネルを描画
    pub fn draw_stone_panel(&self, gfx: &Gfx, x0: f32, y0: f32, x1: f32, y1: f32) {
        if let Some(bmp) = &self.hud_stone {
            // タイル状に敷き詰める
            let tw = 256.0;
            let mut x = x0;
            while x < x1 {
                let nx = (x + tw).min(x1);
                gfx.draw_bitmap(bmp, x, y0, nx, y1);
                x += tw;
            }
        } else {
            gfx.fill_rect(x0, y0, x1, y1, [0.11, 0.09, 0.07, 0.96]);
        }
        // 立体的な金・青銅の二重縁取り
        gfx.stroke_rect(x0 + 0.5, y0 + 0.5, x1 - 0.5, y1 - 0.5, 2.0, [0.48, 0.57, 0.57, 0.95]);
        gfx.stroke_rect(x0 + 3.0, y0 + 3.0, x1 - 3.0, y1 - 3.0, 1.0, [0.16, 0.23, 0.25, 0.9]);
    }

    /// 内部のくぼんだ木目・竹簡ボックスを描画
    pub fn draw_inset_box(&self, gfx: &Gfx, x0: f32, y0: f32, x1: f32, y1: f32) {
        if let Some(bmp) = &self.panel_wood {
            gfx.draw_bitmap(bmp, x0, y0, x1, y1);
        } else {
            gfx.fill_round(x0, y0, x1, y1, 4.0, [0.14, 0.11, 0.08, 0.94]);
        }
        // 彫り込みの影と金枠
        gfx.stroke_rect(x0, y0, x1, y1, 1.5, [0.05, 0.04, 0.03, 0.9]);
        gfx.stroke_rect(x0 + 1.5, y0 + 1.5, x1 - 1.5, y1 - 1.5, 1.0, [0.40, 0.49, 0.49, 0.75]);
    }

    /// Drafting registration marks at panel corners.
    pub fn draw_corner_ornaments(&self, gfx: &Gfx, x0: f32, y0: f32, x1: f32, y1: f32, s: f32) {
        let rule = [0.65, 0.67, 0.58, 0.75];
        for (x, y, dx, dy) in [(x0,y0,1.0,1.0),(x1,y0,-1.0,1.0),(x0,y1,1.0,-1.0),(x1,y1,-1.0,-1.0)] {
            gfx.fill_rect(x.min(x + dx * s), y, x.max(x + dx * s), y + 1.0, rule);
            gfx.fill_rect(x, y.min(y + dy * s), x + 1.0, y.max(y + dy * s), rule);
        }
    }

}

/// タイル (x, y) 内のサブピクセル (px, py) の色を Age of Empires 風に高精細描画する
pub fn paint_aoe_terrain_pixel(
    terrain: &[crate::geo::Terrain],
    shade: &[f32],
    x: i32,
    y: i32,
    px: i32,
    py: i32,
    scale: i32,
) -> [f32; 3] {
    use crate::geo::{Terrain, MAP_H, MAP_W};

    let fx = x as f32 + (px as f32 + 0.5) / scale as f32;
    let fy = y as f32 + (py as f32 + 0.5) / scale as f32;

    // タイル境界の四角いカクつきを消すための有機的ドメインワープ
    let wx = fx + (fbm(fx * 1.4, fy * 1.4, 911, 2) - 0.5) * 0.52;
    let wy = fy + (fbm(fx * 1.4 + 31.0, fy * 1.4 + 17.0, 912, 2) - 0.5) * 0.52;

    let sample_tile = |tx: i32, ty: i32| -> (Terrain, f32) {
        let cx = tx.clamp(0, MAP_W as i32 - 1);
        let cy = ty.clamp(0, MAP_H as i32 - 1);
        let idx = cy as usize * MAP_W + cx as usize;
        (terrain[idx], shade[idx])
    };

    let is_water = |t: Terrain| matches!(t, Terrain::Sea | Terrain::Lake | Terrain::River);
    let lat_t = (fy / MAP_H as f32).clamp(0.0, 1.0);
    let macro_patch = fbm(fx * 0.35, fy * 0.35, 613, 3);
    let micro1 = fbm(fx * 3.4, fy * 3.4, 701, 3) - 0.5;
    let micro2 = hash2(x * scale + px, y * scale + py, 707) - 0.5;

    let tile_surface_color = |t: Terrain, sh: f32| -> [f32; 3] {
        match t {
            Terrain::Sea => {
                let deep = lerp3([0.05, 0.16, 0.33], [0.10, 0.27, 0.48], sh);
                lerp3(deep, [0.08, 0.22, 0.42], macro_patch * 0.35)
            }
            Terrain::Lake => lerp3([0.13, 0.35, 0.52], [0.18, 0.43, 0.60], macro_patch),
            Terrain::River => [0.16 + micro1 * 0.05, 0.39 + micro1 * 0.06, 0.58 + micro1 * 0.06],
            Terrain::Plain => {
                // 北欧・ロシアの涼しい緑から、中欧の緑野、地中海の乾いた草地へのグラデーション
                let north = [0.33, 0.48, 0.29];
                let mid = [0.40, 0.55, 0.25];
                let south = [0.56, 0.54, 0.31];
                let mut g_col = if lat_t < 0.45 {
                    lerp3(north, mid, (lat_t / 0.45).clamp(0.0, 1.0))
                } else {
                    lerp3(mid, south, ((lat_t - 0.45) / 0.55).clamp(0.0, 1.0))
                };
                // Age of Empires 風の陽光が差す草むらの濃淡パッチ
                g_col = lerp3(g_col, [g_col[0] * 1.12, g_col[1] * 1.10, g_col[2] * 0.94], (macro_patch - 0.35).clamp(0.0, 0.65));
                lerp3(g_col, [g_col[0] * 0.80, g_col[1] * 0.82, g_col[2] * 0.78], sh)
            }
            Terrain::Steppe => {
                let base = lerp3([0.68, 0.60, 0.37], [0.55, 0.49, 0.29], sh);
                [base[0] + micro1 * 0.08, base[1] + micro1 * 0.07, base[2] + micro1 * 0.05]
            }
            Terrain::Desert => {
                let base = lerp3([0.86, 0.76, 0.52], [0.76, 0.66, 0.44], sh);
                [base[0] + micro1 * 0.06, base[1] + micro1 * 0.06, base[2] + micro1 * 0.04]
            }
            Terrain::Forest => {
                // 樹林の下草・苔・腐葉土の深い緑（上にAI生成森林スプライトが重なる）
                let ground = [0.15, 0.29, 0.13];
                let moss = [0.22, 0.39, 0.16];
                let sun_leaf = [0.30, 0.47, 0.20];
                let tree_n = fbm(fx * 3.8, fy * 3.8, 909, 3);
                if tree_n > 0.64 {
                    sun_leaf
                } else if tree_n > 0.36 {
                    moss
                } else {
                    ground
                }
            }
            Terrain::Hill => {
                let h0 = fbm(fx * 1.6, fy * 1.6, 606, 3);
                let h1 = fbm((fx + 0.15) * 1.6, (fy + 0.15) * 1.6, 606, 3);
                let slope = (h0 - h1) * 3.2;
                let base = lerp3([0.55, 0.47, 0.30], [0.44, 0.38, 0.24], sh);
                [
                    (base[0] + slope * 0.18).clamp(0.1, 0.9),
                    (base[1] + slope * 0.16).clamp(0.1, 0.9),
                    (base[2] + slope * 0.12).clamp(0.1, 0.9),
                ]
            }
            Terrain::Mountain => {
                // 山岳基部の岩盤と山裾の陰影（上にAI生成山岳スプライトが重なる）
                let ridge = fbm(fx * 2.2, fy * 2.2, 505, 4);
                let ridge_se = fbm((fx + 0.12) * 2.2, (fy + 0.12) * 2.2, 505, 4);
                let lit = (ridge - ridge_se) * 3.6;
                let rock = lerp3([0.34, 0.31, 0.28], [0.50, 0.46, 0.40], ridge);
                [
                    (rock[0] + lit * 0.18).clamp(0.12, 0.85),
                    (rock[1] + lit * 0.17).clamp(0.12, 0.85),
                    (rock[2] + lit * 0.15).clamp(0.12, 0.85),
                ]
            }
        }
    };

    // 周囲4タイルの双線形スムーズステップ補間でタイルの四角い境界を完全に解消する
    let gx = wx - 0.5;
    let gy = wy - 0.5;
    let x0 = gx.floor() as i32;
    let y0 = gy.floor() as i32;
    let tx = gx - x0 as f32;
    let ty = gy - y0 as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);

    let (t00, s00) = sample_tile(x0, y0);
    let (t10, s10) = sample_tile(x0 + 1, y0);
    let (t01, s01) = sample_tile(x0, y0 + 1);
    let (t11, s11) = sample_tile(x0 + 1, y0 + 1);

    let c00 = tile_surface_color(t00, s00);
    let c10 = tile_surface_color(t10, s10);
    let c01 = tile_surface_color(t01, s01);
    let c11 = tile_surface_color(t11, s11);

    let c_top = lerp3(c00, c10, sx);
    let c_bot = lerp3(c01, c11, sx);
    let mut c = lerp3(c_top, c_bot, sy);

    let w00 = if is_water(t00) { 1.0 } else { 0.0 };
    let w10 = if is_water(t10) { 1.0 } else { 0.0 };
    let w01 = if is_water(t01) { 1.0 } else { 0.0 };
    let w11 = if is_water(t11) { 1.0 } else { 0.0 };
    let water_w = (w00 * (1.0 - sx) + w10 * sx) * (1.0 - sy) + (w01 * (1.0 - sx) + w11 * sx) * sy;

    // 海岸線・川岸の砂浜と浅瀬の美しいグラデーション
    if water_w > 0.04 && water_w < 0.96 {
        let sand = [0.76, 0.68, 0.46];
        let shallow = [0.22, 0.48, 0.60];
        if water_w < 0.48 {
            // 陸側の砂州・水際
            let beach_t = ((water_w - 0.04) / 0.44).clamp(0.0, 1.0);
            c = lerp3(c, sand, beach_t * 0.78);
        } else {
            // 水側の浅瀬と白いさざ波
            let shoal_t = ((0.96 - water_w) / 0.48).clamp(0.0, 1.0);
            c = lerp3(c, shallow, shoal_t * 0.72);
            let foam = ((water_w - 0.48).abs() < 0.08) as i32 as f32;
            if foam > 0.0 && fbm(fx * 4.0, fy * 4.0, 808, 2) > 0.46 {
                c = lerp3(c, [0.84, 0.90, 0.92], 0.40);
            }
        }
    }

    // 水面のきらめき・草地の微細ディテール
    if water_w >= 0.5 {
        let ripple = ((fx * 3.5 + fy * 1.2).sin() * (fx * 1.8 - fy * 4.0).cos()).max(0.0);
        if ripple > 0.75 {
            c = [c[0] + 0.08, c[1] + 0.10, c[2] + 0.12];
        }
    } else {
        c = [
            c[0] + micro1 * 0.055 + micro2 * 0.03,
            c[1] + micro1 * 0.055 + micro2 * 0.03,
            c[2] + micro1 * 0.045 + micro2 * 0.02,
        ];
    }

    [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0)]
}

fn c4(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

fn darker(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

/// マップ外周の Age of Empires 風木製・真鍮フレームを描く
pub fn draw_map_frame(g: &mut Gfx, mw: f32, mh: f32) {
    let b1 = 3.5;
    let b2 = 1.2;
    // 外側の重厚な黒檀・木枠
    g.rect(-b1, -b1, mw + b1, 0.0, [0.14, 0.10, 0.07, 1.0]);
    g.rect(-b1, mh, mw + b1, mh + b1, [0.10, 0.07, 0.05, 1.0]);
    g.rect(-b1, 0.0, 0.0, mh, [0.13, 0.09, 0.06, 1.0]);
    g.rect(mw, 0.0, mw + b1, mh, [0.09, 0.06, 0.04, 1.0]);
    // 内側の金・真鍮トリム
    g.rect_outline(-b2, -b2, mw + b2, mh + b2, 0.45, [0.78, 0.64, 0.34, 1.0]);
    g.rect_outline(-0.15, -0.15, mw + 0.15, mh + 0.15, 0.2, [0.25, 0.18, 0.10, 1.0]);
}

/// 遠景（zoom < 5.0）用の森林・山脈の簡易立体表現（zoom >= 5.0 では Direct2D 側でAI生成スプライトを描画）
pub fn draw_world_terrain_props(
    g: &mut Gfx,
    map: &crate::geo::Map,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    zoom: f32,
    fog_at: impl Fn(crate::world::V2) -> u8,
) {
    use crate::geo::{Map, Terrain};
    use crate::world::v2;

    if zoom >= 5.0 || zoom < 3.5 {
        return;
    }
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            if !Map::inb(tx, ty) {
                continue;
            }
            let center = v2(tx as f32 + 0.5, ty as f32 + 0.5);
            if fog_at(center) == 0 {
                continue;
            }
            if map.terrain[Map::idx(tx, ty)] == Terrain::Mountain {
                let px = tx as f32 + 0.48;
                let py = ty as f32 + 0.18;
                let by = ty as f32 + 0.94;
                g.tri((px, py), (tx as f32 - 0.1, by), (px + 0.05, by), [0.54, 0.49, 0.43, 0.85]);
                g.tri((px, py), (px + 0.05, by), (tx as f32 + 1.1, by), [0.26, 0.23, 0.21, 0.88]);
            }
        }
    }
}

/// 銅山の下地影（zoom < 5.0 ではポリゴン描画、zoom >= 5.0 では Direct2D 側でAI生成スプライトを描画）
pub fn draw_world_mine(g: &mut Gfx, c: crate::world::V2, amount: f32, px: f32) {
    let s = 0.36 + 0.16 * (amount / 2500.0).clamp(0.2, 1.0);
    g.circle(c.x + 0.05, c.y + 0.12, s * 0.95, 10, [0.0, 0.0, 0.0, 0.30]);
    if px <= 1.0 / 5.0 {
        return;
    }
    g.tri((c.x - s, c.y + s * 0.65), (c.x - s * 0.35, c.y - s * 0.85), (c.x + s * 0.2, c.y + s * 0.65), [0.52, 0.44, 0.36, 1.0]);
    g.tri((c.x - s * 0.35, c.y - s * 0.85), (c.x + s * 0.45, c.y - s * 0.65), (c.x + s, c.y + s * 0.65), [0.36, 0.30, 0.24, 1.0]);
    g.circle(c.x, c.y, s * 0.32, 6, [0.32, 0.85, 0.65, 1.0]);
}

/// 建物（都・邑・兵営・田）の地面基壇・国色境界を描画（本体は Direct2D 側でAI生成スプライトを描画）
pub fn draw_world_building(g: &mut Gfx, b: &crate::world::Building, nc: [f32; 3], px: f32) {
    let (x0, y0, x1, y1) = (b.tx as f32, b.ty as f32, b.tx as f32 + 2.0, b.ty as f32 + 2.0);
    let c = b.center();
    let alpha = if b.done() { 1.0 } else { 0.62 };

    if px <= 1.0 / 2.2 {
        // 建物の地面落影と国色トリム台座（上部に Direct2D で高精細AI生成建物スプライトを重ねる）
        match b.kind {
            BKind::Farm => {
                g.rect(x0 + 0.06, y0 + 0.06, x1 - 0.06, y1 - 0.06, [0.28, 0.22, 0.13, 0.45 * alpha]);
                g.rect_outline(x0 + 0.04, y0 + 0.04, x1 - 0.04, y1 - 0.04, (1.6 * px).max(0.05), c4(nc, 0.72 * alpha));
            }
            BKind::Barracks | BKind::Fort | BKind::Dock => {
                g.circle(c.x + 0.06, c.y + 0.14, 1.02, 16, [0.0, 0.0, 0.0, 0.34 * alpha]);
                g.rect_outline(x0 + 0.08, y0 + 0.08, x1 - 0.08, y1 - 0.08, (1.8 * px).max(0.06), c4(nc, 0.78 * alpha));
            }
            BKind::Town => {
                g.circle(c.x + 0.08, c.y + 0.16, 1.18, 18, [0.0, 0.0, 0.0, 0.38 * alpha]);
                g.rect_outline(x0 + 0.02, y0 + 0.02, x1 - 0.02, y1 - 0.02, (2.0 * px).max(0.07), c4(nc, 0.82 * alpha));
            }
            BKind::Capital => {
                g.circle(c.x + 0.10, c.y + 0.18, 1.42, 20, [0.0, 0.0, 0.0, 0.42 * alpha]);
                g.rect_outline(x0 - 0.14, y0 - 0.14, x1 + 0.14, y1 + 0.14, (2.2 * px).max(0.08), c4(nc, 0.88 * alpha));
            }
        }
        return;
    }

    // 超広域ズームアウト時（zoom < 5.0）のフォールバック描画
    let e = if b.kind == BKind::Capital { -0.25 } else { 0.0 };
    g.rect(x0 + e, y0 + e, x1 - e, y1 - e, c4(darker(nc, 0.75), alpha));
    g.rect_outline(x0 + e, y0 + e, x1 - e, y1 - e, (1.8 * px).max(0.06), [0.92, 0.78, 0.36, alpha]);
}

/// ユニットの足元リング・落影を描画（本体は Direct2D 側でAI生成スプライトを描画）
pub fn draw_world_unit(
    g: &mut Gfx,
    u: &crate::world::Unit,
    nc: [f32; 3],
    era: Era,
    res_colors: &[[f32; 4]; 3],
    px: f32,
) {
    use crate::world::v2;

    let p = u.pos;
    let body = if u.flash > 0.0 { [1.0, 1.0, 1.0, 1.0] } else { c4(nc, 1.0) };
    let dark = c4(darker(nc, 0.32), 1.0);
    let bronze = [0.92, 0.76, 0.36, 1.0];
    let wood = [0.52, 0.34, 0.18, 1.0];

    let sz = 0.82 + 0.06 * (u.power - 1.0).min(5.0);
    let d = if u.dir.len() > 0.01 { u.dir.norm() } else { v2(0.0, 1.0) };
    let n = v2(-d.y, d.x);
    let pt = |fwd: f32, side: f32| -> (f32, f32) {
        let v = p + d * (fwd * sz) + n * (side * sz);
        (v.x, v.y)
    };

    // 足元の落影と国色の台座リング（縮小したユニットサイズに合わせてコンパクト化）
    g.circle(p.x + 0.03, p.y + 0.10, 0.25 * sz, 12, [0.0, 0.0, 0.0, 0.32]);
    if u.selected {
        g.ring(p.x, p.y + 0.08, 0.38, (2.0 * px).max(0.045), 18, [0.35, 1.0, 0.45, 0.98]);
        g.ring(p.x, p.y + 0.08, 0.33, px.max(0.025), 18, [0.95, 1.0, 0.75, 0.65]);
    }
    if px <= 1.0 / 6.5 {
        // ズーム時は Direct2D 側で高精細AI生成ユニットスプライトを描画するため、足元に国色台座リングのみ描画
        g.circle(p.x, p.y + 0.08, 0.22 * sz, 12, [nc[0] * 0.65, nc[1] * 0.65, nc[2] * 0.65, 0.48]);
        g.ring(p.x, p.y + 0.08, 0.24 * sz, (1.5 * px).max(0.038), 14, [nc[0] * 0.65 + 0.35, nc[1] * 0.65 + 0.35, nc[2] * 0.65 + 0.35, 0.92]);
        if u.flash > 0.0 {
            g.circle(p.x, p.y, 0.28 * sz, 12, [1.0, 0.95, 0.85, 0.55]);
        }
        return;
    } else if !u.selected {
        g.ring(p.x, p.y, 0.24 * sz, (1.2 * px).max(0.03), 10, [nc[0], nc[1], nc[2], 0.55]);
    }

    match u.kind {
        UnitKind::Worker => {
            let r = 0.20 * sz;
            g.circle(p.x, p.y, r + 1.5 * px, 10, dark);
            g.circle(p.x, p.y, r, 10, body);
            if u.carry > 0.5 {
                let cc = res_colors[u.carry_kind];
                let (bx, by) = pt(-0.16, -0.18);
                g.circle(bx, by, 0.10 * sz, 8, cc);
            }
        }
        UnitKind::Infantry | UnitKind::Artillery | UnitKind::Storm | UnitKind::Alpine | UnitKind::Submarine | UnitKind::Transport => {
            let r = 0.22 * sz;
            g.circle(p.x, p.y, r + 1.5 * px, 10, dark);
            g.circle(p.x, p.y, r, 10, body);
            g.circle(p.x, p.y, r * 0.48, 8, bronze);
        }
        UnitKind::Cavalry | UnitKind::Tank | UnitKind::Destroyer => {
            let _ = (era, wood);
            let r = 0.25 * sz;
            g.circle(p.x, p.y, r + 1.5 * px, 10, dark);
            g.circle(p.x, p.y, r, 10, body);
            g.circle(p.x, p.y, r * 0.52, 8, bronze);
        }
    }
}


// Approximate introduction of standard field uniforms; this is visual only.
fn late_infantry_index(nation: usize, year: f32) -> Option<usize> {
    use crate::geo::{GER, GBR, FRA, BEL};
    match nation {
        GER if year >= 1916.5 => Some(0),
        GBR if year >= 1916.0 => Some(1),
        FRA if year >= 1915.5 => Some(2),
        BEL if year >= 1915.5 => Some(3),
        _ => None,
    }
}

#[cfg(test)]
mod asset_tests {
    use super::*;
    #[test]
    fn uniforms_follow_country_and_date() {
        use crate::geo::{GER, GBR, FRA, BEL, OTT};
        for n in [GER, GBR, FRA, BEL, OTT] { assert_eq!(late_infantry_index(n, 1914.5), None); }
        assert_eq!(late_infantry_index(FRA, 1915.5), Some(2));
        assert_eq!(late_infantry_index(BEL, 1915.5), Some(3));
        assert_eq!(late_infantry_index(GBR, 1916.0), Some(1));
        assert_eq!(late_infantry_index(GER, 1916.5), Some(0));
        assert_eq!(late_infantry_index(OTT, 1918.0), None);
    }
    #[test]
    fn embedded_sprites_have_valid_premultiplied_alpha() {
        for (sprites, size) in [(RAW_NATIONAL_INFANTRY.as_slice(),160), (RAW_LATE_INFANTRY.as_slice(),160), (RAW_UNIT_SPRITES.as_slice(),160), (RAW_BLD_SPRITES.as_slice(),192)] {
            for raw in sprites {
                assert_eq!(raw.len(), size * size * 4);
                assert!(raw.chunks_exact(4).any(|p| p[3] == 0));
                assert!(raw.chunks_exact(4).any(|p| p[3] > 200));
                assert!(raw.chunks_exact(4).all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]));
            }
        }
    }
}
