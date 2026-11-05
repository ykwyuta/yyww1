//! 天使の降臨：1919年6月（ヴェルサイユ条約の日）までに勝者が現れなければ、
//! 天使の軍勢が欧州の空から降り、半年のうちにすべての都市を占領する（強制終了）

use super::*;

/// 天使がすべての都市を占領するまでの年数（半年）
pub const DESCENT_YEARS: f32 = 0.5;
/// 天使の能力（どの兵にも傷つけられない）
const DIVINE_POWER: f32 = 12.0;
/// 同時にいる天使の上限
const DIVINE_MAX: usize = 120;

pub struct Descent {
    /// 降臨した年（経過年）
    pub start: f32,
    /// 互換のため（使わない）
    pub revived: bool,
    /// 降臨時に人間が持っていた都市の数
    pub total: usize,
    pub taken: usize,
}

impl World {
    /// 残り年数（すべての都市が占領されるまで）
    pub fn descent_left(&self) -> Option<f32> {
        self.descent.as_ref().map(|d| (d.start + DESCENT_YEARS - self.years).max(0.0))
    }

    /// 1919年6月：天使が降臨する
    pub(super) fn begin_descent(&mut self) {
        let a = ANGEL;
        self.nations[a].alive = true;
        self.nations[a].appeared = true;
        self.nations[a].capital = None;
        let total = self.blds.iter().filter(|b| b.alive && b.kind.city() && b.done() && b.nation as usize != a).count();
        self.descent = Some(Descent { start: self.years, revived: false, total, taken: 0 });
        // 各国の首都に天使が降りる
        let caps: Vec<Handle> = (0..NATIONS.len()).filter_map(|n| if self.nations[n].alive { self.nations[n].capital } else { None }).collect();
        for c in caps {
            self.spawn_divine(c, 4);
        }
        let text = t!(
            "1919年6月28日、ヴェルサイユに講和の鐘は鳴らなかった。終わらぬ戦いを見かねて、天使の軍勢が欧州の空から降り立った。天使には何者も抗えず、半年のうちにすべての都市は天使のものとなる。",
            "June 28, 1919: no bells of peace ring at Versailles. Weary of the endless war, a host of angels descends from the skies of Europe. None can resist them, and within half a year every city will be theirs.",
            "1919年6月28日，凡尔赛并未响起和平的钟声。天使大军不忍见战争无休无止，从欧洲上空降临。无人能够抵挡天使，半年之内所有城市都将归于天使。"
        );
        self.push_msg(t!("天使が降臨した", "The angels have descended", "天使降临了").to_string(), [1.0, 0.9, 0.5]);
        self.banner = None;
        self.banner_queue.clear();
        self.queue_banner(Banner { title: t!("天使の降臨", "Descent of the Angels", "天使降临").to_string(), text: text.to_string(), t: 12.0 });
    }

    /// 都市に天使を出す
    fn spawn_divine(&mut self, city: Handle, n: usize) {
        let have = self.units.iter().filter(|u| u.alive && u.divine).count();
        for k in 0..n.min(DIVINE_MAX.saturating_sub(have)) {
            let Some(sp) = self.spawn_point(city) else { break };
            let kind = if k % 2 == 0 { UnitKind::Cavalry } else { UnitKind::Infantry };
            let h = self.spawn_unit(ANGEL, kind, sp, DIVINE_POWER);
            self.unit_mut(h).unwrap().divine = true;
        }
    }

    /// 毎月：予定どおりに都市を天使のものにし、天使を次の都市へ向かわせる
    pub(super) fn monthly_descent(&mut self) {
        let Some(d) = &self.descent else { return };
        let p = ((self.years - d.start) / DESCENT_YEARS).clamp(0.0, 1.0);
        let (total, taken) = (d.total, d.taken);
        let a = ANGEL;
        let rest: Vec<usize> = (0..self.blds.len()).filter(|&i| {
            let b = &self.blds[i];
            b.alive && b.kind.city() && b.done() && b.nation as usize != a
        }).collect();
        // 首都から遠い都市から（小さな都市から）先に
        let mut rest: Vec<(f32, usize)> = rest
            .into_iter()
            .map(|i| {
                let n = self.blds[i].nation as usize;
                let home = if n < NATIONS.len() { self.nations[n].home } else { self.blds[i].center() };
                (-self.blds[i].center().dist(home), i)
            })
            .collect();
        rest.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        let want = if p >= 1.0 { taken + rest.len() } else { ((total as f32 * p).ceil() as usize).max(taken) };
        let n = (want - taken).min(rest.len());
        let mut done = 0;
        for &(_, bi) in rest.iter() {
            if done >= n {
                break;
            }
            if !self.blds[bi].alive || self.blds[bi].nation as usize == a {
                continue;
            }
            self.capture_city(bi, a);
            let h = self.bh(bi);
            self.spawn_divine(h, 1);
            done += 1;
        }
        if let Some(d) = &mut self.descent {
            d.taken += done;
        }
        // 天使は次の都市へ
        let next: Vec<V2> = rest.iter().skip(n).take(12).map(|&(_, bi)| self.blds[bi].center()).collect();
        if !next.is_empty() {
            let idle: Vec<Handle> = (0..self.units.len()).filter(|&i| self.units[i].alive && self.units[i].divine && !matches!(self.units[i].order, Order::Attack(_))).map(|i| self.uh(i)).collect();
            for (k, h) in idle.into_iter().enumerate() {
                let g = next[k % next.len()];
                let pl = self.player_orders;
                self.order_move(&[h], g, true);
                self.player_orders = pl;
            }
        }
        // すべての都市が天使のものとなった：強制終了
        if p >= 1.0 && self.game_over.is_none() {
            let pl = self.player;
            let text = tf!("天使の軍勢が欧州のすべての都市を占領した。{} の戦いはここに終わる……", "The host of angels has occupied every city in Europe. The war of {} ends here...", "天使大军占领了欧洲的所有城市。{}的战争就此终结……", tx(self.nations[pl].name));
            self.game_over = Some((false, text));
        }
    }
}
