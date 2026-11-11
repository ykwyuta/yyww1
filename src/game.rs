//! 画面・入力・HUD（独自実装の UI）

mod chronicle;
mod status;

mod mobile;
mod save;
mod tutorial;

use crate::art::{paint_aoe_terrain_pixel, Art, CmdIcon, ResIcon};
use crate::geo::*;
use crate::gfx::{Font, Gfx, Texture};
use crate::rng::hash2;
use crate::world::*;
use crate::Ev;
use crate::gfx::Bitmap as ID2D1Bitmap;

const TEX_SCALE: usize = 8;
const OVERLAY_SCALE: usize = 4;
const TOP_H: f32 = 36.0;
const BOT_H: f32 = 176.0;
const MM_W: f32 = 196.0;
const MM_H: f32 = 168.0;

const VK_BACK: u32 = 0x08;
const VK_TAB: u32 = 0x09;
const VK_SHIFT: u32 = 0x10;
const VK_CONTROL: u32 = 0x11;
const VK_ESCAPE: u32 = 0x1B;
const VK_SPACE: u32 = 0x20;
const VK_HOME: u32 = 0x24;
const VK_LEFT: u32 = 0x25;
const VK_UP: u32 = 0x26;
const VK_RIGHT: u32 = 0x27;
const VK_DOWN: u32 = 0x28;
const VK_DELETE: u32 = 0x2E;
const VK_F1: u32 = 0x70;
const VK_F2: u32 = 0x71;
const VK_F3: u32 = 0x72;
const VK_F4: u32 = 0x73;
const VK_F5: u32 = 0x74;
/// 下の画面への操作を遮る透明なボタンの目印
const BLOCK_LABEL: &str = "#block";
const VK_F11: u32 = 0x7A;
const VK_ADD: u32 = 0x6B;
const VK_SUB: u32 = 0x6D;
const VK_OEM_PLUS: u32 = 0xBB;
const VK_OEM_MINUS: u32 = 0xBD;
const VK_OEM_PERIOD: u32 = 0xBE;

/// 費用表示用の短い資源名（英語は F/W/B）
fn res_short(k: usize) -> &'static str {
    match k {
        0 => t!("食", "F", "粮"),
        1 => t!("木", "W", "木"),
        _ => t!("鉄", "I", "铁"),
    }
}

/// 資源の名前（0:食糧 1:木材 2:鉄）
fn res_name(k: usize) -> &'static str {
    match k {
        0 => t!("食糧", "Food", "粮食"),
        1 => t!("木", "Wood", "木"),
        _ => t!("鉄", "Iron", "铁"),
    }
}
const RES_COLORS: [[f32; 4]; 3] = [[0.95, 0.85, 0.35, 1.0], [0.55, 0.85, 0.45, 1.0], [0.95, 0.6, 0.35, 1.0]];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Title,
    Playing,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Place(BKind),
    AttackMove,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Act {
    Train(UnitKind),
    Place(BKind),
    Upgrade,
    Convert,
    Stop,
    AttackMove,
    SelectAllWorkers,
    SelectAllMilitary,
    SelectKind(UnitKind),
    FilterKind(UnitKind),
    WorkersAuto,
    AllWorkersAuto,
    AllWorkersWood,
    AllWorkersMine,
    AllWorkersUpgrade,
    ArmyDefend,
    AllArmyAttack,
    AllArmyDefend,
    AllArmyUpgrade,
    Speed(u32),
    Diplo,
    Help,
    War(usize),
    /// 和睦の申し入れへの返答（true=受諾）
    PeaceAnswer(bool),
    Start,
    Home,
    Restart,
    CloseOverlay,
    Demolish,
    ResumeBuild,
    IdleWorker,
    Chron,
    ChronPage(i32),
    Market,
    /// 無制限潜水艦作戦の切り替え（ドイツ）
    Unrestricted,
    /// 選んだ兵を兵器に割り当てる
    Crew(UnitKind),
    Tutorial,
    TutNext,
    TutQuit,
    Scale(i32),
    Fullscreen,
    #[cfg(target_arch = "wasm32")]
    RefreshCache,
    /// 表示言語を切り替える
    Lang,
    SaveMenu,
    /// 選択・建設の取り消し（タッチ操作用の ✕）
    Cancel,
    /// 出来事の告知（バナー）を閉じる
    DismissBanner,
    /// 連合の一覧を開く・畳む（タッチ向けの画面）
    ToggleAlerts,
    /// いま出ているお知らせを消す（タッチ向けの画面）
    ClearMsgs,
    SaveSlot(u8),
    LoadSlot(u8),
    ExportSlot(u8),
    ImportFile,
    Noop,
}

struct Button {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    label: String,
    key: u32,
    enabled: bool,
    act: Act,
    tip: String,
}

pub struct Game {
    gfx: Gfx,
    art: Art,
    world: World,
    seed: u32,
    terrain_px: Vec<u32>,
    terrain_tex: Texture,
    overlay_px: Vec<u32>,
    overlay_tex: Texture,
    overlay_ver: u32,
    minimap_px: Vec<u32>,
    minimap_bmp: Option<ID2D1Bitmap>,
    minimap_gen: u32,
    minimap_ver: u32,
    minimap_t: f32,
    minimap_fog: u32,
    /// 視界：0=未踏、1=探索済み（霧）、2=視界内
    fog: Vec<u8>,
    fog_px: Vec<u32>,
    fog_tex: Texture,
    fog_t: f32,
    fog_ver: u32,
    cam: V2,
    zoom: f32,
    mouse: (f32, f32),
    keys: [bool; 256],
    ldown: Option<(f32, f32)>,
    mdrag: Option<(f32, f32, V2)>,
    mm_drag: bool,
    screen: Screen,
    mode: Mode,
    sel: Vec<Handle>,
    sel_b: Option<Handle>,
    /// 範囲選択・Shift で一緒に選んだ同じ種類の建物（sel_b を含む）。訓練はこれら全てに予約する
    sel_bs: Vec<Handle>,
    groups: Vec<Vec<Handle>>,
    speed: u32,
    paused: bool,
    /// 部隊を選んでから命令するまでの一時停止
    cmd_pause: bool,
    last_sel: Vec<Handle>,
    last_orders: u32,
    acc: f32,
    show_diplo: bool,
    show_help: bool,
    show_chron: bool,
    show_market: bool,
    /// 記録（セーブ・ロード）画面
    show_save: bool,
    /// いまの入力がタッチ操作か
    touch: bool,
    /// スマートフォン向け画面で一覧（外交・交易など）をなぞってスクロールした量（UI 座標）
    ov_scroll: f32,
    ov_key: u8,
    ov_max: f32,
    /// タッチ操作でツールチップを出しておく時刻
    tip_until: f32,
    /// タッチ操作の建設：1回目のタップで場所を示し、同じ場所を2回目にタップで建てる
    place_tap: Option<(f32, f32)>,
    /// タッチ向けの画面：連合の一覧を開いているか
    alerts_open: bool,
    /// タッチ向けの画面：この時刻までのお知らせは消した
    msg_cut: f32,
    /// 最後に記録した年（自動記録の間隔）
    autosave_years: f32,
    chron_scroll: usize,
    title_sel: Option<usize>,
    notice: Option<(String, f32)>,
    buttons: Vec<Button>,
    last_click: (f32, Option<Handle>),
    /// 建物を最後にクリックした時刻と建物（ダブルクリックで同じ種類を全選択）
    last_bclick: (f32, Option<Handle>),
    clock: f32,
    marker: Option<(V2, f32, bool)>,
    idle_cycle: usize,
    /// 自動撮影（YY_AUTO="国番号,秒,出力パス[,ズーム]"）
    auto: Option<(Option<usize>, f32, String, f32)>,
    script: Vec<(f32, Ev)>,
    tut: Option<tutorial::Tut>,
}

/// 兵種の説明
fn unit_desc(k: UnitKind) -> &'static str {
    match k {
        UnitKind::Worker => t!("生産ユニット。農場・森・鉱山で資源を集め、建物を建てる。兵は民から徴兵する。", "Worker. Gathers resources at farms, forests and mines, and builds. Soldiers are drafted from workers.", "生产单位。在农场、森林、矿山采集资源并建造建筑。兵由民征召。"),
        UnitKind::Infantry => t!("小銃を持つ歩兵。少し離れて撃つ。騎兵に強い。", "Riflemen who fire from short range. Strong against cavalry.", "持步枪的步兵，可稍远距离射击。克制骑兵。"),
        UnitKind::Artillery => t!("遠くから砲撃する。歩兵・戦車・建物に強いが、近づかれると脆い（騎兵に弱い）。", "Bombards from long range. Strong against infantry, tanks and buildings, but fragile up close (weak to cavalry).", "远程炮击。克制步兵、坦克与建筑，但被近身则脆弱（惧骑兵）。"),
        UnitKind::Cavalry => t!("足の速い騎兵。砲兵と民に強いが、機関銃の時代には歩兵に弱い。", "Fast cavalry. Strong against artillery and workers, but weak against infantry in the age of the machine gun.", "速度快的骑兵。克制炮兵与民，但在机枪时代惧步兵。"),
        UnitKind::Tank => t!("鉄の箱。歩兵・騎兵に強く、体力が多いが遅い。砲兵に弱い。兵5人を割り当てて造り、兵には戻せない。", "An iron box. Strong against infantry and cavalry with lots of HP, but slow. Weak to artillery. Crewed by 5 soldiers; they cannot be released.", "钢铁之箱。克制步兵与骑兵，体力多但缓慢。惧炮兵。由5名士兵编成，无法撤回。"),
        UnitKind::Storm => t!("浸透戦術の精鋭。歩兵・砲兵に強く、建物の攻撃に優れる。", "Elite infiltration troops. Strong against infantry and artillery, good at attacking buildings.", "渗透战术的精锐。克制步兵与炮兵，擅长攻击建筑。"),
        UnitKind::Alpine => t!("山岳部隊。丘や森を苦にせず、そこで受ける損害が少ない。", "Mountain troops. Unhindered by hills and forests, and take less damage there.", "山地部队。不受丘陵与森林影响，在其中受到的伤害较少。"),
        UnitKind::Submarine => t!("海の中から輸送船を沈める。駆逐艦にしか攻撃されない。軍港のそばの兵4人を割り当てて造り、兵には戻せない。", "Sinks transports from beneath the waves. Only destroyers can attack it. Crewed by 4 soldiers at a dock; they cannot be released.", "从海中击沉运输船。只有驱逐舰能攻击它。由军港附近4名士兵编成，无法撤回。"),
        UnitKind::Destroyer => t!("Uボートを狩る快速の軍艦。船団を守る。軍港のそばの兵6人を割り当てて造り、兵には戻せない。", "A fast warship that hunts U-boats and guards convoys. Crewed by 6 soldiers at a dock; they cannot be released.", "猎杀U艇、护卫船队的快速军舰。由军港附近6名士兵编成，无法撤回。"),
        UnitKind::Transport => t!("兵や物資を運ぶ船。海の向こうへ兵を動かすと、港で自動的に乗り降りする。", "A ship carrying troops or supplies. Order troops across the sea and they board and land at ports automatically.", "运送士兵或物资的船只。命令士兵渡海时，会在港口自动上下船。"),
    }
}

