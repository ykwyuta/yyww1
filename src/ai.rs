//! コンピュータ国家の思考

use crate::geo::{NATIONS, Bloc, GER, AUH, GBR, FRA, RUS};
use crate::rng::Rng;
use crate::world::*;

pub struct AiState {
    pub rng: Rng,
    pub(crate) think_t: f32,
    pub(crate) war_t: f32,
    pub(crate) attack_t: f32,
    /// 好戦性（国ごとの性格）
    pub aggr: f32,
}

impl AiState {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let aggr = rng.range(0.7, 1.3);
        let think_t = rng.range(0.0, 1.0);
        AiState { rng, think_t, war_t: 10.0 + think_t * 5.0, attack_t: 0.0, aggr }
    }
}

pub fn update_all(w: &mut World, dt: f32) {
    let mut ais = std::mem::take(&mut w.ai);
    for n in 0..ais.len() {
        if (n == w.player && !w.ai_player) || !w.nations[n].alive {
            continue;
        }
        let a = &mut ais[n];
        a.think_t -= dt;
        a.war_t -= dt;
        a.attack_t -= dt;
        if a.think_t <= 0.0 {
            a.think_t = 1.0 + a.rng.f32() * 0.4;
            think(w, n, a);
        }
    }
    w.ai = ais;
}

fn find_spot(w: &World, n: usize, kind: BKind, center: V2, rmin: f32, rmax: f32, rng: &mut Rng) -> Option<(i32, i32)> {
    let tries = if kind == BKind::Dock { 120 } else { 60 };
    for k in 0..tries {
        let ang = rng.range(0.0, std::f32::consts::TAU);
        // 見つからなければ範囲を広げる
        let d = rng.range(rmin, rmax + (k / 20) as f32 * 3.0);
        let tx = (center.x + ang.cos() * d) as i32 - 1;
        let ty = (center.y + ang.sin() * d) as i32 - 1;
        if w.can_place(n, kind, tx, ty).is_ok() {
            return Some((tx, ty));
        }
    }
    None
}

struct Mil {
    h: Handle,
    order: Order,
    pos: V2,
    power: f32,
    kind: UnitKind,
}

