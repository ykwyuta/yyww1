//! チュートリアル：やることを1段ずつ示し、達成したら自動で次へ進む

use super::*;

/// 各段の達成条件
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Goal {
    /// 説明だけの段（「次へ」で進む）
    Next,
    CamMoved,
    SelectWorker,
    GatherWood,
    PlaceFarm,
    FarmDone,
    SelectCity,
    TrainWorker,
    GatherMine,
    PlaceBarracks,
    BarracksDone,
    TrainSoldier,
    Upgrade,
    MoveSoldier,
    OpenDiplo,
    /// 最終段
    Finish,
}

/// 強調表示する場所
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Hl {
    TopRes,
    TopUpkeep,
    Minimap,
    Commands,
    Btn(Act),
    Workers,
    Forest,
    Mine,
    Capital,
    Farms,
    Barracks,
    Soldiers,
}

pub(super) struct Step {
    title: &'static str,
    body: &'static str,
    goal: Goal,
    hl: &'static [Hl],
}

pub(super) const STEPS: &[Step] = &[
    Step {
        title: "ようこそ、フランスの指導者よ",
        body: "このチュートリアルでは、国づくりと戦争の基本を順番に学びます。\n\n・この枠に「やること」が出ます。\n・できたら自動で次へ進みます。\n・説明だけの段は「次へ」（Enter）で進みます。\n・チュートリアル中は他国から攻められません。",
        goal: Goal::Next,
        hl: &[],
    },
    Step {
        title: "画面の見方",
        body: "上の帯：資源（食糧・木材・鉄）と1秒あたりの増減、民と兵の数、維持費、世論、日付。\n左下：ミニマップ（クリックでその場所へ）。\n下の中央：選んだものの情報。\n右下：命令ボタン（Q・W・E などのキーでも押せます）。",
        goal: Goal::Next,
        hl: &[Hl::TopRes, Hl::Minimap, Hl::Commands],
    },
    Step {
        title: "地図を動かす",
        body: "マウスホイールで拡大・縮小。矢印キーか、画面の端にマウスを寄せると移動します。\n\n▶ やってみよう：地図を少し動かすか、拡大・縮小してください。\n\n（Home キーでいつでも首都に戻れます）",
        goal: Goal::CamMoved,
        hl: &[],
    },
    Step {
        title: "民を選ぶ",
        body: "作業着の人が「民」です。資源を集め、建物を建てます。兵も民から徴兵します。\n\n▶ やってみよう：光っている民を左クリックで選んでください。\n\n（左ボタンを押したままドラッグすると、まとめて選べます）",
        goal: Goal::SelectWorker,
        hl: &[Hl::Workers],
    },
    Step {
        title: "木材を集める",
        body: "濃い緑のマスが森です。\n\n▶ やってみよう：民を選んだまま、光っている森を右クリックしてください。民が木を伐り、都市まで運びます。",
        goal: Goal::GatherWood,
        hl: &[Hl::Forest],
    },
    Step {
        title: "農場をつくる",
        body: "食糧は農場から取れます。兵を養うのにも使う一番大事な資源です。\n\n▶ やってみよう：民を選び、右下の「農場」ボタン（Q）を押してから、都市の近くの平地をクリックして置いてください。\n緑なら置けます。赤なら置けません（右クリックで取り消し）。",
        goal: Goal::PlaceFarm,
        hl: &[Hl::Btn(Act::Place(BKind::Farm))],
    },
    Step {
        title: "農場の完成を待つ",
        body: "民が農場を建てています。完成すると、建てた民がそのまま耕して食糧を運びます。\n\n完成するまで待ちましょう。\n上の「×2」〜「×8」ボタンで早送りできます。",
        goal: Goal::FarmDone,
        hl: &[Hl::Farms],
    },
    Step {
        title: "首都を選ぶ",
        body: "首都・都市では民を増やせます。都市は史実のものだけで、新しく築くことはできません。\n\n▶ やってみよう：光っている首都（パリ）をクリックして選んでください。",
        goal: Goal::SelectCity,
        hl: &[Hl::Capital],
    },
    Step {
        title: "民を増やす",
        body: "▶ やってみよう：右下の「民」ボタン（Q）を押して、民を1人訓練してください。\n\n民と兵を合わせた人口には上限があります。敵の都市を奪うと上限が上がり、自国の都市を失うと下がります。",
        goal: Goal::TrainWorker,
        hl: &[Hl::Btn(Act::Train(UnitKind::Worker))],
    },
    Step {
        title: "鉄を掘る",
        body: "丘の上の坑道が鉱山です。鉄は兵の装備や能力の強化に使います。資源は開戦時がもっとも豊富で、掘るほど減っていきます。\n\n▶ やってみよう：民を選び、光っている鉱山を右クリックしてください。",
        goal: Goal::GatherMine,
        hl: &[Hl::Mine],
    },
    Step {
        title: "兵営を建てる",
        body: "兵は兵営で、民を徴兵して育てます。\n\n▶ やってみよう：民を選び「兵営」ボタン（W）を押して、自国の領内（自国の色の範囲）に置いてください。",
        goal: Goal::PlaceBarracks,
        hl: &[Hl::Btn(Act::Place(BKind::Barracks))],
    },
    Step {
        title: "兵営の完成を待つ",
        body: "完成するまで待ちましょう。\n\n建てる民を増やすと早く建ちます。民を選んで、建設中の建物を右クリックすると手伝います。",
        goal: Goal::BarracksDone,
        hl: &[Hl::Barracks],
    },
    Step {
        title: "兵を徴兵する",
        body: "兵営を選ぶと兵を訓練できます。兵を1人つくるたびに民が1人減ります。軍を大きくすれば、そのぶん経済は細ります。\n・歩兵：小銃で撃つ。騎兵に強い\n・砲兵：遠くから撃つ。歩兵と建物に強い\n・騎兵：足が速い。砲兵に強い\n\n▶ やってみよう：兵営を選び、どれか1つ訓練してください。",
        goal: Goal::TrainSoldier,
        hl: &[Hl::Barracks, Hl::Btn(Act::Train(UnitKind::Infantry))],
    },
    Step {
        title: "維持費と世論",
        body: "兵は存在するだけで食糧を食べます（上の「維持費」）。食糧が尽きると兵は弱っていきます。\n\n人口に対して兵が多すぎる、戦場での損失が敵より多い、といった状況が続くと「世論」が下がります。帝政の国では世論が尽きると革命が起きて降伏してしまいます。",
        goal: Goal::Next,
        hl: &[Hl::TopUpkeep],
    },
    Step {
        title: "少数精鋭",
        body: "民・兵は1人ずつ能力を高められます。\n\n▶ やってみよう：民か兵を選び、「教化」／「訓練」ボタン（T）を押してください。能力が +0.5 され、コマが少し大きくなります。",
        goal: Goal::Upgrade,
        hl: &[Hl::Btn(Act::Upgrade)],
    },
    Step {
        title: "兵と民の入れ替え",
        body: "「動員解除→民」「徴兵→歩兵」ボタン（C）で、兵を民に、民を兵に変えられます。ただし能力は 2/3 に減ります。\n\n戦いが落ち着いたら兵を民に戻して生産を増やし、危ないときは民を兵にする、という使い方ができます。",
        goal: Goal::Next,
        hl: &[Hl::Btn(Act::Convert)],
    },
    Step {
        title: "部隊を動かす",
        body: "兵を選んで地面を右クリックすると移動、敵を右クリックすると攻撃します。\n「進軍」（A）を押してから地点をクリックすると、途中の敵と戦いながら進みます。\n海の向こうを指すと、兵は港で輸送船に乗って渡ります。\n\n▶ やってみよう：兵を選んで、どこかへ動かしてください。",
        goal: Goal::MoveSoldier,
        hl: &[Hl::Soldiers],
    },
    Step {
        title: "外交",
        body: "陣営（同盟国・協商国）と中立国の一覧です。中立国に宣戦すると、その国は敵の陣営に加わります。陣営どうしの和睦はありません。\n\n▶ やってみよう：右上の「外交」ボタン（F2）を押して、外交画面を開いてください。",
        goal: Goal::OpenDiplo,
        hl: &[Hl::Btn(Act::Diplo)],
    },
    Step {
        title: "海と植民地",
        body: "英仏には植民地や米国から、船で資源と民が届きます。ドイツの Uボートはこれを沈めます。駆逐艦で Uボートを狩りましょう。\n\n米国は中立ですが、Uボートに船を沈められ続けると協商国側で参戦します。",
        goal: Goal::Next,
        hl: &[],
    },
    Step {
        title: "戦場の霧",
        body: "黒い所はまだ見ていない土地、暗い所は前に見た土地です。敵の兵は、自国の民・兵・建物の近くでしか見えません。\n\n民か兵を送って、周りを探ってみましょう。",
        goal: Goal::Next,
        hl: &[],
    },
    Step {
        title: "チュートリアル完了",
        body: "目的：同盟国以外の国（参戦した敵国）をすべて打ち破ること。1919年6月までに決着がつかなければ、天使が降りてすべての都市を占領します。\n\n困ったら F1 でヘルプ。文字が小さいときは Ctrl＋＋ で拡大、F11 で全画面にできます。\n\n「このまま続ける」で、この国のまま遊べます（他国からも攻められるようになります）。",
        goal: Goal::Finish,
        hl: &[],
    },
];