/// 世論の色（高いほど緑、低いほど赤）
fn opinion_color(op: f32) -> [f32; 3] {
    if op >= 50.0 {
        lerp3([0.95, 0.85, 0.40], [0.55, 0.95, 0.50], ((op - 50.0) / 50.0).clamp(0.0, 1.0))
    } else {
        lerp3([1.0, 0.30, 0.25], [0.95, 0.85, 0.40], (op / 50.0).clamp(0.0, 1.0))
    }
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn rgba(c: [f32; 3], a: f32) -> u32 {
    let r = (c[0].clamp(0.0, 1.0) * 255.0) as u32;
    let g = (c[1].clamp(0.0, 1.0) * 255.0) as u32;
    let b = (c[2].clamp(0.0, 1.0) * 255.0) as u32;
    let a = (a.clamp(0.0, 1.0) * 255.0) as u32;
    r | (g << 8) | (b << 16) | (a << 24)
}

fn bgra(c: [f32; 3]) -> u32 {
    let r = (c[0].clamp(0.0, 1.0) * 255.0) as u32;
    let g = (c[1].clamp(0.0, 1.0) * 255.0) as u32;
    let b = (c[2].clamp(0.0, 1.0) * 255.0) as u32;
    b | (g << 8) | (r << 16) | (255 << 24)
}

fn c4(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

fn darker(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

/// タイルの基本色
fn base_color(t: Terrain, x: i32, y: i32, shade: f32) -> [f32; 3] {
    let lat_t = (y as f32 / MAP_H as f32).clamp(0.0, 1.0);
    match t {
        Terrain::Sea => lerp3([0.10, 0.22, 0.42], [0.14, 0.30, 0.52], shade),
        Terrain::Lake => [0.20, 0.38, 0.58],
        Terrain::River => [0.24, 0.44, 0.66],
        Terrain::Plain => {
            let north = [0.36, 0.52, 0.32];
            let south = [0.58, 0.58, 0.34];
            let c = lerp3(north, south, ((lat_t - 0.45) * 2.0).clamp(0.0, 1.0));
            lerp3(c, darker(c, 0.85), shade)
        }
        Terrain::Steppe => lerp3([0.74, 0.70, 0.48], [0.66, 0.64, 0.42], shade),
        Terrain::Desert => lerp3([0.88, 0.78, 0.54], [0.80, 0.70, 0.48], shade),
        Terrain::Forest => lerp3([0.16, 0.36, 0.17], [0.22, 0.42, 0.20], shade),
        Terrain::Hill => lerp3([0.58, 0.50, 0.34], [0.50, 0.44, 0.30], shade),
        Terrain::Mountain => {
            let h = hash2(x, y, 99);
            if h > 0.93 {
                [0.78, 0.76, 0.74]
            } else {
                lerp3([0.42, 0.38, 0.35], [0.52, 0.48, 0.44], h)
            }
        }
    }
}

impl Game {
    pub fn new(gfx: Gfx) -> Game {
        let seed = 20240 + crate::platform::random_u32() % 1000;
        let world = World::new(seed);
        let tw = (MAP_W * TEX_SCALE) as u32;
        let th = (MAP_H * TEX_SCALE) as u32;
        let ow = (MAP_W * OVERLAY_SCALE) as u32;
        let oh = (MAP_H * OVERLAY_SCALE) as u32;
        let terrain_px = vec![0u32; (tw * th) as usize];
        let overlay_px = vec![0u32; (ow * oh) as usize];
        let terrain_tex = gfx.create_texture(tw, th, &terrain_px).expect("texture");
        let overlay_tex = gfx.create_texture(ow, oh, &overlay_px).expect("texture");
        let fog_px = vec![0u32; MAP_W * MAP_H];
        let fog_tex = gfx.create_texture(MAP_W as u32, MAP_H as u32, &fog_px).expect("texture");
        let mut g = Game {
            gfx,
            art: Art::new(),
            world,
            seed,
            terrain_px,
            terrain_tex,
            overlay_px,
            overlay_tex,
            overlay_ver: u32::MAX,
            minimap_px: vec![0; MAP_W * MAP_H],
            minimap_bmp: None,
            minimap_gen: 0,
            minimap_ver: u32::MAX,
            minimap_t: 0.0,
            minimap_fog: 0,
            fog: vec![2; MAP_W * MAP_H],
            fog_px,
            fog_tex,
            fog_t: 0.0,
            fog_ver: 0,
            cam: v2(MAP_W as f32 / 2.0, MAP_H as f32 / 2.0),
            zoom: 4.0,
            mouse: (-100.0, -100.0),
            keys: [false; 256],
            ldown: None,
            cmd_pause: false,
            last_sel: Vec::new(),
            last_orders: 0,
            mdrag: None,
            mm_drag: false,
            screen: Screen::Title,
            mode: Mode::Normal,
            sel: Vec::new(),
            sel_b: None,
            sel_bs: Vec::new(),
            groups: vec![Vec::new(); 10],
            speed: 1,
            paused: false,
            acc: 0.0,
            show_diplo: false,
            show_help: false,
            show_chron: false,
            show_market: false,
            show_save: false,
            touch: false,
            ov_scroll: 0.0,
            ov_key: 0,
            ov_max: 0.0,
            tip_until: 0.0,
            place_tap: None,
            alerts_open: false,
            msg_cut: -1.0,
            autosave_years: 0.0,
            chron_scroll: 0,
            title_sel: None,
            notice: None,
            buttons: Vec::new(),
            last_click: (-10.0, None),
            last_bclick: (-10.0, None),
            clock: 0.0,
            marker: None,
            idle_cycle: 0,
            script: Vec::new(),
            tut: None,
            auto: std::env::var("YY_AUTO").ok().and_then(|v| {
                let p: Vec<&str> = v.split(',').collect();
                Some((p.first()?.parse().ok(), p.get(1)?.parse().ok()?, p.get(2)?.to_string(), p.get(3).and_then(|z| z.parse().ok()).unwrap_or(22.0)))
            }),
        };
        g.rebuild_terrain();
        g.fit_title_camera();
        if std::env::var("YY_FULL").is_ok() {
            g.gfx.toggle_fullscreen();
        }
        g
    }

    fn fit_title_camera(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let avail_w = (w - 380.0).max(300.0);
        // 地図全体（大西洋と米国まで）。スマートフォンでは小さすぎて国名が重なるので、選べる国の首都の範囲に寄せる
        let (mut x0, mut y0, mut x1, mut y1) = (0.0f32, 0.0f32, MAP_W as f32, MAP_H as f32);
        if self.gfx.compact {
            let (mut a0, mut b0, mut a1, mut b1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for (i, n) in self.world.nations.iter().enumerate().take(NATIONS.len()) {
                if NATIONS[i].playable {
                    a0 = a0.min(n.home.x);
                    b0 = b0.min(n.home.y);
                    a1 = a1.max(n.home.x);
                    b1 = b1.max(n.home.y);
                }
            }
            if a0 < a1 {
                x0 = (a0 - 30.0).max(0.0);
                x1 = (a1 + 30.0).min(MAP_W as f32);
                y0 = (b0 - 24.0).max(0.0);
                y1 = (b1 + 24.0).min(MAP_H as f32);
            }
        }
        self.zoom = (avail_w / (x1 - x0)).min(h / (y1 - y0)) * 0.98;
        self.cam = v2((x0 + x1) / 2.0 + 380.0 / 2.0 / self.zoom, (y0 + y1) / 2.0);
    }

    fn rebuild_terrain(&mut self) {
        let tw = MAP_W * TEX_SCALE;
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                self.paint_tile(x, y, tw);
            }
        }
        let t = &self.terrain_tex;
        self.gfx.update_texture(t, 0, 0, t.w, t.h, &self.terrain_px);
        self.overlay_ver = u32::MAX;
        self.minimap_ver = u32::MAX;
    }

    fn paint_tile(&mut self, x: i32, y: i32, tw: usize) {
        let s = TEX_SCALE as i32;
        for py in 0..s {
            for px in 0..s {
                let gx = x * s + px;
                let gy = y * s + py;
                let c = paint_aoe_terrain_pixel(&self.world.map.terrain, &self.world.map.shade, x, y, px, py, s);
                self.terrain_px[gy as usize * tw + gx as usize] = rgba(c, 1.0);
            }
        }
    }

    fn update_dirty_terrain(&mut self) {
        if self.world.terrain_dirty.is_empty() {
            return;
        }
        let tw = MAP_W * TEX_SCALE;
        let dirty = std::mem::take(&mut self.world.terrain_dirty);
        for &(x, y) in dirty.iter() {
            self.paint_tile(x, y, tw);
            let s = TEX_SCALE;
            let mut buf = vec![0u32; s * s];
            for py in 0..s {
                for px in 0..s {
                    buf[py * s + px] = self.terrain_px[(y as usize * s + py) * tw + x as usize * s + px];
                }
            }
            self.gfx.update_texture(&self.terrain_tex, (x as usize * s) as u32, (y as usize * s) as u32, s as u32, s as u32, &buf);
        }
        self.minimap_ver = u32::MAX;
    }

    fn update_overlay(&mut self) {
        if self.overlay_ver == self.world.territory_version {
            return;
        }
        self.overlay_ver = self.world.territory_version;
        let w = &self.world;
        let os = OVERLAY_SCALE;
        let ow = MAP_W * os;
        let owner_at = |nx: i32, ny: i32| -> u8 {
            let cx = nx.clamp(0, MAP_W as i32 - 1);
            let cy = ny.clamp(0, MAP_H as i32 - 1);
            let ni = Map::idx(cx, cy);
            if w.map.terrain[ni] == Terrain::Sea {
                NONE
            } else {
                w.owner[ni]
            }
        };
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                let o = owner_at(x, y);
                for sy in 0..os {
                    for sx in 0..os {
                        let fx = x as f32 + (sx as f32 + 0.5) / os as f32 - 0.5;
                        let fy = y as f32 + (sy as f32 + 0.5) / os as f32 - 0.5;
                        let x0 = fx.floor() as i32;
                        let y0 = fy.floor() as i32;
                        let tx = fx - x0 as f32;
                        let ty = fy - y0 as f32;
                        let ux = tx * tx * (3.0 - 2.0 * tx);
                        let uy = ty * ty * (3.0 - 2.0 * ty);

                        let o00 = owner_at(x0, y0);
                        let o10 = owner_at(x0 + 1, y0);
                        let o01 = owner_at(x0, y0 + 1);
                        let o11 = owner_at(x0 + 1, y0 + 1);

                        // 最も近い有効な領主を決定
                        let dom = if o != NONE {
                            o
                        } else if o00 != NONE {
                            o00
                        } else if o10 != NONE {
                            o10
                        } else if o01 != NONE {
                            o01
                        } else {
                            o11
                        };
                        if dom == NONE {
                            self.overlay_px[(y as usize * os + sy) * ow + x as usize * os + sx] = 0;
                            continue;
                        }
                        let w00 = if o00 == dom { 1.0 } else { 0.0 };
                        let w10 = if o10 == dom { 1.0 } else { 0.0 };
                        let w01 = if o01 == dom { 1.0 } else { 0.0 };
                        let w11 = if o11 == dom { 1.0 } else { 0.0 };
                        let wt = (w00 * (1.0 - ux) + w10 * ux) * (1.0 - uy) + (w01 * (1.0 - ux) + w11 * ux) * uy;

                        let c = w.nations[dom as usize].color;
                        let px = if wt < 0.36 {
                            0
                        } else if wt < 0.60 {
                            rgba([c[0].min(1.0), c[1].min(1.0), c[2].min(1.0)], 0.86)
                        } else if wt < 0.80 {
                            rgba(c, 0.30)
                        } else {
                            rgba(c, 0.065)
                        };
                        self.overlay_px[(y as usize * os + sy) * ow + x as usize * os + sx] = px;
                    }
                }
            }
        }
        self.gfx.update_texture(&self.overlay_tex, 0, 0, ow as u32, (MAP_H * os) as u32, &self.overlay_px);
    }

    fn update_minimap(&mut self, dt: f32) {
        self.minimap_t -= dt;
        let gen_changed = self.minimap_gen != self.gfx.d2d_gen;
        let unchanged = self.minimap_ver == self.world.territory_version && self.minimap_fog == self.fog_ver;
        if !gen_changed && (self.minimap_t > 0.0 || unchanged) {
            return;
        }
        self.minimap_t = 1.0;
        self.minimap_ver = self.world.territory_version;
        self.minimap_fog = self.fog_ver;
        let fogged = self.fog_active();
        let w = &self.world;
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                let i = Map::idx(x, y);
                let mut c = base_color(w.map.terrain[i], x, y, w.map.shade[i]);
                let o = w.owner[i];
                if o != NONE {
                    c = lerp3(c, w.nations[o as usize].color, 0.55);
                }
                if fogged {
                    match self.fog[i] {
                        0 => c = [0.06, 0.06, 0.07],
                        1 => c = darker(c, 0.55),
                        _ => {}
                    }
                }
                self.minimap_px[i] = bgra(c);
            }
        }
        if gen_changed || self.minimap_bmp.is_none() {
            self.minimap_bmp = self.gfx.create_bitmap(MAP_W as u32, MAP_H as u32, &self.minimap_px);
            self.minimap_gen = self.gfx.d2d_gen;
        } else if let Some(b) = &self.minimap_bmp {
            self.gfx.update_bitmap(b, MAP_W as u32, &self.minimap_px);
        }
    }

    // ------------------------------------------------------------ 視界（戦場の霧）
    /// 霧を表示・適用するか（タイトル画面と終局後は全域を見せる）
    fn fog_active(&self) -> bool {
        self.screen == Screen::Playing && self.world.game_over.is_none()
    }

    fn reset_fog(&mut self) {
        self.fog.fill(0);
        self.reveal_cities();
        self.fog_t = 0.0;
        self.update_fog(0.0);
    }

    /// 都市の場所は地図に載っている：すべての都市（と各国の首都）の所在は初めから知られている
    fn reveal_cities(&mut self) {
        let mut known: Vec<(V2, i32)> = self.world.nations.iter().filter(|n| n.alive).map(|n| (n.home, 3)).collect();
        known.extend(self.world.blds.iter().filter(|b| b.alive && b.kind.city()).map(|b| (b.center(), b.kind.radius() as i32 + 2)));
        for (p, r) in known {
            let (x, y) = p.tile();
            Self::reveal(&mut self.fog, x, y, r, 1);
        }
    }

    fn reveal(fog: &mut [u8], cx: i32, cy: i32, r: i32, level: u8) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r + r {
                    continue;
                }
                let (x, y) = (cx + dx, cy + dy);
                if Map::inb(x, y) {
                    let f = &mut fog[Map::idx(x, y)];
                    *f = (*f).max(level);
                }
            }
        }
    }

    fn update_fog(&mut self, dt: f32) {
        self.fog_t -= dt;
        if self.fog_t > 0.0 {
            return;
        }
        self.fog_t = 0.15;
        for f in self.fog.iter_mut() {
            if *f == 2 {
                *f = 1;
            }
        }
        // 自国と同盟国（同じ陣営）の部隊・建物のまわりが見える
        let pl = self.world.player;
        let friend: Vec<bool> = (0..self.world.nations.len()).map(|n| n == pl || self.world.allied(pl, n)).collect();
        for u in self.world.units.iter() {
            if u.alive && friend.get(u.nation as usize).copied().unwrap_or(false) {
                let (x, y) = u.pos.tile();
                Self::reveal(&mut self.fog, x, y, u.kind.vision(), 2);
            }
        }
        for b in self.world.blds.iter() {
            if b.alive && friend.get(b.nation as usize).copied().unwrap_or(false) {
                let (x, y) = b.center().tile();
                let r = if b.kind.city() { b.kind.radius() as i32 + 3 } else { 5 };
                Self::reveal(&mut self.fog, x, y, r, 2);
            }
        }
        let fog_c = [0.04, 0.05, 0.07];
        for (px, &f) in self.fog_px.iter_mut().zip(self.fog.iter()) {
            *px = match f {
                0 => rgba(fog_c, 1.0),
                1 => rgba(fog_c, 0.5),
                _ => 0,
            };
        }
        self.gfx.update_texture(&self.fog_tex, 0, 0, MAP_W as u32, MAP_H as u32, &self.fog_px);
        self.fog_ver = self.fog_ver.wrapping_add(1);
    }

    /// その地点が今見えているか
    fn seen(&self, p: V2) -> bool {
        if !self.fog_active() {
            return true;
        }
        let (x, y) = p.tile();
        Map::inb(x, y) && self.fog[Map::idx(x, y)] == 2
    }

    /// その地点を一度でも見たか
    fn explored(&self, p: V2) -> bool {
        if !self.fog_active() {
            return true;
        }
        let (x, y) = p.tile();
        Map::inb(x, y) && self.fog[Map::idx(x, y)] >= 1
    }

    // ------------------------------------------------------------ 座標
    fn screen_to_world(&self, sx: f32, sy: f32) -> V2 {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        v2(self.cam.x + (sx - w / 2.0) / self.zoom, self.cam.y + (sy - h / 2.0) / self.zoom)
    }
    fn world_to_screen(&self, p: V2) -> (f32, f32) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        ((p.x - self.cam.x) * self.zoom + w / 2.0, (p.y - self.cam.y) * self.zoom + h / 2.0)
    }
    fn clamp_cam(&mut self) {
        self.cam.x = self.cam.x.clamp(0.0, MAP_W as f32);
        self.cam.y = self.cam.y.clamp(0.0, MAP_H as f32);
    }
    fn mm_rect(&self) -> (f32, f32, f32, f32) {
        let h = self.gfx.height as f32;
        (2.0, h - MM_H - 2.0, 2.0 + MM_W, h - 2.0)
    }
    /// 上部バーの高さ（スマートフォンでは指で押しやすいよう高くする）
    fn top_h(&self) -> f32 {
        if self.gfx.compact {
            mobile::TOP_H_COMPACT
        } else {
            TOP_H
        }
    }
    fn over_ui(&self, x: f32, y: f32) -> bool {
        if self.show_save {
            return true;
        }
        if self.screen == Screen::Title {
            return x > self.gfx.width as f32 - 380.0;
        }
        if let Some(t) = &self.tut {
            let (x0, y0, x1, y1) = t.panel;
            if x >= x0 && x <= x1 && y >= y0 && y <= y1 {
                return true;
            }
        }
        if self.show_diplo || self.show_help || self.show_chron || self.show_market || self.show_save || self.world.game_over.is_some() {
            return true;
        }
        if self.peace_pending() {
            return true;
        }
        y < self.top_h() || y > self.gfx.height as f32 - BOT_H
    }

    fn notice(&mut self, s: &str) {
        if !s.is_empty() {
            self.notice = Some((s.to_string(), 3.0));
        }
    }

    // ------------------------------------------------------------ 指揮中の一時停止
    /// 自国の部隊を新たに選ぶと時間を止め、命令を出すと再開する
    fn update_cmd_pause(&mut self) {
        let pl = self.world.player;
        let own = self.sel.iter().any(|&h| self.world.unit(h).map_or(false, |u| u.nation as usize == pl));
        if self.sel != self.last_sel {
            // 戦死などで減っただけなら選び直しとはみなさない
            let added = self.sel.iter().any(|h| !self.last_sel.contains(h));
            if added {
                self.cmd_pause = own;
            }
            self.last_sel = self.sel.clone();
        }
        if !own {
            self.cmd_pause = false;
        }
        if self.world.player_orders != self.last_orders {
            self.last_orders = self.world.player_orders;
            self.cmd_pause = false;
        }
    }

    /// プレイヤーの返答を待つ和睦の申し入れがあるか（返答するまで時間を止める）
    fn peace_pending(&self) -> bool {
        self.screen == Screen::Playing && self.world.game_over.is_none() && !self.world.peace_asks.is_empty()
    }

    /// 時間を止めているか（部隊の選択中・範囲選択のドラッグ中）
    fn cmd_hold(&self) -> bool {
        let dragging = self.ldown.map_or(false, |(sx, sy)| (sx - self.mouse.0).abs() > 4.0 || (sy - self.mouse.1).abs() > 4.0);
        self.cmd_pause || dragging
    }

    // ------------------------------------------------------------ フレーム
    /// true を返すと終了
    pub fn frame(&mut self, evs: &[Ev], dt: f32, focused: bool) -> bool {
        self.clock += dt;
        // OS からのマウス座標（実ピクセル）を UI 座標系へ
        let ui = self.gfx.ui;
        let mut evs: Vec<Ev> = evs
            .iter()
            .map(|e| match *e {
                Ev::MouseMove(x, y) => Ev::MouseMove(x / ui, y / ui),
                Ev::Down(b, x, y) => Ev::Down(b, x / ui, y / ui),
                Ev::Up(b, x, y) => Ev::Up(b, x / ui, y / ui),
                o => o,
            })
            .collect();
        while !self.script.is_empty() && self.script[0].0 <= self.clock {
            evs.push(self.script.remove(0).1);
        }
        for e in evs {
            if self.handle_event(e) {
                return true;
            }
        }
        if self.screen == Screen::Playing {
            self.scroll(dt, focused);
            self.update_cmd_pause();
            self.world.prune_peace_asks();
            if !self.paused && !self.cmd_hold() && self.world.game_over.is_none() && !self.show_help && !self.peace_pending() {
                self.acc += dt * self.speed as f32;
                let mut steps = 0;
                while self.acc >= DT && steps < 16 {
                    self.world.step();
                    self.acc -= DT;
                    steps += 1;
                    // 和睦の申し入れが届いたら、返答するまで時間を止める
                    if self.peace_pending() {
                        self.acc = 0.0;
                        break;
                    }
                }
                if steps >= 16 {
                    self.acc = 0.0;
                }
            }
            self.autosave_tick();
            self.update_fog(dt);
            self.update_tutorial(dt);
            let pl = self.world.player;
            let sel: Vec<Handle> = self.sel.iter().copied().filter(|&h| self.world.unit(h).map_or(false, |u| u.nation as usize == pl || self.seen(u.pos))).collect();
            self.sel = sel;
            {
                let w = &self.world;
                self.sel_bs.retain(|&h| w.bld(h).is_some());
            }
            if let Some(b) = self.sel_b {
                if self.world.bld(b).is_none() {
                    self.sel_b = self.sel_bs.first().copied();
                }
            }
            if self.sel_b.is_none() {
                self.sel_bs.clear();
            }
        }
        if let Some((_, t)) = &mut self.notice {
            *t -= dt;
            if *t <= 0.0 {
                self.notice = None;
            }
        }
        if let Some(m) = &mut self.marker {
            m.1 -= dt;
            if m.1 <= 0.0 {
                self.marker = None;
            }
        }
        if let Some((nation, secs, _, z)) = self.auto.clone() {
            if self.screen == Screen::Title {
                if let Some(step) = std::env::var("YY_TUT").ok().and_then(|v| v.parse::<usize>().ok()) {
                    self.start_tutorial();
                    self.tut_enter(step);
                    self.zoom = z;
                    if let Some(k) = std::env::var("YY_PRESS").ok().and_then(|v| u32::from_str_radix(&v, 16).ok()) {
                        self.script = vec![(self.clock + 0.5, Ev::Key(k, true)), (self.clock + 0.55, Ev::Key(k, false))];
                    }
                }
            }
            if self.screen == Screen::Title {
                if nation.is_none() {
                    // タイトル画面のまま国を選んでおく（YY_SEL=国番号）
                    if let Some(s) = std::env::var("YY_SEL").ok().and_then(|v| v.parse().ok()) {
                        self.title_sel = Some(s);
                    }
                }
                if let Some(n) = nation {
                    self.title_sel = Some(n);
                    self.do_act(Act::Start);
                    // 撮影用：自国も AI に任せる（YY_AIPLAY=1）
                    self.world.ai_player = std::env::var("YY_AIPLAY").is_ok();
                    // 撮影用：指定の年（紀元前）から始める（YY_YEAR=222 など）
                    if let Some(y) = std::env::var("YY_YEAR").ok().and_then(|v| v.parse::<f32>().ok()) {
                        self.world.years = y - START_YEAR;
                    }
                    // 撮影用：指定の国と交戦させ、和睦を申し入れさせる（YY_PEACE=国番号）
                    if let Some(o) = std::env::var("YY_PEACE").ok().and_then(|v| v.parse::<usize>().ok()) {
                        let pl = self.world.player;
                        self.world.declare_war(pl, o, true);
                        self.world.war_start[pl][o] -= 12.0;
                        self.world.propose_peace(o, pl, 8.0, t!("連合の解散", "the coalition disbanded", "联军解散"));
                    }
                    // 撮影用：民を1人選び、都の近くに田を3つ予約させる（YY_QUEUE=1）
                    if std::env::var("YY_QUEUE").is_ok() {
                        let pl = self.world.player;
                        let home = self.world.nations[pl].home;
                        let wk = (0..self.world.units.len()).find(|&i| self.world.units[i].alive && self.world.units[i].nation as usize == pl && self.world.units[i].kind == UnitKind::Worker);
                        if let Some(i) = wk {
                            let h = self.world.uh(i);
                            let mut spots: Vec<(i32, i32)> = Vec::new();
                            for r in 4..10 {
                                for dy in -r..=r {
                                    for dx in -r..=r {
                                        let (tx, ty) = (home.x as i32 + dx, home.y as i32 + dy);
                                        if spots.len() < 3 && self.world.can_place(pl, BKind::Farm, tx, ty).is_ok() && !spots.iter().any(|&(x, y)| (x - tx).abs() < 4 && (y - ty).abs() < 4) {
                                            spots.push((tx, ty));
                                        }
                                    }
                                }
                            }
                            for (tx, ty) in spots {
                                if let Ok(b) = self.world.place_building(pl, BKind::Farm, tx, ty) {
                                    self.world.queue_build(&[h], b);
                                }
                            }
                            self.set_sel(vec![h]);
                        }
                    }
                    self.speed = std::env::var("YY_SPEED").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
                    self.zoom = z;
                    // 撮影直前にキーを押す（例：72 で戦史を開く）
                    // YY_PRESS は「47,48」のように複数のキーを0.5秒おきに押せる
                    if let Ok(v) = std::env::var("YY_PRESS") {
                        let at = std::env::var("YY_PRESS_AT").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(secs - 0.6);
                        let t = at.max(self.clock + 0.3);
                        for (i, k) in v.split(',').filter_map(|s| u32::from_str_radix(s.trim(), 16).ok()).enumerate() {
                            let tk = t + i as f32 * 0.5;
                            self.script.push((tk, Ev::Key(k, true)));
                            self.script.push((tk + 0.05, Ev::Key(k, false)));
                        }
                    }
                    if std::env::var("YY_DEMO").is_ok() {
                        self.speed = 1;
                        let (cx, cy) = (self.gfx.width as f32 / 2.0, self.gfx.height as f32 / 2.0);
                        let c = self.clock;
                        self.script = vec![
                            (c + 0.3, Ev::MouseMove(cx, cy)),
                            (c + 0.4, Ev::Down(0, cx, cy)),
                            (c + 0.45, Ev::Up(0, cx, cy)),
                            (c + 0.6, Ev::Key(0x51, true)),
                            (c + 0.65, Ev::Key(0x51, false)),
                            (c + 0.9, Ev::Down(0, cx - 160.0, cy - 160.0)),
                            (c + 0.95, Ev::MouseMove(cx + 160.0, cy + 160.0)),
                            (c + 1.0, Ev::Up(0, cx + 160.0, cy + 160.0)),
                            (c + 1.2, Ev::Key(0x51, true)),
                            (c + 1.25, Ev::Key(0x51, false)),
                            (c + 1.4, Ev::MouseMove(cx + 140.0, cy + 80.0)),
                            (c + 1.6, Ev::Down(0, cx + 140.0, cy + 80.0)),
                            (c + 1.65, Ev::Up(0, cx + 140.0, cy + 80.0)),
                            (c + 1.8, Ev::Key(0x57, true)),
                            (c + 1.85, Ev::Key(0x57, false)),
                            (c + 2.0, Ev::MouseMove(cx - 150.0, cy + 60.0)),
                            (c + 2.4, Ev::Key(0x71, true)),
                            (c + 2.45, Ev::Key(0x71, false)),
                        ];
                    }
                }
            }
            if let Some(n) = nation {
                let _ = n;
                // 自国の兵に追従
                if self.clock > 1.0 {
                    let pl = self.world.player;
                    if let Some(u) = self.world.units.iter().find(|u| u.alive && u.nation as usize == pl && u.kind.military()) {
                        let _ = u;
                    }
                }
            }
            if self.clock >= secs {
                // 撮影時のカーソル位置（YY_MOUSE="x,y"、画面ピクセル）：ツールチップの確認用
                if let Some((mx, my)) = std::env::var("YY_MOUSE").ok().and_then(|v| {
                    let mut it = v.split(',').filter_map(|s| s.trim().parse::<f32>().ok());
                    Some((it.next()?, it.next()?))
                }) {
                    self.mouse = (mx / self.gfx.ui, my / self.gfx.ui);
                }
                self.update_dirty_terrain();
                self.update_overlay();
                self.render_nopresent(dt);
                let path = self.auto.as_ref().unwrap().2.clone();
                let _ = self.gfx.save_backbuffer(&path);
                return true;
            }
        }
        self.update_dirty_terrain();
        self.update_overlay();
        self.render(dt);
        false
    }

    fn scroll(&mut self, dt: f32, focused: bool) {
        let sp = 900.0 / self.zoom * dt;
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let (mx, my) = self.mouse;
        let mut d = v2(0.0, 0.0);
        if self.keys[VK_LEFT as usize] {
            d.x -= 1.0;
        }
        if self.keys[VK_RIGHT as usize] {
            d.x += 1.0;
        }
        if self.keys[VK_UP as usize] {
            d.y -= 1.0;
        }
        if self.keys[VK_DOWN as usize] {
            d.y += 1.0;
        }
        if focused && !self.touch && self.mdrag.is_none() && self.ldown.is_none() {
            if mx <= 3.0 && mx >= 0.0 {
                d.x -= 1.0;
            }
            if mx >= w - 4.0 && mx <= w {
                d.x += 1.0;
            }
            if my <= 3.0 && my >= 0.0 {
                d.y -= 1.0;
            }
            if my >= h - 4.0 && my <= h {
                d.y += 1.0;
            }
        }
        self.cam = self.cam + d * sp;
        self.clamp_cam();
    }

    fn handle_event(&mut self, e: Ev) -> bool {
        match e {
            Ev::Resize(w, h) => {
                self.gfx.resize(w, h);
                self.clamp_cam();
                if self.screen == Screen::Title {
                    self.fit_title_camera();
                }
            }
            Ev::MouseMove(x, y) => {
                self.mouse = (x, y);
                if let Some((sx, sy, c0)) = self.mdrag {
                    self.cam = c0 - v2(x - sx, y - sy) * (1.0 / self.zoom);
                    self.clamp_cam();
                }
                if self.mm_drag {
                    self.minimap_jump(x, y);
                }
            }
            Ev::Compact(c) => {
                self.gfx.set_compact(c);
                self.clamp_cam();
                if self.screen == Screen::Title {
                    self.fit_title_camera();
                }
            }
            Ev::Touch(t) => {
                self.touch = t;
                if t {
                    self.tip_until = self.clock + 2.5;
                }
            }
            Ev::Scroll(dy) => {
                self.ov_scroll = (self.ov_scroll - dy / self.gfx.ui).clamp(0.0, self.ov_max);
            }
            Ev::Wheel(d) if self.ov_max > 0.0 && self.overlay_open() => {
                self.ov_scroll = (self.ov_scroll - d * 60.0).clamp(0.0, self.ov_max);
            }
            Ev::Wheel(d) => {
                if self.screen == Screen::Playing && !self.over_ui(self.mouse.0, self.mouse.1) {
                    let before = self.screen_to_world(self.mouse.0, self.mouse.1);
                    self.zoom = (self.zoom * 1.15f32.powf(d)).clamp(1.2, 64.0);
                    let after = self.screen_to_world(self.mouse.0, self.mouse.1);
                    self.cam = self.cam + (before - after);
                    self.clamp_cam();
                }
            }
            Ev::Down(b, x, y) => {
                self.mouse = (x, y);
                match b {
                    0 => {
                        if let Some(a) = self.button_at(x, y) {
                            self.do_act(a);
                            return false;
                        }
                        if self.screen == Screen::Title {
                            self.title_click(x, y);
                            return false;
                        }
                        let (mx0, my0, mx1, my1) = self.mm_rect();
                        if x >= mx0 && x <= mx1 && y >= my0 && y <= my1 && !self.show_diplo && !self.show_help && !self.show_chron && !self.show_market && !self.show_save {
                            self.mm_drag = true;
                            self.minimap_jump(x, y);
                            return false;
                        }
                        if self.over_ui(x, y) {
                            return false;
                        }
                        match self.mode {
                            Mode::Place(_) if self.touch && self.place_tap.map_or(true, |(px, py)| (px - x).abs() > 22.0 || (py - y).abs() > 22.0) => {
                                // タッチ：1回目のタップは置き場所を見せるだけ
                                self.place_tap = Some((x, y));
                                self.notice(t!("同じ場所をもう一度タップすると建てる", "Tap the same spot again to build", "再次点击同一位置即可建造"));
                            }
                            Mode::Place(k) => {
                                self.place_tap = None;
                                // Ctrl を押しながら置くと建設の予約（続けて置ける）
                                let queue = self.keys[VK_CONTROL as usize];
                                self.try_place(k, queue);
                                if !self.keys[VK_SHIFT as usize] && !queue {
                                    self.mode = Mode::Normal;
                                }
                            }
                            Mode::AttackMove => {
                                let p = self.screen_to_world(x, y);
                                let hs = self.own_sel();
                                self.world.order_move(&hs, p, true);
                                self.marker = Some((p, 0.6, true));
                                self.mode = Mode::Normal;
                                if let Some(t) = &mut self.tut {
                                    t.moved = true;
                                }
                            }
                            Mode::Normal => self.ldown = Some((x, y)),
                        }
                    }
                    1 => {
                        if self.mode != Mode::Normal {
                            self.mode = Mode::Normal;
                            return false;
                        }
                        if self.screen != Screen::Playing {
                            return false;
                        }
                        let (mx0, my0, mx1, my1) = self.mm_rect();
                        if x >= mx0 && x <= mx1 && y >= my0 && y <= my1 && !self.show_diplo {
                            let p = v2((x - mx0) / MM_W * MAP_W as f32, (y - my0) / MM_H * MAP_H as f32);
                            self.right_click_world(p);
                            return false;
                        }
                        if !self.over_ui(x, y) {
                            let p = self.screen_to_world(x, y);
                            self.right_click_world(p);
                        }
                    }
                    _ => {
                        if self.screen == Screen::Playing {
                            self.mdrag = Some((x, y, self.cam));
                        }
                    }
                }
            }
            Ev::Up(b, x, y) => {
                self.mouse = (x, y);
                match b {
                    0 => {
                        self.mm_drag = false;
                        if let Some((sx, sy)) = self.ldown.take() {
                            // タッチ：自国の部隊を選んでいるとき、自国の兵・民以外をタップすれば命令（右クリックと同じ）
                            let tap = (sx - x).abs() < 12.0 && (sy - y).abs() < 12.0;
                            if self.touch && tap && self.screen == Screen::Playing && !self.own_sel().is_empty() {
                                let wp = self.screen_to_world(x, y);
                                let own_unit = self.unit_at(wp).and_then(|h| self.world.unit(h)).map_or(false, |u| u.nation as usize == self.world.player);
                                if !own_unit {
                                    self.right_click_world(wp);
                                    return false;
                                }
                            }
                            self.select_release(sx, sy, x, y);
                        }
                    }
                    2 => self.mdrag = None,
                    _ => {}
                }
            }
            Ev::Key(k, down) => {
                if (k as usize) < 256 {
                    self.keys[k as usize] = down;
                }
                if down {
                    return self.key_down(k);
                }
            }
            Ev::Char(_) => {}
        }
        false
    }

    fn minimap_jump(&mut self, x: f32, y: f32) {
        let (mx0, my0, _, _) = self.mm_rect();
        self.cam = v2(((x - mx0) / MM_W * MAP_W as f32).clamp(0.0, MAP_W as f32), ((y - my0) / MM_H * MAP_H as f32).clamp(0.0, MAP_H as f32));
    }

    fn key_down(&mut self, k: u32) -> bool {
        let ctrl = self.keys[VK_CONTROL as usize];
        match k {
            VK_F5 => {
                self.do_act(Act::SaveMenu);
                return false;
            }
            VK_F11 => {
                self.do_act(Act::Fullscreen);
                return false;
            }
            VK_ADD | VK_OEM_PLUS if ctrl => {
                self.do_act(Act::Scale(1));
                return false;
            }
            VK_SUB | VK_OEM_MINUS if ctrl => {
                self.do_act(Act::Scale(-1));
                return false;
            }
            _ => {}
        }
        if self.screen == Screen::Title {
            if k == VK_ESCAPE {
                if self.show_save {
                    self.show_save = false;
                    return false;
                }
                return true;
            }
            if k == VK_SPACE || k == 0x0D {
                if self.title_sel.is_some() {
                    self.do_act(Act::Start);
                }
            }
            return false;
        }
        // ボタンのホットキー
        if !ctrl {
            let mut hit = None;
            for b in self.buttons.iter() {
                if b.key == k && b.enabled {
                    hit = Some(b.act);
                    break;
                }
            }
            if let Some(a) = hit {
                self.do_act(a);
                return false;
            }
        }
        match k {
            VK_ESCAPE => {
                if self.mode != Mode::Normal {
                    self.mode = Mode::Normal;
                } else if self.show_diplo || self.show_help || self.show_chron || self.show_market || self.show_save {
                    self.show_diplo = false;
                    self.show_help = false;
                    self.show_chron = false;
                    self.show_market = false;
                    self.show_save = false;
                } else if self.world.banner.is_some() {
                    // 出来事の告知を閉じる
                    self.world.dismiss_banner();
                } else {
                    self.clear_sel();
                }
            }
            VK_SPACE => self.paused = !self.paused,
            VK_F1 => self.show_help = !self.show_help,
            VK_F2 => self.show_diplo = !self.show_diplo,
            VK_F3 => self.do_act(Act::Chron),
            VK_F4 => self.do_act(Act::Market),
            VK_HOME => self.do_act(Act::Home),
            VK_ADD | VK_OEM_PLUS => self.speed = (self.speed * 2).min(8),
            VK_SUB | VK_OEM_MINUS => self.speed = (self.speed / 2).max(1),
            VK_OEM_PERIOD => self.do_act(Act::IdleWorker),
            VK_DELETE => self.do_act(Act::Demolish),
            VK_TAB | VK_BACK => {}
            0x30..=0x39 => {
                let g = (k - 0x30) as usize;
                if ctrl {
                    self.groups[g] = self.own_sel();
                    self.notice(&tf!("部隊{}に登録", "Saved as group {}", "编为第{}队", g));
                } else {
                    let hs: Vec<Handle> = self.groups[g].iter().copied().filter(|&h| self.world.unit(h).is_some()).collect();
                    if !hs.is_empty() {
                        let double = self.last_click.0 > self.clock - 0.4 && self.last_click.1 == Some(hs[0]);
                        self.set_sel(hs.clone());
                        if double {
                            let p = self.world.unit(hs[0]).unwrap().pos;
                            self.cam = p;
                        }
                        self.last_click = (self.clock, Some(hs[0]));
                    }
                }
            }
            _ => {}
        }
        false
    }

    fn clear_sel(&mut self) {
        for h in self.sel.drain(..) {
            if let Some(u) = self.world.unit_mut(h) {
                u.selected = false;
            }
        }
        self.sel_b = None;
        self.sel_bs.clear();
    }

    /// 選んでいる建物（同じ種類のものを何棟も選べる）
    fn sel_blds(&self) -> Vec<Handle> {
        if self.sel_bs.is_empty() {
            self.sel_b.into_iter().collect()
        } else {
            self.sel_bs.clone()
        }
    }

    /// 訓練で同じ組に入る建物の種類（首都と都市は同じ組）
    fn train_group(k: BKind) -> u8 {
        match k {
            BKind::Capital | BKind::Town => 1,
            BKind::Barracks => 2,
            BKind::Dock => 3,
            _ => 0,
        }
    }

    fn set_sel(&mut self, hs: Vec<Handle>) {
        self.clear_sel();
        for &h in hs.iter() {
            if let Some(u) = self.world.unit_mut(h) {
                u.selected = true;
            }
        }
        self.sel = hs;
    }

    fn own_sel(&self) -> Vec<Handle> {
        let p = self.world.player;
        self.sel.iter().copied().filter(|&h| self.world.unit(h).map_or(false, |u| u.nation as usize == p)).collect()
    }

    fn all_workers(&self) -> Vec<Handle> {
        let p = self.world.player;
        (0..self.world.units.len())
            .filter(|&i| {
                let u = &self.world.units[i];
                u.alive && u.nation as usize == p && u.kind == UnitKind::Worker
            })
            .map(|i| self.world.uh(i))
            .collect()
    }

    /// 兵の数の表示
    fn soldier_text(&self, n: u32, _pop: u32) -> String {
        format!("{}", n)
    }

    /// 世論の説明（内訳つき）
    fn opinion_tip(&self, n: usize) -> String {
        let p = self.world.opinion_parts(n);
        let nat = &self.world.nations[n];
        let gov = if NATIONS[n].gov == Gov::Empire {
            t!("帝政の国：世論が 0 近くまで落ちると革命が起きて降伏する", "Empire: if opinion falls near 0, revolution forces surrender", "帝制国家：舆论接近0时将爆发革命而投降")
        } else {
            t!("民主制の国：革命は起きないが、世論が 50 を割るとストライキで生産が落ちる", "Democracy: no revolution, but below 50 strikes cut production", "民主制国家：不会革命，但舆论低于50时罢工会降低生产")
        };
        tf!(
            "世論 {:.0}/100。{}\n今月の見込み：戦争の長期化 {:+.1}・損害の差 {:+.1}・軍事費 {:+.1}・食糧 {:+.1}・都市の得失 {:+.1}\n損害の累計：失った兵力 {:.0}・与えた兵力 {:.0}　軍事費の比率 {:.0}%\n（クリックで戦況 F4）",
            "Public opinion {:.0}/100. {}\nThis month: war weariness {:+.1}, casualty balance {:+.1}, military spending {:+.1}, food {:+.1}, cities won/lost {:+.1}\nTotal losses: lost {:.0}, inflicted {:.0}   Military burden {:.0}%\n(Click for the War screen, F4)",
            "舆论 {:.0}/100。{}\n本月预计：战争长期化 {:+.1}・损失差 {:+.1}・军费 {:+.1}・粮食 {:+.1}・城市得失 {:+.1}\n累计损失：失去兵力 {:.0}・造成兵力 {:.0}　军费比率 {:.0}%\n（点击查看战况 F4）",
            nat.opinion, gov, p.weariness, p.casualties, p.spending, p.food, p.cities, nat.lost_total, nat.inflicted_total, self.world.mil_burden(n) * 100.0
        )
    }

    fn all_military(&self) -> Vec<Handle> {
        let p = self.world.player;
        (0..self.world.units.len())
            .filter(|&i| {
                let u = &self.world.units[i];
                u.alive && u.nation as usize == p && u.kind.military()
            })
            .map(|i| self.world.uh(i))
            .collect()
    }

    fn upgrade_handles(&mut self, mut hs: Vec<Handle>) {
        let mut err = None;
        let mut ok = 0;
        hs.sort_by(|a, b| self.world.unit(*a).unwrap().power.partial_cmp(&self.world.unit(*b).unwrap().power).unwrap());
        for h in hs {
            match self.world.upgrade_unit(h) {
                Ok(()) => ok += 1,
                Err(e) => err = Some(e),
            }
        }
        if ok == 0 {
            if let Some(e) = err {
                self.notice(e);
            }
        } else {
            self.notice(&tf!("{}部隊の能力を高めた", "Raised the power of {} units", "提升了{}个单位的能力", ok));
        }
    }

    /// 民（選択民または全民）を空いている田・森・銅山へバランス配分する
    fn order_workers_balanced(&mut self, hs: &[Handle], skip_builders: bool) {
        let pl = self.world.player;
        let mut gathering = [0usize; 3];
        for u in self.world.units.iter() {
            if u.alive && u.nation as usize == pl && u.kind == UnitKind::Worker && !hs.iter().any(|&h| self.world.unit(h).map_or(false, |x| std::ptr::eq(x, u))) {
                if let Order::Gather { src, .. } = u.order {
                    gathering[src.res()] += 1;
                }
            }
        }
        let mut assigned = 0;
        let mut built = 0;
        let res = self.world.nations[pl].res;
        let mut want = [0.44f32, 0.33, 0.23];
        if self.world.nations[pl].upkeep > self.world.nations[pl].income[0] * 0.55 {
            want[0] += 0.18;
        }
        for k in 0..3 {
            if res[k] > 700.0 {
                want[k] *= 0.45;
            }
        }
        let sumw: f32 = want.iter().sum();

        for &h in hs {
            let Some(u) = self.world.unit(h) else { continue };
            if u.kind != UnitKind::Worker {
                continue;
            }
            if skip_builders && matches!(u.order, Order::Build(_)) {
                continue;
            }
            if let Order::Build(bh) = u.order {
                if self.world.bld(bh).map_or(false, |b| !b.done()) && self.world.bld_builders(bh) <= 1 {
                    continue;
                }
            }
            let p = u.pos;
            // 作業者のいない未完成の建物があれば最優先で建設を再開させる
            if let Some(bh) = self.world.unfinished_bld(pl, p) {
                self.world.order_build(&[h], bh);
                built += 1;
                assigned += 1;
                continue;
            }
            let total = (gathering[0] + gathering[1] + gathering[2] + 1) as f32;
            let mut best_k = 0;
            let mut best_def = f32::MIN;
            for k in 0..3 {
                let def = want[k] / sumw * total - gathering[k] as f32;
                if def > best_def {
                    best_def = def;
                    best_k = k;
                }
            }
            let src = match best_k {
                0 => self.world.free_farm(pl, p).map(Src::Farm),
                2 => self.world.nearest_mine(p, 28.0).map(Src::Mine),
                _ => self.world.nearest_forest(p, 22).map(|(x, y)| Src::Forest(x, y)),
            }
            .or_else(|| self.world.free_farm(pl, p).map(Src::Farm))
            .or_else(|| self.world.nearest_forest(p, 28).map(|(x, y)| Src::Forest(x, y)))
            .or_else(|| self.world.nearest_mine(p, 36.0).map(Src::Mine));
            if let Some(s) = src {
                gathering[s.res()] += 1;
                self.world.order_gather(&[h], s);
                assigned += 1;
            }
        }
        if assigned > 0 {
            if built > 0 {
                self.notice(&tf!("未完成の建物へ民{}体を派遣し、計{}体を自動配分した", "Sent {} workers to unfinished buildings; {} assigned in all", "派{}名民前往未完工建筑，共自动分配{}名", built, assigned));
            } else {
                self.notice(&tf!("民{}体を農場・森・鉱山へ自動配分した", "Assigned {} workers to farms, forests and mines", "已将{}名民分配到农场、森林、矿山", assigned));
            }
        } else {
            self.notice(t!("配分できる民や資源が近くにない", "No workers or resources nearby to assign", "附近没有可分配的民或资源"));
        }
    }

    /// 指定した未完成建物へ、近くの民を1体派遣して建設を再開・加速させる
    fn dispatch_worker_to_build(&mut self, bh: Handle) {
        let pl = self.world.player;
        let Some(b) = self.world.bld(bh) else { return };
        if b.nation as usize != pl || b.done() {
            return;
        }
        let c = b.center();
        let bname = b.kind.name();
        let mut best: Option<(f32, Handle)> = None;
        for (i, u) in self.world.units.iter().enumerate() {
            if !u.alive || u.nation as usize != pl || u.kind != UnitKind::Worker {
                continue;
            }
            if u.order == Order::Build(bh) {
                continue;
            }
            let pen = match u.order {
                Order::Idle => 0.0,
                Order::Build(_) => 100.0,
                _ => 8.0,
            };
            let score = u.pos.dist(c) + pen;
            if best.map_or(true, |bs| score < bs.0) {
                best = Some((score, self.world.uh(i)));
            }
        }
        if let Some((_, wh)) = best {
            self.world.order_build(&[wh], bh);
            self.marker = Some((c, 0.6, false));
            self.notice(&tf!("民を{}の建設へ派遣した", "Sent workers to build the {}", "已派民去建造{}", bname));
        } else if self.world.bld_builders(bh) > 0 {
            self.notice(&tf!("すでに民が{}を建設中", "Workers are already building the {}", "已有民在建造{}", bname));
        } else {
            self.notice(t!("派遣できる民がいない", "No workers to send", "没有可派遣的民"));
        }
    }

    /// 民に一斉伐採（最寄りの森）または一斉採掘（最寄りの銅山）を命じる
    fn order_workers_resource(&mut self, hs: &[Handle], wood: bool) {
        let mut assigned = 0;
        let home = self.world.nations[self.world.player].home;
        for &h in hs {
            let Some(u) = self.world.unit(h) else { continue };
            if u.kind != UnitKind::Worker || matches!(u.order, Order::Build(_)) {
                continue;
            }
            let p = u.pos;
            let src = if wood {
                self.world
                    .nearest_forest(p, 28)
                    .or_else(|| self.world.nearest_forest(home, 36))
                    .map(|(x, y)| Src::Forest(x, y))
            } else {
                self.world
                    .nearest_mine(p, 35.0)
                    .or_else(|| self.world.nearest_mine(home, 45.0))
                    .map(Src::Mine)
            };
            if let Some(s) = src {
                self.world.order_gather(&[h], s);
                assigned += 1;
            }
        }
        if assigned > 0 {
            let kind_str = if wood { t!("森の伐採", "cut wood", "伐木") } else { t!("鉱山の採掘", "mine iron", "采矿") };
            self.notice(&tf!("民{}体に{}を命じた", "Ordered {} workers to {}", "命令{}名民{}", assigned, kind_str));
        } else {
            self.notice(if wood { t!("近くに伐採できる森がない", "No forest nearby to cut", "附近没有可砍伐的森林") } else { t!("近くに採掘できる鉱山がない", "No mine nearby", "附近没有可开采的矿山") });
        }
    }

    /// 軍事ユニットに領内防衛・侵入者迎撃・城邑集結を命じる
    fn order_army_defend(&mut self, hs: &[Handle]) {
        if hs.is_empty() {
            self.notice(t!("軍事ユニットがいない", "No soldiers", "没有军事单位"));
            return;
        }
        let pl = self.world.player;
        let home = self.world.nations[pl].home;
        let mut intruder: Option<V2> = None;
        for u in self.world.units.iter() {
            if !u.alive || !self.world.at_war(pl, u.nation as usize) {
                continue;
            }
            let (x, y) = u.pos.tile();
            if Map::inb(x, y) && self.world.owner[Map::idx(x, y)] as usize == pl {
                if intruder.map_or(true, |p| p.dist(home) > u.pos.dist(home)) {
                    intruder = Some(u.pos);
                }
            }
        }
        if let Some(p) = intruder {
            self.world.order_move(hs, p, true);
            self.marker = Some((p, 0.6, true));
            self.notice(&tf!("{}部隊に領内侵入者の迎撃を命じた", "Ordered {} units to intercept intruders", "命令{}个单位迎击入侵者", hs.len()));
            if let Some(t) = &mut self.tut {
                t.moved = true;
            }
            return;
        }
        let cities: Vec<V2> = self
            .world
            .blds
            .iter()
            .filter(|b| b.alive && b.nation as usize == pl && b.kind.city() && b.done())
            .map(|b| b.center())
            .collect();
        let rally = if cities.is_empty() {
            home
        } else {
            // 各部隊から最も近い自国城邑へ帰還・防衛配置
            for (k, &h) in hs.iter().enumerate() {
                if let Some(u) = self.world.unit(h) {
                    let up = u.pos;
                    let c = cities.iter().copied().min_by(|a, b| a.dist(up).partial_cmp(&b.dist(up)).unwrap()).unwrap_or(home);
                    let ang = (k as f32) * 1.1;
                    let goal = c + v2(ang.cos() * 2.2, ang.sin() * 2.2);
                    self.world.order_move(&[h], goal, true);
                }
            }
            cities[0]
        };
        self.marker = Some((rally, 0.6, false));
        self.notice(&tf!("{}部隊に都市の防衛集結を命じた", "Ordered {} units to gather and defend the cities", "命令{}个单位集结防守城邑", hs.len()));
        if let Some(t) = &mut self.tut {
            t.moved = true;
        }
    }

    /// 全軍に総進撃（交戦国の最寄り城邑・敵兵への攻撃移動＋進軍モード）を命じる
    fn order_all_army_attack(&mut self) {
        let hs = self.all_military();
        if hs.is_empty() {
            self.notice(t!("軍事ユニットがいない", "No soldiers", "没有军事单位"));
            return;
        }
        self.set_sel(hs.clone());
        let pl = self.world.player;
        let home = self.world.nations[pl].home;
        // 交戦中の敵城邑または敵部隊を探す
        let mut best_target: Option<(f32, V2)> = None;
        for b in self.world.blds.iter() {
            if b.alive && self.world.at_war(pl, b.nation as usize) && b.kind.city() {
                let d = b.center().dist(home);
                if best_target.map_or(true, |x| d < x.0) {
                    best_target = Some((d, b.center()));
                }
            }
        }
        if best_target.is_none() {
            for u in self.world.units.iter() {
                if u.alive && self.world.at_war(pl, u.nation as usize) {
                    let d = u.pos.dist(home);
                    if best_target.map_or(true, |x| d < x.0) {
                        best_target = Some((d, u.pos));
                    }
                }
            }
        }
        if let Some((_, goal)) = best_target {
            self.world.order_move(&hs, goal, true);
            self.marker = Some((goal, 0.6, true));
            self.mode = Mode::AttackMove;
            self.notice(&tf!("全軍{}部隊に敵陣への総進撃を命じた（地図クリックで目標変更可）", "All {} units ordered to advance on the enemy (click the map to change the target)", "全军{}个单位向敌阵总攻（点击地图可改变目标）", hs.len()));
            if let Some(t) = &mut self.tut {
                t.moved = true;
            }
        } else {
            self.mode = Mode::AttackMove;
            self.notice(&tf!("全軍{}部隊を選択：進軍先を地図上でクリックしてください", "All {} units selected: click the map to choose where to march", "已选全军{}个单位：请在地图上点击进军目标", hs.len()));
        }
    }

    fn unit_at(&self, p: V2) -> Option<Handle> {
        let mut best: Option<(f32, Handle)> = None;
        for (i, u) in self.world.units.iter().enumerate() {
            if !u.alive {
                continue;
            }
            if u.nation as usize != self.world.player && !self.seen(u.pos) {
                continue;
            }
            let r: f32 = if u.kind == UnitKind::Worker { 0.45 } else { 0.55 };
            // タッチは指の太さぶん広く拾う（画面上で約 18 単位）
            let r = if self.touch { r.max(18.0 / self.zoom) } else { r };
            let d = u.pos.dist(p);
            if d < r && best.map_or(true, |b| d < b.0) {
                best = Some((d, self.world.uh(i)));
            }
        }
        best.map(|b| b.1)
    }

    fn bld_at(&self, p: V2) -> Option<Handle> {
        let (x, y) = p.tile();
        if Map::inb(x, y) {
            let b = self.world.bmap[Map::idx(x, y)];
            if b != 0 && self.explored(p) {
                return Some(self.world.bh(b as usize - 1));
            }
        }
        let mut best: Option<(f32, Handle)> = None;
        for (i, b) in self.world.blds.iter().enumerate() {
            if !b.alive || !self.explored(b.center()) {
                continue;
            }
            let d = b.dist_to_footprint(p);
            if d < 0.45 && best.map_or(true, |bs| d < bs.0) {
                best = Some((d, self.world.bh(i)));
            }
        }
        best.map(|bs| bs.1)
    }

    fn select_release(&mut self, sx: f32, sy: f32, x: f32, y: f32) {
        let p = self.world.player;
        if (sx - x).abs() < 5.0 && (sy - y).abs() < 5.0 {
            let wp = self.screen_to_world(x, y);
            if let Some(h) = self.unit_at(wp) {
                let shift = self.keys[VK_SHIFT as usize];
                let double = self.last_click.0 > self.clock - 0.35 && self.last_click.1 == Some(h);
                self.last_click = (self.clock, Some(h));
                let u = self.world.unit(h).unwrap();
                if double && u.nation as usize == p {
                    // 画面内の同種ユニットを全選択
                    let kind = u.kind;
                    let (w, hh) = (self.gfx.width as f32, self.gfx.height as f32);
                    let a = self.screen_to_world(0.0, self.top_h());
                    let b = self.screen_to_world(w, hh - BOT_H);
                    let hs: Vec<Handle> = (0..self.world.units.len())
                        .filter(|&i| {
                            let u = &self.world.units[i];
                            u.alive && u.nation as usize == p && u.kind == kind && u.pos.x >= a.x && u.pos.x <= b.x && u.pos.y >= a.y && u.pos.y <= b.y
                        })
                        .map(|i| self.world.uh(i))
                        .collect();
                    self.set_sel(hs);
                } else if shift && u.nation as usize == p {
                    let mut hs = self.own_sel();
                    if let Some(k) = hs.iter().position(|&x| x == h) {
                        hs.remove(k);
                    } else {
                        hs.push(h);
                    }
                    self.set_sel(hs);
                } else {
                    self.set_sel(vec![h]);
                }
            } else if let Some(b) = self.bld_at(wp) {
                // Shift：自国の同じ種類の建物を選び足す・外す
                let shift = self.keys[VK_SHIFT as usize];
                let grp = |g: &Self, h: Handle| g.world.bld(h).filter(|bd| bd.nation as usize == p).map(|bd| Self::train_group(bd.kind)).unwrap_or(0);
                let g0 = self.sel_b.map(|h| grp(self, h)).unwrap_or(0);
                let double = self.last_bclick.0 > self.clock - 0.35 && self.last_bclick.1 == Some(b);
                self.last_bclick = (self.clock, Some(b));
                if double && grp(self, b) != 0 {
                    // ダブルクリック：画面内の自国の同じ種類の建物をすべて選ぶ
                    let g = grp(self, b);
                    let (w, hh) = (self.gfx.width as f32, self.gfx.height as f32);
                    let a = self.screen_to_world(0.0, self.top_h());
                    let z = self.screen_to_world(w, hh - BOT_H);
                    let mut bs: Vec<Handle> = vec![b];
                    for (i, bd) in self.world.blds.iter().enumerate() {
                        let c = bd.center();
                        if bd.alive && bd.done() && bd.nation as usize == p && Self::train_group(bd.kind) == g && c.x >= a.x && c.x <= z.x && c.y >= a.y && c.y <= z.y {
                            let h = self.world.bh(i);
                            if h != b {
                                bs.push(h);
                            }
                        }
                    }
                    self.clear_sel();
                    self.sel_b = Some(b);
                    self.sel_bs = bs;
                } else if shift && self.sel.is_empty() && g0 != 0 && grp(self, b) == g0 {
                    let mut bs = self.sel_blds();
                    if let Some(k) = bs.iter().position(|&x| x == b) {
                        bs.remove(k);
                    } else {
                        bs.push(b);
                    }
                    self.sel_b = bs.first().copied();
                    self.sel_bs = bs;
                } else {
                    self.clear_sel();
                    self.sel_b = Some(b);
                }
            } else {
                self.clear_sel();
            }
        } else {
            let a = self.screen_to_world(sx.min(x), sy.min(y));
            let b = self.screen_to_world(sx.max(x), sy.max(y));
            let hs: Vec<Handle> = (0..self.world.units.len())
                .filter(|&i| {
                    let u = &self.world.units[i];
                    u.alive && u.nation as usize == p && u.pos.x >= a.x && u.pos.x <= b.x && u.pos.y >= a.y && u.pos.y <= b.y
                })
                .map(|i| self.world.uh(i))
                .collect();
            if !hs.is_empty() {
                let mut hs = hs;
                if self.keys[VK_SHIFT as usize] {
                    for h in self.own_sel() {
                        if !hs.contains(&h) {
                            hs.push(h);
                        }
                    }
                }
                self.set_sel(hs);
            } else {
                // 兵・民がいなければ、範囲内の自国の兵営（無ければ都市）をまとめて選ぶ
                let in_box = |c: V2| c.x >= a.x && c.x <= b.x && c.y >= a.y && c.y <= b.y;
                let mut found: Vec<(u8, Handle)> = Vec::new();
                for (i, bd) in self.world.blds.iter().enumerate() {
                    if bd.alive && bd.done() && bd.nation as usize == p && in_box(bd.center()) {
                        let g = Self::train_group(bd.kind);
                        if g == 1 || g == 2 {
                            found.push((g, self.world.bh(i)));
                        }
                    }
                }
                let g = if found.iter().any(|x| x.0 == 2) { 2 } else { 1 };
                let mut bs: Vec<Handle> = found.iter().filter(|x| x.0 == g).map(|x| x.1).collect();
                if self.keys[VK_SHIFT as usize] && self.sel.is_empty() && self.sel_b.map_or(false, |h| self.world.bld(h).map_or(false, |bd| Self::train_group(bd.kind) == g)) {
                    for h in self.sel_blds() {
                        if !bs.contains(&h) {
                            bs.push(h);
                        }
                    }
                }
                if !bs.is_empty() {
                    self.clear_sel();
                    self.sel_b = bs.first().copied();
                    self.sel_bs = bs;
                } else if !self.keys[VK_SHIFT as usize] {
                    self.clear_sel();
                }
            }
        }
    }

    fn right_click_world(&mut self, p: V2) {
        let before: Vec<bool> = self.sel.iter().map(|&h| self.world.unit(h).map_or(false, |u| u.ferry.is_some())).collect();
        self.right_click_world_inner(p);
        // 海の向こうへの命令：港で輸送船に乗って渡ることを知らせる
        let mut plan = None;
        for (k, &h) in self.sel.iter().enumerate() {
            if let Some(f) = self.world.unit(h).filter(|u| u.nation as usize == self.world.player).and_then(|u| u.ferry) {
                if !before.get(k).copied().unwrap_or(false) || plan.is_none() {
                    plan = Some(f);
                }
            }
        }
        if let Some(f) = plan {
            let near = |w: &World, p: V2| -> String {
                w.blds.iter().filter(|b| b.alive && b.kind.city()).min_by(|a, b| a.center().dist(p).total_cmp(&b.center().dist(p))).filter(|b| b.center().dist(p) < 20.0).and_then(|b| b.city_name()).map(|s| crate::lang::tx(s).to_string()).unwrap_or_default()
            };
            let (a, b) = (near(&self.world, f.embark), near(&self.world, f.land));
            let s = if a.is_empty() || b.is_empty() {
                t!("海を渡る：港（青い印）で輸送船に乗り、上陸地（橙の印）に上陸してから目標へ進む", "Crossing the sea: troops board transports at the port (blue mark), land at the landing site (orange mark), then advance", "渡海：在港口（蓝色标记）登上运输船，在登陆点（橙色标记）登陆后向目标前进").to_string()
            } else {
                tf!("海を渡る：{}の港（青い印）で輸送船に乗り、{}のそば（橙の印）に上陸してから目標へ進む", "Crossing the sea: troops board transports at {} (blue mark), land near {} (orange mark), then advance", "渡海：在{}港（蓝色标记）登上运输船，在{}附近（橙色标记）登陆后向目标前进", a, b)
            };
            self.notice(&s);
        }
    }

    fn right_click_world_inner(&mut self, p: V2) {
        let pl = self.world.player;
        // ユニット未選択時：未完成建物の右クリックなら民を派遣、建物選択中なら集合地点設定
        if self.sel.is_empty() {
            if let Some(bh) = self.bld_at(p) {
                if let Some(b) = self.world.bld(bh) {
                    if b.nation as usize == pl && !b.done() {
                        self.dispatch_worker_to_build(bh);
                        return;
                    }
                }
            }
            for bh in self.sel_blds() {
                if let Some(b) = self.world.bld_mut(bh) {
                    if b.nation as usize == pl && b.kind != BKind::Farm {
                        b.rally = Some(p);
                        self.marker = Some((p, 0.6, false));
                    }
                }
            }
            return;
        }
        let hs = self.own_sel();
        if hs.is_empty() {
            return;
        }
        let workers: Vec<Handle> = hs.iter().copied().filter(|&h| self.world.unit(h).map_or(false, |u| u.kind == UnitKind::Worker)).collect();
        let soldiers: Vec<Handle> = hs.iter().copied().filter(|&h| self.world.unit(h).map_or(false, |u| u.kind.military())).collect();
        if !soldiers.is_empty() {
            if let Some(t) = &mut self.tut {
                t.moved = true;
            }
        }
        // 敵ユニット
        if let Some(t) = self.unit_at(p) {
            let tn = self.world.unit(t).unwrap().nation as usize;
            if tn != pl {
                if self.world.at_war(pl, tn) {
                    self.world.order_attack(&hs, Target::Unit(t));
                    self.marker = Some((p, 0.6, true));
                } else {
                    self.notice(&tf!("{} とは交戦していない（外交 F2 で宣戦できる）", "Not at war with {} (declare war in Diplomacy, F2)", "未与{}交战（可在外交 F2 中宣战）", crate::lang::tx(self.world.nations[tn].name)));
                    self.world.order_move(&hs, p, false);
                }
                return;
            }
        }
        if let Some(bh) = self.bld_at(p) {
            let b = self.world.bld(bh).unwrap();
            let (bn, kind, done, bc) = (b.nation as usize, b.kind, b.done(), b.center());
            if bn != pl {
                if self.world.at_war(pl, bn) {
                    self.world.order_attack(&hs, Target::Bld(bh));
                    self.marker = Some((p, 0.6, true));
                } else {
                    self.notice(&tf!("{} とは交戦していない（外交 F2 で宣戦できる）", "Not at war with {} (declare war in Diplomacy, F2)", "未与{}交战（可在外交 F2 中宣战）", crate::lang::tx(self.world.nations[bn].name)));
                }
                return;
            }
            if !done {
                if !workers.is_empty() {
                    self.world.order_build(&workers, bh);
                    self.marker = Some((bc, 0.6, false));
                    self.notice(&tf!("{}の建設へ民{}体を向かわせた", "Sent {1} workers to build the {0}", "派{1}名民去建造{0}", kind.name(), workers.len()));
                } else if self.world.bld_builders(bh) == 0 {
                    self.dispatch_worker_to_build(bh);
                }
                if !soldiers.is_empty() {
                    self.world.order_move(&soldiers, p, false);
                }
                return;
            }
            if kind == BKind::Farm && !workers.is_empty() {
                self.world.order_gather(&workers[..1], Src::Farm(bh));
                if workers.len() > 1 {
                    self.world.order_move(&workers[1..], p, false);
                }
                self.world.order_move(&soldiers, p, false);
                return;
            }
        }
        let (tx, ty) = p.tile();
        if !workers.is_empty() && Map::inb(tx, ty) {
            if self.world.map.terrain[Map::idx(tx, ty)] == Terrain::Forest {
                self.world.order_gather(&workers, Src::Forest(tx, ty));
                self.world.order_move(&soldiers, p, false);
                self.marker = Some((p, 0.6, false));
                return;
            }
            if let Some(m) = self.world.map.mines.iter().position(|m| m.x == tx && m.y == ty && m.amount > 0.0) {
                self.world.order_gather(&workers, Src::Mine(m));
                self.world.order_move(&soldiers, p, false);
                self.marker = Some((p, 0.6, false));
                return;
            }
        }
        self.world.order_move(&hs, p, false);
        self.marker = Some((p, 0.6, false));
    }

    fn placement_tile(&self) -> (i32, i32) {
        let p = self.screen_to_world(self.mouse.0, self.mouse.1);
        ((p.x - 0.5).round() as i32, (p.y - 0.5).round() as i32)
    }

    /// 建物を置く。queue なら建設の予約（選んだ民は今の建設を終えてから取りかかる）
    fn try_place(&mut self, k: BKind, queue: bool) {
        let (tx, ty) = self.placement_tile();
        let pl = self.world.player;
        match self.world.place_building(pl, k, tx, ty) {
            Ok(bh) => {
                let ws: Vec<Handle> = self.own_sel().into_iter().filter(|&h| self.world.unit(h).map_or(false, |u| u.kind == UnitKind::Worker)).collect();
                if queue {
                    self.world.queue_build(&ws, bh);
                } else {
                    self.world.order_build(&ws, bh);
                }
            }
            Err(e) => self.notice(e),
        }
    }

    fn title_click(&mut self, x: f32, y: f32) {
        let p = self.screen_to_world(x, y);
        let mut best: Option<(f32, usize)> = None;
        for (i, n) in self.world.nations.iter().enumerate() {
            if !n.alive || i >= NATIONS.len() {
                continue;
            }
            let d = n.home.dist(p);
            if d < 10.0 && best.map_or(true, |b| d < b.0) {
                best = Some((d, i));
            }
        }
        if let Some((_, i)) = best {
            if NATIONS[i].playable {
                self.title_sel = Some(i);
            } else {
                // 米国は選べない
                self.notice(t!("米国は選べない（妨害を受けると協商国側で参戦する）", "The United States cannot be chosen (it joins the Entente if provoked)", "不能选择美国（受到妨碍时会加入协约国参战）"));
            }
        }
    }

    fn button_at(&self, x: f32, y: f32) -> Option<Act> {
        for b in self.buttons.iter().rev() {
            if x >= b.x0 && x <= b.x1 && y >= b.y0 && y <= b.y1 {
                return Some(if b.enabled { b.act } else { Act::Noop });
            }
        }
        // タッチは指先がずれやすいので、すぐ近くのボタンも拾う
        if self.touch {
            let near = self
                .buttons
                .iter()
                .rev()
                .map(|b| ((b.x0 - x).max(x - b.x1).max(0.0).hypot((b.y0 - y).max(y - b.y1).max(0.0)), b))
                .filter(|(d, _)| *d <= 10.0)
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, b)) = near {
                return Some(if b.enabled { b.act } else { Act::Noop });
            }
        }
        None
    }

    fn do_act(&mut self, a: Act) {
        // 部隊への指示（移動・攻撃以外）でも時間を再開する
        if matches!(a, Act::Upgrade | Act::Convert | Act::Stop | Act::WorkersAuto | Act::ArmyDefend | Act::Demolish) {
            self.cmd_pause = false;
        }
        let pl = self.world.player;
        match a {
            Act::Noop => {}
            Act::Start => {
                // 押した場所に出るボタンの説明を、始めた直後に出さない
                self.tip_until = 0.0;
                if let Some(n) = self.title_sel.filter(|&n| n < NATIONS.len() && NATIONS[n].playable) {
                    self.world.player = n;
                    self.screen = Screen::Playing;
                    self.zoom = 22.0;
                    self.cam = self.world.nations[n].home;
                    self.reset_fog();
                    self.autosave_years = self.world.years;
                    let name = crate::lang::tx(self.world.nations[n].name);
                    self.world.push_msg(tf!("1914年6月28日。あなたは {} を率いる。", "June 28, 1914. You lead {}.", "1914年6月28日。你将率领{}。", name), [1.0, 0.9, 0.6]);
                    self.world.start_game();
                    self.world.push_msg(t!("F1でヘルプ、F2で外交画面を開く", "F1 opens Help, F2 opens Diplomacy", "F1 打开帮助，F2 打开外交画面").into(), [0.8, 0.8, 0.8]);
                }
            }
            Act::Tutorial => self.start_tutorial(),
            Act::TutNext => self.tut_next(),
            Act::TutQuit => self.end_tutorial(),
            Act::Scale(d) => {
                self.gfx.adjust_scale(d);
                if self.screen == Screen::Title {
                    self.fit_title_camera();
                }
                self.notice(&tf!("文字の大きさ ×{:.2}", "Text size ×{:.2}", "文字大小 ×{:.2}", self.gfx.ui));
            }
            Act::Fullscreen => self.gfx.toggle_fullscreen(),
            #[cfg(target_arch = "wasm32")]
            Act::RefreshCache => unsafe { crate::platform::js::js_refresh_cache() },
            Act::Lang => {
                let l = crate::lang::lang().next();
                crate::lang::set_lang(l);
                crate::platform::save_lang(l);
                self.gfx.reload_fonts();
            }
            Act::DismissBanner => self.world.dismiss_banner(),
            Act::ToggleAlerts => self.alerts_open = !self.alerts_open,
            Act::ClearMsgs => self.msg_cut = self.world.time,
            Act::Cancel => {
                if self.mode != Mode::Normal {
                    self.mode = Mode::Normal;
                    self.place_tap = None;
                } else if self.overlay_open() {
                    // 開いている一覧を閉じる（Esc と同じ）
                    self.show_diplo = false;
                    self.show_help = false;
                    self.show_chron = false;
                    self.show_market = false;
                    self.show_save = false;
                } else {
                    self.clear_sel();
                }
            }
            Act::SaveMenu => {
                self.show_save = !self.show_save;
                if self.show_save {
                    self.show_diplo = false;
                    self.show_help = false;
                    self.show_chron = false;
                    self.show_market = false;
                }
            }
            Act::SaveSlot(s) => self.save_slot(s, false),
            Act::LoadSlot(s) => {
                self.notice(t!("記録を読み込んでいる…", "Loading the save...", "正在读取存档…"));
                crate::platform::load_slot(s);
            }
            Act::ExportSlot(s) => crate::platform::export_slot(s),
            Act::ImportFile => crate::platform::import_file(),
            Act::Restart => {
                self.tut = None;
                self.seed = self.seed.wrapping_add(17);
                self.world = World::new(self.seed);
                self.rebuild_terrain();
                self.screen = Screen::Title;
                self.title_sel = None;
                self.sel.clear();
                self.sel_b = None;
                self.show_diplo = false;
                self.show_help = false;
                self.show_chron = false;
                self.show_market = false;
                self.show_save = false;
                self.paused = false;
                self.speed = 1;
                self.fit_title_camera();
            }
            Act::Train(k) => {
                // 選んだ建物すべてに1人ずつ予約する
                let bs = self.sel_blds();
                let mut ok = 0;
                let mut err = None;
                for bh in bs.iter().copied() {
                    match self.world.enqueue(bh, k) {
                        Ok(()) => ok += 1,
                        Err(e) => err = Some(e),
                    }
                }
                if bs.len() > 1 && ok > 0 {
                    let mut s = tf!("{}棟で{}を{}人訓練する", "Training {2} {1} in {0} buildings", "在{}座建筑中训练{}{}名", ok, k.name(self.world.era), ok);
                    if let Some(e) = err {
                        s.push_str(&tf!("（{}棟は{}）", " ({} could not: {})", "（{}座：{}）", bs.len() - ok, e));
                    }
                    self.notice(&s);
                } else if let Some(e) = err.filter(|_| ok == 0) {
                    self.notice(e);
                }
            }
            Act::Place(k) => {
                let cost = k.cost();
                let r = self.world.nations[pl].res;
                if r[0] < cost[0] || r[1] < cost[1] || r[2] < cost[2] {
                    self.notice(t!("資源が足りない", "Not enough resources", "资源不足"));
                } else {
                    self.mode = Mode::Place(k);
                    self.place_tap = None;
                }
            }
            Act::Upgrade => {
                let hs = self.own_sel();
                self.upgrade_handles(hs);
            }
            Act::Crew(k) => {
                let hs = self.own_sel();
                match self.world.crew_weapon(&hs, k) {
                    Ok(h) => {
                        let w = &self.world;
                        self.sel.retain(|&x| w.unit(x).is_some());
                        self.sel.push(h);
                    }
                    Err(e) => self.notice(e),
                }
            }
            Act::Convert => {
                let mut err = None;
                for h in self.own_sel() {
                    if let Err(e) = self.world.convert_unit(h) {
                        err = Some(e);
                    }
                }
                if let Some(e) = err {
                    self.notice(e);
                }
            }
            Act::Stop => {
                for h in self.own_sel() {
                    if let Some(u) = self.world.unit_mut(h) {
                        u.order = Order::Idle;
                        u.resume = None;
                        u.path.clear();
                        u.build_queue.clear();
                    }
                }
            }
            Act::AttackMove => {
                if !self.own_sel().is_empty() {
                    self.mode = Mode::AttackMove;
                }
            }
            Act::SelectAllWorkers => {
                let hs = self.all_workers();
                if hs.is_empty() {
                    self.notice(t!("民がいない", "No workers", "没有民"));
                } else {
                    self.notice(&tf!("民全体（{}体）を選択した", "Selected all workers ({})", "已选择全部民（{}名）", hs.len()));
                    self.set_sel(hs);
                }
            }
            Act::SelectAllMilitary => {
                let hs = self.all_military();
                if hs.is_empty() {
                    self.notice(t!("軍事ユニットがいない", "No soldiers", "没有军事单位"));
                } else {
                    self.notice(&tf!("全軍事ユニット（{}体）を選択した", "Selected all soldiers ({})", "已选择全部军事单位（{}个）", hs.len()));
                    self.set_sel(hs);
                }
            }
            Act::SelectKind(kind) => {
                let hs: Vec<Handle> = (0..self.world.units.len())
                    .filter(|&i| {
                        let u = &self.world.units[i];
                        u.alive && u.nation as usize == pl && u.kind == kind
                    })
                    .map(|i| self.world.uh(i))
                    .collect();
                if hs.is_empty() {
                    self.notice(&tf!("{}はいない", "No {} units", "没有{}", kind.name(self.world.era)));
                } else {
                    self.notice(&tf!("{}全体（{}体）を選択した", "Selected all {} units ({})", "已选择全部{}（{}个）", kind.name(self.world.era), hs.len()));
                    self.set_sel(hs);
                }
            }
            Act::FilterKind(kind) => {
                let hs: Vec<Handle> = self
                    .own_sel()
                    .into_iter()
                    .filter(|&h| self.world.unit(h).map_or(false, |u| u.kind == kind))
                    .collect();
                if !hs.is_empty() {
                    self.set_sel(hs);
                }
            }
            Act::WorkersAuto => {
                let hs = self.own_sel();
                self.order_workers_balanced(&hs, false);
            }
            Act::AllWorkersAuto => {
                let hs = self.all_workers();
                self.order_workers_balanced(&hs, true);
            }
            Act::AllWorkersWood => {
                let hs = self.all_workers();
                self.order_workers_resource(&hs, true);
            }
            Act::AllWorkersMine => {
                let hs = self.all_workers();
                self.order_workers_resource(&hs, false);
            }
            Act::AllWorkersUpgrade => {
                let hs = self.all_workers();
                if hs.is_empty() {
                    self.notice(t!("民がいない", "No workers", "没有民"));
                } else {
                    self.upgrade_handles(hs);
                }
            }
            Act::ArmyDefend => {
                let hs: Vec<Handle> = self.own_sel().into_iter().filter(|&h| self.world.unit(h).map_or(false, |u| u.kind.military())).collect();
                self.order_army_defend(&hs);
            }
            Act::AllArmyAttack => {
                self.order_all_army_attack();
            }
            Act::AllArmyDefend => {
                let hs = self.all_military();
                self.order_army_defend(&hs);
            }
            Act::AllArmyUpgrade => {
                let hs = self.all_military();
                if hs.is_empty() {
                    self.notice(t!("軍事ユニットがいない", "No soldiers", "没有军事单位"));
                } else {
                    self.upgrade_handles(hs);
                }
            }
            Act::Speed(s) => {
                if s == 0 {
                    self.paused = !self.paused;
                } else {
                    self.speed = s;
                    self.paused = false;
                }
            }
            Act::Diplo => {
                self.show_diplo = !self.show_diplo;
                self.show_help = false;
                self.show_chron = false;
                self.show_market = false;
                self.show_save = false;
            }
            Act::Help => {
                self.show_help = !self.show_help;
                self.show_diplo = false;
                self.show_chron = false;
                self.show_market = false;
                self.show_save = false;
            }
            Act::Chron => {
                self.show_chron = !self.show_chron;
                self.chron_scroll = 0;
                self.show_diplo = false;
                self.show_help = false;
                self.show_market = false;
                self.show_save = false;
            }
            Act::Market => {
                self.show_market = !self.show_market;
                self.show_diplo = false;
                self.show_help = false;
                self.show_chron = false;
            }
            Act::Unrestricted => {
                if pl == GER && self.world.nations[GER].alive {
                    self.world.sea.unrestricted = !self.world.sea.unrestricted;
                    let t = if self.world.sea.unrestricted {
                        t!("無制限潜水艦作戦を始めた。中立国（米国）の船も沈める。米国の参戦を招くおそれがある", "Unrestricted submarine warfare begins: neutral (American) ships will be sunk too. This risks bringing America into the war", "开始无限制潜艇战，中立国（美国）的船只也将被击沉。可能招致美国参战")
                    } else {
                        t!("無制限潜水艦作戦をやめた", "Unrestricted submarine warfare called off", "停止了无限制潜艇战")
                    };
                    self.world.push_msg(t.to_string(), [1.0, 0.75, 0.4]);
                    self.notice(t);
                }
            }
            Act::ChronPage(d) => {
                self.chron_scroll = (self.chron_scroll as i32 + d).max(0) as usize;
            }
            Act::CloseOverlay => {
                self.show_diplo = false;
                self.show_help = false;
                self.show_chron = false;
                self.show_market = false;
                self.show_save = false;
            }
            Act::War(o) => {
                if self.world.truce[pl][o] > self.world.years {
                    let r = (self.world.truce[pl][o] - self.world.years).ceil() as i32;
                    self.notice(&tf!("休戦期間中（残り{}年）", "Truce in effect ({} years left)", "休战期内（剩余{}年）", r));
                } else {
                    self.world.declare_war(pl, o, false);
                }
            }
            Act::PeaceAnswer(accept) => self.world.answer_peace(accept),
            Act::Home => {
                if let Some(c) = self.world.nations[pl].capital.and_then(|h| self.world.bld(h)) {
                    self.cam = c.center();
                }
            }
            Act::Demolish => {
                if let Some(bh) = self.sel_b {
                    if let Some(b) = self.world.bld(bh) {
                        if b.nation as usize == pl && (!b.kind.city() || !b.done()) {
                            let i = bh.idx as usize;
                            self.world.remove_building(i);
                            self.sel_b = None;
                        }
                    }
                }
            }
            Act::ResumeBuild => {
                if let Some(bh) = self.sel_b {
                    self.dispatch_worker_to_build(bh);
                }
            }
            Act::IdleWorker => {
                let idle: Vec<usize> = (0..self.world.units.len())
                    .filter(|&i| {
                        let u = &self.world.units[i];
                        u.alive && u.nation as usize == pl && u.kind == UnitKind::Worker && u.order == Order::Idle
                    })
                    .collect();
                if idle.is_empty() {
                    self.notice(t!("待機中の民はいない", "No idle workers", "没有闲置的民"));
                } else {
                    self.idle_cycle = (self.idle_cycle + 1) % idle.len();
                    let i = idle[self.idle_cycle];
                    let h = self.world.uh(i);
                    self.cam = self.world.units[i].pos;
                    self.set_sel(vec![h]);
                }
            }
        }
    }

    // ------------------------------------------------------------ 描画
    fn render(&mut self, dt: f32) {
        self.render_nopresent(dt);
        self.gfx.present();
    }

    fn render_nopresent(&mut self, dt: f32) {
        self.update_minimap(dt);
        self.gfx.begin([0.07, 0.06, 0.05, 1.0]);
        self.render_world();
        if self.gfx.d2d_begin() {
            self.art.ensure(&self.gfx);
            self.buttons.clear();
            self.render_world_labels();
            if self.screen == Screen::Playing {
                self.render_build_queue();
            }
            // スマートフォン向け画面では、縦に収まらない一覧は指でなぞってスクロールする
            let key = self.overlay_key();
            if key != self.ov_key {
                self.ov_key = key;
                self.ov_scroll = 0.0;
            }
            self.ov_max = 0.0;
            if self.screen == Screen::Title {
                self.render_title();
                if self.show_save {
                    self.scrolled(Self::render_save);
                }
            } else {
                self.render_hud();
                if self.show_diplo {
                    self.scrolled(Self::render_diplo);
                }
                if self.show_help {
                    self.scrolled(Self::render_help);
                }
                if self.show_chron {
                    self.scrolled(Self::render_chron);
                }
                if self.show_market {
                    self.scrolled(Self::render_status);
                }
                if self.show_save {
                    self.scrolled(Self::render_save);
                }
                if self.peace_pending() {
                    self.render_peace_ask();
                }
                if self.world.game_over.is_some() {
                    self.render_gameover();
                }
                self.render_tutorial();
                self.render_tooltip();
            }
            self.gfx.d2d_end();
        }
    }

    fn render_world(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let z = self.zoom;
        let sx = 2.0 * z / w;
        let sy = -2.0 * z / h;
        self.gfx.set_xform(sx, sy, -self.cam.x * sx, -self.cam.y * sy);
        let place_tile = self.placement_tile();
        let fog_on = self.fog_active();
        let pl = self.world.player;
        let fog = &self.fog;
        let fog_at = move |p: V2| -> u8 {
            if !fog_on {
                return 2;
            }
            let (x, y) = p.tile();
            if Map::inb(x, y) {
                fog[Map::idx(x, y)]
            } else {
                0
            }
        };
        let g = &mut self.gfx;
        let mw = MAP_W as f32;
        let mh = MAP_H as f32;
        crate::art::draw_map_frame(g, mw, mh);
        g.draw_textured(&self.terrain_tex, 0.0, 0.0, mw, mh, [1.0; 4], true);
        g.draw_textured(&self.overlay_tex, 0.0, 0.0, mw, mh, [1.0; 4], true);
        let wd = &self.world;
        let view0 = v2(self.cam.x - w / 2.0 / z - 2.0, self.cam.y - h / 2.0 / z - 2.0);
        let view1 = v2(self.cam.x + w / 2.0 / z + 2.0, self.cam.y + h / 2.0 / z + 2.0);
        let inview = |p: V2| p.x >= view0.x && p.x <= view1.x && p.y >= view0.y && p.y <= view1.y;
        let px = 1.0 / z; // 1ピクセル
        // 森林の樹木・山岳の立体表現
        crate::art::draw_world_terrain_props(
            g,
            &wd.map,
            view0.x.floor() as i32,
            view0.y.floor() as i32,
            view1.x.ceil() as i32,
            view1.y.ceil() as i32,
            z,
            &fog_at,
        );
        // 銅山
        for m in wd.map.mines.iter() {
            if m.amount <= 0.0 {
                continue;
            }
            let c = v2(m.x as f32 + 0.5, m.y as f32 + 0.5);
            if !inview(c) || fog_at(c) == 0 {
                continue;
            }
            crate::art::draw_world_mine(g, c, m.amount, px);
        }
        // 建物
        for (bi, b) in wd.blds.iter().enumerate() {
            if !b.alive {
                continue;
            }
            let c = b.center();
            if !inview(c) || fog_at(c) == 0 {
                continue;
            }
            let nc = wd.nations[b.nation as usize].color;
            let (x0, y0, x1, y1) = (b.tx as f32, b.ty as f32, b.tx as f32 + 2.0, b.ty as f32 + 2.0);
            if wd.descent.is_some() && b.nation as usize == ANGEL && b.kind.city() {
                let dr = if b.kind == BKind::Capital { 1.58 } else { 1.32 };
                g.circle(c.x, c.y + 0.08, dr, 24, [1.0, 0.92, 0.45, 0.25]);
                g.ring(c.x, c.y + 0.08, dr, (2.2 * px).max(0.06), 24, [1.0, 0.90, 0.42, 0.90]);
            }
            crate::art::draw_world_building(g, b, nc, px);
            let sel = self.sel_bs.contains(&wd.bh(bi)) || self.sel_b == Some(wd.bh(bi));
            if sel {
                g.rect_outline(x0 - 0.32, y0 - 0.32, x1 + 0.32, y1 + 0.32, (2.2 * px).max(0.05), [1.0, 0.92, 0.55, 0.95]);
                if let Some(r) = b.rally {
                    g.line((c.x, c.y), (r.x, r.y), 1.5 * px, [1.0, 0.92, 0.55, 0.65]);
                    g.rect(r.x - 0.05, r.y - 0.65, r.x + 0.05, r.y, [0.92, 0.85, 0.65, 1.0]);
                    g.tri((r.x + 0.05, r.y - 0.65), (r.x + 0.48, r.y - 0.48), (r.x + 0.05, r.y - 0.32), c4(nc, 1.0));
                }
            }
        }
        // ユニット
        let era = wd.era;
        for u in wd.units.iter() {
            if !u.alive || !inview(u.pos) || (u.nation as usize != pl && fog_at(u.pos) != 2) {
                continue;
            }
            let nc = wd.nations[u.nation as usize].color;
            let ue = era;
            if u.divine {
                g.circle(u.pos.x, u.pos.y + 0.12, 0.56, 16, [1.0, 0.94, 0.55, 0.28]);
                g.ring(u.pos.x, u.pos.y + 0.12, 0.56, (1.8 * px).max(0.045), 16, [1.0, 0.92, 0.48, 0.92]);
            }
            crate::art::draw_world_unit(g, u, nc, ue, &RES_COLORS, px);
        }
        // 矢
        for s in wd.shots.iter() {
            let t = 1.0 - s.t / 0.3;
            let p = s.a + (s.b - s.a) * t.clamp(0.0, 1.0);
            if fog_at(p) != 2 {
                continue;
            }
            let d = (s.b - s.a).norm();
            g.line((p.x - d.x * 0.34, p.y - d.y * 0.34), (p.x, p.y), (2.2 * px).max(0.045), [0.85, 0.72, 0.45, 0.95]);
            g.circle(p.x, p.y, (1.8 * px).max(0.04), 5, [1.0, 0.95, 0.65, 1.0]);
        }
        if fog_on {
            g.draw_textured(&self.fog_tex, 0.0, 0.0, mw, mh, [1.0; 4], true);
        }
        // 海を渡る部隊の道筋：乗船する港 → 海 → 上陸地（選んでいる自国の部隊）
        let mut routes: Vec<(V2, V2, V2, V2)> = Vec::new();
        for &h in self.sel.iter() {
            if let Some(u) = wd.unit(h) {
                if let Some(f) = u.ferry.filter(|_| u.nation as usize == wd.player) {
                    g.line((u.pos.x, u.pos.y), (f.embark.x, f.embark.y), 1.6 * px, [0.85, 0.95, 1.0, 0.45]);
                    if !routes.iter().any(|r| r.0.dist(f.embark) < 1.0 && r.2.dist(f.land) < 1.0) {
                        routes.push((f.embark, f.sea, f.land, f.goal));
                    }
                }
            }
        }
        for (emb, sea, land, goal) in routes {
            let c = [0.55, 0.85, 1.0, 0.9];
            g.line((sea.x, sea.y), (land.x, land.y), 2.6 * px, c);
            g.line((emb.x, emb.y), (sea.x, sea.y), 2.6 * px, c);
            g.line((land.x, land.y), (goal.x, goal.y), 1.6 * px, [1.0, 0.45, 0.35, 0.7]);
            // 乗船地（青）と上陸地（橙）の印
            let (rr, rc) = ((9.0 * px).max(0.9), (4.0 * px).max(0.35));
            g.ring(emb.x, emb.y, rr, 2.4 * px, 16, [0.55, 0.85, 1.0, 1.0]);
            g.circle(emb.x, emb.y, rc, 8, [0.55, 0.85, 1.0, 1.0]);
            g.ring(land.x, land.y, rr, 2.4 * px, 16, [1.0, 0.75, 0.3, 1.0]);
            g.circle(land.x, land.y, rc, 8, [1.0, 0.75, 0.3, 1.0]);
        }
        // 移動目標の印
        if let Some((p, t, atk)) = self.marker {
            let r = 0.2 + t * 0.6;
            let c = if atk { [1.0, 0.3, 0.2, t / 0.6] } else { [0.4, 1.0, 0.4, t / 0.6] };
            g.ring(p.x, p.y, r, 2.0 * px, 16, c);
        }
        // 建設予定地
        if let Mode::Place(k) = self.mode {
            let (tx, ty) = place_tile;
            let ok = wd.can_place(wd.player, k, tx, ty).is_ok();
            let c = if ok { [0.3, 1.0, 0.3, 0.45] } else { [1.0, 0.25, 0.2, 0.45] };
            g.rect(tx as f32, ty as f32, tx as f32 + 2.0, ty as f32 + 2.0, c);
            if k.city() {
                g.ring(tx as f32 + 1.0, ty as f32 + 1.0, k.radius() as f32, 2.0 * px, 48, [1.0, 1.0, 1.0, 0.5]);
            }
        }
        g.flush();
        // 画面座標での選択矩形
        if let Some((sx, sy)) = self.ldown {
            let (mx, my) = self.mouse;
            if (sx - mx).abs() > 4.0 || (sy - my).abs() > 4.0 {
                self.gfx.set_screen_xform();
                let g = &mut self.gfx;
                g.rect(sx.min(mx), sy.min(my), sx.max(mx), sy.max(my), [0.4, 1.0, 0.4, 0.12]);
                g.rect_outline(sx.min(mx), sy.min(my), sx.max(mx), sy.max(my), 1.5, [0.4, 1.0, 0.4, 0.9]);
                g.flush();
            }
        }
    }

    /// 選んだ民の建設の順番（建設中＝1、予約＝2, 3, …）を建設地に示す
    fn render_build_queue(&mut self) {
        let pl = self.world.player;
        let mut order: Vec<(Handle, usize)> = Vec::new();
        for h in self.sel.iter() {
            let Some(u) = self.world.unit(*h) else { continue };
            if u.nation as usize != pl || u.kind != UnitKind::Worker || u.build_queue.is_empty() {
                continue;
            }
            let cur = if let Order::Build(b) = u.order { Some(b) } else { None };
            for (k, &b) in cur.iter().chain(u.build_queue.iter()).enumerate() {
                match order.iter_mut().find(|x| x.0 == b) {
                    Some(x) => x.1 = x.1.min(k + 1),
                    None => order.push((b, k + 1)),
                }
            }
        }
        for (b, k) in order {
            let Some(bd) = self.world.bld(b) else { continue };
            if bd.done() {
                continue;
            }
            let (sx, sy) = self.world_to_screen(bd.center());
            let r = 11.0;
            self.gfx.fill_rect(sx - r, sy - r - 18.0, sx + r, sy + r - 18.0, [0.12, 0.10, 0.06, 0.85]);
            self.gfx.fill_rect(sx - r, sy + r - 20.0, sx + r, sy + r - 18.0, [1.0, 0.8, 0.35, 1.0]);
            let s = format!("{}", k);
            let tw = self.gfx.measure(&s, Font::Normal);
            self.gfx.text(&s, sx - tw / 2.0, sy - r - 18.0, tw + 4.0, 2.0 * r, Font::Normal, [1.0, 0.88, 0.5, 1.0]);
        }
    }

    fn render_world_labels(&mut self) {
        let z = self.zoom;
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let fog_on = self.fog_active();
        let pl = self.world.player;

        enum WorldItem {
            Mountain { sx: f32, sy: f32, r: f32, flip_x: bool, fogged: bool },
            Forest { sx: f32, sy: f32, r: f32, flip_x: bool, fogged: bool },
            Mine { sx: f32, sy: f32, r: f32, fogged: bool },
            Building {
                sx: f32,
                sy: f32,
                r: f32,
                kind: BKind,
                nc: [f32; 3],
                done: bool,
                progress: f32,
                hp_frac: Option<f32>,
                fogged: bool,
                divine: bool,
            },
            Unit {
                sx: f32,
                sy: f32,
                kind: UnitKind,
                ue: Era,
                nation: usize,
                pw: f32,
                nc: [f32; 3],
                carry: Option<usize>,
                flip_x: bool,
                hp_frac: Option<f32>,
                divine: bool,
            },
        }

        if z >= 2.2 {
            let mut items: Vec<(f32, WorldItem)> = Vec::new();
            let view0 = v2(self.cam.x - w / 2.0 / z - 2.5, self.cam.y - h / 2.0 / z - 2.5);
            let view1 = v2(self.cam.x + w / 2.0 / z + 2.5, self.cam.y + h / 2.0 / z + 2.5);
            let x0 = (view0.x.floor() as i32).max(0);
            let y0 = (view0.y.floor() as i32).max(0);
            let x1 = (view1.x.ceil() as i32).min(MAP_W as i32 - 1);
            let y1 = (view1.y.ceil() as i32).min(MAP_H as i32 - 1);

            // 山岳・森林のAI生成スプライト
            let thin = z < 8.5;
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    let idx = Map::idx(tx, ty);
                    let t = self.world.map.terrain[idx];
                    if !matches!(t, Terrain::Mountain | Terrain::Forest) {
                        continue;
                    }
                    if z < 5.5 {
                        if ((tx * 3 + ty * 5) & 3) != 0 {
                            continue;
                        }
                    } else if thin && ((tx + ty) & 1) != 0 {
                        continue;
                    }
                    let f = if fog_on { self.fog[idx] } else { 2 };
                    if f == 0 {
                        continue;
                    }
                    let fogged = f == 1;
                    let h1 = crate::rng::hash2(tx, ty, 31);
                    let h2 = crate::rng::hash2(tx, ty, 47);
                    let flip_x = h1 > 0.5;
                    let ox = (h1 - 0.5) * 0.28;
                    let oy = (h2 - 0.5) * 0.24;
                    let wp = v2(tx as f32 + 0.5 + ox, ty as f32 + 0.5 + oy);
                    let (sx, sy) = self.world_to_screen(wp);
                    if sx < -64.0 || sy < -64.0 || sx > w + 64.0 || sy > h + 64.0 {
                        continue;
                    }
                    match t {
                        Terrain::Mountain => {
                            let mult = if z < 5.5 { 1.65 } else if thin { 1.25 } else { 0.92 + 0.22 * h2 };
                            let r = (z * mult).clamp(5.5, 44.0);
                            items.push((sy + r * 0.25, WorldItem::Mountain { sx, sy, r, flip_x, fogged }));
                        }
                        Terrain::Forest => {
                            let mult = if z < 5.5 { 1.45 } else if thin { 1.12 } else { 0.80 + 0.20 * h2 };
                            let r = (z * mult).clamp(5.0, 38.0);
                            items.push((sy + r * 0.22, WorldItem::Forest { sx, sy, r, flip_x, fogged }));
                        }
                        _ => {}
                    }
                }
            }

            // 銅山のAI生成スプライト
            for m in self.world.map.mines.iter() {
                if m.amount <= 0.0 {
                    continue;
                }
                let wp = v2(m.x as f32 + 0.5, m.y as f32 + 0.5);
                if !self.explored(wp) {
                    continue;
                }
                let (sx, sy) = self.world_to_screen(wp);
                if sx < -64.0 || sy < -64.0 || sx > w + 64.0 || sy > h + 64.0 {
                    continue;
                }
                let fogged = fog_on && !self.seen(wp);
                let s = 0.78 + 0.24 * (m.amount / 2500.0).clamp(0.2, 1.0);
                let r = (z * s).clamp(6.5, 38.0);
                items.push((sy + r * 0.28, WorldItem::Mine { sx, sy, r, fogged }));
            }

            // 建物（都・邑・兵営・田）のAI生成スプライト
            for (bi, b) in self.world.blds.iter().enumerate() {
                if !b.alive {
                    continue;
                }
                let c = b.center();
                if !self.explored(c) {
                    continue;
                }
                let (sx, sy) = self.world_to_screen(c);
                if sx < -96.0 || sy < -96.0 || sx > w + 96.0 || sy > h + 96.0 {
                    continue;
                }
                let fogged = fog_on && !self.seen(c);
                let nc = self.world.nations[b.nation as usize].color;
                let mult = match b.kind {
                    BKind::Capital => if z < 5.5 { 2.25 } else { 1.62 },
                    BKind::Town => if z < 5.5 { 1.75 } else { 1.38 },
                    BKind::Barracks => 1.25,
                    BKind::Farm => 1.16,
                    BKind::Fort => 1.3,
                    BKind::Dock => 1.3,
                };
                let r = (z * mult).clamp(9.0, 68.0);
                let sel = self.sel_bs.contains(&self.world.bh(bi)) || self.sel_b == Some(self.world.bh(bi));
                let hp_frac = if b.done() && (b.hp < b.kind.max_hp() - 0.5 || sel) {
                    Some((b.hp / b.kind.max_hp()).clamp(0.0, 1.0))
                } else {
                    None
                };
                let sort_y = if b.kind == BKind::Farm { sy - r * 0.2 } else { sy + r * 0.45 };
                let divine = self.world.descent.is_some() && b.nation as usize == ANGEL && b.kind.city();
                items.push((
                    sort_y,
                    WorldItem::Building {
                        sx,
                        sy,
                        r,
                        kind: b.kind,
                        nc,
                        done: b.done(),
                        progress: b.progress,
                        hp_frac,
                        fogged,
                        divine,
                    },
                ));
            }

            // ユニットの高精細AI生成スプライト（マップや建物に対して適切なサイズに縮小）
            if z >= 6.5 && self.screen == Screen::Playing {
                for u in self.world.units.iter() {
                    if !u.alive || (u.nation as usize != pl && !self.seen(u.pos)) {
                        continue;
                    }
                    let (sx, sy) = self.world_to_screen(u.pos);
                    if sx < -48.0 || sy < -48.0 || sx > w + 48.0 || sy > h + 48.0 {
                        continue;
                    }
                    let nc = self.world.nations[u.nation as usize].color;
                    let carry = if u.carry > 0.5 { Some(u.carry_kind) } else { None };
                    let flip_x = u.dir.x < -0.05;
                    let mhp = u.max_hp();
                    let hp_frac = if u.hp < mhp - 0.5 || u.selected {
                        Some((u.hp / mhp).clamp(0.0, 1.0))
                    } else {
                        None
                    };
                    items.push((
                        sy + 2.0,
                        WorldItem::Unit {
                            sx,
                            sy,
                            kind: u.kind,
                            ue: self.world.era,
                            nation: u.nation as usize,
                            pw: u.power,
                            nc,
                            carry,
                            flip_x,
                            hp_frac,
                            divine: u.divine,
                        },
                    ));
                }
            }

            items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            for (_, item) in items {
                match item {
                    WorldItem::Mountain { sx, sy, r, flip_x, fogged } => {
                        if let Some(bmp) = self.art.mountain_sprite(flip_x, fogged) {
                            self.gfx.draw_bitmap(bmp, sx - r, sy - r * 1.18, sx + r, sy + r * 0.82);
                        }
                    }
                    WorldItem::Forest { sx, sy, r, flip_x, fogged } => {
                        if let Some(bmp) = self.art.forest_sprite(flip_x, fogged) {
                            self.gfx.draw_bitmap(bmp, sx - r, sy - r * 1.12, sx + r, sy + r * 0.88);
                        }
                    }
                    WorldItem::Mine { sx, sy, r, fogged } => {
                        if let Some(bmp) = self.art.mine_sprite(fogged) {
                            self.gfx.draw_bitmap(bmp, sx - r, sy - r * 1.05, sx + r, sy + r * 0.95);
                        }
                    }
                    WorldItem::Building { sx, sy, r, kind, nc, done, progress, hp_frac, fogged, divine } => {
                        let alpha = if done { 1.0 } else { 0.65 };
                        let b_bmp = if divine {
                            self.art.divine_bld_sprite(kind, fogged)
                        } else {
                            self.art.bld_sprite(kind, fogged)
                        };
                        if let Some(bmp) = b_bmp {
                            self.gfx.draw_bitmap_alpha(bmp, sx - r, sy - r * 1.08, sx + r, sy + r * 0.92, alpha);
                        }
                        // 建物右下の国色紋章バッジ（どの国の城邑・兵営か一目で判別可能に）
                        if kind != BKind::Farm && z >= 7.0 {
                            let br = (r * 0.16).clamp(3.2, 6.5);
                            let bx = sx + r * 0.56;
                            let by = sy + r * 0.48;
                            let k = if fogged { 0.55 } else { 1.0 };
                            self.gfx.fill_circle(bx, by, br + 1.2, [0.06, 0.05, 0.04, 0.92]);
                            if divine {
                                self.gfx.fill_circle(bx, by, br, [1.0 * k, 0.90 * k, 0.42 * k, 1.0]);
                                self.gfx.stroke_circle(bx, by, br + 0.6, 1.3, [0.95 * k, 0.98 * k, 1.0 * k, 0.98]);
                            } else {
                                self.gfx.fill_circle(bx, by, br, [nc[0] * k, nc[1] * k, nc[2] * k, 1.0]);
                                self.gfx.stroke_circle(bx, by, br + 0.5, 1.1, [0.95 * k, 0.82 * k, 0.38 * k, 0.95]);
                            }
                        }
                        if !done {
                            let bw = (r * 1.4).max(16.0);
                            let by = sy + r * 0.72;
                            self.gfx.fill_round(sx - bw * 0.5, by, sx + bw * 0.5, by + 3.5, 1.5, [0.08, 0.06, 0.04, 0.68]);
                            if progress > 0.01 {
                                self.gfx.fill_round(sx - bw * 0.5, by, sx - bw * 0.5 + bw * progress, by + 3.5, 1.5, [0.35, 0.78, 1.0, 0.95]);
                            }
                            self.gfx.stroke_round(sx - bw * 0.5, by, sx + bw * 0.5, by + 3.5, 1.5, 0.8, [0.78, 0.64, 0.34, 0.75]);
                        } else if let Some(f) = hp_frac {
                            let bw = (r * 1.4).max(16.0);
                            let by = sy - r * 1.02;
                            self.gfx.fill_round(sx - bw * 0.5, by, sx + bw * 0.5, by + 3.5, 1.5, [0.08, 0.06, 0.04, 0.75]);
                            self.gfx.fill_round(sx - bw * 0.5, by, sx - bw * 0.5 + bw * f, by + 3.5, 1.5, [1.0 - f, 0.22 + 0.75 * f, 0.15, 0.95]);
                        }
                    }
                    WorldItem::Unit { sx, sy, kind, ue, nation, pw, nc, carry, flip_x, hp_frac, divine } => {
                        let big = matches!(kind, UnitKind::Cavalry | UnitKind::Artillery | UnitKind::Tank | UnitKind::Destroyer | UnitKind::Transport | UnitKind::Submarine);
                        let scale = if divine {
                            if big { 0.82 } else { 0.70 }
                        } else if big {
                            0.76
                        } else {
                            0.62
                        };
                        let max_r = if divine { 24.0 } else { 22.0 };
                        let r = (z * scale * (1.0 + 0.04 * (pw - 1.0).min(4.0))).clamp(7.5, max_r);
                        let u_bmp = if divine {
                            self.art.divine_unit_sprite(kind, flip_x)
                        } else {
                            self.art.national_unit_sprite(kind, ue, nation, self.world.year_f(), flip_x)
                        };
                        if let Some(bmp) = u_bmp {
                            self.gfx.draw_bitmap(bmp, sx - r, sy - r * 1.35, sx + r, sy + r * 0.65);
                        }
                        // 足元右下の国色小紋章（上帝の軍は神々しい白金紋章）
                        let pr = (r * 0.19).clamp(2.2, 3.8);
                        let px0 = sx + r * 0.42;
                        let py0 = sy + r * 0.32;
                        self.gfx.fill_circle(px0, py0, pr + 0.9, [0.08, 0.06, 0.04, 0.92]);
                        if divine {
                            self.gfx.fill_circle(px0, py0, pr, [1.0, 0.92, 0.48, 1.0]);
                            self.gfx.stroke_circle(px0, py0, pr + 0.5, 0.9, [0.92, 0.98, 1.0, 0.95]);
                        } else {
                            self.gfx.fill_circle(px0, py0, pr, c4(nc, 1.0));
                            self.gfx.stroke_circle(px0, py0, pr + 0.4, 0.8, [0.95, 0.82, 0.42, 0.9]);
                        }
                        // 資源運搬中はミニ資源アイコンを左上に表示
                        if let Some(ck) = carry {
                            if z >= 11.0 {
                                let cr = (r * 0.38).clamp(3.8, 6.2);
                                let cx = sx - r * 0.55;
                                let cy = sy - r * 0.65;
                                self.gfx.fill_circle(cx, cy, cr + 0.8, [0.08, 0.06, 0.04, 0.85]);
                                if let Some(rbmp) = &self.art.res_icons[ck] {
                                    self.gfx.draw_bitmap(rbmp, cx - cr, cy - cr, cx + cr, cy + cr);
                                }
                            }
                        }
                        // 強化済みユニットは金の星（ランクピップ）を右下に表示
                        if pw > 1.01 && z >= 12.0 {
                            let stars = ((pw - 1.0) / 0.5).round().clamp(1.0, 6.0) as i32;
                            for s in 0..stars {
                                let px = sx + r * 0.44 - s as f32 * 3.8;
                                let py = sy + r * 0.48;
                                self.gfx.fill_circle(px, py, 2.3, [0.12, 0.08, 0.04, 0.95]);
                                self.gfx.fill_circle(px, py, 1.5, [1.0, 0.88, 0.32, 1.0]);
                            }
                        }
                        if let Some(f) = hp_frac {
                            let bw = (r * 1.4).max(11.0);
                            let by = sy - r * 1.40;
                            self.gfx.fill_rect(sx - bw * 0.5, by, sx + bw * 0.5, by + 2.8, [0.0, 0.0, 0.0, 0.8]);
                            self.gfx.fill_rect(sx - bw * 0.5, by, sx - bw * 0.5 + bw * f, by + 2.8, [1.0 - f, 0.25 + 0.7 * f, 0.15, 1.0]);
                        }
                    }
                }
            }
        }
        // 城邑名（Age of Empires 風の銘板バナー付き）
        let mut labels = Vec::new();
        for b in self.world.blds.iter() {
            if !b.alive || !b.kind.city() || !self.explored(b.center()) {
                continue;
            }
            let (sx, sy) = self.world_to_screen(b.center());
            if sx < -100.0 || sy < -50.0 || sx > w + 100.0 || sy > h + 50.0 {
                continue;
            }
            let n = &self.world.nations[b.nation as usize];
            let cap = b.kind == BKind::Capital;
            if z < 9.0 && !cap {
                continue;
            }
            let divine = self.world.descent.is_some() && b.nation as usize == ANGEL;
            let city = b.city_name().map(crate::lang::tx).unwrap_or("");
            let name = if divine {
                tf!("天使・{}", "Angels · {}", "天使・{}", city)
            } else if cap && z < 9.0 {
                crate::lang::tx(n.name).to_string()
            } else if cap {
                tf!("{}・{}", "{} · {}", "{}・{}", crate::lang::tx(n.name), city)
            } else {
                city.to_string()
            };
            let col = if divine { [1.0, 0.88, 0.38] } else { n.color };
            // 重なったときに残す順：タイトルで選んだ国・自国、首都、人口の多い国
            let ni = b.nation as usize;
            let mut pri = if cap { 100.0 } else { 0.0 } + if ni < NATIONS.len() { NATIONS[ni].pop as f32 } else { 50.0 };
            if (self.screen == Screen::Title && self.title_sel == Some(ni)) || (self.screen != Screen::Title && ni == self.world.player) {
                pri += 1000.0;
            }
            labels.push((pri, sx, sy, name, cap, col, divine));
        }
        // 小さく見ているときは国名が重なるので、重なる名札は優先度の低いほうを出さない
        labels.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut placed: Vec<(f32, f32, f32, f32)> = Vec::new();
        for (_, sx, sy, name, cap, c, divine) in labels {
            let f = if cap && z >= 9.0 { Font::BigCenter } else { Font::NormalCenter };
            let off = if z >= 9.0 { 1.58 * z + 14.0 } else { 1.25 * z + 6.0 };
            let ty = sy - off - 14.0;
            let lw = self.gfx.measure(&name, f) + 10.0;
            let r = (sx - lw * 0.5, ty + 4.0, sx + lw * 0.5, ty + 24.0);
            if placed.iter().any(|q| r.0 < q.2 && q.0 < r.2 && r.1 < q.3 && q.1 < r.3) {
                continue;
            }
            placed.push(r);
            if z >= 8.0 {
                let bw = self.gfx.measure(&name, f) + 18.0;
                let bh = if cap { 24.0 } else { 20.0 };
                let bx0 = sx - bw * 0.5;
                let by0 = ty + (28.0 - bh) * 0.5;
                self.gfx.fill_round(bx0, by0, bx0 + bw, by0 + bh, 4.0, [0.08, 0.06, 0.04, 0.85]);
                self.gfx.stroke_round(bx0, by0, bx0 + bw, by0 + bh, 4.0, if divine { 1.6 } else { 1.2 }, [c[0] * 0.7 + 0.25, c[1] * 0.7 + 0.25, c[2] * 0.7 + 0.2, 0.95]);
            }
            let col = [c[0] * 0.45 + 0.55, c[1] * 0.45 + 0.55, c[2] * 0.45 + 0.55, 1.0];
            self.gfx.text_shadow(&name, sx - 120.0, ty, 240.0, 28.0, f, col);
        }
    }

    fn act_cmd_icon(&self, act: Act) -> Option<CmdIcon> {
        let era = self.world.era;
        match act {
            Act::SelectAllWorkers | Act::ResumeBuild => Some(CmdIcon::Worker),
            Act::Train(k) | Act::SelectKind(k) | Act::FilterKind(k) => {
                let _ = era;
                Some(CmdIcon::of_unit(k))
            }
            Act::SelectAllMilitary => Some(CmdIcon::AllArmy),
            Act::WorkersAuto | Act::AllWorkersAuto => Some(CmdIcon::GatherAuto),
            Act::AllWorkersWood => Some(CmdIcon::GatherWood),
            Act::AllWorkersMine => Some(CmdIcon::GatherBronze),
            Act::Place(BKind::Farm) => Some(CmdIcon::Farm),
            Act::Place(BKind::Barracks) => Some(CmdIcon::Barracks),
            Act::Place(BKind::Town) => Some(CmdIcon::Town),
            Act::Place(BKind::Capital) => Some(CmdIcon::Capital),
            Act::Place(BKind::Fort) => Some(CmdIcon::Fort),
            Act::Place(BKind::Dock) => Some(CmdIcon::Dock),
            Act::Upgrade | Act::AllWorkersUpgrade | Act::AllArmyUpgrade => Some(CmdIcon::Upgrade),
            Act::Convert => Some(CmdIcon::Convert),
            Act::Stop => Some(CmdIcon::Stop),
            Act::AttackMove | Act::AllArmyAttack => Some(CmdIcon::AttackMove),
            Act::ArmyDefend | Act::AllArmyDefend => Some(CmdIcon::Defend),
            Act::Demolish => Some(CmdIcon::Demolish),
            Act::Diplo => Some(CmdIcon::Diplo),
            Act::Help => Some(CmdIcon::Help),
            _ => None,
        }
    }

    fn add_button(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, label: &str, sub: &str, key: u32, enabled: bool, act: Act, tip: &str) {
        let (mx, my) = self.mouse;
        let hover = mx >= x0 && mx <= x1 && my >= y0 && my <= y1;
        let btn_bmp = if !enabled {
            self.art.btn_dis.as_ref()
        } else if hover {
            self.art.btn_hover.as_ref()
        } else {
            self.art.btn_norm.as_ref()
        };
        if let Some(bmp) = btn_bmp {
            self.gfx.draw_bitmap(bmp, x0, y0, x1, y1);
        } else {
            let bg = if !enabled {
                [0.18, 0.17, 0.15, 0.9]
            } else if hover {
                [0.45, 0.36, 0.20, 0.95]
            } else {
                [0.30, 0.24, 0.15, 0.95]
            };
            self.gfx.fill_round(x0, y0, x1, y1, 4.0, bg);
            self.gfx.stroke_round(x0, y0, x1, y1, 4.0, 1.0, [0.75, 0.62, 0.35, if enabled { 0.9 } else { 0.4 }]);
        }

        let bh = y1 - y0;
        let mut tx0 = x0;
        if let Some(ci) = self.act_cmd_icon(act) {
            let icon = match act {
                Act::Train(k) | Act::SelectKind(k) | Act::FilterKind(k) => self.art.national_unit_sprite(k, self.world.era, self.world.player, self.world.year_f(), false),
                _ => self.art.cmd_icons[ci as usize].as_ref(),
            };
            if let Some(ibmp) = icon {
                if bh >= 36.0 && !(self.gfx.compact && y1 <= self.top_h()) {
                    let isz = (bh - 10.0).min(34.0);
                    let iy = y0 + (bh - isz) * 0.5;
                    self.gfx.draw_bitmap(ibmp, x0 + 3.0, iy, x0 + 3.0 + isz, iy + isz);
                    if !enabled {
                        self.gfx.fill_rect(x0 + 3.0, iy, x0 + 3.0 + isz, iy + isz, [0.05, 0.05, 0.05, 0.55]);
                    }
                    tx0 = x0 + isz + 4.0;
                } else if x1 - x0 >= 68.0 {
                    let isz = (bh - 6.0).max(14.0);
                    self.gfx.draw_bitmap(ibmp, x0 + 4.0, y0 + 3.0, x0 + 4.0 + isz, y0 + 3.0 + isz);
                    tx0 = x0 + isz + 4.0;
                }
            }
        }

        let tc = if enabled { [1.0, 0.95, 0.85, 1.0] } else { [0.55, 0.52, 0.48, 1.0] };
        let tw = (x1 - tx0).max(10.0);
        if sub.is_empty() {
            self.gfx.text_shadow(label, tx0, y0, tw, bh, Font::NormalCenter, tc);
        } else {
            self.gfx.text_shadow(label, tx0 + 2.0, y0 + 2.0, tw + 24.0, bh * 0.52, Font::Normal, tc);
            // 小見出しが入りきらなければ、アイコンの下まで左へ寄せる
            let sw = self.gfx.measure(sub, Font::Small);
            let sx = (x1 - 4.0 - sw).min(tx0 + 2.0).max(x0 + 4.0);
            self.gfx.text(sub, sx, y0 + bh * 0.53, x1 - sx, bh * 0.45, Font::Small, [tc[0] * 0.92, tc[1] * 0.86, tc[2] * 0.68, 1.0]);
        }
        if key >= 0x41 && key <= 0x5A {
            let ks = ((key as u8) as char).to_string();
            self.gfx.fill_round(x0 + 2.0, y0 + 2.0, x0 + 15.0, y0 + 15.0, 2.0, [0.08, 0.06, 0.04, 0.84]);
            self.gfx.text(&ks, x0 + 4.0, y0 + 1.0, 11.0, 14.0, Font::Small, [1.0, 0.88, 0.45, 0.98]);
        }
        if hover && !tip.is_empty() {
            self.notice = None;
        }
        self.buttons.push(Button { x0, y0, x1, y1, label: label.to_string(), key, enabled, act, tip: tip.to_string() });
    }

    fn cost_str(c: [f32; 3]) -> String {
        let cnt = c.iter().filter(|&&v| v > 0.0).count();
        let sep = if cnt >= 3 && crate::lang::lang() != crate::lang::Lang::En { "" } else { " " };
        let mut s = String::new();
        for k in 0..3 {
            if c[k] > 0.0 {
                if !s.is_empty() {
                    s.push_str(sep);
                }
                s.push_str(&format!("{}{}", res_short(k), c[k] as i32));
            }
        }
        s
    }

    fn render_title(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        // 国都マーカーの強調
        let hov = {
            let p = self.screen_to_world(self.mouse.0, self.mouse.1);
            let mut best: Option<(f32, usize)> = None;
            for (i, n) in self.world.nations.iter().enumerate() {
                if i < NATIONS.len() && NATIONS[i].playable && n.alive && n.home.dist(p) < 10.0 && best.map_or(true, |b| n.home.dist(p) < b.0) {
                    best = Some((n.home.dist(p), i));
                }
            }
            best.map(|b| b.1)
        };
        for (i, n) in self.world.nations.iter().enumerate() {
            if !n.alive || i >= NATIONS.len() {
                continue;
            }
            let (sx, sy) = self.world_to_screen(n.home);
            if Some(i) == self.title_sel || Some(i) == hov {
                self.gfx.fill_circle(sx, sy, 15.0, [1.0, 0.92, 0.55, 0.38]);
                self.gfx.stroke_circle(sx, sy, 15.0, 2.0, [1.0, 0.88, 0.40, 0.9]);
            }
            // 陣営の印（同盟国＝黒、協商国＝赤、中立＝白）
            let mark = match NATIONS[i].bloc {
                Bloc::Central => [0.15, 0.15, 0.15, 1.0],
                Bloc::Entente => [0.85, 0.2, 0.2, 1.0],
                Bloc::Neutral => [0.92, 0.92, 0.88, 1.0],
            };
            self.gfx.fill_circle(sx, sy, 4.5, mark);
            self.gfx.stroke_circle(sx, sy, 4.8, 1.0, [1.0, 0.85, 0.3, 1.0]);
            if !NATIONS[i].playable {
                self.gfx.stroke_circle(sx, sy, 8.0, 1.0, [0.6, 0.6, 0.6, 0.8]);
            }
        }
        let px = w - 380.0;
        self.art.draw_stone_panel(&self.gfx, px, 0.0, w, h);
        self.art.draw_corner_ornaments(&self.gfx, px + 4.0, 4.0, w - 4.0, h - 4.0, 18.0);
        // 英語の題字は長いので、扁額を広げた飾りを使う
        let wide = crate::lang::lang() == crate::lang::Lang::En;
        let crest = if wide { &self.art.title_crest_wide } else { &self.art.title_crest };
        if let Some(crest) = crest {
            self.gfx.draw_bitmap(crest, px + 20.0, 10.0, w - 20.0, 126.0);
        }
        let title = t!("欧州大戦", "The Great War", "欧洲大战");
        let plaque = if wide { crate::art::TITLE_PLAQUE_W_WIDE } else { crate::art::TITLE_PLAQUE_W };
        // 扁額（両端に少し余白）に収まらなければ一回り小さい書体で書く
        let tf = if self.gfx.measure(title, Font::Title) <= plaque - 8.0 { Font::Title } else { Font::TitleMid };
        self.gfx.text_shadow(title, px, 19.0, 380.0, 56.0, tf, [0.94, 0.92, 0.82, 1.0]);
        self.gfx.text_shadow(t!("— 1914–1919 —", "— 1914–1919 —", "— 1914–1919 —"), px, 76.0, 380.0, 28.0, Font::BigCenter, [0.96, 0.90, 0.78, 1.0]);

        let compact = self.gfx.compact;
        if !compact || self.title_sel.is_none() {
        self.art.draw_inset_box(&self.gfx, px + 18.0, 132.0, w - 18.0, 306.0);
        self.gfx.text(
            t!("1914年6月28日、サラエボに銃声が響く。同盟国（黒の印）と協商国（赤の印）は一か月のうちに戦争へ雪崩れ込む。白は中立国。\n\n地図上の首都をクリックして、率いる国を選べ（米国は選べない）。同盟国以外をすべて打ち破れば勝利。1919年6月までに決着がつかなければ、天使が降りる。", "June 28, 1914: shots ring out in Sarajevo. Within a month the Central Powers (black marks) and the Entente (red marks) tumble into war. White marks are neutrals.\n\nClick a capital on the map to choose the state you will lead (America cannot be chosen). Defeat everyone outside your alliance to win. If the war is not decided by June 1919, the angels descend.", "1914年6月28日，萨拉热窝响起枪声。同盟国（黑色标记）与协约国（红色标记）在一个月内卷入战争。白色为中立国。\n\n点击地图上的首都，选择你要率领的国家（不能选择美国）。击败同盟以外的所有国家即获胜。若1919年6月前未分胜负，天使将降临。"),
            px + 28.0,
            140.0,
            324.0,
            162.0,
            Font::Normal,
            [0.90, 0.86, 0.78, 1.0],
        );
        }
        if let Some(i) = self.title_sel {
            let n = &self.world.nations[i];
            let d = &NATIONS[i];
            // スマートフォンでは説明文の欄を隠して、国の情報をその位置に詰める
            let oy = if compact { 132.0 - 316.0 } else { 0.0 };
            // スマートフォンでは国の紹介文も省いて（下の行を詰めて）、開戦のボタンに重ならないようにする
            let dy = if compact { -44.0 } else { 0.0 };
            let by1 = oy + dy + if compact { 552.0 } else { 546.0 };
            self.art.draw_inset_box(&self.gfx, px + 18.0, oy + 316.0, w - 18.0, by1);
            self.art.draw_corner_ornaments(&self.gfx, px + 20.0, oy + 318.0, w - 20.0, by1 - 2.0, 12.0);
            if let Some(cap_ic) = self.art.national_unit_sprite(UnitKind::Infantry, self.world.era, i, self.world.year_f(), false) {
                self.gfx.draw_bitmap(cap_ic, px + 30.0, oy + 328.0, px + 78.0, oy + 376.0);
            }
            self.gfx.fill_rect(px + 86.0, oy + 336.0, px + 104.0, oy + 368.0, c4(n.color, 1.0));
            self.gfx.stroke_rect(px + 86.0, oy + 336.0, px + 104.0, oy + 368.0, 1.2, [0.92, 0.78, 0.40, 1.0]);
            self.gfx.text_shadow(crate::lang::tx(n.name), px + 112.0, oy + 332.0, 230.0, 40.0, Font::Big, [1.0, 0.95, 0.85, 1.0]);
            if !compact {
                self.gfx.text(crate::lang::tx(d.desc), px + 32.0, oy + 382.0, 316.0, 44.0, Font::Normal, [0.88, 0.84, 0.76, 1.0]);
            }
            let oy = oy + dy;
            let side = bloc_name(d.bloc);
            let gov = if d.gov == Gov::Empire { t!("帝政（革命のおそれ）", "Empire (can fall to revolution)", "帝制（可能爆发革命）") } else { t!("民主制", "Democracy", "民主制") };
            let s = tf!("{}・{}　工業力 ×{:.2}", "{} · {}   Industry ×{:.2}", "{}・{}　工业力 ×{:.2}", side, gov, d.bias);
            self.gfx.text(&s, px + 32.0, oy + 426.0, 316.0, 20.0, Font::Normal, [0.98, 0.86, 0.48, 1.0]);
            let cities = self.world.blds.iter().filter(|b| b.alive && b.nation as usize == i && b.kind.city()).count();
            let s = tf!("都市：{}　人口：{}　{}", "Cities: {}   Population: {}   {}", "城市：{}　人口：{}　{}", cities, d.pop, crate::lang::tx(INITIAL_HOUSE[i]));
            self.gfx.text(&s, px + 32.0, oy + 448.0, 316.0, 20.0, Font::Normal, [0.88, 0.84, 0.76, 1.0]);
            let sp: Vec<&str> = special_kinds(i).iter().map(|x| x.0.name(Era::Total)).collect();
            let diff = if d.pop >= 55 {
                t!("難易度：易", "Difficulty: Easy", "难度：易")
            } else if d.pop >= 20 {
                t!("難易度：中", "Difficulty: Normal", "难度：中")
            } else {
                t!("難易度：難", "Difficulty: Hard", "难度：难")
            };
            let s = if sp.is_empty() { diff.to_string() } else { tf!("{}　特別な兵：{}", "{}   Special units: {}", "{}　特殊兵种：{}", diff, sp.join(sep())) };
            self.gfx.text(&s, px + 32.0, oy + 470.0, 316.0, 20.0, Font::Normal, [0.88, 0.84, 0.76, 1.0]);
            // 史実に基づく国の特徴（説明はツールチップ）
            let trs: Vec<&Trait> = all_traits_of(i).collect();
            if !trs.is_empty() {
                let names: Vec<String> = trs
                    .iter()
                    .map(|t| if t.from > START_YEAR { tf!("{}（{}年〜）", "{} (from {})", "{}（{}年起）", crate::lang::tx(t.name), t.from as i32) } else if t.to < END_YEAR { tf!("{}（〜{}年）", "{} (until {})", "{}（至{}年）", crate::lang::tx(t.name), t.to as i32) } else { crate::lang::tx(t.name).to_string() })
                    .collect();
                let s = tf!("特徴：{}", "Traits: {}", "特点：{}", names.join(sep()));
                self.gfx.text(&s, px + 32.0, oy + 492.0, 316.0, 48.0, Font::Normal, [0.70, 0.92, 1.0, 1.0]);
                let tip: Vec<String> = trs.iter().map(|t| format!("【{}】{}", crate::lang::tx(t.name), crate::lang::tx(t.desc))).collect();
                self.buttons.push(Button {
                    x0: px + 32.0,
                    y0: oy + 490.0,
                    x1: w - 30.0,
                    y1: oy + 540.0,
                    label: String::new(),
                    key: 0,
                    enabled: true,
                    act: Act::Noop,
                    tip: tip.join("\n"),
                });
            }
            let sy = if compact { (oy + 560.0).min(h - 138.0) } else { 554.0 };
            self.add_button(px + 50.0, sy, w - 50.0, sy + if compact { 40.0 } else { 46.0 }, t!("この国で開戦に臨む", "Lead this state to war", "率领此国迎接开战"), "", 0, true, Act::Start, "");
        }
        if compact {
            self.render_title_buttons_compact(px);
            return;
        }
        let ty = (h - 128.0).max(656.0);
        // 記録から続ける
        self.add_button(px + 24.0, ty - 54.0, w - 24.0, ty - 12.0, t!("続きから（記録を読み込む） F5", "Continue (load a save) F5", "继续（读取存档） F5"), "", 0, true, Act::SaveMenu, t!("記録した対局を再開する。書き出したファイルからも読み込める", "Resume a saved game. Exported files can be loaded too", "继续已保存的对局。也可读取导出的文件"));
        self.add_button(px + 24.0, ty, w - 24.0, ty + 48.0, t!("はじめての方は：チュートリアル", "New here? Tutorial", "初次游玩：教程"), "", 0, true, Act::Tutorial, t!("フランスの指導者として、操作と遊び方を1段ずつ学ぶ", "Learn the controls and the game step by step as leader of France", "以法国领导人的身份逐步学习操作与玩法"));
        let ty = ty + 60.0;
        self.add_button(px + 24.0, ty, px + 70.0, ty + 40.0, "A−", "", 0, true, Act::Scale(-1), t!("文字と画面部品を小さくする（Ctrl＋−）", "Make text and interface smaller (Ctrl+−)", "缩小文字与界面（Ctrl＋−）"));
        self.add_button(px + 74.0, ty, px + 120.0, ty + 40.0, "A＋", "", 0, true, Act::Scale(1), t!("文字と画面部品を大きくする（Ctrl＋＋）", "Make text and interface larger (Ctrl++)", "放大文字与界面（Ctrl＋＋）"));
        let fl = if self.gfx.is_fullscreen() { t!("ウィンドウ F11", "Window F11", "窗口 F11") } else { t!("全画面 F11", "Fullscreen F11", "全屏 F11") };
        // 表示言語（いまの言語の名前を出し、押すと次の言語へ）
        let ll = format!("{} ▸", crate::lang::lang().native());
        self.add_button(px + 124.0, ty, px + 222.0, ty + 40.0, &ll, "", 0, true, Act::Lang, "表示言語を切り替える / Change language / 切换语言（日本語 → English → 中文）");
        self.add_button(px + 226.0, ty, w - 24.0, ty + 40.0, fl, "", 0, true, Act::Fullscreen, t!("全画面表示を切り替える", "Toggle fullscreen", "切换全屏显示"));
        // タイトル画面のツールチップ
        let (mx, my) = self.mouse;
        let tip = self.buttons.iter().find(|b| mx >= b.x0 && mx <= b.x1 && my >= b.y0 && my <= b.y1 && !b.tip.is_empty()).map(|b| b.tip.clone());
        if let Some(t) = tip.filter(|_| !self.show_save && !(self.touch && self.clock > self.tip_until)) {
            // 説明の行数に合わせて上へ伸ばす
            let x0 = px + 24.0;
            let th = self.gfx.measure_h(&t, Font::Small, w - x0 - 40.0) + 10.0;
            // 「続きから」の上に出す
            let y1 = ty - 120.0;
            self.art.draw_inset_box(&self.gfx, x0, y1 - th - 4.0, w - 24.0, y1);
            self.gfx.text(&t, x0 + 8.0, y1 - th, w - x0 - 40.0, th, Font::Small, [0.95, 0.92, 0.85, 1.0]);
        }
    }

    fn render_hud(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        if self.gfx.compact {
            self.render_topbar_compact();
        } else {
        // ---- 上部バー（Age of Empires 風石彫パネル＋資源アイコン）
        self.art.draw_stone_panel(&self.gfx, 0.0, 0.0, w, self.top_h());
        let n = &self.world.nations[pl];
        let mut x = 8.0;
        let res = n.res;
        let inc = n.income;
        let upkeep = n.upkeep;
        let res_icons = [ResIcon::Food, ResIcon::Wood, ResIcon::Bronze];
        let cap = self.world.pop_cap(pl);
        let (n_workers, n_soldiers) = (n.workers, n.soldiers);
        for k in 0..3 {
            if let Some(ic) = &self.art.res_icons[res_icons[k] as usize] {
                self.gfx.draw_bitmap(ic, x, 4.0, x + 28.0, 32.0);
            }
            let rate = if k == 0 { inc[k] - upkeep } else { inc[k] };
            let s = format!("{}", res[k] as i32);
            self.gfx.text_shadow(&s, x + 31.0, 7.0, 72.0, 22.0, Font::Big, RES_COLORS[k]);
            let rs = tf!("{:+.1}/秒", "{:+.1}/s", "{:+.1}/秒", rate);
            let rc = if rate < 0.0 { [1.0, 0.42, 0.32, 1.0] } else { [0.80, 0.78, 0.70, 1.0] };
            self.gfx.text(&rs, x + 88.0, 11.0, 56.0, 18.0, Font::Small, rc);
            x += 148.0;
        }
        // 民全体・全軍をワンクリック選択できるトップバーアイコンボタン
        let (mx, my) = self.mouse;
        let w_hov = mx >= x && mx <= x + 88.0 && my >= 4.0 && my <= 32.0;
        self.gfx.fill_round(x, 4.0, x + 88.0, 32.0, 4.0, if w_hov { [0.35, 0.28, 0.16, 0.9] } else { [0.16, 0.13, 0.09, 0.75] });
        self.gfx.stroke_round(x, 4.0, x + 88.0, 32.0, 4.0, 1.0, [0.75, 0.62, 0.34, 0.85]);
        if let Some(ic) = &self.art.res_icons[ResIcon::Worker as usize] {
            self.gfx.draw_bitmap(ic, x + 3.0, 5.0, x + 29.0, 31.0);
        }
        let s = format!("{}", n_workers);
        self.gfx.text_shadow(&s, x + 32.0, 8.0, 54.0, 22.0, Font::Normal, [0.95, 0.92, 0.85, 1.0]);
        self.buttons.push(Button {
            x0: x,
            y0: 4.0,
            x1: x + 88.0,
            y1: 32.0,
            label: String::new(),
            key: 0,
            enabled: true,
            act: Act::SelectAllWorkers,
            tip: tf!("民（生産ユニット）全体を選択する。民と兵を合わせた人口 {}/{}（兵は民から徴兵する）", "Select all workers. Population (workers + soldiers) {}/{} (soldiers are drafted from workers)", "选择全部民（生产单位）。民与兵合计人口 {}/{}（兵由民征召）", n_workers + n_soldiers, cap),
        });
        x += 94.0;

        let s_hov = mx >= x && mx <= x + 88.0 && my >= 4.0 && my <= 32.0;
        self.gfx.fill_round(x, 4.0, x + 88.0, 32.0, 4.0, if s_hov { [0.35, 0.28, 0.16, 0.9] } else { [0.16, 0.13, 0.09, 0.75] });
        self.gfx.stroke_round(x, 4.0, x + 88.0, 32.0, 4.0, 1.0, [0.75, 0.62, 0.34, 0.85]);
        if let Some(ic) = &self.art.res_icons[ResIcon::Soldier as usize] {
            self.gfx.draw_bitmap(ic, x + 3.0, 5.0, x + 29.0, 31.0);
        }
        let s = self.soldier_text(n_soldiers, n_workers + n_soldiers);
        self.gfx.text_shadow(&s, x + 32.0, 8.0, 54.0, 22.0, Font::Normal, [0.95, 0.92, 0.85, 1.0]);
        self.buttons.push(Button {
            x0: x,
            y0: 4.0,
            x1: x + 88.0,
            y1: 32.0,
            label: String::new(),
            key: 0,
            enabled: true,
            act: Act::SelectAllMilitary,
            tip: t!("軍事ユニット全体（全軍）を選択する", "Select all soldiers", "选择全部军事单位（全军）").to_string(),
        });
        x += 96.0;

        if let Some(ic) = &self.art.res_icons[ResIcon::Upkeep as usize] {
            self.gfx.draw_bitmap(ic, x, 5.0, x + 26.0, 31.0);
        }
        let s = tf!("-{:.1}/秒", "-{:.1}/s", "-{:.1}/秒", upkeep);
        self.gfx.text_shadow(&s, x + 28.0, 8.0, 88.0, 22.0, Font::Normal, [0.98, 0.68, 0.52, 1.0]);
        x += 112.0;
        // 世論
        let (op, opd) = (self.world.nations[pl].opinion, self.world.nations[pl].opinion_delta);
        let oc = opinion_color(op);
        self.gfx.fill_circle(x + 13.0, 18.0, 11.0, c4(oc, 1.0));
        self.gfx.stroke_circle(x + 13.0, 18.0, 11.0, 1.2, [0.25, 0.20, 0.12, 1.0]);
        self.gfx.fill_rect(x + 8.0, 12.0, x + 18.0, 15.0, [0.15, 0.12, 0.08, 1.0]);
        self.gfx.fill_rect(x + 8.0, 17.0, x + 18.0, 20.0, [0.15, 0.12, 0.08, 1.0]);
        self.gfx.fill_rect(x + 8.0, 22.0, x + 15.0, 25.0, [0.15, 0.12, 0.08, 1.0]);
        let s = format!("{}", op as i32);
        self.gfx.text_shadow(&s, x + 28.0, 7.0, 64.0, 22.0, Font::Big, c4(oc, 1.0));
        let s = tf!("{:+.1}/月", "{:+.1}/mo", "{:+.1}/月", opd);
        self.gfx.text(&s, x + 66.0, 11.0, 70.0, 18.0, Font::Small, if opd < 0.0 { [1.0, 0.5, 0.4, 1.0] } else { [0.80, 0.78, 0.70, 1.0] });
        self.buttons.push(Button {
            x0: x,
            y0: 4.0,
            x1: x + 136.0,
            y1: 32.0,
            label: String::new(),
            key: 0,
            enabled: true,
            act: Act::Market,
            tip: self.opinion_tip(pl),
        });

        // 速度・メニュー
        let bx = w - 424.0;
        let ex = (bx - 248.0).max(x + 150.0);
        // 幅に収まるいちばん詳しい書き方を選ぶ（言語で文字幅が違うので測って決める）
        let date = self.world.date_str();
        let early = self.world.era == Era::Early;
        let long = format!("{}　{}", date, if early { t!("開戦期", "Opening campaigns", "开战期") } else { t!("総力戦", "Total war", "总体战") });
        let short = date.clone();
        let room = bx - ex - 6.0;
        let (s, ef) = [(long, Font::Big), (short.clone(), Font::Big), (short.clone(), Font::Normal), (short.clone(), Font::Small)]
            .into_iter()
            .find(|(s, f)| self.gfx.measure(s, *f) <= room)
            .unwrap_or((short, Font::Small));
        if let Some(ic) = &self.art.res_icons[ResIcon::Era as usize] {
            self.gfx.draw_bitmap(ic, ex - 22.0, 8.0, ex - 2.0, 28.0);
        }
        self.gfx.text_shadow(&s, ex, if ef == Font::Big { 7.0 } else { 9.0 }, (bx - ex - 4.0).max(60.0), 24.0, ef, [1.0, 0.9, 0.6, 1.0]);
        let paused = self.paused;
        let sp = self.speed;
        self.add_button(bx, 5.0, bx + 36.0, 31.0, if paused { "▶" } else { "❚❚" }, "", 0, true, Act::Speed(0), "");
        for (k, s) in [1u32, 2, 4, 8].iter().enumerate() {
            let x0 = bx + 39.0 + k as f32 * 35.0;
            let lbl = format!("×{}", s);
            self.add_button(x0, 5.0, x0 + 33.0, 31.0, &lbl, "", 0, true, Act::Speed(*s), "");
            if sp == *s && !paused {
                self.gfx.stroke_round(x0, 5.0, x0 + 33.0, 31.0, 3.0, 2.0, [1.0, 0.9, 0.4, 1.0]);
            }
        }
        self.add_button(w - 242.0, 5.0, w - 166.0, 31.0, t!("戦況 F4", "War F4", "战况 F4"), "", 0, true, Act::Market, t!("各国の世論・損害・海上の情勢（Uボート・船団・米国の参戦気運）", "Public opinion and losses of every state, and the war at sea (U-boats, convoys, America's war fever)", "各国舆论、损失与海上形势（U艇、船队、美国参战情绪）"));
        self.add_button(w - 162.0, 5.0, w - 84.0, 31.0, t!("外交 F2", "Diplo F2", "外交 F2"), "", 0, true, Act::Diplo, "");
        self.add_button(w - 80.0, 5.0, w - 4.0, 31.0, t!("ヘルプ F1", "Help F1", "帮助 F1"), "", 0, true, Act::Help, "");
        // 記録（上部バーに余地が無いので、その下の右端に置く）
        self.add_button(w - 80.0, self.top_h() + 4.0, w - 4.0, self.top_h() + 30.0, t!("記録 F5", "Save F5", "存档 F5"), "", 0, true, Act::SaveMenu, t!("記録（セーブ・ロード）。3か月ごとに自動でも記録する", "Save and load. The game also saves automatically every 3 months", "保存与读取。每3个月也会自动保存"));
        }
        let paused = self.paused;
        if paused {
            self.gfx.text_shadow(t!("一時停止中（Space）", "Paused (Space)", "已暂停（Space）"), w / 2.0 - 120.0, self.top_h() + 60.0, 240.0, 30.0, Font::BigCenter, [1.0, 1.0, 0.8, 1.0]);
        } else if let Some(left) = self.world.descent_left() {
            let s = tf!("天使の降臨：すべての都市が占領されるまで あと{}か月", "Descent of the Angels: {} months until every city falls", "天使降临：距所有城市被占领还有{}个月", (left * 12.0).ceil() as i32);
            self.gfx.text_shadow(&s, w / 2.0 - 320.0, self.top_h() + 60.0, 640.0, 30.0, Font::BigCenter, [1.0, 0.75, 0.35, 1.0]);
        } else if self.cmd_hold() {
            self.gfx.text_shadow(t!("指揮中（時は止まっている。命令を出すか選択を解くと進む）", "Commanding: time stopped (give an order or deselect)", "指挥中（时间已停止。下达命令或取消选择后继续）"), w / 2.0 - 320.0, self.top_h() + 60.0, 640.0, 30.0, Font::BigCenter, [0.8, 0.95, 1.0, 1.0]);
        }

        // ---- 警報（世論の悪化・米国の参戦気運）
        let mut cy = self.top_h() + 6.0;
        let mut alerts: Vec<(String, bool)> = Vec::new();
        let op = self.world.nations[pl].opinion;
        if op < UNREST_AT {
            let s = if NATIONS[pl].gov == Gov::Empire {
                tf!("世論 {:.0}：国内が不穏（0 近くで革命が起きて降伏）", "Opinion {:.0}: unrest (near 0, revolution forces surrender)", "舆论 {:.0}：国内动荡（接近0将爆发革命而投降）", op)
            } else {
                tf!("世論 {:.0}：ストライキで生産が落ちている", "Opinion {:.0}: strikes are cutting production", "舆论 {:.0}：罢工导致生产下降", op)
            };
            alerts.push((s, true));
        }
        if self.world.nations[USA].alive && self.world.nations[USA].bloc == Bloc::Neutral && self.world.sea.us_anger > 10.0 {
            alerts.push((tf!("米国の参戦気運 {:.0}/100", "US war fever {:.0}/100", "美国参战情绪 {:.0}/100", self.world.sea.us_anger), self.world.nations[pl].bloc == Bloc::Central && self.world.sea.us_anger > 60.0));
        }
        if self.world.sea.unrestricted {
            alerts.push((t!("無制限潜水艦作戦中", "Unrestricted submarine warfare in effect", "无限制潜艇战进行中").to_string(), false));
        }
        let compact = self.gfx.compact;
        if compact && !self.alerts_open && !alerts.is_empty() {
            // タッチ向けの画面：地図を覆わないよう、畳んだ目印だけ出す（押すと開く）
            let urgent = alerts.iter().any(|a| a.1);
            let lbl = tf!("お知らせ {} ▾", "Alerts {} ▾", "提示 {} ▾", alerts.len());
            let bw = self.gfx.measure(&lbl, Font::Normal) + 28.0;
            self.add_button(w - 6.0 - bw, cy, w - 6.0, cy + 34.0, &lbl, "", 0, true, Act::ToggleAlerts, "");
            if urgent {
                self.gfx.stroke_round(w - 6.0 - bw, cy, w - 6.0, cy + 34.0, 3.0, 2.0, [1.0, 0.35, 0.25, 1.0]);
            }
        }
        let show_list = !compact || self.alerts_open;
        let lw = if compact { 472.0f32.min(w * 0.62) } else { 472.0 };
        let list_y0 = cy;
        for (s, urgent) in alerts.iter().filter(|_| show_list) {
            let bg = if *urgent { [0.42, 0.10, 0.08, 0.90] } else { [0.20, 0.16, 0.10, 0.85] };
            self.gfx.fill_round(w - lw, cy, w - 6.0, cy + 23.0, 3.0, bg);
            self.gfx.stroke_round(w - lw, cy, w - 6.0, cy + 23.0, 3.0, 1.2, [0.92, 0.72, 0.34, 0.9]);
            self.gfx.text_shadow(s, w - lw + 8.0, cy + 2.0, lw - 12.0, 20.0, Font::Normal, [1.0, 0.88, 0.52, 1.0]);
            cy += 25.0;
        }
        if compact && self.alerts_open {
            // 開いた一覧は、押すと畳む
            let label = t!("▴ 畳む", "▴ Hide", "▴ 收起");
            self.add_button(w - 96.0, cy + 2.0, w - 6.0, cy + 34.0, label, "", 0, true, Act::ToggleAlerts, "");
            self.buttons.push(Button { x0: w - lw, y0: list_y0, x1: w - 6.0, y1: cy, label: String::new(), key: 0, enabled: true, act: Act::ToggleAlerts, tip: String::new() });
        }

        // ---- メッセージ
        let now = self.world.time;
        let mut y = self.top_h() + 8.0;
        let cut = self.msg_cut;
        let msgs: Vec<(String, [f32; 3], f32)> = self.world.msgs.iter().rev().take(if self.gfx.compact { 4 } else { 7 }).filter(|m| now - m.time < 25.0 && m.time > cut).map(|m| (m.text.clone(), m.color, now - m.time)).collect();
        let any_msg = !msgs.is_empty();
        for (t, c, age) in msgs.into_iter().rev() {
            let a = if age > 20.0 { (25.0 - age) / 5.0 } else { 1.0 };
            let mw = if self.gfx.compact { w * 0.5 } else { w * 0.55 };
            let mh = self.gfx.measure_h(&t, Font::Normal, mw).clamp(20.0, 40.0);
            self.gfx.text_shadow(&t, 10.0, y, mw, mh, Font::Normal, [c[0], c[1], c[2], a]);
            y += mh;
        }
        if any_msg {
            // お知らせを消す ✕
            self.add_button(10.0, y + 2.0, 46.0, y + 34.0, "✕", "", 0, true, Act::ClearMsgs, t!("お知らせを消す", "Clear messages", "清除消息"));
        }

        // ---- イベントバナー
        if let Some(b) = &self.world.banner {
            let a = b.t.min(1.0);
            let (title, text) = (b.title.clone(), b.text.clone());
            let bw = 760.0f32.min(w - 40.0);
            let x0 = (w - bw) / 2.0;
            let y0 = self.top_h() + if self.gfx.compact { 40.0 } else { 90.0 };
            let th = self.gfx.measure_h(&text, Font::NormalCenter, bw - 40.0).max(24.0);
            let bh = 66.0 + th + 34.0;
            self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + bw, y0 + bh);
            self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + bw - 3.0, y0 + bh - 3.0, 14.0);
            // 右上の ✕ に重ならないよう、題は左右に余白を取り、収まらなければ一回り小さい書体で書く
            let tw = bw - 104.0;
            let tf = if self.gfx.measure(&title, Font::Title) <= tw { Font::Title } else { Font::TitleMid };
            self.gfx.text_shadow(&title, x0 + 52.0, y0 + 8.0, tw, 44.0, tf, [1.0, 0.85, 0.4, a]);
            self.gfx.text_shadow(&text, x0 + 20.0, y0 + 58.0, bw - 40.0, th + 6.0, Font::NormalCenter, [1.0, 0.95, 0.85, a]);
            // 告知は押すと閉じる（時が止まっている間も消せるように）。右上の ✕・Esc でも閉じる
            let more = self.world.banner_queue_len();
            let hint = if self.gfx.compact {
                t!("タップで閉じる", "Tap to close", "点击关闭").to_string()
            } else if more > 0 {
                tf!("クリック・Esc で閉じる（あと{}件）", "Click or press Esc to close ({} more)", "点击或按 Esc 关闭（还有{}条）", more)
            } else {
                t!("クリック・Esc で閉じる", "Click or press Esc to close", "点击或按 Esc 关闭").to_string()
            };
            self.gfx.text_shadow(&hint, x0, y0 + bh - 28.0, bw, 22.0, Font::NormalCenter, [0.85, 0.80, 0.70, a]);
            self.buttons.push(Button { x0, y0, x1: x0 + bw, y1: y0 + bh, label: String::new(), key: 0, enabled: true, act: Act::DismissBanner, tip: String::new() });
            self.add_button(x0 + bw - 40.0, y0 + 8.0, x0 + bw - 8.0, y0 + 40.0, "✕", "", 0, true, Act::DismissBanner, t!("閉じる（Esc）", "Close (Esc)", "关闭（Esc）"));
        }

        // ---- 下部パネル（Age of Empires 風石彫レリーフ＆3区画フレーム）
        let py = h - BOT_H;
        self.art.draw_stone_panel(&self.gfx, 0.0, py, w, h);

        // ミニマップ枠
        let (mx0, my0, mx1, my1) = self.mm_rect();
        self.gfx.fill_rect(mx0 - 2.0, my0 - 2.0, mx1 + 2.0, my1 + 2.0, [0.05, 0.04, 0.03, 1.0]);
        if let Some(b) = &self.minimap_bmp {
            self.gfx.draw_bitmap(b, mx0, my0, mx1, my1);
        }
        let sxs = MM_W / MAP_W as f32;
        let sys = MM_H / MAP_H as f32;
        let dots: Vec<(f32, f32, [f32; 3], bool)> = self
            .world
            .units
            .iter()
            .filter(|u| u.alive && u.kind.military() && (u.nation as usize == pl || self.seen(u.pos)))
            .map(|u| (mx0 + u.pos.x * sxs, my0 + u.pos.y * sys, self.world.nations[u.nation as usize].color, u.nation as usize == pl))
            .collect();
        for (x, y, c, mine) in dots {
            let s = if mine { 1.6 } else { 1.0 };
            self.gfx.fill_rect(x - s, y - s, x + s, y + s, if mine { [1.0, 1.0, 1.0, 1.0] } else { c4(darker(c, 0.6), 1.0) });
        }
        let a = self.screen_to_world(0.0, self.top_h());
        let b = self.screen_to_world(w, h - BOT_H);
        self.gfx.stroke_rect(
            (mx0 + a.x * sxs).max(mx0),
            (my0 + a.y * sys).max(my0),
            (mx0 + b.x * sxs).min(mx1),
            (my0 + b.y * sys).min(my1),
            1.2,
            [1.0, 0.96, 0.80, 0.92],
        );
        self.gfx.stroke_rect(mx0, my0, mx1, my1, 2.0, [0.78, 0.64, 0.34, 1.0]);
        self.art.draw_corner_ornaments(&self.gfx, mx0, my0, mx1, my1, 10.0);

        // 中央の情報欄ボックス
        let ix = MM_W + 12.0;
        let iw = w - ix - 394.0;
        self.art.draw_inset_box(&self.gfx, ix, py + 6.0, ix + iw, h - 4.0);
        self.art.draw_corner_ornaments(&self.gfx, ix + 2.0, py + 8.0, ix + iw - 2.0, h - 6.0, 11.0);
        self.render_info(ix + 12.0, py + 12.0, iw - 24.0);
        {
            // 兵を選んでいれば、兵器（戦車・軍艦）に割り当てるボタン
            let era_now = self.world.era;
            let crews = self.own_sel().iter().filter(|&&h| self.world.unit(h).map_or(false, World::can_crew)).count();
            if crews > 0 {
                let mut bx1 = ix + iw - 14.0;
                for &(k, _) in special_kinds(pl).iter().rev() {
                    if !k.weapon() {
                        continue;
                    }
                    let need = k.crew() as usize;
                    let c = self.world.train_cost(pl, k);
                    let res = self.world.nations[pl].res;
                    let avail = self.world.kind_available(pl, k);
                    let ok = avail && crews >= need && res[0] >= c[0] && res[1] >= c[1] && res[2] >= c[2];
                    let key = if k.naval() { 'U' } else { 'B' };
                    let label = tf!("{}に割り当て", "Crew {}", "编入{}", k.name(era_now));
                    let sub = format!("{}{} {}", need, t!("人", " men", "人"), Self::cost_str(c));
                    let mut tip = tf!(
                        "選んだ兵のうち{}人を{}に割り当てる。兵は{}に変わり、兵に戻せない。能力は乗った兵の平均。{}",
                        "Assign {} of the selected soldiers to a {}. They become a {} and can never be released. Its power is the crew's average. {}",
                        "将所选士兵中的{}人编入{}。士兵变为{}，无法撤回。能力为乘员的平均值。{}",
                        need,
                        k.name(era_now),
                        k.name(era_now),
                        if k.naval() { t!("自国の軍港のそば（10マス以内）の兵だけが乗れる。", "Only soldiers near one of your docks (within 10 tiles) can crew it.", "只有己方军港附近（10格内）的士兵可以乘坐。") } else { t!("自国の領内の兵だけが乗れる。", "Only soldiers inside your territory can crew it.", "只有己方领土内的士兵可以乘坐。") }
                    );
                    if !avail {
                        let from = special_kinds(pl).iter().find(|x| x.0 == k).map(|x| x.1).unwrap_or(0.0);
                        let y = from.floor() as i32;
                        let m = ((from - y as f32) * 12.0).floor() as i32 + 1;
                        tip.push_str(&tf!("\n{}年{}月から造れる", "\nAvailable from {}/{}", "\n{}年{}月起可建造", y, m));
                    }
                    self.add_button(bx1 - 176.0, h - 58.0, bx1, h - 14.0, &label, &sub, key as u32, ok, Act::Crew(k), &tip);
                    bx1 -= 182.0;
                }
            }
        }

        // スマートフォン：選択や建設場所選びを指で解く ✕
        if self.gfx.compact && (self.mode != Mode::Normal || !self.sel.is_empty() || self.sel_b.is_some()) {
            let tip = if self.mode != Mode::Normal { t!("取り消す", "Cancel", "取消") } else { t!("選択を解く", "Deselect", "取消选择") };
            self.add_button(ix + iw - 46.0, py + 10.0, ix + iw - 8.0, py + 48.0, "✕", "", 0, true, Act::Cancel, tip);
        }

        // 右側のコマンド欄ボックス
        let cx0 = w - 388.0;
        self.art.draw_inset_box(&self.gfx, cx0, py + 6.0, w - 4.0, h - 4.0);
        self.art.draw_corner_ornaments(&self.gfx, cx0 + 2.0, py + 8.0, w - 6.0, h - 6.0, 11.0);
        if !self.show_diplo && !self.show_help && !self.show_chron && !self.show_market && !self.show_save {
            self.render_commands(w - 382.0, py + 12.0);
        }

        if let Some((t, life)) = &self.notice {
            let a = life.min(1.0);
            let t = t.clone();
            self.gfx.text_shadow(&t, w / 2.0 - 300.0, py - 34.0, 600.0, 26.0, Font::BigCenter, [1.0, 0.85, 0.5, a]);
        }
    }

    /// ボタンのツールチップ（すべての画面部品の上に描く。上端で切れるならカーソルの下へ）
    fn render_tooltip(&mut self) {
        if self.touch && self.clock > self.tip_until {
            return;
        }
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let (mx, my) = self.mouse;
        // 後から置いたボタン（上に重なる画面）を優先
        let mut tip = None;
        for b in self.buttons.iter().rev() {
            if mx >= b.x0 && mx <= b.x1 && my >= b.y0 && my <= b.y1 {
                if b.label == BLOCK_LABEL {
                    break;
                }
                if !b.tip.is_empty() {
                    tip = Some(b.tip.clone());
                    break;
                }
            }
        }
        let Some(t) = tip else { return };
        let tw = 340.0f32.min(w - 8.0);
        let th = self.gfx.measure_h(&t, Font::Small, tw - 16.0) + 14.0;
        let x0 = (mx - tw * 0.5).clamp(4.0, (w - tw - 4.0).max(4.0));
        let y0 = if my - th - 10.0 >= 2.0 { my - th - 10.0 } else { (my + 24.0).min(h - th - 2.0) };
        self.art.draw_inset_box(&self.gfx, x0, y0, x0 + tw, y0 + th);
        self.gfx.text(&t, x0 + 8.0, y0 + 6.0, tw - 16.0, th - 8.0, Font::Small, [0.95, 0.92, 0.85, 1.0]);
    }

    fn order_text(&self, u: &Unit) -> &'static str {
        match u.order {
            Order::Idle => t!("待機中", "Idle", "待命"),
            Order::Move(_) => t!("移動中", "Moving", "移动中"),
            Order::AttackMove(_) => t!("進軍中", "Advancing", "进军中"),
            Order::Attack(_) => t!("交戦中", "Fighting", "交战中"),
            Order::Gather { src, phase } => match (src.res(), phase) {
                (_, 2) => t!("運搬中", "Carrying", "搬运中"),
                (0, _) => t!("農耕中", "Farming", "耕作中"),
                (1, _) => t!("伐採中", "Cutting wood", "伐木中"),
                _ => t!("採掘中", "Mining", "采矿中"),
            },
            Order::Build(_) => t!("建設中", "Building", "建造中"),
        }
    }

    fn render_info(&mut self, x: f32, y: f32, wd: f32) {
        let pl = self.world.player;
        let era = self.world.era;
        let cap = self.world.power_cap();
        let tc = [0.95, 0.92, 0.85, 1.0];
        if self.sel.len() == 1 {
            let u = self.world.unit(self.sel[0]).unwrap();
            let n = &self.world.nations[u.nation as usize];
            let nc = n.color;
            // Age of Empires 風の大型全身ユニット肖像カード＋体力バー
            let px0 = x + 2.0;
            let py0 = y + 2.0;
            let psz = 88.0;
            self.art.draw_inset_box(&self.gfx, px0 - 2.0, py0 - 2.0, px0 + psz + 2.0, py0 + psz + 2.0);
            self.gfx.fill_circle(px0 + psz * 0.5, py0 + psz * 0.74, psz * 0.34, c4(nc, 0.42));
            self.gfx.stroke_circle(px0 + psz * 0.5, py0 + psz * 0.74, psz * 0.34, 1.5, c4(nc, 0.9));
            let sprite_bmp = if u.divine {
                self.art.divine_unit_sprite(u.kind, false).or_else(|| self.art.national_unit_sprite(u.kind, era, u.nation as usize, self.world.year_f(), false))
            } else {
                self.art.national_unit_sprite(u.kind, era, u.nation as usize, self.world.year_f(), false)
            };
            if let Some(bmp) = sprite_bmp {
                self.gfx.draw_bitmap(bmp, px0 + 2.0, py0 + 2.0, px0 + psz - 2.0, py0 + psz - 2.0);
            }
            // 肖像下のHPバー
            let f = (u.hp / u.max_hp()).clamp(0.0, 1.0);
            self.gfx.fill_rect(px0, py0 + psz + 4.0, px0 + psz, py0 + psz + 11.0, [0.08, 0.06, 0.05, 0.95]);
            self.gfx.fill_rect(px0 + 1.0, py0 + psz + 5.0, px0 + 1.0 + (psz - 2.0) * f, py0 + psz + 10.0, [1.0 - f, 0.25 + 0.7 * f, 0.15, 1.0]);
            self.gfx.stroke_rect(px0, py0 + psz + 4.0, px0 + psz, py0 + psz + 11.0, 1.0, [0.75, 0.62, 0.34, 0.9]);

            let tx = x + 102.0;
            let tw = (wd - 102.0).max(100.0);
            let title = if u.divine {
                tf!("天使・{}", "Angel {}", "天使・{}", u.kind.name(era))
            } else if let Some(sh) = &u.ship {
                if sh.route != NONE {
                    tf!("{}からの船団【{}】", "Convoy from {} [{}]", "来自{}的船队【{}】", crate::lang::tx(CONVOYS[sh.route as usize].name), crate::lang::tx(n.name))
                } else {
                    tf!("輸送船（兵{}人）【{}】", "Transport ({} troops) [{}]", "运输船（{}名士兵）【{}】", sh.cargo.len(), crate::lang::tx(n.name))
                }
            } else {
                format!("{}　【{}】", u.kind.name(era), crate::lang::tx(n.name))
            };
            let lines = tf!(
                "兵力 {:.0}/{:.0}　　能力 {:.1}（時代上限 {:.1}）\n状態：{}{}",
                "HP {:.0}/{:.0}    Power {:.1} (era limit {:.1})\nStatus: {}{}",
                "兵力 {:.0}/{:.0}　　能力 {:.1}（时代上限 {:.1}）\n状态：{}{}",
                u.hp.max(0.0),
                u.max_hp(),
                u.power,
                cap,
                self.order_text(u),
                if u.carry > 0.5 { tf!("（{} {:.0}）", " ({} {:.0})", "（{} {:.0}）", res_name(u.carry_kind), u.carry) } else { String::new() }
            );
            let desc = if u.divine {
                t!("終わらぬ戦いを見かねて天より降りた天使。どの兵にも傷つけられない。", "An angel sent down from heaven, weary of the endless war. No soldier can hurt it.", "不忍见战争无休无止而从天而降的天使。任何士兵都无法伤其分毫。")
            } else {
                unit_desc(u.kind)
            };
            self.gfx.text_shadow(&title, tx, y + 2.0, tw, 28.0, Font::Big, tc);
            self.gfx.text(&lines, tx, y + 34.0, tw, 52.0, Font::Normal, tc);
            self.gfx.text(desc, x + 2.0, y + 106.0, wd, 38.0, Font::Small, [0.80, 0.76, 0.68, 1.0]);
        } else if self.sel.len() > 1 {
            let mut cnt = [0usize; N_KINDS];
            let mut pw = 0.0;
            let mut hp = 0.0;
            let mut mhp = 0.0;
            for &h in self.sel.iter() {
                let u = self.world.unit(h).unwrap();
                cnt[u.kind as usize] += 1;
                pw += u.power;
                hp += u.hp;
                mhp += u.max_hp();
            }
            let title = tf!("{} 部隊を編制選択中", "{} units selected", "已选择{}个单位", self.sel.len());
            self.gfx.text_shadow(&title, x, y, wd, 26.0, Font::Big, tc);
            let kinds = ALL_KINDS;
            let (mx, my) = self.mouse;
            let mut ix = x;
            for k in 0..N_KINDS {
                if cnt[k] > 0 && ix + 86.0 > x + wd {
                    break;
                }
                if cnt[k] > 0 {
                    let bx0 = ix;
                    let by0 = y + 28.0;
                    let bx1 = ix + 86.0;
                    let by1 = y + 72.0;
                    let hov = mx >= bx0 && mx <= bx1 && my >= by0 && my <= by1;
                    self.gfx.fill_round(bx0, by0, bx1, by1, 4.0, if hov { [0.35, 0.28, 0.16, 0.95] } else { [0.16, 0.12, 0.08, 0.9] });
                    self.gfx.stroke_round(bx0, by0, bx1, by1, 4.0, 1.2, [0.78, 0.64, 0.34, 0.9]);
                    if let Some(bmp) = self.art.national_unit_sprite(kinds[k], era, self.world.player, self.world.year_f(), false) {
                        self.gfx.draw_bitmap(bmp, bx0 + 3.0, by0 + 3.0, bx0 + 41.0, by1 - 3.0);
                    }
                    let ks = format!("×{}", cnt[k]);
                    self.gfx.text_shadow(&ks, bx0 + 44.0, by0 + 10.0, 40.0, 24.0, Font::Big, tc);
                    self.buttons.push(Button {
                        x0: bx0,
                        y0: by0,
                        x1: bx1,
                        y1: by1,
                        label: String::new(),
                        key: 0,
                        enabled: true,
                        act: Act::FilterKind(kinds[k]),
                        tip: tf!("{}（{}体）：クリックでこの兵種のみに絞り込む", "{} ({}): click to keep only this type", "{}（{}个）：点击只保留该兵种", kinds[k].name(era), cnt[k]),
                    });
                    ix += 94.0;
                }
            }
            let s = tf!("総能力 {:.1}（平均 {:.2}）　　総兵力 {:.0}/{:.0}", "Total power {:.1} (avg {:.2})    Total HP {:.0}/{:.0}", "总能力 {:.1}（平均 {:.2}）　　总兵力 {:.0}/{:.0}", pw, pw / self.sel.len() as f32, hp, mhp);
            self.gfx.text(&s, x, y + 76.0, wd, 30.0, Font::Normal, tc);
            let f = (hp / mhp.max(1.0)).clamp(0.0, 1.0);
            let bw = wd.min(320.0);
            self.gfx.fill_rect(x, y + 106.0, x + bw, y + 114.0, [0.08, 0.06, 0.05, 0.95]);
            self.gfx.fill_rect(x + 1.0, y + 107.0, x + 1.0 + (bw - 2.0) * f, y + 113.0, [1.0 - f, 0.25 + 0.7 * f, 0.15, 1.0]);
            self.gfx.stroke_rect(x, y + 106.0, x + bw, y + 114.0, 1.0, [0.75, 0.62, 0.34, 0.9]);
        } else if let Some(bh) = self.sel_b {
            let b = self.world.bld(bh).unwrap();
            let n = &self.world.nations[b.nation as usize];
            let nc = n.color;
            let is_divine_city = self.world.descent.is_some() && b.nation as usize == 2 && b.kind.city();
            let px0 = x + 2.0;
            let py0 = y + 4.0;
            let psz = 68.0;
            self.gfx.fill_rect(px0 - 2.0, py0 - 2.0, px0 + psz + 2.0, py0 + psz + 2.0, if is_divine_city { [0.95, 0.85, 0.45, 0.95] } else { c4(nc, 0.9) });
            let bld_bmp = if is_divine_city {
                self.art.divine_bld_sprite(b.kind, false).or_else(|| self.art.bld_icon(b.kind))
            } else {
                self.art.bld_icon(b.kind)
            };
            if let Some(bmp) = bld_bmp {
                self.gfx.draw_bitmap(bmp, px0, py0, px0 + psz, py0 + psz);
            }
            let f = (b.hp / b.kind.max_hp()).clamp(0.0, 1.0);
            self.gfx.fill_rect(px0, py0 + psz + 4.0, px0 + psz, py0 + psz + 11.0, [0.08, 0.06, 0.05, 0.95]);
            self.gfx.fill_rect(px0 + 1.0, py0 + psz + 5.0, px0 + 1.0 + (psz - 2.0) * f, py0 + psz + 10.0, [1.0 - f, 0.25 + 0.7 * f, 0.15, 1.0]);
            self.gfx.stroke_rect(px0, py0 + psz + 4.0, px0 + psz, py0 + psz + 11.0, 1.0, [0.75, 0.62, 0.34, 0.9]);

            let tx = x + 84.0;
            let tw = (wd - 84.0).max(100.0);
            let city = b.city_name().map(crate::lang::tx);
            let title = if is_divine_city {
                tf!("天使の{}・{}", "Angels' {} · {}", "天使的{}・{}", b.kind.name(), city.unwrap_or(""))
            } else if self.sel_bs.len() > 1 {
                // 何棟も選んでいるとき
                let kname = if b.kind.city() { t!("都市", "Cities", "城市") } else { b.kind.name() };
                tf!("{} ×{}（訓練はすべてに予約）　【{}】", "{} ×{} (training queues in each)　[{}]", "{} ×{}（训练将在每座中预约）　【{}】", kname, self.sel_bs.len(), crate::lang::tx(n.name))
            } else if let Some(c) = city {
                format!("{}（{}）　【{}】", c, b.kind.name(), crate::lang::tx(n.name))
            } else {
                format!("{}　【{}】", b.kind.name(), crate::lang::tx(n.name))
            };
            let mut s = tf!("耐久 {:.0}/{:.0}", "HP {:.0}/{:.0}", "耐久 {:.0}/{:.0}", b.hp.max(0.0), b.kind.max_hp());
            if !b.done() {
                let builders = self.world.bld_builders(bh);
                if builders > 0 {
                    s.push_str(&tf!("　（建設中 {:.0}%・作業民{}体）", "  (building {:.0}%, {} workers)", "　（建造中 {:.0}%・施工民{}名）", b.progress * 100.0, builders));
                } else {
                    s.push_str(&tf!("　（建設中断 {:.0}%・右の「建設を再開」で民を派遣）", "  (paused at {:.0}%: press Resume building to send workers)", "　（建造中断 {:.0}%・点右侧“继续建造”派民）", b.progress * 100.0));
                }
            }
            let desc = if is_divine_city {
                t!("天使に占領された都市。半年のうちに欧州のすべての都市が天使のものとなる。", "A city taken by the angels. Within half a year every city in Europe will be theirs.", "被天使占领的城市。半年之内欧洲所有城市都将归于天使。")
            } else {
                match b.kind {
                    BKind::Capital => t!("国の中心。民を訓練し、資源の集積地となる。失うと遷都し、全ての都市を失うと滅亡する。", "Heart of the state. Trains workers and stores resources. If it falls the capital moves; lose every city and the state falls.", "国家中心。训练民，存放资源。失去则迁都，失去全部城市则灭亡。"),
                    BKind::Town => t!("史実の都市。領土を広げ、民を訓練し、資源の集積地となる。新しい都市は築けない。都市を得ると人口の上限が上がる。", "A historical city. Expands territory, trains workers and stores resources. New cities cannot be founded; capturing cities raises your population limit.", "史实城市。扩张领土，训练民，存放资源。无法新建城市；夺取城市可提高人口上限。"),
                    BKind::Barracks => t!("民を徴兵して歩兵・砲兵・騎兵（国によっては突撃兵・山岳兵）を訓練する。戦車は兵を5人選んで割り当てる。", "Drafts workers into infantry, artillery and cavalry (and, for some states, stormtroopers or mountain troops). Tanks are made by assigning 5 selected soldiers.", "征民训练步兵、炮兵、骑兵（部分国家还有突击兵、山地兵）。坦克须选5名士兵编入。"),
                    BKind::Farm => t!("民が耕すと食糧を産む。", "Produces food when a worker farms it.", "民耕作后产粮食。"),
                    BKind::Fort => t!("塹壕陣地：近づく敵兵を撃ち、そばの味方の損害を減らす。数は都市の数で決まる上限まで。", "Trench line: shoots enemy soldiers that come near and shields nearby troops. How many you may build depends on your number of cities.", "堑壕阵地：射击靠近的敌兵并减少附近友军的伤害。数量上限取决于城市数。"),
                    BKind::Dock => t!("軍港：そばにいる兵を選んで Uボート（4人）・駆逐艦（6人）に割り当てる。", "Naval base: select soldiers near it and assign them to a U-boat (4) or destroyer (6).", "军港：选择附近的士兵编入U艇（4人）或驱逐舰（6人）。"),
                }
            };
            let queue: Vec<(UnitKind, f32)> = b.queue.iter().enumerate().map(|(i, k)| (*k, if i == 0 { b.queue_t / k.train_time() } else { 0.0 })).collect();
            self.gfx.text_shadow(&title, tx, y, tw, 28.0, Font::Big, tc);
            self.gfx.text(&s, tx, y + 30.0, tw, 24.0, Font::Normal, tc);
            self.gfx.text(desc, tx, y + 56.0, tw, 40.0, Font::Small, [0.80, 0.76, 0.68, 1.0]);
            for (i, (k, f)) in queue.iter().enumerate() {
                let qx = tx + i as f32 * 44.0;
                let qy = y + 98.0;
                self.gfx.fill_round(qx, qy, qx + 40.0, qy + 40.0, 3.0, [0.18, 0.14, 0.10, 1.0]);
                if let Some(ubmp) = self.art.national_unit_sprite(*k, era, self.world.player, self.world.year_f(), false) {
                    self.gfx.draw_bitmap(ubmp, qx + 2.0, qy + 2.0, qx + 38.0, qy + 38.0);
                }
                if *f > 0.0 {
                    self.gfx.fill_rect(qx + 2.0, qy + 34.0, qx + 2.0 + 36.0 * f, qy + 38.0, [0.35, 0.9, 0.4, 1.0]);
                }
            }
        } else {
            let n = &self.world.nations[pl];
            let nc = n.color;
            let is_divine = self.world.descent.is_some() && pl == 2;
            let px0 = x + 2.0;
            let py0 = y + 4.0;
            let psz = 68.0;
            self.gfx.fill_rect(px0 - 2.0, py0 - 2.0, px0 + psz + 2.0, py0 + psz + 2.0, if is_divine { [0.95, 0.85, 0.45, 0.95] } else { c4(nc, 0.9) });
            let cap_bmp = if is_divine {
                self.art.divine_bld_sprite(BKind::Capital, false).or_else(|| self.art.bld_icon(BKind::Capital))
            } else {
                self.art.bld_icon(BKind::Capital)
            };
            if let Some(bmp) = cap_bmp {
                self.gfx.draw_bitmap(bmp, px0, py0, px0 + psz, py0 + psz);
            }
            let tx = x + 84.0;
            let tw = (wd - 84.0).max(100.0);
            let title = if self.gfx.compact {
                tf!("{} の国勢", "State of {}", "{}国势", crate::lang::tx(n.name))
            } else {
                tf!("{} の国勢（右のボタンで民全体・全軍へ命令可）", "State of {} (the buttons on the right command all workers or all soldiers)", "{}国势（右侧按钮可命令全体民或全军）", crate::lang::tx(n.name))
            };
            let wars: Vec<&str> = (0..NATIONS.len()).filter(|&o| self.world.at_war(pl, o)).map(|o| crate::lang::tx(self.world.nations[o].name)).collect();
            let s = tf!(
                "{}　都市 {}　人口 {}/{}　世論 {:.0}　工業 ×{:.2}　生産力 ×{:.2}　上限 {:.1}\n交戦中：{}",
                "{}  Cities {}  Pop {}/{}  Opinion {:.0}  Industry ×{:.2}  Output ×{:.2}  Limit {:.1}\nAt war with: {}",
                "{}　城市 {}　人口 {}/{}　舆论 {:.0}　工业 ×{:.2}　生产力 ×{:.2}　上限 {:.1}\n交战中：{}",
                bloc_name(n.bloc),
                n.cities,
                n.workers + n.soldiers,
                self.world.pop_cap(pl),
                n.opinion,
                n.bias,
                self.world.era_mult(),
                cap,
                if wars.is_empty() {
                    t!("なし", "none", "无").to_string()
                } else if wars.len() > 8 {
                    format!("{}{}", wars[..8].join(sep()), tf!("ほか{}国", " +{} more", "等{}国", wars.len() - 8))
                } else {
                    wars.join(sep())
                }
            );
            self.gfx.text_shadow(&title, tx, y, tw, 26.0, Font::Big, tc);
            self.gfx.text(&s, tx, y + 28.0, tw, 48.0, Font::Normal, tc);

            // 自国の各兵種アイコンボタン（クリックでその兵種全体を選択）
            let mut cnt = [0usize; N_KINDS];
            for u in self.world.units.iter() {
                if u.alive && u.nation as usize == pl {
                    cnt[u.kind as usize] += 1;
                }
            }
            let kinds = ALL_KINDS;
            let (mx, my) = self.mouse;
            let mut ix = tx;
            for k in 0..N_KINDS {
                // 持っていない特別な兵種は出さない
                if cnt[k] == 0 && k > 3 {
                    continue;
                }
                if ix + 78.0 > x + wd {
                    break;
                }
                let bx0 = ix;
                let by0 = y + 82.0;
                let bx1 = ix + 78.0;
                let by1 = y + 122.0;
                let hov = mx >= bx0 && mx <= bx1 && my >= by0 && my <= by1;
                self.gfx.fill_round(bx0, by0, bx1, by1, 4.0, if hov { [0.35, 0.28, 0.16, 0.95] } else { [0.16, 0.12, 0.08, 0.9] });
                self.gfx.stroke_round(bx0, by0, bx1, by1, 4.0, 1.1, [0.78, 0.64, 0.34, 0.85]);
                if let Some(bmp) = self.art.national_unit_sprite(kinds[k], era, self.world.player, self.world.year_f(), false) {
                    self.gfx.draw_bitmap(bmp, bx0 + 3.0, by0 + 3.0, bx0 + 37.0, by1 - 3.0);
                }
                let ks = format!("×{}", cnt[k]);
                self.gfx.text_shadow(&ks, bx0 + 40.0, by0 + 8.0, 36.0, 22.0, Font::Normal, tc);
                self.buttons.push(Button {
                    x0: bx0,
                    y0: by0,
                    x1: bx1,
                    y1: by1,
                    label: String::new(),
                    key: 0,
                    enabled: cnt[k] > 0,
                    act: Act::SelectKind(kinds[k]),
                    tip: tf!("{}全体（{}体）を選択する", "Select all {} ({})", "选择全部{}（{}个）", kinds[k].name(era), cnt[k]),
                });
                ix += 84.0;
            }
        }
    }

    fn render_commands(&mut self, x: f32, y: f32) {
        let pl = self.world.player;
        let era = self.world.era;
        let (bw, bh, gap) = (122.0, 46.0, 4.0);
        let pos = |i: usize| -> (f32, f32, f32, f32) {
            let c = (i % 3) as f32;
            let r = (i / 3) as f32;
            let x0 = x + c * (bw + gap);
            let y0 = y + r * (bh + gap);
            (x0, y0, x0 + bw, y0 + bh)
        };
        let res = self.world.nations[pl].res;
        let afford = |c: [f32; 3]| res[0] >= c[0] && res[1] >= c[1] && res[2] >= c[2];
        let own = self.own_sel();
        let (n_workers, n_soldiers) = (self.world.nations[pl].workers, self.world.nations[pl].soldiers);
        let n_army = self.all_military().len();
        let n_trainable = n_soldiers;
        if !own.is_empty() {
            let any_worker = own.iter().any(|&h| self.world.unit(h).unwrap().kind == UnitKind::Worker);
            let any_mil = own.iter().any(|&h| self.world.unit(h).unwrap().kind.military());
            let mut i = 0;
            if any_worker {
                let can_dock = !naval_kinds(pl).is_empty();
                for (k, key, tip) in [
                    (BKind::Farm, 'Q', t!("農場：民が耕すと食糧を産む。自国領内に建設。", "Farm: produces food when farmed. Build inside your territory.", "农场：民耕作后产粮食。建于己方领土内。")),
                    (BKind::Barracks, 'W', t!("兵営：民を徴兵して兵にする。自国領内に建設。", "Barracks: drafts workers into soldiers. Build inside your territory.", "兵营：征民为兵。建于己方领土内。")),
                    (BKind::Dock, 'E', if can_dock { t!("軍港：そばの兵を Uボート・駆逐艦に割り当てられる。海に面した自国領内に建設。", "Naval base: soldiers near it can crew U-boats and destroyers. Build on your coast.", "军港：附近的士兵可编入U艇、驱逐舰。建于临海的己方领土内。") } else { t!("軍港：この国は軍艦を造れない（兵は港で自動的に船に乗る）", "Naval base: this state cannot build warships (troops board ships at ports automatically)", "军港：该国无法建造军舰（士兵会在港口自动登船）") }),
                ] {
                    let (x0, y0, x1, y1) = pos(i);
                    let c = k.cost();
                    let ok = afford(c) && (k != BKind::Dock || can_dock);
                    self.add_button(x0, y0, x1, y1, k.name(), &Self::cost_str(c), key as u32, ok, Act::Place(k), tip);
                    i += 1;
                }
            }
            // 鍛錬
            let u0 = self.world.unit(own[0]).unwrap();
            let u0_kind = u0.kind;
            let up_cost = self.world.upgrade_cost(u0);
            let label = if any_mil && !any_worker { t!("訓練", "Train", "训练") } else if any_worker && !any_mil { t!("教化", "Educate", "教化") } else { t!("鍛錬", "Train", "锻炼") };
            let (x0, y0, x1, y1) = pos(i);
            self.add_button(x0, y0, x1, y1, label, &Self::cost_str(up_cost), 'T' as u32, true, Act::Upgrade, t!("選択部隊の能力を +0.5 高める（自国領内のみ）。兵は戦闘力と兵力、民は採集量が上がる。費用は現在の能力に比例し、食糧・木材・鉄のうち蓄えの多いものから優先して払う。", "Raises the selected units' power by +0.5 (inside your territory only). Soldiers gain strength and HP, workers gather more. The cost grows with power and is paid mostly from whichever of food, wood and iron you have most of.", "使所选单位能力+0.5（仅限己方领土内）。士兵提升战斗力与兵力，民提升采集量。费用与当前能力成正比，优先从粮食、木材、铁中储量较多者支付。"));
            i += 1;
            let (x0, y0, x1, y1) = pos(i);
            let (cl, ct) = if any_mil && !any_worker {
                (t!("動員解除→民", "Demobilize", "复员→民"), t!("兵を動員解除して民に戻す（能力は2/3に目減り）。経済は戻るが軍は減る。", "Demobilize soldiers back into workers (power drops to 2/3). The economy recovers but the army shrinks.", "将士兵复员为民（能力降为2/3）。经济恢复但军队减少。"))
            } else if any_worker && !any_mil {
                (t!("徴兵→歩兵", "Draft", "征兵→步兵"), t!("民をその場で徴兵して歩兵にする（能力は2/3に目減り）。兵営なら装備の整った兵を育てられる。", "Draft workers on the spot as infantry (power drops to 2/3). Barracks produce better-equipped soldiers.", "就地征民为步兵（能力降为2/3）。在兵营可训练装备更好的士兵。"))
            } else {
                (t!("兵⇔民", "Convert", "兵⇔民"), t!("兵は民に、民は歩兵に転換する（能力は2/3に目減り）。", "Soldiers become workers and workers become infantry (power drops to 2/3).", "士兵转为民，民转为步兵（能力降为2/3）。"))
            };
            self.add_button(x0, y0, x1, y1, cl, t!("能力×2/3", "power ×2/3", "能力×2/3"), 'C' as u32, true, Act::Convert, ct);
            i += 1;
            let (x0, y0, x1, y1) = pos(i);
            if any_worker && !any_mil {
                // 民だけを選んでいるときは、停止の代わりに要塞
                let (nf, cap) = (self.world.fort_count(pl), self.world.fort_cap(pl));
                let c = BKind::Fort.cost();
                let tip = tf!(
                    "塹壕陣地：近づく敵兵を撃ち（射程{:.0}）、そばの味方の兵が受ける損害を3割減らす。自国領内に建設。数は4＋都市1つにつき2つ、最大60（いま{}／{}）。",
                    "Trench line: shoots enemy soldiers that come near (range {:.0}) and cuts damage to friendly troops beside it by 30%. Build inside your territory. Up to 4 plus 2 per city, at most 60 (now {}/{}).",
                    "堑壕阵地：射击靠近的敌兵（射程{:.0}），并使附近友军受到的伤害减少三成。建于己方领土内。数量为4＋每座城市2座，最多60（现有{}／{}）。",
                    FORT_RANGE, nf, cap
                );
                let lbl = tf!("塹壕 {}/{}", "Trench {}/{}", "堑壕 {}/{}", nf, cap);
                self.add_button(x0, y0, x1, y1, &lbl, &Self::cost_str(c), 'R' as u32, afford(c) && nf < cap, Act::Place(BKind::Fort), &tip);
            } else {
                self.add_button(x0, y0, x1, y1, t!("停止", "Stop", "停止"), "", 'S' as u32, true, Act::Stop, t!("選択中の部隊を停止して待機させる。", "Stop the selected units and make them wait.", "让所选单位停止待命。"));
            }
            i += 1;

            if any_worker && !any_mil {
                // 下段：選択民および民全体への一括命令
                let (x0, y0, x1, y1) = pos(6);
                self.add_button(x0, y0, x1, y1, t!("自動採集", "Auto gather", "自动采集"), t!("農場・森・鉱山へ", "auto-assign", "农场・森・矿"), 'F' as u32, true, Act::WorkersAuto, t!("選択中の民を、空いている農場・森・鉱山へバランスよく自動配分する。", "Spread the selected workers evenly over free farms, forests and mines.", "将所选的民均衡分配到空闲的农场、森林、矿山。"));
                let all_sel = own.len() >= n_workers as usize;
                let (x0, y0, x1, y1) = pos(7);
                if all_sel {
                    self.add_button(x0, y0, x1, y1, t!("全民・伐採", "All: wood", "全民・伐木"), t!("最寄りの森へ", "nearest forest", "去最近的森林"), 'G' as u32, true, Act::AllWorkersWood, t!("すべての民（建設中を除く）に最寄りの森の伐採を命じる。", "Order every worker (except builders) to cut the nearest forest.", "命令全部民（建造中除外）砍伐最近的森林。"));
                } else {
                    self.add_button(x0, y0, x1, y1, t!("全民を選択", "All workers", "选择全民"), &tf!("全{}体", "all {}", "全{}名", n_workers), 'G' as u32, n_workers > 0, Act::SelectAllWorkers, t!("自国のすべての民を選択する。", "Select all your workers.", "选择我国全部民。"));
                }
                let (x0, y0, x1, y1) = pos(8);
                if all_sel {
                    self.add_button(x0, y0, x1, y1, t!("全民・採掘", "All: iron", "全民・采矿"), t!("最寄りの鉱山", "nearest mine", "最近的矿山"), 'V' as u32, true, Act::AllWorkersMine, t!("すべての民（建設中を除く）に最寄りの鉱山の採掘を命じる。", "Order every worker (except builders) to mine the nearest mine.", "命令全部民（建造中除外）开采最近的矿山。"));
                } else {
                    self.add_button(x0, y0, x1, y1, t!("全民・自動", "All: auto", "全民・自动"), t!("民全体を配分", "assign everyone", "分配全部民"), 'V' as u32, n_workers > 0, Act::AllWorkersAuto, t!("自国のすべての民（建設中を除く）を農場・森・鉱山へ自動でバランス配分する。", "Spread all your workers (except builders) evenly over farms, forests and mines.", "将我国全部民（建造中除外）均衡分配到农场、森林、矿山。"));
                }
            } else if any_mil && !any_worker {
                let (x0, y0, x1, y1) = pos(i);
                self.add_button(x0, y0, x1, y1, t!("進軍", "Attack-move", "进军"), t!("攻撃移動", "fight on the way", "攻击移动"), 'A' as u32, true, Act::AttackMove, t!("クリックした地点へ、途中の敵と戦いながら進む。", "March to the clicked point, fighting enemies on the way.", "向点击的地点进军，沿途与敌交战。"));
                i += 1;
                let (x0, y0, x1, y1) = pos(i);
                self.add_button(x0, y0, x1, y1, t!("防衛・帰還", "Defend", "防卫・归还"), t!("領内迎撃・集結", "guard cities", "领内迎击・集结"), 'D' as u32, true, Act::ArmyDefend, t!("領内の侵入者を迎え撃ち、平時は最寄りの自国の都市へ帰還・防衛配置する。", "Intercept intruders in your land; in peacetime return to the nearest city and guard it.", "迎击领内入侵者，平时返回最近的己方城市防守。"));
                i += 1;
                let (x0, y0, x1, y1) = pos(i);
                self.add_button(x0, y0, x1, y1, t!("同兵種選択", "Same type", "选择同兵种"), u0_kind.name(era), 'F' as u32, true, Act::SelectKind(u0_kind), t!("自国の同じ兵種のユニットをすべて選択する。", "Select all your units of the same type.", "选择我国所有同兵种单位。"));
                // 下段：全軍（軍事ユニット全体）への命令
                let (x0, y0, x1, y1) = pos(6);
                self.add_button(x0, y0, x1, y1, t!("全軍を選択", "All soldiers", "选择全军"), &tf!("全{}体", "all {}", "全{}个", n_army), 'G' as u32, n_army > 0, Act::SelectAllMilitary, t!("自国のすべての軍事ユニットを選択する。", "Select all your soldiers.", "选择我国全部军事单位。"));
                let (x0, y0, x1, y1) = pos(7);
                self.add_button(x0, y0, x1, y1, t!("全軍・進撃", "All: attack", "全军・进击"), t!("全軍で敵陣へ", "whole army", "全军攻敌"), 'Q' as u32, n_army > 0, Act::AllArmyAttack, t!("全軍事ユニットを選択し、交戦国の都市・敵部隊へ一斉進撃（または地図クリックで進軍先指定）する。海の向こうへは港で船に乗る。", "Select every soldier and advance on enemy cities and armies (or click the map to choose where). Troops board ships at ports to cross the sea.", "选择全部军事单位，向交战国城市、敌军全面进击（或点击地图指定目标）。渡海时在港口登船。"));
                let (x0, y0, x1, y1) = pos(8);
                self.add_button(x0, y0, x1, y1, t!("全軍・防衛", "All: defend", "全军・防卫"), t!("全軍で防衛", "whole army", "全军防守"), 'W' as u32, n_army > 0, Act::AllArmyDefend, t!("全軍事ユニットに領内侵入者の迎撃および都市への防衛帰還を命じる。", "Order every soldier to intercept intruders and return to defend the cities.", "命令全部军事单位迎击入侵者并返回城市防守。"));
            } else {
                let (x0, y0, x1, y1) = pos(i);
                self.add_button(x0, y0, x1, y1, t!("進軍", "Attack-move", "进军"), t!("攻撃移動", "fight on the way", "攻击移动"), 'A' as u32, true, Act::AttackMove, t!("クリックした地点へ、途中の敵と戦いながら進む。", "March to the clicked point, fighting enemies on the way.", "向点击的地点进军，沿途与敌交战。"));
                let (x0, y0, x1, y1) = pos(7);
                self.add_button(x0, y0, x1, y1, t!("全民を選択", "All workers", "选择全民"), &tf!("全{}体", "all {}", "全{}名", n_workers), 'G' as u32, n_workers > 0, Act::SelectAllWorkers, t!("自国のすべての民を選択する。", "Select all your workers.", "选择我国全部民。"));
                let (x0, y0, x1, y1) = pos(8);
                self.add_button(x0, y0, x1, y1, t!("全軍を選択", "All soldiers", "选择全军"), &tf!("全{}体", "all {}", "全{}个", n_army), 'V' as u32, n_army > 0, Act::SelectAllMilitary, t!("自国のすべての軍事ユニットを選択する。", "Select all your soldiers.", "选择我国全部军事单位。"));
            }
        } else if let Some(bh) = self.sel_b {
            let b = self.world.bld(bh).unwrap();
            if b.nation as usize != pl {
                return;
            }
            let (kind, done) = (b.kind, b.done());
            let mut i = 0;
            if done {
                let mut list: Vec<UnitKind> = match kind {
                    BKind::Capital | BKind::Town => vec![UnitKind::Worker],
                    BKind::Barracks => vec![UnitKind::Infantry, UnitKind::Artillery, UnitKind::Cavalry],
                    BKind::Dock | BKind::Farm | BKind::Fort => vec![],
                };
                if kind == BKind::Barracks {
                    for &(k, _) in special_kinds(pl) {
                        if !k.weapon() {
                            list.push(k);
                        }
                    }
                }
                let keys = ['Q', 'W', 'E', 'R', 'F', 'G'];
                let has_workers = n_workers > 0;
                for k in list {
                    let (x0, y0, x1, y1) = pos(i);
                    let c = self.world.train_cost(pl, k);
                    let avail = self.world.kind_available(pl, k);
                    let from = special_kinds(pl).iter().find(|x| x.0 == k).map(|x| x.1).unwrap_or(0.0);
                    let mut tip = tf!("{}\n訓練時間 {:.0}秒　維持費 {}", "{}\nTraining time {:.0}s   Upkeep {}", "{}\n训练时间 {:.0}秒　维持费 {}", unit_desc(k), k.train_time(), if k.military() { t!("食糧0.07/秒×能力", "0.07 food/s × power", "粮食0.07/秒×能力") } else { t!("なし", "none", "无") });
                    if k.military() {
                        tip.push_str(t!("\n民を1人徴兵する（その分だけ生産が減る）", "\nDrafts one worker (production drops accordingly)", "\n征召一名民（生产相应减少）"));
                    }
                    if !avail {
                        let y = from.floor() as i32;
                        let m = ((from - y as f32) * 12.0).floor() as i32 + 1;
                        tip.push_str(&tf!("\n{}年{}月から造れる", "\nAvailable from {}/{}", "\n{}年{}月起可建造", y, m));
                    }
                    let ok = afford(c) && avail && (!k.military() || has_workers);
                    self.add_button(x0, y0, x1, y1, k.name(era), &Self::cost_str(c), keys[i.min(5)] as u32, ok, Act::Train(k), &tip);
                    i += 1;
                }
            } else {
                let (x0, y0, x1, y1) = pos(0);
                self.add_button(
                    x0,
                    y0,
                    x1,
                    y1,
                    t!("建設を再開", "Resume building", "继续建造"),
                    t!("民を派遣", "send workers", "派遣民"),
                    'Q' as u32,
                    n_workers > 0,
                    Act::ResumeBuild,
                    t!("近くの民をこの建物の建設へ派遣し、中断した建設を再開（または作業人数を追加）する（Q）", "Send nearby workers to build this, resuming a paused build or adding hands (Q)", "派附近的民来建造此建筑，继续中断的工程或增派人手（Q）"),
                );
            }
            if !kind.city() || !done {
                let (x0, y0, x1, y1) = pos(8);
                self.add_button(x0, y0, x1, y1, t!("取り壊す", "Demolish", "拆除"), "", 0, true, Act::Demolish, t!("この建物を取り壊す（Delete）", "Demolish this building (Delete)", "拆除此建筑（Delete）"));
            }
        } else {
            // 未選択時：民全体・軍事ユニット全体に対する全体命令パネル
            let (x0, y0, x1, y1) = pos(0);
            self.add_button(x0, y0, x1, y1, t!("全民・自動", "All: auto", "全民・自动"), t!("農場・森・鉱山へ", "auto-assign", "农场・森・矿"), 'Q' as u32, n_workers > 0, Act::AllWorkersAuto, t!("自国のすべての民（建設中を除く）を、空いている農場・近くの森・鉱山へバランスよく自動配分する。", "Spread all your workers (except builders) evenly over free farms and nearby forests and mines.", "将我国全部民（建造中除外）均衡分配到空闲的农场、附近的森林与矿山。"));
            let (x0, y0, x1, y1) = pos(1);
            self.add_button(x0, y0, x1, y1, t!("全民・伐採", "All: wood", "全民・伐木"), t!("最寄りの森へ", "nearest forest", "去最近的森林"), 'W' as u32, n_workers > 0, Act::AllWorkersWood, t!("自国のすべての民（建設中を除く）に、最寄りの森の伐採を命じる。", "Order all your workers (except builders) to cut the nearest forest.", "命令我国全部民（建造中除外）砍伐最近的森林。"));
            let (x0, y0, x1, y1) = pos(2);
            self.add_button(x0, y0, x1, y1, t!("全民・採掘", "All: iron", "全民・采矿"), t!("最寄りの鉱山", "nearest mine", "最近的矿山"), 'E' as u32, n_workers > 0, Act::AllWorkersMine, t!("自国のすべての民（建設中を除く）に、最寄りの鉱山の採掘を命じる。", "Order all your workers (except builders) to mine the nearest mine.", "命令我国全部民（建造中除外）开采最近的矿山。"));

            let (x0, y0, x1, y1) = pos(3);
            self.add_button(x0, y0, x1, y1, t!("全民・教化", "All: educate", "全民・教化"), t!("能力+0.5", "power +0.5", "能力+0.5"), 'T' as u32, n_workers > 0, Act::AllWorkersUpgrade, t!("自国領内の民を、能力の低い順に資源が続く限り一斉に教化する。", "Educate the workers in your territory, weakest first, as far as resources last.", "按能力由低到高，在资源允许范围内一并教化己方领土内的民。"));
            let (x0, y0, x1, y1) = pos(4);
            self.add_button(x0, y0, x1, y1, t!("全民を選択", "All workers", "选择全民"), &tf!("全{}体", "all {}", "全{}名", n_workers), 'G' as u32, n_workers > 0, Act::SelectAllWorkers, t!("自国のすべての民を選択する（建設や一斉移動を行えます）。", "Select all your workers (to build or move them together).", "选择我国全部民（可进行建造或统一移动）。"));
            let (x0, y0, x1, y1) = pos(5);
            self.add_button(x0, y0, x1, y1, t!("全軍を選択", "All soldiers", "选择全军"), &tf!("全{}体", "all {}", "全{}个", n_army), 'V' as u32, n_army > 0, Act::SelectAllMilitary, t!("自国のすべての軍事ユニットを選択する。", "Select all your soldiers.", "选择我国全部军事单位。"));

            let (x0, y0, x1, y1) = pos(6);
            self.add_button(x0, y0, x1, y1, t!("全軍・進撃", "All: attack", "全军・进击"), t!("敵城・目標へ", "enemy cities", "敌城・目标"), 'A' as u32, n_army > 0, Act::AllArmyAttack, t!("全軍事ユニットを選択し、交戦国の最寄りの都市・敵部隊へ総進撃（または地図クリックで進軍先指定）する。", "Select every soldier and advance on the nearest enemy city and armies (or click the map to choose where).", "选择全部军事单位，向交战国最近的城市、敌军总攻（或点击地图指定目标）。"));
            let (x0, y0, x1, y1) = pos(7);
            self.add_button(x0, y0, x1, y1, t!("全軍・防衛", "All: defend", "全军・防卫"), t!("領内迎撃・帰還", "guard cities", "领内迎击・归还"), 'D' as u32, n_army > 0, Act::AllArmyDefend, t!("全軍事ユニットに、領内の侵入者の迎撃または自国の都市への防衛帰還を命じる。", "Order every soldier to intercept intruders or return to defend your cities.", "命令全部军事单位迎击入侵者或返回己方城市防守。"));
            let (x0, y0, x1, y1) = pos(8);
            self.add_button(x0, y0, x1, y1, t!("全軍・訓練", "All: train", "全军・训练"), t!("能力+0.5", "power +0.5", "能力+0.5"), 'R' as u32, n_trainable > 0, Act::AllArmyUpgrade, t!("自国領内の軍事ユニットを、能力の低い順に資源が続く限り一斉に訓練する。", "Train the soldiers in your territory, weakest first, as far as resources last.", "按能力由低到高，在资源允许范围内一并训练己方领土内的军事单位。"));
        }
    }

    fn render_diplo(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        let pw = 960.0f32.min(w - 20.0);
        let ph = 620.0f32.min(h - 60.0);
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.45]);
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        if let Some(ic) = &self.art.cmd_icons[CmdIcon::Diplo as usize] {
            self.gfx.draw_bitmap(ic, x0 + 16.0, y0 + 8.0, x0 + 48.0, y0 + 40.0);
        }
        self.gfx.text_shadow(t!("外交", "Diplomacy", "外交"), x0 + 56.0, y0 + 10.0, 200.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        let mine = self.world.nations[pl].bloc;
        let s = tf!("自国：{}（{}）　同盟国以外をすべて打ち破れば勝利（残り{}国）", "Our side: {} ({})   Defeat everyone outside your alliance to win ({} left)", "我方：{}（{}）　击败同盟以外的所有国家即获胜（剩余{}国）", crate::lang::tx(self.world.nations[pl].name), bloc_name(mine), self.world.enemies_left(pl));
        self.gfx.text(&s, x0 + 160.0, y0 + 16.0, pw - 370.0, 24.0, Font::Normal, [0.85, 0.82, 0.75, 1.0]);
        self.add_button(x0 + pw - 90.0, y0 + 10.0, x0 + pw - 14.0, y0 + 40.0, t!("閉じる", "Close", "关闭"), "", 0, true, Act::CloseOverlay, "");
        self.add_button(x0 + pw - 196.0, y0 + 10.0, x0 + pw - 98.0, y0 + 40.0, t!("戦史 F3", "Chronicle F3", "战史 F3"), "", 0, true, Act::Chron, t!("開戦・参戦・戦い・政変などの記録と、各国の君主家・政体", "Records of declarations, battles and upheavals, and each state's ruling house", "宣战、参战、战役、政变等记录及各国君主家与政体"));
        self.art.draw_inset_box(&self.gfx, x0 + 10.0, y0 + 48.0, x0 + pw - 10.0, y0 + ph - 10.0);
        let mut top = y0 + 54.0;
        // ドイツ：無制限潜水艦作戦
        if pl == GER {
            let on = self.world.sea.unrestricted;
            let lbl = if on { t!("無制限潜水艦作戦：実施中（やめる）", "Unrestricted U-boat war: ON (stop)", "无限制潜艇战：进行中（停止）") } else { t!("無制限潜水艦作戦を始める", "Begin unrestricted U-boat warfare", "开始无限制潜艇战") };
            let tip = tf!("Uボートが中立の米国の船も沈めるようになる。英国の補給を断てるが、米国の参戦気運が高まる（いま {:.0}/100、100 で参戦）。", "U-boats will sink neutral American ships too. It can starve Britain, but raises America's war fever (now {:.0}/100; at 100 America enters the war).", "U艇将连中立美国的船只也一并击沉。可切断英国补给，但会提高美国的参战情绪（现为 {:.0}/100，达到100即参战）。", self.world.sea.us_anger);
            self.add_button(x0 + 18.0, top, x0 + 380.0, top + 30.0, lbl, "", 0, true, Act::Unrestricted, &tip);
            top += 36.0;
        }
        let home = self.world.nations[pl].home;
        let mut list: Vec<usize> = (0..NATIONS.len()).filter(|&i| i != pl && self.world.nations[i].alive).collect();
        // 敵・味方・中立の順、その中は近い順
        let rank = |w: &World, o: usize| if w.at_war(pl, o) { 0 } else if w.allied(pl, o) { 1 } else { 2 };
        list.sort_by(|&a, &b| rank(&self.world, a).cmp(&rank(&self.world, b)).then(self.world.nations[a].home.dist(home).partial_cmp(&self.world.nations[b].home.dist(home)).unwrap()));
        let rowh = 25.0;
        let rows_per = ((y0 + ph - 16.0 - top) / rowh).max(1.0) as usize;
        let cols = if list.len() <= rows_per { 1 } else { 2 };
        let colw = (pw - 30.0) / cols as f32;
        for (k, &o) in list.iter().enumerate() {
            let c = k / rows_per;
            if c >= cols {
                break;
            }
            let r = k % rows_per;
            let rx = x0 + 15.0 + c as f32 * colw;
            let ry = top + r as f32 * rowh;
            let n = &self.world.nations[o];
            let war = self.world.at_war(pl, o);
            let ally = self.world.allied(pl, o);
            if r % 2 == 0 {
                self.gfx.fill_rect(rx, ry, rx + colw - 8.0, ry + rowh, [1.0, 1.0, 1.0, 0.04]);
            }
            self.gfx.fill_rect(rx + 4.0, ry + 5.0, rx + 18.0, ry + 19.0, c4(n.color, 1.0));
            self.gfx.text(crate::lang::tx(n.name), rx + 24.0, ry + 3.0, 150.0, 20.0, Font::Normal, [0.95, 0.92, 0.85, 1.0]);
            let st = tf!("{} 都市{} 世論{:.0}", "{} Cities {} Op {:.0}", "{} 城市{} 舆论{:.0}", bloc_name(n.bloc), n.cities, n.opinion);
            self.gfx.text(&st, rx + 176.0, ry + 5.0, 150.0, 20.0, Font::Small, [0.75, 0.72, 0.66, 1.0]);
            let mine_bloc = self.world.nations[pl].bloc;
            let (rel, rc) = if war {
                (t!("交戦中", "At war", "交战中"), [1.0, 0.45, 0.35, 1.0])
            } else if ally {
                (t!("同盟", "Allied", "同盟"), [0.6, 0.9, 1.0, 1.0])
            } else if n.bloc != Bloc::Neutral && mine_bloc != Bloc::Neutral {
                (t!("敵陣営（未交戦）", "Enemy side (not yet at war)", "敌方阵营（尚未交战）"), [1.0, 0.7, 0.5, 1.0])
            } else {
                (t!("中立", "Neutral", "中立"), [0.75, 0.75, 0.7, 1.0])
            };
            self.gfx.text(rel, rx + 330.0, ry + 4.0, (colw - 420.0).max(80.0), 20.0, Font::Small, rc);
            let bx = rx + colw - 82.0;
            if !war && !ally {
                let tip = t!("宣戦する。中立国を攻めると、その国は敵の陣営に加わる", "Declare war. A neutral state you attack joins the enemy side", "宣战。进攻中立国会使其加入敌方阵营");
                self.add_button(bx, ry + 2.0, bx + 70.0, ry + rowh - 2.0, t!("宣戦", "War", "宣战"), "", 0, self.world.descent.is_none(), Act::War(o), tip);
            }
        }
    }

    /// 和睦の申し入れ：受けるか退けるかを選ぶ（返答するまで時間は止まる）
    fn render_peace_ask(&mut self) {
        let Some(q) = self.world.peace_asks.front().cloned() else { return };
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        let o = q.from;
        let pw = 520.0f32.min(w - 20.0);
        let ph = 250.0;
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0 - 40.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.35]);
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        if let Some(ic) = &self.art.cmd_icons[CmdIcon::Diplo as usize] {
            self.gfx.draw_bitmap(ic, x0 + 16.0, y0 + 8.0, x0 + 48.0, y0 + 40.0);
        }
        self.gfx.text_shadow(t!("和睦の申し入れ", "Peace offered", "议和请求"), x0 + 56.0, y0 + 10.0, 260.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        let rest = self.world.peace_asks.len();
        if rest > 1 {
            let s = tf!("ほか{}件", "{} more", "另有{}件", rest - 1);
            self.gfx.text(&s, x0 + pw - 110.0, y0 + 18.0, 96.0, 20.0, Font::Small, [0.8, 0.78, 0.7, 1.0]);
        }
        self.art.draw_inset_box(&self.gfx, x0 + 12.0, y0 + 48.0, x0 + pw - 12.0, y0 + ph - 62.0);
        let n = &self.world.nations[o];
        self.gfx.fill_rect(x0 + 24.0, y0 + 62.0, x0 + 40.0, y0 + 78.0, c4(n.color, 1.0));
        let s = tf!("{} が和睦を申し入れてきた（{}）", "{} offers peace ({})", "{}提出议和（{}）", crate::lang::tx(n.name), q.why);
        self.gfx.text(&s, x0 + 48.0, y0 + 58.0, pw - 70.0, 24.0, Font::Normal, [0.95, 0.92, 0.85, 1.0]);
        let dur = (self.world.years - self.world.war_start[pl][o]).max(0.0);
        let s = tf!(
            "交戦{}年　国力：自国 {:.0}　{} {:.0}\n受諾すると{}年間休戦し、その間は互いに宣戦できない。\n退けると交戦を続ける。",
            "At war for {} years.  Strength: us {:.0}, {} {:.0}\nAccepting brings a {}-year truce in which neither side may declare war.\nRefusing continues the war.",
            "交战{}年　国力：我国 {:.0}　{} {:.0}\n接受则休战{}年，其间双方均不能宣战。\n拒绝则继续交战。",
            dur.floor() as i32,
            self.world.nations[pl].strength,
            crate::lang::tx(n.name),
            n.strength,
            q.years as i32
        );
        self.gfx.text(&s, x0 + 24.0, y0 + 88.0, pw - 48.0, 96.0, Font::Small, [0.82, 0.8, 0.74, 1.0]);
        let bw = (pw - 60.0) / 2.0;
        let by = y0 + ph - 52.0;
        let lbl = tf!("受諾（休戦{}年）", "Accept ({}-year truce)", "接受（休战{}年）", q.years as i32);
        self.add_button(x0 + 20.0, by, x0 + 20.0 + bw, by + 38.0, &lbl, "", 'Y' as u32, true, Act::PeaceAnswer(true), t!("和睦して休戦する（Y）", "Make peace and agree a truce (Y)", "议和并休战（Y）"));
        self.add_button(x0 + 40.0 + bw, by, x0 + 40.0 + 2.0 * bw, by + 38.0, t!("拒否（交戦を続ける）", "Refuse (keep fighting)", "拒绝（继续交战）"), "", 'N' as u32, true, Act::PeaceAnswer(false), t!("申し入れを退けて戦を続ける（N）", "Refuse the offer and keep fighting (N)", "拒绝请求，继续作战（N）"));
    }

    fn render_help(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pw = 1180.0f32.min(w - 20.0);
        let ph = (h - 20.0).max(300.0);
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.45]);
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        if let Some(ic) = &self.art.cmd_icons[CmdIcon::Help as usize] {
            self.gfx.draw_bitmap(ic, x0 + 16.0, y0 + 8.0, x0 + 48.0, y0 + 40.0);
        }
        self.gfx.text_shadow(t!("遊び方", "How to Play", "玩法"), x0 + 56.0, y0 + 10.0, 220.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        self.add_button(x0 + pw - 90.0, y0 + 10.0, x0 + pw - 14.0, y0 + 40.0, t!("閉じる", "Close", "关闭"), "", 0, true, Act::CloseOverlay, "");
        #[cfg(target_arch = "wasm32")]
        self.add_button(x0 + pw - 310.0, y0 + 10.0, x0 + pw - 100.0, y0 + 40.0,
            t!("キャッシュを更新して再読み込み", "Refresh cache and reload", "刷新缓存并重新加载"),
            "", 0, true, Act::RefreshCache, "");
        self.art.draw_inset_box(&self.gfx, x0 + 14.0, y0 + 46.0, x0 + pw - 14.0, y0 + ph - 14.0);
        let text = t!(
            "■ 目的　1914年6月28日、サラエボ事件から大戦が始まる。同盟国以外の国（あなたと戦っている敵国）をすべて打ち破れば勝利。1919年6月28日（ヴェルサイユ条約の日）までに勝者が現れなければ、天使が降りて半年のうちにすべての都市を占領し、ゲームは終わる。時間は全ての国で共通に流れる（1か月＝100秒）。\n■ 開戦　七月危機の宣戦布告（オーストリア→セルビア、ドイツ→ロシア・フランス・ベルギー、イギリス→ドイツ……）は史実の日付に自動で起こる。オスマン帝国・イタリア・ブルガリア・ポルトガル・ルーマニア・ギリシャも史実の日付に参戦する。陣営どうしの和睦はない。中立国に宣戦すると、その国は敵の陣営に加わる。\n\n■ 資源　食糧（農場を耕す）・木材（森を伐採）・鉄（鉱山を採掘）。民が集めて都市へ運ぶ。都市からも税収がある。生産力も資源も開戦時がもっとも豊かで、戦争が長引くほど減っていく。\n■ 人口と徴兵　民と兵を合わせた人口には上限がある（敵の都市を奪えば増え、失えば減る）。兵は民を徴兵してつくる（兵営で兵を1人訓練するたびに民が1人減る）。軍を大きくするほど経済は細る。兵を動員解除して民に戻すこともできる（能力は2/3に目減り）。軍事ユニットは存在するだけで食糧を消費する（維持費）。\n■ 都市　都市は史実のものだけで、新しく築くことはできない。全ての都市を失った国は滅亡する。\n\n■ 世論　戦場での損失が相手より多い、人口に対して兵が多すぎる（軍事費が重い）、食糧が尽きた、都市を失った……といった状況が続くと世論が下がる。帝政の国（ドイツ・オーストリア・オスマン・ロシア・ブルガリア）では世論が 0 近くまで落ちると革命が起き、その国は降伏する（都市は敵国に明け渡される）。民主制の国では革命は起きないが、世論が 50 を割るとストライキで生産が落ちる。\n■ 兵種　歩兵（小銃）・砲兵（遠距離、歩兵と建物に強い）・騎兵（速い、砲兵に強い）はどの国も造れる。国ごとに特別な兵がある：戦車（英国1916年9月〜・フランス1917年4月〜・米国1918年〜）、突撃兵（ドイツ1916年7月〜）、山岳兵（イタリア・オーストリア・フランス・スイス）、Uボート（ドイツ・オーストリア）、駆逐艦（英・仏・伊・露・米）。戦車・Uボート・駆逐艦は兵器で、兵を選んで「〜に割り当て」（B／U）を押すと造れる（戦車は兵5人・Uボートは4人・駆逐艦は6人。戦車は自国の領内で、軍艦は自国の軍港のそばで）。兵は兵器に変わり、兵には戻せない。\n■ 海を渡る　海の向こうへ兵を動かすと、兵は最寄りの味方の港（無ければ海岸）で輸送船に乗り、目的地に近い港・海岸に上陸する。輸送船はUボートに沈められることがある。たとえばドイツから英国へ攻め込むには、兵を選んで英国の都市（ロンドンなど）を右クリックすればよい。兵は自国の港（青い印）へ歩いて輸送船に乗り、英国の海岸（橙の印）に上陸して都市を攻める。\n■ 植民地と船団　英国・フランスには植民地（カナダ・インド・アフリカ・アルジェリアなど）と米国から船で資源と民が届く。Uボートはこれを沈められ、駆逐艦は Uボートを狩る。\n■ 米国　米国は選べない。中立の米国の船を沈められるなど「妨害」を受けて参戦気運が 100 に達すると、協商国側で参戦する。ドイツは外交画面で「無制限潜水艦作戦」を始められる（米国の船も沈めるが、米国の参戦を招く）。\n■ 塹壕陣地　民だけを選ぶと「塹壕（R）」で築ける。近づく敵兵を撃ち、そばの味方の兵が受ける損害を3割減らす。数は4＋都市1つにつき2つ（最大60）。\n■ 史実の出来事　マルヌの奇跡・毒ガス・ルシタニア号・ヴェルダン・ソンム・ブルシーロフ攻勢・ツィンメルマン電報・春季攻勢・スペインかぜなどが史実の日付に起こる。記録は「戦史」（F3）で見られる。\n■ 国の特徴　国ごとに史実に基づく特徴がある（タイトル画面と戦史で確認できる）。\n■ 指揮中は時が止まる　自国の部隊を選ぶと時間が止まり、命令を出すと（または選択を解くと）再び進む。\n■ 戦場の霧　自国と同盟国の民・兵・建物の周囲しか見えない。都市の所在（どこにあり、どの国のものか）だけはすべて初めから分かっている。\n\n■ 操作\n　左クリック：選択　ドラッグ：範囲選択　ダブルクリック：同種を全選択（兵営・都市にも使え、訓練はまとめて予約）　Shift：追加選択\n　右クリック：移動／攻撃／採集（森・鉱山・農場）／建設の手伝い　建物選択中の右クリック：集合地点\n　A：進軍（攻撃移動）　S：停止　T：訓練・教化　C：兵⇔民の転換　Q/W/E/R：建設・訓練\n　Ctrl を押しながら建物を置く：建設の予約（続けて置ける）\n　Ctrl+数字：部隊登録　数字：部隊呼び出し　.：待機中の民　Home：首都へ\n　ホイール：拡大縮小　中ボタンドラッグ・矢印キー・画面端：スクロール　ミニマップクリック：移動\n　Space：一時停止　＋／－：速度　Esc：取り消し　F2：外交　F3：戦史　F4：戦況　F5：記録　F11：全画面　Ctrl＋＋／Ctrl＋−：文字の大きさ",
            "■ Goal  The Great War begins with the Sarajevo assassination on June 28, 1914. Defeat every state outside your alliance that is at war with you to win. If no victor emerges by June 28, 1919 (the day of the Treaty of Versailles), the angels descend, take every city within half a year, and the game ends. Time flows the same for every state (1 month = 100 s).\n■ Outbreak  The July Crisis declarations (Austria on Serbia, Germany on Russia, France and Belgium, Britain on Germany...) happen automatically on their historical dates. The Ottoman Empire, Italy, Bulgaria, Portugal, Romania and Greece also enter on their historical dates. There is no peace between the alliances. Declare war on a neutral and it joins the enemy side.\n\n■ Resources  Food (farm), wood (cut forests), iron (mine). Workers carry them to your cities, which also pay tax. Production and resources are richest at the outbreak and decline the longer the war lasts.\n■ Population and conscription  Workers plus soldiers are limited (capturing enemy cities raises the limit; losing cities lowers it). Soldiers are drafted from workers: every soldier trained at a barracks takes one worker. The bigger the army, the thinner the economy. Soldiers can be demobilized back into workers (power drops to 2/3). Soldiers eat food just by existing (upkeep).\n■ Cities  Only the historical cities exist; no new ones can be founded. A state that loses all its cities falls.\n\n■ Public opinion  Opinion falls while you keep losing more than the enemy, keep too many soldiers for your population (heavy military spending), run out of food, or lose cities. In the empires (Germany, Austria-Hungary, the Ottomans, Russia, Bulgaria), opinion falling near 0 brings revolution and surrender (its cities go to the enemy). Democracies have no revolutions, but below 50 strikes cut production.\n■ Units  Every state can build infantry (rifles), artillery (long range, strong against infantry and buildings) and cavalry (fast, strong against artillery). Some have special units: tanks (Britain from Sept 1916, France from Apr 1917, the US from 1918), stormtroopers (Germany from July 1916), mountain troops (Italy, Austria-Hungary, France, Switzerland), U-boats (Germany, Austria-Hungary) and destroyers (Britain, France, Italy, Russia, the US). Tanks, U-boats and destroyers are weapons: select soldiers and press ‘Crew …’ (B / U) to make one (a tank takes 5 soldiers, a U-boat 4, a destroyer 6; tanks inside your territory, warships next to one of your naval bases). The soldiers become the weapon and can never be released.\n■ Crossing the sea  Order troops across the sea and they board transports at the nearest friendly port (or beach) and land at the port or beach nearest the goal. U-boats can sink transports. To invade Britain from Germany, for example, select soldiers and right-click a British city (London, say): they march to a home port (blue mark), sail, land on the British coast (orange mark) and attack the city.\n■ Colonies and convoys  Britain and France receive resources and workers by ship from their colonies (Canada, India, Africa, Algeria...) and from America. U-boats can sink them; destroyers hunt U-boats.\n■ America  The United States cannot be chosen. If it is obstructed — its neutral ships sunk, for example — its war fever rises, and at 100 it joins the Entente. Germany can begin unrestricted submarine warfare on the Diplomacy screen (sinking American ships too, at the risk of bringing America in).\n■ Trench lines  With only workers selected, press Trench (R) to dig one. It fires on approaching enemies and cuts damage to friendly troops beside it by 30%. Up to 4 plus 2 per city (at most 60).\n■ Historical events  The Miracle of the Marne, poison gas, the Lusitania, Verdun, the Somme, the Brusilov Offensive, the Zimmermann Telegram, the Spring Offensive, the Spanish flu and more happen on their dates. See them in the Chronicle (F3).\n■ National traits  Each state has historical traits (see the title screen and the Chronicle).\n■ Time stops while commanding  Selecting your own units stops time; it resumes when you give an order or clear the selection.\n■ Fog of war  You see only around your own and your allies' workers, soldiers and buildings. Every city (where it is and who holds it) is known from the start.\n\n■ Controls\n  Left click: select   Drag: box select   Double click: select all of a type (works on barracks and cities too; training queues in each)   Shift: add to selection\n  Right click: move / attack / gather (forest, mine, farm) / help build   Right click with a building selected: rally point\n  A: attack-move   S: stop   T: train / educate   C: convert soldier ⇔ worker   Q/W/E/R: build / train\n  Hold Ctrl while placing buildings: reserve builds (keep placing)\n  Ctrl+number: save group   Number: recall group   .: idle worker   Home: go to capital\n  Wheel: zoom   Middle drag / arrow keys / screen edge: scroll   Minimap click: jump\n  Space: pause   + / -: speed   Esc: cancel   F2: Diplomacy   F3: Chronicle   F4: War   F5: save / load   F11: fullscreen   Ctrl++ / Ctrl+−: text size",
            "■ 目标　1914年6月28日，大战因萨拉热窝事件爆发。击败同盟以外的所有敌国（与你交战的国家）即获胜。若1919年6月28日（凡尔赛条约签订之日）前仍无胜者，天使将降临，在半年内占领所有城市，游戏结束。时间对所有国家同样流逝（1个月＝100秒）。\n■ 开战　七月危机中的宣战（奥地利→塞尔维亚、德国→俄国・法国・比利时、英国→德国……）会在史实日期自动发生。奥斯曼帝国、意大利、保加利亚、葡萄牙、罗马尼亚、希腊也会在史实日期参战。阵营之间不会议和。向中立国宣战会使其加入敌方阵营。\n\n■ 资源　粮食（耕种农场）、木材（伐木）、铁（采矿）。由民采集并运往城市，城市也会缴税。生产力与资源在开战时最为充足，战争越久越少。\n■ 人口与征兵　民与兵合计的人口有上限（夺取敌方城市则增加，失去城市则减少）。士兵由民征召：在兵营每训练一名士兵就减少一名民。军队越大，经济越单薄。士兵可复员为民（能力降为2/3）。军事单位只要存在就会消耗粮食（维持费）。\n■ 城市　只有史实中的城市，无法新建。失去全部城市的国家灭亡。\n\n■ 舆论　若在战场上的损失持续多于敌方、兵相对人口过多（军费沉重）、粮食耗尽、失去城市……舆论就会下降。帝制国家（德国、奥匈、奥斯曼、俄国、保加利亚）的舆论接近0时将爆发革命而投降（城市移交给敌国）。民主制国家不会革命，但舆论低于50时罢工会降低生产。\n■ 兵种　各国都能建造步兵（步枪）、炮兵（远程，克制步兵与建筑）、骑兵（迅速，克制炮兵）。部分国家拥有特殊兵种：坦克（英国1916年9月起、法国1917年4月起、美国1918年起）、突击兵（德国1916年7月起）、山地兵（意大利、奥匈、法国、瑞士）、U艇（德国、奥匈）、驱逐舰（英、法、意、俄、美）。坦克、U艇、驱逐舰是兵器：选中士兵后按“编入…”（B／U）即可建造（坦克需5名士兵、U艇4名、驱逐舰6名；坦克须在己方领土内，军舰须在己方军港附近）。士兵变为兵器，无法撤回。\n■ 渡海　命令士兵前往海的另一边时，他们会在最近的友方港口（若无则海岸）登上运输船，并在离目的地最近的港口或海岸登陆。运输船可能被U艇击沉。例如德国要进攻英国，只需选中士兵并右键点击英国的城市（如伦敦）：士兵会走到本国港口（蓝色标记）登上运输船，在英国海岸（橙色标记）登陆并进攻城市。\n■ 殖民地与船队　英国、法国会从殖民地（加拿大、印度、非洲、阿尔及利亚等）及美国经海路获得资源与民。U艇可将其击沉，驱逐舰可猎杀U艇。\n■ 美国　不能选择美国。若中立美国的船只被击沉等“妨碍”使其参战情绪达到100，美国将加入协约国参战。德国可在外交画面开始“无限制潜艇战”（连美国船只也击沉，但会招致美国参战）。\n■ 堑壕阵地　只选中民时，可用“堑壕（R）”修筑。射击靠近的敌兵，并使附近友军受到的伤害减少三成。数量为4＋每座城市2座（最多60）。\n■ 史实事件　马恩河奇迹、毒气、卢西塔尼亚号、凡尔登、索姆河、布鲁西洛夫攻势、齐默尔曼电报、春季攻势、西班牙流感等会在史实日期发生。可在“战史”（F3）中查看。\n■ 国家特点　各国拥有基于史实的特点（可在标题画面与战史中查看）。\n■ 指挥中时间停止　选中己方单位时时间停止，下达命令（或取消选择）后继续。\n■ 战争迷雾　只能看到己方及同盟国的民、兵、建筑周围。所有城市的位置（在哪里、属于哪国）从一开始就已知。\n\n■ 操作\n　左键：选择　拖动：框选　双击：选择全部同类（也可用于兵营和城市，训练将一并预约）　Shift：追加选择\n　右键：移动／攻击／采集（森林・矿山・农场）／协助建造　选中建筑时右键：集结点\n　A：进军（攻击移动）　S：停止　T：训练・教化　C：兵⇔民转换　Q/W/E/R：建造・训练\n　按住 Ctrl 放置建筑：预约建造（可连续放置）\n　Ctrl+数字：编队　数字：调出编队　.：闲置的民　Home：回首都\n　滚轮：缩放　中键拖动・方向键・画面边缘：滚动　点击小地图：移动\n　Space：暂停　＋／－：速度　Esc：取消　F2：外交　F3：战史　F4：战况　F5：存档　F11：全屏　Ctrl＋＋／Ctrl＋−：文字大小"
        );
        // 入りきらなければ小さい字にする
        let f = if self.gfx.measure_h(text, Font::Normal, pw - 48.0) > ph - 64.0 { Font::Small } else { Font::Normal };
        self.gfx.text(text, x0 + 24.0, y0 + 52.0, pw - 48.0, ph - 64.0, f, [0.92, 0.9, 0.84, 1.0]);
    }

    fn render_gameover(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let Some((win, text)) = self.world.game_over.clone() else { return };
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.55]);
        let pw = 640.0;
        let x0 = (w - pw) / 2.0;
        let y0 = h / 2.0 - 140.0;
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + 260.0);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + 257.0, 16.0);
        let title = if win {
            t!("勝利", "Victory", "胜利")
        } else if self.world.descent.is_some() {
            t!("天使の平和", "The Peace of the Angels", "天使的和平")
        } else {
            t!("敗北", "Defeat", "败北")
        };
        self.gfx.text_shadow(title, x0, y0 + 20.0, pw, 60.0, Font::Title, if win { [1.0, 0.85, 0.4, 1.0] } else { [0.9, 0.4, 0.35, 1.0] });
        self.gfx.text(&text, x0 + 20.0, y0 + 100.0, pw - 40.0, 60.0, Font::BigCenter, [0.95, 0.92, 0.85, 1.0]);
        self.add_button(x0 + pw / 2.0 - 210.0, y0 + 180.0, x0 + pw / 2.0 - 10.0, y0 + 228.0, t!("新しい対局へ", "New game", "开始新对局"), "", 0, true, Act::Restart, "");
        self.add_button(x0 + pw / 2.0 + 10.0, y0 + 180.0, x0 + pw / 2.0 + 210.0, y0 + 228.0, t!("記録から再開 F5", "Load a save F5", "读取存档 F5"), "", 0, true, Act::SaveMenu, "");
    }
}
