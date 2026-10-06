use crate::ui::{CORAL, Theme, Ui, bordered, rounded};
use jarcade::{
    board_pan::{BoardPan, PinchZoom},
    feedback::Pulse,
    layout::touch_point,
    nonograms::{Cell, Game, PUZZLES, Size},
};
use macroquad::prelude::*;
mod layout;
use layout::{SetupLayout, board_viewport, landscape};

fn puzzle_theme(saver: bool) -> Theme {
    Theme {
        bg: if saver { BLACK } else { WHITE },
        panel: if saver {
            BLACK
        } else {
            color_u8!(248, 246, 252, 255)
        },
        line: if saver {
            color_u8!(58, 53, 68, 255)
        } else {
            color_u8!(228, 222, 237, 255)
        },
        text: if saver {
            color_u8!(245, 241, 255, 255)
        } else {
            color_u8!(43, 35, 63, 255)
        },
        muted: if saver {
            color_u8!(170, 160, 188, 255)
        } else {
            color_u8!(132, 120, 150, 255)
        },
        accent: CORAL,
        saver,
    }
}

#[derive(Clone, Copy)]
enum Glyph {
    Back,
    Help,
    Undo,
    Hint,
    Reset,
    Minus,
    Fit,
    Plus,
    Arrow,
    Gallery,
    Infinity,
}
fn glyph(g: Glyph, c: Vec2, color: Color) {
    let line = |a: Vec2, b: Vec2| draw_line(c.x + a.x, c.y + a.y, c.x + b.x, c.y + b.y, 1.8, color);
    match g {
        Glyph::Back | Glyph::Arrow => {
            let d = if matches!(g, Glyph::Back) { -1. } else { 1. };
            line(vec2(-8., 0.), vec2(8., 0.));
            line(vec2(3. * d, -5.), vec2(8. * d, 0.));
            line(vec2(3. * d, 5.), vec2(8. * d, 0.));
        }
        Glyph::Minus | Glyph::Plus => {
            line(vec2(-7., 0.), vec2(7., 0.));
            if matches!(g, Glyph::Plus) {
                line(vec2(0., -7.), vec2(0., 7.));
            }
        }
        Glyph::Fit => {
            for d in [vec2(-1., -1.), vec2(1., -1.), vec2(-1., 1.), vec2(1., 1.)] {
                let p = d * 7.;
                line(p, p - vec2(d.x * 4., 0.));
                line(p, p - vec2(0., d.y * 4.));
            }
        }
        Glyph::Hint => {
            draw_circle_lines(c.x, c.y - 3., 6., 1.8, color);
            line(vec2(-3., 2.), vec2(-3., 6.));
            line(vec2(3., 2.), vec2(3., 6.));
            line(vec2(-3., 6.), vec2(3., 6.));
            line(vec2(-2., 9.), vec2(2., 9.));
            for d in [vec2(-1., 0.), vec2(1., 0.), vec2(0., -1.)] {
                line(d * 10. + vec2(0., -3.), d * 12. + vec2(0., -3.));
            }
        }
        Glyph::Undo => {
            line(vec2(-8., -3.), vec2(1., -3.));
            line(vec2(-8., -3.), vec2(-3., -8.));
            line(vec2(-8., -3.), vec2(-3., 2.));
            for i in 0..16 {
                let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / 16.;
                let b = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * (i + 1) as f32 / 16.;
                line(
                    vec2(1. + a.cos() * 5., 2. + a.sin() * 5.),
                    vec2(1. + b.cos() * 5., 2. + b.sin() * 5.),
                );
            }
            line(vec2(1., 7.), vec2(-4., 7.));
        }
        Glyph::Reset => {
            let start = -std::f32::consts::FRAC_PI_2;
            let end = std::f32::consts::PI * 1.27;
            for i in 0..24 {
                let a = start + (end - start) * i as f32 / 24.;
                let b = start + (end - start) * (i + 1) as f32 / 24.;
                line(vec2(a.cos(), a.sin()) * 7., vec2(b.cos(), b.sin()) * 7.);
            }
            let p = vec2(end.cos(), end.sin()) * 7.;
            let d = vec2(-end.sin(), end.cos());
            let n = vec2(-d.y, d.x);
            line(p, p - d * 4. + n * 3.);
            line(p, p - d * 4. - n * 3.);
        }
        Glyph::Help => {
            draw_circle_lines(c.x, c.y, 10., 1.5, color);
            line(vec2(-3., -4.), vec2(0., -6.));
            line(vec2(0., -6.), vec2(4., -3.));
            line(vec2(4., -3.), vec2(0., 1.));
            line(vec2(0., 1.), vec2(0., 3.));
            draw_circle(c.x, c.y + 6., 1., color);
        }
        Glyph::Gallery => {
            for y in 0..2 {
                for x in 0..2 {
                    rounded(
                        Rect::new(c.x - 8. + x as f32 * 9., c.y - 8. + y as f32 * 9., 6., 6.),
                        1.5,
                        color,
                    );
                }
            }
        }
        Glyph::Infinity => {
            let mut last = c + vec2(-10., 0.);
            for i in 1..=48 {
                let t = std::f32::consts::TAU * i as f32 / 48.;
                let p = c + vec2(-10. * t.cos(), 5. * (2. * t).sin());
                draw_line(last.x, last.y, p.x, p.y, 1.8, color);
                last = p;
            }
        }
    }
}
fn icon_button(ui: &mut Ui, g: Glyph, rect: Rect) -> bool {
    glyph(g, rect.center(), ui.theme.text);
    ui.hit(rect)
}
fn primary_button(ui: &mut Ui, label: &str, rect: Rect) -> bool {
    if ui.theme.saver {
        bordered(rect, 16., ui.theme.accent, BLACK);
    } else {
        rounded(rect, 16., ui.theme.text);
    }
    let color = if ui.theme.saver {
        ui.theme.accent
    } else {
        WHITE
    };
    ui.centered(
        label,
        Rect::new(rect.x + 20., rect.y, rect.w - 48., rect.h),
        if rect.w < 200. { 13. } else { 16. },
        color,
        true,
    );
    glyph(
        Glyph::Arrow,
        vec2(rect.right() - 24., rect.center().y),
        color,
    );
    ui.hit(rect)
}

