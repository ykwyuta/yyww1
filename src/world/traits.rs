//! 国ごとの史実に基づく特徴と、国ごとに造れる兵種

use super::*;

/// 特徴による補正（1.0＝補正なし）
#[derive(Clone, Copy, Debug)]
pub struct TraitMods {
    /// 生産（都市の税・採集）
    pub eco: f32,
    /// 兵種ごとの攻撃
    pub atk: [f32; N_KINDS],
    /// 兵種ごとの訓練費
    pub cost: [f32; N_KINDS],
    /// 軍の維持費
    pub upkeep: f32,
    /// 都市・建物が受ける損害
    pub def: f32,
    /// 鍛錬・教化の費用
    pub upgrade: f32,
    /// 損害による世論の悪化の受けやすさ
    pub morale: f32,
    /// 自国の領内で守る兵の受ける損害（内線・塹壕）
    pub entrench: f32,
}

impl TraitMods {
    pub const NONE: TraitMods = TraitMods { eco: 1.0, atk: [1.0; N_KINDS], cost: [1.0; N_KINDS], upkeep: 1.0, def: 1.0, upgrade: 1.0, morale: 1.0, entrench: 1.0 };
}

pub struct Trait {
    pub nation: usize,
    pub name: &'static str,
    pub desc: &'static str,
    /// この年（小数）から有効
    pub from: f32,
    /// この年（小数）まで有効
    pub to: f32,
    pub fx: fn(&mut TraitMods),
}

const ALL: (f32, f32) = (1900.0, 2000.0);

macro_rules! tr {
    ($n:expr, $name:expr, $desc:expr, $span:expr, $f:expr) => {
        Trait { nation: $n, name: $name, desc: $desc, from: $span.0, to: $span.1, fx: $f }
    };
}

const INF: usize = UnitKind::Infantry as usize;
const ART: usize = UnitKind::Artillery as usize;
const CAV: usize = UnitKind::Cavalry as usize;
const TNK: usize = UnitKind::Tank as usize;
const STM: usize = UnitKind::Storm as usize;
const ALP: usize = UnitKind::Alpine as usize;
const SUB: usize = UnitKind::Submarine as usize;
const DES: usize = UnitKind::Destroyer as usize;