fn think(w: &mut World, n: usize, a: &mut AiState) {
    let res = w.nations[n].res;
    let home = w.nations[n].home;
    let mut workers: Vec<(Handle, Order, V2)> = Vec::new();
    let mut soldiers: Vec<Mil> = Vec::new();
    let mut ships: Vec<Mil> = Vec::new();
    let mut gathering = [0usize; 3];
    for (i, u) in w.units.iter().enumerate() {
        if !u.alive || u.nation as usize != n || u.ship.is_some() {
            continue;
        }
        let h = Handle { idx: i as u32, gen: u.gen };
        if u.kind.naval() {
            if u.kind != UnitKind::Transport {
                ships.push(Mil { h, order: u.order, pos: u.pos, power: u.power, kind: u.kind });
            }
        } else if u.kind.military() {
            if u.ferry.is_none() {
                soldiers.push(Mil { h, order: u.order, pos: u.pos, power: u.power, kind: u.kind });
            }
        } else {
            if let Order::Gather { src, .. } = u.order {
                gathering[src.res()] += 1;
            }
            workers.push((h, u.order, u.pos));
        }
    }
    let mut cities = Vec::new();
    let mut barracks = Vec::new();
    let mut docks = Vec::new();
    let mut unfinished = Vec::new();
    let mut farms = 0;
    let mut n_barracks = 0;
    let mut n_docks = 0;
    for (i, b) in w.blds.iter().enumerate() {
        if !b.alive || b.nation as usize != n {
            continue;
        }
        let h = Handle { idx: i as u32, gen: b.gen };
        if !b.done() {
            unfinished.push(h);
        }
        match b.kind {
            BKind::Capital | BKind::Town => {
                if b.done() {
                    cities.push(h)
                }
            }
            BKind::Barracks => {
                n_barracks += 1;
                if b.done() {
                    barracks.push(h)
                }
            }
            BKind::Dock => {
                n_docks += 1;
                if b.done() {
                    docks.push(h)
                }
            }
            BKind::Farm => farms += 1,
            BKind::Fort => {}
        }
    }
    if cities.is_empty() {
        return;
    }
    let cap = w.pop_cap(n);
    let pop = w.nations[n].pop();
    let at_war = w.at_war_any(n) || w.descent.is_some();
    let nsold = w.nations[n].soldiers + w.nations[n].q_soldiers;

    // ---- 民の訓練（人口の上限まで）
    if pop < cap && res[0] >= 60.0 {
        let free: Vec<Handle> = cities.iter().copied().filter(|&c| w.bld(c).map_or(false, |b| b.queue.is_empty())).collect();
        if !free.is_empty() {
            let c = free[a.rng.below(free.len() as u32) as usize];
            let _ = w.enqueue(c, UnitKind::Worker);
        }
    }

    // ---- 未完成の建物に建設者を
    for &bh in unfinished.iter() {
        let builders = workers.iter().filter(|x| x.1 == Order::Build(bh)).count();
        if builders == 0 {
            let bc = w.bld(bh).unwrap().center();
            let mut best: Option<(f32, usize)> = None;
            for (k, x) in workers.iter().enumerate() {
                if matches!(x.1, Order::Build(_)) {
                    continue;
                }
                let d = x.2.dist(bc);
                if best.map_or(true, |b| d < b.0) {
                    best = Some((d, k));
                }
            }
            if let Some((_, k)) = best {
                w.order_build(&[workers[k].0], bh);
                workers[k].1 = Order::Build(bh);
            }
        }
    }

    // ---- 兵営（大国は2つ）
    let res = w.nations[n].res;
    let want_barracks = if NATIONS[n].pop >= 40 { 2 } else { 1 };
    if n_barracks < want_barracks && res[1] >= 150.0 && res[2] >= 30.0 {
        let c = if n_barracks == 0 { home } else { w.bld(cities[a.rng.below(cities.len() as u32) as usize]).unwrap().center() };
        if let Some((tx, ty)) = find_spot(w, n, BKind::Barracks, c, 3.0, 7.0, &mut a.rng) {
            if let Ok(bh) = w.place_building(n, BKind::Barracks, tx, ty) {
                assign_builder(w, &mut workers, bh);
            }
        }
    }

    // ---- 軍港（軍艦を造れる国）
    let res = w.nations[n].res;
    let naval = naval_kinds(n);
    if !naval.is_empty() && n_docks == 0 && at_war && res[1] >= 260.0 && res[2] >= 100.0 && a.rng.chance(0.3) {
        // 首都に近い港町から
        let mut ports: Vec<(f32, V2)> = cities.iter().filter_map(|&c| {
            let b = w.bld(c)?;
            w.port_of(c.idx as usize).map(|(l, _)| (b.center().dist(home), l))
        }).collect();
        ports.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        for &(_, l) in ports.iter().take(3) {
            if let Some((tx, ty)) = find_spot(w, n, BKind::Dock, l, 0.0, 4.0, &mut a.rng) {
                if let Ok(bh) = w.place_building(n, BKind::Dock, tx, ty) {
                    assign_builder(w, &mut workers, bh);
                    break;
                }
            }
        }
    }

    // ---- 資源配分
    let total = workers.len().max(1) as f32;
    let mut want = [0.45, 0.30, 0.25];
    let res = w.nations[n].res;
    for k in 0..3 {
        if res[k] > 1500.0 {
            want[k] *= 0.4;
        }
    }
    if w.nations[n].upkeep > w.nations[n].income[0] * 0.5 {
        want[0] += 0.15;
    }
    let sumw: f32 = want.iter().sum();
    // 農場が足りなければ作る
    let food_target = (total * want[0] / sumw * 0.7).round() as usize;
    if farms < food_target.max(1) && res[1] >= 60.0 && unfinished.len() < 3 {
        let c = cities[a.rng.below(cities.len() as u32) as usize];
        let cc = w.bld(c).unwrap().center();
        if let Some((tx, ty)) = find_spot(w, n, BKind::Farm, cc, 2.5, 6.0, &mut a.rng) {
            if let Ok(bh) = w.place_building(n, BKind::Farm, tx, ty) {
                assign_builder(w, &mut workers, bh);
            }
        }
    }
    for x in workers.iter() {
        if x.1 != Order::Idle {
            continue;
        }
        // 不足している資源
        let mut best_k = 0;
        let mut best_def = f32::MIN;
        for k in 0..3 {
            let def = want[k] / sumw * total - gathering[k] as f32;
            if def > best_def {
                best_def = def;
                best_k = k;
            }
        }
        let p = x.2;
        let src = match best_k {
            0 => w.free_farm(n, p).map(Src::Farm),
            2 => w.nearest_mine_for(Some(n), p, 22.0).map(Src::Mine),
            _ => None,
        }
        .or_else(|| w.nearest_forest_for(Some(n), p, 16).map(|(x, y)| Src::Forest(x, y)))
        .or_else(|| w.nearest_mine_for(Some(n), p, 30.0).map(Src::Mine))
        .or_else(|| w.free_farm(n, p).map(Src::Farm));
        if let Some(s) = src {
            gathering[s.res()] += 1;
            w.order_gather(&[x.0], s);
        }
    }

    // ---- 敵の方角（いちばん近い交戦国の都市）
    let enemy_city = |w: &World, from: V2| -> Option<(f32, V2, usize)> {
        let mut best: Option<(f32, V2, usize)> = None;
        for (bi, b) in w.blds.iter().enumerate() {
            if b.alive && b.kind.city() && b.done() && w.at_war(n, b.nation as usize) {
                let mut d = b.center().dist(from);
                // 海の向こうの都市は遠く見る
                if w.needs_ferry(from, b.center()) {
                    d = d * 1.6 + 40.0;
                }
                if best.map_or(true, |x| d < x.0) {
                    best = Some((d, b.center(), bi));
                }
            }
        }
        best
    };

    // ---- 塹壕陣地：戦時、敵に近い都市のそばに築く
    let res = w.nations[n].res;
    let fort_c = BKind::Fort.cost();
    let can_pay = res[0] >= fort_c[0] && res[1] >= fort_c[1] && res[2] >= fort_c[2];
    let forts_building = unfinished.iter().any(|&h| w.bld(h).map_or(false, |b| b.kind == BKind::Fort));
    if can_pay && at_war && !forts_building && w.fort_count(n) < w.fort_cap(n) && a.rng.chance(0.2) {
        // 敵にもっとも近い自国の都市
        let mut front: Option<(f32, V2, V2)> = None;
        for &c in cities.iter() {
            let cc = w.bld(c).unwrap().center();
            if let Some((d, ep, _)) = enemy_city(w, cc) {
                if front.map_or(true, |f| d < f.0) {
                    front = Some((d, cc, ep));
                }
            }
        }
        if let Some((_, cc, ep)) = front {
            let toward = cc + (ep - cc).norm() * 5.0;
            if let Some((tx, ty)) = find_spot(w, n, BKind::Fort, toward, 0.0, 4.0, &mut a.rng) {
                if let Ok(bh) = w.place_building(n, BKind::Fort, tx, ty) {
                    assign_builder(w, &mut workers, bh);
                }
            }
        }
    }

    // ---- 軍の訓練：兵は民から徴兵する。軍事費が重くなりすぎない範囲で
    let nat = &w.nations[n];
    let res = nat.res;
    let share_goal = if at_war {
        // 世論が悪いときは控える。英国は徴兵制（1916年）までは志願兵だけ
        // ロシアは動員が遅い（1915年まで）
        let base = if n == GBR && w.year_f() < ymd(1916, 1, 27) {
            0.35
        } else if n == RUS && w.year_f() < ymd(1915, 1, 1) {
            0.4
        } else {
            0.5
        };
        if nat.opinion < 30.0 { base - 0.12 } else { base }
    } else if nat.bloc == Bloc::Neutral {
        0.2
    } else {
        0.3
    };
    let pop_now = (nat.workers + nat.soldiers + nat.q_soldiers).max(1) as f32;
    let share = (nat.soldiers + nat.q_soldiers) as f32 / pop_now;
    let food_ok = res[0] > 250.0 && nat.income[0] > nat.upkeep * 1.1;
    if share < share_goal && food_ok && nat.workers > 6 && !barracks.is_empty() {
        let b = barracks[a.rng.below(barracks.len() as u32) as usize];
        if w.bld(b).map_or(false, |x| x.queue.len() < 2) {
            let k = pick_kind(w, n, &mut a.rng);
            if w.enqueue(b, k).is_err() {
                let _ = w.enqueue(b, UnitKind::Infantry);
            }
        }
    }
    // 軍艦
    if at_war && !docks.is_empty() && !naval.is_empty() {
        let want_ships = if n == GER { 7 } else if n == AUH { 3 } else { 4 };
        if ships.len() < want_ships && w.nations[n].res[2] > 100.0 && w.nations[n].workers > 10 {
            let d = docks[a.rng.below(docks.len() as u32) as usize];
            if w.bld(d).map_or(false, |x| x.queue.is_empty()) {
                let k = naval[a.rng.below(naval.len() as u32) as usize];
                let _ = w.enqueue(d, k);
            }
        }
    }

    // ---- 能力の鍛錬（人口の上限に達したら質で補う）
    let res = w.nations[n].res;
    if res[0] > 500.0 && res[1] > 400.0 && (pop >= cap || res[0] > 1500.0) && !soldiers.is_empty() {
        if let Some(s) = soldiers.iter().filter(|s| s.kind != UnitKind::Transport).min_by(|x, y| x.power.partial_cmp(&y.power).unwrap()) {
            let _ = w.upgrade_unit(s.h);
        }
    }
    if res[0] > 600.0 && res[1] > 400.0 && pop >= cap {
        let pw = |h: Handle| w.unit(h).map_or(f32::MAX, |u| u.power);
        if let Some(x) = workers.iter().filter(|x| w.unit(x.0).is_some()).min_by(|x, y| pw(x.0).partial_cmp(&pw(y.0)).unwrap()) {
            let _ = w.upgrade_unit(x.0);
        }
    }

    // ---- 外交：ドイツの AI は史実どおりに無制限潜水艦作戦を始める（年表のイベント）。ここでは何もしない
    if a.war_t <= 0.0 {
        a.war_t = 8.0 + a.rng.f32() * 4.0;
    }

    // ---- 軍の指揮
    let _ = nsold;
    command_army(w, n, a, &soldiers);
    command_navy(w, n, a, &ships);

    // ---- 英雄召喚（民を捧げるので最後に）
    w.ai_summon(n, &mut a.rng);
}

