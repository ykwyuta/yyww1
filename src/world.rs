//! ゲーム世界のシミュレーション（ユニット・建物・経済・戦闘・外交・世論・海上輸送・イベント）

use crate::ai::AiState;
use crate::geo::*;
use crate::path::PathFinder;
use crate::rng::Rng;
use std::collections::VecDeque;

mod history;
mod naval;
mod opinion;
mod traits;
mod descent;
mod heroes;
mod save;
pub use history::{Chron, Person, HISTORY, INITIAL_HOUSE};
#[cfg(test)]
pub use history::Fx;
pub use naval::*;
pub use opinion::*;
pub use traits::*;
pub use descent::*;
pub use heroes::*;
pub use save::SAVE_VERSION;
pub use crate::lang::{sep, tx};

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}
pub fn v2(x: f32, y: f32) -> V2 {
    V2 { x, y }
}
impl std::ops::Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl std::ops::Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
impl std::ops::Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, s: f32) -> V2 {
        v2(self.x * s, self.y * s)
    }
}
impl V2 {
    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub fn dist(self, o: V2) -> f32 {
        (self - o).len()
    }
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l > 1e-6 {
            self * (1.0 / l)
        } else {
            v2(0.0, 0.0)
        }
    }
    pub fn tile(self) -> (i32, i32) {
        (self.x.floor() as i32, self.y.floor() as i32)
    }
}

/// 1年の長さ（秒）。1か月＝100秒
pub const SEC_PER_YEAR: f32 = 1200.0;
/// ゲームの始まり：1914年6月28日（サラエボ事件）
pub const START_YEAR: f32 = 1914.0 + 178.0 / 365.0;
/// 総力戦の時代（1916年7月1日、ソンムの戦い）
pub const TOTAL_WAR_YEAR: f32 = 1916.0 + 182.0 / 365.0;
/// 終戦の期限：1919年6月28日（ヴェルサイユ条約の日）。それまでに勝者が現れなければ天使が現れる
pub const END_YEAR: f32 = 1919.0 + 178.0 / 365.0;
pub const DT: f32 = 1.0 / 30.0;
pub const NONE: u8 = 255;

/// 年・月・日を年の小数にする
pub const fn ymd(y: i32, m: i32, d: i32) -> f32 {
    y as f32 + ((m - 1) as f32 * 30.44 + (d - 1) as f32) / 365.25
}

/// 時代（開戦期・総力戦期）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Era {
    Early,
    Total,
}

pub const N_KINDS: usize = 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnitKind {
    Worker,
    Infantry,
    Artillery,
    Cavalry,
    /// 戦車（英仏米、1916年9月〜）
    Tank,
    /// 突撃兵（ドイツ、1916年〜）
    Storm,
    /// 山岳兵（イタリアのアルピーニ、オーストリアの皇帝猟兵）
    Alpine,
    /// Uボート（ドイツ・オーストリア）
    Submarine,
    /// 駆逐艦（英仏伊露米）
    Destroyer,
    /// 輸送船（兵の渡海・植民地からの船団。訓練はできない）
    Transport,
}

pub const ALL_KINDS: [UnitKind; N_KINDS] = [
    UnitKind::Worker,
    UnitKind::Infantry,
    UnitKind::Artillery,
    UnitKind::Cavalry,
    UnitKind::Tank,
    UnitKind::Storm,
    UnitKind::Alpine,
    UnitKind::Submarine,
    UnitKind::Destroyer,
    UnitKind::Transport,
];