// A pixel emblem for fresh boards; saved boards show actual player marks.
const ENDLESS_MARK: [&str; 9] = [
    "..........",
    "..........",
    ".###..###.",
    "##.####.##",
    "##..##..##",
    "##.####.##",
    ".###..###.",
    "..........",
    "..........",
];
const HIDDEN_MARK: [&str; 9] = [
    "..####...",
    ".##..##..",
    ".....##..",
    "....##...",
    "...##....",
    "...##....",
    ".........",
    "...##....",
    ".........",
];
fn mosaic(ui: &Ui, rect: Rect, cells: Option<&[Cell]>, endless: bool) {
    let side = cells.map_or(if endless { 10 } else { 9 }, |c| {
        (c.len() as f32).sqrt() as usize
    });
    let unit = rect.w / side as f32;
    let gap = (unit * 0.09).clamp(0.6, 2.4);
    for y in 0..side {
        for x in 0..side {
            let cell = cells.map_or_else(
                || {
                    let filled = if endless {
                        y < ENDLESS_MARK.len() && ENDLESS_MARK[y].as_bytes()[x] == b'#'
                    } else {
                        HIDDEN_MARK[y].as_bytes()[x] == b'#'
                    };
                    if filled { Cell::Filled } else { Cell::Blank }
                },
                |cells| cells[y * side + x],
            );
            let r = Rect::new(
                rect.x + x as f32 * unit + gap,
                rect.y + y as f32 * unit + gap,
                unit - gap * 2.,
                unit - gap * 2.,
            );
            let color = if cell == Cell::Filled {
                if cells.is_none() {
                    ui.theme.accent
                } else {
                    ui.theme.text
                }
            } else if ui.theme.saver {
                ui.theme.line
            } else {
                color_u8!(242, 238, 249, 255)
            };
            if !ui.theme.saver || cell != Cell::Blank {
                rounded(r, (unit * 0.15).min(3.), color);
            }
            if cell == Cell::Cross {
                draw_tool(Tool::Cross, r.center(), unit * 0.4, ui.theme.muted);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Fill,
    Cross,
    Move,
}
impl Tool {
    fn cell(self) -> Cell {
        if self == Self::Cross {
            Cell::Cross
        } else {
            Cell::Filled
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Fill => "Fill",
            Self::Cross => "Cross",
            Self::Move => "Move",
        }
    }
}

struct BoardGeometry {
    viewport: Rect,
    base: Vec2,
    left: f32,
    top: f32,
}
impl BoardGeometry {
    fn new(game: &Game) -> Self {
        let w = screen_width();
        let h = screen_height();
        let viewport = board_viewport(w, h);
        let left = game
            .row_clues()
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(1)
            .max(1) as f32
            * 0.62
            + 0.4;
        let top = game
            .column_clues()
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(1)
            .max(1) as f32
            * 0.66
            + 0.4;
        let base = vec2(game.side() as f32 + left, game.side() as f32 + top) * 32.;
        Self {
            viewport,
            base,
            left,
            top,
        }
    }
    fn grid(&self, pan: &BoardPan, side: usize) -> Rect {
        let board = pan.board(self.viewport, self.base * pan.zoom);
        let unit = 32. * pan.zoom;
        Rect::new(
            board.x + self.left * unit,
            board.y + self.top * unit,
            side as f32 * unit,
            side as f32 * unit,
        )
    }
    fn cell_at(&self, point: Vec2, pan: &BoardPan, side: usize) -> Option<usize> {
        let grid = self.grid(pan, side);
        let unit = grid.w / side as f32;
        let left = grid.x.max(self.viewport.x + self.left * unit);
        let top = grid.y.max(self.viewport.y + self.top * unit);
        if !self.viewport.contains(point)
            || point.x < left
            || point.y < top
            || point.x >= grid.right()
            || point.y >= grid.bottom()
        {
            return None;
        }
        Some(((point.y - grid.y) / unit) as usize * side + ((point.x - grid.x) / unit) as usize)
    }
}

pub struct NonogramsPage {
    pub game: Game,
    pub revision: u64,
    drawn_revision: u64,
    configuring: bool,
    size: Size,
    pending: usize,
    endless: bool,
    tool: Tool,
    pan: BoardPan,
    pinch: PinchZoom,
    touch_id: Option<u64>,
    mouse_active: bool,
    painting: bool,
    last_cell: Option<usize>,
    selected: usize,
    keyboard: bool,
    help: bool,
    reset_confirm: bool,
    dirty: bool,
    save_failed: bool,
    home: bool,
    viewport: Vec2,
}
impl NonogramsPage {
    pub fn new(game: Game) -> Self {
        let size = game.puzzle().size();
        let endless = game.is_endless() || !game.has_library_progress();
        let pending = if game.is_endless() {
            PUZZLES.iter().position(|p| p.size() == size).unwrap_or(0)
        } else {
            game.selected()
        };
        Self {
            game,
            revision: 0,
            drawn_revision: 0,
            configuring: true,
            size,
            pending,
            endless,
            tool: Tool::Fill,
            pan: BoardPan::default(),
            pinch: PinchZoom::default(),
            touch_id: None,
            mouse_active: false,
            painting: false,
            last_cell: None,
            selected: 0,
            keyboard: false,
            help: false,
            reset_confirm: false,
            dirty: false,
            save_failed: false,
            home: false,
            viewport: Vec2::ZERO,
        }
    }
    pub fn enter(&mut self) {
        self.cancel_gesture();
        self.configuring = true;
        self.help = false;
        self.reset_confirm = false;
        self.home = false;
        self.revision += 1;
    }
    pub fn back(&mut self) -> bool {
        self.cancel_gesture();
        self.revision += 1;
        if self.help {
            self.help = false;
            false
        } else if self.reset_confirm {
            self.reset_confirm = false;
            false
        } else if !self.configuring {
            self.configuring = true;
            false
        } else {
            true
        }
    }
    pub fn take_home(&mut self) -> bool {
        std::mem::take(&mut self.home)
    }
    pub fn take_save(&mut self) -> Option<String> {
        std::mem::take(&mut self.dirty).then(|| self.game.encode())
    }
    pub fn saved(&mut self, ok: bool) {
        self.save_failed = !ok;
    }
    pub fn cancel_gesture(&mut self) {
        self.game.cancel_stroke();
        self.pan.cancel();
        self.pinch = PinchZoom::default();
        self.touch_id = None;
        self.mouse_active = false;
        self.painting = false;
        self.last_cell = None;
    }
    pub fn needs_frame(&self) -> bool {
        self.mouse_active || self.drawn_revision != self.revision
    }
    pub fn announcement(&self, focus: Option<usize>) -> String {
        let mut message = self.state_announcement();
        if let Some(label) = focus.and_then(|i| self.focused_label(i)) {
            message.push_str(&format!(" Focused control: {label}. Enter to activate."));
        }
        message
    }
    fn focused_label(&self, i: usize) -> Option<String> {
        if self.help {
            return (i == 0).then(|| "Got it".into());
        }
        if self.reset_confirm {
            return ["Cancel", "Clear"].get(i).map(|s| (*s).into());
        }
        if i < 2 {
            return Some(if i == 0 { "Back" } else { "Help" }.into());
        }
        if self.configuring {
            match i {
                2 => Some("Endless".into()),
                3 => Some("Pictures".into()),
                4..=6 => Some(Size::ALL[i - 4].name().into()),
                7 if self.endless => Some(self.start_label().into()),
                7..=10 if !self.endless => Some(format!("Picture {:02}", i - 6)),
                11 if !self.endless => Some(self.start_label().into()),
                _ => None,
            }
        } else if i < 8 {
            ["Undo", "Hint", "Reset", "Zoom out", "Fit board", "Zoom in"]
                .get(i - 2)
                .map(|s| (*s).into())
        } else if self.game.won() {
            (i == 8).then(|| {
                if self.game.is_endless() {
                    "Next puzzle"
                } else {
                    "Next picture"
                }
                .into()
            })
        } else {
            ["Fill", "Cross", "Move"].get(i - 8).map(|s| (*s).into())
        }
    }
    fn state_announcement(&self) -> String {
        if self.help {
            return "Jarcade. Nonograms. Fill runs of squares to match the numbers. Leave at least one empty square between runs. Cross marks empty squares. Drag to paint; choose Move to pan, or pinch to zoom. Square: Fill. X: Cross. Arrows: Move. Curved arrow: Undo. Lightbulb: Hint. Circular arrow: Reset. Minus/plus: zoom. Corners: fit board. Close help to continue.".into();
        }
        if self.reset_confirm {
            return "Jarcade. Nonograms. Clear this picture? Your progress can still be restored with Undo. Clear or Cancel.".into();
        }
        if self.configuring && self.endless {
            return format!(
                "Jarcade. Nonograms. Endless puzzles. {}. Puzzle {}. {}. Play to begin.",
                self.size.name(),
                self.game.endless_number(self.size).unwrap_or(1),
                if self.game.completed(Game::endless_index(self.size)) {
                    "Completed. Next puzzle available"
                } else {
                    "Progress saved separately for each size"
                }
            );
        }
        if self.configuring {
            let number = PUZZLES
                .iter()
                .filter(|p| p.size() == self.size)
                .position(|p| p.id == PUZZLES[self.pending].id)
                .unwrap_or(0)
                + 1;
            return format!(
                "Jarcade. Nonograms. Choose a picture. {}. Picture {number} selected. {}. Play to begin.",
                self.size.name(),
                if self.game.completed(self.pending) {
                    "Completed"
                } else if self.game.marked(self.pending) > 0 {
                    "In progress"
                } else {
                    "New picture"
                }
            );
        }
        if self.game.won() {
            return format!(
                "Jarcade. Nonograms. {} complete: {}. {} hints used. Choose the next {} or go back.",
                if self.game.is_endless() {
                    "Puzzle"
                } else {
                    "Picture"
                },
                self.game.puzzle().name,
                self.game.hints(),
                if self.game.is_endless() {
                    "puzzle"
                } else {
                    "picture"
                }
            );
        }
        format!(
            "Jarcade. Nonograms. {}. {} mode. Row {}, column {}. {:?}. Row clues {:?}, column clues {:?}. {} hints used.",
            self.game.puzzle().size().name(),
            self.tool.label(),
            self.selected / self.game.side() + 1,
            self.selected % self.game.side() + 1,
            self.game.cells()[self.selected],
            self.game.row_clues()[self.selected / self.game.side()],
            self.game.column_clues()[self.selected % self.game.side()],
            self.game.hints()
        )
    }
    fn changed(&mut self) -> Option<Pulse> {
        self.dirty = true;
        self.revision += 1;
        Some(if self.game.won() {
            Pulse::Won
        } else {
            Pulse::Tap
        })
    }
    fn start(&mut self) {
        self.cancel_gesture();
        if self.endless {
            let seed = super::seed();
            if self.game.completed(Game::endless_index(self.size)) {
                self.game.next_endless(self.size, seed);
            } else {
                self.game.choose_endless(self.size, seed);
            }
        } else {
            self.game.choose(self.pending);
        }
        self.configuring = false;
        self.selected = 0;
        self.keyboard = false;
        self.tool = Tool::Fill;
        self.pan = BoardPan::default();
        self.fit_initial();
        self.dirty = true;
        self.revision += 1;
    }
    fn fit_initial(&mut self) {
        let l = BoardGeometry::new(&self.game);
        self.pan.fit(l.viewport, l.base);
        // Large puzzles remain finger-sized; Move/pinch exposes overflow.
        self.pan.zoom = self.pan.zoom.clamp(0.69, 2.5);
        self.pan.clamp(l.viewport, l.base * self.pan.zoom);
        self.viewport = vec2(screen_width(), screen_height());
    }
    fn header(&mut self, ui: &mut Ui) {
        if icon_button(ui, Glyph::Back, Rect::new(8., 6., 44., 44.)) {
            self.home = self.back();
        }
        let title = if self.configuring {
            "Nonograms".into()
        } else if self.game.won() {
            self.game.puzzle().name.into()
        } else {
            format!(
                "Puzzle {:02}",
                if self.game.is_endless() {
                    self.game.endless_number(self.size).unwrap_or(1)
                } else {
                    (self.pending % 4 + 1) as u64
                }
            )
        };
        ui.centered(
            &title,
            Rect::new(56., 6., screen_width() - 112., 44.),
            20.,
            ui.theme.text,
            true,
        );
        if icon_button(
            ui,
            Glyph::Help,
            Rect::new(screen_width() - 52., 6., 44., 44.),
        ) {
            self.cancel_gesture();
            self.help = true;
            ui.reset_focus();
            self.revision += 1;
        }
    }
    fn setup(&mut self, ui: &mut Ui) {
        let l = SetupLayout::new(screen_width(), screen_height());
        let narrow = landscape(screen_width(), screen_height());
        for (i, (name, g)) in [("Endless", Glyph::Infinity), ("Pictures", Glyph::Gallery)]
            .into_iter()
            .enumerate()
        {
            let r = Rect::new(
                l.modes.x + i as f32 * l.modes.w * 0.5,
                l.modes.y,
                l.modes.w * 0.5,
                44.,
            );
            let active = self.endless == (i == 0);
            let color = if active {
                ui.theme.text
            } else {
                ui.theme.muted
            };
            let text_width = ui.text_width(name, 14., active);
            let start = r.center().x - (text_width + 32.) * 0.5;
            glyph(g, vec2(start + 10., r.center().y), color);
            ui.centered(
                name,
                Rect::new(start + 30., r.y, text_width, r.h),
                14.,
                color,
                active,
            );
            if active {
                rounded(
                    Rect::new(r.center().x - 22., r.bottom() - 3., 44., 3.),
                    1.5,
                    ui.theme.accent,
                );
            }
            if ui.hit(r) {
                self.endless = i == 0;
                self.revision += 1;
            }
        }
        let gap = 6.;
        let width = (l.sizes.w - 2. * gap) / 3.;
        for (i, size) in Size::ALL.into_iter().enumerate() {
            let r = Rect::new(l.sizes.x + i as f32 * (width + gap), l.sizes.y, width, 48.);
            let active = self.size == size;
            if active {
                bordered(
                    r,
                    14.,
                    ui.theme.accent,
                    if ui.theme.saver {
                        BLACK
                    } else {
                        color_u8!(255, 244, 237, 255)
                    },
                );
            } else {
                rounded(r, 14., ui.theme.panel);
            }
            ui.centered(
                size.name(),
                r,
                if narrow { 12. } else { 14. },
                if active {
                    ui.theme.text
                } else {
                    ui.theme.muted
                },
                active,
            );
            if ui.hit(r) {
                self.size = size;
                self.pending = PUZZLES.iter().position(|p| p.size() == size).unwrap_or(0);
                self.revision += 1;
            }
        }
        if self.endless {
            self.endless_setup(ui, l.content);
        } else {
            self.pictures_setup(ui, l.content);
        }
        let label = self.start_label();
        if primary_button(ui, label, l.play) {
            self.start();
            ui.reset_focus();
        }
        if self.save_failed {
            ui.centered(
                "Progress could not be saved",
                Rect::new(l.sizes.x, l.sizes.bottom() + 1., l.sizes.w, 13.),
                10.,
                CORAL,
                false,
            );
        }
    }
    fn start_label(&self) -> &'static str {
        let index = if self.endless {
            Game::endless_index(self.size)
        } else {
            self.pending
        };
        if self.game.completed(index) {
            if self.endless {
                "Next puzzle"
            } else {
                "View picture"
            }
        } else if self.game.marked(index) > 0 {
            "Continue"
        } else {
            "Start puzzle"
        }
    }
    fn endless_setup(&mut self, ui: &mut Ui, area: Rect) {
        let index = Game::endless_index(self.size);
        let number = self.game.endless_number(self.size).unwrap_or(1);
        let marked = self.game.marked(index);
        let compact = area.h < 175.;
        let board_side = if compact {
            (area.h - 12.).min(area.w * 0.39)
        } else {
            (area.h - 92.).min(area.w - 40.).min(330.)
        };
        let image = if compact {
            Rect::new(
                area.x + 4.,
                area.center().y - board_side * 0.5,
                board_side,
                board_side,
            )
        } else {
            Rect::new(
                area.center().x - board_side * 0.5,
                area.center().y - (board_side + 80.) * 0.5,
                board_side,
                board_side,
            )
        };
        let saved = self.game.cells_for(index).filter(|_| marked > 0);
        mosaic(ui, image, saved, true);
        let text = if compact {
            Rect::new(
                image.right() + 18.,
                area.center().y - 32.,
                area.right() - image.right() - 22.,
                32.,
            )
        } else {
            Rect::new(area.x, image.bottom() + 16., area.w, 32.)
        };
        let title = if marked > 0 || self.game.completed(index) {
            format!("Puzzle {number:02}")
        } else {
            "Endless".into()
        };
        ui.centered(
            &title,
            text,
            if compact { 20. } else { 28. },
            ui.theme.text,
            true,
        );
        let status = if self.game.completed(index) {
            "Solved. One more?"
        } else if marked > 0 {
            "Pick up where you left off"
        } else {
            "A new puzzle, every time"
        };
        ui.centered(
            status,
            Rect::new(text.x, text.bottom() + 5., text.w, 22.),
            if compact { 11. } else { 13. },
            ui.theme.muted,
            false,
        );
    }
    fn pictures_setup(&mut self, ui: &mut Ui, area: Rect) {
        let columns = if area.w >= 680. { 4 } else { 2 };
        let rows = 4 / columns;
        let gap = if area.h < 120. { 6. } else { 12. };
        let w = (area.w - gap * (columns - 1) as f32) / columns as f32;
        let h = ((area.h - gap * (rows - 1) as f32) / rows as f32).min(228.);
        let top = area.y + (area.h - (h * rows as f32 + gap * (rows - 1) as f32)) * 0.5;
        let indices: Vec<_> = PUZZLES
            .iter()
            .enumerate()
            .filter_map(|(i, p)| (p.size() == self.size).then_some(i))
            .collect();
        for (number, index) in indices.into_iter().enumerate() {
            let r = Rect::new(
                area.x + (number % columns) as f32 * (w + gap),
                top + (number / columns) as f32 * (h + gap),
                w,
                h,
            );
            let active = self.pending == index;
            let solved = self.game.completed(index);
            let marked = self.game.marked(index);
            bordered(
                r,
                16.,
                if active {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                ui.theme.bg,
            );
            let compact = h < 100.;
            let side = if compact {
                (h - 16.).min(w * 0.25)
            } else {
                (h - if solved || marked > 0 { 76. } else { 56. }).min(w - 44.)
            };
            let image = if compact {
                Rect::new(r.x + 10., r.center().y - side * 0.5, side, side)
            } else {
                Rect::new(r.center().x - side * 0.5, r.y + 14., side, side)
            };
            if solved {
                draw_picture(image, index, ui.theme.accent);
            } else {
                mosaic(
                    ui,
                    image,
                    self.game.cells_for(index).filter(|_| marked > 0),
                    false,
                );
            }
            let label = if solved {
                PUZZLES[index].name.into()
            } else {
                format!("{:02}", number + 1)
            };
            let text = if compact {
                Rect::new(
                    image.right() + 6.,
                    r.center().y - 12.,
                    r.right() - image.right() - 12.,
                    24.,
                )
            } else {
                Rect::new(r.x + 8., image.bottom() + 12., r.w - 16., 24.)
            };
            ui.centered(
                &label,
                text,
                if compact { 13. } else { 16. },
                ui.theme.text,
                true,
            );
            if !compact && (solved || marked > 0) {
                ui.centered(
                    if solved {
                        "Solved"
                    } else if marked > 0 {
                        "In progress"
                    } else {
                        "Uncover it"
                    },
                    Rect::new(text.x, text.bottom(), text.w, 20.),
                    11.,
                    ui.theme.muted,
                    false,
                );
            }
            if ui.hit(r) {
                self.pending = index;
                self.revision += 1;
            }
        }
    }
    fn controls(&mut self, ui: &mut Ui, l: &BoardGeometry) -> Option<Pulse> {
        let narrow = landscape(screen_width(), screen_height());
        let rects = layout::action_rects(screen_width(), screen_height());
        let mut pulse = None;
        for (i, g) in [Glyph::Undo, Glyph::Hint, Glyph::Reset]
            .into_iter()
            .enumerate()
        {
            if icon_button(ui, g, rects[i]) {
                self.cancel_gesture();
                match i {
                    0 => {
                        if self.game.undo() {
                            pulse = self.changed();
                        }
                    }
                    1 => {
                        if let Some(index) = self.game.hint() {
                            self.selected = index;
                            self.show_cell(index, l);
                            pulse = self.changed();
                        }
                    }
                    _ => {
                        self.reset_confirm = true;
                        ui.reset_focus();
                        self.revision += 1;
                    }
                }
            }
        }
        if !narrow && screen_width() > 330. {
            let x = (rects[2].right() + rects[3].x) * 0.5;
            draw_line(x, 72., x, 88., 1., ui.theme.line);
        }
        let decrease = icon_button(ui, Glyph::Minus, rects[3]);
        let fit = icon_button(ui, Glyph::Fit, rects[4]);
        let increase = icon_button(ui, Glyph::Plus, rects[5]);
        if decrease || increase || is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::Equal)
        {
            self.cancel_gesture();
            self.pan.zoom_at(
                l.viewport.center(),
                self.pan.zoom
                    * if increase || is_key_pressed(KeyCode::Equal) {
                        1.25
                    } else {
                        0.8
                    },
                l.viewport,
                l.base,
            );
        }
        if fit || is_key_pressed(KeyCode::Key0) {
            self.cancel_gesture();
            self.pan.fit(l.viewport, l.base);
        }
        let width = if narrow {
            144.
        } else {
            (screen_width() - 24.).min(280.)
        };
        let tools = Rect::new(
            if narrow {
                16.
            } else {
                (screen_width() - width) * 0.5
            },
            screen_height() - 64.,
            width,
            52.,
        );
        if self.game.won() {
            if primary_button(
                ui,
                if self.game.is_endless() {
                    "Next puzzle"
                } else {
                    "Next picture"
                },
                tools,
            ) {
                if self.game.is_endless() {
                    self.start();
                    ui.reset_focus();
                    return Some(Pulse::Tap);
                }
                self.cancel_gesture();
                let next = (self.game.selected() + 1) % PUZZLES.len();
                self.pending = next;
                self.size = PUZZLES[next].size();
                self.configuring = true;
                self.revision += 1;
            }
        } else {
            rounded(tools, 18., ui.theme.panel);
            if ui.theme.saver {
                bordered(tools, 18., ui.theme.line, BLACK);
            }
            for (i, tool) in [Tool::Fill, Tool::Cross, Tool::Move]
                .into_iter()
                .enumerate()
            {
                let r = Rect::new(
                    tools.x + 4. + i as f32 * (tools.w - 8.) / 3.,
                    tools.y + 4.,
                    (tools.w - 8.) / 3.,
                    44.,
                );
                let active = self.tool == tool;
                if active {
                    rounded(r, 14., ui.theme.text);
                }
                let color = if active {
                    if ui.theme.saver { BLACK } else { WHITE }
                } else {
                    ui.theme.muted
                };
                draw_tool(
                    tool,
                    vec2(r.center().x, r.center().y - if active { 5. } else { 0. }),
                    23.,
                    color,
                );
                if active {
                    ui.centered(
                        tool.label(),
                        Rect::new(r.x, r.bottom() - 17., r.w, 14.),
                        10.,
                        color,
                        true,
                    );
                }
                if ui.hit(r) {
                    self.cancel_gesture();
                    self.tool = tool;
                    self.revision += 1;
                }
            }
        }
        pulse
    }
    fn show_cell(&mut self, index: usize, l: &BoardGeometry) {
        let unit = 32. * self.pan.zoom;
        let point = vec2(
            (index % self.game.side()) as f32 + l.left,
            (index / self.game.side()) as f32 + l.top,
        ) * unit;
        self.pan.offset = self
            .pan
            .offset
            .min(point)
            .max(point + Vec2::splat(unit) - l.viewport.size());
        self.pan.clamp(l.viewport, l.base * self.pan.zoom);
    }
    fn begin(&mut self, point: Vec2, l: &BoardGeometry, cross: bool) {
        self.keyboard = false;
        if let Some(index) = l.cell_at(point, &self.pan, self.game.side())
            && self.tool != Tool::Move
            && !self.game.won()
        {
            self.painting = true;
            self.selected = index;
            self.last_cell = Some(index);
            self.game
                .begin_stroke(index, if cross { Cell::Cross } else { self.tool.cell() });
        } else {
            self.pan.begin(point);
        }
    }
    fn move_pointer(&mut self, point: Vec2, l: &BoardGeometry) {
        if self.painting {
            if let Some(index) = l.cell_at(point, &self.pan, self.game.side()) {
                if let Some(previous) = self.last_cell {
                    for cell in cell_path(previous, index, self.game.side()) {
                        self.game.paint(cell);
                    }
                } else {
                    self.game.paint(index);
                }
                self.last_cell = Some(index);
                self.selected = index;
            } else {
                self.last_cell = None;
            }
        } else {
            self.pan.update(point, l.viewport, l.base * self.pan.zoom);
        }
    }
    fn end(&mut self, point: Vec2, l: &BoardGeometry) -> Option<Pulse> {
        self.move_pointer(point, l);
        let changed = self.game.finish_stroke();
        self.pan.cancel();
        self.painting = false;
        self.last_cell = None;
        self.touch_id = None;
        self.mouse_active = false;
        if changed { self.changed() } else { None }
    }
    fn input(&mut self, ui: &mut Ui, l: &BoardGeometry, press: Option<Vec2>) -> Option<Pulse> {
        let mut pulse = None;
        let events = touches();
        let active = events
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .map(|t| (t.id, touch_point(t.position, screen_dpi_scale())))
            .collect::<Vec<_>>();
        let suppress = self
            .pinch
            .update(&active, &mut self.pan, l.viewport, l.base)
            // Macroquad retains one entry per touch ID. A complete two-finger
            // gesture can arrive in one frame with both entries already Ended.
            || events.len() > 1;
        if suppress {
            self.game.cancel_stroke();
            self.pan.cancel();
            self.painting = false;
            self.touch_id = None;
            self.mouse_active = false;
            self.last_cell = None;
        } else {
            // Preserve the down-point if a complete stroke arrived between frames.
            if let Some(point) = press.filter(|p| l.viewport.contains(*p))
                && self.touch_id.is_none()
                && !self.mouse_active
            {
                if let Some(touch) = events.iter().find(|t| t.phase != TouchPhase::Cancelled) {
                    self.touch_id = Some(touch.id);
                } else {
                    self.mouse_active = true;
                }
                self.begin(point, l, false);
            }
            for touch in &events {
                let point = touch_point(touch.position, screen_dpi_scale());
                match touch.phase {
                    TouchPhase::Started
                        if l.viewport.contains(point) && self.touch_id.is_none() =>
                    {
                        self.touch_id = Some(touch.id);
                        self.begin(point, l, false);
                    }
                    TouchPhase::Moved if self.touch_id == Some(touch.id) => {
                        self.move_pointer(point, l)
                    }
                    TouchPhase::Ended if self.touch_id == Some(touch.id) => {
                        pulse = self.end(point, l)
                    }
                    TouchPhase::Cancelled => self.cancel_gesture(),
                    _ => {}
                }
            }
        }
        let mouse = Vec2::from(mouse_position());
        if !suppress && events.is_empty() && self.touch_id.is_none() {
            let right = is_mouse_button_pressed(MouseButton::Right);
            if (is_mouse_button_pressed(MouseButton::Left) || right)
                && l.viewport.contains(mouse)
                && !self.mouse_active
            {
                self.mouse_active = true;
                self.begin(mouse, l, right);
            }
            if self.mouse_active {
                if is_mouse_button_released(MouseButton::Left)
                    || is_mouse_button_released(MouseButton::Right)
                {
                    pulse = self.end(mouse, l);
                } else if is_mouse_button_down(MouseButton::Left)
                    || is_mouse_button_down(MouseButton::Right)
                {
                    self.move_pointer(mouse, l);
                } else {
                    self.cancel_gesture();
                }
            }
        }
        let (wx, wy) = mouse_wheel();
        if l.viewport.contains(mouse) && (wx != 0. || wy != 0.) {
            self.cancel_gesture();
            if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
                self.pan.zoom_at(
                    mouse,
                    self.pan.zoom * if wy > 0. { 1.1 } else { 0.9 },
                    l.viewport,
                    l.base,
                );
            } else {
                let scale = if cfg!(target_arch = "wasm32") {
                    1.
                } else {
                    32.
                };
                let delta = if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    vec2(-wy, -wx)
                } else {
                    vec2(-wx, -wy)
                };
                self.pan
                    .scroll(delta * scale, l.viewport, l.base * self.pan.zoom);
            }
        }
        for (key, dx, dy) in [
            (KeyCode::Left, -1, 0),
            (KeyCode::Right, 1, 0),
            (KeyCode::Up, 0, -1),
            (KeyCode::Down, 0, 1),
        ] {
            if is_key_pressed(key) {
                self.cancel_gesture();
                let side = self.game.side();
                let x = self.selected % side;
                let y = self.selected / side;
                self.selected = (y as i32 + dy).clamp(0, side as i32 - 1) as usize * side
                    + (x as i32 + dx).clamp(0, side as i32 - 1) as usize;
                self.keyboard = true;
                self.revision += 1;
                self.show_cell(self.selected, l);
                ui.reset_focus();
            }
        }
        if !ui.keyboard_focus {
            if is_key_pressed(KeyCode::F) {
                self.cancel_gesture();
                self.tool = Tool::Fill;
                self.revision += 1;
            }
            if is_key_pressed(KeyCode::C) {
                self.cancel_gesture();
                self.tool = Tool::Cross;
                self.revision += 1;
            }
            if is_key_pressed(KeyCode::V) {
                self.cancel_gesture();
                self.tool = Tool::Move;
                self.revision += 1;
            }
            if ((is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter))
                && self.tool != Tool::Move)
                || is_key_pressed(KeyCode::X)
            {
                self.cancel_gesture();
                self.game.begin_stroke(
                    self.selected,
                    if is_key_pressed(KeyCode::X) {
                        Cell::Cross
                    } else {
                        self.tool.cell()
                    },
                );
                if self.game.finish_stroke() {
                    pulse = self.changed();
                }
                self.keyboard = true;
            }
            if (is_key_pressed(KeyCode::U)
                || (is_key_pressed(KeyCode::Z)
                    && (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))))
                && self.game.undo()
            {
                pulse = self.changed();
            }
            if is_key_pressed(KeyCode::H)
                && let Some(index) = self.game.hint()
            {
                self.selected = index;
                self.show_cell(index, l);
                pulse = self.changed();
            }
            if is_key_pressed(KeyCode::R) {
                self.cancel_gesture();
                self.reset_confirm = true;
                self.revision += 1;
            }
        }
        pulse
    }
    pub fn draw(&mut self, ui: &mut Ui, press: Option<Vec2>) -> Option<Pulse> {
        ui.theme = puzzle_theme(ui.theme.saver);
        self.drawn_revision = self.revision;
        if self.help || self.reset_confirm {
            return self.modal(ui);
        }
        self.header(ui);
        if self.help || self.reset_confirm {
            return self.modal(ui);
        }
        if self.configuring {
            self.setup(ui);
            return None;
        }
        if self.viewport != vec2(screen_width(), screen_height()) {
            self.cancel_gesture();
            self.fit_initial();
        }
        let l = BoardGeometry::new(&self.game);
        self.pan.clamp(l.viewport, l.base * self.pan.zoom);
        let mut pulse = self.controls(ui, &l);
        if self.help || self.reset_confirm {
            return self.modal(ui).or(pulse);
        }
        if self.configuring {
            self.setup(ui);
            return pulse;
        }
        pulse = self.input(ui, &l, press).or(pulse);
        if self.help || self.reset_confirm {
            return self.modal(ui).or(pulse);
        }
        let dpi = screen_dpi_scale();
        // SAFETY: keep clipping synchronous and isolated to this board draw.
        unsafe {
            get_internal_gl().flush();
            get_internal_gl().quad_gl.scissor(Some((
                (l.viewport.x * dpi) as i32,
                (l.viewport.y * dpi) as i32,
                (l.viewport.w * dpi) as i32,
                (l.viewport.h * dpi) as i32,
            )));
        }
        let grid = l.grid(&self.pan, self.game.side());
        draw_board(
            ui,
            &self.game,
            grid,
            self.keyboard.then_some(self.selected),
            &ui.theme,
            Some(l.viewport),
        );
        unsafe {
            get_internal_gl().flush();
            get_internal_gl().quad_gl.scissor(None);
        }
        let content = l.base * self.pan.zoom;
        for vertical in [false, true] {
            let (length, total, offset) = if vertical {
                (l.viewport.h, content.y, self.pan.offset.y)
            } else {
                (l.viewport.w, content.x, self.pan.offset.x)
            };
            if total > length {
                let thumb = (length * length / total).max(20.);
                let start = offset / (total - length) * (length - thumb);
                let bar = if vertical {
                    Rect::new(l.viewport.right() + 3., l.viewport.y + start, 2., thumb)
                } else {
                    Rect::new(l.viewport.x + start, l.viewport.bottom() + 3., thumb, 2.)
                };
                rounded(bar, 1., ui.theme.muted);
            }
        }
        self.control_hint(ui);
        pulse
    }
    fn control_hint(&self, ui: &Ui) {
        for (i, (r, name)) in layout::action_rects(screen_width(), screen_height())
            .into_iter()
            .zip(["Undo", "Hint", "Reset", "Zoom out", "Fit board", "Zoom in"])
            .enumerate()
        {
            if ui.focused_item() == Some(i + 2) {
                let width = ui.text_width(name, 12., true) + 20.;
                let rect = Rect::new(
                    (r.center().x - width * 0.5).clamp(8., screen_width() - width - 8.),
                    r.bottom() + 3.,
                    width,
                    24.,
                );
                if ui.theme.saver {
                    bordered(rect, 8., ui.theme.accent, BLACK);
                } else {
                    rounded(rect, 8., ui.theme.text);
                }
                ui.centered(
                    name,
                    rect,
                    12.,
                    if ui.theme.saver {
                        ui.theme.accent
                    } else {
                        WHITE
                    },
                    true,
                );
                break;
            }
        }
    }
    fn modal(&mut self, ui: &mut Ui) -> Option<Pulse> {
        let width = (screen_width() - 24.).min(460.);
        let x = (screen_width() - width) * 0.5;
        let height = if self.help {
            (screen_height() - 24.).min(338.)
        } else {
            198.
        };
        let top = (screen_height() - height) * 0.5;
        bordered(
            Rect::new(x, top, width, height),
            24.,
            ui.theme.line,
            ui.theme.bg,
        );
        ui.centered(
            if self.help {
                "How to play"
            } else {
                "Clear this picture?"
            },
            Rect::new(x + 12., top + 14., width - 24., 34.),
            if width < 300. { 18. } else { 22. },
            ui.theme.text,
            true,
        );
        if self.help {
            let unit = ((width - 90.) / 5.).min(32.);
            let grid = Rect::new(
                x + (width - unit * 5.) * 0.5 + 16.,
                top + 60.,
                unit * 5.,
                unit,
            );
            ui.centered(
                "2 1",
                Rect::new(grid.x - 42., grid.y, 36., unit),
                16.,
                ui.theme.text,
                true,
            );
            for i in 0..5 {
                let r = Rect::new(
                    grid.x + i as f32 * unit + 1.,
                    grid.y + 1.,
                    unit - 2.,
                    unit - 2.,
                );
                rounded(
                    r,
                    3.,
                    if [0, 1, 3].contains(&i) {
                        ui.theme.text
                    } else {
                        ui.theme.panel
                    },
                );
                if i == 4 {
                    draw_tool(Tool::Cross, r.center(), 16., ui.theme.muted);
                }
            }
            ui.centered(
                "Filled runs, with a gap between them",
                Rect::new(x + 10., grid.bottom() + 8., width - 20., 20.),
                if width < 300. { 11. } else { 13. },
                ui.theme.muted,
                false,
            );
            let row_top = grid.bottom() + 39.;
            for (i, tool) in [Tool::Fill, Tool::Cross, Tool::Move]
                .into_iter()
                .enumerate()
            {
                let center = vec2(x + width * (i as f32 + 0.5) / 3., row_top + 10.);
                draw_tool(tool, center, 22., ui.theme.text);
                ui.centered(
                    tool.label(),
                    Rect::new(center.x - 40., center.y + 14., 80., 18.),
                    12.,
                    ui.theme.muted,
                    false,
                );
            }
            for (i, (g, name)) in [
                (Glyph::Undo, "Undo"),
                (Glyph::Hint, "Hint"),
                (Glyph::Reset, "Reset"),
            ]
            .into_iter()
            .enumerate()
            {
                let center = vec2(x + width * (i as f32 + 0.5) / 3., row_top + 67.);
                glyph(g, center, ui.theme.text);
                ui.centered(
                    name,
                    Rect::new(center.x - 40., center.y + 14., 80., 18.),
                    12.,
                    ui.theme.muted,
                    false,
                );
            }
            if height > 320. {
                ui.centered(
                    "Pinch to zoom · corners icon to fit",
                    Rect::new(x + 10., top + height - 90., width - 20., 20.),
                    if width < 300. { 11. } else { 12. },
                    ui.theme.muted,
                    false,
                );
            }
            if primary_button(
                ui,
                "Got it",
                Rect::new(x + 16., top + height - 62., width - 32., 46.),
            ) {
                self.help = false;
                self.revision += 1;
            }
        } else {
            ui.centered(
                "You can restore it with Undo.",
                Rect::new(x + 10., top + 65., width - 20., 28.),
                13.,
                ui.theme.muted,
                false,
            );
            let w = (width - 42.) * 0.5;
            if ui.button(
                "Cancel",
                Rect::new(x + 16., top + height - 62., w, 46.),
                false,
            ) {
                self.reset_confirm = false;
                self.revision += 1;
            }
            if primary_button(
                ui,
                "Clear",
                Rect::new(x + 26. + w, top + height - 62., w, 46.),
            ) {
                self.reset_confirm = false;
                if self.game.reset() {
                    return self.changed();
                }
            }
        }
        None
    }
}

