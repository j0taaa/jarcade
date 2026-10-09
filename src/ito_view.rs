//! Shared-device vector UI. The room transport is never opened by Ito.
use crate::{
    online_style::primary,
    online_view::{clip, fit, page_input, wrap},
    platform,
    ui::{Icon, Ui, bordered},
};
use jarcade::{
    board_pan::BoardPan,
    feedback::Pulse,
    ito::{Game, Language, MAX_CLUE, Phase},
    layout::touch_point,
};
use macroquad::{miniquad::KeyMods, prelude::*};
const INK: Color = color_u8!(112, 94, 189, 255);

struct Drag {
    id: usize,
    owner: Option<u64>,
    order: Vec<usize>,
    to: usize,
}
pub struct ItoPage {
    game: Game,
    pub revision: u64,
    pan: BoardPan,
    blocked: bool,
    drag: Option<Drag>,
    size: Vec2,
    selected: Option<usize>,
    editing: Option<usize>,
    field: String,
    error: String,
    help: bool,
    confirm: bool,
    reset: bool,
    hands: bool,
    save_failed: bool,
    #[cfg(any(target_os = "android", target_os = "ios"))]
    keyboard_pan: BoardPan,
}
impl ItoPage {
    pub fn new(seed: u64) -> Self {
        Self {
            game: platform::load_ito(seed),
            revision: 0,
            pan: BoardPan::default(),
            blocked: false,
            drag: None,
            size: Vec2::ZERO,
            selected: None,
            editing: None,
            field: String::new(),
            error: String::new(),
            help: false,
            confirm: false,
            reset: false,
            hands: false,
            save_failed: false,
            #[cfg(any(target_os = "android", target_os = "ios"))]
            keyboard_pan: BoardPan::default(),
        }
    }
    fn save(&mut self) {
        self.revision += 1;
        self.save_failed = !platform::save_ito(&self.game);
    }
    fn close_editor(&mut self) {
        if self.editing.take().is_some() {
            platform::editor_close();
        }
    }
    pub fn interrupt(&mut self) {
        let hidden = self.game.conceal();
        self.drag = None;
        self.pan.cancel();
        self.blocked = true;
        self.close_editor();
        self.confirm = false;
        self.hands = false;
        if hidden {
            self.save();
        } else {
            self.revision += 1;
        }
    }
    pub fn enter(&mut self) {
        self.interrupt();
        self.help = false;
        self.reset = false;
    }
    pub fn needs_frame(&self) -> bool {
        self.pan.active() || self.drag.is_some()
    }
    pub fn poll(&mut self) {
        while let Some(e) = platform::editor_poll() {
            if e.id == 8 && self.editing.is_some() {
                self.field = e
                    .text
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(MAX_CLUE)
                    .collect();
                self.commit_field();
                if e.done {
                    self.close_editor();
                }
                self.revision += 1;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.editing.is_some() {
            while let Some(c) = get_char_pressed() {
                if !c.is_control() && self.field.chars().count() < MAX_CLUE {
                    self.field.push(c);
                    self.commit_field();
                    self.revision += 1;
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.field.pop();
                self.commit_field();
                self.revision += 1;
            }
            if is_key_pressed(KeyCode::Enter) {
                self.close_editor();
                clear_input_queue();
                self.revision += 1;
            }
        }
    }
    fn commit_field(&mut self) {
        if let Some(id) = self.editing {
            match self.game.clue(id, &self.field) {
                Ok(changed) => {
                    self.error.clear();
                    if changed {
                        self.save();
                    }
                }
                Err(e) => self.error = e.into(),
            }
        }
    }
    fn change_phase(&mut self) {
        self.pan = BoardPan::default();
        self.selected = None;
        self.close_editor();
        self.error.clear();
        self.save();
    }
    pub fn back(&mut self) -> bool {
        self.revision += 1;
        if self.editing.is_some() {
            self.close_editor();
            return false;
        }
        if self.help || self.confirm || self.reset || self.hands {
            self.help = false;
            self.confirm = false;
            self.reset = false;
            self.hands = false;
            self.pan = BoardPan::default();
            return false;
        }
        self.interrupt();
        true
    }
    pub fn announcement(&self) -> String {
        let (category, low, high) = self.game.category();
        let mut s = format!(
            "Jarcade. Ito. Offline local multiplayer. {:?}. {category}. Low: {low}. High: {high}. ",
            self.game.phase()
        );
        match self.game.phase() {
            Phase::Hand { player, .. } => {
                s.push_str(&format!("Private hand for player {}. ", player + 1));
                for &id in self.game.order() {
                    let c = self.game.card(id).unwrap();
                    if c.owner == player {
                        s.push_str(&format!(
                            "Card {}: {}. Clue: {}. ",
                            c.letter,
                            c.number.unwrap(),
                            c.clue
                        ));
                    }
                }
            }
            Phase::Handoff { player, .. } => s.push_str(&format!(
                "Pass to player {}. All numbers hidden. Reveal only when ready.",
                player + 1
            )),
            Phase::Arrange | Phase::Reveal | Phase::Result { .. } => {
                for (pos, &id) in self.game.order().iter().enumerate() {
                    let c = self.game.card(id).unwrap();
                    s.push_str(&format!(
                        "Position {}. Player {} card {}: {}",
                        pos + 1,
                        c.owner + 1,
                        c.letter,
                        c.clue
                    ));
                    if let Some(n) = c.number {
                        s.push_str(&format!(". Number {n}"));
                    }
                    s.push_str(". ");
                }
            }
            Phase::Setup => s.push_str(&format!(
                "{} players, {} cards each. English or Portuguese categories. Start round.",
                self.game.config().players,
                self.game.config().cards_each
            )),
        }
        if self.help {
            s = "Jarcade. Ito. Rules open. Close rules to continue.".into();
        }
        if !self.error.is_empty() {
            s.push_str(&self.error);
        }
        s
    }
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, KeyMods, bool)],
    ) -> (bool, Option<Pulse>) {
        ui.theme.accent = if ui.theme.saver {
            color_u8!(185, 170, 250, 255)
        } else {
            INK
        };
        ui.theme.panel = if ui.theme.saver {
            BLACK
        } else {
            color_u8!(248, 246, 253, 255)
        };
        let size = vec2(screen_width(), screen_height());
        if size != self.size {
            self.interrupt();
            self.size = size;
        }
        let w = (screen_width() - 32.).min(900.);
        let x = (screen_width() - w) / 2.;
        if ui.icon_button(Icon::Back, Rect::new(x, 8., 44., 44.), false) && self.back() {
            return (true, None);
        }
        ui.centered(
            "Ito",
            Rect::new(x + 52., 8., w - 104., 44.),
            25.,
            ui.theme.text,
            true,
        );
        if ui.button("?", Rect::new(x + w - 44., 8., 44., 44.), false) {
            self.interrupt();
            self.help = !self.help;
            self.pan = BoardPan::default();
            self.revision += 1;
        }
        if self.game.phase() != Phase::Setup
            && ui.button("↻", Rect::new(x + w - 94., 8., 44., 44.), false)
        {
            self.interrupt();
            self.reset = true;
            self.revision += 1;
        }
        let b = Rect::new(x, 66., w, screen_height() - 82.);
        if self.help {
            self.rules(ui, press, b);
            return (false, None);
        }
        #[cfg(any(target_os = "android", target_os = "ios"))]
        if self.editing.is_some() {
            self.keyboard(ui, press, b);
            return (false, None);
        }
        if self.confirm || self.reset || self.hands {
            self.modal(ui, b);
            return (false, None);
        }
        let mut pulse = None;
        match self.game.phase() {
            Phase::Setup => self.setup(ui, press, b),
            Phase::Handoff { player, .. } => self.handoff(ui, press, player, b),
            Phase::Hand { player, .. } => self.hand(ui, press, player, b),
            Phase::Arrange | Phase::Reveal | Phase::Result { .. } => {
                pulse = self.table(ui, press, keys, b);
            }
        }
        if !self.error.is_empty() {
            fit(
                ui,
                &self.error,
                Rect::new(x, 56., w, 16.),
                11.,
                crate::ui::CORAL,
                true,
            );
        }
        if self.save_failed {
            fit(
                ui,
                "Progress could not be saved",
                Rect::new(x, b.bottom() - 72., w, 18.),
                11.,
                crate::ui::CORAL,
                false,
            );
        }
        (false, pulse)
    }
    fn setup(&mut self, ui: &mut Ui, press: Option<Vec2>, b: Rect) {
        let w = b.w.min(580.);
        let x = b.center().x - w / 2.;
        let vp = Rect::new(x, b.y, w, b.h - 66.);
        let (o, tap) = page_input(&mut self.pan, &mut self.blocked, press, vp, 440.);
        let old = ui.override_pointer(tap);
        clip(Some(vp));
        ui.centered(
            "Small clues. Shared intuition.",
            Rect::new(x, o.y, w, 25.),
            15.,
            ui.theme.muted,
            false,
        );
        emblem(
            vec2(x + w / 2., o.y + 79.),
            41.,
            ui.theme.accent,
            ui.theme.saver,
        );
        let cat = Rect::new(x, o.y + 138., w, 112.);
        category(ui, self.game.category(), cat);
        let shuffle = Rect::new(cat.right() - 42., cat.y + 5., 36., 36.);
        draw_shuffle(shuffle.center(), ui.theme.accent);
        if ui.hit(shuffle) {
            self.game.shuffle_category();
            self.save();
        }
        let mut config = self.game.config();
        for (index, (label, value, min, max)) in [
            ("Players", config.players, 2, 10),
            ("Cards each", config.cards_each, 1, 3),
        ]
        .into_iter()
        .enumerate()
        {
            let y = o.y + 266. + index as f32 * 62.;
            fit(
                ui,
                label,
                Rect::new(x, y, w - 156., 44.),
                16.,
                ui.theme.text,
                true,
            );
            let minus = ui.button("−", Rect::new(x + w - 148., y, 44., 44.), false);
            let plus = ui.button("+", Rect::new(x + w - 44., y, 44., 44.), false);
            let changed = if minus && value > min {
                Some(value - 1)
            } else if plus && value < max {
                Some(value + 1)
            } else {
                None
            };
            ui.centered(
                &value.to_string(),
                Rect::new(x + w - 100., y, 48., 44.),
                20.,
                ui.theme.text,
                true,
            );
            if let Some(v) = changed {
                if index == 0 {
                    config.players = v;
                } else {
                    config.cards_each = v;
                }
            }
        }
        for (i, (label, lang)) in [
            ("English", Language::English),
            ("Português", Language::Portuguese),
        ]
        .into_iter()
        .enumerate()
        {
            if ui.tab(
                label,
                Rect::new(x + i as f32 * (w + 8.) / 2., o.y + 394., (w - 8.) / 2., 44.),
                config.language == lang,
            ) {
                config.language = lang;
            }
        }
        if self.game.configure(config) {
            self.save();
        }
        clip(None);
        ui.override_pointer(old);
        if primary(
            ui,
            "Start round",
            Rect::new(x, b.bottom() - 48., w, 48.),
            true,
        ) {
            self.game.start();
            self.change_phase();
        }
    }
    fn handoff(&mut self, ui: &mut Ui, press: Option<Vec2>, player: usize, b: Rect) {
        let landscape = b.w >= 480. && b.h < 450.;
        if landscape {
            let info = Rect::new(b.right() - 190., b.y, 190., b.h);
            category(
                ui,
                self.game.category(),
                Rect::new(info.x, info.y, info.w, 104.),
            );
            hidden_card(
                ui,
                vec2(b.x + (b.w - 206.) / 2., b.center().y),
                b.h.min(220.) * 0.3,
            );
            fit(
                ui,
                &format!("Pass to player {}", player + 1),
                Rect::new(info.x, info.y + 120., info.w, 28.),
                21.,
                ui.theme.text,
                true,
            );
            fit(
                ui,
                "Only this player should look",
                Rect::new(info.x, info.y + 156., info.w, 22.),
                12.,
                ui.theme.muted,
                false,
            );
            if primary(
                ui,
                "Reveal my hand",
                Rect::new(info.x, b.bottom() - 48., info.w, 48.),
                true,
            ) {
                self.game.show_hand();
                self.change_phase();
            }
        } else {
            let vp = Rect::new(b.x, b.y, b.w, b.h - 64.);
            let (o, _) = page_input(&mut self.pan, &mut self.blocked, press, vp, 380.);
            clip(Some(vp));
            category(ui, self.game.category(), Rect::new(b.x, o.y, b.w, 104.));
            hidden_card(ui, vec2(b.center().x, o.y + 204.), 48.);
            fit(
                ui,
                &format!("Pass to player {}", player + 1),
                Rect::new(b.x, o.y + 280., b.w, 32.),
                24.,
                ui.theme.text,
                true,
            );
            fit(
                ui,
                "Only this player should look",
                Rect::new(b.x, o.y + 322., b.w, 24.),
                13.,
                ui.theme.muted,
                false,
            );
            clip(None);
            if primary(
                ui,
                "Reveal my hand",
                Rect::new(b.x, b.bottom() - 48., b.w, 48.),
                true,
            ) {
                self.game.show_hand();
                self.change_phase();
            }
        }
    }
    fn hand(&mut self, ui: &mut Ui, press: Option<Vec2>, player: usize, b: Rect) {
        let landscape = b.w >= 480. && b.h < 450.;
        let small = !landscape && b.h < 390.;
        let mut work = if landscape {
            Rect::new(b.x, b.y, b.w - 200., b.h)
        } else {
            b
        };
        let old_pointer = if small {
            let vp = Rect::new(b.x, b.y, b.w, b.h - 66.);
            let (o, tap) = page_input(&mut self.pan, &mut self.blocked, press, vp, 390.);
            work.y = o.y;
            clip(Some(vp));
            Some(ui.override_pointer(tap))
        } else {
            None
        };
        category(
            ui,
            self.game.category(),
            Rect::new(work.x, work.y, work.w, 104.),
        );
        let n = usize::from(self.game.config().cards_each);
        let first = player * n;
        let selected = self
            .selected
            .filter(|&id| self.game.owner(id) == Some(player))
            .unwrap_or(first);
        self.selected = Some(selected);
        let cards_y = work.y + 122.;
        let gap = 10.;
        let cw = (work.w - gap * (n - 1) as f32) / n as f32;
        let ch = if landscape {
            110.
        } else {
            (work.h - 300.).clamp(90., 180.)
        };
        for i in 0..n {
            let id = first + i;
            let c = self.game.card(id).unwrap();
            let r = Rect::new(work.x + i as f32 * (cw + gap), cards_y, cw, ch);
            bordered(
                r,
                18.,
                if selected == id {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                ui.theme.panel,
            );
            ui.centered(
                &c.letter.to_string(),
                Rect::new(r.x, r.y + 10., r.w, 20.),
                12.,
                ui.theme.muted,
                true,
            );
            ui.centered(
                &c.number.unwrap().to_string(),
                Rect::new(r.x, r.y + 28., r.w, r.h - 48.),
                if cw < 100. { 42. } else { 58. },
                ui.theme.accent,
                true,
            );
            if !c.clue.is_empty() {
                draw_circle(r.right() - 12., r.y + 14., 3., ui.theme.accent);
            }
            if ui.hit(r) && selected != id {
                self.close_editor();
                self.selected = Some(id);
                self.revision += 1;
            }
        }
        let selected = self.selected.unwrap();
        let value = self.game.card(selected).unwrap();
        let field = if landscape {
            Rect::new(b.right() - 184., b.y + 40., 184., 100.)
        } else if small {
            Rect::new(work.x, work.y + 300., work.w, 80.)
        } else {
            Rect::new(b.x, b.bottom() - 146., b.w, 80.)
        };
        fit(
            ui,
            &format!("Clue for {}", value.letter),
            Rect::new(field.x, field.y - 30., field.w, 24.),
            13.,
            ui.theme.muted,
            true,
        );
        bordered(
            field,
            16.,
            if self.editing == Some(selected) {
                ui.theme.accent
            } else {
                ui.theme.line
            },
            ui.theme.panel,
        );
        let text = if self.editing == Some(selected) {
            &self.field
        } else {
            value.clue
        };
        wrap(
            ui,
            if text.is_empty() {
                "Give an example…"
            } else {
                text
            },
            Rect::new(field.x + 14., field.y + 12., field.w - 28., field.h - 20.),
            16.,
            if text.is_empty() {
                ui.theme.muted
            } else {
                ui.theme.text
            },
            false,
        );
        if ui.hit(field) {
            self.field = value.clue.into();
            self.editing = Some(selected);
            platform::editor_open(&self.field, 8, field, MAX_CLUE);
            #[cfg(any(target_os = "android", target_os = "ios"))]
            {
                self.keyboard_pan = BoardPan::default();
            }
        }
        if self.editing.is_some() {
            platform::editor_position(8, field);
        }
        if let Some(old) = old_pointer {
            clip(None);
            ui.override_pointer(old);
        }
        let footer = Rect::new(field.x, b.bottom() - 48., field.w, 48.);
        if primary(ui, "Hide & pass", footer, self.game.hand_ready()) {
            self.game.pass();
            self.change_phase();
        }
    }
    fn table(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, KeyMods, bool)],
        b: Rect,
    ) -> Option<Pulse> {
        let phase = self.game.phase();
        let landscape = b.w >= 480. && b.h < 450.;
        let info = if landscape {
            Rect::new(b.right() - 190., b.y, 190., b.h)
        } else {
            b
        };
        category(
            ui,
            self.game.category(),
            Rect::new(info.x, info.y, info.w, if landscape { 96. } else { 104. }),
        );
        let title = match phase {
            Phase::Arrange => "Find the order",
            Phase::Reveal => "Follow the thread",
            Phase::Result { won: true } => "In perfect order!",
            _ => "A little tangled",
        };
        fit(
            ui,
            title,
            Rect::new(
                info.x,
                info.y + if landscape { 104. } else { 114. },
                info.w,
                28.,
            ),
            20.,
            ui.theme.text,
            true,
        );
        let vp = if landscape {
            Rect::new(b.x, b.y, b.w - 206., b.h - 4.)
        } else {
            Rect::new(b.x, b.y + 158., b.w, (b.h - 222.).max(40.))
        };
        let rh = 78.;
        let content = self.game.order().len() as f32 * rh;
        // Grip drags reorder; swiping elsewhere pans. A cancelled drag commits nothing.
        let original = self.game.order().to_vec();
        let current = self
            .drag
            .as_ref()
            .map_or(original.clone(), |d| preview_order(&d.order, d.id, d.to));
        let (o, tap) = if self.drag.is_some() {
            self.drag_input(vp, rh);
            (
                self.pan.board(vp, vec2(vp.w, content.max(vp.h))).point(),
                None,
            )
        } else {
            let mut grab = false;
            if phase == Phase::Arrange
                && !self.blocked
                && let Some(p) = press
                && vp.contains(p)
            {
                let y = self.pan.board(vp, vec2(vp.w, content.max(vp.h))).y;
                let idx = ((p.y - y) / rh).floor() as usize;
                if p.x >= vp.right() - 44.
                    && let Some(&id) = original.get(idx)
                {
                    let live = touches()
                        .into_iter()
                        .find(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled));
                    self.drag = Some(Drag {
                        id,
                        owner: live.map(|t| t.id),
                        order: original.clone(),
                        to: idx,
                    });
                    self.selected = Some(id);
                    self.pan.cancel();
                    grab = true;
                }
            }
            if grab {
                (
                    self.pan.board(vp, vec2(vp.w, content.max(vp.h))).point(),
                    None,
                )
            } else {
                page_input(&mut self.pan, &mut self.blocked, press, vp, content)
            }
        };
        let old = ui.override_pointer(tap);
        clip(Some(vp));
        for (i, &id) in current.iter().enumerate() {
            let c = self.game.card(id).unwrap();
            let r = Rect::new(vp.x + 28., o.y + i as f32 * rh + 3., vp.w - 28., rh - 10.);
            let bad = if let (Some(n), Some(prev)) = (
                c.number,
                i.checked_sub(1)
                    .and_then(|p| self.game.card(current[p]))
                    .and_then(|c| c.number),
            ) {
                n < prev
            } else {
                false
            };
            let selected = self.selected == Some(id) && phase == Phase::Arrange;
            draw_line(
                vp.x + 12.,
                r.y - 3.,
                vp.x + 12.,
                r.bottom() + 7.,
                1.,
                ui.theme.line,
            );
            draw_circle(
                vp.x + 12.,
                r.center().y,
                4.,
                if bad {
                    crate::ui::CORAL
                } else {
                    ui.theme.accent
                },
            );
            bordered(
                r,
                14.,
                if selected {
                    ui.theme.accent
                } else if bad {
                    crate::ui::CORAL
                } else {
                    ui.theme.line
                },
                ui.theme.bg,
            );
            fit(
                ui,
                &format!("PLAYER {} · {}", c.owner + 1, c.letter),
                Rect::new(r.x + 12., r.y + 8., r.w - 64., 16.),
                10.,
                ui.theme.muted,
                true,
            );
            let label_w = r.w - if phase == Phase::Arrange { 52. } else { 76. };
            fit(
                ui,
                c.clue,
                Rect::new(r.x + 12., r.y + 26., label_w - 12., 32.),
                15.,
                ui.theme.text,
                true,
            );
            if let Some(n) = c.number {
                ui.centered(
                    &n.to_string(),
                    Rect::new(r.right() - 58., r.y, 50., r.h),
                    28.,
                    if bad {
                        crate::ui::CORAL
                    } else {
                        ui.theme.accent
                    },
                    true,
                );
            } else if phase == Phase::Arrange {
                for dx in [-3., 3.] {
                    for dy in [-6., 0., 6.] {
                        draw_circle(r.right() - 22. + dx, r.center().y + dy, 1.5, ui.theme.muted);
                    }
                }
            } else {
                fit(
                    ui,
                    "?",
                    Rect::new(r.right() - 58., r.y, 50., r.h),
                    25.,
                    ui.theme.muted,
                    true,
                );
            }
            if ui.hit(r) && phase == Phase::Arrange {
                self.selected = if selected { None } else { Some(id) };
                self.revision += 1;
            }
        }
        clip(None);
        ui.override_pointer(old);
        for &(key, mods, repeat) in keys {
            if phase == Phase::Arrange
                && mods.ctrl
                && !repeat
                && let Some(id) = self.selected
            {
                let from = self.game.order().iter().position(|&i| i == id).unwrap();
                let to = match key {
                    KeyCode::Up => from.saturating_sub(1),
                    KeyCode::Down => (from + 1).min(self.game.order().len() - 1),
                    _ => from,
                };
                if self.game.move_card(id, to) {
                    self.save();
                }
            }
        }
        let footer = if landscape {
            Rect::new(info.x, b.bottom() - 48., info.w, 48.)
        } else {
            Rect::new(b.x, b.bottom() - 48., b.w, 48.)
        };
        let mut pulse = None;
        if phase == Phase::Arrange {
            let small = if landscape { 44. } else { 48. };
            for i in 0..2 {
                let r = if landscape {
                    Rect::new(info.x + i as f32 * 52., footer.y - 54., small, 44.)
                } else {
                    Rect::new(footer.x + i as f32 * 52., footer.y, small, 48.)
                };
                if ui.icon_button(Icon::Arrow((0, if i == 0 { -1 } else { 1 })), r, false)
                    && let Some(id) = self.selected
                {
                    let from = self.game.order().iter().position(|&v| v == id).unwrap();
                    let to = if i == 0 {
                        from.saturating_sub(1)
                    } else {
                        (from + 1).min(self.game.order().len() - 1)
                    };
                    if self.game.move_card(id, to) {
                        self.save();
                    }
                }
            }
            let peek = if landscape {
                Rect::new(info.right() - 44., footer.y - 54., 44., 44.)
            } else {
                Rect::new(footer.x + 104., footer.y, 44., 48.)
            };
            eye(peek.center(), ui.theme.text);
            if ui.hit(peek) {
                self.hands = true;
                self.revision += 1;
            }
            let r = if landscape {
                footer
            } else {
                Rect::new(footer.x + 156., footer.y, footer.w - 156., 48.)
            };
            if primary(ui, "Reveal", r, true) {
                self.confirm = true;
                self.revision += 1;
            }
        } else if phase == Phase::Reveal {
            if primary(ui, "Reveal next", footer, true) {
                self.game.reveal_next();
                self.save();
                let pos = self.game.revealed().saturating_sub(1);
                self.pan.offset.y = (pos as f32 * rh - vp.h + rh).max(0.);
                if let Phase::Result { won } = self.game.phase() {
                    pulse = Some(if won { Pulse::Won } else { Pulse::Lost });
                }
            }
        } else if primary(ui, "New round", footer, true) {
            self.game.new_round();
            self.change_phase();
        }
        if phase == Phase::Arrange && ui.keyboard_focus && is_key_pressed(KeyCode::R) {
            self.reset = true;
            self.revision += 1;
        }
        pulse
    }
    fn drag_input(&mut self, vp: Rect, rh: f32) {
        let events = touches();
        let live = events
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .count();
        if live > 1 || events.iter().any(|t| t.phase == TouchPhase::Cancelled) {
            self.drag = None;
            self.blocked = true;
            return;
        }
        let Some(d) = &mut self.drag else {
            return;
        };
        let (p, end) = if let Some(id) = d.owner {
            match events.iter().find(|t| t.id == id) {
                Some(t) => (
                    touch_point(t.position, screen_dpi_scale()),
                    t.phase == TouchPhase::Ended,
                ),
                None => {
                    self.drag = None;
                    return;
                }
            }
        } else {
            let (x, y) = mouse_position();
            (vec2(x, y), !is_mouse_button_down(MouseButton::Left))
        };
        let content = vec2(vp.w, d.order.len() as f32 * rh);
        let speed = if p.y < vp.y + 28. {
            -220.
        } else if p.y > vp.bottom() - 28. {
            220.
        } else {
            0.
        };
        if speed != 0. && !end {
            self.pan
                .scroll(vec2(0., speed * get_frame_time().min(0.05)), vp, content);
        }
        let y = self.pan.board(vp, content).y;
        d.to = (((p.y - y) / rh).floor() as isize).clamp(0, d.order.len() as isize - 1) as usize;
        if end {
            let d = self.drag.take().unwrap();
            if self.game.move_card(d.id, d.to) {
                self.save();
            }
            self.blocked = live > 0;
        }
    }
    fn modal(&mut self, ui: &mut Ui, b: Rect) {
        if self.hands {
            let cols = if b.w >= 480. { 4 } else { 2 };
            let rows = usize::from(self.game.config().players).div_ceil(cols);
            let h = 80. + rows as f32 * 52.;
            let w = b.w.min(500.);
            let r = Rect::new(b.center().x - w / 2., b.center().y - h / 2., w, h);
            bordered(r, 18., ui.theme.line, ui.theme.bg);
            fit(
                ui,
                "Whose hand?",
                Rect::new(r.x, r.y + 10., r.w, 26.),
                20.,
                ui.theme.text,
                true,
            );
            for i in 0..usize::from(self.game.config().players) {
                let cell = Rect::new(
                    r.x + 10. + (i % cols) as f32 * (w - 20.) / cols as f32,
                    r.y + 48. + (i / cols) as f32 * 52.,
                    (w - 20.) / cols as f32 - 6.,
                    44.,
                );
                if ui.button(&format!("Player {}", i + 1), cell, false) {
                    self.hands = false;
                    self.game.review_hand(i);
                    self.change_phase();
                }
            }
            return;
        }
        let r = Rect::new(
            b.center().x - b.w.min(420.) / 2.,
            b.center().y - 85.,
            b.w.min(420.),
            170.,
        );
        bordered(r, 20., ui.theme.line, ui.theme.bg);
        fit(
            ui,
            if self.reset {
                "Start over?"
            } else {
                "Everyone agrees?"
            },
            Rect::new(r.x + 12., r.y + 16., r.w - 24., 32.),
            22.,
            ui.theme.text,
            true,
        );
        fit(
            ui,
            if self.reset {
                "Your current thread will be cleared."
            } else {
                "The order locks when you reveal."
            },
            Rect::new(r.x + 12., r.y + 58., r.w - 24., 24.),
            13.,
            ui.theme.muted,
            false,
        );
        if ui.button(
            "Keep playing",
            Rect::new(r.x + 12., r.bottom() - 60., (r.w - 36.) / 2., 48.),
            false,
        ) {
            self.confirm = false;
            self.reset = false;
            self.revision += 1;
        }
        if primary(
            ui,
            if self.reset { "Start over" } else { "Reveal" },
            Rect::new(r.center().x + 6., r.bottom() - 60., (r.w - 36.) / 2., 48.),
            true,
        ) {
            if self.reset {
                self.game.abandon();
            } else {
                self.game.begin_reveal();
            }
            self.reset = false;
            self.confirm = false;
            self.change_phase();
        }
    }
    fn rules(&mut self, ui: &mut Ui, press: Option<Vec2>, b: Rect) {
        let rules = [
            "Give examples that convey a secret number from 1 to 100. The category shows what low and high mean. Never say your number, use numerical clues, or show your hand to another player.",
            "Pass the device privately. Each player reveals their own hand, writes one clue per card, then hides it before passing. For two players, try two or three cards each.",
            "Once all clues are written, discuss their relative meaning and arrange the thread from lowest to highest. Drag a card by its dotted grip. Swipe elsewhere to scroll. Tap a card and use the arrow buttons, or Ctrl+Up/Down, to move it.",
            "The eye button opens a private hand review: choose your player, pass the device and reveal only when alone. You can change your clues and return to the shared board.",
            "When everyone agrees, reveal and lock the order. Reveal numbers one at a time. An inversion loses the round; if all numbers rise, everyone wins. New rounds deal fresh numbers and another category.",
            "Back, app interruptions and reloads hide private hands and preserve the round. This game is entirely offline and uses one shared device.",
        ];
        let vp = Rect::new(b.x, b.y, b.w, b.h - 58.);
        let heights: Vec<_> = rules
            .iter()
            .map(|s| {
                crate::online_view::wrapped_lines(ui, s, b.w - 24., 15.).len() as f32 * 24. + 26.
            })
            .collect();
        let (o, _) = page_input(
            &mut self.pan,
            &mut self.blocked,
            press,
            vp,
            heights.iter().sum(),
        );
        clip(Some(vp));
        let mut y = o.y;
        for (s, h) in rules.iter().zip(heights) {
            wrap(
                ui,
                s,
                Rect::new(b.x + 12., y, b.w - 24., h),
                15.,
                ui.theme.text,
                false,
            );
            y += h;
        }
        clip(None);
        if ui.button("Close", Rect::new(b.x, b.bottom() - 48., b.w, 48.), false) {
            self.help = false;
            self.pan = BoardPan::default();
            self.revision += 1;
        }
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn keyboard(&mut self, ui: &mut Ui, press: Option<Vec2>, b: Rect) {
        let chars: Vec<_> = "ABCDEFGHIJKLMNOPQRSTUVWXYZÁÃÂÀÉÊÍÓÔÕÚÇ".chars().collect();
        let cols = (b.w / 44.).floor().max(4.) as usize;
        let vp = Rect::new(b.x, b.y, b.w, b.h - 56.);
        let total = 78. + chars.len().div_ceil(cols) as f32 * 50. + 110.;
        let (o, tap) = page_input(&mut self.keyboard_pan, &mut self.blocked, press, vp, total);
        let old = ui.override_pointer(tap);
        clip(Some(vp));
        fit(
            ui,
            &self.field,
            Rect::new(b.x, o.y, b.w, 56.),
            18.,
            ui.theme.text,
            false,
        );
        let w = (b.w - 6. * (cols - 1) as f32) / cols as f32;
        for (i, c) in chars.into_iter().enumerate() {
            let r = Rect::new(
                b.x + (i % cols) as f32 * (w + 6.),
                o.y + 70. + (i / cols) as f32 * 50.,
                w,
                44.,
            );
            if ui.button(&c.to_string(), r, false) && self.field.chars().count() < MAX_CLUE {
                self.field.push(c);
                self.commit_field();
                self.revision += 1;
            }
        }
        let y = o.y + 70. + 38usize.div_ceil(cols) as f32 * 50.;
        for (i, label) in ["Space", "⌫"].into_iter().enumerate() {
            if ui.button(
                label,
                Rect::new(b.x + i as f32 * (b.w + 8.) / 2., y, (b.w - 8.) / 2., 44.),
                false,
            ) {
                if i == 0 && self.field.chars().count() < MAX_CLUE {
                    self.field.push(' ');
                } else if i == 1 {
                    self.field.pop();
                }
                self.commit_field();
                self.revision += 1;
            }
        }
        clip(None);
        ui.override_pointer(old);
        if ui.button("Done", Rect::new(b.x, b.bottom() - 48., b.w, 48.), true) {
            self.close_editor();
            self.revision += 1;
        }
    }
}
fn preview_order(order: &[usize], id: usize, to: usize) -> Vec<usize> {
    let mut o = order.to_vec();
    if let Some(from) = o.iter().position(|&v| v == id) {
        let card = o.remove(from);
        o.insert(to.min(o.len()), card);
    }
    o
}
fn category(ui: &Ui, (title, low, high): (&str, &str, &str), r: Rect) {
    bordered(r, 18., ui.theme.line, ui.theme.panel);
    let title_box = Rect::new(r.x + 16., r.y + 14., r.w - 56., r.h - 44.);
    let mut size = if r.w < 240. { 15. } else { 19. };
    while size > 10. {
        let lines = crate::online_view::wrapped_lines(ui, title, title_box.w, size).len();
        if size + lines.saturating_sub(1) as f32 * (size + 9.) <= title_box.h {
            break;
        }
        size -= 1.;
    }
    wrap(ui, title, title_box, size, ui.theme.text, true);
    let half = (r.w - 32.) / 2.;
    fit(
        ui,
        low,
        Rect::new(r.x + 12., r.bottom() - 29., half, 20.),
        11.,
        ui.theme.muted,
        false,
    );
    fit(
        ui,
        high,
        Rect::new(r.center().x + 4., r.bottom() - 29., half, 20.),
        11.,
        ui.theme.accent,
        true,
    );
}
fn eye(c: Vec2, ink: Color) {
    draw_ellipse_lines(c.x, c.y, 10., 6., 0., 1.5, ink);
    draw_circle(c.x, c.y, 3., ink);
}
fn draw_shuffle(c: Vec2, ink: Color) {
    for sign in [-1., 1.] {
        draw_line(
            c.x - 8.,
            c.y - sign * 5.,
            c.x + 8.,
            c.y + sign * 5.,
            1.5,
            ink,
        );
        draw_line(
            c.x + 8.,
            c.y + sign * 5.,
            c.x + 3.,
            c.y + sign * 5.,
            1.5,
            ink,
        );
        draw_line(c.x + 8., c.y + sign * 5., c.x + 8., c.y, 1.5, ink);
    }
}
fn hidden_card(ui: &Ui, c: Vec2, size: f32) {
    let r = Rect::new(c.x - size * 0.7, c.y - size, size * 1.4, size * 2.);
    bordered(r, 18., ui.theme.accent, ui.theme.bg);
    eye(c, ui.theme.accent);
}
fn emblem(c: Vec2, size: f32, ink: Color, saver: bool) {
    for i in 0..3 {
        let p = c + vec2(
            (i as f32 - 1.) * size * 0.8,
            if i == 1 { -size * 0.25 } else { size * 0.2 },
        );
        let r = Rect::new(
            p.x - size * 0.38,
            p.y - size * 0.56,
            size * 0.76,
            size * 1.12,
        );
        bordered(
            r,
            size * 0.14,
            ink,
            if saver {
                BLACK
            } else {
                color_u8!(248, 246, 253, 255)
            },
        );
        draw_circle(p.x, p.y, size * 0.08, ink);
    }
    for i in 0..40 {
        let x1 = c.x - size * 1.2 + i as f32 * size * 2.4 / 40.;
        let x2 = x1 + size * 2.4 / 40.;
        let y = |x: f32| c.y + size * 0.65 + ((x - c.x) / size * 2.5).sin() * size * 0.15;
        draw_line(x1, y(x1), x2, y(x2), 2., ink);
    }
}
pub fn preview(ui: &Ui, r: Rect) {
    let ink = if ui.theme.saver {
        color_u8!(185, 170, 250, 255)
    } else {
        INK
    };
    let inset = (r.w * 0.08).clamp(7., 18.);
    fit(
        ui,
        "Things to carry uphill",
        Rect::new(r.x + inset, r.y + 3., r.w - 2. * inset, r.h * 0.2),
        (r.w * 0.055).clamp(8., 14.),
        ui.theme.text,
        true,
    );
    let row = r.h * 0.23;
    let x = r.x + inset;
    let y = r.y + r.h * 0.26;
    draw_line(x + 3., y + row * 0.4, x + 3., y + row * 2.4, 1., ink);
    for (i, label) in ["Feather", "Cat", "Elephant"].into_iter().enumerate() {
        let boxr = Rect::new(
            x + 12.,
            y + i as f32 * row,
            r.w - 2. * inset - 12.,
            row * 0.8,
        );
        draw_circle(x + 3., boxr.center().y, 2., ink);
        bordered(boxr, 5., ui.theme.line, ui.theme.bg);
        ui.centered(
            label,
            boxr,
            (row * 0.42).clamp(7., 12.),
            ui.theme.text,
            false,
        );
        for dy in [-2., 2.] {
            draw_circle(boxr.right() - 6., boxr.center().y + dy, 0.8, ui.theme.muted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grip_previews_preserve_all_cards_and_do_not_change_the_committed_order() {
        let o = vec![0, 1, 2, 3];
        assert_eq!(preview_order(&o, 0, 3), vec![1, 2, 3, 0]);
        assert_eq!(preview_order(&o, 3, 0), vec![3, 0, 1, 2]);
        assert_eq!(preview_order(&o, 99, 0), o);
        assert_eq!(o, vec![0, 1, 2, 3]);
    }
}
