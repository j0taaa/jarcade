use crate::ui::{Theme, Ui, bordered, rounded};
use jarcade::{
    board_pan::{BoardPan, PinchZoom},
    feedback::Pulse,
    layout::touch_point,
    sudoku::{Difficulty, Game, Generator, Mark, Puzzle, Relation, Tool, Variant, bit},
};
use macroquad::prelude::*;
mod art;
mod layout;
use art::{BLUE, Glyph};
use layout::Layout;
fn theme(saver: bool) -> Theme {
    Theme {
        bg: if saver { BLACK } else { WHITE },
        panel: if saver {
            BLACK
        } else {
            color_u8!(245, 246, 251, 255)
        },
        line: if saver {
            color_u8!(65, 68, 86, 255)
        } else {
            color_u8!(216, 220, 233, 255)
        },
        text: if saver {
            color_u8!(238, 240, 255, 255)
        } else {
            color_u8!(38, 42, 66, 255)
        },
        muted: if saver {
            color_u8!(154, 163, 191, 255)
        } else {
            color_u8!(115, 123, 148, 255)
        },
        accent: if saver {
            color_u8!(160, 175, 255, 255)
        } else {
            BLUE
        },
        saver,
    }
}
fn button(ui: &mut Ui, r: Rect, active: bool) -> bool {
    if active {
        if ui.theme.saver {
            bordered(r, 10., ui.theme.accent, BLACK);
        } else {
            rounded(r, 10., BLUE);
        }
    } else {
        bordered(r, 10., ui.theme.line, ui.theme.bg);
    }
    ui.hit(r)
}
fn icon_button(ui: &mut Ui, g: Glyph, r: Rect) -> bool {
    art::glyph(ui, g, r);
    ui.hit(r)
}
fn wrap(ui: &Ui, text: &str, r: Rect, size: f32, colour: Color) {
    if r.h < size {
        return;
    }
    let mut line = String::new();
    let mut y = r.y + size;
    for word in text.split_whitespace() {
        let next = if line.is_empty() {
            word.into()
        } else {
            format!("{line} {word}")
        };
        if ui.text_width(&next, size, false) > r.w && !line.is_empty() {
            ui.label(&line, r.x, y, size, colour);
            line = word.into();
            y += size + 5.;
            if y > r.bottom() {
                return;
            }
        } else {
            line = next;
        }
    }
    ui.label(&line, r.x, y, size, colour);
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Modal {
    None,
    Help,
    Reset,
}
pub struct SudokuPage {
    pub game: Option<Game>,
    pub revision: u64,
    drawn_revision: u64,
    configuring: bool,
    variant: Variant,
    difficulty: Difficulty,
    tool: Tool,
    samples: Vec<Puzzle>,
    generator: Option<Generator>,
    selected: Vec<usize>,
    modal: Modal,
    dirty: bool,
    home: bool,
    save_failed: bool,
    message: String,
    mistakes: Vec<usize>,
    pan: BoardPan,
    pinch: PinchZoom,
    touch_id: Option<u64>,
    mouse_active: bool,
    before_selection: Vec<usize>,
    last_point: Option<Vec2>,
    viewport: Vec2,
}
impl SudokuPage {
    pub fn new(game: Option<Game>) -> Self {
        let variant = game.as_ref().map_or(Variant::Classic, |g| g.puzzle.variant);
        let difficulty = game
            .as_ref()
            .map_or(Difficulty::Easy, |g| g.puzzle.difficulty);
        Self {
            game,
            revision: 0,
            drawn_revision: 0,
            configuring: true,
            variant,
            difficulty,
            tool: Tool::Digit,
            samples: Variant::ALL
                .map(|v| Generator::new(42, v, Difficulty::Easy).finish())
                .into(),
            generator: None,
            selected: vec![0],
            modal: Modal::None,
            dirty: false,
            home: false,
            save_failed: false,
            message: String::new(),
            mistakes: Vec::new(),
            pan: BoardPan::default(),
            pinch: PinchZoom::default(),
            touch_id: None,
            mouse_active: false,
            before_selection: Vec::new(),
            last_point: None,
            viewport: Vec2::ZERO,
        }
    }
    pub fn enter(&mut self) {
        self.cancel_gesture();
        self.configuring = true;
        self.modal = Modal::None;
        self.home = false;
        self.revision += 1;
    }
    pub fn back(&mut self) -> bool {
        self.cancel_gesture();
        self.revision += 1;
        if self.modal != Modal::None {
            self.modal = Modal::None;
            false
        } else if self.generator.take().is_some() {
            false
        } else if !self.configuring {
            self.configuring = true;
            false
        } else {
            true
        }
    }
    pub fn cancel_gesture(&mut self) {
        self.pan.cancel();
        self.pinch = PinchZoom::default();
        self.touch_id = None;
        self.mouse_active = false;
        self.last_point = None;
    }
    pub fn take_home(&mut self) -> bool {
        std::mem::take(&mut self.home)
    }
    pub fn take_save(&mut self) -> Option<String> {
        if std::mem::take(&mut self.dirty) {
            self.game.as_ref().map(Game::encode)
        } else {
            None
        }
    }
    pub fn saved(&mut self, ok: bool) {
        self.save_failed = !ok;
    }
    pub fn needs_frame(&self) -> bool {
        self.generator.is_some() || self.mouse_active || self.drawn_revision != self.revision
    }
    fn changed(&mut self) -> Option<Pulse> {
        self.dirty = true;
        self.revision += 1;
        self.mistakes.clear();
        self.message.clear();
        Some(if self.game.as_ref().is_some_and(Game::won) {
            Pulse::Won
        } else {
            Pulse::Tap
        })
    }
    fn start(&mut self) {
        self.cancel_gesture();
        self.generator = Some(Generator::new(crate::seed(), self.variant, self.difficulty));
        self.revision += 1;
    }
    fn resume(&mut self) {
        self.configuring = false;
        self.tool = Tool::Digit;
        self.selected = vec![0];
        self.pan = BoardPan::default();
        self.viewport = Vec2::ZERO;
        self.message.clear();
        self.mistakes.clear();
        self.revision += 1;
    }
    pub fn preview(&self, ui: &Ui, r: Rect) {
        let p = self.game.as_ref().map_or(&self.samples[0], |g| &g.puzzle);
        let marks = self.game.as_ref().map_or(&[][..], |g| g.marks.as_slice());
        art::draw_board(ui, p, marks, &[], &[], r, true);
    }
    fn focused_label(&self, i: usize) -> Option<String> {
        if self.modal == Modal::Help {
            return (i == 0).then(|| "Got it".into());
        }
        if self.modal == Modal::Reset {
            return ["Cancel", "Clear"].get(i).map(|s| (*s).into());
        }
        if self.generator.is_some() {
            return (i == 0).then(|| "Cancel generation".into());
        }
        if i < 2 {
            return Some(if i == 0 { "Back" } else { "Rules and controls" }.into());
        }
        if self.configuring {
            if i < 8 {
                return Some(Variant::ALL[i - 2].name().into());
            }
            if i < 11 {
                return Some(Difficulty::ALL[i - 8].name().into());
            }
            if i == 11 {
                return Some("New puzzle".into());
            }
            if i == 12 {
                return Some("Resume saved puzzle".into());
            }
        } else {
            if i < 11 {
                return Some(format!("{}", i - 1));
            }
            if i < 15 {
                return Some(Tool::ALL[i - 11].name().into());
            }
            if i == 15 {
                return Some("Erase".into());
            }
            return ["Undo", "Redo", "Check", "Hint", "Fit board", "Reset"]
                .get(i - 16)
                .map(|s| (*s).into());
        }
        None
    }
    pub fn announcement(&self, focus: Option<usize>) -> String {
        let mut s = if self.modal == Modal::Help {
            format!(
                "Jarcade. Sudoku. Rules. {} Space cycles Digit, Corner, Centre and Colour. 1–9 enter a mark; Delete erases. Drag to select multiple cells. Ctrl Z/Y undo/redo. Pinch or mouse wheel to zoom; drag a zoomed board to pan.",
                self.variant.rules().join(" ")
            )
        } else if self.modal == Modal::Reset {
            "Jarcade. Sudoku. Clear this puzzle? Undo can restore the marks. Cancel or Clear."
                .into()
        } else if self.generator.is_some() {
            format!(
                "Jarcade. Sudoku. Generating {} {} puzzle. Verifying a unique logical solution.",
                self.variant.name(),
                self.difficulty.name()
            )
        } else if self.configuring {
            format!(
                "Jarcade. Sudoku. Choose a variant. {}. Difficulty {}. New puzzle{}. Every puzzle has one logical solution.",
                self.variant.name(),
                self.difficulty.name(),
                if self.game.is_some() {
                    " or resume saved puzzle"
                } else {
                    ""
                }
            )
        } else if let Some(g) = &self.game {
            format!(
                "Jarcade. Sudoku. {}. {}. {} mode. {} of 81 filled. Selected cells: {}. {} {}",
                g.puzzle.variant.name(),
                g.puzzle.difficulty.name(),
                self.tool.name(),
                g.filled(),
                self.selected
                    .iter()
                    .map(|i| format!("r{}c{}", i / 9 + 1, i % 9 + 1))
                    .collect::<Vec<_>>()
                    .join(", "),
                if g.won() { "Puzzle complete." } else { "" },
                self.message
            )
        } else {
            "Jarcade. Sudoku.".into()
        };
        if let Some(label) = focus.and_then(|i| self.focused_label(i)) {
            s.push_str(&format!(" Focused control: {label}. Enter to activate."));
        }
        s
    }
    fn header(&mut self, ui: &mut Ui) {
        if icon_button(ui, Glyph::Back, Rect::new(8., 8., 44., 44.)) {
            if self.back() {
                self.home = true;
            }
            ui.reset_focus();
        }
        if icon_button(
            ui,
            Glyph::Help,
            Rect::new(screen_width() - 52., 8., 44., 44.),
        ) {
            self.modal = Modal::Help;
            self.revision += 1;
            ui.reset_focus();
        }
        ui.centered(
            "Sudoku",
            Rect::new(60., 10., screen_width() - 120., 24.),
            20.,
            ui.theme.text,
            true,
        );
        if !self.message.is_empty() && !self.configuring && screen_height() < 450. {
            wrap(
                ui,
                &self.message,
                Rect::new(56., 34., screen_width() - 112., 26.),
                10.,
                ui.theme.muted,
            );
        } else {
            ui.centered(
                &format!("{} · {}", self.variant.name(), self.difficulty.name()),
                Rect::new(54., 36., screen_width() - 108., 19.),
                11.,
                ui.theme.muted,
                false,
            );
        }
    }
    fn setup(&mut self, ui: &mut Ui) {
        let w = (screen_width() - 32.).min(780.);
        let x = (screen_width() - w) * 0.5;
        let compact = screen_height() < 500.;
        let cols = if screen_width() >= 680. || compact {
            3
        } else {
            2
        };
        let rows = 6 / cols;
        let gap = 10.;
        let cw = (w - gap * (cols - 1) as f32) / cols as f32;
        let top = if compact { 64. } else { 80. };
        let card_h = ((screen_height() - top - 166. - gap * (rows - 1) as f32) / rows as f32)
            .clamp(
                if compact { 56. } else { 68. },
                if compact { 100. } else { 156. },
            );
        for (i, v) in Variant::ALL.into_iter().enumerate() {
            let r = Rect::new(
                x + (i % cols) as f32 * (cw + gap),
                top + (i / cols) as f32 * (card_h + gap),
                cw,
                card_h,
            );
            let active = v == self.variant;
            bordered(
                r,
                14.,
                if active {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                ui.theme.bg,
            );
            let size = if compact {
                (card_h - 12.).min(44.)
            } else {
                (card_h - 52.).min(cw - 24.).max(30.)
            };
            let p = &self.samples[i];
            let preview = if compact {
                Rect::new(r.x + 8., r.y + 6., size, size)
            } else {
                Rect::new(r.center().x - size / 2., r.y + 10., size, size)
            };
            art::draw_board(ui, p, &[], &[], &[], preview, true);
            let title = if compact {
                Rect::new(r.x + size + 16., r.y, r.w - size - 22., r.h)
            } else {
                Rect::new(r.x + 6., r.bottom() - 32., r.w - 12., 22.)
            };
            ui.centered(
                v.name(),
                title,
                if cw < 100. { 12. } else { 15. },
                if active {
                    ui.theme.accent
                } else {
                    ui.theme.text
                },
                true,
            );
            if ui.hit(r) {
                self.variant = v;
                self.revision += 1;
            }
        }
        let y = top + rows as f32 * (card_h + gap) + 6.;
        let difficulty = Rect::new(x, y, w, 42.);
        for (i, d) in Difficulty::ALL.into_iter().enumerate() {
            let r = Rect::new(x + i as f32 * w / 3., y, w / 3. - 4., 42.);
            let active = self.difficulty == d;
            ui.centered(
                d.name(),
                r,
                14.,
                if active {
                    ui.theme.accent
                } else {
                    ui.theme.muted
                },
                active,
            );
            if active {
                rounded(
                    Rect::new(r.x + 12., r.bottom() - 3., r.w - 24., 2.5),
                    1.,
                    BLUE,
                );
            }
            if ui.hit(r) {
                self.difficulty = d;
                self.revision += 1;
            }
        }
        let new = Rect::new(
            x,
            difficulty.bottom() + 14.,
            if self.game.is_some() {
                (w - 10.) / 2.
            } else {
                w
            },
            46.,
        );
        let clicked = button(ui, new, true);
        ui.centered(
            "New puzzle",
            new,
            15.,
            if ui.theme.saver {
                ui.theme.accent
            } else {
                WHITE
            },
            true,
        );
        if clicked {
            self.start();
            ui.reset_focus();
        }
        if let Some(g) = &self.game {
            let r = Rect::new(new.right() + 10., new.y, new.w, new.h);
            let resume = button(ui, r, false);
            ui.centered("Resume", r, 15., ui.theme.text, true);
            let same = g.puzzle.variant == self.variant && g.puzzle.difficulty == self.difficulty;
            if !same && screen_height() > 600. {
                ui.centered(
                    &format!(
                        "Saved: {} · {}",
                        g.puzzle.variant.name(),
                        g.puzzle.difficulty.name()
                    ),
                    Rect::new(x, r.bottom() + 12., w, 20.),
                    11.,
                    ui.theme.muted,
                    false,
                );
            }
            if resume {
                self.variant = self.game.as_ref().unwrap().puzzle.variant;
                self.difficulty = self.game.as_ref().unwrap().puzzle.difficulty;
                self.resume();
                ui.reset_focus();
            }
        }
        if screen_height() > 640. {
            ui.centered(
                self.variant.subtitle(),
                Rect::new(x, new.bottom() + 42., w, 26.),
                13.,
                ui.theme.muted,
                false,
            );
        }
    }
    fn modal(&mut self, ui: &mut Ui) -> Option<Pulse> {
        let help = self.modal == Modal::Help;
        let short = help && screen_height() < 450.;
        let w = (screen_width() - 24.).min(if short { 720. } else { 460. });
        let h = if help {
            (screen_height() - 24.).min(416.)
        } else {
            190.
        };
        let r = Rect::new((screen_width() - w) / 2., (screen_height() - h) / 2., w, h);
        bordered(r, 20., ui.theme.line, ui.theme.bg);
        ui.centered(
            if help {
                self.variant.name()
            } else {
                "Clear this puzzle?"
            },
            Rect::new(r.x + 12., r.y + 16., w - 24., 28.),
            22.,
            ui.theme.text,
            true,
        );
        if help {
            let rules = self.variant.rules().join(" ");
            if short {
                let col = (w - 66.) / 2.;
                wrap(
                    ui,
                    &rules,
                    Rect::new(r.x + 22., r.y + 62., col, h - 132.),
                    14.,
                    ui.theme.text,
                );
                wrap(
                    ui,
                    "1–9: digit. Space: next tool. Z / X / C / V: digit / corner / centre / colour. Shift: corner; Ctrl: centre. Drag selects cells. Ctrl + Z / Y: undo / redo. Pinch or scroll: zoom. Check finds mistakes; Hint explains a forced digit.",
                    Rect::new(r.x + 44. + col, r.y + 62., col, h - 132.),
                    12.,
                    ui.theme.muted,
                );
            } else {
                wrap(
                    ui,
                    &rules,
                    Rect::new(r.x + 22., r.y + 62., w - 44., 110.),
                    15.,
                    ui.theme.text,
                );
                wrap(
                    ui,
                    "1–9: digit · Space: next tool. Z / X / C / V: digit / corner / centre / colour. Shift: corner; Ctrl: centre. Drag selects cells. Ctrl + Z / Y: undo / redo.",
                    Rect::new(r.x + 22., r.y + 172., w - 44., 108.),
                    12.,
                    ui.theme.muted,
                );
                wrap(
                    ui,
                    "Pinch or scroll to zoom. Drag to pan when zoomed. Check highlights incorrect digits; Hint explains the next forced digit.",
                    Rect::new(r.x + 22., r.y + 278., w - 44., 70.),
                    12.,
                    ui.theme.muted,
                );
            }
            let close = Rect::new(r.x + 20., r.bottom() - 60., w - 40., 44.);
            let hit = button(ui, close, true);
            ui.centered(
                "Got it",
                close,
                14.,
                if ui.theme.saver {
                    ui.theme.accent
                } else {
                    WHITE
                },
                true,
            );
            if hit {
                self.modal = Modal::None;
                ui.reset_focus();
                self.revision += 1;
            }
        } else {
            wrap(
                ui,
                "Your pencil marks and digits will be cleared. Undo can restore them.",
                Rect::new(r.x + 24., r.y + 62., w - 48., 56.),
                14.,
                ui.theme.muted,
            );
            let a = Rect::new(r.x + 20., r.bottom() - 60., (w - 50.) / 2., 44.);
            let b = Rect::new(a.right() + 10., a.y, a.w, a.h);
            let cancel = button(ui, a, false);
            ui.centered("Cancel", a, 14., ui.theme.text, true);
            let clear = button(ui, b, true);
            ui.centered(
                "Clear",
                b,
                14.,
                if ui.theme.saver {
                    ui.theme.accent
                } else {
                    WHITE
                },
                true,
            );
            if cancel || clear {
                self.modal = Modal::None;
                ui.reset_focus();
                self.revision += 1;
                if clear && self.game.as_mut().is_some_and(Game::reset) {
                    return self.changed();
                }
            }
        }
        None
    }
    fn enter_number(&mut self, d: u8, tool: Tool) -> Option<Pulse> {
        if self
            .game
            .as_mut()
            .is_some_and(|g| g.enter(&self.selected, d, tool))
        {
            self.changed()
        } else {
            None
        }
    }
    fn action(&mut self, i: usize, l: &Layout) -> Option<Pulse> {
        match i {
            0 => {
                if self.game.as_mut().is_some_and(Game::undo) {
                    return self.changed();
                }
            }
            1 => {
                if self.game.as_mut().is_some_and(Game::redo) {
                    return self.changed();
                }
            }
            2 => {
                let g = self.game.as_ref()?;
                self.mistakes = g.mistakes();
                self.message = if g.won() {
                    "Beautifully solved.".into()
                } else if self.mistakes.is_empty() {
                    "Looking good. Keep going.".into()
                } else {
                    format!(
                        "{} incorrect digit{}. Highlighted on the board.",
                        self.mistakes.len(),
                        if self.mistakes.len() == 1 { "" } else { "s" }
                    )
                };
                self.revision += 1;
            }
            3 => {
                if let Some((cell, message)) = self.game.as_mut()?.hint() {
                    self.selected = vec![cell];
                    self.dirty = true;
                    self.message = message;
                    self.mistakes = self.game.as_ref()?.mistakes();
                    self.pan.show_cell(
                        cell,
                        9,
                        l.board.w * self.pan.zoom / 9.,
                        l.board,
                        l.board.size() * self.pan.zoom,
                    );
                    self.revision += 1;
                    return Some(Pulse::Tap);
                }
            }
            4 => {
                self.pan = BoardPan::default();
                self.cancel_gesture();
                self.revision += 1;
            }
            5 => {
                self.modal = Modal::Reset;
                self.cancel_gesture();
                self.revision += 1;
            }
            _ => {}
        }
        None
    }
    fn controls(&mut self, ui: &mut Ui, l: &Layout) -> Option<Pulse> {
        let mut pulse = None;
        for d in 1..=9 {
            let r = l.number(d);
            let hit = button(ui, r, false);
            if self.tool == Tool::Colour {
                rounded(
                    Rect::new(r.x + 7., r.y + 7., r.w - 14., r.h - 14.),
                    8.,
                    art::colour(d as u8, ui.theme.saver),
                );
            } else {
                ui.centered(&d.to_string(), r, 24., ui.theme.accent, true);
            }
            if hit {
                pulse = self.enter_number(d as u8, self.tool).or(pulse);
            }
        }
        for (i, t) in Tool::ALL.into_iter().enumerate() {
            let r = l.tool(i);
            let active = self.tool == t;
            let hit = button(ui, r, active);
            art::tool_icon(ui, t, r, active);
            if hit {
                self.tool = t;
                self.revision += 1;
            }
        }
        let r = l.erase();
        let erase = button(ui, r, false);
        art::glyph(ui, Glyph::Erase, r);
        if erase
            && self
                .game
                .as_mut()
                .is_some_and(|g| g.erase(&self.selected, self.tool))
        {
            pulse = self.changed().or(pulse);
        }
        for (i, g) in [
            Glyph::Undo,
            Glyph::Redo,
            Glyph::Check,
            Glyph::Hint,
            Glyph::Reset,
            Glyph::Reset,
        ]
        .into_iter()
        .enumerate()
        {
            let r = l.action(i);
            let hit = if i == 4 {
                ui.centered("1:1", r, 13., ui.theme.text, true);
                ui.hit(r)
            } else {
                icon_button(ui, g, r)
            };
            if hit {
                pulse = self.action(i, l).or(pulse);
            }
        }
        if l.message.h <= 0. {
            return pulse;
        }
        if let Some(f) = ui.focused_item().and_then(|i| self.focused_label(i)) {
            ui.centered(
                &f,
                Rect::new(l.message.x, l.message.y, l.message.w, 20.),
                12.,
                ui.theme.muted,
                false,
            );
        } else if self.game.as_ref().is_some_and(Game::won) {
            ui.centered(
                "Beautifully solved",
                Rect::new(l.message.x, l.message.y, l.message.w, 24.),
                16.,
                BLUE,
                true,
            );
        } else {
            let message = if self.save_failed {
                "Could not save on this device."
            } else if self.message.is_empty() {
                self.tool.name()
            } else {
                &self.message
            };
            wrap(ui, message, l.message, 12., ui.theme.muted);
        }
        pulse
    }
    fn select_at(&mut self, p: Vec2, l: &Layout, add: bool) {
        if let Some(i) = self
            .pan
            .cell_at(p, l.board, 9, 9, l.board.w * self.pan.zoom / 9.)
        {
            if !add {
                self.selected.clear();
            }
            if !self.selected.contains(&i) {
                self.selected.push(i);
            }
            self.revision += 1;
        }
    }
    fn select_segment(&mut self, p: Vec2, l: &Layout) {
        let start = self.last_point.unwrap_or(p);
        let unit = l.board.w * self.pan.zoom / 9.;
        let n = (start.distance(p) / (unit * 0.35)).ceil().max(1.) as usize;
        for j in 1..=n.min(256) {
            self.select_at(start.lerp(p, j as f32 / n as f32), l, true);
        }
        self.last_point = Some(p);
    }
    fn input(
        &mut self,
        ui: &mut Ui,
        l: &Layout,
        press: Option<Vec2>,
        keys: &[(KeyCode, macroquad::miniquad::KeyMods, bool)],
    ) -> Option<Pulse> {
        let mut pulse = None;
        let ctrl = is_key_down(KeyCode::LeftControl)
            || is_key_down(KeyCode::RightControl)
            || is_key_down(KeyCode::LeftSuper)
            || is_key_down(KeyCode::RightSuper);
        let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        // Use modifiers from each queued key-down, even when the chord has
        // already been released by the time this display frame arrives.
        for &(key, mods, repeat) in keys {
            if repeat {
                continue;
            }
            if key == KeyCode::Space {
                self.tool = self.tool.next();
                ui.reset_focus();
                self.revision += 1;
                continue;
            }
            if ui.keyboard_focus {
                continue;
            }
            let command = mods.ctrl || mods.logo;
            if command && key == KeyCode::Z {
                pulse = self.action(if mods.shift { 1 } else { 0 }, l).or(pulse);
                continue;
            }
            if command && key == KeyCode::Y {
                pulse = self.action(1, l).or(pulse);
                continue;
            }
            if command && key == KeyCode::A {
                self.selected = (0..81).collect();
                self.revision += 1;
                continue;
            }
            if !command
                && let Some(tool) = match key {
                    KeyCode::Z => Some(Tool::Digit),
                    KeyCode::X => Some(Tool::Corner),
                    KeyCode::C => Some(Tool::Centre),
                    KeyCode::V => Some(Tool::Colour),
                    _ => None,
                }
            {
                self.tool = tool;
                self.revision += 1;
            }
            let number = match key {
                KeyCode::Key1 | KeyCode::Kp1 => Some(1),
                KeyCode::Key2 | KeyCode::Kp2 => Some(2),
                KeyCode::Key3 | KeyCode::Kp3 => Some(3),
                KeyCode::Key4 | KeyCode::Kp4 => Some(4),
                KeyCode::Key5 | KeyCode::Kp5 => Some(5),
                KeyCode::Key6 | KeyCode::Kp6 => Some(6),
                KeyCode::Key7 | KeyCode::Kp7 => Some(7),
                KeyCode::Key8 | KeyCode::Kp8 => Some(8),
                KeyCode::Key9 | KeyCode::Kp9 => Some(9),
                _ => None,
            };
            if let Some(d) = number {
                let tool = if command && mods.shift {
                    Tool::Colour
                } else if command {
                    Tool::Centre
                } else if mods.shift {
                    Tool::Corner
                } else {
                    self.tool
                };
                pulse = self.enter_number(d, tool).or(pulse);
            }
            if matches!(
                key,
                KeyCode::Backspace | KeyCode::Delete | KeyCode::Key0 | KeyCode::Kp0
            ) && self
                .game
                .as_mut()
                .is_some_and(|g| g.erase(&self.selected, self.tool))
            {
                pulse = self.changed().or(pulse);
            }
            if key == KeyCode::H {
                pulse = self.action(3, l).or(pulse);
            }
            if key == KeyCode::K {
                pulse = self.action(2, l).or(pulse);
            }
            if key == KeyCode::R {
                pulse = self.action(5, l).or(pulse);
            }
            if let Some((dx, dy)) = match key {
                KeyCode::Left => Some((-1, 0)),
                KeyCode::Right => Some((1, 0)),
                KeyCode::Up => Some((0, -1)),
                KeyCode::Down => Some((0, 1)),
                _ => None,
            } {
                let i = *self.selected.last().unwrap_or(&0);
                let x = (i as i32 % 9 + dx).rem_euclid(9);
                let y = (i as i32 / 9 + dy).rem_euclid(9);
                let next = (y * 9 + x) as usize;
                if !mods.shift && !command {
                    self.selected.clear();
                }
                if !self.selected.contains(&next) {
                    self.selected.push(next);
                }
                self.pan.show_cell(
                    next,
                    9,
                    l.board.w * self.pan.zoom / 9.,
                    l.board,
                    l.board.size() * self.pan.zoom,
                );
                self.revision += 1;
            }
        }
        let touches = touches();
        let active: Vec<_> = touches
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .map(|t| (t.id, touch_point(t.position, screen_dpi_scale())))
            .collect();
        if self
            .pinch
            .update(&active, &mut self.pan, l.board, l.board.size())
        {
            if self.touch_id.take().is_some() {
                self.selected = self.before_selection.clone();
            }
            self.last_point = None;
            self.mouse_active = false;
            self.revision += 1;
            return pulse;
        }
        if !touches.is_empty() {
            if self.touch_id.is_none()
                && active.is_empty()
                && touches.iter().any(|t| t.phase == TouchPhase::Ended)
                && let Some(p) = press.filter(|p| l.board.contains(*p))
            {
                self.select_at(p, l, false);
                ui.reset_focus();
            }
            if self.touch_id.is_none()
                && let Some(t) = touches.iter().find(|t| {
                    t.phase == TouchPhase::Started
                        && l.board
                            .contains(touch_point(t.position, screen_dpi_scale()))
                })
            {
                let p = touch_point(t.position, screen_dpi_scale());
                self.touch_id = Some(t.id);
                self.before_selection = self.selected.clone();
                self.last_point = Some(p);
                if self.pan.zoom > 1.05 {
                    self.pan.begin(p);
                } else {
                    self.select_at(p, l, false);
                }
                ui.reset_focus();
            }
            if let Some(t) = touches.iter().find(|t| Some(t.id) == self.touch_id) {
                let p = touch_point(t.position, screen_dpi_scale());
                match t.phase {
                    TouchPhase::Moved => {
                        if self.pan.zoom > 1.05 {
                            self.pan.update(p, l.board, l.board.size() * self.pan.zoom);
                        } else {
                            self.select_segment(p, l);
                        }
                        self.revision += 1;
                    }
                    TouchPhase::Ended => {
                        if self.pan.zoom > 1.05 {
                            if let Some(p) =
                                self.pan.end(p, l.board, l.board.size() * self.pan.zoom)
                            {
                                self.select_at(p, l, false);
                            }
                        } else {
                            self.select_segment(p, l);
                        }
                        self.touch_id = None;
                        self.last_point = None;
                    }
                    TouchPhase::Cancelled => {
                        self.selected = self.before_selection.clone();
                        self.cancel_gesture();
                        self.revision += 1;
                    }
                    _ => {}
                }
            }
            return pulse;
        }
        let (mx, my) = mouse_position();
        let mouse = vec2(mx, my);
        if let Some(p) = press.filter(|p| l.board.contains(*p)) {
            self.mouse_active = true;
            self.last_point = Some(p);
            if self.pan.zoom > 1.05 {
                self.pan.begin(p);
            } else {
                self.select_at(p, l, ctrl || shift);
            }
            ui.reset_focus();
        }
        if self.mouse_active {
            if is_mouse_button_down(MouseButton::Left) {
                if self.pan.zoom > 1.05 {
                    self.pan
                        .update(mouse, l.board, l.board.size() * self.pan.zoom);
                } else {
                    self.select_segment(mouse, l);
                }
                self.revision += 1;
            } else {
                if self.pan.zoom > 1.05 {
                    if let Some(p) = self.pan.end(mouse, l.board, l.board.size() * self.pan.zoom) {
                        self.select_at(p, l, ctrl || shift);
                    }
                } else {
                    self.select_segment(mouse, l);
                }
                self.mouse_active = false;
                self.last_point = None;
            }
        }
        let (_, wheel) = mouse_wheel();
        if wheel != 0. && l.board.contains(mouse) {
            self.pan.zoom_at(
                mouse,
                (self.pan.zoom * (1. + wheel * 0.1)).clamp(1., 2.5),
                l.board,
                l.board.size(),
            );
            self.revision += 1;
        }
        pulse
    }
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, macroquad::miniquad::KeyMods, bool)],
    ) -> Option<Pulse> {
        ui.theme = theme(ui.theme.saver);
        self.drawn_revision = self.revision;
        if self.modal != Modal::None {
            return self.modal(ui);
        }
        if let Some(generator) = self.generator.as_mut() {
            let done = generator.step();
            let progress = generator.progress();
            ui.centered(
                "Making your puzzle",
                Rect::new(16., screen_height() * 0.4, screen_width() - 32., 36.),
                24.,
                ui.theme.text,
                true,
            );
            ui.centered(
                "One solution. All logic.",
                Rect::new(16., screen_height() * 0.4 + 42., screen_width() - 32., 28.),
                13.,
                ui.theme.muted,
                false,
            );
            let bar = Rect::new(
                (screen_width() - 180.) / 2.,
                screen_height() * 0.4 + 92.,
                180.,
                3.,
            );
            rounded(bar, 1., ui.theme.line);
            rounded(
                Rect {
                    w: bar.w * progress,
                    ..bar
                },
                1.,
                BLUE,
            );
            let r = Rect::new((screen_width() - 120.) / 2., bar.bottom() + 28., 120., 44.);
            let cancel = button(ui, r, false);
            ui.centered("Cancel", r, 14., ui.theme.text, false);
            if cancel {
                self.generator = None;
                self.revision += 1;
            } else if done {
                self.game = Some(Game::new(self.generator.take().unwrap().finish()));
                self.dirty = true;
                self.resume();
            }
            return None;
        }
        self.header(ui);
        if self.modal != Modal::None {
            return None;
        }
        if self.configuring {
            self.setup(ui);
            return None;
        }
        let l = Layout::new(screen_width(), screen_height());
        if self.viewport != vec2(screen_width(), screen_height()) {
            self.cancel_gesture();
            self.pan = BoardPan::default();
            self.viewport = vec2(screen_width(), screen_height());
        }
        let blocked = touches()
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .count()
            >= 2;
        let pointer = if blocked {
            Some(ui.override_pointer(None))
        } else {
            None
        };
        let mut pulse = self.controls(ui, &l);
        if let Some(pointer) = pointer {
            ui.override_pointer(pointer);
        }
        if self.modal != Modal::None {
            return pulse;
        }
        pulse = self.input(ui, &l, press, keys).or(pulse);
        self.pan.clamp(l.board, l.board.size() * self.pan.zoom);
        crate::online_view::clip(Some(l.board));
        let grid = self.pan.board(l.board, l.board.size() * self.pan.zoom);
        if let Some(g) = &self.game {
            art::draw_board(
                ui,
                &g.puzzle,
                &g.marks,
                &self.selected,
                &self.mistakes,
                grid,
                false,
            );
        }
        crate::online_view::clip(None);
        pulse
    }
}