fn assign_builder(w: &mut World, workers: &mut [(Handle, Order, V2)], bh: Handle) {
    let bc = w.bld(bh).unwrap().center();
    let mut best: Option<(f32, usize)> = None;
    for (k, x) in workers.iter().enumerate() {
        if matches!(x.1, Order::Build(_)) {
            continue;
        }
        let d = x.2.dist(bc);
        if best.map_or(true, |b| d < b.0) {
            best = Some((d, k));
        }
    }
    if let Some((_, k)) = best {
        w.order_build(&[workers[k].0], bh);
        workers[k].1 = Order::Build(bh);
    }
}

/// 訓練する兵種を選ぶ
fn pick_kind(w: &World, n: usize, rng: &mut Rng) -> UnitKind {
    let mut opts: Vec<(UnitKind, f32)> = vec![(UnitKind::Infantry, 5.0), (UnitKind::Artillery, 2.2), (UnitKind::Cavalry, if w.era == Era::Early { 1.2 } else { 0.5 })];
    for &(k, _) in special_kinds(n) {
        if k.naval() || !w.kind_available(n, k) {
            continue;
        }
        let wt = match k {
            UnitKind::Tank => 2.0,
            UnitKind::Storm => 2.5,
            UnitKind::Alpine => 1.8,
            _ => 1.0,
        };
        opts.push((k, wt));
    }
    let tot: f32 = opts.iter().map(|o| o.1).sum();
    let mut x = rng.f32() * tot;
    for &(k, wt) in opts.iter() {
        x -= wt;
        if x <= 0.0 {
            return k;
        }
    }
    UnitKind::Infantry
}

