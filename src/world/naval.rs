//! 海上：兵の渡海（港で輸送船に乗り、海を渡って上陸する）、植民地からの船団、
//! 米国の物資船、Uボートによる通商破壊と米国の参戦気運

use super::*;

/// 1隻に乗れる兵の数
pub const SHIP_CAP: usize = 8;
/// 米国が参戦する気運
pub const US_ENTRY: f32 = 100.0;

/// 船に乗っている兵
#[derive(Clone, Copy, Debug)]
pub struct Cargo {
    pub kind: UnitKind,
    pub power: f32,
    /// 体力の割合
    pub hp: f32,
    pub hero: u8,
}

/// 渡海の予定
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ferry {
    /// 乗船する岸（陸）と、船の待つ海
    pub embark: V2,
    pub sea: V2,
    /// 上陸する岸（陸）と、その沖の海
    pub land: V2,
    pub land_sea: V2,
    /// 上陸してから向かう先
    pub goal: V2,
    pub attack: bool,
}

/// 船の状態
#[derive(Clone, Debug)]
pub struct Ship {
    pub cargo: Vec<Cargo>,
    /// 船団の積み荷（食糧・木材・鉄）と民
    pub res: [f32; 3],
    pub workers: u8,
    /// 積み荷を届ける国（NONE=自国）
    pub deliver_to: u8,
    /// 0:乗船待ち 1:航海中
    pub phase: u8,
    pub t: f32,
    /// 行き先の海と、上陸・荷揚げする岸
    pub dest: V2,
    pub land: V2,
    pub goal: V2,
    pub attack: bool,
    /// 植民地からの船団の出どころ（CONVOYS の番号。兵を運ぶ船は NONE）
    pub route: u8,
}

/// 植民地・米国からの船団の航路
pub struct ConvoyRoute {
    pub name: &'static str,
    /// 船団を出す国（米国の物資船は米国の船）
    pub owner: usize,
    /// 積み荷を受け取る国
    pub to: usize,
    pub lon: f32,
    pub lat: f32,
    /// 積み荷（食糧・木材・鉄）と民
    pub res: [f32; 3],
    pub workers: u8,
}

pub const CONVOYS: &[ConvoyRoute] = &[
    ConvoyRoute { name: "カナダ", owner: GBR, to: GBR, lon: -61.0, lat: 43.6, res: [160.0, 140.0, 60.0], workers: 1 },
    ConvoyRoute { name: "インド・オーストラリア", owner: GBR, to: GBR, lon: 32.6, lat: 31.7, res: [180.0, 60.0, 90.0], workers: 2 },
    ConvoyRoute { name: "南アフリカ・南米", owner: GBR, to: GBR, lon: -14.5, lat: 30.2, res: [200.0, 40.0, 80.0], workers: 1 },
    ConvoyRoute { name: "アルジェリア", owner: FRA, to: FRA, lon: 3.0, lat: 37.4, res: [160.0, 40.0, 50.0], workers: 2 },
    ConvoyRoute { name: "西アフリカ", owner: FRA, to: FRA, lon: -14.8, lat: 31.0, res: [140.0, 70.0, 40.0], workers: 1 },
    ConvoyRoute { name: "インドシナ", owner: FRA, to: FRA, lon: 32.7, lat: 31.9, res: [150.0, 50.0, 60.0], workers: 1 },
    ConvoyRoute { name: "アメリカの物資（英国へ）", owner: USA, to: GBR, lon: -72.5, lat: 40.2, res: [160.0, 80.0, 140.0], workers: 0 },
    ConvoyRoute { name: "アメリカの物資（フランスへ）", owner: USA, to: FRA, lon: -72.0, lat: 39.8, res: [150.0, 80.0, 130.0], workers: 0 },
];

/// 海上の情勢
pub struct SeaState {
    /// 無制限潜水艦作戦（同盟国の Uボートが中立の米国の船も沈める）
    pub unrestricted: bool,
    /// 米国の参戦気運（100 で参戦）
    pub us_anger: f32,
    /// 同盟国が沈めた船・届いた船団の数
    pub sunk: u32,
    pub delivered: u32,
    /// 最近の船団のようす
    pub last: String,
}

