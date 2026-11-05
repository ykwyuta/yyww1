//! 世論：戦場での損失が相手より多い、国力に比べて軍事費が重い、食糧が尽きた、都市を失った……
//! といった状況が続くと世論は下がる。帝政の国では世論が尽きると革命が起きて降伏する。
//! 民主制の国では革命は起きないが、ストライキで生産が落ちる。

use super::*;

/// 革命が起きる世論
pub const REVOLT_AT: f32 = 4.0;
/// 不穏とされる世論
pub const UNREST_AT: f32 = 25.0;

/// 陣営の名前
pub fn bloc_name(b: Bloc) -> &'static str {
    match b {
        Bloc::Central => t!("同盟国", "Central Powers", "同盟国"),
        Bloc::Entente => t!("協商国", "Entente", "协约国"),
        Bloc::Neutral => t!("中立", "Neutral", "中立"),
    }
}

/// 世論による生産の倍率（世論 50 以上で 1.0、0 で 0.7）
pub fn opinion_eco(op: f32) -> f32 {
    0.7 + 0.3 * (op / 50.0).clamp(0.0, 1.0)
}

/// 世論の内訳（表示用）
#[derive(Clone, Copy, Default, Debug)]
pub struct OpinionParts {
    pub weariness: f32,
    pub casualties: f32,
    pub spending: f32,
    pub food: f32,
    pub cities: f32,
    pub other: f32,
}

impl OpinionParts {
    pub fn total(&self) -> f32 {
        self.weariness + self.casualties + self.spending + self.food + self.cities + self.other
    }
}

impl World {
    /// 交戦中か
    pub fn at_war_any(&self, n: usize) -> bool {
        (0..NATIONS.len()).any(|o| self.war[n][o])
    }

    /// 国力に対する軍事費の比率（兵の割合と、維持費の食糧収入に対する比）
    pub fn mil_burden(&self, n: usize) -> f32 {
        let nat = &self.nations[n];
        let pop = (nat.workers + nat.soldiers).max(1) as f32;
        let share = nat.soldiers as f32 / pop;
        let upkeep_ratio = nat.upkeep / nat.income[0].max(0.3);
        share.max(upkeep_ratio * 0.6)
    }

    /// この月の世論の変化の見込み
    pub fn opinion_parts(&self, n: usize) -> OpinionParts {
        let nat = &self.nations[n];
        let mut p = OpinionParts::default();
        let war = self.at_war_any(n);
        if war {
            // 戦争の長期化
            p.weariness = -0.4 - (self.years / (END_YEAR - START_YEAR)).clamp(0.0, 1.0) * 0.8;
            // 損失が相手より多い（人口に対する割合で見る）
            let pop = ((nat.workers + nat.soldiers) as f32).max(10.0);
            let r = (nat.lost - nat.inflicted) / pop;
            p.casualties = (-r * 6.0).clamp(-3.0, 2.0);
            if p.casualties < 0.0 {
                p.casualties *= nat.tm.morale;
            }
        } else {
            p.weariness = if nat.opinion < 70.0 { 2.0 } else { 0.0 };
        }
        // 国力に対する軍事費
        let b = self.mil_burden(n);
        if b > 0.45 {
            p.spending = -((b - 0.45) * 10.0).min(3.0);
        } else if !war {
            p.spending = 0.5;
        }
        // 食糧が尽きた
        if nat.res[0] < 1.0 && nat.upkeep > 0.0 {
            p.food = -5.0;
        }
        // 都市の得失
        p.cities = (nat.won_cities as f32 * 2.0 - nat.lost_cities as f32 * 2.5).clamp(-6.0, 4.0);
        // 英雄がいれば士気が上がる
        if self.hero_of(n).is_some() {
            p.other += 1.0;
        }
        // 開戦の熱狂：開戦から半年は世論が下がりにくい
        if self.years < 0.5 {
            for v in [&mut p.weariness, &mut p.casualties, &mut p.spending] {
                if *v < 0.0 {
                    *v *= 0.5;
                }
            }
        }
        p
    }

