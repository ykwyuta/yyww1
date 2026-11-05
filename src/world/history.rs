//! 史実イベント：七月危機の宣戦布告の連鎖、中立国の参戦、指導者の交代、大攻勢、スペインかぜなど
//!
//! 年表は日付の順に並べ、その日が来たら発生する。対象国が滅亡していれば省略する。

use super::*;

/// 各国の開戦時の君主家・政体。NATIONS と同じ並び
pub const INITIAL_HOUSE: &[&str] = &[
    "ホーエンツォレルン家", "ハプスブルク家", "オスマン家", "サクス＝コバーグ＝ゴータ家", "サクス＝コバーグ＝ゴータ家", "第三共和政", "ロマノフ家", "カラジョルジェヴィチ家",
    "サクス＝コバーグ＝ゴータ家", "サヴォイア家", "ホーエンツォレルン＝ジグマリンゲン家", "グリュックスブルク家", "第一共和政", "合衆国", "オラニエ＝ナッサウ家", "スイス連邦",
    "ボルボン家", "ベルナドッテ家", "グリュックスブルク家", "グリュックスブルク家",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HKind {
    /// 宣戦・参戦
    War,
    /// 戦い・攻勢
    Battle,
    /// 政治（指導者・君主の交代）
    Politics,
    /// 海の出来事
    Sea,
    /// 内乱・暴動
    Revolt,
    /// 災厄（疫病など）
    Disaster,
}

impl HKind {
    pub fn label(self) -> &'static str {
        match self {
            HKind::War => t!("開戦", "War", "开战"),
            HKind::Battle => t!("戦い", "Battle", "战役"),
            HKind::Politics => t!("政治", "Politics", "政治"),
            HKind::Sea => t!("海", "Sea", "海上"),
            HKind::Revolt => t!("騒乱", "Unrest", "动乱"),
            HKind::Disaster => t!("災厄", "Disaster", "灾祸"),
        }
    }
    /// 国にとって不利な出来事か
    #[allow(dead_code)]
    pub fn bad(self) -> bool {
        matches!(self, HKind::Revolt | HKind::Disaster)
    }
    pub fn color(self) -> [f32; 3] {
        match self {
            HKind::War => [1.0, 0.6, 0.4],
            HKind::Revolt | HKind::Disaster => [1.0, 0.55, 0.45],
            _ => [0.75, 0.9, 1.0],
        }
    }
}