fn command_army(w: &mut World, n: usize, a: &mut AiState, soldiers: &[Mil]) {
    if soldiers.is_empty() {
        return;
    }
    let home = w.nations[n].home;
    // 防衛：もっとも脅かされている自国の都市（周りの敵兵の力が大きいもの）へ、近くの兵を集める
    let mut threat: Option<(f32, V2, f32)> = None;
    for b in w.blds.iter() {
        if !b.alive || b.nation as usize != n || !b.kind.city() || !b.done() {
            continue;
        }
        let c = b.center();
        let (enemy, mine) = w.power_near(n, c, 10.0);
        if enemy > 0.0 {
            // 足りない分が大きいほど、首都ほど急ぐ
            let need = enemy * 1.3 + 1.0 - mine + if b.kind == BKind::Capital { 3.0 } else { 0.0 };
            if need > 0.0 && threat.map_or(true, |t| need > t.0) {
                threat = Some((need, c, enemy));
            }
        }
    }
    let idle: Vec<&Mil> = soldiers.iter().filter(|s| s.order == Order::Idle).collect();
    if let Some((need, p, _)) = threat {
        let mut cands: Vec<&Mil> = soldiers
            .iter()
            .filter(|s| !matches!(s.order, Order::Attack(_)) && s.pos.dist(p) > 4.0 && s.pos.dist(p) < 40.0 && !w.needs_ferry(s.pos, p))
            .collect();
        cands.sort_by(|a, b| a.pos.dist(p).partial_cmp(&b.pos.dist(p)).unwrap());
        let mut sum = 0.0;
        let mut hs = Vec::new();
        for s in cands {
            if sum >= need {
                break;
            }
            sum += s.power;
            hs.push(s.h);
        }
        if !hs.is_empty() {
            w.order_move(&hs, p, true);
        }
    }
    let at_war = (0..NATIONS.len()).any(|o| w.at_war(n, o));
    if !at_war {
        // 平時：遠くの兵は帰還
        let far: Vec<Handle> = soldiers.iter().filter(|s| s.order == Order::Idle && s.pos.dist(home) > 25.0).map(|s| s.h).collect();
        if !far.is_empty() {
            let g = home + v2(a.rng.range(-3.0, 3.0), a.rng.range(2.0, 4.0));
            w.order_move(&far, g, false);
        }
        return;
    }
    if a.attack_t > 0.0 {
        return;
    }
    // 守備隊：首都の近くに兵の2割を残す
    let garrison = (soldiers.len() / 5).max(2);
    let near_home = soldiers.iter().filter(|s| s.pos.dist(home) < 10.0).count();
    let mut cand: Vec<&Mil> = idle.iter().copied().filter(|s| s.kind != UnitKind::Transport).collect();
    if near_home <= garrison {
        cand.retain(|s| s.pos.dist(home) >= 10.0);
    }
    let need = ((soldiers.len() as f32) * 0.4).ceil().max(6.0) as usize;
    if cand.len() < need.min(soldiers.len()) {
        return;
    }
    // 兵ごとに最寄りの敵の都市を狙う
    let enemy_cities: Vec<V2> = w.blds.iter().filter(|b| b.alive && b.kind.city() && b.done() && w.at_war(n, b.nation as usize)).map(|b| b.center()).collect();
    let our_cities: Vec<V2> = w.blds.iter().filter(|b| b.alive && b.kind.city() && b.done() && (b.nation as usize == n || w.allied(n, b.nation as usize))).map(|b| b.center()).collect();
    let mut groups: Vec<(V2, Vec<Handle>)> = Vec::new();
    for s in cand.iter() {
        let mut best: Option<(f32, V2)> = None;
        for &c in enemy_cities.iter() {
            let mut d = c.dist(s.pos);
            if best.map_or(false, |x| d >= x.0) {
                continue;
            }
            if w.needs_ferry(s.pos, c) {
                d = d * 1.6 + 40.0;
            }
            if best.map_or(true, |x| d < x.0) {
                best = Some((d, c));
            }
        }
        if let Some((_, p)) = best {
            match groups.iter_mut().find(|g| g.0.dist(p) < 1.0) {
                Some(g) => g.1.push(s.h),
                None => groups.push((p, vec![s.h])),
            }
        }
    }
    for (p, hs) in groups {
        // 目標の周りの敵兵を数える（局所的に優勢なときだけ攻める）
        let (mut enemy, _) = w.power_near(n, p, 14.0);
        // 都市そのものの守り（砲台）も敵の力に数える
        for b in w.blds.iter() {
            if b.alive && (b.kind.city() || b.kind == BKind::Fort) && b.done() && b.center().dist(p) < 6.0 && w.at_war(n, b.nation as usize) {
                enemy += if b.kind == BKind::Fort { 3.0 } else { 4.0 } * (b.hp / b.kind.max_hp());
            }
        }
        let my_power: f32 = hs.iter().filter_map(|&h| w.unit(h)).map(|u| u.power).sum();
        let attack = my_power >= enemy * 1.7 + 4.0 && hs.len() >= 5;
        if attack {
            w.order_move(&hs, p, true);
            continue;
        }
        // 劣勢なら、目標にいちばん近い自国か同盟国の都市（前線）で守りを固める
        let mut front: Option<(f32, V2)> = None;
        for &c in our_cities.iter() {
            let d = c.dist(p);
            if best_front(front, d) {
                front = Some((d, c));
            }
        }
        if let Some((_, f)) = front {
            // 前線の都市の、敵の側に少し出たところ
            let g = f + (p - f).norm() * 2.5;
            let far: Vec<Handle> = hs.iter().copied().filter(|&h| w.unit(h).map_or(false, |u| u.pos.dist(g) > 6.0)).collect();
            if !far.is_empty() {
                w.order_move(&far, g, true);
            }
        }
    }
    a.attack_t = 20.0 + a.rng.f32() * 12.0;
}