impl SeaState {
    pub fn new() -> SeaState {
        SeaState { unrestricted: false, us_anger: 0.0, sunk: 0, delivered: 0, last: String::new() }
    }
}

/// 海の連結成分（地中海・黒海・カスピ海などを区別する）
pub fn sea_components(terrain: &[Terrain]) -> Vec<u16> {
    let n = MAP_W * MAP_H;
    let mut comp = vec![u16::MAX; n];
    let mut id = 0u16;
    for s in 0..n {
        if comp[s] != u16::MAX || !terrain[s].navigable() {
            continue;
        }
        let mut stack = vec![s];
        comp[s] = id;
        while let Some(c) = stack.pop() {
            let (x, y) = ((c % MAP_W) as i32, (c / MAP_W) as i32);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if Map::inb(nx, ny) {
                    let ni = Map::idx(nx, ny);
                    if comp[ni] == u16::MAX && terrain[ni].navigable() {
                        comp[ni] = id;
                        stack.push(ni);
                    }
                }
            }
        }
        id = id.wrapping_add(1);
    }
    comp
}

/// 海に面した陸のマスを間引いて集める（上陸・乗船の候補）
pub fn coast_points(terrain: &[Terrain]) -> Vec<((i32, i32), (i32, i32))> {
    let mut v = Vec::new();
    for y in 1..MAP_H as i32 - 1 {
        for x in 1..MAP_W as i32 - 1 {
            if (x + y * 3) % 3 != 0 {
                continue;
            }
            let t = terrain[Map::idx(x, y)];
            if !t.passable() {
                continue;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (sx, sy) = (x + dx, y + dy);
                if terrain[Map::idx(sx, sy)].navigable() {
                    v.push(((x, y), (sx, sy)));
                    break;
                }
            }
        }
    }
    v
}

fn tc(t: (i32, i32)) -> V2 {
    v2(t.0 as f32 + 0.5, t.1 as f32 + 0.5)
}

