//! 戦況：各国の世論・損害・人口と、海上の情勢（Uボート・船団・米国の参戦気運）

use super::*;

impl Game {
    pub(super) fn render_status(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        let pw = 1040.0f32.min(w - 20.0);
        let ph = 640.0f32.min(h - 40.0);
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.45]);
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        self.gfx.text_shadow(t!("戦況", "The War", "战况"), x0 + 24.0, y0 + 10.0, 120.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        let s = tf!("{}　天使の降臨まで あと{}か月", "{}   {} months until the angels descend", "{}　距天使降临还有{}个月", self.world.date_str(), (((END_YEAR - self.world.year_f()) * 12.0).ceil() as i32).max(0));
        self.gfx.text(&s, x0 + 120.0, y0 + 16.0, pw - 240.0, 24.0, Font::Normal, [0.85, 0.82, 0.75, 1.0]);
        self.add_button(x0 + pw - 90.0, y0 + 10.0, x0 + pw - 14.0, y0 + 40.0, t!("閉じる", "Close", "关闭"), "", 0, true, Act::CloseOverlay, "");
        let tc = [0.95, 0.92, 0.85, 1.0];
        let dim = [0.72, 0.69, 0.62, 1.0];
        let gold = [1.0, 0.85, 0.5, 1.0];

        // ---- 上：海上の情勢
        let (sx0, sy0, sx1) = (x0 + 12.0, y0 + 48.0, x0 + pw - 12.0);
        let sea_h = 96.0;
        self.art.draw_inset_box(&self.gfx, sx0, sy0, sx1, sy0 + sea_h);
        let sea = &self.world.sea;
        let us = &self.world.nations[USA];
        let us_state = if !us.alive {
            t!("滅亡", "fallen", "灭亡").to_string()
        } else if us.bloc == Bloc::Neutral {
            tf!("中立（参戦気運 {:.0}/100）", "neutral (war fever {:.0}/100)", "中立（参战情绪 {:.0}/100）", sea.us_anger)
        } else {
            tf!("参戦（{}）", "at war ({})", "参战（{}）", bloc_name(us.bloc))
        };
        let lines = [
            tf!(
                "無制限潜水艦作戦：{}　　同盟国が沈めた船 {}隻　　届いた船団 {}隻",
                "Unrestricted submarine warfare: {}    Ships sunk by the Central Powers: {}    Convoys delivered: {}",
                "无限制潜艇战：{}　　同盟国击沉船只 {}艘　　抵达船队 {}艘",
                if sea.unrestricted { t!("実施中", "in effect", "进行中") } else { t!("なし", "no", "无") },
                sea.sunk,
                sea.delivered
            ),
            tf!("米国：{}", "United States: {}", "美国：{}", us_state),
            t!(
                "英仏には植民地（カナダ・インド・アフリカなど）と米国から、船で資源と民が届く。Uボートはこれを沈められる。中立の米国の船を沈めると米国の参戦気運が高まり、100 で米国は協商国側で参戦する。",
                "Britain and France receive resources and workers by sea from their colonies (Canada, India, Africa…) and from America. U-boats can sink these ships. Sinking neutral American ships raises America's war fever; at 100 America joins the Entente.",
                "英法从殖民地（加拿大、印度、非洲等）及美国经海路获得资源与民。U艇可将其击沉。击沉中立美国的船只会提高美国的参战情绪，达到100时美国将加入协约国参战。"
            )
            .to_string(),
        ];
        let mut y = sy0 + 8.0;
        for (k, l) in lines.iter().enumerate() {
            let f = if k < 2 { Font::Normal } else { Font::Small };
            let lh = self.gfx.measure_h(l, f, sx1 - sx0 - 28.0).max(20.0);
            self.gfx.text(l, sx0 + 14.0, y, sx1 - sx0 - 28.0, lh + 2.0, f, if k == 0 { gold } else { tc });
            y += lh + 4.0;
        }

        // ---- 下：各国の世論・損害
        let (tx0, ty0, tx1, ty1) = (x0 + 12.0, sy0 + sea_h + 8.0, x0 + pw - 12.0, y0 + ph - 12.0);
        self.art.draw_inset_box(&self.gfx, tx0, ty0, tx1, ty1);
        let cols: [(&str, f32); 8] = [
            (t!("国", "State", "国家"), 0.0),
            (t!("陣営", "Side", "阵营"), 190.0),
            (t!("世論", "Opinion", "舆论"), 300.0),
            (t!("前月比", "Change", "环比"), 380.0),
            (t!("都市", "Cities", "城市"), 460.0),
            (t!("民/兵", "Wkr/Sol", "民/兵"), 530.0),
            (t!("失った兵力", "Lost", "失去兵力"), 640.0),
            (t!("与えた兵力", "Inflicted", "造成兵力"), 760.0),
        ];
        for (name, cx) in cols.iter() {
            self.gfx.text(name, tx0 + 14.0 + cx, ty0 + 6.0, 120.0, 20.0, Font::Small, gold);
        }
        let mut list: Vec<usize> = (0..NATIONS.len()).filter(|&i| self.world.nations[i].alive || self.world.nations[i].surrendered).collect();
        let rank = |b: Bloc| match b {
            Bloc::Central => 0,
            Bloc::Entente => 1,
            Bloc::Neutral => 2,
        };
        list.sort_by(|&a, &b| {
            let (na, nb) = (&self.world.nations[a], &self.world.nations[b]);
            rank(na.bloc).cmp(&rank(nb.bloc)).then(nb.strength.partial_cmp(&na.strength).unwrap())
        });
        let rowh = ((ty1 - ty0 - 56.0) / list.len().max(1) as f32).clamp(14.0, 21.0);
        let mut y = ty0 + 28.0;
        for &o in list.iter() {
            if y + rowh > ty1 - 26.0 {
                break;
            }
            let n = &self.world.nations[o];
            let c = if o == pl { gold } else if n.alive { tc } else { dim };
            self.gfx.fill_rect(tx0 + 14.0, y + 4.0, tx0 + 26.0, y + 16.0, c4(n.color, 1.0));
            let gov = if NATIONS[o].gov == Gov::Empire { "♛" } else { "" };
            self.gfx.text(&format!("{}{}", crate::lang::tx(n.name), gov), tx0 + 32.0, y, 170.0, rowh, Font::Small, c);
            let side = if n.surrendered { t!("降伏", "Surrendered", "投降").to_string() } else if !n.alive { t!("滅亡", "Fallen", "灭亡").to_string() } else { bloc_name(n.bloc).to_string() };
            self.gfx.text(&side, tx0 + 14.0 + 190.0, y, 110.0, rowh, Font::Small, c);
            if n.alive {
                let oc = opinion_color(n.opinion);
                self.gfx.fill_rect(tx0 + 14.0 + 300.0, y + 7.0, tx0 + 14.0 + 300.0 + 44.0 * n.opinion / 100.0, y + 14.0, c4(oc, 0.9));
                self.gfx.text(&format!("{:.0}", n.opinion), tx0 + 14.0 + 348.0, y, 32.0, rowh, Font::Small, c);
                self.gfx.text(&format!("{:+.1}", n.opinion_delta), tx0 + 14.0 + 380.0, y, 70.0, rowh, Font::Small, if n.opinion_delta < 0.0 { [1.0, 0.55, 0.45, 1.0] } else { c });
                self.gfx.text(&format!("{}", n.cities), tx0 + 14.0 + 460.0, y, 60.0, rowh, Font::Small, c);
                self.gfx.text(&format!("{}/{}", n.workers, n.soldiers), tx0 + 14.0 + 530.0, y, 100.0, rowh, Font::Small, c);
            }
            self.gfx.text(&format!("{:.0}", n.lost_total), tx0 + 14.0 + 640.0, y, 100.0, rowh, Font::Small, c);
            self.gfx.text(&format!("{:.0}", n.inflicted_total), tx0 + 14.0 + 760.0, y, 100.0, rowh, Font::Small, c);
            y += rowh;
        }
        self.gfx.text(
            t!("♛＝帝政（世論が 0 近くまで落ちると革命が起きて降伏する）", "♛ = empire (if opinion falls near 0, revolution forces surrender)", "♛＝帝制（舆论接近0时爆发革命而投降）"),
            tx0 + 14.0,
            ty1 - 24.0,
            tx1 - tx0 - 28.0,
            20.0,
            Font::Small,
            dim,
        );
    }
}