fn draw_tool(tool: Tool, center: Vec2, size: f32, color: Color) {
    let d = size * 0.4;
    match tool {
        Tool::Fill => rounded(
            Rect::new(center.x - d, center.y - d, d * 2., d * 2.),
            2.,
            color,
        ),
        Tool::Cross => {
            draw_line(
                center.x - d,
                center.y - d,
                center.x + d,
                center.y + d,
                1.7,
                color,
            );
            draw_line(
                center.x - d,
                center.y + d,
                center.x + d,
                center.y - d,
                1.7,
                color,
            );
        }
        Tool::Move => {
            draw_line(center.x - d, center.y, center.x + d, center.y, 1.5, color);
            draw_line(center.x, center.y - d, center.x, center.y + d, 1.5, color);
            for direction in [vec2(-1., 0.), vec2(1., 0.), vec2(0., -1.), vec2(0., 1.)] {
                let tip = center + direction * d;
                let side = vec2(-direction.y, direction.x) * d * 0.4;
                draw_triangle(
                    tip,
                    tip - direction * d * 0.55 + side,
                    tip - direction * d * 0.55 - side,
                    color,
                );
            }
        }
    }
}
fn draw_board(
    ui: &Ui,
    game: &Game,
    grid: Rect,
    selected: Option<usize>,
    theme: &Theme,
    clip: Option<Rect>,
) {
    let side = game.side();
    let unit = grid.w / side as f32;
    let fill = if game.won() { theme.accent } else { theme.text };
    draw_rectangle(grid.x, grid.y, grid.w, grid.h, theme.panel);
    for (i, cell) in game.cells().iter().enumerate() {
        let rect = Rect::new(
            grid.x + (i % side) as f32 * unit,
            grid.y + (i / side) as f32 * unit,
            unit,
            unit,
        );
        if *cell == Cell::Filled {
            rounded(
                Rect::new(rect.x + 1.5, rect.y + 1.5, rect.w - 3., rect.h - 3.),
                (unit * 0.1).min(4.),
                fill,
            );
        }
        if *cell == Cell::Cross {
            draw_tool(Tool::Cross, rect.center(), unit * 0.5, theme.muted);
        }
        if selected == Some(i) {
            draw_rectangle_lines(
                rect.x + 1.,
                rect.y + 1.,
                rect.w - 2.,
                rect.h - 2.,
                2.,
                theme.text,
            );
        }
    }
    for i in 0..=side {
        let thick = i % 5 == 0;
        let color = if thick { theme.muted } else { theme.line };
        let weight = if thick { 1.4 } else { 0.7 };
        draw_line(
            grid.x + i as f32 * unit,
            grid.y,
            grid.x + i as f32 * unit,
            grid.bottom(),
            weight,
            color,
        );
        draw_line(
            grid.x,
            grid.y + i as f32 * unit,
            grid.right(),
            grid.y + i as f32 * unit,
            weight,
            color,
        );
    }
    let left_units = game
        .row_clues()
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(1)
        .max(1) as f32
        * 0.62
        + 0.4;
    let top_units = game
        .column_clues()
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(1)
        .max(1) as f32
        * 0.66
        + 0.4;
    let clue_left = clip.map_or(grid.x, |view| grid.x.max(view.x + left_units * unit));
    let clue_top = clip.map_or(grid.y, |view| grid.y.max(view.y + top_units * unit));
    if let Some(view) = clip {
        // Keep numbers beside the visible rows and columns when the board pans.
        draw_rectangle(
            view.x,
            view.y,
            (clue_left - view.x).max(0.),
            view.h,
            theme.bg,
        );
        draw_rectangle(
            view.x,
            view.y,
            view.w,
            (clue_top - view.y).max(0.),
            theme.bg,
        );
    }
    for (column, clues) in game.column_clues().iter().enumerate() {
        if grid.x + (column as f32 + 0.5) * unit < clue_left {
            continue;
        }
        let color = if game.line_complete(column, true) {
            theme.accent
        } else {
            theme.text
        };
        if clues.is_empty() {
            ui.centered(
                "0",
                Rect::new(
                    grid.x + column as f32 * unit,
                    clue_top - unit * 0.7,
                    unit,
                    unit * 0.65,
                ),
                (unit * 0.48).min(20.),
                color,
                false,
            );
        }
        for (j, clue) in clues.iter().enumerate() {
            ui.centered(
                &clue.to_string(),
                Rect::new(
                    grid.x + column as f32 * unit,
                    clue_top - (clues.len() - j) as f32 * unit * 0.66,
                    unit,
                    unit * 0.65,
                ),
                (unit * 0.48).min(20.),
                color,
                true,
            );
        }
    }
    for (row, clues) in game.row_clues().iter().enumerate() {
        if grid.y + (row as f32 + 0.5) * unit < clue_top {
            continue;
        }
        let color = if game.line_complete(row, false) {
            theme.accent
        } else {
            theme.text
        };
        if clues.is_empty() {
            ui.centered(
                "0",
                Rect::new(
                    clue_left - unit * 0.7,
                    grid.y + row as f32 * unit,
                    unit * 0.65,
                    unit,
                ),
                (unit * 0.48).min(20.),
                color,
                false,
            );
        }
        for (j, clue) in clues.iter().enumerate() {
            ui.centered(
                &clue.to_string(),
                Rect::new(
                    clue_left - (clues.len() - j) as f32 * unit * 0.62,
                    grid.y + row as f32 * unit,
                    unit * 0.61,
                    unit,
                ),
                (unit * 0.48).min(20.),
                color,
                true,
            );
        }
    }
}
fn draw_picture(rect: Rect, index: usize, color: Color) {
    let puzzle = &PUZZLES[index];
    let side = puzzle.rows.len();
    let unit = rect.w.min(rect.h) / side as f32;
    for i in 0..side * side {
        if puzzle.filled(i) {
            rounded(
                Rect::new(
                    rect.x + (i % side) as f32 * unit + 0.5,
                    rect.y + (i / side) as f32 * unit + 0.5,
                    unit - 1.,
                    unit - 1.,
                ),
                1.,
                color,
            );
        }
    }
}
pub fn preview(ui: &Ui, rect: Rect) {
    static PREVIEW: std::sync::OnceLock<Game> = std::sync::OnceLock::new();
    let game = PREVIEW.get_or_init(|| {
        let mut game = Game::new();
        game.begin_stroke(0, Cell::Cross);
        game.finish_stroke();
        game.begin_stroke(1, Cell::Filled);
        for i in 0..25 {
            if game.puzzle().filled(i) {
                game.paint(i);
            }
        }
        game.finish_stroke();
        game
    });
    let size = rect.w.min(rect.h) * 0.8;
    let unit = size / 6.8;
    let grid_size = unit * 5.;
    let grid = Rect::new(
        rect.center().x - grid_size * 0.5 + unit * 0.4,
        rect.center().y - grid_size * 0.5 + unit * 0.5,
        grid_size,
        grid_size,
    );
    draw_board(ui, game, grid, None, &puzzle_theme(ui.theme.saver), None);
}