impl UnitKind {
    pub fn military(self) -> bool {
        self != UnitKind::Worker
    }
    /// 海を行く
    pub fn naval(self) -> bool {
        matches!(self, UnitKind::Submarine | UnitKind::Destroyer | UnitKind::Transport)
    }
    /// 兵器に乗る兵の数（1＝兵器ではない）。兵器は兵を割り当てて造り、兵に戻せない
    pub fn crew(self) -> u32 {
        match self {
            UnitKind::Tank => 5,
            UnitKind::Submarine => 4,
            UnitKind::Destroyer => 6,
            _ => 1,
        }
    }
    /// 兵を割り当てて造る兵器（戦車・Uボート・駆逐艦）
    pub fn weapon(self) -> bool {
        self.crew() > 1
    }
    /// 陸の兵（民は含まない）
    pub fn land_mil(self) -> bool {
        self.military() && !self.naval()
    }
    /// 視界半径（タイル）
    pub fn vision(self) -> i32 {
        match self {
            UnitKind::Worker => 5,
            UnitKind::Infantry | UnitKind::Storm | UnitKind::Alpine => 7,
            UnitKind::Artillery => 8,
            UnitKind::Cavalry => 9,
            UnitKind::Tank => 7,
            UnitKind::Submarine => 9,
            UnitKind::Destroyer => 9,
            UnitKind::Transport => 5,
        }
    }
    pub fn name(self, _era: Era) -> &'static str {
        match self {
            UnitKind::Worker => t!("民", "Worker", "民"),
            UnitKind::Infantry => t!("歩兵", "Infantry", "步兵"),
            UnitKind::Artillery => t!("砲兵", "Artillery", "炮兵"),
            UnitKind::Cavalry => t!("騎兵", "Cavalry", "骑兵"),
            UnitKind::Tank => t!("戦車", "Tank", "坦克"),
            UnitKind::Storm => t!("突撃兵", "Stormtrooper", "突击兵"),
            UnitKind::Alpine => t!("山岳兵", "Mountain troops", "山地兵"),
            UnitKind::Submarine => t!("Uボート", "U-boat", "U艇"),
            UnitKind::Destroyer => t!("駆逐艦", "Destroyer", "驱逐舰"),
            UnitKind::Transport => t!("輸送船", "Transport", "运输船"),
        }
    }
    /// [食糧, 木材, 鉄]（兵は民を1人徴兵したうえで、装備にこれだけかかる。兵器は兵を割り当てたうえで、兵器そのものにこれだけかかる）
    pub fn cost(self) -> [f32; 3] {
        match self {
            UnitKind::Worker => [50.0, 0.0, 0.0],
            UnitKind::Infantry => [40.0, 20.0, 10.0],
            UnitKind::Artillery => [40.0, 60.0, 30.0],
            UnitKind::Cavalry => [80.0, 25.0, 10.0],
            UnitKind::Tank => [0.0, 60.0, 100.0],
            UnitKind::Storm => [60.0, 30.0, 20.0],
            UnitKind::Alpine => [50.0, 40.0, 15.0],
            UnitKind::Submarine => [0.0, 100.0, 80.0],
            UnitKind::Destroyer => [0.0, 120.0, 100.0],
            UnitKind::Transport => [0.0, 0.0, 0.0],
        }
    }
    pub fn train_time(self) -> f32 {
        match self {
            UnitKind::Worker => 8.0,
            UnitKind::Infantry => 8.0,
            UnitKind::Artillery => 12.0,
            UnitKind::Cavalry => 10.0,
            UnitKind::Tank => 18.0,
            UnitKind::Storm => 12.0,
            UnitKind::Alpine => 10.0,
            UnitKind::Submarine => 20.0,
            UnitKind::Destroyer => 20.0,
            UnitKind::Transport => 5.0,
        }
    }
    pub fn base_hp(self) -> f32 {
        match self {
            UnitKind::Worker => 40.0,
            UnitKind::Infantry => 110.0,
            UnitKind::Artillery => 70.0,
            UnitKind::Cavalry => 120.0,
            UnitKind::Tank => 700.0,
            UnitKind::Storm => 120.0,
            UnitKind::Alpine => 110.0,
            UnitKind::Submarine => 300.0,
            UnitKind::Destroyer => 600.0,
            UnitKind::Transport => 110.0,
        }
    }
    pub fn base_atk(self) -> f32 {
        match self {
            UnitKind::Worker => 2.0,
            UnitKind::Infantry => 6.5,
            UnitKind::Artillery => 13.0,
            UnitKind::Cavalry => 8.5,
            UnitKind::Tank => 30.0,
            UnitKind::Storm => 9.0,
            UnitKind::Alpine => 7.0,
            UnitKind::Submarine => 60.0,
            UnitKind::Destroyer => 40.0,
            UnitKind::Transport => 0.0,
        }
    }
    pub fn range(self) -> f32 {
        match self {
            UnitKind::Infantry | UnitKind::Alpine => 2.6,
            UnitKind::Storm => 1.6,
            UnitKind::Artillery => 7.0,
            UnitKind::Tank => 2.2,
            UnitKind::Submarine => 3.0,
            UnitKind::Destroyer => 4.0,
            _ => 0.75,
        }
    }
    pub fn cooldown(self) -> f32 {
        match self {
            UnitKind::Artillery => 3.2,
            UnitKind::Tank => 1.6,
            UnitKind::Submarine => 3.0,
            UnitKind::Destroyer => 1.5,
            UnitKind::Cavalry => 1.3,
            _ => 1.4,
        }
    }
    pub fn speed(self, _era: Era) -> f32 {
        match self {
            UnitKind::Worker => 1.9,
            UnitKind::Infantry | UnitKind::Alpine => 1.8,
            UnitKind::Storm => 2.1,
            UnitKind::Artillery => 1.3,
            UnitKind::Cavalry => 3.2,
            UnitKind::Tank => 1.2,
            UnitKind::Submarine => 2.8,
            UnitKind::Destroyer => 3.8,
            UnitKind::Transport => 3.2,
        }
    }
    pub fn sight(self) -> f32 {
        match self {
            UnitKind::Worker => 4.0,
            UnitKind::Artillery => 8.0,
            UnitKind::Transport => 3.0,
            UnitKind::Destroyer => 7.0,
            UnitKind::Submarine => 8.0,
            _ => 6.0,
        }
    }
    /// 相性補正
    pub fn bonus(self, def: UnitKind) -> f32 {
        use UnitKind::*;
        match (self, def) {
            (Infantry, Cavalry) | (Alpine, Cavalry) => 1.5,
            (Cavalry, Artillery) => 1.8,
            (Cavalry, Worker) => 1.3,
            (Artillery, Infantry) | (Artillery, Storm) | (Artillery, Alpine) => 1.3,
            (Artillery, Tank) => 1.6,
            (Tank, Infantry) | (Tank, Alpine) | (Tank, Cavalry) => 1.5,
            (Storm, Infantry) | (Storm, Alpine) | (Storm, Artillery) => 1.35,
            (Submarine, Transport) => 2.2,
            (Destroyer, Submarine) => 2.6,
            (Destroyer, Transport) => 1.5,
            (Submarine, Destroyer) => 0.8,
            _ => 1.0,
        }
    }
    pub fn vs_building(self) -> f32 {
        match self {
            UnitKind::Infantry | UnitKind::Alpine => 0.8,
            UnitKind::Storm => 1.0,
            UnitKind::Artillery => 1.5,
            UnitKind::Cavalry => 0.5,
            UnitKind::Tank => 1.3,
            UnitKind::Worker => 0.4,
            UnitKind::Submarine | UnitKind::Transport | UnitKind::Destroyer => 0.0,
        }
    }
    /// 丘で受ける損害の軽減（山岳兵は丘・森に強い）
    pub fn hill_def(self) -> f32 {
        if self == UnitKind::Alpine {
            0.6
        } else {
            0.8
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BKind {
    Capital,
    Town,
    Barracks,
    Farm,
    /// 塹壕陣地：近づく敵兵を撃つ（数に上限あり）
    Fort,
    /// 軍港：Uボート・駆逐艦を造る（海に面した場所に建てる）
    Dock,
}

/// 都市が無くても持てる民・兵の数（滅んだ国から分けられる量）
pub const BASE_CAP: u32 = 4;

/// 塹壕陣地の射程・攻撃間隔
pub const FORT_RANGE: f32 = 7.5;
pub const FORT_COOLDOWN: f32 = 1.2;

impl BKind {
    pub fn name(self) -> &'static str {
        match self {
            BKind::Capital => t!("首都", "Capital", "首都"),
            BKind::Town => t!("都市", "City", "城市"),
            BKind::Barracks => t!("兵営", "Barracks", "兵营"),
            BKind::Farm => t!("農場", "Farm", "农场"),
            BKind::Fort => t!("塹壕陣地", "Trench line", "堑壕阵地"),
            BKind::Dock => t!("軍港", "Naval base", "军港"),
        }
    }
    pub fn cost(self) -> [f32; 3] {
        match self {
            BKind::Capital => [0.0, 0.0, 0.0],
            BKind::Town => [0.0, 0.0, 0.0],
            BKind::Barracks => [0.0, 150.0, 30.0],
            BKind::Farm => [0.0, 60.0, 0.0],
            BKind::Fort => [100.0, 250.0, 60.0],
            BKind::Dock => [0.0, 250.0, 80.0],
        }
    }
    pub fn max_hp(self) -> f32 {
        match self {
            BKind::Capital => 6000.0,
            BKind::Town => 3200.0,
            BKind::Barracks => 1200.0,
            BKind::Farm => 300.0,
            BKind::Fort => 2200.0,
            BKind::Dock => 1500.0,
        }
    }
    pub fn build_time(self) -> f32 {
        match self {
            BKind::Capital => 60.0,
            BKind::Town => 40.0,
            BKind::Barracks => 20.0,
            BKind::Farm => 8.0,
            BKind::Fort => 20.0,
            BKind::Dock => 30.0,
        }
    }
    pub fn radius(self) -> i32 {
        match self {
            BKind::Capital => 12,
            BKind::Town => 9,
            _ => 0,
        }
    }
    pub fn city(self) -> bool {
        matches!(self, BKind::Capital | BKind::Town)
    }
    pub fn blocks(self) -> bool {
        self != BKind::Farm
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Handle {
    pub idx: u32,
    pub gen: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Target {
    Unit(Handle),
    Bld(Handle),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Src {
    Farm(Handle),
    Forest(i32, i32),
    Mine(usize),
}
impl Src {
    pub fn res(self) -> usize {
        match self {
            Src::Farm(_) => 0,
            Src::Forest(..) => 1,
            Src::Mine(_) => 2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Order {
    Idle,
    Move(V2),
    AttackMove(V2),
    Attack(Target),
    /// phase 0:資源へ 1:採集中 2:集積地へ
    Gather { src: Src, phase: u8 },
    Build(Handle),
}

pub struct Unit {
    pub gen: u32,
    pub alive: bool,
    pub nation: u8,
    pub kind: UnitKind,
    pub pos: V2,
    pub hp: f32,
    pub power: f32,
    pub order: Order,
    pub resume: Option<V2>,
    pub path: Vec<(i16, i16)>,
    pub path_i: usize,
    pub path_goal: (i32, i32),
    pub path_pending: bool,
    pub path_done: bool,
    pub cooldown: f32,
    pub carry_kind: usize,
    pub carry: f32,
    pub scan_t: f32,
    pub dir: V2,
    pub flash: f32,
    pub selected: bool,
    pub last_attacker: Option<Handle>,
    /// 天使（英雄にしか傷つけられない）
    pub divine: bool,
    /// 英雄（HEROES の番号、NONE=英雄ではない）
    pub hero: u8,
    /// 建設の予約（今の建設を終えたら順に取りかかる）
    pub build_queue: VecDeque<Handle>,
    /// 渡海の予定（港へ向かって船に乗る）
    pub ferry: Option<Ferry>,
    /// 船（輸送船の積み荷と行き先）
    pub ship: Option<Box<Ship>>,
}

impl Unit {
    /// 兵の数で重みをつけた力（兵器は乗っている兵の数だけ）
    pub fn strength(&self) -> f32 {
        self.power * self.kind.crew() as f32
    }
    pub fn max_hp(&self) -> f32 {
        self.kind.base_hp() * self.power
    }
    pub fn reset_nav(&mut self) {
        self.path.clear();
        self.path_i = 0;
        self.path_goal = (-9, -9);
        self.path_done = false;
    }
}

pub struct Building {
    pub gen: u32,
    pub alive: bool,
    pub nation: u8,
    pub kind: BKind,
    pub tx: i32,
    pub ty: i32,
    pub hp: f32,
    pub progress: f32,
    pub queue: VecDeque<UnitKind>,
    pub queue_t: f32,
    pub rally: Option<V2>,
    pub cooldown: f32,
    /// 都市の名前（CITY_NAMES の番号。都市以外は u16::MAX）
    pub city: u16,
}

impl Building {
    pub fn center(&self) -> V2 {
        v2(self.tx as f32 + 1.0, self.ty as f32 + 1.0)
    }
    pub fn done(&self) -> bool {
        self.progress >= 1.0
    }
    pub fn dist_to_footprint(&self, p: V2) -> f32 {
        let cx = p.x.clamp(self.tx as f32, self.tx as f32 + 2.0);
        let cy = p.y.clamp(self.ty as f32, self.ty as f32 + 2.0);
        p.dist(v2(cx, cy))
    }
    pub fn city_name(&self) -> Option<&'static str> {
        city_name(self.city)
    }
}

/// 都市の番号から名前（国ごとの都市の並びを通しで数える）
pub fn month_en(m: i32) -> &'static str {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][((m - 1).clamp(0, 11)) as usize]
}

pub fn city_name(id: u16) -> Option<&'static str> {
    let mut k = id as usize;
    for nd in NATIONS {
        if k < nd.cities.len() {
            return Some(nd.cities[k].name);
        }
        k -= nd.cities.len();
    }
    None
}

pub struct Nation {
    pub name: &'static str,
    pub color: [f32; 3],
    pub bias: f32,
    pub alive: bool,
    pub appeared: bool,
    pub res: [f32; 3],
    pub capital: Option<Handle>,
    pub workers: u32,
    pub soldiers: u32,
    pub q_workers: u32,
    pub q_soldiers: u32,
    pub cities: u32,
    pub mil_power: f32,
    pub eco_power: f32,
    pub tiles: u32,
    pub upkeep: f32,
    pub strength: f32,
    pub earned: [f32; 3],
    pub income: [f32; 3],
    pub home: V2,
    /// 君主家・政体
    pub house: &'static str,
    /// 仕えている人物
    pub figures: Vec<Person>,
    /// 人物による生産・武の倍率
    pub eco_mul: f32,
    pub mil_mul: f32,
    /// 国の特徴による補正
    pub tm: TraitMods,
    /// 滅んだ国から分けられた人口上限
    pub cap_bonus: u32,
    /// 陣営（中立国は参戦すると変わる）
    pub bloc: Bloc,
    /// 世論（0〜100）。帝政の国は 0 近くまで落ちると革命が起きて降伏する
    pub opinion: f32,
    /// この月に失った兵力・与えた兵力（世論の計算に使う）
    pub lost: f32,
    pub inflicted: f32,
    /// 開戦からの累計
    pub lost_total: f32,
    pub inflicted_total: f32,
    /// この月に落とした・落とされた都市の数
    pub won_cities: u32,
    pub lost_cities: u32,
    /// 世論の前月からの変化（表示用）
    pub opinion_delta: f32,
    /// 降伏した（革命）
    pub surrendered: bool,
}

impl Nation {
    pub fn fresh(name: &'static str, color: [f32; 3], bias: f32, home: V2, house: &'static str, bloc: Bloc) -> Nation {
        Nation {
            name,
            color,
            bias,
            alive: true,
            appeared: true,
            res: [0.0; 3],
            capital: None,
            workers: 0,
            soldiers: 0,
            q_workers: 0,
            q_soldiers: 0,
            cities: 0,
            mil_power: 0.0,
            eco_power: 0.0,
            tiles: 0,
            upkeep: 0.0,
            strength: 0.0,
            earned: [0.0; 3],
            income: [0.0; 3],
            home,
            house,
            figures: Vec::new(),
            eco_mul: 1.0,
            mil_mul: 1.0,
            tm: TraitMods::NONE,
            cap_bonus: 0,
            bloc,
            opinion: 72.0,
            lost: 0.0,
            inflicted: 0.0,
            lost_total: 0.0,
            inflicted_total: 0.0,
            won_cities: 0,
            lost_cities: 0,
            opinion_delta: 0.0,
            surrendered: false,
        }
    }
    /// 民・兵（訓練待ちを含む）の数
    pub fn pop(&self) -> u32 {
        self.workers + self.soldiers + self.q_workers + self.q_soldiers
    }
    /// 民・兵の上限。都市を得れば増え、失えば減る（1都市あたり1）
    pub fn pop_cap(&self, base: u32, cities0: u32) -> u32 {
        let c = self.cities as i32 - cities0 as i32;
        ((base as i32 + c).max(base as i32 / 2).max(4) as u32) + self.cap_bonus
    }
}

/// プレイヤーへの和睦の申し入れ（受諾か拒否かを選ぶ）
#[derive(Clone, Debug)]
pub struct PeaceAsk {
    pub from: usize,
    pub years: f32,
    pub why: String,
}

pub struct Msg {
    pub text: String,
    pub time: f32,
    pub color: [f32; 3],
}

pub struct Shot {
    pub a: V2,
    pub b: V2,
    pub t: f32,
}

pub struct Banner {
    pub title: String,
    pub text: String,
    pub t: f32,
}

pub const GCELL: i32 = 4;
pub const GW: i32 = (MAP_W as i32 + GCELL - 1) / GCELL;
pub const GH: i32 = (MAP_H as i32 + GCELL - 1) / GCELL;

pub struct World {
    pub map: Map,
    pub cost: Vec<f32>,
    /// 船の移動コスト（海だけ通れる）
    pub sea_cost: Vec<f32>,
    /// 陸・海の連結成分（渡海の要否と港の選択に使う）
    pub land_comp: Vec<u16>,
    pub sea_comp: Vec<u16>,
    /// 海に面した陸のマス（間引いたもの）：(陸のマス, 隣の海のマス)
    pub coast: Vec<((i32, i32), (i32, i32))>,
    pub bmap: Vec<u32>,
    pub owner: Vec<u8>,
    pub units: Vec<Unit>,
    pub blds: Vec<Building>,
    free_units: Vec<u32>,
    free_blds: Vec<u32>,
    pub nations: Vec<Nation>,
    pub war: Vec<Vec<bool>>,
    pub war_start: Vec<Vec<f32>>,
    pub truce: Vec<Vec<f32>>,
    pub peace_offer: Vec<bool>,
    /// 和睦の申し入れを断られた国は、この年まで申し入れない
    pub offer_cd: Vec<f32>,
    /// プレイヤーの返答を待つ和睦の申し入れ
    pub peace_asks: VecDeque<PeaceAsk>,
    pub years: f32,
    pub era: Era,
    pub player: usize,
    pub msgs: Vec<Msg>,
    pub banner: Option<Banner>,
    banner_queue: VecDeque<Banner>,
    /// 次に起こる史実イベント
    hist_i: usize,
    /// 戦史（起きた出来事・起こらなかった出来事）
    pub chronicle: Vec<Chron>,
    pub shots: Vec<Shot>,
    pub rng: Rng,
    pf: PathFinder,
    path_queue: VecDeque<Handle>,
    pub grid: Vec<Vec<u32>>,
    pub territory_dirty: bool,
    pub territory_version: u32,
    /// 国ごとの領土のマス数（recompute_territory で数え直す）
    tile_count: Vec<u32>,
    /// 国ごとの完成した都市（毎ステップ作り直す。運び先を探すのに全建物を見ないため）
    city_list: Vec<Vec<u32>>,
    /// 押し合いを1ステップおきにするための拍子
    sep_tick: bool,
    pub terrain_dirty: Vec<(i32, i32)>,
    pub game_over: Option<(bool, String)>,
    /// チュートリアル中はプレイヤーへの宣戦を禁じる
    pub protect_player: bool,
    /// プレイヤーが部隊に命令した回数
    pub player_orders: u32,
    /// 天使の降臨（1919年6月までに勝者が現れなかった）
    pub descent: Option<Descent>,
    /// 撮影用：プレイヤーの国も AI が動かす
    pub ai_player: bool,
    /// 英雄召喚（常に有効。記録の互換のために残す）
    pub heroic: bool,
    pub time: f32,
    pub ai: Vec<AiState>,
    /// 海上：植民地からの船団・米国の参戦気運・無制限潜水艦作戦
    pub sea: SeaState,
    stat_t: f32,
    income_t: f32,
    month_tick: f32,
}

/// 開戦時の陣営
fn d0_bloc(n: usize) -> Bloc {
    NATIONS[n].bloc
}

/// 天使の枠（国としては扱わない）
pub const ANGEL: usize = NATIONS.len();

impl World {
    pub fn new(seed: u32) -> World {
        let map = generate_map(seed);
        let n = MAP_W * MAP_H;
        let mut cost = vec![0.0; n];
        let mut sea_cost = vec![f32::INFINITY; n];
        for i in 0..n {
            cost[i] = map.terrain[i].move_cost();
            if map.terrain[i].navigable() {
                sea_cost[i] = 1.0;
            }
        }
        let (land_comp, _) = land_components(&map.terrain);
        let sea_comp = sea_components(&map.terrain);
        let coast = coast_points(&map.terrain);
        let nn = NATIONS.len();
        let mut w = World {
            map,
            cost,
            sea_cost,
            land_comp,
            sea_comp,
            coast,
            bmap: vec![0; n],
            owner: vec![NONE; n],
            tile_count: Vec::new(),
            city_list: Vec::new(),
            sep_tick: false,
            units: Vec::new(),
            blds: Vec::new(),
            free_units: Vec::new(),
            free_blds: Vec::new(),
            nations: Vec::new(),
            war: vec![vec![false; nn]; nn],
            war_start: vec![vec![0.0; nn]; nn],
            truce: vec![vec![0.0; nn]; nn],
            peace_offer: vec![false; nn],
            offer_cd: vec![0.0; nn],
            peace_asks: VecDeque::new(),
            years: 0.0,
            era: Era::Early,
            player: 0,
            msgs: Vec::new(),
            banner: None,
            banner_queue: VecDeque::new(),
            hist_i: 0,
            chronicle: Vec::new(),
            shots: Vec::new(),
            rng: Rng::new(seed as u64 * 7 + 3),
            pf: PathFinder::new(),
            path_queue: VecDeque::new(),
            grid: vec![Vec::new(); (GW * GH) as usize],
            territory_dirty: true,
            territory_version: 0,
            terrain_dirty: Vec::new(),
            game_over: None,
            protect_player: false,
            player_orders: 0,
            descent: None,
            ai_player: false,
            heroic: true,
            time: 0.0,
            ai: Vec::new(),
            sea: SeaState::new(),
            stat_t: 0.0,
            income_t: 0.0,
            month_tick: 0.0,
        };
        for (i, d) in NATIONS.iter().enumerate() {
            let (hx, hy) = ll_to_tile(d.lon, d.lat);
            w.nations.push(Nation::fresh(d.name, d.color, d.bias, v2(hx, hy), INITIAL_HOUSE[i], d.bloc));
            // 大国ほど蓄えが多い（資源は開戦時がもっとも豊富）
            let r = 600.0 + d.pop as f32 * 14.0;
            w.nations[i].res = [r, r * 0.8, r * 0.6];
            w.ai.push(AiState::new(i as u64 * 31 + seed as u64));
        }
        // 天使の枠
        let mut angel = Nation::fresh("天使", [0.98, 0.94, 0.72], 0.0, v2(0.0, 0.0), "", Bloc::Neutral);
        angel.alive = false;
        angel.appeared = false;
        w.nations.push(angel);
        let mut cid: u16 = 0;
        for i in 0..nn {
            let d = &NATIONS[i];
            for (k, cd) in d.cities.iter().enumerate() {
                let kind = if k == 0 { BKind::Capital } else { BKind::Town };
                if let Some(h) = w.found_city(i, cd.lon, cd.lat, kind) {
                    w.blds[h.idx as usize].city = cid;
                    if k == 0 {
                        w.nations[i].capital = Some(h);
                        w.nations[i].home = w.blds[h.idx as usize].center();
                    }
                }
                cid += 1;
            }
        }
        w.recompute_territory();
        // 開戦前の軍：民と兵を人口の上限の8割ほど、都市に分けて置く
        for i in 0..nn {
            w.initial_forces(i);
        }
        w.recompute_territory();
        w.update_stats();
        w.refresh_traits();
        w
    }

    /// 開戦前の民と兵を都市に配る
    fn initial_forces(&mut self, i: usize) {
        let d = &NATIONS[i];
        let cities: Vec<Handle> = (0..self.blds.len()).filter(|&b| self.blds[b].alive && self.blds[b].nation as usize == i && self.blds[b].kind.city()).map(|b| self.bh(b)).collect();
        if cities.is_empty() {
            return;
        }
        let total = (d.pop as f32 * 0.82) as usize;
        // 開戦時の兵の割合：大陸の大国は大軍、英国は小さな職業軍、中立国は少なめ
        let mil_share = match i {
            GER | AUH => 0.42,
            FRA => 0.38,
            RUS => 0.38,
            GBR => 0.16,
            SRB | BEL => 0.34,
            _ if d.bloc == Bloc::Neutral => 0.22,
            _ => 0.32,
        };
        let soldiers = (total as f32 * mil_share) as usize;
        let workers = total - soldiers;
        for k in 0..workers {
            let c = cities[k % cities.len()];
            if let Some(sp) = self.spawn_point(c) {
                let h = self.spawn_unit(i, UnitKind::Worker, sp, 1.0);
                self.auto_gather(h);
            }
        }
        // 兵は敵陣営に近い都市（前線）に置く。中立国は首都の周り
        let mut front: Vec<(f32, Handle)> = cities
            .iter()
            .map(|&h| {
                let c = self.blds[h.idx as usize].center();
                let mut d = f32::MAX;
                for (j, od) in NATIONS.iter().enumerate() {
                    let enemy = j != i && od.bloc != Bloc::Neutral && d0_bloc(i) != Bloc::Neutral && od.bloc != d0_bloc(i);
                    if !enemy {
                        continue;
                    }
                    for oc in od.cities {
                        let (x, y) = ll_to_tile(oc.lon, oc.lat);
                        let dd = c.dist(v2(x, y)) + if self.needs_ferry(c, v2(x, y)) { 60.0 } else { 0.0 };
                        d = d.min(dd);
                    }
                }
                if d == f32::MAX {
                    d = c.dist(self.nations[i].home);
                }
                (d, h)
            })
            .collect();
        front.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let nf = ((cities.len() as f32 * 0.4).ceil() as usize).clamp(1, cities.len());
        for k in 0..soldiers {
            let c = front[k % nf].1;
            let kind = match k % 6 {
                2 => UnitKind::Artillery,
                5 => UnitKind::Cavalry,
                _ => UnitKind::Infantry,
            };
            if let Some(sp) = self.spawn_point(c) {
                self.spawn_unit(i, kind, sp, 1.0);
            }
        }
    }

    fn found_city(&mut self, nation: usize, lon: f32, lat: f32, kind: BKind) -> Option<Handle> {
        let (fx, fy) = ll_to_tile(lon, lat);
        let (cx, cy) = (fx as i32 - 1, fy as i32 - 1);
        for r in 0..8i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (tx, ty) = (cx + dx, cy + dy);
                    if self.footprint_free(tx, ty) {
                        return Some(self.spawn_building(nation, kind, tx, ty, true));
                    }
                }
            }
        }
        None
    }

    /// 民・兵の上限
    pub fn pop_cap(&self, n: usize) -> u32 {
        if n >= NATIONS.len() {
            return 0;
        }
        self.nations[n].pop_cap(NATIONS[n].pop, NATIONS[n].cities.len() as u32)
    }

    // ------------------------------------------------------------ 基本操作
    pub fn unit(&self, h: Handle) -> Option<&Unit> {
        let u = self.units.get(h.idx as usize)?;
        if u.alive && u.gen == h.gen {
            Some(u)
        } else {
            None
        }
    }
    pub fn unit_mut(&mut self, h: Handle) -> Option<&mut Unit> {
        let u = self.units.get_mut(h.idx as usize)?;
        if u.alive && u.gen == h.gen {
            Some(u)
        } else {
            None
        }
    }
    pub fn bld(&self, h: Handle) -> Option<&Building> {
        let b = self.blds.get(h.idx as usize)?;
        if b.alive && b.gen == h.gen {
            Some(b)
        } else {
            None
        }
    }
    pub fn bld_mut(&mut self, h: Handle) -> Option<&mut Building> {
        let b = self.blds.get_mut(h.idx as usize)?;
        if b.alive && b.gen == h.gen {
            Some(b)
        } else {
            None
        }
    }
    pub fn uh(&self, i: usize) -> Handle {
        Handle { idx: i as u32, gen: self.units[i].gen }
    }
    pub fn bh(&self, i: usize) -> Handle {
        Handle { idx: i as u32, gen: self.blds[i].gen }
    }

    pub fn spawn_unit(&mut self, nation: usize, kind: UnitKind, pos: V2, power: f32) -> Handle {
        let u = Unit {
            gen: 0,
            alive: true,
            nation: nation as u8,
            kind,
            pos,
            hp: kind.base_hp() * power,
            power,
            order: Order::Idle,
            resume: None,
            path: Vec::new(),
            path_i: 0,
            path_goal: (-1, -1),
            path_pending: false,
            path_done: false,
            cooldown: 0.0,
            carry_kind: 0,
            carry: 0.0,
            scan_t: self.rng.f32() * 0.5,
            dir: v2(0.0, 1.0),
            flash: 0.0,
            selected: false,
            last_attacker: None,
            divine: false,
            hero: NONE,
            build_queue: VecDeque::new(),
            ferry: None,
            ship: None,
        };
        if let Some(i) = self.free_units.pop() {
            let gen = self.units[i as usize].gen + 1;
            self.units[i as usize] = Unit { gen, ..u };
            Handle { idx: i, gen }
        } else {
            self.units.push(u);
            Handle { idx: self.units.len() as u32 - 1, gen: 0 }
        }
    }

    pub fn kill_unit(&mut self, i: usize) {
        if self.units[i].alive {
            self.units[i].alive = false;
            self.units[i].selected = false;
            self.units[i].path.clear();
            self.units[i].ship = None;
            self.units[i].ferry = None;
            self.free_units.push(i as u32);
        }
    }

    pub fn footprint_free(&self, tx: i32, ty: i32) -> bool {
        for dy in 0..2 {
            for dx in 0..2 {
                let (x, y) = (tx + dx, ty + dy);
                if !Map::inb(x, y) {
                    return false;
                }
                let i = Map::idx(x, y);
                if !self.map.terrain[i].buildable() || self.bmap[i] != 0 {
                    return false;
                }
                if self.map.mines.iter().any(|m| m.x == x && m.y == y && m.amount > 0.0) {
                    return false;
                }
            }
        }
        true
    }

    /// 建設可能か（理由付き）
    pub fn can_place(&self, nation: usize, kind: BKind, tx: i32, ty: i32) -> Result<(), &'static str> {
        if kind.city() {
            return Err(t!("新しい都市は築けない（都市は史実のものだけ）", "New cities cannot be founded (only the historical cities exist)", "无法新建城市（只有史实中的城市）"));
        }
        if !self.footprint_free(tx, ty) {
            return Err(t!("ここには建設できない", "Cannot build here", "此处无法建造"));
        }
        let own = |x: i32, y: i32| self.owner[Map::idx(x, y)];
        for dy in 0..2 {
            for dx in 0..2 {
                if own(tx + dx, ty + dy) as usize != nation {
                    return Err(t!("自国の領土内に建設する必要がある", "Must be built inside your territory", "必须建在己方领土内"));
                }
            }
        }
        if kind == BKind::Fort && self.fort_count(nation) >= self.fort_cap(nation) {
            return Err(t!("塹壕陣地の数が上限に達している（都市が多いほど上限が上がる）", "Trench line limit reached (more cities raise the limit)", "堑壕阵地数量已达上限（城市越多上限越高）"));
        }
        if kind == BKind::Dock {
            if !self.dock_sea(tx, ty).is_some() {
                return Err(t!("軍港は海に面した場所に建てる", "A naval base must face the sea", "军港必须建在临海处"));
            }
            if !naval_kinds(nation).iter().any(|k| k.naval()) {
                return Err(t!("この国は軍艦を造れない", "This state cannot build warships", "该国无法建造军舰"));
            }
        }
        Ok(())
    }

    /// 軍港の隣の海のマス（無ければ None）
    pub fn dock_sea(&self, tx: i32, ty: i32) -> Option<(i32, i32)> {
        for dy in -1..=2 {
            for dx in -1..=2 {
                let (x, y) = (tx + dx, ty + dy);
                if Map::inb(x, y) && self.map.terrain[Map::idx(x, y)] == Terrain::Sea {
                    return Some((x, y));
                }
            }
        }
        None
    }

    /// 築ける塹壕陣地の数（都市2つごとに1つ、2〜10）
    pub fn fort_cap(&self, nation: usize) -> u32 {
        (2 + self.nations[nation].cities / 2).min(10)
    }

    /// 建設中を含む要塞の数
    pub fn fort_count(&self, nation: usize) -> u32 {
        self.blds.iter().filter(|b| b.alive && b.nation as usize == nation && b.kind == BKind::Fort).count() as u32
    }

    pub fn spawn_building(&mut self, nation: usize, kind: BKind, tx: i32, ty: i32, complete: bool) -> Handle {
        let b = Building {
            gen: 0,
            alive: true,
            nation: nation as u8,
            kind,
            tx,
            ty,
            hp: if complete { kind.max_hp() } else { kind.max_hp() * 0.1 },
            progress: if complete { 1.0 } else { 0.0 },
            queue: VecDeque::new(),
            queue_t: 0.0,
            rally: None,
            cooldown: 0.0,
            city: u16::MAX,
        };
        let h = if let Some(i) = self.free_blds.pop() {
            let gen = self.blds[i as usize].gen + 1;
            self.blds[i as usize] = Building { gen, ..b };
            Handle { idx: i, gen }
        } else {
            self.blds.push(b);
            Handle { idx: self.blds.len() as u32 - 1, gen: 0 }
        };
        for dy in 0..2 {
            for dx in 0..2 {
                let i = Map::idx(tx + dx, ty + dy);
                self.bmap[i] = h.idx + 1;
                if kind.blocks() {
                    self.cost[i] = f32::INFINITY;
                }
            }
        }
        if kind.city() && complete {
            self.territory_dirty = true;
        }
        h
    }

    pub fn remove_building(&mut self, i: usize) {
        let b = &self.blds[i];
        if !b.alive {
            return;
        }
        let (tx, ty, city) = (b.tx, b.ty, b.kind.city());
        for dy in 0..2 {
            for dx in 0..2 {
                let ti = Map::idx(tx + dx, ty + dy);
                self.bmap[ti] = 0;
                self.cost[ti] = self.map.terrain[ti].move_cost();
            }
        }
        self.blds[i].alive = false;
        self.free_blds.push(i as u32);
        if city {
            self.territory_dirty = true;
        }
    }

    pub fn spawn_point(&self, h: Handle) -> Option<V2> {
        let b = self.bld(h)?;
        let mut fallback = None;
        for r in 1..6 {
            let mut cands = Vec::new();
            for dy in -r..=1 + r {
                for dx in -r..=1 + r {
                    let (x, y) = (b.tx + dx, b.ty + dy);
                    if dx > -r && dx < 1 + r && dy > -r && dy < 1 + r {
                        continue;
                    }
                    if Map::inb(x, y) && self.cost[Map::idx(x, y)].is_finite() {
                        cands.push(v2(x as f32 + 0.5, y as f32 + 0.5));
                    }
                }
            }
            if !cands.is_empty() {
                let free: Vec<V2> = cands
                    .iter()
                    .copied()
                    .filter(|&p| !self.units.iter().any(|u| u.alive && u.pos.dist(p) < 0.85))
                    .collect();
                if !free.is_empty() {
                    let k = (self.units.len().wrapping_mul(3) + (self.time * 7.3) as usize) % free.len();
                    return Some(free[k]);
                }
                if fallback.is_none() {
                    let k = (self.units.len() + (self.time * 7.3) as usize) % cands.len();
                    fallback = Some(cands[k]);
                }
            }
        }
        fallback
    }

    pub fn passable_at(&self, p: V2) -> bool {
        let (x, y) = p.tile();
        Map::inb(x, y) && self.cost[Map::idx(x, y)].is_finite()
    }

    /// 海のマスか（船が通れる）
    pub fn sea_at(&self, p: V2) -> bool {
        let (x, y) = p.tile();
        Map::inb(x, y) && self.sea_cost[Map::idx(x, y)].is_finite()
    }

    /// 陸の兵なら通れる陸、船なら海
    pub fn walkable(&self, p: V2, naval: bool) -> bool {
        if naval {
            self.sea_at(p)
        } else {
            self.passable_at(p)
        }
    }

    pub fn line_clear_for(&self, a: V2, b: V2, naval: bool) -> bool {
        let d = b - a;
        let l = d.len();
        let n = (l / 0.4).ceil() as i32;
        for s in 1..=n {
            let p = a + d * (s as f32 / n as f32);
            if !self.walkable(p, naval) {
                return false;
            }
        }
        true
    }

    pub fn push_msg(&mut self, text: String, color: [f32; 3]) {
        self.msgs.push(Msg { text, time: self.time, color });
        if self.msgs.len() > 60 {
            self.msgs.remove(0);
        }
    }

    /// 1年あたりの秒数
    pub fn sec_per_year(&self) -> f32 {
        SEC_PER_YEAR
    }

    /// いまの年（小数）
    pub fn year_f(&self) -> f32 {
        START_YEAR + self.years
    }

    /// いまの年・月
    pub fn ym(&self) -> (i32, i32) {
        let y = self.year_f();
        let yi = y.floor() as i32;
        let m = (((y - yi as f32) * 12.0).floor() as i32).clamp(0, 11) + 1;
        (yi, m)
    }

    /// 日付の表示（1914年7月）
    pub fn date_str(&self) -> String {
        let (y, m) = self.ym();
        match crate::lang::lang() {
            crate::lang::Lang::En => format!("{} {}", month_en(m), y),
            _ => format!("{}年{}月", y, m),
        }
    }

    pub fn power_cap(&self) -> f32 {
        if self.era == Era::Early {
            4.0
        } else {
            6.0
        }
    }

    /// 生産の倍率：開戦時がもっとも高く、戦争が長引くほど下がる（1919年に 0.55）
    pub fn era_mult(&self) -> f32 {
        let t = (self.years / (END_YEAR - START_YEAR)).clamp(0.0, 1.3);
        (1.0 - 0.45 * t).max(0.4)
    }

    /// 交戦しているか（天使とはどの国も戦う）
    pub fn at_war(&self, a: usize, b: usize) -> bool {
        a != b && (a == ANGEL || b == ANGEL || self.war[a][b])
    }

    /// 同じ陣営（中立同士は同盟ではない）
    pub fn allied(&self, a: usize, b: usize) -> bool {
        a != b && a < NATIONS.len() && b < NATIONS.len() && self.nations[a].bloc != Bloc::Neutral && self.nations[a].bloc == self.nations[b].bloc
    }

    /// 宣戦する。中立国を攻めると、その国は相手の陣営に加わる。同盟国はともに戦う
    pub fn declare_war(&mut self, a: usize, b: usize, forced: bool) -> bool {
        if a == b || a >= NATIONS.len() || b >= NATIONS.len() || self.war[a][b] || !self.nations[a].alive || !self.nations[b].alive {
            return false;
        }
        if self.allied(a, b) {
            return false;
        }
        if !forced && self.truce[a][b] > self.years {
            return false;
        }
        // 中立の国が攻めるときは、攻めた相手の敵の陣営に入る
        if self.nations[a].bloc == Bloc::Neutral && self.nations[b].bloc != Bloc::Neutral {
            let other = if self.nations[b].bloc == Bloc::Central { Bloc::Entente } else { Bloc::Central };
            self.nations[a].bloc = other;
        }
        if self.nations[b].bloc == Bloc::Neutral && self.nations[a].bloc != Bloc::Neutral {
            let other = if self.nations[a].bloc == Bloc::Central { Bloc::Entente } else { Bloc::Central };
            self.nations[b].bloc = other;
        }
        self.set_war(a, b);
        true
    }

    fn set_war(&mut self, a: usize, b: usize) {
        if self.war[a][b] || a == b {
            return;
        }
        self.war[a][b] = true;
        self.war[b][a] = true;
        self.war_start[a][b] = self.years;
        self.war_start[b][a] = self.years;
        self.peace_offer[a] = false;
        self.peace_offer[b] = false;
        let p = self.player;
        if a == p || b == p {
            let t = tf!("{} が {} に宣戦布告した", "{} declared war on {}", "{}向{}宣战", tx(self.nations[a].name), tx(self.nations[b].name));
            self.push_msg(t, [1.0, 0.45, 0.35]);
        }
    }

    /// 中立国が陣営に加わって参戦する：相手の陣営の、すでに戦っている国すべてと交戦する
    pub fn join_war(&mut self, n: usize, bloc: Bloc) {
        if !self.nations[n].alive || bloc == Bloc::Neutral {
            return;
        }
        self.nations[n].bloc = bloc;
        for o in 0..NATIONS.len() {
            if o == n || !self.nations[o].alive {
                continue;
            }
            let ob = self.nations[o].bloc;
            if ob != Bloc::Neutral && ob != bloc {
                self.set_war(n, o);
            }
        }
    }

    pub fn make_peace(&mut self, a: usize, b: usize, truce_years: f32) {
        if !self.war[a][b] {
            return;
        }
        // 天使とは和睦できない
        if self.descent.is_some() {
            return;
        }
        self.war[a][b] = false;
        self.war[b][a] = false;
        self.truce[a][b] = self.years + truce_years;
        self.truce[b][a] = self.years + truce_years;
        if a == self.player || b == self.player {
            let o = if a == self.player { b } else { a };
            self.peace_offer[o] = false;
            let t = tf!("{} と和睦した", "Made peace with {}", "与{}议和", tx(self.nations[o].name));
            self.push_msg(t, [0.6, 0.9, 0.6]);
        }
    }

    /// 和睦を申し入れる。プレイヤーが当事者なら返答を待ち（false）、それ以外は即座に和睦する（true）
    pub fn propose_peace(&mut self, a: usize, b: usize, truce_years: f32, why: &str) -> bool {
        let p = self.player;
        if (a != p && b != p) || self.ai_player {
            self.make_peace(a, b, truce_years);
            return true;
        }
        let o = if a == p { b } else { a };
        if !self.war[a][b] || self.descent.is_some() || self.peace_asks.iter().any(|q| q.from == o) {
            return false;
        }
        let t = tf!("{} が和睦を申し入れてきた（{}）", "{} offers peace ({})", "{}提出议和（{}）", tx(self.nations[o].name), why);
        self.push_msg(t, [0.7, 0.9, 1.0]);
        self.peace_asks.push_back(PeaceAsk { from: o, years: truce_years, why: why.to_string() });
        false
    }

    /// 返答待ちの申し入れのうち、既に意味のないもの（交戦が終わった・滅んだ）を捨てる
    pub fn prune_peace_asks(&mut self) {
        let p = self.player;
        let (war, nations) = (&self.war, &self.nations);
        self.peace_asks.retain(|q| q.from != p && nations[q.from].alive && war[p][q.from]);
    }

    /// 先頭の和睦の申し入れに答える
    pub fn answer_peace(&mut self, accept: bool) {
        self.prune_peace_asks();
        let Some(q) = self.peace_asks.pop_front() else { return };
        let p = self.player;
        if accept {
            self.make_peace(p, q.from, q.years);
        } else {
            self.peace_offer[q.from] = false;
            self.offer_cd[q.from] = self.years + 8.0;
            let t = tf!("{} の和睦の申し入れを退けた（交戦を続ける）", "Refused {}'s offer of peace (the war goes on)", "拒绝了{}的议和（继续交战）", tx(self.nations[q.from].name));
            self.push_msg(t, [1.0, 0.6, 0.4]);
        }
    }

    // ------------------------------------------------------------ 領土
    pub fn recompute_territory(&mut self) {
        let n = MAP_W * MAP_H;
        let mut best = vec![f32::MAX; n];
        self.owner.fill(NONE);
        for b in self.blds.iter() {
            if !b.alive || !b.kind.city() || !b.done() {
                continue;
            }
            let r = b.kind.radius();
            let c = b.center();
            let (cx, cy) = (c.x as i32, c.y as i32);
            for dy in -r..=r {
                for dx in -r..=r {
                    let (x, y) = (cx + dx, cy + dy);
                    if !Map::inb(x, y) {
                        continue;
                    }
                    let d = ((x as f32 + 0.5 - c.x).powi(2) + (y as f32 + 0.5 - c.y).powi(2)).sqrt();
                    let rr = r as f32;
                    if d > rr {
                        continue;
                    }
                    let i = Map::idx(x, y);
                    if self.map.terrain[i] == Terrain::Sea {
                        continue;
                    }
                    let score = d / rr;
                    if score < best[i] {
                        best[i] = score;
                        self.owner[i] = b.nation;
                    }
                }
            }
        }
        let mut tc = vec![0u32; self.nations.len()];
        for &o in self.owner.iter() {
            if (o as usize) < tc.len() {
                tc[o as usize] += 1;
            }
        }
        self.tile_count = tc;
        self.territory_dirty = false;
        self.territory_version += 1;
    }

    // ------------------------------------------------------------ 統計
    pub fn update_stats(&mut self) {
        for n in self.nations.iter_mut() {
            n.workers = 0;
            n.soldiers = 0;
            n.q_workers = 0;
            n.q_soldiers = 0;
            n.cities = 0;
            n.mil_power = 0.0;
            n.eco_power = 0.0;
            n.upkeep = 0.0;
            n.tiles = 0;
        }
        for u in self.units.iter() {
            if !u.alive {
                continue;
            }
            let n = &mut self.nations[u.nation as usize];
            if let Some(sh) = &u.ship {
                // 船に乗っている兵も数える（輸送船そのものは数えない）
                for c in sh.cargo.iter() {
                    let crew = c.kind.crew();
                    n.soldiers += crew;
                    n.mil_power += c.power * c.hp.max(0.2) * crew as f32;
                    n.upkeep += 0.07 * c.power * crew as f32;
                }
                continue;
            }
            if u.divine || u.hero != NONE {
                // 天使は天のもの、英雄は人口の上限・維持費の外
                n.mil_power += u.power * (u.hp / u.max_hp()).max(0.2);
            } else if u.kind.military() {
                let crew = u.kind.crew();
                n.soldiers += crew;
                n.mil_power += u.power * (u.hp / u.max_hp()).max(0.2) * crew as f32;
                n.upkeep += 0.07 * u.power * crew as f32;
            } else {
                n.workers += 1;
                n.eco_power += u.power;
            }
        }
        for b in self.blds.iter() {
            if !b.alive {
                continue;
            }
            let n = &mut self.nations[b.nation as usize];
            if b.kind.city() && b.done() {
                n.cities += 1;
            }
            for k in b.queue.iter() {
                if k.military() {
                    n.q_soldiers += 1;
                } else {
                    n.q_workers += 1;
                }
            }
        }
        for (k, n) in self.nations.iter_mut().enumerate() {
            n.tiles = self.tile_count.get(k).copied().unwrap_or(0);
        }
        for n in self.nations.iter_mut() {
            n.upkeep *= n.tm.upkeep;
            n.strength = n.mil_power * 10.0 + n.cities as f32 * 25.0 + n.eco_power * 4.0 + n.tiles as f32 * 0.03;
        }
    }

    // ------------------------------------------------------------ 命令
    /// プレイヤーの部隊への命令を数える（指揮中の一時停止を解くため）
    fn note_player_order(&mut self, hs: &[Handle]) {
        let p = self.player;
        if hs.iter().any(|&h| self.unit(h).map_or(false, |u| u.nation as usize == p)) {
            self.player_orders = self.player_orders.wrapping_add(1);
        }
    }

    pub fn order_move(&mut self, hs: &[Handle], goal: V2, attack: bool) {
        self.note_player_order(hs);
        // 海の向こうへ向かう陸の部隊は、港で船に乗る
        let plan = self.plan_ferry(hs, goal, attack);
        let n = hs.len();
        let cols = (n as f32).sqrt().ceil() as i32;
        for (k, &h) in hs.iter().enumerate() {
            let (r, c) = (k as i32 / cols.max(1), k as i32 % cols.max(1));
            let off = v2((c - cols / 2) as f32 * 0.8, (r - cols / 2) as f32 * 0.8);
            let mut g = goal + if n > 1 { off } else { v2(0.0, 0.0) };
            let naval = self.unit(h).map_or(false, |u| u.kind.naval());
            if naval {
                if !self.sea_at(g) {
                    g = goal;
                }
            } else if !self.passable_at(g) {
                g = goal;
            }
            let fp = plan.filter(|p| !naval && self.unit(h).map_or(false, |u| self.needs_ferry(u.pos, p.goal)));
            if let Some(u) = self.unit_mut(h) {
                if u.ship.is_some() {
                    // 船団・兵を運ぶ船は、命令では動かない
                    continue;
                }
                match fp {
                    Some(mut p) => {
                        p.goal = g;
                        u.order = Order::Move(p.embark);
                        u.ferry = Some(p);
                    }
                    None => {
                        u.order = if attack && u.kind.military() { Order::AttackMove(g) } else { Order::Move(g) };
                        u.ferry = None;
                    }
                }
                u.resume = None;
                u.build_queue.clear();
                u.reset_nav();
            }
        }
    }

    pub fn order_attack(&mut self, hs: &[Handle], t: Target) {
        self.note_player_order(hs);
        // 海の向こうの目標へは、船で渡って進軍する
        if let Some((tp, _, _)) = self.target_info(t) {
            let far: Vec<Handle> = hs.iter().copied().filter(|&h| self.unit(h).map_or(false, |u| !u.kind.naval() && self.needs_ferry(u.pos, tp))).collect();
            if !far.is_empty() {
                self.order_move(&far, tp, true);
            }
        }
        for &h in hs {
            if self.unit(h).map_or(false, |u| u.ferry.is_some() || u.ship.is_some()) {
                continue;
            }
            if let Some(u) = self.unit_mut(h) {
                u.order = Order::Attack(t);
                u.ferry = None;
                u.resume = None;
                u.build_queue.clear();
                u.reset_nav();
            }
        }
    }

    pub fn order_gather(&mut self, hs: &[Handle], src: Src) {
        self.note_player_order(hs);
        for &h in hs {
            if let Some(u) = self.unit_mut(h) {
                if u.kind == UnitKind::Worker {
                    u.ferry = None;
                    u.order = Order::Gather { src, phase: 0 };
                    u.build_queue.clear();
                    u.reset_nav();
                }
            }
        }
    }

    pub fn order_build(&mut self, hs: &[Handle], b: Handle) {
        self.note_player_order(hs);
        for &h in hs {
            if let Some(u) = self.unit_mut(h) {
                if u.kind == UnitKind::Worker {
                    u.ferry = None;
                    u.order = Order::Build(b);
                    u.build_queue.clear();
                    u.reset_nav();
                }
            }
        }
    }

    /// 建設を予約する：建設中の民は今の建設を終えてから順に取りかかり、それ以外の民はすぐに建て始める
    pub fn queue_build(&mut self, hs: &[Handle], b: Handle) {
        self.note_player_order(hs);
        for &h in hs {
            let busy = match self.unit(h) {
                Some(u) if u.kind == UnitKind::Worker => match u.order {
                    Order::Build(cur) => cur != b && self.bld(cur).map_or(false, |x| !x.done()),
                    _ => false,
                },
                _ => continue,
            };
            let u = self.unit_mut(h).unwrap();
            if busy {
                if !u.build_queue.contains(&b) {
                    u.build_queue.push_back(b);
                }
            } else {
                u.order = Order::Build(b);
                u.build_queue.clear();
                u.reset_nav();
            }
        }
    }

    /// 予約された次の建設に取りかかる（なければ false）
    fn next_queued_build(&mut self, i: usize) -> bool {
        let ni = self.units[i].nation as usize;
        while let Some(b) = self.units[i].build_queue.pop_front() {
            if self.bld(b).map_or(false, |x| x.nation as usize == ni && !x.done()) {
                let u = &mut self.units[i];
                u.order = Order::Build(b);
                u.reset_nav();
                return true;
            }
        }
        false
    }

    pub fn try_pay(&mut self, nation: usize, cost: [f32; 3]) -> bool {
        let r = &mut self.nations[nation].res;
        if r[0] >= cost[0] && r[1] >= cost[1] && r[2] >= cost[2] {
            for k in 0..3 {
                r[k] -= cost[k];
            }
            true
        } else {
            false
        }
    }

    pub fn place_building(&mut self, nation: usize, kind: BKind, tx: i32, ty: i32) -> Result<Handle, &'static str> {
        self.can_place(nation, kind, tx, ty)?;
        if !self.try_pay(nation, kind.cost()) {
            return Err(t!("資源が足りない", "Not enough resources", "资源不足"));
        }
        Ok(self.spawn_building(nation, kind, tx, ty, false))
    }

    pub fn can_train(&self, b: &Building, kind: UnitKind) -> bool {
        if !b.done() {
            return false;
        }
        match b.kind {
            BKind::Capital | BKind::Town => kind == UnitKind::Worker,
            // 兵器（戦車・Uボート・駆逐艦）は訓練せず、兵を割り当てて造る
            BKind::Barracks => kind.land_mil() && !kind.weapon(),
            BKind::Dock => false,
            BKind::Farm | BKind::Fort => false,
        }
    }

    /// 兵にする民を選ぶ（手の空いた民、次いで兵営に近い民）
    fn pick_conscript(&self, ni: usize, p: V2) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, u) in self.units.iter().enumerate() {
            if !u.alive || u.nation as usize != ni || u.kind != UnitKind::Worker || u.hero != NONE {
                continue;
            }
            let busy = match u.order {
                Order::Idle => 0.0,
                Order::Gather { .. } => 30.0,
                _ => 80.0,
            };
            let d = u.pos.dist(p) + busy;
            if best.map_or(true, |b| d < b.0) {
                best = Some((d, i));
            }
        }
        best.map(|b| b.1)
    }

    pub fn enqueue(&mut self, bh: Handle, kind: UnitKind) -> Result<(), &'static str> {
        let Some(b) = self.bld(bh) else { return Err("") };
        if !self.can_train(b, kind) {
            return Err(t!("ここでは訓練できない", "Cannot train that here", "此处无法训练"));
        }
        let ni = b.nation as usize;
        let bp = b.center();
        if !self.kind_available(ni, kind) {
            return Err(t!("この国ではまだ造れない", "This state cannot build that (yet)", "该国（尚）无法建造"));
        }
        if b.queue.len() >= 5 {
            return Err(t!("訓練待ちが一杯", "Training queue is full", "训练队列已满"));
        }
        let mut conscript = None;
        if kind.military() {
            // 兵は民から徴兵する（その分だけ生産が減る）
            match self.pick_conscript(ni, bp) {
                Some(i) => conscript = Some(i),
                None => return Err(t!("徴兵できる民がいない（兵は民から徴兵する）", "No workers to conscript (soldiers are drafted from workers)", "没有可征召的民（兵由民征召）")),
            }
        } else if self.nations[ni].pop() >= self.pop_cap(ni) {
            return Err(t!("民・兵が人口の上限に達している", "Population limit reached", "民与兵已达人口上限"));
        }
        if !self.try_pay(ni, self.train_cost(ni, kind)) {
            return Err(t!("資源が足りない", "Not enough resources", "资源不足"));
        }
        if let Some(i) = conscript {
            self.kill_unit(i);
            self.nations[ni].workers = self.nations[ni].workers.saturating_sub(1);
        }
        let b = self.bld_mut(bh).unwrap();
        b.queue.push_back(kind);
        let n = &mut self.nations[ni];
        if kind.military() {
            n.q_soldiers += 1;
        } else {
            n.q_workers += 1;
        }
        Ok(())
    }

    /// 兵器に割り当てられる兵か（陸の兵で、兵器・英雄・天使でなく、渡海中でない）
    pub fn can_crew(u: &Unit) -> bool {
        u.alive && u.kind.land_mil() && !u.kind.weapon() && u.hero == NONE && !u.divine && u.ferry.is_none() && u.ship.is_none()
    }

    /// 兵器に兵を割り当てる：選んだ兵のうち必要な数を兵器に乗せる（兵は兵器に変わり、戻せない）。
    /// 戦車は自国の領内で、軍艦は自国の軍港のそばで。兵器の能力は乗った兵の能力の平均
    pub fn crew_weapon(&mut self, hs: &[Handle], kind: UnitKind) -> Result<Handle, &'static str> {
        if !kind.weapon() {
            return Err("");
        }
        let cand: Vec<usize> = hs.iter().filter_map(|&h| self.unit(h).map(|u| (h.idx as usize, u))).filter(|(_, u)| Self::can_crew(u)).map(|x| x.0).collect();
        let Some(&first) = cand.first() else {
            return Err(t!("兵器に割り当てる兵を選ぶ", "Select soldiers to crew the weapon", "请选择分配给兵器的士兵"));
        };
        let ni = self.units[first].nation as usize;
        if !self.kind_available(ni, kind) {
            return Err(t!("この国ではまだ造れない", "This state cannot build that (yet)", "该国（尚）无法建造"));
        }
        let need = kind.crew() as usize;
        let mut cand: Vec<usize> = cand.into_iter().filter(|&i| self.units[i].nation as usize == ni).collect();
        if cand.len() < need {
            return Err(match kind {
                UnitKind::Tank => t!("戦車には兵が5人要る", "A tank needs 5 soldiers", "坦克需要5名士兵"),
                UnitKind::Submarine => t!("Uボートには兵が4人要る", "A U-boat needs 4 soldiers", "U艇需要4名士兵"),
                _ => t!("駆逐艦には兵が6人要る", "A destroyer needs 6 soldiers", "驱逐舰需要6名士兵"),
            });
        }
        // 兵の集まりの中心に近い兵から乗せる
        let mut c = v2(0.0, 0.0);
        for &i in cand.iter() {
            c = c + self.units[i].pos;
        }
        c = c * (1.0 / cand.len() as f32);
        cand.sort_by(|&a, &b| self.units[a].pos.dist(c).partial_cmp(&self.units[b].pos.dist(c)).unwrap());
        cand.truncate(need);
        let anchor = self.units[cand[0]].pos;
        let spawn = if kind.naval() {
            // 自国の軍港のそば
            let mut best: Option<(f32, V2)> = None;
            for b in self.blds.iter() {
                if b.alive && b.done() && b.kind == BKind::Dock && b.nation as usize == ni {
                    let d = b.center().dist(anchor);
                    if d <= 10.0 && best.map_or(true, |x| d < x.0) {
                        if let Some((x, y)) = self.dock_sea(b.tx, b.ty) {
                            best = Some((d, v2(x as f32 + 0.5, y as f32 + 0.5)));
                        }
                    }
                }
            }
            match best {
                Some(b) => b.1,
                None => return Err(t!("軍艦は自国の軍港のそばの兵に割り当てる", "Ships are crewed by soldiers near one of your docks", "军舰须由己方军港附近的士兵操纵")),
            }
        } else {
            let (x, y) = anchor.tile();
            if self.owner[Map::idx(x, y)] as usize != ni {
                return Err(t!("戦車は自国の領内の兵に割り当てる", "Tanks are crewed by soldiers inside your territory", "坦克须由己方领土内的士兵操纵"));
            }
            anchor
        };
        if !self.try_pay(ni, self.train_cost(ni, kind)) {
            return Err(t!("資源が足りない", "Not enough resources", "资源不足"));
        }
        let mut power = 0.0;
        let mut hpr = 0.0;
        for &i in cand.iter() {
            let u = &self.units[i];
            power += u.power;
            hpr += u.hp / u.max_hp();
        }
        power /= need as f32;
        hpr /= need as f32;
        let sel = cand.iter().any(|&i| self.units[i].selected);
        for &i in cand.iter() {
            self.kill_unit(i);
        }
        let h = self.spawn_unit(ni, kind, spawn, power);
        if let Some(u) = self.unit_mut(h) {
            u.hp = u.max_hp() * hpr.clamp(0.1, 1.0);
            u.selected = sel;
        }
        Ok(h)
    }

    pub fn upgrade_cost(&self, u: &Unit) -> [f32; 3] {
        let p = u.power;
        let ni = u.nation as usize;
        let m = if ni < NATIONS.len() { self.nations[ni].tm.upgrade } else { 1.0 };
        let total = (40.0 * p * m * u.kind.crew() as f32).round();
        // 蓄えの多い資源から優先して払う（資源の偏りで余らせないため）。蓄えが乏しいときは兵は糧・銅、民は糧・木が基本
        let base = if u.kind.military() { [25.0, 0.0, 15.0] } else { [25.0, 15.0, 0.0] };
        let res = if ni < NATIONS.len() { self.nations[ni].res } else { [0.0; 3] };
        let have = res[0].max(0.0) + res[1].max(0.0) + res[2].max(0.0);
        let w: [f32; 3] = if have >= total && have > 0.0 { [res[0].max(0.0), res[1].max(0.0), res[2].max(0.0)] } else { base };
        let ws = w[0] + w[1] + w[2];
        let mut c = [0.0f32; 3];
        for k in 0..3 {
            c[k] = (total * w[k] / ws).floor();
        }
        // 端数は最も多い資源で払う
        let big = (0..3).max_by(|&a, &b| w[a].partial_cmp(&w[b]).unwrap()).unwrap();
        c[big] += total - (c[0] + c[1] + c[2]);
        c
    }

    /// 訓練（兵）・教化（民）：能力を高める
    pub fn upgrade_unit(&mut self, h: Handle) -> Result<(), &'static str> {
        let cap = self.power_cap();
        let Some(u) = self.unit(h) else { return Err("") };
        if u.hero != NONE {
            return Err(t!("英雄は鍛錬できない", "Heroes cannot be trained", "英雄无法训练"));
        }
        if u.kind == UnitKind::Transport || u.divine {
            return Err("");
        }
        if u.power + 0.01 >= cap {
            return Err(t!("この時代の能力上限に達している", "Already at this era's power limit", "已达本时代能力上限"));
        }
        let (x, y) = u.pos.tile();
        if self.owner[Map::idx(x, y)] != u.nation {
            return Err(t!("自国領内でのみ鍛錬できる", "Can only train inside your territory", "只能在己方领土内训练"));
        }
        let ni = u.nation as usize;
        let cost = self.upgrade_cost(u);
        if !self.try_pay(ni, cost) {
            return Err(t!("資源が足りない", "Not enough resources", "资源不足"));
        }
        let u = self.unit_mut(h).unwrap();
        let ratio = u.hp / u.max_hp();
        u.power = (u.power + 0.5).min(cap);
        u.hp = ratio * u.max_hp();
        Ok(())
    }

    /// 兵⇔民の転換（能力は 2/3 に目減り）
    pub fn convert_unit(&mut self, h: Handle) -> Result<(), &'static str> {
        let Some(u) = self.unit(h) else { return Err("") };
        if u.hero != NONE {
            return Err(t!("英雄は民にできない", "Heroes cannot become workers", "英雄无法转为民"));
        }
        if u.kind.naval() || u.divine || u.ferry.is_some() {
            return Err(t!("船は民にできない", "Ships cannot become workers", "船只无法转为民"));
        }
        if u.kind.weapon() {
            return Err(t!("兵器に割り当てた兵は戻せない", "Crews assigned to a weapon cannot be released", "已分配给兵器的士兵无法撤回"));
        }
        let ni = u.nation as usize;
        let to_mil = !u.kind.military();
        let u = self.unit_mut(h).unwrap();
        let ratio = u.hp / u.max_hp();
        u.power = (u.power * 2.0 / 3.0).max(0.34);
        u.kind = if to_mil { UnitKind::Infantry } else { UnitKind::Worker };
        u.hp = ratio * u.max_hp();
        u.order = Order::Idle;
        u.carry = 0.0;
        let n = &mut self.nations[ni];
        if to_mil {
            n.soldiers += 1;
            n.workers -= 1;
        } else {
            n.soldiers -= 1;
            n.workers += 1;
        }
        Ok(())
    }

    // ------------------------------------------------------------ 検索
    pub fn rebuild_grid(&mut self) {
        for c in self.grid.iter_mut() {
            c.clear();
        }
        for (i, u) in self.units.iter().enumerate() {
            if u.alive {
                let gx = (u.pos.x as i32 / GCELL).clamp(0, GW - 1);
                let gy = (u.pos.y as i32 / GCELL).clamp(0, GH - 1);
                self.grid[(gy * GW + gx) as usize].push(i as u32);
            }
        }
    }

    /// 敵対ユニット探索（建物の射撃など、攻め手の兵種を問わない場合）
    pub fn nearest_enemy_unit(&self, nation: usize, p: V2, r: f32) -> Option<usize> {
        self.nearest_enemy_unit_for(nation, None, p, r)
    }

    /// その国にとって敵のユニットか（無制限潜水艦作戦中は、中立の米国の輸送船も狙う）
    pub fn hostile(&self, nation: usize, u: &Unit) -> bool {
        let o = u.nation as usize;
        if self.at_war(nation, o) {
            return true;
        }
        self.sea.unrestricted && u.kind == UnitKind::Transport && o == USA && nation < NATIONS.len() && self.nations[nation].bloc == Bloc::Central && !self.allied(nation, o)
    }

    /// 攻め手の兵種で攻撃できる相手か（Uボートは船だけ、潜っている Uボートは駆逐艦だけが狙える）
    pub fn can_hit(ak: Option<UnitKind>, tk: UnitKind) -> bool {
        match ak {
            Some(UnitKind::Transport) => false,
            Some(UnitKind::Submarine) => tk.naval(),
            Some(UnitKind::Destroyer) => tk.naval(),
            Some(UnitKind::Worker) => !tk.naval(),
            _ => tk != UnitKind::Submarine,
        }
    }

    pub fn nearest_enemy_unit_for(&self, nation: usize, ak: Option<UnitKind>, p: V2, r: f32) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        let g0x = ((p.x - r) as i32 / GCELL).clamp(0, GW - 1);
        let g1x = ((p.x + r) as i32 / GCELL).clamp(0, GW - 1);
        let g0y = ((p.y - r) as i32 / GCELL).clamp(0, GH - 1);
        let g1y = ((p.y + r) as i32 / GCELL).clamp(0, GH - 1);
        for gy in g0y..=g1y {
            for gx in g0x..=g1x {
                for &i in self.grid[(gy * GW + gx) as usize].iter() {
                    let u = &self.units[i as usize];
                    if !u.alive || !Self::can_hit(ak, u.kind) || !self.hostile(nation, u) {
                        continue;
                    }
                    let d = u.pos.dist(p);
                    // 兵を優先（Uボートは輸送船を優先）
                    let d2 = if ak == Some(UnitKind::Submarine) && u.kind == UnitKind::Transport {
                        d - 3.0
                    } else if u.kind.military() {
                        d
                    } else {
                        d + 2.0
                    };
                    if d <= r && best.map_or(true, |b| d2 < b.0) {
                        best = Some((d2, i as usize));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    pub fn nearest_enemy_bld(&self, nation: usize, p: V2, r: f32) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, b) in self.blds.iter().enumerate() {
            if !b.alive || !self.at_war(nation, b.nation as usize) {
                continue;
            }
            let d = b.center().dist(p);
            if d <= r && best.map_or(true, |x| d < x.0) {
                best = Some((d, i));
            }
        }
        best.map(|b| b.1)
    }

    /// 国ごとの完成した城邑の一覧を作り直す
    fn rebuild_city_list(&mut self) {
        let n = self.nations.len();
        if self.city_list.len() != n {
            self.city_list = vec![Vec::new(); n];
        }
        for l in self.city_list.iter_mut() {
            l.clear();
        }
        for (i, b) in self.blds.iter().enumerate() {
            if b.alive && b.kind.city() && b.done() {
                if let Some(l) = self.city_list.get_mut(b.nation as usize) {
                    l.push(i as u32);
                }
            }
        }
    }

    pub fn nearest_dropsite(&self, nation: usize, p: V2) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        // 一覧は毎ステップ作り直すが、途中で落ちた城邑は確かめて除く
        let list: Vec<usize> = match self.city_list.get(nation) {
            Some(l) => l.iter().map(|&i| i as usize).collect(),
            None => (0..self.blds.len()).collect(),
        };
        for i in list {
            let b = &self.blds[i];
            if b.alive && b.nation as usize == nation && b.kind.city() && b.done() {
                let d = b.center().dist(p);
                if best.map_or(true, |x| d < x.0) {
                    best = Some((d, i));
                }
            }
        }
        best.map(|b| b.1)
    }

    pub fn nearest_forest_for(&self, nation: Option<usize>, p: V2, maxr: i32) -> Option<(i32, i32)> {
        let (cx, cy) = p.tile();
        let mut best: Option<(i32, (i32, i32))> = None;
        for r in 0..=maxr {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    if Map::inb(x, y) {
                        let idx = Map::idx(x, y);
                        if self.map.terrain[idx] == Terrain::Forest {
                            if let Some(n) = nation {
                                let o = self.owner[idx];
                                if o != NONE && o as usize != n {
                                    continue;
                                }
                            }
                            let d = dx * dx + dy * dy;
                            if best.map_or(true, |b| d < b.0) {
                                best = Some((d, (x, y)));
                            }
                        }
                    }
                }
            }
            if best.is_some() {
                return best.map(|b| b.1);
            }
        }
        None
    }

    pub fn nearest_forest(&self, p: V2, maxr: i32) -> Option<(i32, i32)> {
        self.nearest_forest_for(None, p, maxr)
    }

    pub fn nearest_mine_for(&self, nation: Option<usize>, p: V2, maxd: f32) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, m) in self.map.mines.iter().enumerate() {
            if m.amount <= 0.0 {
                continue;
            }
            if let Some(n) = nation {
                if Map::inb(m.x, m.y) {
                    let o = self.owner[Map::idx(m.x, m.y)];
                    if o != NONE && o as usize != n {
                        continue;
                    }
                }
            }
            let d = v2(m.x as f32 + 0.5, m.y as f32 + 0.5).dist(p);
            if d <= maxd && best.map_or(true, |b| d < b.0) {
                best = Some((d, i));
            }
        }
        best.map(|b| b.1)
    }

    pub fn nearest_mine(&self, p: V2, maxd: f32) -> Option<usize> {
        self.nearest_mine_for(None, p, maxd)
    }

    pub fn target_info(&self, t: Target) -> Option<(V2, f32, usize)> {
        match t {
            Target::Unit(h) => self.unit(h).map(|u| (u.pos, 0.3, u.nation as usize)),
            Target::Bld(h) => self.bld(h).map(|b| (b.center(), 1.0, b.nation as usize)),
        }
    }

    // ------------------------------------------------------------ 戦闘
    fn damage_unit(&mut self, ti: usize, dmg: f32, attacker: Option<Handle>, an: usize) {
        let killed;
        // 天使を傷つけられるのは英雄だけ
        let by_hero = attacker.and_then(|h| self.unit(h)).map_or(false, |a| a.hero != NONE);
        {
            let u = &mut self.units[ti];
            if !u.alive || (u.divine && !by_hero) {
                return;
            }
            // 英雄の一撃は天使によく効く
            u.hp -= if u.divine { dmg * 3.0 } else { dmg };
            u.flash = 0.15;
            u.last_attacker = attacker;
            killed = u.hp <= 0.0;
            if !killed && u.kind.military() && u.kind != UnitKind::Transport && u.order == Order::Idle {
                if let Some(a) = attacker {
                    u.order = Order::Attack(Target::Unit(a));
                }
            }
        }
        if killed {
            let vn = self.units[ti].nation as usize;
            // 損害の記録（世論に響く）。船が沈めば積んでいた兵も失う
            let mut loss = if self.units[ti].kind == UnitKind::Transport { 0.0 } else { self.units[ti].strength() };
            if let Some(sh) = &self.units[ti].ship {
                loss += sh.cargo.iter().map(|c| c.power * c.kind.crew() as f32).sum::<f32>();
            }
            let sunk_transport = self.units[ti].kind == UnitKind::Transport;
            if vn < NATIONS.len() {
                self.nations[vn].lost += loss;
                self.nations[vn].lost_total += loss;
            }
            if an < NATIONS.len() {
                self.nations[an].inflicted += loss;
                self.nations[an].inflicted_total += loss;
            }
            if sunk_transport {
                self.ship_sunk(ti, an);
            }
            self.kill_unit(ti);
        }
    }

    fn damage_bld(&mut self, bi: usize, dmg: f32, an: usize) {
        let vn = self.blds[bi].nation as usize;
        let dmg = if vn < NATIONS.len() { dmg * self.nations[vn].tm.def } else { dmg };
        let b = &mut self.blds[bi];
        if !b.alive {
            return;
        }
        b.hp -= dmg;
        if b.hp > 0.0 {
            return;
        }
        if b.kind.city() && b.done() {
            self.capture_city(bi, an);
        } else {
            let (vn, kind) = (b.nation as usize, b.kind);
            self.remove_building(bi);
            if vn == self.player {
                self.push_msg(tf!("{}が破壊された", "Our {} was destroyed", "{}被摧毁了", kind.name()), [1.0, 0.5, 0.4]);
            }
        }
    }

    fn capture_city(&mut self, bi: usize, an: usize) {
        let old = self.blds[bi].nation as usize;
        let was_cap = self.blds[bi].kind == BKind::Capital;
        {
            let b = &mut self.blds[bi];
            b.nation = an as u8;
            b.hp = b.kind.max_hp() * 0.3;
            b.queue.clear();
            b.queue_t = 0.0;
            b.rally = None;
            b.kind = BKind::Town;
        }
        let bh = self.bh(bi);
        if self.nations[an].capital.and_then(|h| self.bld(h)).is_none() {
            self.blds[bi].kind = BKind::Capital;
            self.nations[an].capital = Some(bh);
        }
        self.territory_dirty = true;
        if an < NATIONS.len() {
            self.nations[an].won_cities += 1;
        }
        if old < NATIONS.len() {
            self.nations[old].lost_cities += 1;
        }
        let p = self.player;
        let name = match self.blds[bi].city_name() {
            Some(c) => tx(c),
            None => if was_cap { t!("首都", "capital", "首都") } else { t!("都市", "city", "城市") },
        };
        let text = tf!("{} が {} の{}を占領した", "{} took {}'s {}", "{}攻占了{}的{}", tx(self.nations[an].name), tx(self.nations[old].name), name);
        let col = if an == p { [0.5, 1.0, 0.5] } else if old == p { [1.0, 0.35, 0.3] } else { [0.85, 0.85, 0.7] };
        if an == p || old == p || was_cap {
            self.push_msg(text, col);
        }
        if was_cap {
            // 遷都
            let mut newcap = None;
            for (i, b) in self.blds.iter().enumerate() {
                if b.alive && b.nation as usize == old && b.kind == BKind::Town && b.done() {
                    newcap = Some(i);
                    break;
                }
            }
            match newcap {
                Some(i) => {
                    self.blds[i].kind = BKind::Capital;
                    let h = self.bh(i);
                    self.nations[old].capital = Some(h);
                    self.nations[old].home = self.blds[i].center();
                }
                None => self.nations[old].capital = None,
            }
        }
        let has_city = self.blds.iter().any(|b| b.alive && b.nation as usize == old && b.kind.city() && b.done());
        if !has_city {
            self.eliminate(old, an);
        }
    }

    pub fn eliminate(&mut self, n: usize, by: usize) {
        if !self.nations[n].alive {
            return;
        }
        self.nations[n].alive = false;
        for i in 0..self.units.len() {
            if self.units[i].alive && self.units[i].nation as usize == n {
                self.kill_unit(i);
            }
        }
        for i in 0..self.blds.len() {
            if self.blds[i].alive && self.blds[i].nation as usize == n {
                self.remove_building(i);
            }
        }
        for o in 0..NATIONS.len() {
            self.war[n][o] = false;
            self.war[o][n] = false;
        }
        self.share_cap(n, by);
        if self.player == n || by == self.player {
            self.territory_dirty = true;
        }
        let by_name = if by < self.nations.len() { tx(self.nations[by].name) } else { "?" };
        let text = tf!("{} は {} に滅ぼされた", "{} was destroyed by {}", "{}被{}所灭", tx(self.nations[n].name), by_name);
        self.push_msg(text, [1.0, 0.8, 0.3]);
        if n == self.player {
            let t = if self.descent.is_some() {
                tf!("{} は天使の軍勢にすべての都市を奪われた……", "{} lost every city to the host of angels...", "{}的所有城市都被天使大军夺走了……", tx(self.nations[n].name))
            } else {
                tf!("{} は {} に滅ぼされた……", "{} was destroyed by {}...", "{}被{}所灭……", tx(self.nations[n].name), by_name)
            };
            self.game_over = Some((false, t));
        }
    }

    /// 滅んだ国の人口上限（基本の4と、それまでに分けられた分）を生き残った国に分ける。
    /// 等分し、余りは攻め滅ぼした国、次いで国力の高い国から順に1ずつ
    fn share_cap(&mut self, n: usize, by: usize) {
        let pts = BASE_CAP + self.nations[n].cap_bonus;
        self.nations[n].cap_bonus = 0;
        let mut order: Vec<usize> = (0..NATIONS.len()).filter(|&i| i != n && self.nations[i].alive).collect();
        if order.is_empty() {
            return;
        }
        order.sort_by(|&a, &b| self.nations[b].strength.partial_cmp(&self.nations[a].strength).unwrap());
        if let Some(k) = order.iter().position(|&i| i == by) {
            let c = order.remove(k);
            order.insert(0, c);
        }
        let (each, rest) = (pts / order.len() as u32, pts as usize % order.len());
        let mut mine = 0;
        for (k, &i) in order.iter().enumerate() {
            let add = each + if k < rest { 1 } else { 0 };
            self.nations[i].cap_bonus += add;
            if i == self.player {
                mine = add;
            }
        }
        if mine > 0 {
            let p = self.player;
            let cap = self.pop_cap(p);
            let t = tf!("{}の滅亡で、人口の上限が +{}（いま{}）", "With the fall of {}, our population limit rises by {} (now {})", "因{}灭亡，人口上限+{}（现为{}）", tx(self.nations[n].name), mine, cap);
            self.push_msg(t, [0.7, 1.0, 0.8]);
        }
    }

    // ------------------------------------------------------------ 更新
    pub fn step(&mut self) {
        let dt = DT;
        self.time += dt;
        let spy = self.sec_per_year();
        self.years += dt / spy;
        if self.game_over.is_some() {
            return;
        }
        if self.era == Era::Early && self.year_f() >= TOTAL_WAR_YEAR {
            self.begin_total_war();
        }
        // 史実の出来事はその日に起こす
        if self.hist_i < HISTORY.len() && HISTORY[self.hist_i].at <= self.year_f() {
            self.run_history();
        }
        self.stat_t -= dt;
        if self.stat_t <= 0.0 {
            self.stat_t = 0.25;
            self.update_stats();
        }
        self.economy(dt);
        self.update_buildings(dt);
        self.rebuild_city_list();
        self.rebuild_grid();
        for i in 0..self.units.len() {
            if self.units[i].alive {
                self.update_unit(i, dt);
            }
        }
        // 押し合いは1ステップおきに、2ステップ分の強さで（見た目は変わらず計算は半分）
        self.sep_tick = !self.sep_tick;
        if self.sep_tick {
            self.separate(dt * 2.0);
        }
        self.process_paths();
        for s in self.shots.iter_mut() {
            s.t -= dt;
        }
        self.shots.retain(|s| s.t > 0.0);
        if self.territory_dirty {
            self.recompute_territory();
        }
        if let Some(b) = &mut self.banner {
            b.t -= dt;
            if b.t <= 0.0 {
                self.banner = self.banner_queue.pop_front();
            }
        }
        self.month_tick += dt;
        if self.month_tick >= spy / 12.0 {
            self.month_tick -= spy / 12.0;
            self.monthly();
        }
        crate::ai::update_all(self, dt);
    }

    fn economy(&mut self, dt: f32) {
        let em = self.era_mult();
        // 都市からの税収（世論が落ちると、民主制の国ではストライキで減る）
        for b in self.blds.iter() {
            if b.alive && b.kind.city() && b.done() && (b.nation as usize) < NATIONS.len() {
                let n = &mut self.nations[b.nation as usize];
                let m = n.bias * n.eco_mul * n.tm.eco * em * opinion_eco(n.opinion) * if b.kind == BKind::Capital { 1.3 } else { 1.0 };
                let add = [0.5 * m * dt, 0.2 * m * dt, 0.2 * m * dt];
                for k in 0..3 {
                    n.res[k] += add[k];
                    n.earned[k] += add[k];
                }
            }
        }
        // 軍の維持費（存在するだけで糧を消費）
        let spy = self.sec_per_year();
        let mut starving = vec![false; self.nations.len()];
        for (i, n) in self.nations.iter_mut().enumerate() {
            if !n.alive {
                continue;
            }
            n.res[0] -= n.upkeep * dt;
            if n.res[0] < 0.0 {
                n.res[0] = 0.0;
                starving[i] = true;
            }
        }
        let _ = spy;
        for u in self.units.iter_mut() {
            if u.alive && u.hero != NONE {
                // 英雄は傷が癒える
                u.hp = (u.hp + u.max_hp() * 0.015 * dt).min(u.max_hp());
            } else if u.alive && u.kind.military() && !u.divine && starving[u.nation as usize] {
                u.hp -= u.max_hp() * 0.01 * dt;
            }
        }
        for i in 0..self.units.len() {
            if self.units[i].alive && self.units[i].hp <= 0.0 {
                self.kill_unit(i);
            }
        }
        self.income_t += dt;
        if self.income_t >= 5.0 {
            for n in self.nations.iter_mut() {
                for k in 0..3 {
                    let r = n.earned[k] / self.income_t;
                    n.income[k] = n.income[k] * 0.5 + r * 0.5;
                    n.earned[k] = 0.0;
                }
            }
            self.income_t = 0.0;
        }
    }

    fn update_buildings(&mut self, dt: f32) {
        for bi in 0..self.blds.len() {
            if !self.blds[bi].alive || !self.blds[bi].done() {
                continue;
            }
            // 訓練
            if let Some(&k) = self.blds[bi].queue.front() {
                self.blds[bi].queue_t += dt;
                if self.blds[bi].queue_t >= k.train_time() {
                    let h = self.bh(bi);
                    let sp = if k.naval() {
                        let b = &self.blds[bi];
                        self.dock_sea(b.tx, b.ty).map(|(x, y)| v2(x as f32 + 0.5, y as f32 + 0.5))
                    } else {
                        self.spawn_point(h)
                    };
                    if let Some(sp) = sp {
                        let b = &mut self.blds[bi];
                        b.queue.pop_front();
                        b.queue_t = 0.0;
                        let (ni, rally) = (b.nation as usize, b.rally);
                        let uh = self.spawn_unit(ni, k, sp, 1.0);
                        if let Some(r) = rally {
                            self.order_move(&[uh], r, true);
                        } else if k == UnitKind::Worker {
                            self.auto_gather(uh);
                        }
                        if k.military() {
                            self.nations[ni].soldiers += 1;
                            self.nations[ni].q_soldiers = self.nations[ni].q_soldiers.saturating_sub(1);
                        } else {
                            self.nations[ni].workers += 1;
                            self.nations[ni].q_workers = self.nations[ni].q_workers.saturating_sub(1);
                        }
                    }
                }
            }
            // 都市・塹壕陣地は少しずつ修復される
            if matches!(self.blds[bi].kind, BKind::Capital | BKind::Town | BKind::Fort) {
                let b = &mut self.blds[bi];
                b.hp = (b.hp + b.kind.max_hp() * 0.003 * dt).min(b.kind.max_hp());
            }
            // 都市・塹壕陣地の防衛射撃
            let fort = self.blds[bi].kind == BKind::Fort;
            if self.blds[bi].kind.city() || fort {
                let b = &mut self.blds[bi];
                b.cooldown -= dt;
                if b.cooldown <= 0.0 {
                    let (c, ni) = (b.center(), b.nation as usize);
                    let (range, cd, dmg) = match (fort, self.era == Era::Early) {
                        (true, true) => (FORT_RANGE, FORT_COOLDOWN, 10.0),
                        (true, false) => (FORT_RANGE, FORT_COOLDOWN, 14.0),
                        (false, true) => (6.5, 1.6, 10.0),
                        (false, false) => (6.5, 1.6, 14.0),
                    };
                    b.cooldown = cd;
                    if let Some(t) = self.nearest_enemy_unit(ni, c, range) {
                        let tp = self.units[t].pos;
                        self.shots.push(Shot { a: c, b: tp, t: 0.25 });
                        self.damage_unit(t, dmg, None, ni);
                    }
                }
            }
        }
    }

    /// 新しい民に仕事を割り当てる
    pub fn auto_gather(&mut self, h: Handle) {
        let Some(u) = self.unit(h) else { return };
        let ni = u.nation as usize;
        let p = u.pos;
        // 未完成で作業者のいない建物があれば最優先で建設に向かう
        if let Some(bh) = self.unfinished_bld(ni, p) {
            self.order_build(&[h], bh);
            return;
        }
        // 空いている田
        if let Some(f) = self.free_farm(ni, p) {
            self.order_gather(&[h], Src::Farm(f));
            return;
        }
        let res = self.nations[ni].res;
        if res[2] < res[1] * 0.6 {
            if let Some(m) = self.nearest_mine_for(Some(ni), p, 16.0) {
                self.order_gather(&[h], Src::Mine(m));
                return;
            }
        }
        if let Some((x, y)) = self.nearest_forest_for(Some(ni), p, 14) {
            self.order_gather(&[h], Src::Forest(x, y));
        }
    }

    pub fn bld_builders(&self, bh: Handle) -> usize {
        self.units.iter().filter(|u| u.alive && u.order == Order::Build(bh)).count()
    }

    pub fn unfinished_bld(&self, ni: usize, p: V2) -> Option<Handle> {
        let mut best: Option<(f32, Handle)> = None;
        for (i, b) in self.blds.iter().enumerate() {
            if b.alive && b.nation as usize == ni && !b.done() {
                let h = self.bh(i);
                if self.bld_builders(h) == 0 {
                    let d = b.center().dist(p);
                    if best.map_or(true, |x| d < x.0) {
                        best = Some((d, h));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    /// 建物へ接近する際の通行可能な目標地点を返す（2×2の通行不可タイル内部ではなく隣接する通行可能タイル上の点）
    pub fn bld_approach_point(&self, bh: Handle, from: V2) -> V2 {
        let Some(b) = self.bld(bh) else { return from };
        let c = b.center();
        if !b.kind.blocks() && self.passable_at(c) {
            return c;
        }
        let mut best: Option<(f32, V2)> = None;
        for dy in -1..=2 {
            for dx in -1..=2 {
                if dx >= 0 && dx <= 1 && dy >= 0 && dy <= 1 {
                    continue;
                }
                let (x, y) = (b.tx + dx, b.ty + dy);
                if !Map::inb(x, y) || !self.cost[Map::idx(x, y)].is_finite() {
                    continue;
                }
                let tc = v2(x as f32 + 0.5, y as f32 + 0.5);
                let bx = tc.x.clamp(b.tx as f32, b.tx as f32 + 2.0);
                let by = tc.y.clamp(b.ty as f32, b.ty as f32 + 2.0);
                let ap = tc + (v2(bx, by) - tc) * 0.35;
                let corner_pen = if (dx == -1 || dx == 2) && (dy == -1 || dy == 2) { 0.45 } else { 0.0 };
                let score = from.dist(ap) + corner_pen;
                if best.map_or(true, |bs| score < bs.0) {
                    best = Some((score, ap));
                }
            }
        }
        best.map(|bs| bs.1).unwrap_or(c)
    }

    pub fn farm_users(&self, f: Handle) -> usize {
        self.units.iter().filter(|u| u.alive && matches!(u.order, Order::Gather { src: Src::Farm(h), .. } if h == f)).count()
    }

    pub fn free_farm(&self, ni: usize, p: V2) -> Option<Handle> {
        // 田ごとに全ユニットを数えると田の数×ユニット数になるので、先に使われている田を集める
        let mut used = vec![false; self.blds.len()];
        for u in self.units.iter() {
            if let (true, Order::Gather { src: Src::Farm(h), .. }) = (u.alive, u.order) {
                if let Some(x) = used.get_mut(h.idx as usize) {
                    if self.blds[h.idx as usize].gen == h.gen {
                        *x = true;
                    }
                }
            }
        }
        let mut best: Option<(f32, Handle)> = None;
        for (i, b) in self.blds.iter().enumerate() {
            if b.alive && b.nation as usize == ni && b.kind == BKind::Farm && b.done() {
                let h = self.bh(i);
                if !used[i] {
                    let d = b.center().dist(p);
                    if best.map_or(true, |x| d < x.0) {
                        best = Some((d, h));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    // ---- 移動
    fn request_path(&mut self, i: usize, goal: (i32, i32)) {
        let u = &mut self.units[i];
        u.path_goal = goal;
        u.path_done = false;
        if !u.path_pending {
            u.path_pending = true;
            let h = Handle { idx: i as u32, gen: u.gen };
            self.path_queue.push_back(h);
        }
    }

    fn process_paths(&mut self) {
        // 1ステップで展開してよいマスの数。実時間で区切ると機械の速さで結果が変わるので、仕事量で区切る
        const BUDGET: usize = 30_000;
        let mut n = 0;
        let mut spent = 0;
        // 同じステップで、隣り合う地点から同じ目的地へ求めた経路は使い回す（部隊でまとめて動かすとき）
        let mut memo: Vec<((i32, i32), (i32, i32), bool, Vec<(i16, i16)>)> = Vec::new();
        while let Some(h) = self.path_queue.pop_front() {
            let i = h.idx as usize;
            if !(self.units[i].alive && self.units[i].gen == h.gen && self.units[i].path_pending) {
                continue;
            }
            let start = self.units[i].pos.tile();
            let goal = self.units[i].path_goal;
            let pos = self.units[i].pos;
            let naval = self.units[i].kind.naval();
            let reuse = memo.iter().find(|(s, g, nv, p)| {
                *nv == naval && *g == goal && !p.is_empty() && (s.0 - start.0).abs().max((s.1 - start.1).abs()) <= 1 && self.line_clear_for(pos, v2(s.0 as f32 + 0.5, s.1 as f32 + 0.5), naval)
            });
            let path = if let Some((_, _, _, p)) = reuse {
                p.clone()
            } else {
                let grid = if naval { &self.sea_cost } else { &self.cost };
                let r = if Map::inb(goal.0, goal.1) && grid[Map::idx(goal.0, goal.1)].is_finite() { 0 } else { 2 };
                let p = self.pf.find(grid, start, goal, r, 200000, crate::path::UNIT_WEIGHT);
                spent += self.pf.last_exp;
                memo.push((start, goal, naval, p.clone()));
                p
            };
            let u = &mut self.units[i];
            u.path = path;
            u.path_i = 0;
            u.path_pending = false;
            u.path_done = u.path.is_empty();
            n += 1;
            if n >= 4 && spent > BUDGET {
                break;
            }
        }
    }

    fn step_towards(&mut self, i: usize, to: V2, dt: f32) {
        let u = &self.units[i];
        let naval = u.kind.naval();
        let sp = u.kind.speed(self.era);
        let (tx, ty) = u.pos.tile();
        let mut tc = if Map::inb(tx, ty) && !naval { self.map.terrain[Map::idx(tx, ty)].move_cost() } else { 1.0 };
        // 山岳兵は丘・森を苦にしない
        if u.kind == UnitKind::Alpine && tc > 1.0 && tc < 2.0 {
            tc = 1.0;
        }
        let sp = sp / tc.clamp(1.0, 3.0).sqrt();
        let d = to - u.pos;
        let l = d.len();
        if l < 1e-4 {
            return;
        }
        let mv = (sp * dt).min(l);
        let np = u.pos + d * (mv / l);
        let dir = d * (1.0 / l);
        let ok = self.walkable(np, naval) || !self.walkable(u.pos, naval);
        let u = &mut self.units[i];
        u.dir = dir;
        if ok {
            u.pos = np;
        } else {
            let a = v2(np.x, u.pos.y);
            let b = v2(u.pos.x, np.y);
            if self.walkable(a, naval) {
                self.units[i].pos = a;
            } else if self.walkable(b, naval) {
                self.units[i].pos = b;
            }
        }
    }

    /// 目的地へ移動。到着（または到達不能）なら true
    fn nav(&mut self, i: usize, goal: V2, r: f32, dt: f32) -> bool {
        let pos = self.units[i].pos;
        let naval = self.units[i].kind.naval();
        let dist = pos.dist(goal);
        if dist <= r {
            self.units[i].path.clear();
            return true;
        }
        if dist < 4.0 && self.line_clear_for(pos, goal, naval) {
            self.units[i].path.clear();
            self.step_towards(i, goal, dt);
            return false;
        }
        let gt = goal.tile();
        let pg = self.units[i].path_goal;
        let same = if self.units[i].path_done {
            pg == gt
        } else {
            (pg.0 - gt.0).abs().max((pg.1 - gt.1).abs()) <= 1
        };
        if !same {
            self.units[i].path.clear();
            self.request_path(i, gt);
            return false;
        }
        let u = &self.units[i];
        if u.path_i < u.path.len() {
            let (wx, wy) = u.path[u.path_i];
            let wp = v2(wx as f32 + 0.5, wy as f32 + 0.5);
            if pos.dist(wp) < 0.45 {
                let u = &mut self.units[i];
                u.path_i += 1;
                if u.path_i >= u.path.len() {
                    u.path.clear();
                    u.path_i = 0;
                    u.path_done = true;
                }
            } else {
                // 先読み：次の点が直接見えるなら飛ばす
                let ni = u.path_i + 1;
                if ni < u.path.len() {
                    let (nx, ny) = u.path[ni];
                    let np = v2(nx as f32 + 0.5, ny as f32 + 0.5);
                    if pos.dist(wp) < 1.2 && self.line_clear_for(pos, np, naval) {
                        self.units[i].path_i = ni;
                    }
                }
                self.step_towards(i, wp, dt);
            }
            return false;
        }
        if u.path_pending {
            return false;
        }
        if u.path_done {
            // 経路の終端に着いた：直進できなければ到達不能
            if dist < 6.0 && self.line_clear_for(pos, goal, naval) {
                self.step_towards(i, goal, dt);
                return false;
            }
            return true;
        }
        self.request_path(i, gt);
        false
    }

    fn update_unit(&mut self, i: usize, dt: f32) {
        {
            let u = &mut self.units[i];
            u.cooldown -= dt;
            u.flash = (u.flash - dt).max(0.0);
            u.scan_t -= dt;
        }
        if self.units[i].ship.is_some() {
            self.update_ship(i, dt);
            return;
        }
        let order = self.units[i].order;
        let ni = self.units[i].nation as usize;
        let kind = self.units[i].kind;
        match order {
            Order::Idle => {
                if kind.military() && kind != UnitKind::Transport && self.units[i].scan_t <= 0.0 {
                    self.units[i].scan_t = 0.5;
                    if let Some(t) = self.nearest_enemy_unit_for(ni, Some(kind), self.units[i].pos, kind.sight()) {
                        let th = self.uh(t);
                        self.units[i].order = Order::Attack(Target::Unit(th));
                    }
                }
                // 自国領内で待機中は回復
                let (x, y) = self.units[i].pos.tile();
                if Map::inb(x, y) && self.owner[Map::idx(x, y)] as usize == ni {
                    let u = &mut self.units[i];
                    u.hp = (u.hp + u.max_hp() * 0.01 * dt).min(u.max_hp());
                }
            }
            Order::Move(g) => {
                let r = if self.units[i].ferry.is_some() { 1.2 } else { 0.25 };
                if self.nav(i, g, r, dt) {
                    self.units[i].order = Order::Idle;
                    if self.units[i].ferry.is_some() {
                        self.board_ship(i);
                    }
                }
            }
            Order::AttackMove(g) => {
                if self.units[i].scan_t <= 0.0 {
                    self.units[i].scan_t = 0.4;
                    let p = self.units[i].pos;
                    let hits_bld = kind.vs_building() > 0.0;
                    let found = self
                        .nearest_enemy_unit_for(ni, Some(kind), p, kind.sight())
                        .map(|t| Target::Unit(self.uh(t)))
                        .or_else(|| if hits_bld { self.nearest_enemy_bld(ni, p, kind.sight()).map(|b| Target::Bld(self.bh(b))) } else { None });
                    if let Some(t) = found {
                        self.units[i].order = Order::Attack(t);
                        self.units[i].resume = Some(g);
                        return;
                    }
                }
                if self.nav(i, g, 0.6, dt) {
                    self.units[i].order = Order::Idle;
                }
            }
            Order::Attack(t) => {
                let info = self.target_info(t);
                let valid = match (info, t) {
                    (Some(_), Target::Unit(h)) => self.unit(h).map_or(false, |u| Self::can_hit(Some(kind), u.kind) && self.hostile(ni, u)),
                    (Some((_, _, tn)), Target::Bld(_)) => self.at_war(ni, tn) && kind.vs_building() > 0.0,
                    (None, _) => false,
                };
                if !valid {
                    let r = self.units[i].resume.take();
                    self.units[i].order = match r {
                        Some(g) => Order::AttackMove(g),
                        None => Order::Idle,
                    };
                    self.units[i].scan_t = 0.0;
                    return;
                }
                let (tp, rad, _) = info.unwrap();
                let pos = self.units[i].pos;
                let range = kind.range() + rad;
                let d = pos.dist(tp);
                // 進軍中なら深追いしない
                if let (Some(_), Target::Unit(_)) = (self.units[i].resume, t) {
                    if d > kind.sight() * 1.8 {
                        let g = self.units[i].resume.take().unwrap();
                        self.units[i].order = Order::AttackMove(g);
                        return;
                    }
                }
                // 攻撃中も近くに敵兵がいれば建物より優先
                if let Target::Bld(_) = t {
                    if self.units[i].scan_t <= 0.0 {
                        self.units[i].scan_t = 0.6;
                        if let Some(e) = self.nearest_enemy_unit_for(ni, Some(kind), pos, kind.sight() * 0.8) {
                            if self.units[e].kind.military() {
                                if self.units[i].resume.is_none() {
                                    self.units[i].resume = Some(tp);
                                }
                                self.units[i].order = Order::Attack(Target::Unit(self.uh(e)));
                                return;
                            }
                        }
                    }
                }
                let in_range = match t {
                    Target::Bld(bh) => d <= range || self.bld(bh).map_or(false, |b| b.dist_to_footprint(pos) <= kind.range() + 0.35),
                    Target::Unit(_) => d <= range,
                };
                if in_range {
                    self.units[i].dir = (tp - pos).norm();
                    if self.units[i].cooldown <= 0.0 {
                        self.units[i].cooldown = kind.cooldown();
                        let atk = kind.base_atk() * self.units[i].power * self.nations[ni].mil_mul * self.nations[ni].tm.atk[kind as usize] * self.rng.range(0.85, 1.15);
                        let me = self.uh(i);
                        if kind.range() > 1.5 {
                            self.shots.push(Shot { a: pos, b: tp, t: if kind == UnitKind::Artillery { 0.45 } else { 0.25 } });
                        }
                        match t {
                            Target::Unit(h) => {
                                let ti = h.idx as usize;
                                let tk = self.units[ti].kind;
                                let (tx, ty) = self.units[ti].pos.tile();
                                let cover = Map::inb(tx, ty) && matches!(self.map.terrain[Map::idx(tx, ty)], Terrain::Hill | Terrain::Forest);
                                // 自国の領内で守る兵（塹壕に籠もる）と、塹壕陣地のそばの兵は損害が減る。
                                // 都市を失った国ほど守りは必死になる（総動員）
                                let tn = self.units[ti].nation;
                                let home_def = !tk.naval() && Map::inb(tx, ty) && self.owner[Map::idx(tx, ty)] == tn;
                                let dug_in = home_def && self.near_own_fort(tn as usize, self.units[ti].pos);
                                let home_k = if home_def { (0.65 - 0.3 * self.lost_frac(tn as usize)).max(0.38) * self.nations[tn as usize].tm.entrench } else { 1.0 };
                                // 敵地の奥では補給が細る
                                let (ax, ay) = pos.tile();
                                let ao = if Map::inb(ax, ay) { self.owner[Map::idx(ax, ay)] as usize } else { NONE as usize };
                                let supplied = kind.naval() || ao == ni || (ao < NATIONS.len() && self.allied(ni, ao));
                                let dmg = atk * kind.bonus(tk) * if cover { tk.hill_def() } else { 1.0 } * home_k * if dug_in { 0.7 } else { 1.0 } * if supplied { 1.0 } else { 0.85 };
                                self.damage_unit(ti, dmg, Some(me), ni);
                            }
                            Target::Bld(h) => {
                                let dmg = atk * kind.vs_building() * 0.8;
                                self.damage_bld(h.idx as usize, dmg, ni);
                            }
                        }
                    }
                } else {
                    let (nav_goal, nav_r) = match t {
                        Target::Bld(bh) if kind.range() < 1.5 => (self.bld_approach_point(bh, pos), 0.45),
                        _ => (tp, range * 0.9),
                    };
                    if self.nav(i, nav_goal, nav_r, dt) {
                        let unreachable = match t {
                            Target::Bld(bh) => d > range + 0.5 && self.bld(bh).map_or(true, |b| b.dist_to_footprint(self.units[i].pos) > kind.range() + 0.65),
                            Target::Unit(_) => d > range + 0.5,
                        };
                        if unreachable {
                            self.units[i].order = Order::Idle;
                        }
                    }
                }
            }
            Order::Gather { src, phase } => self.update_gather(i, src, phase, dt),
            Order::Build(bh) => {
                let Some(b) = self.bld(bh) else {
                    if !self.next_queued_build(i) {
                        self.units[i].order = Order::Idle;
                    }
                    return;
                };
                if b.done() || b.nation as usize != ni {
                    let (kind_b, own) = (b.kind, b.nation as usize == ni);
                    // 予約があれば次の建設へ
                    if self.next_queued_build(i) {
                        return;
                    }
                    if kind_b == BKind::Farm && own && self.farm_users(bh) == 0 {
                        self.units[i].order = Order::Gather { src: Src::Farm(bh), phase: 0 };
                    } else {
                        self.units[i].order = Order::Idle;
                    }
                    return;
                }
                let c = b.center();
                let pos = self.units[i].pos;
                let near = if b.kind.blocks() {
                    b.dist_to_footprint(pos) <= 1.05
                } else {
                    pos.dist(c) <= 1.15 || b.dist_to_footprint(pos) <= 0.45
                };
                if near {
                    self.units[i].path.clear();
                    let d = (c - pos).norm();
                    if d.len() > 0.0 {
                        self.units[i].dir = d;
                    }
                    let pw = self.units[i].power;
                    let b = self.bld_mut(bh).unwrap();
                    let inc = dt * pw / b.kind.build_time();
                    b.progress += inc;
                    b.hp = (b.hp + b.kind.max_hp() * 0.9 * inc).min(b.kind.max_hp());
                    if b.progress >= 1.0 {
                        b.progress = 1.0;
                        if b.kind.city() {
                            self.territory_dirty = true;
                            if self.nations[ni].capital.and_then(|h| self.bld(h)).is_none() {
                                let b = self.bld_mut(bh).unwrap();
                                b.kind = BKind::Capital;
                                self.nations[ni].capital = Some(bh);
                            }
                        }
                        if ni == self.player {
                            let nm = self.bld(bh).unwrap().kind.name();
                            self.push_msg(tf!("{}が完成した", "{} completed", "{}建成了", nm), [0.7, 0.9, 1.0]);
                        }
                    }
                } else {
                    let ap = self.bld_approach_point(bh, pos);
                    let nr = if b.kind.blocks() { 0.45 } else { 0.85 };
                    if self.nav(i, ap, nr, dt) {
                        let reached = self.bld(bh).map_or(false, |b| {
                            if b.kind.blocks() {
                                b.dist_to_footprint(self.units[i].pos) <= 1.25
                            } else {
                                self.units[i].pos.dist(b.center()) <= 1.5 || b.dist_to_footprint(self.units[i].pos) <= 0.75
                            }
                        });
                        if !reached {
                            // 辿り着けなければ次の予約へ
                            if !self.next_queued_build(i) {
                                self.units[i].order = Order::Idle;
                            }
                        }
                    }
                }
            }
        }
    }

    fn update_gather(&mut self, i: usize, src: Src, phase: u8, dt: f32) {
        let ni = self.units[i].nation as usize;
        let res = src.res();
        // 資源の位置と有効性
        let (sp, valid, sr) = match src {
            Src::Farm(h) => match self.bld(h) {
                Some(b) if b.nation as usize == ni && b.done() => (b.center(), true, 1.0),
                _ => (v2(0.0, 0.0), false, 0.0),
            },
            Src::Forest(x, y) => (v2(x as f32 + 0.5, y as f32 + 0.5), Map::inb(x, y) && self.map.terrain[Map::idx(x, y)] == Terrain::Forest, 1.2),
            Src::Mine(m) => {
                let mm = &self.map.mines[m];
                (v2(mm.x as f32 + 0.5, mm.y as f32 + 0.5), mm.amount > 0.0, 1.0)
            }
        };
        let cap = 10.0 * self.units[i].power;
        if phase != 2 && !valid {
            // 近くの同種資源へ
            let p = self.units[i].pos;
            let next = match src {
                Src::Forest(..) => self.nearest_forest(p, 8).map(|(x, y)| Src::Forest(x, y)),
                Src::Mine(_) => self.nearest_mine(p, 10.0).map(Src::Mine),
                Src::Farm(_) => self.free_farm(ni, p).map(Src::Farm),
            };
            match next {
                Some(s) => self.units[i].order = Order::Gather { src: s, phase: 0 },
                None => {
                    self.units[i].order = if self.units[i].carry > 0.0 { Order::Gather { src, phase: 2 } } else { Order::Idle };
                }
            }
            return;
        }
        match phase {
            0 => {
                if self.units[i].carry_kind != res {
                    self.units[i].carry = 0.0;
                    self.units[i].carry_kind = res;
                }
                if self.units[i].carry >= cap {
                    self.units[i].order = Order::Gather { src, phase: 2 };
                    return;
                }
                if self.nav(i, sp, sr, dt) {
                    if self.units[i].pos.dist(sp) <= sr + 0.6 {
                        self.units[i].order = Order::Gather { src, phase: 1 };
                    } else {
                        self.units[i].order = Order::Idle;
                    }
                }
            }
            1 => {
                let base = [0.9, 0.8, 0.5][res];
                let n = &self.nations[ni];
                let amt = base * self.units[i].power * n.bias * self.era_mult() * dt;
                let mut take = amt;
                match src {
                    Src::Forest(x, y) => {
                        let ti = Map::idx(x, y);
                        take = take.min(self.map.wood[ti]);
                        self.map.wood[ti] -= take;
                        if self.map.wood[ti] <= 0.0 {
                            self.map.terrain[ti] = Terrain::Plain;
                            if self.bmap[ti] == 0 {
                                self.cost[ti] = 1.0;
                            }
                            self.terrain_dirty.push((x, y));
                        }
                    }
                    Src::Mine(m) => {
                        take = take.min(self.map.mines[m].amount);
                        self.map.mines[m].amount -= take;
                    }
                    Src::Farm(_) => {}
                }
                let sp_dir = (sp - self.units[i].pos).norm();
                let u = &mut self.units[i];
                if sp_dir.len() > 0.0 {
                    u.dir = sp_dir;
                }
                u.carry += take;
                if u.carry >= cap {
                    u.carry = cap;
                    u.order = Order::Gather { src, phase: 2 };
                }
            }
            _ => {
                let p = self.units[i].pos;
                let Some(di) = self.nearest_dropsite(ni, p) else {
                    self.units[i].order = Order::Idle;
                    return;
                };
                let dh = self.bh(di);
                if self.blds[di].dist_to_footprint(p) <= 1.15 {
                    self.units[i].path.clear();
                    let carry = self.units[i].carry * self.nations[ni].eco_mul * self.nations[ni].tm.eco;
                    let k = self.units[i].carry_kind;
                    self.nations[ni].res[k] += carry;
                    self.nations[ni].earned[k] += carry;
                    self.units[i].carry = 0.0;
                    self.units[i].order = Order::Gather { src, phase: 0 };
                } else {
                    let ap = self.bld_approach_point(dh, p);
                    if self.nav(i, ap, 0.5, dt) && self.blds[di].dist_to_footprint(self.units[i].pos) > 1.35 {
                        self.units[i].order = Order::Idle;
                    }
                }
            }
        }
    }

    fn separate(&mut self, dt: f32) {
        // 押し合いは 0.55 以内のユニット同士だけ。格子（4マス）のうち、その範囲にかかる升（たいてい1つ、多くて4つ）だけを見る
        const R: f32 = 0.55;
        let mut pushes: Vec<(usize, V2)> = Vec::new();
        for a in 0..self.units.len() {
            let ua = &self.units[a];
            if !ua.alive {
                continue;
            }
            let p = ua.pos;
            let g0x = ((p.x - R) as i32 / GCELL).clamp(0, GW - 1);
            let g1x = ((p.x + R) as i32 / GCELL).clamp(0, GW - 1);
            let g0y = ((p.y - R) as i32 / GCELL).clamp(0, GH - 1);
            let g1y = ((p.y + R) as i32 / GCELL).clamp(0, GH - 1);
            let mut push = v2(0.0, 0.0);
            for gy in g0y..=g1y {
                for gx in g0x..=g1x {
                    for &b in self.grid[(gy * GW + gx) as usize].iter() {
                        let b = b as usize;
                        if a == b {
                            continue;
                        }
                        let ub = &self.units[b];
                        if !ub.alive {
                            continue;
                        }
                        let d = p - ub.pos;
                        let l2 = d.x * d.x + d.y * d.y;
                        if l2 < R * R {
                            let l = l2.sqrt();
                            let dir = if l > 1e-4 { d * (1.0 / l) } else { v2(((a * 7) % 5) as f32 - 2.0, ((a * 3) % 5) as f32 - 2.0).norm() };
                            push = push + dir * (R - l);
                        }
                    }
                }
            }
            if push.x != 0.0 || push.y != 0.0 {
                pushes.push((a, push));
            }
        }
        for (i, p) in pushes {
            let np = self.units[i].pos + p * (dt * 3.0).min(0.5);
            if self.walkable(np, self.units[i].kind.naval()) {
                self.units[i].pos = np;
            }
        }
    }

    // ------------------------------------------------------------ 月ごとの出来事
    fn monthly(&mut self) {
        self.run_history();
        self.monthly_sea();
        self.monthly_opinion();
        self.refresh_traits();
        // 終了判定
        if self.descent.is_none() {
            let pl = self.player;
            if self.nations[pl].alive && self.nations[pl].bloc != Bloc::Neutral && self.enemies_left(pl) == 0 && self.game_over.is_none() {
                let t = tf!("{} は敵国をすべて打ち破った！ 大戦はあなたの陣営の勝利に終わった。", "{} has defeated every enemy! The Great War ends in victory for your side.", "{}击败了所有敌国！大战以你方阵营的胜利告终。", tx(self.nations[self.player].name));
                self.game_over = Some((true, t));
            } else if self.year_f() >= END_YEAR {
                // 1919年6月までに勝者が現れなければ、天使が現れる
                self.begin_descent();
            }
        } else {
            self.monthly_descent();
        }
    }

    /// 対局の始まり（サラエボ事件を起こす）
    pub fn start_game(&mut self) {
        self.run_history();
    }

    /// まだ倒していない敵国の数（陣営の違う交戦国。参戦していない中立国は数えない）
    pub fn enemies_left(&self, n: usize) -> usize {
        (0..NATIONS.len()).filter(|&o| o != n && self.nations[o].alive && !self.allied(n, o) && (self.at_war(n, o) || (self.nations[o].bloc != Bloc::Neutral && self.nations[n].bloc != Bloc::Neutral))).count()
    }

    /// 総力戦の時代（1916年7月、ソンムの戦い）
    fn begin_total_war(&mut self) {
        self.era = Era::Total;
        let text = t!(
            "ヴェルダンとソンムで、一日に数万の兵が倒れる消耗戦が始まった。兵の能力の上限が 6 に上がり、英仏は戦車を、ドイツは突撃兵を投入できるようになる。",
            "At Verdun and the Somme a war of attrition begins, with tens of thousands falling in a single day. The power limit rises to 6; Britain and France can field tanks, and Germany stormtroopers.",
            "凡尔登与索姆河，一日便有数万士兵倒下的消耗战开始了。士兵能力上限提升至6，英法可投入坦克，德国可投入突击兵。"
        );
        self.push_msg(tf!("【総力戦】{}", "[Total war] {}", "【总体战】{}", text), [1.0, 0.85, 0.3]);
        self.queue_banner(Banner { title: t!("総力戦の時代", "The Age of Total War", "总体战时代").into(), text: text.into(), t: 8.0 });
        self.update_stats();
    }

    /// 地点の周りの陸の兵の力：(敵, 自国)
    pub fn power_near(&self, n: usize, p: V2, r: f32) -> (f32, f32) {
        let (mut enemy, mut mine) = (0.0, 0.0);
        let g0x = ((p.x - r) as i32 / GCELL).clamp(0, GW - 1);
        let g1x = ((p.x + r) as i32 / GCELL).clamp(0, GW - 1);
        let g0y = ((p.y - r) as i32 / GCELL).clamp(0, GH - 1);
        let g1y = ((p.y + r) as i32 / GCELL).clamp(0, GH - 1);
        for gy in g0y..=g1y {
            for gx in g0x..=g1x {
                for &i in self.grid[(gy * GW + gx) as usize].iter() {
                    let u = &self.units[i as usize];
                    if !u.alive || u.kind.naval() || !u.kind.military() || u.pos.dist(p) > r {
                        continue;
                    }
                    let un = u.nation as usize;
                    if un == n {
                        mine += u.strength();
                    } else if self.at_war(n, un) {
                        enemy += u.strength();
                    }
                }
            }
        }
        (enemy, mine)
    }

    /// 開戦時から失った都市の割合（0〜1）
    pub fn lost_frac(&self, n: usize) -> f32 {
        if n >= NATIONS.len() {
            return 0.0;
        }
        let c0 = NATIONS[n].cities.len() as f32;
        (1.0 - self.nations[n].cities as f32 / c0).clamp(0.0, 1.0)
    }

    /// 自国の塹壕陣地のそばか
    pub fn near_own_fort(&self, n: usize, p: V2) -> bool {
        let r = 4.0;
        self.blds.iter().any(|b| b.alive && b.kind == BKind::Fort && b.done() && b.nation as usize == n && b.center().dist(p) <= r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::geo::*;

    /// 年表は日付の順で、国の番号と君主家の表が国家データと揃っている
    #[test]
    fn history_table() {
        assert_eq!(INITIAL_HOUSE.len(), NATIONS.len());
        for w in HISTORY.windows(2) {
            assert!(w[0].at <= w[1].at, "{} の後に {}", w[0].title, w[1].title);
        }
        for e in HISTORY {
            assert!(e.nation < NATIONS.len());
            assert!(e.at >= START_YEAR - 0.01 && e.at < END_YEAR, "{}", e.title);
        }
        // 英雄の素材は英雄の数だけある
        assert_eq!(HEROES.len(), 46);
        for n in 0..NATIONS.len() {
            if NATIONS[n].playable {
                assert!(heroes_of(n).count() >= 1, "{} に英雄がいない", NATIONS[n].name);
            }
        }
    }

    /// 史実の都市はすべて陸に置かれ、最初から建っている
    #[test]
    fn cities_founded() {
        let w = World::new(12345);
        let total: usize = NATIONS.iter().map(|d| d.cities.len()).sum();
        let built = w.blds.iter().filter(|b| b.alive && b.kind.city()).count();
        assert_eq!(built, total, "置けなかった都市がある");
        for (i, d) in NATIONS.iter().enumerate() {
            assert!(w.nations[i].capital.is_some(), "{} に首都が無い", d.name);
            assert!(w.nations[i].workers > 0 && w.nations[i].soldiers > 0, "{} に民・兵がいない", d.name);
        }
        // 都市の名前が引ける
        let named = w.blds.iter().filter(|b| b.alive && b.kind.city() && b.city_name().is_some()).count();
        assert_eq!(named, total);
        // 新しい都市は築けない
        let home = w.nations[GER].home;
        assert!(w.can_place(GER, BKind::Town, home.x as i32 + 5, home.y as i32 + 5).is_err());
    }

    /// サラエボ事件から宣戦布告までは自動で起こる。史実の同盟関係が最初からある
    #[test]
    fn july_crisis() {
        let mut w = World::new(12345);
        w.player = BEL;
        assert!(w.allied(GER, AUH) && w.allied(GBR, FRA) && w.allied(FRA, RUS));
        assert!(!w.at_war(GER, FRA));
        w.start_game();
        while w.year_f() < ymd(1914, 9, 1) {
            w.step();
        }
        for (a, b) in [(AUH, SRB), (GER, RUS), (GER, FRA), (GER, BEL), (GBR, GER), (AUH, RUS), (GBR, AUH)] {
            assert!(w.at_war(a, b), "{} と {} が交戦していない", NATIONS[a].name, NATIONS[b].name);
        }
        assert!(!w.at_war(ITA, AUH) && w.nations[ITA].bloc == Bloc::Neutral);
        assert!(!w.at_war(USA, GER));
        // 同盟国どうしは宣戦できない
        assert!(!w.declare_war(GER, AUH, false));
        // 中立国を攻めると、その国は敵の陣営に入る
        assert!(w.declare_war(GER, NED, false));
        assert_eq!(w.nations[NED].bloc, Bloc::Entente);
    }

    /// 兵は民から徴兵する：兵営で兵を訓練すると民が減る
    #[test]
    fn soldiers_come_from_workers() {
        let mut w = World::new(12345);
        let n = FRA;
        w.nations[n].res = [99999.0; 3];
        let home = w.nations[n].home;
        let mut b = None;
        'outer: for r in 3..12 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if let Ok(h) = w.place_building(n, BKind::Barracks, home.x as i32 + dx, home.y as i32 + dy) {
                        b = Some(h);
                        break 'outer;
                    }
                }
            }
        }
        let bh = b.expect("兵営を置けない");
        w.blds[bh.idx as usize].progress = 1.0;
        w.update_stats();
        let (wk0, pop0) = (w.nations[n].workers, w.nations[n].pop());
        w.enqueue(bh, UnitKind::Infantry).unwrap();
        assert_eq!(w.nations[n].workers, wk0 - 1);
        assert_eq!(w.nations[n].pop(), pop0, "人口は変わらない（民が兵になっただけ）");
        // 国ごとに造れる兵種が違う
        assert!(w.enqueue(bh, UnitKind::Storm).is_err(), "フランスが突撃兵を造れた");
        assert!(w.kind_available(GER, UnitKind::Submarine) && !w.kind_available(FRA, UnitKind::Submarine));
        assert!(!w.kind_available(GBR, UnitKind::Tank), "1914年に戦車がある");
        // 民がいなければ兵は訓練できない
        for i in 0..w.units.len() {
            if w.units[i].alive && w.units[i].nation as usize == n && w.units[i].kind == UnitKind::Worker {
                w.kill_unit(i);
            }
        }
        w.update_stats();
        assert!(w.enqueue(bh, UnitKind::Infantry).is_err());
    }

    /// 英雄は自国出身の者だけ呼べる。一国一人。天使を傷つけられるのは英雄だけ
    #[test]
    fn hero_summon_rules() {
        let mut w = World::new(12345);
        let n = FRA;
        let workers: Vec<Handle> = (0..w.units.len()).filter(|&i| w.units[i].alive && w.units[i].nation as usize == n && w.units[i].kind == UnitKind::Worker).map(|i| w.uh(i)).take(4).collect();
        let foch = HEROES.iter().position(|h| h.name == "フォッシュ").unwrap();
        let hind = HEROES.iter().position(|h| h.name == "ヒンデンブルク").unwrap();
        assert!(w.can_summon(n, hind, &workers).is_err(), "他国の英雄を呼べた");
        let (v, p) = w.sacrifice_power(n, &workers);
        let h = w.summon_hero(n, foch, &workers).expect("召喚できない");
        let hero = w.unit(h).unwrap();
        assert_eq!(hero.hero as usize, foch);
        assert!((hero.power - p * HEROES[foch].mult).abs() < 1e-3);
        assert!(v.iter().all(|&x| w.unit(x).is_none()), "捧げた民が残っている");
        let joffre = HEROES.iter().position(|h| h.name == "ジョッフル").unwrap();
        let more: Vec<Handle> = (0..w.units.len()).filter(|&i| w.units[i].alive && w.units[i].nation as usize == n && w.units[i].kind == UnitKind::Worker).map(|i| w.uh(i)).take(3).collect();
        assert!(w.can_summon(n, joffre, &more).is_err(), "二人目を呼べた");
        // 天使：普通の兵では傷つかず、英雄なら傷つく
        let pos = w.unit(h).unwrap().pos;
        let g = w.spawn_unit(ANGEL, UnitKind::Infantry, pos, 12.0);
        w.unit_mut(g).unwrap().divine = true;
        let gi = g.idx as usize;
        let hp0 = w.units[gi].hp;
        let soldier = (0..w.units.len()).find(|&i| w.units[i].alive && w.units[i].nation as usize == n && w.units[i].kind.military() && w.units[i].hero == NONE).unwrap();
        let sh = w.uh(soldier);
        w.damage_unit(gi, 100.0, Some(sh), n);
        assert_eq!(w.units[gi].hp, hp0);
        w.damage_unit(gi, 100.0, Some(h), n);
        assert!(w.units[gi].hp < hp0);
    }

    /// 海の向こうへ動かした兵は、港で船に乗って渡り、上陸する
    #[test]
    fn troops_cross_the_channel() {
        let mut w = World::new(12345);
        // 英国はプレイヤー（AI に兵を動かされない）
        w.player = GBR;
        let n = GBR;
        let london = w.nations[n].home;
        let paris = w.nations[FRA].home;
        assert!(w.needs_ferry(london, paris));
        let hs: Vec<Handle> = (0..w.units.len())
            .filter(|&i| w.units[i].alive && w.units[i].nation as usize == n && w.units[i].kind.land_mil() && w.units[i].pos.dist(london) < 30.0)
            .map(|i| w.uh(i))
            .take(4)
            .collect();
        assert!(!hs.is_empty());
        let goal = paris + v2(3.0, 3.0);
        w.order_move(&hs, goal, false);
        assert!(w.unit(hs[0]).unwrap().ferry.is_some(), "渡海の計画が立たない");
        let mut landed = false;
        let mut sailed = false;
        for _ in 0..(400.0 / DT) as usize {
            w.step();
            sailed |= w.units.iter().any(|u| u.alive && u.nation as usize == n && u.ship.as_ref().map_or(false, |s| !s.cargo.is_empty() && s.phase == 1));
            let near = w.units.iter().filter(|u| u.alive && u.nation as usize == n && u.kind.land_mil() && u.pos.dist(paris) < 20.0).count();
            // 混み合った港で遅れる兵もいるので、半分が渡れば良しとする
            if near * 2 >= hs.len() + 1 {
                landed = true;
                break;
            }
        }
        assert!(sailed, "船が出なかった");
        assert!(landed, "大陸に上陸しなかった");
    }

    /// 帝政の国は世論が尽きると革命が起きて降伏し、都市は敵に明け渡される
    #[test]
    fn revolution_surrender() {
        let mut w = World::new(12345);
        w.player = BEL;
        w.start_game();
        while w.year_f() < ymd(1914, 8, 20) {
            w.step();
        }
        let c0 = w.nations[GER].cities + w.nations[AUH].cities;
        w.nations[RUS].opinion = 0.0;
        w.nations[RUS].lost = 200.0;
        w.monthly_opinion();
        assert!(!w.nations[RUS].alive && w.nations[RUS].surrendered, "ロシアで革命が起きない");
        w.update_stats();
        assert!(w.nations[GER].cities + w.nations[AUH].cities > c0 + 18, "都市が明け渡されていない");
        // 民主制の国では革命は起きない
        w.nations[FRA].opinion = 0.0;
        w.monthly_opinion();
        assert!(w.nations[FRA].alive);
    }

    /// 米国は妨害を受けて参戦気運が高まらない限り参戦しない
    #[test]
    fn us_needs_provocation() {
        let mut w = World::new(12345);
        w.player = FRA;
        w.start_game();
        w.sea.unrestricted = false;
        for _ in 0..30 {
            w.monthly_sea();
        }
        assert_eq!(w.nations[USA].bloc, Bloc::Neutral);
        // 中立の米国の船を沈めると気運が上がる
        w.sea.unrestricted = true;
        let ship = w.spawn_unit(USA, UnitKind::Transport, w.nations[GBR].home, 1.0);
        let sub = w.spawn_unit(GER, UnitKind::Submarine, w.nations[GBR].home, 1.0);
        let si = ship.idx as usize;
        assert!(w.hostile(GER, &w.units[si]));
        w.damage_unit(si, 9999.0, Some(sub), GER);
        assert!(w.sea.us_anger >= 15.0);
        w.sea.us_anger = US_ENTRY;
        w.monthly_sea();
        assert_eq!(w.nations[USA].bloc, Bloc::Entente);
        assert!(w.at_war(USA, GER));
        // 米国は選べない
        assert!(!NATIONS[USA].playable);
    }

    /// 1919年6月までに勝者が現れなければ天使が降り、半年ですべての都市が占領されて終わる
    #[test]
    fn angels_end_the_war() {
        let mut w = World::new(12345);
        w.player = FRA;
        w.years = END_YEAR - START_YEAR - 0.01;
        let mut steps = 0;
        while steps < 500_000 {
            // 自国が滅んでも、天使がすべての都市を占領するまで進める
            if let Some((_, _)) = &w.game_over {
                if w.descent.as_ref().map_or(false, |d| w.years - d.start >= DESCENT_YEARS) {
                    break;
                }
                w.game_over = None;
            }
            w.step();
            steps += 1;
        }
        let d = w.descent.as_ref().expect("天使が降りていない");
        assert!(w.years - d.start <= DESCENT_YEARS + 0.1);
        let human = w.blds.iter().filter(|b| b.alive && b.kind.city() && b.nation as usize != ANGEL).count();
        assert_eq!(human, 0, "天使のものでない都市が残っている");
        assert!(matches!(&w.game_over, Some((false, _))));
    }

    /// 塹壕陣地：数に上限があり、近づく敵兵を撃つ
    #[test]
    fn fort_cap_and_shoots() {
        let mut w = World::new(12345);
        let n = GER;
        w.nations[n].res = [99999.0; 3];
        let cap = w.fort_cap(n);
        let home = w.nations[n].home;
        let mut spots = Vec::new();
        for r in 3..16 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (tx, ty) = (home.x as i32 + dx, home.y as i32 + dy);
                    if w.can_place(n, BKind::Fort, tx, ty).is_ok() && !spots.iter().any(|&(x, y): &(i32, i32)| (x - tx).abs() < 3 && (y - ty).abs() < 3) {
                        spots.push((tx, ty));
                    }
                }
            }
        }
        assert!(spots.len() > cap as usize);
        let mut hs = Vec::new();
        for &(tx, ty) in spots.iter().take(cap as usize) {
            hs.push(w.place_building(n, BKind::Fort, tx, ty).unwrap());
        }
        let (tx, ty) = spots[cap as usize];
        assert!(w.place_building(n, BKind::Fort, tx, ty).is_err(), "上限を超えて築けてしまう");
        let b = hs[0].idx as usize;
        w.blds[b].progress = 1.0;
        w.blds[b].hp = BKind::Fort.max_hp();
        let c = w.blds[b].center();
        let e = FRA;
        w.declare_war(n, e, true);
        let uh = w.spawn_unit(e, UnitKind::Infantry, c + v2(FORT_RANGE - 1.5, 0.0), 1.0);
        let hp0 = w.unit(uh).unwrap().hp;
        w.rebuild_grid();
        for _ in 0..30 {
            w.update_buildings(DT);
        }
        assert!(w.unit(uh).map_or(true, |u| u.hp < hp0), "塹壕が撃たない");
    }

    /// 同盟国以外（参戦した敵国）をすべて打ち破れば勝利。中立のままの国は数えない
    #[test]
    fn victory_condition() {
        let mut w = World::new(12345);
        w.player = FRA;
        w.start_game();
        while w.year_f() < ymd(1914, 8, 15) {
            w.step();
        }
        assert!(w.game_over.is_none());
        for n in [GER, AUH] {
            w.eliminate(n, FRA);
        }
        // オスマン帝国・ブルガリアはまだ中立なので数えない
        assert_eq!(w.enemies_left(FRA), 0);
        w.monthly();
        assert!(matches!(&w.game_over, Some((true, _))), "{:?}", w.game_over);
        // 中立国のままのプレイヤーは勝利しない
        let mut w = World::new(12345);
        w.player = SWE;
        w.monthly();
        assert!(w.game_over.is_none());
    }

    /// 生産は開戦時がもっとも高い
    #[test]
    fn production_declines() {
        let mut w = World::new(12345);
        let a = w.era_mult();
        w.years = 2.0;
        let b = w.era_mult();
        w.years = END_YEAR - START_YEAR;
        let c = w.era_mult();
        assert!(a > b && b > c && (a - 1.0).abs() < 1e-6);
    }

    /// 全 AI で長期間シミュレーションし、国家の推移とイベントを確認する
    #[test]
    fn long_sim() {
        let seed = std::env::var("SIM_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(12345u32);
        let mut w = World::new(seed);
        // 観戦用：米国（AI が動かす）をプレイヤー扱いにする
        w.player = USA;
        w.ai_player = true;
        w.start_game();
        let t0 = std::time::Instant::now();
        let years = std::env::var("SIM_YEARS").ok().and_then(|s| s.parse().ok()).unwrap_or(5.2f32);
        let mut next_report = 0.0;
        let mut seen_t = -1.0f32;
        let mut steps = 0usize;
        while w.years < years {
            steps += 1;
            if let Some((win, t)) = &w.game_over {
                if w.descent.is_none() {
                    println!("  [game over] {} {}", win, t);
                    w.game_over = None;
                } else {
                    break;
                }
            }
            w.step();
            if w.years >= next_report {
                next_report += 1.0 / 6.0;
                let alive: Vec<String> = w
                    .nations
                    .iter()
                    .take(NATIONS.len())
                    .filter(|n| n.alive && n.bloc != Bloc::Neutral)
                    .map(|n| format!("{}(c{} w{} s{} op{:.0} 食{:.0}木{:.0}鉄{:.0} L{:.0}/I{:.0})", n.name, n.cities, n.workers, n.soldiers, n.opinion, n.res[0], n.res[1], n.res[2], n.lost_total, n.inflicted_total))
                    .collect();
                let units = w.units.iter().filter(|u| u.alive).count();
                let ships = w.units.iter().filter(|u| u.alive && u.ship.is_some()).count();
                let subs = w.units.iter().filter(|u| u.alive && u.kind == UnitKind::Submarine).count();
                println!("{} t={:.1}s units={} ships={} subs={} sunk={} delivered={} anger={:.0} :: {}", w.date_str(), t0.elapsed().as_secs_f32(), units, ships, subs, w.sea.sunk, w.sea.delivered, w.sea.us_anger, alive.join(" "));
            }
            for m in w.msgs.iter().filter(|m| m.time > seen_t) {
                println!("  [msg] {}", m.text);
            }
            seen_t = w.time;
        }
        println!("elapsed {:.1}s for {} steps; game_over {:?}", t0.elapsed().as_secs_f32(), steps, w.game_over);
    }

    #[test]
    fn build_queue_in_order() {
        let mut w = World::new(12345);
        let n = FRA;
        w.player = n;
        w.nations[n].res = [99999.0; 3];
        let home = w.nations[n].home;
        let mut spots: Vec<(i32, i32)> = Vec::new();
        for r in 3..12 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (tx, ty) = (home.x as i32 + dx, home.y as i32 + dy);
                    if spots.len() < 3 && w.can_place(n, BKind::Farm, tx, ty).is_ok() && !spots.iter().any(|&(x, y)| (x - tx).abs() < 3 && (y - ty).abs() < 3) {
                        spots.push((tx, ty));
                    }
                }
            }
        }
        assert_eq!(spots.len(), 3);
        let wk = (0..w.units.len()).find(|&i| w.units[i].alive && w.units[i].nation as usize == n && w.units[i].kind == UnitKind::Worker && w.units[i].pos.dist(home) < 8.0).unwrap();
        let h = w.uh(wk);
        let bs: Vec<Handle> = spots.iter().map(|&(x, y)| w.place_building(n, BKind::Farm, x, y).unwrap()).collect();
        for &b in bs.iter() {
            w.queue_build(&[h], b);
        }
        assert_eq!(w.unit(h).unwrap().order, Order::Build(bs[0]));
        assert_eq!(w.unit(h).unwrap().build_queue.len(), 2);
        let mut done_order = Vec::new();
        for _ in 0..(160.0 / DT) as usize {
            w.step();
            for (k, &b) in bs.iter().enumerate() {
                if w.bld(b).map_or(false, |x| x.done()) && !done_order.contains(&k) {
                    done_order.push(k);
                }
            }
            if done_order.len() == 3 {
                break;
            }
        }
        assert_eq!(&done_order[..1], &[0]);
    }
}
