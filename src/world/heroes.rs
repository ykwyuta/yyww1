//! 英雄召喚：民を捧げて、その国の英雄（名将・撃墜王・艦長など）を呼び出す。
//! 呼べるのは自国出身の英雄だけ。一国が同時に従えられる英雄は一人だけ。
//! 英雄は天使を傷つけられ、英雄の守る都市は天使にも奪われない。

use super::*;

/// 一度に捧げられる民の上限
pub const SACRIFICE_MAX: usize = 10;
/// 英雄が守る都市の範囲（タイル）
pub const HERO_GUARD: f32 = 15.0;

pub struct HeroDef {
    pub name: &'static str,
    /// 出身国
    pub nation: usize,
    /// 肩書
    pub era: &'static str,
    pub kind: UnitKind,
    /// 捧げた民の能力に対する倍率（1.5〜3.0）
    pub mult: f32,
    pub desc: &'static str,
}

macro_rules! hd {
    ($n:expr, $nat:expr, $e:expr, $k:ident, $m:expr, $d:expr) => {
        HeroDef { name: $n, nation: $nat, era: $e, kind: UnitKind::$k, mult: $m, desc: $d }
    };
}

pub const HEROES: &[HeroDef] = &[
    // ドイツ
    hd!("ヒンデンブルク", GER, "陸軍元帥", Infantry, 2.6, "タンネンベルクでロシア軍を包囲殲滅した救国の英雄"),
    hd!("ルーデンドルフ", GER, "参謀次長", Artillery, 2.4, "リエージュ要塞を落とし、1918年の春季攻勢を指揮した"),
    hd!("マッケンゼン", GER, "陸軍元帥", Cavalry, 2.3, "ゴルリツェ突破とセルビア・ルーマニア征服を指揮した騎兵将軍"),
    hd!("リヒトホーフェン", GER, "撃墜王「赤い男爵」", Cavalry, 2.0, "槍騎兵から飛行士に転じ、80機を撃墜した空の英雄"),
    hd!("ヴェディゲン", GER, "U-9 艦長", Submarine, 2.6, "1914年9月、一隻のUボートで英巡洋艦3隻を1時間で沈めた"),
    // オーストリア＝ハンガリー
    hd!("コンラート", AUH, "参謀総長", Infantry, 2.0, "開戦を主導したオーストリア軍の参謀総長"),
    hd!("ボロエヴィッチ", AUH, "「イゾンツォの獅子」", Alpine, 2.6, "イゾンツォ川で11度のイタリア軍の攻勢を退けた"),
    hd!("ゲオルク・フォン・トラップ", AUH, "Uボート艦長", Submarine, 2.2, "オーストリア海軍随一のUボート・エース"),
    // オスマン帝国
    hd!("ムスタファ・ケマル", OTT, "第19師団長", Infantry, 2.8, "ガリポリで英連邦軍の上陸を阻み、後にトルコ共和国を建てた"),
    hd!("エンヴェル・パシャ", OTT, "陸軍大臣", Infantry, 1.8, "青年トルコ人の指導者。帝国を同盟国側で参戦させた"),
    hd!("ファフレッティン・パシャ", OTT, "「砂漠の虎」", Cavalry, 2.1, "メディナを2年半守り抜いた将軍"),
    // ブルガリア
    hd!("ジェコフ", BUL, "総司令官", Infantry, 2.2, "セルビア戦役を指揮したブルガリア軍総司令官"),
    hd!("ヴァゾフ", BUL, "ドイラン軍司令官", Artillery, 2.3, "ドイランの戦いで英仏軍を退けた砲兵将軍"),
    // イギリス
    hd!("キッチナー", GBR, "陸軍大臣", Infantry, 2.2, "「祖国は君を必要としている」――数百万の志願兵を集めた"),
    hd!("ヘイグ", GBR, "英国派遣軍総司令官", Infantry, 2.0, "ソンムとパッシェンデールを戦い、百日攻勢で勝利を収めた"),
    hd!("アレンビー", GBR, "エジプト遠征軍司令官", Cavalry, 2.5, "ガザを破り、エルサレムに入城した騎兵将軍"),
    hd!("ロレンス", GBR, "「アラビアのロレンス」", Cavalry, 2.4, "アラブの反乱を率い、アカバを落とした"),
    hd!("ジェリコー", GBR, "大艦隊司令長官", Destroyer, 2.4, "ユトランド沖でドイツ大洋艦隊と戦った提督"),
    // フランス
    hd!("ジョッフル", FRA, "総司令官", Infantry, 2.3, "マルヌの奇跡でパリを救った「パパ・ジョッフル」"),
    hd!("フォッシュ", FRA, "連合軍総司令官", Infantry, 2.8, "「わが右翼は押され、中央は崩れる。状況は最高、我攻撃す」"),
    hd!("ペタン", FRA, "ヴェルダンの守将", Artillery, 2.6, "「奴らを通すな」――ヴェルダンを守り抜いた"),
    hd!("ギヌメール", FRA, "撃墜王", Cavalry, 1.9, "53機を撃墜したフランスの空の英雄"),
    // ロシア
    hd!("ブルシーロフ", RUS, "南西方面軍司令官", Infantry, 2.7, "1916年の大攻勢でオーストリア軍を壊滅寸前に追い込んだ"),
    hd!("ニコライ大公", RUS, "最高総司令官", Cavalry, 2.2, "開戦時のロシア軍最高総司令官。皇帝の叔父"),
    hd!("ユデーニチ", RUS, "カフカス軍司令官", Infantry, 2.3, "サルカミシュとエルズルムでオスマン軍を破った"),
    hd!("コルチャーク", RUS, "黒海艦隊司令官", Destroyer, 2.1, "機雷戦の名手。黒海艦隊を率いた提督"),
    // セルビア
    hd!("プトニク", SRB, "ヴォイヴォダ（元帥）", Infantry, 2.5, "ツェルの戦いでオーストリア軍を撃退した老元帥"),
    hd!("ミシッチ", SRB, "第1軍司令官", Infantry, 2.4, "コルバラの戦いでベオグラードを奪い返した"),
    // ベルギー
    hd!("アルベール1世", BEL, "「騎士王」", Infantry, 2.3, "イーゼル川で国土の最後の一角を守り抜いた国王"),
    // イタリア
    hd!("カドルナ", ITA, "参謀総長", Infantry, 1.8, "イゾンツォ川で11度の攻勢を命じた厳格な将軍"),
    hd!("ディアス", ITA, "参謀総長", Infantry, 2.4, "ピアーヴェで踏みとどまり、ヴィットリオ・ヴェネトで勝った"),
    hd!("ダンヌンツィオ", ITA, "詩人・飛行士", Cavalry, 1.9, "ウィーン上空を飛んでビラを撒いた詩人"),
    hd!("リッツォ", ITA, "魚雷艇長", Destroyer, 2.2, "小さな魚雷艇でオーストリアの戦艦2隻を沈めた"),
    // ルーマニア
    hd!("アヴェレスク", ROU, "第2軍司令官", Infantry, 2.2, "マラシュティでドイツ軍の攻勢を止めた"),
    hd!("テオドロイウ", ROU, "「ジウ川のジャンヌ・ダルク」", Infantry, 1.8, "兄の戦死後に志願して戦った女性将校"),
    // ギリシャ
    hd!("ダングリス", GRE, "国防軍司令官", Infantry, 1.9, "ヴェニゼロスとともに国防政府を率いた将軍"),
    // ポルトガル
    hd!("タマニーニ", POR, "ポルトガル派遣軍司令官", Infantry, 1.7, "西部戦線に送られたポルトガル軍を率いた"),
    // アメリカ
    hd!("パーシング", USA, "米国派遣軍司令官", Infantry, 2.6, "「ラファイエット、我ら来たれり」――米軍をフランスへ率いた"),
    hd!("ヨーク軍曹", USA, "第82師団の軍曹", Infantry, 2.0, "アルゴンヌの森で独力で132人を捕虜にした"),
    hd!("リッケンバッカー", USA, "撃墜王", Cavalry, 1.8, "米国随一のエース・パイロット"),
    // 中立国
    hd!("ヴィレ将軍", SUI, "スイス軍総司令官", Alpine, 2.0, "大戦中、中立を守るスイス軍を率いた"),
    hd!("スナイデルス", NED, "オランダ軍総司令官", Infantry, 1.7, "総動員でオランダの中立を守った"),
    hd!("アルフォンソ13世", ESP, "スペイン国王", Infantry, 1.6, "中立を守り、捕虜の救済に尽くした国王"),
    hd!("グスタフ5世", SWE, "スウェーデン国王", Infantry, 1.6, "北欧三国の中立を守った国王"),
    hd!("アムンセン", NOR, "極地探検家", Infantry, 1.5, "人類で初めて南極点に到達した探検家"),
    hd!("クリスチャン10世", DEN, "デンマーク国王", Infantry, 1.5, "大戦を通じて中立を守った国王"),
];