/// Paint cells crossed by a fast pointer, including both endpoints.
fn cell_path(from: usize, to: usize, side: usize) -> Vec<usize> {
    let (mut x, mut y) = ((from % side) as i32, (from / side) as i32);
    let (tx, ty) = ((to % side) as i32, (to / side) as i32);
    let (dx, dy) = ((tx - x).abs(), -(ty - y).abs());
    let (sx, sy) = (if x < tx { 1 } else { -1 }, if y < ty { 1 } else { -1 });
    let mut error = dx + dy;
    let mut path = Vec::new();
    loop {
        path.push(y as usize * side + x as usize);
        if x == tx && y == ty {
            break;
        }
        let double = error * 2;
        if double >= dy {
            error += dy;
            x += sx;
        }
        if double <= dx {
            error += dx;
            y += sy;
        }
    }
    path
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fast_drags_cover_skipped_cells_without_crossing_row_boundaries() {
        assert_eq!(cell_path(0, 4, 5), vec![0, 1, 2, 3, 4]);
        assert_eq!(cell_path(24, 4, 5), vec![24, 19, 14, 9, 4]);
        assert_eq!(cell_path(0, 24, 5), vec![0, 6, 12, 18, 24]);
        assert_eq!(cell_path(4, 5, 5).first(), Some(&4));
        assert_eq!(cell_path(4, 5, 5).last(), Some(&5));
    }
}