pub const TRAITS: &[Trait] = &[
    tr!(GER, "プロイセン参謀本部", "モルトケ以来の参謀本部が鍛えた陸軍。陸の兵の攻撃+20%、鍛錬の費用-20%", ALL, |m| {
        for k in [INF, ART, CAV, STM] {
            m.atk[k] *= 1.2;
        }
        m.upgrade *= 0.8;
    }),
    tr!(GER, "内線作戦", "鉄道網で兵を素早く前線から前線へ回す。自国の領内で守る兵の受ける損害-30%", ALL, |m| m.entrench *= 0.7),
    tr!(GER, "城内平和", "開戦とともに政党も労組も政争をやめ、挙国一致で戦った（1917年まで）。損害による世論の悪化-50%", (1900.0, ymd(1917, 1, 1)), |m| m.morale *= 0.5),
    tr!(AUH, "皇帝への忠誠", "老帝フランツ・ヨーゼフのもとに諸民族がまとまった（1917年まで）。損害による世論の悪化-30%", (1900.0, ymd(1917, 1, 1)), |m| m.morale *= 0.7),
    tr!(AUH, "カルパティアの守り", "山脈に守られた国境。自国の領内で守る兵の受ける損害-10%", ALL, |m| m.entrench *= 0.9),
    tr!(GER, "Uボート艦隊", "英国の海上交通を断つ潜水艦隊。Uボートの攻撃+15%", ALL, |m| m.atk[SUB] *= 1.15),
    tr!(GER, "英国の海上封鎖", "北海を封鎖され、食糧と原料の輸入が途絶えた（1915年〜）。生産-10%", (ymd(1915, 1, 1), 2000.0), |m| m.eco *= 0.9),
    tr!(GER, "カブラの冬", "1916年の凶作で、カブラで飢えをしのいだ（1917年〜）。生産-8%", (ymd(1917, 1, 1), 2000.0), |m| m.eco *= 0.92),
    tr!(AUH, "皇帝猟兵", "チロルの山岳部隊。山岳兵の攻撃+20%", ALL, |m| m.atk[ALP] *= 1.2),
    tr!(AUH, "多民族の軍隊", "十を超える言語の兵からなる軍。維持費+10%", ALL, |m| m.upkeep *= 1.1),
    tr!(OTT, "ガリポリの守り", "ダーダネルス海峡の要塞群。都市・建物の受ける損害-30%", ALL, |m| m.def *= 0.7),
    tr!(OTT, "砂漠の騎兵", "アラブとトルコの騎兵。騎兵の訓練費-25%、攻撃+10%", ALL, |m| {
        m.cost[CAV] *= 0.75;
        m.atk[CAV] *= 1.1;
    }),
    tr!(BUL, "バルカンのプロイセン", "バルカン戦争で鍛えられた歩兵。歩兵の攻撃+15%", ALL, |m| m.atk[INF] *= 1.15),
    tr!(GBR, "ロイヤル・ネイビー", "世界最強の海軍。駆逐艦の攻撃+30%、訓練費-20%", ALL, |m| {
        m.atk[DES] *= 1.3;
        m.cost[DES] *= 0.8;
    }),
    tr!(GBR, "キッチナーの志願兵", "「祖国は君を必要としている」。歩兵の訓練費-15%", ALL, |m| m.cost[INF] *= 0.85),
    tr!(GBR, "陸上軍艦", "世界で初めて戦車を戦場に送った（1916年9月〜）。戦車の攻撃+15%", (ymd(1916, 9, 15), 2000.0), |m| m.atk[TNK] *= 1.15),
    tr!(FRA, "75mm野砲", "速射砲の傑作。砲兵の攻撃+25%", ALL, |m| m.atk[ART] *= 1.25),
    tr!(FRA, "攻勢至上主義", "銃剣突撃のエラン・ヴィタール（1914年）。歩兵の攻撃+10%、維持費+5%", (1900.0, ymd(1915, 1, 1)), |m| {
        m.atk[INF] *= 1.1;
        m.upkeep *= 1.05;
    }),
    tr!(RUS, "ロシアの蒸気ローラー", "尽きることのない兵の数。歩兵の訓練費-30%、維持費-15%、損害による世論の悪化-40%", ALL, |m| {
        m.cost[INF] *= 0.7;
        m.upkeep *= 0.85;
        m.morale *= 0.6;
    }),
    tr!(RUS, "コサック", "ドンとクバンの騎兵。騎兵の攻撃+25%", ALL, |m| m.atk[CAV] *= 1.25),
    tr!(RUS, "広大な国土", "攻め込んだ敵を呑み込む広い大地と冬。都市・建物の受ける損害-25%", ALL, |m| m.def *= 0.75),
    tr!(RUS, "旧式の軍", "将校も装備も足りず、統制の取れない大軍（1917年まで）。陸の兵の攻撃-15%", (1900.0, ymd(1917, 1, 1)), |m| {
        for k in [INF, ART, CAV] {
            m.atk[k] *= 0.85;
        }
    }),
    tr!(RUS, "砲弾不足", "1915年の砲弾危機。砲兵の攻撃-20%（1916年まで）", (1900.0, ymd(1916, 1, 1)), |m| m.atk[ART] *= 0.8),
    tr!(SRB, "バルカン戦争の古強者", "二度のバルカン戦争を戦い抜いた兵。歩兵の攻撃+20%", ALL, |m| m.atk[INF] *= 1.2),
    tr!(BEL, "要塞の国", "リエージュとアントウェルペンの要塞群。都市・建物の受ける損害-35%", ALL, |m| m.def *= 0.65),
    tr!(ITA, "アルピーニ", "アルプスの山岳部隊。山岳兵の攻撃+20%、訓練費-15%", ALL, |m| {
        m.atk[ALP] *= 1.2;
        m.cost[ALP] *= 0.85;
    }),
    tr!(ROU, "プロイェシュティの油田", "欧州有数の油田。生産+10%", ALL, |m| m.eco *= 1.1),
    tr!(GRE, "海洋の国", "島々と港の国。維持費-10%", ALL, |m| m.upkeep *= 0.9),
    tr!(POR, "最古の同盟", "1386年以来の英国の同盟国。生産+5%", ALL, |m| m.eco *= 1.05),
    tr!(USA, "民主主義の兵器廠", "世界一の工業力。生産+20%", ALL, |m| m.eco *= 1.2),
    tr!(USA, "ドウボーイ", "若く意気盛んな米兵。歩兵の攻撃+10%", ALL, |m| m.atk[INF] *= 1.1),
    tr!(NED, "中立の商人", "両陣営と取引を続けた商業国。生産+10%", ALL, |m| m.eco *= 1.1),
    tr!(SUI, "国民皆兵", "山国の民兵。都市・建物の受ける損害-40%、山岳兵の攻撃+15%", ALL, |m| {
        m.def *= 0.6;
        m.atk[ALP] *= 1.15;
    }),
    tr!(ESP, "中立の利", "両陣営への輸出で潤った。生産+10%", ALL, |m| m.eco *= 1.1),
    tr!(SWE, "鉄鉱石", "キルナの鉄鉱石。生産+10%", ALL, |m| m.eco *= 1.1),
    tr!(NOR, "大商船隊", "世界有数の商船隊。生産+5%", ALL, |m| m.eco *= 1.05),
    tr!(DEN, "酪農の国", "豊かな農業。生産+5%", ALL, |m| m.eco *= 1.05),
];

