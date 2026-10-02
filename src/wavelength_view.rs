//! A shared, vector-drawn pass-and-play dial. Nothing here uses the room server.
use crate::{
    online_style, platform,
    ui::{Icon, Ui, bordered, rounded},
};
use jarcade::{
    feedback::Pulse,
    layout::touch_point,
    wavelength::{Deck, Game, Phase, Proximity},
};
use macroquad::miniquad::KeyMods;
use macroquad::prelude::*;

const CORAL: Color = color_u8!(226, 100, 78, 255);
const TEAL: Color = color_u8!(66, 150, 139, 255);
const CREAM: Color = color_u8!(251, 247, 236, 255);

pub struct WavelengthPage {
    game: Game,
    pub revision: u64,
    drag: Option<Option<u64>>,
    blocked: bool,
    size: Vec2,
    help: bool,
    decks: bool,
    custom: bool,
    fields: [String; 2],
    editing: Option<usize>,
    save_failed: bool,
}
impl WavelengthPage {
    pub fn new(seed: u64) -> Self {
        let game = platform::load_wavelength(seed);
        Self {
            fields: game.custom().clone(),
            game,
            revision: 0,
            drag: None,
            blocked: false,
            size: Vec2::ZERO,
            help: false,
            decks: false,
            custom: false,
            editing: None,
            save_failed: false,
        }
    }
    fn save(&mut self) {
        self.revision += 1;
        self.save_failed = !platform::save_wavelength(&self.game);
    }
    pub fn interrupt(&mut self) {
        self.game.conceal();
        self.drag = None;
        self.blocked = false;
        self.close_editor();
        self.save();
    }
    fn close_editor(&mut self) {
        if self.editing.take().is_some() {
            platform::editor_close();
        }
    }
    pub fn back(&mut self) -> bool {
        self.close_editor();
        self.revision += 1;
        if self.custom {
            self.custom = false;
            return false;
        }
        if self.help || self.decks {
            self.help = false;
            self.decks = false;
            return false;
        }
        self.interrupt();
        true
    }
    pub fn enter(&mut self) {
        self.help = false;
        self.decks = false;
        self.custom = false;
        self.drag = None;
        self.revision += 1;
    }
    pub fn needs_frame(&self) -> bool {
        self.drag.is_some()
    }
    pub fn poll(&mut self) {
        while let Some(edit) = platform::editor_poll() {
            if (3..5).contains(&edit.id) {
                self.fields[edit.id - 3] = edit
                    .text
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(48)
                    .collect();
                if edit.done {
                    self.editing = None;
                }
                self.revision += 1;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(id) = self.editing {
            while let Some(c) = get_char_pressed() {
                if !c.is_control() && self.fields[id].chars().count() < 48 {
                    self.fields[id].push(c);
                    self.revision += 1;
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.fields[id].pop();
                self.revision += 1;
            }
            if is_key_pressed(KeyCode::Enter) {
                self.close_editor();
                clear_input_queue();
            }
        }
    }
    pub fn announcement(&self) -> String {
        let v = self.game.view();
        let mut text = format!(
            "Jarcade. Wavelength. Local multiplayer. Round {}. {:?}. {} to {}. ",
            v.round, v.phase, v.left, v.right
        );
        match v.phase {
            Phase::Ready => text.push_str("Only the clue giver should look. Reveal target."),
            Phase::Peek => text.push_str(&format!(
                "Private target {} percent. Give a clue out loud, then hide and pass.",
                (v.target.unwrap() * 100.).round()
            )),
            Phase::Handoff => text.push_str("Target hidden. Pass the device. Ready to guess."),
            Phase::Guess => text.push_str(&format!(
                "Guess {} percent. Drag the needle or use Left and Right. Confirm guess.",
                (v.guess * 100.).round()
            )),
            Phase::Result => text.push_str(&format!(
                "{}. Target {} percent; guess {} percent. Next round.",
                v.result.unwrap().title(),
                (v.target.unwrap() * 100.).round(),
                (v.guess * 100.).round()
            )),
        }
        if self.help {
            text.push_str(" How to play is open.");
        }
        if self.decks {
            text.push_str(" Prompt decks are open.");
        }
        if self.custom {
            text.push_str(" Custom spectrum editor is open.");
        }
        text
    }
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, KeyMods, bool)],
    ) -> (bool, Option<Pulse>) {
        ui.theme.accent = if ui.theme.saver {
            color_u8!(249, 175, 151, 255)
        } else {
            CORAL
        };
        ui.theme.panel = if ui.theme.saver { BLACK } else { CREAM };
        let size = vec2(screen_width(), screen_height());
        if self.size != size {
            if self.drag.take().is_some() {
                self.save();
            }
            self.size = size;
        }
        let live = touches()
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .count();
        if live > 1 || touches().iter().any(|t| t.phase == TouchPhase::Cancelled) {
            self.blocked = true;
            if self.drag.take().is_some() {
                self.save();
            }
        }
        let original = if self.blocked || self.drag.is_some() {
            Some(ui.override_pointer(None))
        } else {
            None
        };
        let w = (screen_width() - 32.).min(1060.);
        let x = (screen_width() - w) / 2.;
        if ui.icon_button(Icon::Back, Rect::new(x, 8., 44., 44.), false) && self.back() {
            return (true, None);
        }
        fit(
            ui,
            "Wavelength",
            Rect::new(x + 50., 8., w - 150., 44.),
            21.,
            ui.theme.text,
            true,
        );
        let deck_available = matches!(
            self.game.view().phase,
            Phase::Ready | Phase::Peek | Phase::Result
        );
        if deck_button(ui, Rect::new(x + w - 94., 8., 44., 44.), deck_available) {
            self.game.conceal();
            self.save();
            self.decks = !self.decks;
            self.help = false;
            self.custom = false;
            self.close_editor();
        }
        if ui.button("?", Rect::new(x + w - 44., 8., 44., 44.), false) {
            self.game.conceal();
            self.save();
            self.help = !self.help;
            self.decks = false;
            self.custom = false;
            self.close_editor();
        }
        let mut pulse = None;
        if self.help {
            self.draw_help(ui, x, w);
        } else if self.decks {
            self.draw_decks(ui, x, w);
        } else {
            pulse = self.draw_game(ui, if self.blocked { None } else { press }, x, w, keys);
        }
        if let Some(original) = original {
            ui.override_pointer(original);
        }
        if self.blocked && live == 0 {
            self.blocked = false;
        }
        if ui.activated {
            self.revision += 1;
        }
        if self.save_failed {
            ui.centered(
                "Progress could not be saved",
                Rect::new(x, screen_height() - 23., w, 18.),
                10.,
                ui.theme.muted,
                false,
            );
        }
        (false, pulse)
    }
    fn draw_game(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        x: f32,
        w: f32,
        keys: &[(KeyCode, KeyMods, bool)],
    ) -> Option<Pulse> {
        let v = self.game.view();
        let phase = v.phase;
        let target = v.target;
        let left = v.left.to_owned();
        let right = v.right.to_owned();
        let result = v.result;
        let round = v.round;
        if phase == Phase::Handoff {
            let center = vec2(screen_width() / 2., (screen_height() * 0.4).max(100.));
            let phone = Rect::new(center.x - 27., center.y - 58., 54., 85.);
            bordered(phone, 12., ui.theme.accent, ui.theme.bg);
            wave_mark(center + vec2(0., -21.), 17., ui.theme.accent);
            draw_circle_lines(center.x, center.y + 14., 3., 1., ui.theme.accent);
            crate::ui::draw_icon(Icon::Arrow((1, 0)), center + vec2(52., -16.), TEAL);
            ui.centered(
                "Pass the device",
                Rect::new(x, center.y + 45., w, 36.),
                24.,
                ui.theme.text,
                true,
            );
            if screen_height() >= 400. {
                fit(
                    ui,
                    "The target is hidden. It’s their turn.",
                    Rect::new(x, center.y + 88., w, 26.),
                    14.,
                    ui.theme.muted,
                    false,
                );
            }
            if online_style::primary(
                ui,
                "Ready to guess",
                Rect::new(x, screen_height() - 68., w, 48.),
                true,
            ) || (!self.blocked
                && keys
                    .iter()
                    .any(|(key, _, repeat)| *key == KeyCode::Space && !repeat))
            {
                self.game.advance();
                self.save();
                ui.activated = true;
            }
            return None;
        }
        let wide = w >= 480. && screen_width() > screen_height() * 1.15;
        let dw = if wide { w * 0.65 } else { w.min(620.) };
        let dx = if wide { x } else { x + (w - dw) / 2. };
        let compact = screen_height() < 550.;
        let available = screen_height() - 66. - if wide && compact { 20. } else { 82. };
        let extra = if compact { 112. } else { 176. };
        let radius = (dw / 2. - 12.).min((available - extra).max(52.)).min(300.);
        let top = 66. + ((available - (radius + extra)) / 2.).max(0.);
        let role = match phase {
            Phase::Ready | Phase::Peek => "Clue giver",
            Phase::Guess => "Find their wavelength",
            _ => result.unwrap().title(),
        };
        let role_w = (ui.text_width(role, 13., true) + 52.).min(dw);
        rounded(
            Rect::new(dx + (dw - role_w) / 2., top, role_w, 30.),
            15.,
            ui.theme.panel,
        );
        wave_mark(
            vec2(dx + (dw - role_w) / 2. + 20., top + 15.),
            10.,
            ui.theme.accent,
        );
        fit(
            ui,
            role,
            Rect::new(dx + (dw - role_w) / 2. + 34., top, role_w - 42., 30.),
            13.,
            ui.theme.accent,
            true,
        );
        let endpoints_y = top + if compact { 32. } else { 42. };
        let endpoint_w = (dw - 60.) / 2.;
        endpoint(
            ui,
            &left,
            Rect::new(dx, endpoints_y, endpoint_w, 52.),
            CORAL,
        );
        endpoint(
            ui,
            &right,
            Rect::new(dx + dw - endpoint_w, endpoints_y, endpoint_w, 52.),
            TEAL,
        );
        let can_shuffle = matches!(phase, Phase::Ready | Phase::Result);
        let shuffle = Rect::new(dx + (dw - 44.) / 2., endpoints_y + 4., 44., 44.);
        if can_shuffle && shuffle_button(ui, shuffle) {
            self.game.shuffle();
            self.save();
        }
        let center = vec2(
            dx + dw / 2.,
            endpoints_y + if compact { 56. } else { 64. } + radius,
        );
        let dial = Rect::new(
            center.x - radius - 8.,
            center.y - radius - 8.,
            radius * 2. + 16.,
            radius + 38.,
        );

        let instruction = match phase {
            Phase::Ready => "Only the clue giver should look.",
            Phase::Peek => "Say a clue out loud. Then hide the target.",
            Phase::Guess => "Drag the needle to where the clue belongs.",
            _ => "Swap roles. Try another spectrum.",
        };
        let button_label = match phase {
            Phase::Ready => "Reveal target",
            Phase::Peek => "Hide & pass",
            Phase::Guess => "Confirm guess",
            _ => "Next round",
        };
        let bx = if wide { x + dw + 24. } else { dx };
        let bw = if wide { w - dw - 24. } else { dw };
        let by = if wide {
            (screen_height() / 2. + 40.).min(screen_height() - 68.)
        } else {
            screen_height() - 68.
        };
        let hint_y = if wide { by - 80. } else { center.y + 43. };
        if !compact || wide {
            wrap(
                ui,
                instruction,
                Rect::new(bx, hint_y, bw, 50.),
                14.,
                ui.theme.muted,
                false,
            );
        }
        if wide {
            wave_mark(vec2(bx + bw / 2., hint_y - 50.), 22., ui.theme.accent);
        }
        // A dial drag owns its pointer until release; crossing the button never confirms.
        let was_drag = self.drag.is_some();
        let mut pulse = None;
        if phase == Phase::Guess {
            if !self.blocked && self.drag.is_none() && press.is_some_and(|p| dial.contains(p)) {
                let id = touches()
                    .iter()
                    .find(|t| matches!(t.phase, TouchPhase::Started | TouchPhase::Ended))
                    .map(|t| t.id);
                self.drag = Some(id);
                if let Some(value) = position(press.unwrap(), center)
                    && self.game.set_guess(value)
                {
                    self.revision += 1;
                }
            }
            if let Some(id) = self.drag {
                let touch = id.and_then(|id| touches().into_iter().find(|t| t.id == id));
                let point = touch
                    .as_ref()
                    .map(|t| touch_point(t.position, screen_dpi_scale()))
                    .unwrap_or_else(|| {
                        let (x, y) = mouse_position();
                        vec2(x, y)
                    });
                if let Some(value) = position(point, center) {
                    let old = (self.game.view().guess * 30.).round();
                    if self.game.set_guess(value) {
                        self.revision += 1;
                        if old != (value * 30.).round() {
                            pulse = Some(Pulse::Tap);
                        }
                    }
                }
                if touch
                    .as_ref()
                    .is_some_and(|t| matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
                    || (id.is_none() && !is_mouse_button_down(MouseButton::Left))
                    || (id.is_some() && touch.is_none() && touches().is_empty())
                {
                    self.drag = None;
                    self.save();
                }
            }
            if self.drag.is_none() && !was_drag {
                let mut changed = false;
                for (key, mods, _) in keys {
                    let step = if mods.shift { 0.05 } else { 0.015 };
                    let delta = match key {
                        KeyCode::Left | KeyCode::A => -step,
                        KeyCode::Right | KeyCode::D => step,
                        _ => 0.,
                    };
                    if delta != 0. && self.game.set_guess(self.game.view().guess + delta) {
                        changed = true;
                    }
                }
                if changed {
                    self.save();
                    pulse = Some(Pulse::Tap);
                    ui.keyboard_focus = false;
                }
            }
        }
        draw_dial(
            ui,
            center,
            radius,
            target,
            if phase == Phase::Peek {
                None
            } else {
                Some(self.game.view().guess)
            },
        );
        let previous = if was_drag || self.drag.is_some() {
            Some(ui.override_pointer(None))
        } else {
            None
        };
        let shortcut = self.drag.is_none()
            && !was_drag
            && !self.blocked
            && keys.iter().any(|(key, _, repeat)| {
                !repeat
                    && (*key == KeyCode::Space || (!ui.keyboard_focus && *key == KeyCode::Enter))
            });
        if (online_style::primary(ui, button_label, Rect::new(bx, by, bw, 48.), true) || shortcut)
            && !was_drag
            && self.drag.is_none()
            && !self.blocked
        {
            self.game.advance();
            self.save();
            ui.activated = true;
            if phase == Phase::Guess {
                pulse = Some(if self.game.view().result == Some(Proximity::InTune) {
                    Pulse::Won
                } else {
                    Pulse::Tap
                });
            }
        }
        if let Some(previous) = previous {
            ui.override_pointer(previous);
        }
        if screen_height() >= 400. {
            fit(
                ui,
                &format!("Round {round} · same device"),
                Rect::new(dx, center.y + 26., dw, 16.),
                10.,
                ui.theme.muted,
                false,
            );
        }
        pulse
    }
    fn draw_help(&mut self, ui: &mut Ui, x: f32, w: f32) {
        let w = w.min(600.);
        let x = x + (screen_width().min(1092.) - 32. - w).max(0.) / 2.;
        ui.centered(
            "Get on the same wavelength",
            Rect::new(x, 75., w, 32.),
            20.,
            ui.theme.text,
            true,
        );
        let lines = [
            (
                "1",
                "Peek privately",
                "One person sees the target between two opposite ideas.",
            ),
            (
                "2",
                "Give a clue",
                "Say something that belongs at that spot. Hide the target, then pass the device.",
            ),
            (
                "3",
                "Make a guess",
                "Everyone else discusses the clue and drags the needle. Left/Right also work.",
            ),
            (
                "4",
                "Reveal together",
                "Confirm to see the colored bands. No scores, no timer. Swap roles and play again.",
            ),
        ];
        let compact = screen_height() < 500.;
        let columns = if compact && w >= 480. { 2 } else { 1 };
        let cell_w = (w - (columns - 1) as f32 * 20.) / columns as f32;
        let step = if compact {
            ((screen_height() - 68. - 108. - 28.) / 3.).clamp(32., 44.)
        } else {
            88.
        };
        for (i, (number, title, detail)) in lines.iter().enumerate() {
            let x = x + (i % columns) as f32 * (cell_w + 20.);
            let y = 108. + (i / columns) as f32 * step;
            rounded(Rect::new(x, y, 28., 28.), 14., ui.theme.panel);
            ui.centered(
                number,
                Rect::new(x, y, 28., 28.),
                13.,
                ui.theme.accent,
                true,
            );
            fit(
                ui,
                title,
                Rect::new(x + 40., y, cell_w - 40., 28.),
                15.,
                ui.theme.text,
                true,
            );
            if !compact {
                wrap(
                    ui,
                    detail,
                    Rect::new(x + 40., y + 29., cell_w - 40., 54.),
                    14.,
                    ui.theme.muted,
                    false,
                );
            }
        }
        if ui.button("Got it", Rect::new(x, screen_height() - 68., w, 48.), false) {
            self.help = false;
        }
    }
    fn draw_decks(&mut self, ui: &mut Ui, x: f32, w: f32) {
        let width = w.min(600.);
        let x = x + (w - width) / 2.;
        if self.custom {
            ui.centered(
                "Your spectrum",
                Rect::new(x, 70., width, 34.),
                24.,
                ui.theme.text,
                true,
            );
            let fy = if screen_height() < 500. { 116. } else { 160. };
            for id in 0..2 {
                let r = Rect::new(x, fy + id as f32 * 78., width, 54.);
                self.field(ui, id, r);
            }
            #[cfg(any(target_os = "android", target_os = "ios"))]
            if self.editing.is_some() {
                self.draw_keyboard(ui, x, width);
                return;
            }
            let can = self.fields.iter().all(|s| !s.trim().is_empty());
            if online_style::primary(
                ui,
                "Use this spectrum",
                Rect::new(x, screen_height() - 68., width, 48.),
                can,
            ) && self.game.set_custom(&self.fields[0], &self.fields[1])
            {
                self.close_editor();
                self.decks = false;
                self.custom = false;
                self.save();
            }
            return;
        }
        ui.centered(
            "Pick a spectrum deck",
            Rect::new(x, 72., width, 34.),
            22.,
            ui.theme.text,
            true,
        );
        let compact = screen_height() < 500.;
        let gap = if compact { 58. } else { 74. };
        let columns = if compact && width >= 480. { 2 } else { 1 };
        let cell_w = (width - (columns - 1) as f32 * 12.) / columns as f32;
        for (i, deck) in Deck::ALL.iter().enumerate() {
            let r = Rect::new(
                x + (i % columns) as f32 * (cell_w + 12.),
                120. + (i / columns) as f32 * gap,
                cell_w,
                if compact { 48. } else { 60. },
            );
            bordered(
                r,
                18.,
                if self.game.view().deck == *deck {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                ui.theme.bg,
            );
            wave_mark(
                vec2(r.x + 26., r.center().y),
                12.,
                if i % 2 == 0 { CORAL } else { TEAL },
            );
            fit(
                ui,
                deck.title(),
                Rect::new(r.x + 52., r.y, r.w - 70., r.h),
                17.,
                ui.theme.text,
                true,
            );
            if ui.hit(r) {
                if *deck == Deck::Custom {
                    self.custom = true;
                    self.fields = self.game.custom().clone();
                } else if self.game.choose_deck(*deck) {
                    self.decks = false;
                    self.save();
                }
            }
        }
    }
    fn field(&mut self, ui: &mut Ui, id: usize, r: Rect) {
        bordered(
            r,
            16.,
            if self.editing == Some(id) {
                ui.theme.accent
            } else {
                ui.theme.line
            },
            ui.theme.bg,
        );
        fit(
            ui,
            if self.fields[id].is_empty() {
                ["One extreme", "The other extreme"][id]
            } else {
                &self.fields[id]
            },
            Rect::new(r.x + 12., r.y, r.w - 24., r.h),
            17.,
            ui.theme.text,
            true,
        );
        if ui.hit(r) {
            self.editing = Some(id);
            platform::editor_open(&self.fields[id], id + 3, r, 48);
        }
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn draw_keyboard(&mut self, ui: &mut Ui, x: f32, w: f32) {
        let Some(id) = self.editing else {
            return;
        };
        let keys = "QWERTYUIOPASDFGHJKLZXCVBNM0123456789"
            .chars()
            .collect::<Vec<_>>();
        let cols = 10;
        let gap = 4.;
        let cw = (w - gap * 9.) / 10.;
        let y = (screen_height() - 270.).max(80.);
        rounded(Rect::new(x, y, w, 270.), 18., ui.theme.bg);
        for (i, c) in keys.iter().enumerate() {
            let r = Rect::new(
                x + (i % cols) as f32 * (cw + gap),
                y + (i / cols) as f32 * 44.,
                cw,
                40.,
            );
            if ui.button(&c.to_string(), r, false) && self.fields[id].chars().count() < 48 {
                self.fields[id].push(*c);
                self.revision += 1;
            }
        }
        let bw = (w - 16.) / 3.;
        for (i, label) in ["Space", "Delete", "Done"].iter().enumerate() {
            if ui.button(
                label,
                Rect::new(x + i as f32 * (bw + 8.), y + 184., bw, 48.),
                false,
            ) {
                match i {
                    0 => {
                        if self.fields[id].chars().count() < 48 {
                            self.fields[id].push(' ');
                        }
                    }
                    1 => {
                        self.fields[id].pop();
                    }
                    _ => self.close_editor(),
                };
                self.revision += 1;
            }
        }
    }
}

pub fn position(point: Vec2, center: Vec2) -> Option<f32> {
    let delta = point - center;
    if !delta.is_finite() || delta.length_squared() < 64. || (delta.y > 0. && delta.x.abs() < 8.) {
        return None;
    }
    let angle = (-delta.y).max(0.).atan2(delta.x);
    Some((1. - angle / std::f32::consts::PI).clamp(0.02, 0.98))
}

fn radial(center: Vec2, r: f32, p: f32) -> Vec2 {
    let angle = std::f32::consts::PI * (1. - p);
    center + vec2(angle.cos(), -angle.sin()) * r
}
fn sector(center: Vec2, r: f32, left: f32, right: f32, color: Color) {
    let left = left.clamp(0., 1.);
    let right = right.clamp(0., 1.);
    let steps = ((right - left) * 100.).ceil().max(1.) as usize;
    for i in 0..steps {
        draw_triangle(
            center,
            radial(center, r, left + (right - left) * i as f32 / steps as f32),
            radial(
                center,
                r,
                left + (right - left) * (i + 1) as f32 / steps as f32,
            ),
            color,
        );
    }
}
pub fn draw_dial(ui: &Ui, center: Vec2, r: f32, target: Option<f32>, guess: Option<f32>) {
    let face = if ui.theme.saver { BLACK } else { CREAM };
    if !ui.theme.saver {
        sector(
            center + vec2(0., 4.),
            r + 8.,
            0.,
            1.,
            color_u8!(235, 226, 207, 255),
        );
    }
    sector(center, r + 8., 0., 1., ui.theme.line);
    sector(center, r + 6., 0., 1., face);
    if let Some(target) = target {
        let colors = if ui.theme.saver {
            [
                color_u8!(51, 90, 81, 255),
                color_u8!(60, 127, 111, 255),
                color_u8!(161, 129, 52, 255),
                CORAL,
            ]
        } else {
            [
                color_u8!(223, 240, 230, 255),
                color_u8!(152, 206, 181, 255),
                color_u8!(250, 209, 119, 255),
                CORAL,
            ]
        };
        for (width, color) in [0.14, 0.10, 0.062, 0.027].into_iter().zip(colors) {
            sector(center, r - 12., target - width, target + width, color);
        }
        let tip = radial(center, r - 20., target);
        online_style::spark(
            tip,
            5.,
            if ui.theme.saver {
                WHITE
            } else {
                color_u8!(86, 53, 46, 255)
            },
        );
    } else {
        wave_mark(center + vec2(0., -r * 0.43), r * 0.10, ui.theme.line);
    }
    for i in 0..=36 {
        let p = i as f32 / 36.;
        let a = radial(center, r - if i % 6 == 0 { 7. } else { 3. }, p);
        let b = radial(center, r - 13., p);
        draw_line(
            a.x,
            a.y,
            b.x,
            b.y,
            if i % 6 == 0 { 1.6 } else { 1. },
            ui.theme.muted,
        );
    }
    draw_line(
        center.x - r - 6.,
        center.y + 2.,
        center.x + r + 6.,
        center.y + 2.,
        1.,
        ui.theme.line,
    );
    if let Some(guess) = guess {
        let tip = radial(center, r * 0.79, guess);
        draw_line(
            center.x,
            center.y,
            tip.x,
            tip.y,
            (r * 0.028).clamp(3., 7.),
            ui.theme.text,
        );
        draw_circle(tip.x, tip.y, (r * 0.014).clamp(2., 4.), ui.theme.text);
    }
    let knob = (r * 0.13).clamp(12., 29.);
    draw_poly(center.x, center.y, 64, knob + 2., 0., ui.theme.line);
    draw_poly(center.x, center.y, 64, knob, 0., ui.theme.bg);
    draw_circle_lines(center.x, center.y, knob * 0.55, 1., ui.theme.accent);
}
pub fn preview(ui: &Ui, r: Rect) {
    let radius = (r.w * 0.44).min(r.h * 0.60);
    draw_dial(
        ui,
        vec2(r.center().x, r.center().y + radius * 0.35),
        radius,
        Some(0.66),
        Some(0.61),
    );
}
fn wave_mark(center: Vec2, size: f32, ink: Color) {
    for row in [-1., 0., 1.] {
        let mut prev = center + vec2(-size, row * size * 0.38);
        for i in 1..=24 {
            let x = -size + size * 2. * i as f32 / 24.;
            let y = (x / size * std::f32::consts::PI).sin() * size * 0.17;
            let next = center + vec2(x, y + row * size * 0.38);
            draw_line(
                prev.x,
                prev.y,
                next.x,
                next.y,
                (size * 0.1).clamp(0.65, 1.8),
                ink,
            );
            prev = next;
        }
    }
}
fn deck_button(ui: &mut Ui, r: Rect, enabled: bool) -> bool {
    bordered(r, 22., ui.theme.line, ui.theme.bg);
    let c = r.center();
    let ink = if enabled {
        ui.theme.accent
    } else {
        ui.theme.muted
    };
    bordered(
        Rect::new(c.x - 9., c.y - 9., 15., 19.),
        3.,
        ink,
        ui.theme.bg,
    );
    bordered(
        Rect::new(c.x - 4., c.y - 6., 14., 18.),
        3.,
        ink,
        ui.theme.bg,
    );
    wave_mark(c + vec2(3., 3.), 4., ink);
    enabled && ui.hit(r)
}
fn shuffle_button(ui: &mut Ui, r: Rect) -> bool {
    bordered(r, 22., ui.theme.line, ui.theme.bg);
    let c = r.center();
    for sign in [-1., 1.] {
        let start = c + vec2(-8., sign * 5.);
        let end = c + vec2(8., -sign * 5.);
        draw_line(start.x, start.y, end.x, end.y, 1.7, ui.theme.muted);
        draw_triangle(
            end,
            end + vec2(-5., 0.),
            end + vec2(0., sign * 5.),
            ui.theme.muted,
        );
    }
    ui.hit(r)
}
fn endpoint(ui: &Ui, text: &str, r: Rect, ink: Color) {
    wrap(ui, text, r, 17., ui.theme.text, true);
    rounded(
        Rect::new(r.center().x - 20., r.bottom() + 1., 40., 3.),
        1.5,
        ink,
    );
}
fn fit(ui: &Ui, text: &str, r: Rect, size: f32, ink: Color, bold: bool) {
    let size = (size * r.w / ui.text_width(text, size, bold).max(1.)).clamp(8., size);
    ui.centered(text, r, size, ink, bold);
}
fn wrap(ui: &Ui, text: &str, r: Rect, size: f32, ink: Color, bold: bool) {
    let mut size = size;
    let lines = loop {
        let mut lines: Vec<String> = vec![String::new()];
        for word in text.split_whitespace() {
            let next = if lines.last().unwrap().is_empty() {
                word.into()
            } else {
                format!("{} {word}", lines.last().unwrap())
            };
            if ui.text_width(&next, size, bold) <= r.w {
                *lines.last_mut().unwrap() = next;
                continue;
            }
            if !lines.last().unwrap().is_empty() {
                lines.push(String::new());
            }
            for c in word.chars() {
                let next = format!("{}{c}", lines.last().unwrap());
                if ui.text_width(&next, size, bold) > r.w && !lines.last().unwrap().is_empty() {
                    lines.push(String::new());
                }
                lines.last_mut().unwrap().push(c);
            }
        }
        if (size + 3.) * lines.len() as f32 <= r.h || size <= 8. {
            break lines;
        }
        size -= 1.;
    };
    let line_h = size + 3.;
    let top = r.y + (r.h - line_h * lines.len() as f32) / 2.;
    for (i, line) in lines.iter().enumerate() {
        fit(
            ui,
            line,
            Rect::new(r.x, top + i as f32 * line_h, r.w, line_h),
            size,
            ink,
            bold,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_projection_tracks_the_dial_and_ignores_an_ambiguous_pivot() {
        let center = vec2(195., 515.);
        assert_eq!(position(center, center), None);
        assert_eq!(position(center + vec2(0., 20.), center), None);
        assert_eq!(position(vec2(f32::NAN, 0.), center), None);
        assert_eq!(position(center + vec2(-100., 0.), center), Some(0.02));
        assert_eq!(position(center + vec2(100., 0.), center), Some(0.98));
        assert_eq!(position(center + vec2(0., -100.), center), Some(0.5));
        for p in [0.02, 0.25, 0.5, 0.75, 0.98] {
            assert!((position(radial(center, 100., p), center).unwrap() - p).abs() < 0.00001);
        }
    }
}