/// 段に入ったときの状態（達成判定の基準）
#[derive(Clone, Copy, Default)]
struct Base {
    cam: V2,
    zoom: f32,
    farms: usize,
    farms_done: usize,
    barracks: usize,
    barracks_done: usize,
    workers: usize,
    soldiers: usize,
    max_power: f32,
}

pub(super) struct Tut {
    pub step: usize,
    base: Base,
    /// 達成後、次の段へ進むまでの残り時間
    done_t: Option<f32>,
    /// この段で兵に移動を命じたか
    pub moved: bool,
    /// パネルの矩形（クリックが地図に抜けないように）
    pub panel: (f32, f32, f32, f32),
}

const PANEL_W: f32 = 430.0;

impl Game {
    /// チュートリアル用の対局を始める（フランスで開始）
    pub(super) fn start_tutorial(&mut self) {
        self.do_act(Act::Restart);
        self.title_sel = Some(FRA);
        self.do_act(Act::Start);
        self.world.protect_player = true;
        self.speed = 1;
        self.tut = Some(Tut { step: 0, base: Base::default(), done_t: None, moved: false, panel: (0.0, 0.0, 0.0, 0.0) });
        self.tut_enter(0);
    }

    pub(super) fn end_tutorial(&mut self) {
        self.tut = None;
        self.world.protect_player = false;
    }