/// その時に有効な特徴
pub fn traits_of(n: usize, y: f32) -> impl Iterator<Item = &'static Trait> {
    TRAITS.iter().filter(move |t| t.nation == n && y >= t.from && y < t.to)
}

/// 時を問わずその国の特徴すべて
pub fn all_traits_of(n: usize) -> impl Iterator<Item = &'static Trait> {
    TRAITS.iter().filter(move |t| t.nation == n)
}

/// その国の特別な兵種と、造れるようになる時（年の小数）
pub fn special_kinds(n: usize) -> &'static [(UnitKind, f32)] {
    const TANK: f32 = ymd(1916, 9, 15);
    const STORM: f32 = ymd(1916, 7, 1);
    const FRA_TANK: f32 = ymd(1917, 4, 16);
    const USA_TANK: f32 = ymd(1918, 1, 1);
    match n {
        GER => &[(UnitKind::Storm, STORM), (UnitKind::Submarine, 0.0)],
        AUH => &[(UnitKind::Alpine, 0.0), (UnitKind::Submarine, 0.0)],
        GBR => &[(UnitKind::Tank, TANK), (UnitKind::Destroyer, 0.0)],
        FRA => &[(UnitKind::Tank, FRA_TANK), (UnitKind::Alpine, 0.0), (UnitKind::Destroyer, 0.0)],
        ITA => &[(UnitKind::Alpine, 0.0), (UnitKind::Destroyer, 0.0)],
        RUS => &[(UnitKind::Destroyer, 0.0)],
        USA => &[(UnitKind::Tank, USA_TANK), (UnitKind::Destroyer, 0.0)],
        SUI => &[(UnitKind::Alpine, 0.0)],
        _ => &[],
    }
}

/// 軍港で造れる船
pub fn naval_kinds(n: usize) -> Vec<UnitKind> {
    special_kinds(n).iter().map(|x| x.0).filter(|k| k.naval()).collect()
}

impl World {
    /// 特徴の補正を計算し直す（毎月）
    pub(super) fn refresh_traits(&mut self) {
        let y = self.year_f();
        for i in 0..NATIONS.len() {
            let mut m = TraitMods::NONE;
            for t in traits_of(i, y) {
                (t.fx)(&mut m);
            }
            self.nations[i].tm = m;
        }
    }

    /// その国がいまその兵種を造れるか
    pub fn kind_available(&self, n: usize, k: UnitKind) -> bool {
        match k {
            UnitKind::Worker | UnitKind::Infantry | UnitKind::Artillery | UnitKind::Cavalry => true,
            UnitKind::Transport => false,
            _ => n < NATIONS.len() && special_kinds(n).iter().any(|&(kk, from)| kk == k && self.year_f() >= from),
        }
    }

    /// その国の兵種の訓練費
    pub fn train_cost(&self, n: usize, k: UnitKind) -> [f32; 3] {
        let m = if n < NATIONS.len() { self.nations[n].tm.cost[k as usize] } else { 1.0 };
        let c = k.cost();
        [(c[0] * m).round(), (c[1] * m).round(), (c[2] * m).round()]
    }
}