impl World {
    fn comp_at(&self, p: V2) -> u16 {
        let (x, y) = p.tile();
        if !Map::inb(x, y) {
            return u16::MAX;
        }
        let c = self.land_comp[Map::idx(x, y)];
        if c != u16::MAX {
            return c;
        }
        // 建物の上や山の際などでは、近くの陸の成分を使う
        for r in 1..4i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (xx, yy) = (x + dx, y + dy);
                    if Map::inb(xx, yy) {
                        let c = self.land_comp[Map::idx(xx, yy)];
                        if c != u16::MAX {
                            return c;
                        }
                    }
                }
            }
        }
        u16::MAX
    }

    fn sea_comp_at(&self, p: V2) -> u16 {
        let (x, y) = p.tile();
        if Map::inb(x, y) {
            self.sea_comp[Map::idx(x, y)]
        } else {
            u16::MAX
        }
    }

    /// 陸の部隊が from から to へ行くのに海を渡る必要があるか
    pub fn needs_ferry(&self, from: V2, to: V2) -> bool {
        let (a, b) = (self.comp_at(from), self.comp_at(to));
        a != u16::MAX && b != u16::MAX && a != b
    }

    /// 都市の港（都市のそばの海に面した陸と、その海）
    pub fn port_of(&self, bi: usize) -> Option<(V2, V2)> {
        let b = &self.blds[bi];
        let mut best: Option<(i32, (i32, i32), (i32, i32))> = None;
        for dy in -4..=5i32 {
            for dx in -4..=5i32 {
                let (x, y) = (b.tx + dx, b.ty + dy);
                if !Map::inb(x, y) || !self.map.terrain[Map::idx(x, y)].passable() || self.bmap[Map::idx(x, y)] != 0 {
                    continue;
                }
                for (ex, ey) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (sx, sy) = (x + ex, y + ey);
                    if Map::inb(sx, sy) && self.map.terrain[Map::idx(sx, sy)].navigable() {
                        let d = (dx - 1).abs() + (dy - 1).abs();
                        if best.map_or(true, |bb| d < bb.0) {
                            best = Some((d, (x, y), (sx, sy)));
                        }
                    }
                }
            }
        }
        best.map(|(_, l, s)| (tc(l), tc(s)))
    }

    /// 渡海の計画：出発地のある陸から、目的地のある陸へ。味方の港を優先し、無ければ海岸に乗り降りする
    pub fn plan_ferry(&self, hs: &[Handle], goal: V2, attack: bool) -> Option<Ferry> {
        let mut c = v2(0.0, 0.0);
        let mut k = 0.0;
        let mut nation = None;
        for &h in hs {
            if let Some(u) = self.unit(h) {
                if !u.kind.naval() && u.ship.is_none() {
                    c = c + u.pos;
                    k += 1.0;
                    nation = Some(u.nation as usize);
                }
            }
        }
        let n = nation?;
        let from = c * (1.0 / k);
        // 群れの中心が海や山にかかることがあるので、先頭の兵の位置で陸を決める
        let lead = hs.iter().filter_map(|&h| self.unit(h)).find(|u| !u.kind.naval()).map(|u| u.pos)?;
        if !self.needs_ferry(lead, goal) {
            return None;
        }
        let (src, dst) = (self.comp_at(lead), self.comp_at(goal));
        // 海の成分ごとに、もっともよい乗船地・上陸地
        let mut emb: Vec<(u16, f32, V2, V2)> = Vec::new();
        let mut lan: Vec<(u16, f32, V2, V2)> = Vec::new();
        let consider = |list: &mut Vec<(u16, f32, V2, V2)>, sc: u16, score: f32, l: V2, s: V2| {
            if sc == u16::MAX {
                return;
            }
            match list.iter_mut().find(|e| e.0 == sc) {
                Some(e) => {
                    if score < e.1 {
                        *e = (sc, score, l, s);
                    }
                }
                None => list.push((sc, score, l, s)),
            }
        };
        // 味方の港
        for (bi, b) in self.blds.iter().enumerate() {
            if !b.alive || !b.kind.city() || !b.done() {
                continue;
            }
            let bn = b.nation as usize;
            if bn != n && !self.allied(n, bn) {
                continue;
            }
            let Some((l, s)) = self.port_of(bi) else { continue };
            let cc = self.comp_at(l);
            let sc = self.sea_comp_at(s);
            if cc == src {
                consider(&mut emb, sc, l.dist(from) - 8.0, l, s);
            }
            if cc == dst {
                consider(&mut lan, sc, l.dist(goal) - 8.0, l, s);
            }
        }
        // 海岸
        for &(l, s) in self.coast.iter() {
            let (lp, sp) = (tc(l), tc(s));
            let cc = self.land_comp[Map::idx(l.0, l.1)];
            let sc = self.sea_comp[Map::idx(s.0, s.1)];
            if cc == src {
                consider(&mut emb, sc, lp.dist(from), lp, sp);
            }
            if cc == dst {
                consider(&mut lan, sc, lp.dist(goal), lp, sp);
            }
        }
        let mut best: Option<(f32, Ferry)> = None;
        for e in emb.iter() {
            for l in lan.iter() {
                if e.0 != l.0 {
                    continue;
                }
                let score = e.1 + l.1 + e.3.dist(l.3) * 0.15;
                if best.as_ref().map_or(true, |b| score < b.0) {
                    best = Some((score, Ferry { embark: e.2, sea: e.3, land: l.2, land_sea: l.3, goal, attack }));
                }
            }
        }
        best.map(|b| b.1)
    }

    /// 港に着いた兵が船に乗る（同じ行き先の船が待っていれば相乗り）
    pub(super) fn board_ship(&mut self, i: usize) {
        let Some(f) = self.units[i].ferry else { return };
        let ni = self.units[i].nation as usize;
        if self.units[i].pos.dist(f.embark) > 3.5 {
            // 岸に届かなかった
            self.units[i].ferry = None;
            return;
        }
        let mut ship = None;
        for (j, u) in self.units.iter().enumerate() {
            if !u.alive || u.nation as usize != ni || u.kind != UnitKind::Transport {
                continue;
            }
            if let Some(sh) = &u.ship {
                if sh.phase == 0 && sh.route == NONE && sh.cargo.len() < SHIP_CAP && sh.dest.dist(f.land_sea) < 3.0 && u.pos.dist(f.sea) < 3.0 {
                    ship = Some(j);
                    break;
                }
            }
        }
        let j = match ship {
            Some(j) => j,
            None => {
                let h = self.spawn_unit(ni, UnitKind::Transport, f.sea, 1.0);
                let u = self.unit_mut(h).unwrap();
                u.ship = Some(Box::new(Ship { cargo: Vec::new(), res: [0.0; 3], workers: 0, deliver_to: NONE, phase: 0, t: 2.5, dest: f.land_sea, land: f.land, goal: f.goal, attack: f.attack, route: NONE }));
                h.idx as usize
            }
        };
        let u = &self.units[i];
        let cargo = Cargo { kind: u.kind, power: u.power, hp: (u.hp / u.max_hp()).clamp(0.05, 1.0), hero: u.hero };
        let was_sel = u.selected;
        if let Some(sh) = self.units[j].ship.as_mut() {
            sh.cargo.push(cargo);
        }
        self.units[j].selected |= was_sel;
        self.kill_unit(i);
    }

    /// 船の動き：乗船を待ち、海を渡り、上陸・荷揚げする
    pub(super) fn update_ship(&mut self, i: usize, dt: f32) {
        let (phase, full) = {
            let sh = self.units[i].ship.as_mut().unwrap();
            if sh.phase == 0 {
                sh.t -= dt;
            }
            (sh.phase, sh.cargo.len() >= SHIP_CAP || sh.t <= 0.0)
        };
        if phase == 0 {
            if full {
                let dest = self.units[i].ship.as_ref().unwrap().dest;
                let u = &mut self.units[i];
                u.ship.as_mut().unwrap().phase = 1;
                u.order = Order::Move(dest);
                u.reset_nav();
            }
            return;
        }
        let dest = self.units[i].ship.as_ref().unwrap().dest;
        if self.nav(i, dest, 1.2, dt) {
            self.unload_ship(i);
        }
    }

    /// 上陸・荷揚げして船は去る
    fn unload_ship(&mut self, i: usize) {
        let Some(sh) = self.units[i].ship.take() else { return };
        let ni = self.units[i].nation as usize;
        let pos = self.units[i].pos;
        let mut land = sh.land;
        if land.dist(pos) > 6.0 {
            // 行き先に届かなかったときは、いちばん近い岸に降ろす
            if let Some(&(l, _)) = self.coast.iter().min_by(|a, b| tc(a.1).dist(pos).partial_cmp(&tc(b.1).dist(pos)).unwrap()) {
                land = tc(l);
            }
        }
        if sh.route != NONE {
            // 植民地・米国からの船団：積み荷を届ける
            let to = if sh.deliver_to == NONE { ni } else { sh.deliver_to as usize };
            if to < NATIONS.len() && self.nations[to].alive {
                let em = self.era_mult();
                for k in 0..3 {
                    self.nations[to].res[k] += sh.res[k];
                    self.nations[to].earned[k] += sh.res[k];
                }
                let mut wk = 0;
                for _ in 0..sh.workers {
                    if self.nations[to].pop() < self.pop_cap(to) {
                        let p = self.passable_near(land);
                        let h = self.spawn_unit(to, UnitKind::Worker, p, 1.0);
                        self.nations[to].workers += 1;
                        self.auto_gather(h);
                        wk += 1;
                    }
                }
                let _ = em;
                self.sea.delivered += 1;
                if to == self.player {
                    let r = &CONVOYS[sh.route as usize];
                    let t = tf!("{}からの船団が着いた（食糧{:.0} 木材{:.0} 鉄{:.0}{}）", "A convoy from {} arrived (food {:.0}, wood {:.0}, iron {:.0}{})", "来自{}的船队抵达（粮食{:.0} 木材{:.0} 铁{:.0}{}）",
                        tx(r.name), sh.res[0], sh.res[1], sh.res[2],
                        if wk > 0 { tf!("、民{}人", ", {} workers", "，民{}人", wk) } else { String::new() });
                    self.push_msg(t, [0.6, 0.85, 1.0]);
                }
            }
        } else {
            // 兵を降ろす
            let mut hs = Vec::new();
            let sel = self.units[i].selected && ni == self.player;
            for (k, c) in sh.cargo.iter().enumerate() {
                let off = v2((k % 3) as f32 * 0.7 - 0.7, (k / 3) as f32 * 0.7 - 0.7);
                let p = self.passable_near(land + off);
                let h = self.spawn_unit(ni, c.kind, p, c.power);
                let u = self.unit_mut(h).unwrap();
                u.hp = u.max_hp() * c.hp;
                u.hero = c.hero;
                u.selected = sel;
                hs.push(h);
            }
            if !hs.is_empty() {
                let (goal, attack) = (sh.goal, sh.attack);
                // 上陸後に向かう先が同じ陸でなければ、その場に留まる
                if !self.needs_ferry(land, goal) {
                    let pl = self.player_orders;
                    self.order_move(&hs, goal, attack);
                    self.player_orders = pl;
                }
                if ni == self.player {
                    self.push_msg(tf!("{}人が上陸した", "{} troops landed", "{}人登陆了", hs.len()), [0.7, 0.9, 1.0]);
                }
            }
        }
        self.kill_unit(i);
    }

    /// 通れる陸の近くの点
    pub fn passable_near(&self, p: V2) -> V2 {
        if self.passable_at(p) {
            return p;
        }
        let (cx, cy) = p.tile();
        for r in 1..12i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let q = v2((cx + dx) as f32 + 0.5, (cy + dy) as f32 + 0.5);
                    if self.passable_at(q) {
                        return q;
                    }
                }
            }
        }
        p
    }

    /// 近くの海の点
    fn sea_near(&self, p: V2) -> Option<V2> {
        let (cx, cy) = p.tile();
        for r in 0..20i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let q = v2((cx + dx) as f32 + 0.5, (cy + dy) as f32 + 0.5);
                    if self.sea_at(q) {
                        return Some(q);
                    }
                }
            }
        }
        None
    }

    /// 輸送船が沈められた
    pub(super) fn ship_sunk(&mut self, ti: usize, an: usize) {
        let vn = self.units[ti].nation as usize;
        let central = an < NATIONS.len() && self.nations[an].bloc == Bloc::Central;
        if central {
            self.sea.sunk += 1;
            if vn == USA && !self.at_war(an, USA) {
                // 中立の米国の船を沈めた
                self.sea.us_anger += 20.0;
                self.push_msg(tf!("{} の Uボートが米国の船を沈めた（米国の参戦気運 {:.0}/100）", "A {} U-boat sank an American ship (US war fever {:.0}/100)", "{}的U艇击沉了美国船只（美国参战情绪 {:.0}/100）", tx(self.nations[an].name), self.sea.us_anger), [1.0, 0.7, 0.4]);
            } else if self.sea.unrestricted && self.nations[USA].bloc == Bloc::Neutral {
                // 乗っていた米国人の犠牲
                self.sea.us_anger += 4.0;
            }
        }
        if vn == self.player {
            let what = match self.units[ti].ship.as_ref() {
                Some(sh) if !sh.cargo.is_empty() => tf!("兵{}人を乗せた輸送船", "a transport carrying {} troops", "载有{}名士兵的运输船", sh.cargo.len()),
                Some(sh) if sh.route != NONE => tf!("{}からの船団", "the convoy from {}", "来自{}的船队", tx(CONVOYS[sh.route as usize].name)),
                _ => t!("輸送船", "a transport", "运输船").to_string(),
            };
            self.push_msg(tf!("{}が撃沈された", "{} was sunk", "{}被击沉了", what), [1.0, 0.4, 0.35]);
        }
    }

    /// 毎月：植民地・米国から船団を出し、米国の参戦気運を確かめる
    pub(super) fn monthly_sea(&mut self) {
        let em = self.era_mult();
        for (r, cr) in CONVOYS.iter().enumerate() {
            if !self.nations[cr.owner].alive || !self.nations[cr.to].alive {
                continue;
            }
            // 米国の物資船：交戦中の英仏へ、米国が中立か協商側のときだけ
            if cr.owner == USA && (self.nations[USA].bloc == Bloc::Central || !(0..NATIONS.len()).any(|o| self.war[cr.to][o])) {
                continue;
            }
            // 植民地の船団は2か月に1度ほど、米国の物資船は毎月
            if cr.owner != USA && !self.rng.chance(0.55) {
                continue;
            }
            let Some(start) = self.sea_near(ll_v2(cr.lon, cr.lat)) else { continue };
            let Some((land, sea)) = self.home_port(cr.to, start) else { continue };
            let h = self.spawn_unit(cr.owner, UnitKind::Transport, start, 1.0);
            let res = [cr.res[0] * em, cr.res[1] * em, cr.res[2] * em];
            let u = self.unit_mut(h).unwrap();
            u.ship = Some(Box::new(Ship { cargo: Vec::new(), res, workers: cr.workers, deliver_to: cr.to as u8, phase: 1, t: 0.0, dest: sea, land, goal: land, attack: false, route: r as u8 }));
            u.order = Order::Move(sea);
        }
        // 米国の参戦気運
        if self.nations[USA].alive && self.nations[USA].bloc == Bloc::Neutral {
            if self.sea.us_anger >= US_ENTRY {
                self.us_enters();
            } else {
                self.sea.us_anger = (self.sea.us_anger - 1.0).max(0.0);
            }
        }
    }

    /// 米国の参戦
    pub fn us_enters(&mut self) {
        self.join_war(USA, Bloc::Entente);
        for o in 0..NATIONS.len() {
            if self.nations[o].alive && self.nations[o].bloc == Bloc::Central {
                self.set_war_pub(USA, o);
            }
        }
        let text = t!(
            "度重なる Uボートの攻撃に、ついに米国の世論が動いた。ウィルソン大統領は「世界を民主主義にとって安全な場所にするために」とドイツに宣戦した。米軍は大西洋を渡ってフランスへ向かう。",
            "After repeated U-boat attacks, American opinion has finally turned. President Wilson declares war on Germany \"to make the world safe for democracy.\" American troops will cross the Atlantic to France.",
            "U艇的一再攻击终于激起了美国舆论。威尔逊总统宣称\"为了使世界对民主而言是安全的\"而对德宣战。美军将横渡大西洋前往法国。"
        );
        self.push_msg(tf!("【米国参戦】{}", "[America enters the war] {}", "【美国参战】{}", text), [1.0, 0.85, 0.3]);
        self.queue_banner(Banner { title: t!("米国の参戦", "America Enters the War", "美国参战").into(), text: text.into(), t: 9.0 });
    }

    /// 宣戦（陣営の確認をしない）
    pub fn set_war_pub(&mut self, a: usize, b: usize) {
        if a != b && !self.allied(a, b) && self.nations[a].alive && self.nations[b].alive && !self.war[a][b] {
            self.war[a][b] = true;
            self.war[b][a] = true;
            self.war_start[a][b] = self.years;
            self.war_start[b][a] = self.years;
            if a == self.player || b == self.player {
                let t = tf!("{} が {} に宣戦布告した", "{} declared war on {}", "{}向{}宣战", tx(self.nations[a].name), tx(self.nations[b].name));
                self.push_msg(t, [1.0, 0.45, 0.35]);
            }
        }
    }

    /// 船団の着く港：受け取る国の首都と同じ陸にある港のうち、出発点と同じ海で最寄りのもの
    fn home_port(&self, n: usize, from: V2) -> Option<(V2, V2)> {
        let home = self.nations[n].capital.and_then(|h| self.bld(h)).map(|b| b.center()).unwrap_or(self.nations[n].home);
        let hc = self.comp_at(home);
        let sc = self.sea_comp_at(from);
        let mut best: Option<(f32, (V2, V2))> = None;
        for (bi, b) in self.blds.iter().enumerate() {
            if !b.alive || !b.kind.city() || !b.done() || b.nation as usize != n {
                continue;
            }
            let Some((l, s)) = self.port_of(bi) else { continue };
            if self.sea_comp_at(s) != sc {
                continue;
            }
            // 本国の港を優先
            let d = s.dist(from) + if self.comp_at(l) == hc { 0.0 } else { 400.0 };
            if best.map_or(true, |x| d < x.0) {
                best = Some((d, (l, s)));
            }
        }
        best.map(|b| b.1)
    }

    /// Uボートを持っているか（同盟国）
    pub fn has_submarines(&self, n: usize) -> bool {
        self.units.iter().any(|u| u.alive && u.nation as usize == n && u.kind == UnitKind::Submarine)
    }
}

pub fn ll_v2(lon: f32, lat: f32) -> V2 {
    let (x, y) = ll_to_tile(lon, lat);
    v2(x, y)
}