fn best_front(cur: Option<(f32, V2)>, d: f32) -> bool {
    cur.map_or(true, |c| d < c.0)
}

/// 海軍：Uボートは敵の輸送船を狩り、駆逐艦は Uボートを狩る
fn command_navy(w: &mut World, n: usize, a: &mut AiState, ships: &[Mil]) {
    for s in ships.iter() {
        if s.order != Order::Idle {
            continue;
        }
        let prey = if s.kind == UnitKind::Submarine { UnitKind::Transport } else { UnitKind::Submarine };
        let range = if s.kind == UnitKind::Submarine { 110.0 } else { 60.0 };
        // 狙える獲物
        let mut best: Option<(f32, Handle)> = None;
        for (i, u) in w.units.iter().enumerate() {
            if !u.alive || u.kind != prey || !w.hostile(n, u) {
                continue;
            }
            let d = u.pos.dist(s.pos);
            if d < range && best.map_or(true, |b| d < b.0) {
                best = Some((d, w.uh(i)));
            }
        }
        if let Some((_, t)) = best {
            w.order_attack(&[s.h], Target::Unit(t));
            continue;
        }
        // 獲物がいなければ、敵（駆逐艦は自国）の港の沖を見回る
        let enemy_ports = s.kind == UnitKind::Submarine;
        let mut ports: Vec<V2> = Vec::new();
        for (bi, b) in w.blds.iter().enumerate() {
            if !b.alive || !b.kind.city() {
                continue;
            }
            let bn = b.nation as usize;
            let ok = if enemy_ports { w.at_war(n, bn) } else { bn == n || w.allied(n, bn) };
            if !ok {
                continue;
            }
            if let Some((_, sea)) = w.port_of(bi) {
                // Uボートは英仏の大西洋側の港（船団の着く所）を好む
                let atlantic = enemy_ports && (bn == GBR || bn == FRA);
                if sea.dist(s.pos) < 160.0 || atlantic {
                    ports.push(sea);
                    if atlantic {
                        ports.push(sea);
                    }
                }
            }
        }
        if !ports.is_empty() {
            let p = ports[a.rng.below(ports.len() as u32) as usize];
            let off = v2(a.rng.range(-6.0, 6.0), a.rng.range(-6.0, 6.0));
            let g = if w.sea_at(p + off) { p + off } else { p };
            w.order_move(&[s.h], g, true);
        }
    }
}