    fn count_blds(&self, k: BKind, done_only: bool) -> usize {
        let pl = self.world.player;
        self.world.blds.iter().filter(|b| b.alive && b.nation as usize == pl && b.kind == k && (!done_only || b.done())).count()
    }

    fn own_units(&self) -> impl Iterator<Item = &Unit> {
        let pl = self.world.player;
        self.world.units.iter().filter(move |u| u.alive && u.nation as usize == pl)
    }

    pub(super) fn tut_enter(&mut self, step: usize) {
        let base = Base {
            cam: self.cam,
            zoom: self.zoom,
            farms: self.count_blds(BKind::Farm, false),
            farms_done: self.count_blds(BKind::Farm, true),
            barracks: self.count_blds(BKind::Barracks, false),
            barracks_done: self.count_blds(BKind::Barracks, true),
            workers: self.own_units().filter(|u| u.kind == UnitKind::Worker).count(),
            soldiers: self.own_units().filter(|u| u.kind.military()).count(),
            max_power: self.own_units().map(|u| u.power).fold(0.0, f32::max),
        };
        // 資源不足で詰まらないよう、操作を求める段では手持ちを補う
        let need: Option<[f32; 3]> = match STEPS[step].goal {
            Goal::PlaceFarm => Some(BKind::Farm.cost()),
            Goal::TrainWorker => Some(UnitKind::Worker.cost()),
            Goal::PlaceBarracks => Some(BKind::Barracks.cost()),
            Goal::TrainSoldier => Some(UnitKind::Artillery.cost()),
            Goal::Upgrade => Some([60.0, 40.0, 40.0]),
            _ => None,
        };
        if let Some(c) = need {
            let r = &mut self.world.nations[self.world.player].res;
            for k in 0..3 {
                r[k] = r[k].max(c[k] + 20.0);
            }
        }
        if let Some(t) = &mut self.tut {
            t.step = step;
            t.base = base;
            t.done_t = None;
            t.moved = false;
        }
    }