/// その国の英雄の番号
pub fn heroes_of(n: usize) -> impl Iterator<Item = usize> {
    (0..HEROES.len()).filter(move |&h| HEROES[h].nation == n)
}

impl World {
    /// 近くの海の点
    pub fn sea_near_pub(&self, p: V2) -> Option<V2> {
        let (cx, cy) = p.tile();
        for r in 0..14i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let q = v2((cx + dx) as f32 + 0.5, (cy + dy) as f32 + 0.5);
                    if self.sea_at(q) {
                        return Some(q);
                    }
                }
            }
        }
        None
    }

    /// その国が従えている英雄（ユニット番号）
    pub fn hero_of(&self, n: usize) -> Option<usize> {
        (0..self.units.len()).find(|&i| self.units[i].alive && self.units[i].nation as usize == n && (self.units[i].hero as usize) < HEROES.len())
    }

    /// 英雄がいま戦場にいるか
    pub fn hero_on_field(&self, h: usize) -> bool {
        self.units.iter().any(|u| u.alive && u.hero as usize == h)
    }

    /// 捧げられる民と、その能力の合計
    pub fn sacrifice_power(&self, n: usize, hs: &[Handle]) -> (Vec<Handle>, f32) {
        let mut v = Vec::new();
        let mut p = 0.0;
        for &h in hs {
            if let Some(u) = self.unit(h) {
                if u.nation as usize == n && u.kind == UnitKind::Worker && v.len() < SACRIFICE_MAX {
                    let (x, y) = u.pos.tile();
                    if Map::inb(x, y) && self.owner[Map::idx(x, y)] as usize == n {
                        v.push(h);
                        p += u.power;
                    }
                }
            }
        }
        (v, p)
    }

    /// 召喚できるか
    pub fn can_summon(&self, n: usize, hero: usize, hs: &[Handle]) -> Result<(), &'static str> {
        if HEROES[hero].nation != n {
            return Err(t!("他国出身の英雄は召喚できない", "Heroes from another state cannot be summoned", "无法召唤他国出身的英雄"));
        }
        if self.hero_of(n).is_some() {
            return Err(t!("英雄は一国に一人だけ", "Only one hero per state", "每国只能有一位英雄"));
        }
        if self.hero_on_field(hero) {
            return Err(t!("この英雄は既に戦場にいる", "This hero is already in the field", "该英雄已在战场上"));
        }
        let (v, _) = self.sacrifice_power(n, hs);
        if v.is_empty() {
            return Err(t!("自国領内の民を選んで捧げる", "Select workers inside your territory to dedicate", "请选择己方领土内的民献上"));
        }
        Ok(())
    }

    /// 民を捧げて英雄を召喚する。英雄の能力＝捧げた民の能力の合計×英雄の倍率
    pub fn summon_hero(&mut self, n: usize, hero: usize, hs: &[Handle]) -> Result<Handle, &'static str> {
        self.can_summon(n, hero, hs)?;
        let (v, p) = self.sacrifice_power(n, hs);
        let mut c = v2(0.0, 0.0);
        for &h in v.iter() {
            c = c + self.unit(h).unwrap().pos;
        }
        let c = self.passable_at_or_near(c * (1.0 / v.len() as f32));
        // 船の英雄は近くの海に現れる
        let c = if HEROES[hero].kind.naval() {
            match self.sea_near_pub(c) {
                Some(s) => s,
                None => return Err(t!("近くに海が無い", "There is no sea nearby", "附近没有海")),
            }
        } else {
            c
        };
        for &h in v.iter() {
            self.kill_unit(h.idx as usize);
        }
        let d = &HEROES[hero];
        let power = (p * d.mult).max(1.0);
        let h = self.spawn_unit(n, d.kind, c, power);
        self.unit_mut(h).unwrap().hero = hero as u8;
        let who = tx(self.nations[n].name);
        if n == self.player {
            let text = tf!("民{}人を捧げ、{}（{}）が駆けつけた。能力 {:.1}（民 {:.1} ×{:.1}）", "{} workers were dedicated, and {} ({}) has arrived. Power {:.1} (workers {:.1} ×{:.1})", "献上{}名民，{}（{}）赶来了。能力{:.1}（民{:.1}×{:.1}）", v.len(), tx(d.name), tx(d.era), power, p, d.mult);
            self.push_msg(text.clone(), [1.0, 0.85, 0.5]);
            self.queue_banner(Banner { title: tf!("英雄召喚・{}", "Hero: {}", "召唤英雄・{}", tx(d.name)), text: format!("{}\n{}", tx(d.desc), text), t: 7.0 });
        } else {
            self.push_msg(tf!("{} の英雄・{}が戦場に現れた", "{}'s hero {} has taken the field", "{}的英雄{}出现在战场上", who, tx(d.name)), [0.95, 0.8, 0.6]);
        }
        Ok(h)
    }

    fn passable_at_or_near(&self, p: V2) -> V2 {
        if self.passable_at(p) {
            p
        } else {
            let (cx, cy) = p.tile();
            for r in 1..10i32 {
                for dy in -r..=r {
                    for dx in -r..=r {
                        let q = v2((cx + dx) as f32 + 0.5, (cy + dy) as f32 + 0.5);
                        if self.passable_at(q) {
                            return q;
                        }
                    }
                }
            }
            p
        }
    }

    /// 英雄が守っている都市か（その国の英雄が近くにいる）
    pub fn hero_guarded(&self, bi: usize) -> bool {
        let b = &self.blds[bi];
        let n = b.nation as usize;
        let c = b.center();
        self.hero_of(n).map_or(false, |i| self.units[i].pos.dist(c) <= HERO_GUARD)
    }

    /// AI：戦時や天使の降臨時に、民を捧げて英雄を呼ぶ
    pub fn ai_summon(&mut self, n: usize, rng: &mut Rng) {
        if self.hero_of(n).is_some() {
            return;
        }
        let urgent = self.descent.is_some();
        let at_war = (0..NATIONS.len()).any(|o| self.at_war(n, o));
        if !(urgent || at_war) || !rng.chance(if urgent { 0.2 } else { 0.004 }) {
            return;
        }
        let workers: Vec<Handle> = (0..self.units.len())
            .filter(|&i| self.units[i].alive && self.units[i].nation as usize == n && self.units[i].kind == UnitKind::Worker)
            .map(|i| self.uh(i))
            .collect();
        if workers.len() < if urgent { 6 } else { 16 } {
            return;
        }
        let k = if urgent { workers.len() / 2 } else { 3 };
        let hs: Vec<Handle> = workers.into_iter().take(k.clamp(2, SACRIFICE_MAX)).collect();
        // 自国の英雄から（地上の兵を優先）、強いほど選ばれやすく
        let cands: Vec<usize> = heroes_of(n).filter(|&h| !self.hero_on_field(h) && !HEROES[h].kind.naval()).collect();
        if cands.is_empty() {
            return;
        }
        let tot: f32 = cands.iter().map(|&h| HEROES[h].mult).sum();
        let mut x = rng.f32() * tot;
        let mut pick = cands[0];
        for &h in cands.iter() {
            x -= HEROES[h].mult;
            pick = h;
            if x <= 0.0 {
                break;
            }
        }
        let _ = self.summon_hero(n, pick, &hs);
    }
}