/// イベントの効果（対象は原則としてイベントの国）
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum Fx {
    /// 人物・作戦：生産と兵の攻撃力を一定年数だけ高める（負なら混乱）
    Figure(&'static str, f32, f32, f32),
    /// 人物が去る
    Dismiss(&'static str),
    /// 資源を得る
    Grant(f32, f32, f32),
    /// 全兵の能力を上げる（時代上限まで）
    Train(f32),
    /// 首都に歩兵を召集
    Levy(u32),
    /// 兵の一部を失う
    Purge(f32),
    /// 蓄えの一部を失う
    Plunder(f32),
    /// 君主家が変わる
    House(&'static str),
    /// a が b に宣戦する（プレイヤーの国でも自動で起こる）
    War(usize, usize),
    /// 中立国が陣営に加わって参戦する
    Join(usize, Bloc),
    /// その国の世論が変わる
    Opinion(f32),
    /// すべての国の世論が変わる
    OpinionAll(f32),
    /// 米国の参戦気運（0:いつでも 1:ドイツが Uボートを持っていれば 2:無制限潜水艦作戦中なら）
    Anger(f32, u8),
    /// ドイツが無制限潜水艦作戦を始める（ドイツがプレイヤーなら勧めるだけ）
    Unrestricted,
    /// 疫病：すべての国が民・兵の一部を失う
    Flu(f32),
}

pub struct HEvent {
    /// 日付（年の小数）
    pub at: f32,
    pub nation: usize,
    pub kind: HKind,
    pub title: &'static str,
    pub text: &'static str,
    pub fx: &'static [Fx],
}

macro_rules! ev {
    ($y:expr, $m:expr, $d:expr, $n:expr, $k:ident, $t:expr, $x:expr, [$($fx:expr),* $(,)?]) => {
        HEvent { at: ymd($y, $m, $d), nation: $n, kind: HKind::$k, title: $t, text: $x, fx: &[$($fx),*] }
    };
}

use Fx::*;

/// 史実年表（日付の昇順）
pub const HISTORY: &[HEvent] = &[
    ev!(1914, 6, 28, AUH, Politics, "サラエボ事件", "オーストリア皇位継承者フランツ・フェルディナント大公夫妻が、サラエボでセルビア人の青年に暗殺された。七月危機の始まりである。", [Opinion(8.0)]),
    ev!(1914, 7, 28, AUH, War, "オーストリア、セルビアに宣戦", "最後通牒の一部を拒まれたオーストリア＝ハンガリーは、セルビアに宣戦した。", [War(AUH, SRB)]),
    ev!(1914, 7, 30, RUS, War, "ロシアの総動員", "スラヴの盟邦セルビアを守るため、ロシアは総動員を発令した。", [Levy(6)]),
    ev!(1914, 8, 1, GER, War, "ドイツ、ロシアに宣戦", "動員の撤回を拒んだロシアに、ドイツは宣戦した。", [War(GER, RUS)]),
    ev!(1914, 8, 3, GER, War, "ドイツ、フランスとベルギーに宣戦", "シュリーフェン計画に従い、ドイツ軍は中立国ベルギーを通ってフランスへ向かう。", [War(GER, FRA), War(GER, BEL)]),
    ev!(1914, 8, 4, GBR, War, "イギリス、ドイツに宣戦", "ベルギーの中立の侵犯を理由に、イギリスはドイツに宣戦した。「一枚の紙切れのために」――ドイツ宰相は嘆いた。", [War(GBR, GER)]),
    ev!(1914, 8, 6, AUH, War, "オーストリア、ロシアに宣戦", "オーストリア＝ハンガリーはロシアに宣戦し、セルビアはドイツに宣戦した。", [War(AUH, RUS), War(SRB, GER)]),
    ev!(1914, 8, 12, GBR, War, "英仏、オーストリアに宣戦", "イギリスとフランスはオーストリア＝ハンガリーに宣戦した。欧州の大国はすべて戦争に入った。", [War(GBR, AUH), War(FRA, AUH)]),
    ev!(1914, 8, 28, AUH, War, "オーストリア、ベルギーに宣戦", "オーストリア＝ハンガリーはベルギーにも宣戦した。", [War(AUH, BEL)]),
    ev!(1914, 9, 5, FRA, Battle, "マルヌの奇跡", "パリのタクシーまで動員した反撃で、ドイツ軍の進撃はマルヌ川で止まった。", [Figure("マルヌの奇跡", 0.0, 0.15, 0.25), Opinion(6.0)]),
    ev!(1914, 11, 2, OTT, War, "オスマン帝国の参戦", "ドイツの巡洋艦ゲーベンが黒海のロシアの港を砲撃し、オスマン帝国は同盟国側で参戦した。", [Join(OTT, Bloc::Central)]),
    ev!(1914, 12, 25, GBR, Politics, "クリスマス休戦", "西部戦線の塹壕で、両軍の兵が自然に銃を置き、歌を交わし、ともにサッカーをした。", [OpinionAll(4.0)]),
    ev!(1915, 4, 22, GER, Battle, "毒ガスの使用", "第二次イーペルの戦いで、ドイツ軍は塩素ガスを初めて大規模に使った。", [Figure("毒ガス", 0.0, 0.12, 0.5)]),
    ev!(1915, 5, 7, GER, Sea, "ルシタニア号事件", "英客船ルシタニア号がUボートに沈められ、米国人128人を含む1198人が亡くなった。", [Anger(25.0, 1)]),
    ev!(1915, 5, 23, ITA, War, "イタリアの参戦", "ロンドン密約で「未回収のイタリア」を約されたイタリアは、協商国側で参戦した。", [Join(ITA, Bloc::Entente)]),
    ev!(1915, 10, 14, BUL, War, "ブルガリアの参戦", "マケドニアの回復を求め、ブルガリアは同盟国側で参戦した。", [Join(BUL, Bloc::Central)]),
    ev!(1916, 2, 21, FRA, Battle, "ヴェルダンの戦い", "ドイツ軍はフランス軍を「失血死」させるため、ヴェルダンに大攻勢をかけた。", [Opinion(-4.0)]),
    ev!(1916, 3, 9, POR, War, "ポルトガルの参戦", "港に抑留したドイツ船の接収を受けて、ドイツはポルトガルに宣戦した。", [Join(POR, Bloc::Entente)]),
    ev!(1916, 5, 31, GBR, Sea, "ユトランド沖海戦", "大戦最大の海戦。英艦隊は多くを失ったが、ドイツ大洋艦隊は再び港を出なかった。", [Opinion(-2.0)]),
    ev!(1916, 6, 4, RUS, Battle, "ブルシーロフ攻勢", "ブルシーロフ将軍の大攻勢で、オーストリア軍の戦線は崩壊寸前となった。", [Figure("ブルシーロフ攻勢", 0.0, 0.2, 0.35)]),
    ev!(1916, 7, 1, GBR, Battle, "ソンムの戦い", "攻撃初日に英軍は5万7千の死傷者を出した。総力戦の時代である。", [Opinion(-5.0)]),
    ev!(1916, 8, 27, ROU, War, "ルーマニアの参戦", "ブルシーロフ攻勢の成功を見て、ルーマニアは協商国側で参戦した。", [Join(ROU, Bloc::Entente)]),
    ev!(1916, 9, 15, GBR, Battle, "戦車の初陣", "ソンムのフレール＝クールスレットで、英軍の戦車が初めて戦場に現れた。", [Figure("戦車の衝撃", 0.0, 0.08, 0.25)]),
    ev!(1916, 11, 21, AUH, Politics, "フランツ・ヨーゼフ帝の崩御", "68年在位した老皇帝が崩御し、カール1世が即位した。", [House("ハプスブルク家（カール1世）"), Opinion(-5.0)]),
    ev!(1917, 2, 1, GER, Sea, "無制限潜水艦作戦", "ドイツは、英国に向かう船は中立国の船でも警告なしに沈めると宣言した。", [Unrestricted]),
    ev!(1917, 3, 1, GER, Sea, "ツィンメルマン電報", "ドイツがメキシコに対米参戦を求めた電報が英国に解読され、米国で公表された。", [Anger(50.0, 2)]),
    ev!(1917, 3, 8, RUS, Revolt, "ペトログラードの食糧暴動", "パンを求める女性たちのデモが首都を覆い、兵士も加わった（二月革命）。", [Opinion(-12.0)]),
    ev!(1917, 3, 18, USA, Sea, "米国商船の撃沈", "ヴィジランシア号・シティ・オブ・メンフィス号など米国の商船が相次いで Uボートに沈められた。", [Anger(45.0, 2)]),
    ev!(1917, 4, 16, FRA, Battle, "ニヴェル攻勢", "失敗に終わった大攻勢のあと、フランス軍の各師団で抗命が広がった。", [Opinion(-6.0)]),
    ev!(1917, 6, 27, GRE, War, "ギリシャの参戦", "国王コンスタンティノス1世が退位し、ヴェニゼロスのもとでギリシャは協商国側で参戦した。", [Join(GRE, Bloc::Entente)]),
    ev!(1917, 7, 17, GBR, Politics, "ウィンザー家", "ドイツ風の家名を嫌い、英王室はサクス＝コバーグ＝ゴータ家からウィンザー家へ改称した。", [House("ウィンザー家")]),
    ev!(1917, 11, 20, GBR, Battle, "カンブレーの戦い", "400両を超える戦車がはじめて集中して使われた。", [Figure("戦車の集中運用", 0.0, 0.1, 0.25)]),
    ev!(1918, 3, 4, ESP, Disaster, "スペインかぜ（第一波）", "新型インフルエンザが世界に広がり始めた。中立国スペインの報道で知られたため「スペインかぜ」と呼ばれる。", [Flu(0.05)]),
    ev!(1918, 3, 21, GER, Battle, "ミハエル作戦（春季攻勢）", "東部から回した兵力で、ドイツ軍は最後の大攻勢に出た。", [Figure("春季攻勢", 0.0, 0.2, 0.35), Levy(6)]),
    ev!(1918, 7, 15, FRA, Battle, "第二次マルヌ会戦", "フォッシュ将軍の反撃でドイツ軍の攻勢は止まり、潮目が変わった。", [Figure("第二次マルヌ", 0.0, 0.12, 0.3)]),
    ev!(1918, 8, 8, GBR, Battle, "百日攻勢", "アミアンの戦いを皮切りに、連合軍は休みなく攻め続けた。ルーデンドルフはこの日を「ドイツ陸軍暗黒の日」と呼んだ。", [Figure("百日攻勢", 0.0, 0.15, 0.3)]),
    ev!(1918, 10, 1, ESP, Disaster, "スペインかぜ（第二波）", "より毒性の強い第二波が、塹壕と都市を襲った。", [Flu(0.08)]),
    ev!(1919, 1, 18, SUI, Politics, "講和の期限迫る", "パリでは講和会議が開かれようとしている。1919年6月28日までに決着がつかなければ、天使が降りるという噂が欧州を駆けめぐる。", [OpinionAll(-2.0)]),
];

/// 在世の人物（効果は年限つき）
pub struct Person {
    pub name: &'static str,
    pub eco: f32,
    pub mil: f32,
    pub until: f32,
}

/// 戦史の記録
pub struct Chron {
    /// 日付（年の小数）
    pub at: f32,
    pub nation: usize,
    pub kind: HKind,
    pub title: &'static str,
    pub text: &'static str,
    /// 対象国が滅んでいて起こらなかった
    pub skipped: bool,
}

impl World {
    /// 月が改まるごとに（と開始時に）史実イベントを発生させる
    pub(super) fn run_history(&mut self) {
        let y = self.year_f() + 1.0 / 365.0;
        // 人物の退場
        let years = self.years;
        for n in self.nations.iter_mut() {
            n.figures.retain(|f| f.until > years);
        }
        while self.hist_i < HISTORY.len() && HISTORY[self.hist_i].at <= y {
            let e = &HISTORY[self.hist_i];
            self.hist_i += 1;
            self.fire_event(e);
        }
        self.refresh_mods();
    }

    fn fire_event(&mut self, e: &'static HEvent) {
        let n = e.nation;
        let skipped = !self.nations[n].alive;
        self.chronicle.push(Chron { at: e.at, nation: n, kind: e.kind, title: e.title, text: e.text, skipped });
        if skipped {
            return;
        }
        // チュートリアル中はプレイヤーに不利な出来事を起こさない
        let shield = self.protect_player && n == self.player;
        let mut notes: Vec<String> = Vec::new();
        for fx in e.fx.iter() {
            match *fx {
                Figure(name, eco, mil, yrs) => {
                    if shield && (eco < 0.0 || mil < 0.0) {
                        continue;
                    }
                    let until = self.years + yrs;
                    let nat = &mut self.nations[n];
                    nat.figures.retain(|f| f.name != name);
                    nat.figures.push(Person { name, eco, mil, until });
                    let mut s = String::from(tx(name));
                    if eco != 0.0 {
                        s.push_str(&tf!(" 生産{:+.0}%", " production {:+.0}%", " 生产{:+.0}%", eco * 100.0));
                    }
                    if mil != 0.0 {
                        s.push_str(&tf!(" 武{:+.0}%", " military {:+.0}%", " 武{:+.0}%", mil * 100.0));
                    }
                    let months = (yrs * 12.0).round() as i32;
                    s.push_str(&tf!("（{}か月）", " ({} months)", "（{}个月）", months));
                    notes.push(s);
                }
                Dismiss(name) => {
                    let nat = &mut self.nations[n];
                    if nat.figures.iter().any(|f| f.name == name) {
                        nat.figures.retain(|f| f.name != name);
                        notes.push(tf!("{} が去る", "{} departs", "{}离去", tx(name)));
                    }
                }
                Grant(a, b, c) => {
                    let r = &mut self.nations[n].res;
                    r[0] += a;
                    r[1] += b;
                    r[2] += c;
                    notes.push(tf!("食糧{:.0} 木材{:.0} 鉄{:.0}", "food {:.0}, wood {:.0}, iron {:.0}", "粮食{:.0} 木材{:.0} 铁{:.0}", a, b, c));
                }
                Train(x) => {
                    let cap = self.power_cap();
                    let mut k = 0;
                    for u in self.units.iter_mut() {
                        if u.alive && u.nation as usize == n && u.kind.military() && u.power < cap {
                            let ratio = u.hp / u.max_hp();
                            u.power = (u.power + x).min(cap);
                            u.hp = ratio * u.max_hp();
                            k += 1;
                        }
                    }
                    if k > 0 {
                        notes.push(tf!("兵{}人の能力{:+.1}", "{} soldiers' power {:+.1}", "{}名士兵能力{:+.1}", k, x));
                    }
                }
                Levy(cnt) => {
                    let Some(c) = self.nations[n].capital else { continue };
                    let pw = (1.0 + self.years / (END_YEAR - START_YEAR)).min(self.power_cap());
                    let mut k = 0;
                    for _ in 0..cnt {
                        if let Some(sp) = self.spawn_point(c) {
                            self.spawn_unit(n, UnitKind::Infantry, sp, pw);
                            k += 1;
                        }
                    }
                    if k > 0 {
                        notes.push(tf!("歩兵{}人が召集された", "{} infantry are called up", "征召了{}名步兵", k));
                    }
                }
                Purge(f) => {
                    if shield {
                        continue;
                    }
                    let mut idx: Vec<usize> = (0..self.units.len()).filter(|&i| self.units[i].alive && self.units[i].nation as usize == n && self.units[i].kind.military()).collect();
                    // 都から遠い兵ほど離反しやすい
                    let home = self.nations[n].home;
                    idx.sort_by(|&a, &b| self.units[b].pos.dist(home).partial_cmp(&self.units[a].pos.dist(home)).unwrap());
                    let k = (idx.len() as f32 * f).round() as usize;
                    for &i in idx.iter().take(k) {
                        self.kill_unit(i);
                    }
                    if k > 0 {
                        notes.push(tf!("兵{}人を失う", "{} soldiers lost", "损失{}名士兵", k));
                    }
                }
                Plunder(f) => {
                    if shield {
                        continue;
                    }
                    for r in self.nations[n].res.iter_mut() {
                        *r *= 1.0 - f;
                    }
                    notes.push(tf!("蓄えの{:.0}%を失う", "{:.0}% of stores lost", "损失{:.0}%的储备", f * 100.0));
                }
                House(h) => {
                    if self.nations[n].house != h {
                        notes.push(tf!("君主家：{} → {}", "ruling house: {} → {}", "君主家：{} → {}", tx(self.nations[n].house), tx(h)));
                        self.nations[n].house = h;
                    }
                }
                War(a, b) => {
                    if self.protect_player && (a == self.player || b == self.player) {
                        continue;
                    }
                    if self.nations[a].alive && self.nations[b].alive && !self.war[a][b] {
                        self.set_war_pub(a, b);
                        notes.push(tf!("{}が{}に宣戦", "{} declares war on {}", "{}向{}宣战", tx(self.nations[a].name), tx(self.nations[b].name)));
                    }
                }
                Join(k, bloc) => {
                    if !self.nations[k].alive || self.nations[k].bloc != Bloc::Neutral || (self.protect_player && k == self.player) {
                        continue;
                    }
                    self.join_war(k, bloc);
                    let side = if bloc == Bloc::Central { t!("同盟国", "the Central Powers", "同盟国") } else { t!("協商国", "the Entente", "协约国") };
                    notes.push(tf!("{}が{}側で参戦", "{} joins the war on the side of {}", "{}加入{}一方参战", tx(self.nations[k].name), side));
                }
                Opinion(d) => {
                    if shield && d < 0.0 {
                        continue;
                    }
                    let o = &mut self.nations[n].opinion;
                    *o = (*o + d).clamp(0.0, 100.0);
                    notes.push(tf!("世論{:+.0}", "public opinion {:+.0}", "舆论{:+.0}", d));
                }
                OpinionAll(d) => {
                    for k in 0..NATIONS.len() {
                        let o = &mut self.nations[k].opinion;
                        *o = (*o + d).clamp(0.0, 100.0);
                    }
                    notes.push(tf!("各国の世論{:+.0}", "public opinion everywhere {:+.0}", "各国舆论{:+.0}", d));
                }
                Anger(d, cond) => {
                    let ok = match cond {
                        1 => self.has_submarines(GER) || self.sea.sunk > 0,
                        2 => self.sea.unrestricted,
                        _ => true,
                    };
                    if ok && self.nations[USA].bloc == Bloc::Neutral {
                        self.sea.us_anger += d;
                        notes.push(tf!("米国の参戦気運 +{:.0}（{:.0}/100）", "US war fever +{:.0} ({:.0}/100)", "美国参战情绪+{:.0}（{:.0}/100）", d, self.sea.us_anger));
                    }
                }
                Unrestricted => {
                    if self.player == GER && !self.ai_player {
                        if !self.sea.unrestricted {
                            notes.push(t!("参謀本部が無制限潜水艦作戦を求めている（外交画面で決める。米国の参戦を招く）", "The General Staff demands unrestricted submarine warfare (decide on the Diplomacy screen; it risks bringing America in)", "参谋本部要求实施无限制潜艇战（在外交画面决定，可能招致美国参战）").to_string());
                        }
                    } else if self.nations[GER].alive {
                        self.sea.unrestricted = true;
                        notes.push(t!("中立国の船も警告なしに沈める", "neutral ships will be sunk without warning", "中立国船只也将不经警告被击沉").to_string());
                    }
                }
                Flu(f) => {
                    let mut lost = 0;
                    for k in 0..NATIONS.len() {
                        if !self.nations[k].alive || (self.protect_player && k == self.player) {
                            continue;
                        }
                        let idx: Vec<usize> = (0..self.units.len()).filter(|&i| self.units[i].alive && self.units[i].nation as usize == k && !self.units[i].kind.naval() && !self.units[i].divine).collect();
                        let cnt = (idx.len() as f32 * f).round() as usize;
                        for j in 0..cnt {
                            let i = idx[(j * 7919 + k * 31) % idx.len()];
                            if self.units[i].alive {
                                self.kill_unit(i);
                                lost += 1;
                            }
                        }
                    }
                    notes.push(tf!("各国で{}人が倒れた", "{} fell ill and died across Europe", "各国共有{}人病倒", lost));
                }
            }
        }
        self.refresh_mods();
        self.territory_dirty = true;
        let name = tx(self.nations[n].name);
        let head = tf!("【{}・{}】{}", "[{} · {}] {}", "【{}・{}】{}", name, e.kind.label(), tx(e.title));
        // 本文はバナーと史記に任せ、記録欄は見出しと効果だけにする
        let mut msg = head;
        if !notes.is_empty() {
            msg.push_str(&tf!("（{}）", " ({})", "（{}）", notes.join(t!("、", "; ", "；"))));
        }
        self.push_msg(msg, e.kind.color());
        // 大きな出来事はバナーで知らせる（小国の出来事は記録のみ）
        let involves_player = n == self.player
            || e.fx.iter().any(|f| match *f {
                War(a, b) => a == self.player || b == self.player,
                Join(..) | OpinionAll(_) | Flu(_) | Unrestricted | Anger(..) => true,
                _ => false,
            });
        if involves_player || NATIONS[n].pop >= 40 {
            let mut text = tx(e.text).to_string();
            if !notes.is_empty() {
                text.push_str(&format!("\n{}", notes.join(t!("　", "; ", "；"))));
            }
            let title = format!("{}　{}", name, tx(e.title));
            self.queue_banner(Banner { title, text, t: 7.0 });
        }
    }

    /// 在世の人物の効果を集計する
    pub(super) fn refresh_mods(&mut self) {
        let years = self.years;
        for n in self.nations.iter_mut() {
            let (mut e, mut m) = (1.0f32, 1.0f32);
            for f in n.figures.iter().filter(|f| f.until > years) {
                e += f.eco;
                m += f.mil;
            }
            n.eco_mul = e.clamp(0.4, 3.0);
            n.mil_mul = m.clamp(0.5, 2.5);
        }
    }

    /// いまの告知を閉じて、次の告知へ
    pub fn dismiss_banner(&mut self) {
        self.banner = self.banner_queue.pop_front();
    }

    /// まだ出ていない告知の数
    pub fn banner_queue_len(&self) -> usize {
        self.banner_queue.len()
    }

    pub fn queue_banner(&mut self, b: Banner) {
        if self.banner.is_none() {
            self.banner = Some(b);
        } else {
            self.banner_queue.push_back(b);
        }
    }
}