    fn tut_goal_met(&self, t: &Tut) -> bool {
        let b = &t.base;
        let pl = self.world.player;
        match STEPS[t.step].goal {
            Goal::Next | Goal::Finish => false,
            Goal::CamMoved => self.cam.dist(b.cam) > 5.0 || (self.zoom / b.zoom - 1.0).abs() > 0.2,
            Goal::SelectWorker => self.own_sel().iter().any(|&h| self.world.unit(h).map_or(false, |u| u.kind == UnitKind::Worker)),
            Goal::GatherWood => self.own_units().any(|u| matches!(u.order, Order::Gather { src: Src::Forest(..), .. })),
            Goal::GatherMine => self.own_units().any(|u| matches!(u.order, Order::Gather { src: Src::Mine(..), .. })),
            Goal::PlaceFarm => self.count_blds(BKind::Farm, false) > b.farms,
            Goal::FarmDone => self.count_blds(BKind::Farm, true) > b.farms_done,
            Goal::SelectCity => self.sel_b.and_then(|h| self.world.bld(h)).map_or(false, |bd| bd.nation as usize == pl && bd.kind.city()),
            Goal::TrainWorker => {
                self.own_units().filter(|u| u.kind == UnitKind::Worker).count() > b.workers
                    || self.world.blds.iter().any(|bd| bd.alive && bd.nation as usize == pl && bd.kind.city() && !bd.queue.is_empty())
            }
            Goal::PlaceBarracks => self.count_blds(BKind::Barracks, false) > b.barracks,
            Goal::BarracksDone => self.count_blds(BKind::Barracks, true) > b.barracks_done,
            Goal::TrainSoldier => {
                self.own_units().filter(|u| u.kind.military()).count() > b.soldiers
                    || self.world.blds.iter().any(|bd| bd.alive && bd.nation as usize == pl && bd.kind == BKind::Barracks && !bd.queue.is_empty())
            }
            Goal::Upgrade => self.own_units().map(|u| u.power).fold(0.0, f32::max) > b.max_power + 0.01,
            Goal::MoveSoldier => t.moved,
            Goal::OpenDiplo => self.show_diplo,
        }
    }

    /// 毎フレーム：達成判定と段送り
    pub(super) fn update_tutorial(&mut self, dt: f32) {
        let Some(t) = &self.tut else { return };
        if let Some(rest) = t.done_t {
            let rest = rest - dt;
            if rest <= 0.0 {
                let next = (t.step + 1).min(STEPS.len() - 1);
                self.tut_enter(next);
            } else if let Some(t) = &mut self.tut {
                t.done_t = Some(rest);
            }
            return;
        }
        if self.tut_goal_met(t) {
            if let Some(t) = &mut self.tut {
                t.done_t = Some(1.2);
            }
        }
    }

    pub(super) fn tut_next(&mut self) {
        let Some(t) = &self.tut else { return };
        if t.step + 1 < STEPS.len() {
            self.tut_enter(t.step + 1);
        }
    }

    fn hl_world(&self, h: Hl) -> Vec<V2> {
        let pl = self.world.player;
        let cap = self.world.nations[pl].capital.and_then(|h| self.world.bld(h)).map(|b| b.center()).unwrap_or(self.world.nations[pl].home);
        match h {
            Hl::Workers => self.own_units().filter(|u| u.kind == UnitKind::Worker).take(8).map(|u| u.pos).collect(),
            Hl::Soldiers => self.own_units().filter(|u| u.kind.military()).take(8).map(|u| u.pos).collect(),
            Hl::Forest => self.world.nearest_forest(cap, 30).map(|(x, y)| vec![v2(x as f32 + 0.5, y as f32 + 0.5)]).unwrap_or_default(),
            Hl::Mine => self
                .world
                .map
                .mines
                .iter()
                .filter(|m| m.amount > 0.0)
                .map(|m| v2(m.x as f32 + 0.5, m.y as f32 + 0.5))
                .min_by(|a, b| a.dist(cap).partial_cmp(&b.dist(cap)).unwrap())
                .into_iter()
                .collect(),
            Hl::Capital => vec![cap],
            Hl::Farms => self.world.blds.iter().filter(|b| b.alive && b.nation as usize == pl && b.kind == BKind::Farm && !b.done()).map(|b| b.center()).collect(),
            Hl::Barracks => self.world.blds.iter().filter(|b| b.alive && b.nation as usize == pl && b.kind == BKind::Barracks).map(|b| b.center()).collect(),
            _ => Vec::new(),
        }
    }

