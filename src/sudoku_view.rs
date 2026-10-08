use crate::ui::{Theme, Ui, bordered, rounded};
use jarcade::{
    board_pan::{BoardPan, PinchZoom},
    feedback::Pulse,
    layout::touch_point,
    sudoku::{
        ClueMode, Difficulty, Game, Generator, LineKind, Mark, Puzzle, Relation, Rules, Tool,
        Variant, bit,
    },
};
use macroquad::prelude::*;
mod art;
mod layout;
use art::{BLUE, Glyph};
use layout::{Layout, SetupLayout};
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
    GenerationFailed,
    None,
    Help,
    Reset,
}
const DOUBLE_TAP_SECONDS: f64 = 0.35;
struct SelectionGesture {
    start: Vec2,
    cell: usize,
    additive: bool,
    moved: bool,
    started_at: f64,
}
pub struct SudokuPage {
    pub game: Option<Game>,
    pub revision: u64,
    drawn_revision: u64,
    configuring: bool,
    rule_page: usize,
    rules_pan: BoardPan,
    rules_touch_id: Option<u64>,
    rules: Rules,
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
    highlighted_digit: Option<u8>,
    pan: BoardPan,
    pinch: PinchZoom,
    touch_id: Option<u64>,
    mouse_active: bool,
    before_selection: Vec<usize>,
    last_point: Option<Vec2>,
    gesture: Option<SelectionGesture>,
    last_tap: Option<(usize, f64)>,
    viewport: Vec2,
}
impl SudokuPage {
    pub fn new(game: Option<Game>) -> Self {
        let rules = game.as_ref().map_or(Rules::default(), |g| g.puzzle.rules());
        let difficulty = game
            .as_ref()
            .map_or(Difficulty::Easy, |g| g.puzzle.difficulty);
        Self {
            game,
            revision: 0,
            drawn_revision: 0,
            configuring: true,
            rule_page: 0,
            rules_pan: BoardPan::default(),
            rules_touch_id: None,
            rules,
            difficulty,
            tool: Tool::Digit,
            samples: Variant::OPTIONS.map(|v| Generator::preview(42, v)).into(),
            generator: None,
            selected: Vec::new(),
            modal: Modal::None,
            dirty: false,
            home: false,
            save_failed: false,
            message: String::new(),
            mistakes: Vec::new(),
            highlighted_digit: None,
            pan: BoardPan::default(),
            pinch: PinchZoom::default(),
            touch_id: None,
            mouse_active: false,
            before_selection: Vec::new(),
            last_point: None,
            gesture: None,
            last_tap: None,
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
        self.rules_pan.cancel();
        self.rules_touch_id = None;
        self.pinch = PinchZoom::default();
        self.touch_id = None;
        self.mouse_active = false;
        self.last_point = None;
        self.gesture = None;
        self.last_tap = None;
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
        self.last_tap = None;
        self.dirty = true;
        self.revision += 1;
        self.refresh_mistakes();
        Some(if self.game.as_ref().is_some_and(Game::won) {
            Pulse::Won
        } else {
            Pulse::Tap
        })
    }
    fn mistake_message(&self) -> String {
        if self.mistakes.is_empty() {
            String::new()
        } else {
            format!(
                "{} incorrect digit{}. Highlighted in red.",
                self.mistakes.len(),
                if self.mistakes.len() == 1 { "" } else { "s" }
            )
        }
    }
    fn refresh_mistakes(&mut self) {
        self.mistakes = self.game.as_ref().map_or_else(Vec::new, Game::mistakes);
        self.message = self.mistake_message();
    }
    fn clear_selection(&mut self) {
        self.cancel_gesture();
        self.selected.clear();
        self.highlighted_digit = None;
        self.message = self.mistake_message();
        self.revision += 1;
    }
    fn clear_digit_highlight(&mut self) {
        if self.highlighted_digit.take().is_some() {
            self.message = self.mistake_message();
        }
    }
    fn start(&mut self) {
        self.cancel_gesture();
        self.generator = Some(Generator::with_rules(
            crate::seed(),
            self.rules,
            self.difficulty,
        ));
        self.revision += 1;
    }
    fn resume(&mut self) {
        self.configuring = false;
        self.tool = Tool::Digit;
        self.selected.clear();
        self.highlighted_digit = None;
        self.pan = BoardPan::default();
        self.viewport = Vec2::ZERO;
        self.refresh_mistakes();
        self.revision += 1;
    }
    pub fn preview(&self, ui: &Ui, r: Rect) {
        let p = self.game.as_ref().map_or(&self.samples[0], |g| &g.puzzle);
        let marks = self.game.as_ref().map_or(&[][..], |g| g.marks.as_slice());
        art::draw_board(ui, p, marks, art::Highlights::default(), r, true);
    }
    fn focused_label(&self, i: usize) -> Option<String> {
        if self.modal == Modal::GenerationFailed {
            return (i == 0).then(|| "Edit rules".into());
        }
        if self.modal == Modal::Help {
            return (i == 0).then(|| "Close rules".into());
        }
        if self.modal == Modal::Reset {
            return ["Cancel", "Clear"].get(i).map(|s| (*s).into());
        }
        if self.generator.is_some() {
            return (i == 0).then(|| "Cancel generation".into());
        }
        if i < 2 {
            return Some(if i == 0 { "Back" } else { "Puzzle rules" }.into());
        }
        if self.configuring {
            if i < 5 {
                return Some(["Basics rules", "Line rules", "More rules"][i - 2].into());
            }
            if i < 11 {
                let v = Variant::OPTIONS[self.rule_page * 6 + i - 5];
                return Some(if v == Variant::Classic {
                    "Classic: clear extra rules".into()
                } else {
                    format!(
                        "Toggle {}: {}",
                        v.name(),
                        if self.rules.contains(v) {
                            "selected"
                        } else {
                            "off"
                        }
                    )
                });
            }
            if i < 13 {
                return Some(format!(
                    "{} markings: {}. Cycle Off, Partial, Full",
                    if i == 11 { "XV" } else { "Dots" },
                    if i == 11 {
                        self.rules.xv
                    } else {
                        self.rules.kropki
                    }
                    .name()
                ));
            }
            if i < 16 {
                return Some(Difficulty::ALL[i - 13].name().into());
            }
            if i == 16 {
                return Some("New puzzle".into());
            }
            if i == 17 {
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
            let notes_available = self
                .game
                .as_ref()
                .is_some_and(Game::can_fill_classic_candidates);
            if i == 16 && notes_available {
                return Some("Fill notes: row, column and box only (N)".into());
            }
            return ["Undo", "Redo", "Hint", "Reset"]
                .get(i - 16 - usize::from(notes_available))
                .map(|s| (*s).into());
        }
        None
    }
    pub fn announcement(&self, focus: Option<usize>) -> String {
        let mut s = if self.modal == Modal::GenerationFailed {
            "Jarcade. Sudoku. No puzzle found. Try fewer rules, partial markings, or retry. Saved progress is preserved. Edit rules.".into()
        } else if self.modal == Modal::Help {
            format!(
                "Jarcade. Sudoku. Rules. {} Scroll to read all rules. Close.",
                self.popup_rules().explanation()
            )
        } else if self.modal == Modal::Reset {
            "Jarcade. Sudoku. Clear this puzzle? Undo can restore the marks. Cancel or Clear."
                .into()
        } else if self.generator.is_some() {
            format!(
                "Jarcade. Sudoku. Generating {} {} puzzle. Verifying a unique logical solution.",
                self.rules.name(),
                self.difficulty.name()
            )
        } else if self.configuring {
            format!(
                "Jarcade. Sudoku. Choose a variant. {}. Difficulty {}. New puzzle{}. Every puzzle has one logical solution.",
                self.rules.name(),
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
                g.puzzle.name(),
                g.puzzle.difficulty.name(),
                self.tool.name(),
                g.filled(),
                if self.selected.is_empty() {
                    "none".into()
                } else {
                    self.selected
                        .iter()
                        .map(|i| format!("r{}c{}", i / 9 + 1, i % 9 + 1))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                if g.won() { "Puzzle complete." } else { "" },
                self.message
            )
        } else {
            "Jarcade. Sudoku.".into()
        };
        if !self.configuring
            && self.modal == Modal::None
            && self.generator.is_none()
            && let Some(n) = self.highlighted_digit
            && let Some(g) = &self.game
        {
            let cells = g
                .revealed_cells(n)
                .iter()
                .map(|i| format!("r{}c{}", i / 9 + 1, i % 9 + 1))
                .collect::<Vec<_>>()
                .join(", ");
            s.push_str(&format!(
                " Highlighted digit {n}: {}.",
                if cells.is_empty() { "none" } else { &cells }
            ));
            let candidates = g
                .candidate_cells(n)
                .iter()
                .map(|i| format!("r{}c{}", i / 9 + 1, i % 9 + 1))
                .collect::<Vec<_>>()
                .join(", ");
            s.push_str(&format!(
                " Candidate digit {n}: {}. Pencil notes use a lighter highlight.",
                if candidates.is_empty() {
                    "none"
                } else {
                    &candidates
                }
            ));
        }
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
        let rules_button = Rect::new(screen_width() - 84., 8., 76., 44.);
        let show_rules = button(ui, rules_button, false);
        ui.centered("Rules", rules_button, 12., ui.theme.text, true);
        if show_rules {
            self.cancel_gesture();
            self.modal = Modal::Help;
            self.rules_pan = BoardPan::default();
            self.revision += 1;
            ui.reset_focus();
        }
        ui.centered(
            "Sudoku",
            Rect::new(84., 10., screen_width() - 168., 24.),
            20.,
            ui.theme.text,
            true,
        );
        if !self.message.is_empty() && !self.configuring && screen_height() < 450. {
            wrap(
                ui,
                &self.message,
                Rect::new(56., 34., screen_width() - 144., 26.),
                10.,
                ui.theme.muted,
            );
        } else {
            ui.centered(
                &format!(
                    "{} · {}",
                    if self.rules.name().chars().count() > 28 {
                        format!(
                            "{} rules",
                            Variant::OPTIONS
                                .into_iter()
                                .skip(1)
                                .filter(|&v| v != Variant::Miracle && self.rules.contains(v))
                                .count()
                        )
                    } else {
                        self.rules.name()
                    },
                    self.difficulty.name()
                ),
                Rect::new(56., 36., screen_width() - 144., 19.),
                11.,
                ui.theme.muted,
                false,
            );
        }
    }
    fn setup(&mut self, ui: &mut Ui) {
        let layout = SetupLayout::new(screen_width(), screen_height(), self.game.is_some());
        for i in 0..3 {
            let r = layout.tabs[i];
            let active = self.rule_page == i;
            ui.centered(
                ["Basics", "Lines", "More"][i],
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
                    Rect::new(r.x + 12., r.bottom() - 3., r.w - 24., 2.),
                    1.,
                    ui.theme.accent,
                );
            }
            if ui.hit(r) {
                self.rule_page = i;
                self.revision += 1;
                ui.reset_focus();
            }
        }
        for i in 0..6 {
            let v = Variant::OPTIONS[self.rule_page * 6 + i];
            let r = layout.cards[i];
            let compact = r.h < 100.;
            let cw = r.w;
            let card_h = r.h;
            let active = self.rules.contains(v);
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
            let p = &self.samples[self.rule_page * 6 + i];
            let preview = if compact {
                Rect::new(r.x + 8., r.y + 6., size, size)
            } else {
                Rect::new(r.center().x - size / 2., r.y + 10., size, size)
            };
            art::draw_board(ui, p, &[], art::Highlights::default(), preview, true);
            let title = if compact {
                Rect::new(r.x + size + 16., r.y, r.w - size - 22., r.h)
            } else {
                Rect::new(r.x + 6., r.bottom() - 32., r.w - 12., 22.)
            };
            ui.centered(
                v.name(),
                title,
                (15. * title.w / ui.text_width(v.name(), 15., true).max(1.)).clamp(8., 15.),
                if active {
                    ui.theme.accent
                } else {
                    ui.theme.text
                },
                true,
            );
            if ui.hit(r) {
                self.rules.toggle(v);
                self.revision += 1;
            }
        }
        for i in 0..2 {
            let r = layout.markings[i];
            let mode = if i == 0 {
                self.rules.xv
            } else {
                self.rules.kropki
            };
            let clicked = button(ui, r, false);
            ui.centered(
                &format!("{} · {}", if i == 0 { "XV" } else { "Dots" }, mode.name()),
                r,
                13.,
                if mode.enabled() {
                    ui.theme.accent
                } else {
                    ui.theme.muted
                },
                mode.enabled(),
            );
            if clicked {
                let next = match mode {
                    ClueMode::Off => ClueMode::Partial,
                    ClueMode::Partial => ClueMode::Full,
                    ClueMode::Full => ClueMode::Off,
                };
                if i == 0 {
                    self.rules.xv = next;
                } else {
                    self.rules.kropki = next;
                }
                self.revision += 1;
            }
        }
        for (i, d) in Difficulty::ALL.into_iter().enumerate() {
            let r = layout.difficulty[i];
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
        let new = layout.new;
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
            let r = layout.resume;
            let resume = button(ui, r, false);
            ui.centered("Resume", r, 15., ui.theme.text, true);
            let same = g.puzzle.rules() == self.rules && g.puzzle.difficulty == self.difficulty;
            if !same && screen_height() > 600. {
                ui.centered(
                    &format!(
                        "Saved: {} · {}",
                        if g.puzzle.name().len() > 24 {
                            "Puzzle".into()
                        } else {
                            g.puzzle.name()
                        },
                        g.puzzle.difficulty.name()
                    ),
                    Rect::new(new.x, r.bottom() + 12., new.w * 2. + 10., 20.),
                    11.,
                    ui.theme.muted,
                    false,
                );
            }
            if resume {
                self.rules = self.game.as_ref().unwrap().puzzle.rules();
                self.difficulty = self.game.as_ref().unwrap().puzzle.difficulty;
                self.resume();
                ui.reset_focus();
            }
        }
    }

    fn popup_rules(&self) -> Rules {
        if self.configuring {
            self.rules
        } else {
            self.game.as_ref().map_or(self.rules, |g| g.puzzle.rules())
        }
    }
    fn rules_popup(&mut self, ui: &mut Ui, keys: &[(KeyCode, macroquad::miniquad::KeyMods, bool)]) {
        let w = (screen_width() - 24.).min(460.);
        let font = if screen_height() < 450. { 12. } else { 14. };
        let line_height = font + 5.;
        let rows: Vec<_> = self
            .popup_rules()
            .descriptions()
            .iter()
            .map(|text| crate::online_view::wrapped_lines(ui, text, w - 64., font))
            .collect();
        let content_height: f32 = rows
            .iter()
            .map(|lines| lines.len() as f32 * line_height + 14.)
            .sum();
        let h = (content_height + 138.).min((screen_height() - 24.).min(540.));
        let r = Rect::new((screen_width() - w) / 2., (screen_height() - h) / 2., w, h);
        bordered(r, 20., ui.theme.line, ui.theme.bg);
        ui.centered(
            "Rules",
            Rect::new(r.x + 20., r.y + 16., w - 40., 28.),
            22.,
            ui.theme.text,
            true,
        );
        let body = Rect::new(r.x + 22., r.y + 62., w - 44., h - 138.);
        let content = vec2(body.w, content_height);
        let old_offset = self.rules_pan.offset;
        let mouse = Vec2::from(mouse_position());
        let (_, wheel) = mouse_wheel();
        if body.contains(mouse) && wheel != 0. {
            self.rules_pan.scroll(vec2(0., -wheel * 32.), body, content);
        }
        for &(key, _, _) in keys {
            let delta = match key {
                KeyCode::Down => 36.,
                KeyCode::Up => -36.,
                KeyCode::PageDown => body.h * 0.8,
                KeyCode::PageUp => -body.h * 0.8,
                KeyCode::Home => -content_height,
                KeyCode::End => content_height,
                _ => 0.,
            };
            if delta != 0. {
                self.rules_pan.scroll(vec2(0., delta), body, content);
            }
        }
        let touch_events = touches();
        if self.rules_touch_id.is_none()
            && let Some(t) = touch_events.iter().find(|t| {
                t.phase == TouchPhase::Started
                    && body.contains(touch_point(t.position, screen_dpi_scale()))
            })
        {
            self.rules_touch_id = Some(t.id);
            self.rules_pan
                .begin(touch_point(t.position, screen_dpi_scale()));
        }
        if let Some(t) = touch_events
            .iter()
            .find(|t| Some(t.id) == self.rules_touch_id)
        {
            let point = touch_point(t.position, screen_dpi_scale());
            match t.phase {
                TouchPhase::Moved => self.rules_pan.update(point, body, content),
                TouchPhase::Ended => {
                    self.rules_pan.end(point, body, content);
                    self.rules_touch_id = None;
                }
                TouchPhase::Cancelled => {
                    self.rules_pan.cancel();
                    self.rules_touch_id = None;
                }
                _ => {}
            }
        } else if touch_events.is_empty() && self.rules_touch_id.is_none() {
            if is_mouse_button_pressed(MouseButton::Left) && body.contains(mouse) {
                self.rules_pan.begin(mouse);
            }
            if self.rules_pan.active() {
                if is_mouse_button_down(MouseButton::Left) {
                    self.rules_pan.update(mouse, body, content);
                } else {
                    self.rules_pan.end(mouse, body, content);
                }
            }
        }
        self.rules_pan.clamp(body, content);
        if self.rules_pan.offset != old_offset {
            self.revision += 1;
        }
        crate::online_view::clip(Some(body));
        let mut y = body.y - self.rules_pan.offset.y;
        for lines in rows {
            draw_circle(body.x + 3., y + font * 0.65, 2., ui.theme.muted);
            for line in lines {
                ui.label(&line, body.x + 16., y + font, font, ui.theme.text);
                y += line_height;
            }
            y += 14.;
        }
        crate::online_view::clip(None);
        if content_height > body.h {
            let thumb = (body.h * body.h / content_height).max(24.).min(body.h);
            let top =
                body.y + self.rules_pan.offset.y / (content_height - body.h) * (body.h - thumb);
            rounded(
                Rect::new(body.right() + 8., top, 2., thumb),
                1.,
                ui.theme.line,
            );
        }
        let close = Rect::new(r.x + 20., r.bottom() - 60., w - 40., 44.);
        let clicked = button(ui, close, true);
        ui.centered(
            "Close",
            close,
            14.,
            if ui.theme.saver {
                ui.theme.accent
            } else {
                WHITE
            },
            true,
        );
        if clicked {
            self.modal = Modal::None;
            self.cancel_gesture();
            self.revision += 1;
            ui.reset_focus();
        }
    }
    fn modal(
        &mut self,
        ui: &mut Ui,
        keys: &[(KeyCode, macroquad::miniquad::KeyMods, bool)],
    ) -> Option<Pulse> {
        if self.modal == Modal::Help {
            self.rules_popup(ui, keys);
            return None;
        }

        if self.modal == Modal::GenerationFailed {
            let w = (screen_width() - 24.).min(460.);
            let r = Rect::new(
                (screen_width() - w) / 2.,
                (screen_height() - 220.) / 2.,
                w,
                220.,
            );
            bordered(r, 20., ui.theme.line, ui.theme.bg);
            ui.centered(
                "No puzzle found",
                Rect::new(r.x + 16., r.y + 18., w - 32., 28.),
                20.,
                ui.theme.text,
                true,
            );
            wrap(
                ui,
                "Try fewer rules, partial markings, or retry. Your saved puzzle is safe.",
                Rect::new(r.x + 24., r.y + 64., w - 48., 78.),
                14.,
                ui.theme.muted,
            );
            let close = Rect::new(r.x + 20., r.bottom() - 60., w - 40., 44.);
            let clicked = button(ui, close, false);
            ui.centered("Edit rules", close, 14., ui.theme.text, true);
            if clicked {
                self.modal = Modal::None;
                self.revision += 1;
                ui.reset_focus();
            }
            return None;
        }
        let w = (screen_width() - 24.).min(460.);
        let h = 190.;
        let r = Rect::new((screen_width() - w) / 2., (screen_height() - h) / 2., w, h);
        bordered(r, 20., ui.theme.line, ui.theme.bg);
        ui.centered(
            "Clear this puzzle?",
            Rect::new(r.x + 12., r.y + 16., w - 24., 28.),
            22.,
            ui.theme.text,
            true,
        );
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
        None
    }
    fn enter_number(&mut self, d: u8, tool: Tool) -> Option<Pulse> {
        if self.selected.is_empty() {
            if !(1..=9).contains(&d) {
                return None;
            }
            self.highlighted_digit = if self.highlighted_digit == Some(d) {
                None
            } else {
                Some(d)
            };
            self.message = self.highlighted_digit.map_or_else(
                || self.mistake_message(),
                |n| {
                    format!(
                        "Showing {n} · {} filled · {} notes",
                        self.game.as_ref().map_or(0, |g| g.revealed_cells(n).len()),
                        self.game.as_ref().map_or(0, |g| g.candidate_cells(n).len())
                    )
                },
            );
            self.revision += 1;
            return Some(Pulse::Tap);
        }
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
                if let Some((cell, message)) = self.game.as_mut()?.hint() {
                    self.clear_digit_highlight();
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
            3 => {
                self.modal = Modal::Reset;
                self.cancel_gesture();
                self.revision += 1;
            }
            4 => {
                if !self.game.as_ref()?.can_fill_classic_candidates() {
                    return None;
                }
                self.cancel_gesture();
                let changed = self.game.as_mut()?.fill_classic_candidates();
                self.tool = Tool::Corner;
                let pulse = if changed { self.changed() } else { None };
                self.message = if self.mistakes.is_empty() {
                    "Notes filled · row, column, box only.".into()
                } else {
                    format!("{} Notes filled.", self.mistake_message())
                };
                self.revision += 1;
                return pulse;
            }
            _ => {}
        }
        None
    }
    fn controls(&mut self, ui: &mut Ui, l: &Layout) -> Option<Pulse> {
        let mut pulse = None;
        for d in 1..=9 {
            let r = l.number(d);
            let active = self.highlighted_digit == Some(d as u8);
            let hit = button(ui, r, active);
            if self.tool == Tool::Colour && !self.selected.is_empty() {
                rounded(
                    Rect::new(r.x + 7., r.y + 7., r.w - 14., r.h - 14.),
                    8.,
                    art::colour(d as u8, ui.theme.saver),
                );
            } else {
                ui.centered(
                    &d.to_string(),
                    r,
                    24.,
                    if active && !ui.theme.saver {
                        WHITE
                    } else {
                        ui.theme.accent
                    },
                    true,
                );
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
        let notes_available = self
            .game
            .as_ref()
            .is_some_and(Game::can_fill_classic_candidates);
        let r = l.erase(notes_available);
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
        if notes_available {
            let r = l.candidates();
            let fill = button(ui, r, false);
            art::glyph(ui, Glyph::Wand, r);
            if fill {
                pulse = self.action(4, l).or(pulse);
                ui.reset_focus();
            }
        }
        for (i, g) in [Glyph::Undo, Glyph::Redo, Glyph::Hint, Glyph::Reset]
            .into_iter()
            .enumerate()
        {
            if icon_button(ui, g, l.action(i)) {
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
            } else if self.message.is_empty() && self.selected.is_empty() {
                "Tap a cell, or a digit to highlight matches."
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
            self.clear_digit_highlight();
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
    fn cell_at(&self, p: Vec2, l: &Layout) -> Option<usize> {
        self.pan
            .cell_at(p, l.board, 9, 9, l.board.w * self.pan.zoom / 9.)
    }
    fn begin_selection(&mut self, p: Vec2, l: &Layout, additive: bool, now: f64) {
        let Some(cell) = self.cell_at(p, l) else {
            return;
        };
        // Show selection on press, but only toggle/fill after a completed tap.
        // Drag samples must always add cells, never toggle them back off.
        self.before_selection = self.selected.clone();
        self.gesture = Some(SelectionGesture {
            start: p,
            cell,
            additive,
            moved: false,
            started_at: now,
        });
        self.last_point = Some(p);
        if self.pan.zoom > 1.05 {
            self.pan.begin(p);
        } else {
            self.select_at(p, l, additive);
        }
    }
    fn move_selection(&mut self, p: Vec2, l: &Layout) {
        let cell = self.cell_at(p, l);
        let Some(gesture) = &mut self.gesture else {
            return;
        };
        let threshold = (l.board.w * self.pan.zoom / 9. * 0.2).min(8.);
        gesture.moved |= gesture.start.distance(p) > threshold || cell != Some(gesture.cell);
        if self.pan.zoom > 1.05 {
            self.pan.update(p, l.board, l.board.size() * self.pan.zoom);
        } else if gesture.moved {
            self.select_segment(p, l);
        }
        if self.gesture.as_ref().is_some_and(|g| g.moved) {
            self.last_tap = None;
            self.revision += 1;
        }
    }
    fn tap_cell(
        &mut self,
        cell: usize,
        before: &[usize],
        additive: bool,
        now: f64,
        quick: bool,
    ) -> Option<Pulse> {
        self.clear_digit_highlight();
        let double = !additive
            && quick
            && self.last_tap.is_some_and(|(previous, time)| {
                previous == cell && (0.0..=DOUBLE_TAP_SECONDS).contains(&(now - time))
            });
        self.last_tap = None;
        if double
            && let Some(g) = &mut self.game
            && let Some(number) = g.single_candidate(cell)
            && g.enter(&[cell], number, Tool::Digit)
        {
            self.selected = vec![cell];
            return self.changed();
        }
        self.selected = if before.contains(&cell) {
            before.iter().copied().filter(|&i| i != cell).collect()
        } else if additive {
            let mut cells = before.to_vec();
            cells.push(cell);
            cells
        } else {
            vec![cell]
        };
        self.last_tap = (!additive && quick).then_some((cell, now));
        self.revision += 1;
        None
    }
    fn end_selection(&mut self, p: Vec2, l: &Layout, now: f64) -> Option<Pulse> {
        self.move_selection(p, l);
        let gesture = self.gesture.take()?;
        let tap = if self.pan.zoom > 1.05 {
            self.pan.end(p, l.board, l.board.size() * self.pan.zoom)
        } else {
            Some(p)
        };
        self.last_point = None;
        if !gesture.moved
            && let Some(cell) = tap.and_then(|p| self.cell_at(p, l))
            && cell == gesture.cell
        {
            return self.tap_cell(
                cell,
                &self.before_selection.clone(),
                gesture.additive,
                now,
                now - gesture.started_at <= DOUBLE_TAP_SECONDS,
            );
        }
        self.last_tap = None;
        None
    }
    fn input(
        &mut self,
        ui: &mut Ui,
        l: &Layout,
        press: Option<Vec2>,
        keys: &[(KeyCode, macroquad::miniquad::KeyMods, bool)],
    ) -> Option<Pulse> {
        let mut pulse = None;
        if ui.activated || keys.iter().any(|(_, _, repeat)| !repeat) {
            self.last_tap = None;
        }
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
                self.tool = self.tool.toggle_entry();
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
                if mods.shift {
                    self.clear_selection();
                } else {
                    self.clear_digit_highlight();
                    self.selected = (0..81).collect();
                    self.revision += 1;
                }
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
                pulse = self.action(2, l).or(pulse);
            }
            if key == KeyCode::R {
                pulse = self.action(3, l).or(pulse);
            }
            if !command && key == KeyCode::N {
                pulse = self.action(4, l).or(pulse);
            }
            if let Some((dx, dy)) = match key {
                KeyCode::Left => Some((-1, 0)),
                KeyCode::Right => Some((1, 0)),
                KeyCode::Up => Some((0, -1)),
                KeyCode::Down => Some((0, 1)),
                _ => None,
            } {
                self.clear_digit_highlight();
                let empty = self.selected.is_empty();
                let i = *self.selected.last().unwrap_or(&0);
                let x = (i as i32 % 9 + dx).rem_euclid(9);
                let y = (i as i32 / 9 + dy).rem_euclid(9);
                let next = if empty { 0 } else { (y * 9 + x) as usize };
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
            self.gesture = None;
            self.last_tap = None;
            self.revision += 1;
            return pulse;
        }
        if press.is_some_and(|p| !l.board.contains(p)) && !ui.activated {
            self.clear_selection();
            ui.reset_focus();
        }
        if !touches.is_empty() {
            if self.touch_id.is_none()
                && active.is_empty()
                && touches.iter().any(|t| t.phase == TouchPhase::Ended)
                && let Some(p) = press.filter(|p| l.board.contains(*p))
            {
                self.begin_selection(p, l, false, get_time());
                let end = touches
                    .iter()
                    .find(|t| t.phase == TouchPhase::Ended)
                    .map_or(p, |t| touch_point(t.position, screen_dpi_scale()));
                pulse = self.end_selection(end, l, get_time()).or(pulse);
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
                self.begin_selection(p, l, false, get_time());
                ui.reset_focus();
            }
            if let Some(t) = touches.iter().find(|t| Some(t.id) == self.touch_id) {
                let p = touch_point(t.position, screen_dpi_scale());
                match t.phase {
                    TouchPhase::Moved => {
                        self.move_selection(p, l);
                    }
                    TouchPhase::Ended => {
                        pulse = self.end_selection(p, l, get_time()).or(pulse);
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
            self.begin_selection(p, l, ctrl || shift, get_time());
            ui.reset_focus();
        }
        if self.mouse_active {
            if is_mouse_button_down(MouseButton::Left) {
                self.move_selection(mouse, l);
            } else {
                pulse = self.end_selection(mouse, l, get_time()).or(pulse);
                self.mouse_active = false;
                self.last_point = None;
            }
        }
        let (_, wheel) = mouse_wheel();
        if wheel != 0. && l.board.contains(mouse) {
            self.last_tap = None;
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
            return self.modal(ui, keys);
        }
        if let Some(generator) = self.generator.as_mut() {
            let start = get_time();
            let mut done = false;
            for _ in 0..1_024 {
                done = generator.step();
                if done || get_time() - start >= 0.008 {
                    break;
                }
            }
            let progress = generator.progress();
            ui.centered(
                if self.difficulty == Difficulty::Hard {
                    "Finding a hard puzzle"
                } else {
                    "Making your puzzle"
                },
                Rect::new(16., screen_height() * 0.4, screen_width() - 32., 36.),
                24.,
                ui.theme.text,
                true,
            );
            ui.centered(
                if self.difficulty == Difficulty::Hard {
                    "Advanced logic. One solution."
                } else {
                    "One solution. All logic."
                },
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
                match self.generator.take().unwrap().try_finish() {
                    Ok(puzzle) => {
                        self.game = Some(Game::new(puzzle));
                        self.dirty = true;
                        self.resume();
                    }
                    Err(_) => {
                        self.modal = Modal::GenerationFailed;
                        self.revision += 1;
                    }
                }
                ui.reset_focus();
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
        let mut l = Layout::new(screen_width(), screen_height());
        let board_clip = l.board;
        if self
            .game
            .as_ref()
            .is_some_and(|g| !g.puzzle.sandwiches.is_empty())
        {
            l.reserve_clues();
        }
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
        crate::online_view::clip(Some(board_clip));
        let grid = self.pan.board(l.board, l.board.size() * self.pan.zoom);
        if let Some(g) = &self.game {
            art::draw_board(
                ui,
                &g.puzzle,
                &g.marks,
                art::Highlights {
                    selected: &self.selected,
                    mistakes: &self.mistakes,
                    matches: &self
                        .highlighted_digit
                        .map_or_else(Vec::new, |n| g.revealed_cells(n)),
                    candidates: &self
                        .highlighted_digit
                        .map_or_else(Vec::new, |n| g.candidate_cells(n)),
                },
                grid,
                false,
            );
        }
        crate::online_view::clip(None);
        pulse
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page() -> SudokuPage {
        let mut page = SudokuPage::new(Some(Game::new(
            Generator::new(12, Variant::Classic, Difficulty::Medium).finish(),
        )));
        page.resume();
        page
    }
    #[test]
    fn fill_notes_action_works_without_selection_and_preserves_digit_lookup() {
        let mut page = page();
        let l = Layout::new(390., 844.);
        let before = page.game.as_ref().unwrap().marks.clone();
        let values = page.game.as_ref().unwrap().values();
        page.enter_number(4, Tool::Digit);
        assert!(page.focused_label(16).unwrap().contains("Fill notes"));
        assert_eq!(page.focused_label(17).as_deref(), Some("Undo"));
        assert!(page.action(4, &l).is_some());
        assert!(page.selected.is_empty());
        assert_eq!(page.tool, Tool::Corner);
        assert_eq!(page.highlighted_digit, Some(4));
        assert_eq!(page.game.as_ref().unwrap().values(), values);
        assert!(
            page.game
                .as_ref()
                .unwrap()
                .marks
                .iter()
                .any(|m| m.corner != 0)
        );
        assert_eq!(page.game.as_ref().unwrap().hints, 0);
        assert!(page.dirty);
        assert!(page.message.contains("box only"));
        assert!(page.announcement(None).contains("Candidate digit 4:"));
        assert_eq!(page.focused_label(16).as_deref(), Some("Undo"));
        let saved = page.game.as_ref().unwrap().encode();
        assert!(page.action(4, &l).is_none());
        assert_eq!(page.game.as_ref().unwrap().encode(), saved);
        page.action(0, &l);
        assert_eq!(page.game.as_ref().unwrap().marks, before);
        assert!(page.action(4, &l).is_none());
        assert_eq!(page.focused_label(16).as_deref(), Some("Undo"));
    }
    #[test]
    fn first_edit_hides_wand_and_its_shortcut_is_inert() {
        let mut page = page();
        let l = Layout::new(390., 844.);
        page.selected = vec![
            page.game
                .as_ref()
                .unwrap()
                .puzzle
                .givens
                .iter()
                .position(|&v| v == 0)
                .unwrap(),
        ];
        assert!(page.enter_number(3, Tool::Corner).is_some());
        assert_eq!(page.focused_label(16).as_deref(), Some("Undo"));
        let before = page.game.as_ref().unwrap().encode();
        let tool = page.tool;
        let revision = page.revision;
        assert!(page.action(4, &l).is_none());
        assert_eq!(page.game.as_ref().unwrap().encode(), before);
        assert_eq!(page.tool, tool);
        assert_eq!(page.revision, revision);
    }
    fn point(l: &Layout, cell: usize) -> Vec2 {
        l.board.point() + vec2(cell as f32 % 9. + 0.5, (cell / 9) as f32 + 0.5) * l.board.w / 9.
    }
    fn tap(page: &mut SudokuPage, l: &Layout, cell: usize, time: f64) -> Option<Pulse> {
        let p = point(l, cell);
        page.begin_selection(p, l, false, time);
        page.end_selection(p, l, time + 0.01)
    }
    #[test]
    fn repeated_taps_deselect_and_modifier_clicks_remove_only_the_clicked_cell() {
        let mut page = page();
        let l = Layout::new(390., 844.);
        let before = page.game.as_ref().unwrap().encode();
        tap(&mut page, &l, 0, 1.);
        assert_eq!(page.selected, vec![0]);
        tap(&mut page, &l, 0, 2.);
        assert!(page.selected.is_empty());
        page.selected = vec![0, 1, 2];
        page.begin_selection(point(&l, 1), &l, true, 3.);
        page.end_selection(point(&l, 1), &l, 3.01);
        assert_eq!(page.selected, vec![0, 2]);
        assert_eq!(page.game.as_ref().unwrap().encode(), before);
        assert!(!page.dirty);
    }
    #[test]
    fn quick_double_tap_fills_the_users_single_note_in_any_tool_and_is_undoable() {
        for tool in Tool::ALL {
            let mut page = page();
            let l = Layout::new(390., 844.);
            let g = page.game.as_mut().unwrap();
            let i = g.puzzle.givens.iter().position(|&v| v == 0).unwrap();
            let wrong = g.puzzle.solution[i] % 9 + 1;
            g.enter(&[i], wrong, Tool::Corner);
            g.enter(&[i], wrong, Tool::Centre);
            let before = g.encode();
            let notes = g.marks.clone();
            page.tool = tool;
            tap(&mut page, &l, i, 1.);
            assert_eq!(page.game.as_ref().unwrap().encode(), before);
            assert!(tap(&mut page, &l, i, 1.15).is_some());
            assert_eq!(page.tool, tool);
            assert_eq!(page.selected, vec![i]);
            assert_eq!(page.game.as_ref().unwrap().marks[i].value, wrong);
            assert_eq!(page.mistakes, vec![i]);
            assert!(page.dirty);
            page.game.as_mut().unwrap().undo();
            assert_eq!(page.game.as_ref().unwrap().marks, notes);
            assert_eq!(page.game.as_ref().unwrap().single_candidate(i), Some(wrong));
            page.game.as_mut().unwrap().redo();
            assert_eq!(page.game.as_ref().unwrap().marks[i].value, wrong);
        }
    }
    #[test]
    fn slow_taps_long_holds_and_drags_never_fill_a_candidate() {
        let mut page = page();
        let l = Layout::new(390., 844.);
        let g = page.game.as_mut().unwrap();
        let i = (0..80).find(|&i| g.puzzle.givens[i] == 0).unwrap();
        g.enter(&[i], 4, Tool::Corner);
        let before = g.encode();
        tap(&mut page, &l, i, 1.);
        tap(&mut page, &l, i, 2.);
        assert!(page.selected.is_empty());
        page.begin_selection(point(&l, i), &l, false, 3.);
        page.end_selection(point(&l, i), &l, 4.);
        tap(&mut page, &l, i, 4.1);
        assert!(page.selected.is_empty());
        tap(&mut page, &l, i, 5.);
        page.begin_selection(point(&l, i), &l, false, 5.1);
        page.move_selection(point(&l, i + 1), &l);
        page.move_selection(point(&l, i), &l);
        page.end_selection(point(&l, i), &l, 5.2);
        assert!(page.selected.contains(&(i + 1)));
        tap(&mut page, &l, i, 5.25);
        assert_eq!(page.game.as_ref().unwrap().encode(), before);
    }
    #[test]
    fn double_taps_with_no_notes_or_two_distinct_notes_only_toggle_selection() {
        let mut page = page();
        let l = Layout::new(390., 844.);
        let g = page.game.as_mut().unwrap();
        let i = g.puzzle.givens.iter().position(|&v| v == 0).unwrap();
        tap(&mut page, &l, i, 1.);
        tap(&mut page, &l, i, 1.1);
        assert!(page.selected.is_empty());
        let g = page.game.as_mut().unwrap();
        g.enter(&[i], 2, Tool::Corner);
        g.enter(&[i], 3, Tool::Centre);
        let before = g.encode();
        tap(&mut page, &l, i, 2.);
        tap(&mut page, &l, i, 2.1);
        assert!(page.selected.is_empty());
        assert_eq!(page.game.as_ref().unwrap().encode(), before);
    }
    #[test]
    fn no_selection_numbers_highlight_without_editing_progress_or_undo_history() {
        let mut page = page();
        assert!(page.selected.is_empty());
        let before = page.game.as_ref().unwrap().encode();
        for tool in Tool::ALL {
            page.enter_number(3, tool);
            assert_eq!(page.highlighted_digit, Some(3));
            assert!(page.selected.is_empty());
            assert!(page.announcement(None).contains("Highlighted digit 3:"));
            assert!(!page.dirty);
            assert_eq!(page.game.as_ref().unwrap().encode(), before);
            page.enter_number(3, tool);
            assert_eq!(page.highlighted_digit, None);
        }
        assert!(!page.game.as_mut().unwrap().undo());
    }
    #[test]
    fn no_selection_lookup_lists_note_candidates_separately_and_preserves_progress() {
        let mut page = page();
        let g = page.game.as_mut().unwrap();
        let cells: Vec<_> = (0..81).filter(|&i| g.puzzle.givens[i] == 0).collect();
        g.enter(&[cells[0]], 4, Tool::Corner);
        g.enter(&[cells[1]], 4, Tool::Centre);
        let before = g.encode();
        page.enter_number(4, Tool::Digit);
        let candidates = format!(
            "Candidate digit 4: r{}c{}, r{}c{}.",
            cells[0] / 9 + 1,
            cells[0] % 9 + 1,
            cells[1] / 9 + 1,
            cells[1] % 9 + 1,
        );
        assert!(page.announcement(None).contains(&candidates));
        assert!(page.message.contains("2 notes"));
        assert!(!page.dirty);
        assert!(page.selected.is_empty());
        assert_eq!(page.game.as_ref().unwrap().encode(), before);
        page.enter_number(4, Tool::Digit);
        assert!(!page.announcement(None).contains("Candidate digit"));
        assert_eq!(page.game.as_ref().unwrap().encode(), before);
    }
    #[test]
    fn wrong_digits_warn_immediately_and_undo_and_resume_refresh_feedback() {
        let mut page = page();
        let g = page.game.as_ref().unwrap();
        let i = g.puzzle.givens.iter().position(|&v| v == 0).unwrap();
        let correct = g.puzzle.solution[i];
        page.selected = vec![i];
        page.enter_number(correct % 9 + 1, Tool::Digit);
        assert_eq!(page.mistakes, vec![i]);
        assert!(page.message.contains("incorrect digit"));
        assert!(page.announcement(None).contains("incorrect digit"));
        page.game.as_mut().unwrap().undo();
        page.changed();
        assert!(page.mistakes.is_empty());
        assert!(page.message.is_empty());
        page.game.as_mut().unwrap().redo();
        page.changed();
        assert_eq!(page.mistakes, vec![i]);
        page.resume();
        assert!(page.selected.is_empty());
        assert_eq!(page.mistakes, vec![i]);
        page.selected = vec![i];
        page.enter_number(correct, Tool::Digit);
        assert!(page.mistakes.is_empty());
        assert!(page.message.is_empty());
    }
    #[test]
    fn deselect_clears_digit_highlighting_without_changing_progress() {
        let mut page = page();
        let before = page.game.as_ref().unwrap().encode();
        page.selected = vec![4, 5, 6];
        page.clear_selection();
        assert!(page.selected.is_empty());
        page.enter_number(1, Tool::Digit);
        assert_eq!(page.highlighted_digit, Some(1));
        page.clear_selection();
        assert_eq!(page.highlighted_digit, None);
        assert!(page.selected.is_empty());
        assert_eq!(page.game.as_ref().unwrap().encode(), before);
        assert!(!page.dirty);
    }
}