    /// 毎月：世論を更新し、帝政の国では革命を起こす
    pub(super) fn monthly_opinion(&mut self) {
        for n in 0..NATIONS.len() {
            if !self.nations[n].alive {
                continue;
            }
            let p = self.opinion_parts(n);
            let d = p.total();
            let nat = &mut self.nations[n];
            let before = nat.opinion;
            nat.opinion = (nat.opinion + d).clamp(0.0, 100.0);
            nat.opinion_delta = nat.opinion - before;
            nat.lost = 0.0;
            nat.inflicted = 0.0;
            nat.won_cities = 0;
            nat.lost_cities = 0;
            let op = nat.opinion;
            let empire = NATIONS[n].gov == Gov::Empire;
            if n == self.player && op < UNREST_AT && before >= UNREST_AT {
                let t = if empire {
                    t!("世論が悪化し、国内が不穏になっている。このままでは革命が起きて降伏する！", "Public opinion has soured and unrest is spreading. If this goes on, revolution will force us to surrender!", "舆论恶化，国内动荡不安。照此下去将爆发革命而被迫投降！")
                } else {
                    t!("世論が悪化し、ストライキで生産が落ちている", "Public opinion has soured; strikes are cutting production", "舆论恶化，罢工导致生产下降")
                };
                self.push_msg(t.to_string(), [1.0, 0.5, 0.35]);
            }
            if empire && op <= REVOLT_AT && self.at_war_any(n) && self.descent.is_none() && !(self.protect_player && n == self.player) {
                self.revolution(n);
            }
        }
    }

    /// 革命：帝政は倒れ、その国は降伏する。都市は最寄りの交戦国に明け渡される
    pub fn revolution(&mut self, n: usize) {
        if !self.nations[n].alive {
            return;
        }
        let enemies: Vec<usize> = (0..NATIONS.len()).filter(|&o| self.war[n][o] && self.nations[o].alive).collect();
        let name = tx(self.nations[n].name);
        // 都市を、もっとも近い敵国の都市を持つ国へ
        let mut by = enemies.first().copied().unwrap_or(n);
        let mut gains = vec![0u32; NATIONS.len()];
        if !enemies.is_empty() {
            let enemy_cities: Vec<(V2, usize)> = self.blds.iter().filter(|b| b.alive && b.kind.city() && b.done() && enemies.contains(&(b.nation as usize))).map(|b| (b.center(), b.nation as usize)).collect();
            let mine: Vec<usize> = (0..self.blds.len()).filter(|&i| self.blds[i].alive && self.blds[i].nation as usize == n && self.blds[i].kind.city()).collect();
            for bi in mine {
                let c = self.blds[bi].center();
                let to = enemy_cities.iter().min_by(|a, b| a.0.dist(c).partial_cmp(&b.0.dist(c)).unwrap()).map(|x| x.1).unwrap_or(by);
                let b = &mut self.blds[bi];
                b.nation = to as u8;
                b.kind = BKind::Town;
                b.queue.clear();
                b.queue_t = 0.0;
                b.rally = None;
                gains[to] += 1;
            }
            by = (0..NATIONS.len()).max_by_key(|&o| gains[o]).unwrap_or(by);
            for o in enemies.iter().copied() {
                if gains[o] > 0 && self.nations[o].capital.and_then(|h| self.bld(h)).is_none() {
                    if let Some(i) = (0..self.blds.len()).find(|&i| self.blds[i].alive && self.blds[i].nation as usize == o && self.blds[i].kind.city()) {
                        self.blds[i].kind = BKind::Capital;
                        self.nations[o].capital = Some(self.bh(i));
                    }
                }
            }
        }
        self.nations[n].surrendered = true;
        self.territory_dirty = true;
        let text = tf!(
            "長引く戦争と重い犠牲に民衆の怒りが爆発した。{} で革命が起き、帝政は倒れた。新政府は講和を求めて降伏し、都市は敵国に明け渡された。",
            "Fury at the long war and its terrible losses has boiled over. Revolution has broken out in {}, and the monarchy has fallen. The new government sues for peace and surrenders; its cities are handed to the enemy.",
            "漫长的战争与惨重的牺牲点燃了民众的怒火。{}爆发革命，帝制倒台。新政府求和投降，城市被移交给敌国。",
            name
        );
        let title = tf!("{}の革命", "Revolution in {}", "{}革命", name);
        self.push_msg(format!("【{}】{}", title, text), [1.0, 0.75, 0.3]);
        self.queue_banner(Banner { title, text: text.clone(), t: 9.0 });
        if n == self.player {
            // 降伏：プレイヤーの負け
            let t = tf!("{} で革命が起き、降伏した……", "Revolution broke out in {}, and it surrendered...", "{}爆发革命，投降了……", name);
            self.eliminate(n, by);
            self.game_over = Some((false, t));
        } else {
            self.eliminate(n, by);
        }
        // 敵陣営の世論は上がる
        for o in enemies {
            if self.nations[o].alive {
                self.nations[o].opinion = (self.nations[o].opinion + 12.0).min(100.0);
            }
        }
    }
}