    fn hl_rect(&self, h: Hl) -> Option<(f32, f32, f32, f32)> {
        let (w, hh) = (self.gfx.width as f32, self.gfx.height as f32);
        match h {
            Hl::TopRes => Some((4.0, 2.0, 456.0, TOP_H - 2.0)),
            Hl::TopUpkeep => Some((616.0, 2.0, 752.0, TOP_H - 2.0)),
            Hl::Minimap => {
                let (a, b, c, d) = self.mm_rect();
                Some((a, b, c, d))
            }
            Hl::Commands => Some((w - 388.0, hh - BOT_H + 6.0, w - 6.0, hh - 6.0)),
            Hl::Btn(a) => self.buttons.iter().find(|b| b.act == a).map(|b| (b.x0, b.y0, b.x1, b.y1)),
            _ => None,
        }
    }

    /// パネルと強調表示を描く（HUD の後に呼ぶ）
    pub(super) fn render_tutorial(&mut self) {
        let Some(t) = &self.tut else { return };
        let step = t.step;
        let done = t.done_t.is_some();
        let st = &STEPS[step];
        let pulse = 0.55 + 0.45 * (self.clock * 5.0).sin();
        let gold = [1.0, 0.85, 0.3, pulse];
        // 強調表示
        if !done {
            let mut need_workers = false;
            for &h in st.hl {
                if let Some((x0, y0, x1, y1)) = self.hl_rect(h) {
                    self.gfx.stroke_round(x0 - 4.0, y0 - 4.0, x1 + 4.0, y1 + 4.0, 6.0, 4.0, gold);
                } else if let Hl::Btn(_) = h {
                    // ボタンが出ていない：まず民（または建物）を選ぶよう促す
                    need_workers = true;
                }
                for p in self.hl_world(h) {
                    self.hl_world_ring(p, gold);
                }
            }
            if need_workers {
                let hl = match st.goal {
                    Goal::TrainWorker => Hl::Capital,
                    Goal::TrainSoldier => Hl::Barracks,
                    Goal::Upgrade | Goal::Next => Hl::Soldiers,
                    _ => Hl::Workers,
                };
                for p in self.hl_world(hl) {
                    self.hl_world_ring(p, gold);
                }
            }
        }
        // パネル
        let w = self.gfx.width as f32;
        let x0 = w - PANEL_W - 10.0;
        let y0 = self.top_h() + 10.0;
        if self.show_help || self.show_diplo || self.show_chron || self.show_market || self.show_save {
            // 外交・ヘルプ画面を隠さないよう小さな帯にする
            let y1 = y0 + 30.0;
            let x0 = w - 330.0;
            self.art.draw_stone_panel(&self.gfx, x0, y0, w - 10.0, y1);
            let s = if done { t!("✓ できました！", "✓ Done!", "✓ 完成了！").to_string() } else { tf!("チュートリアル {}/{}：この画面を閉じると続きます", "Tutorial {}/{}: close this screen to continue", "教程 {}/{}：关闭此画面后继续", step + 1, STEPS.len()) };
            let c = if done { [0.5, 1.0, 0.5, 1.0] } else { [0.95, 0.88, 0.6, 1.0] };
            self.gfx.text(&s, x0 + 12.0, y0 + 5.0, 310.0, 22.0, Font::Small, c);
            if let Some(t) = &mut self.tut {
                t.panel = (x0, y0, w - 10.0, y1);
            }
            return;
        }
        let tw = PANEL_W - 36.0;
        let body_h = self.gfx.measure_h(crate::lang::tx(st.body), Font::Normal, tw);
        let y1 = y0 + 92.0 + body_h + 56.0;
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + PANEL_W, y1);
        self.art.draw_inset_box(&self.gfx, x0 + 10.0, y0 + 78.0, x0 + PANEL_W - 10.0, y1 - 52.0);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + PANEL_W - 3.0, y1 - 3.0, 14.0);
        let hdr = tf!("チュートリアル　{} / {}", "Tutorial  {} / {}", "教程　{} / {}", step + 1, STEPS.len());
        self.gfx.text(&hdr, x0 + 18.0, y0 + 10.0, tw, 20.0, Font::Small, [0.75, 0.7, 0.6, 1.0]);
        // 進み具合
        let f = (step + 1) as f32 / STEPS.len() as f32;
        self.gfx.fill_rect(x0 + 18.0, y0 + 32.0, x0 + PANEL_W - 18.0, y0 + 36.0, [0.25, 0.2, 0.15, 1.0]);
        self.gfx.fill_rect(x0 + 18.0, y0 + 32.0, x0 + 18.0 + (PANEL_W - 36.0) * f, y0 + 36.0, [0.95, 0.75, 0.3, 1.0]);
        self.gfx.text(crate::lang::tx(st.title), x0 + 18.0, y0 + 44.0, tw, 34.0, Font::Big, [1.0, 0.88, 0.5, 1.0]);
        self.gfx.text(crate::lang::tx(st.body), x0 + 18.0, y0 + 86.0, tw, body_h + 10.0, Font::Normal, [0.95, 0.93, 0.88, 1.0]);
        let by = y1 - 46.0;
        if done {
            self.gfx.text(t!("✓ できました！", "✓ Done!", "✓ 完成了！"), x0 + 18.0, by + 4.0, tw, 30.0, Font::Big, [0.5, 1.0, 0.5, 1.0]);
        } else {
            match st.goal {
                Goal::Next => {
                    self.add_button(x0 + PANEL_W - 150.0, by, x0 + PANEL_W - 18.0, by + 34.0, t!("次へ ▶", "Next ▶", "下一步 ▶"), "", 0x0D, true, Act::TutNext, "");
                    self.add_button(x0 + 18.0, by, x0 + 128.0, by + 34.0, t!("やめる", "Quit", "退出"), "", 0, true, Act::TutQuit, t!("チュートリアルを終えて、このまま遊ぶ", "End the tutorial and keep playing", "结束教程，继续游玩"));
                }
                Goal::Finish => {
                    self.add_button(x0 + PANEL_W - 200.0, by, x0 + PANEL_W - 18.0, by + 34.0, t!("このまま続ける", "Keep playing", "继续游玩"), "", 0x0D, true, Act::TutQuit, "");
                    self.add_button(x0 + 18.0, by, x0 + 160.0, by + 34.0, t!("タイトルへ", "Title screen", "返回标题"), "", 0, true, Act::Restart, "");
                }
                _ => {
                    self.gfx.text(t!("達成すると自動で進みます", "Advances when done", "完成后自动继续"), x0 + 18.0, by + 8.0, PANEL_W - 228.0 - 26.0, 24.0, Font::Small, [0.7, 0.68, 0.6, 1.0]);
                    self.add_button(x0 + PANEL_W - 228.0, by, x0 + PANEL_W - 128.0, by + 34.0, t!("飛ばす", "Skip", "跳过"), "", 0, true, Act::TutNext, t!("この段を飛ばす", "Skip this step", "跳过此步"));
                    self.add_button(x0 + PANEL_W - 118.0, by, x0 + PANEL_W - 18.0, by + 34.0, t!("やめる", "Quit", "退出"), "", 0, true, Act::TutQuit, t!("チュートリアルを終えて、このまま遊ぶ", "End the tutorial and keep playing", "结束教程，继续游玩"));
                }
            }
        }
        if let Some(t) = &mut self.tut {
            t.panel = (x0, y0, x0 + PANEL_W, y1);
        }
    }

    fn hl_world_ring(&mut self, p: V2, c: [f32; 4]) {
        let (sx, sy) = self.world_to_screen(p);
        let r = (self.zoom * 1.1).max(16.0) + 4.0 * (self.clock * 5.0).sin();
        self.gfx.stroke_circle(sx, sy, r, 3.0, c);
        self.gfx.text_shadow("▼", sx - 20.0, sy - r - 30.0, 40.0, 28.0, Font::BigCenter, c);
    }
}

#[cfg(test)]
mod tests {
    use super::STEPS;

    /// チュートリアルの全段に英語・中国語の訳がある
    #[test]
    fn tutorial_translated() {
        for st in STEPS {
            assert!(crate::lang::has(st.title), "{}", st.title);
            assert!(crate::lang::has(st.body), "{}", st.body);
        }
    }
}
