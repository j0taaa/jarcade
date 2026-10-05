use crate::ui::{CORAL, Icon, Theme, Ui, bordered, rounded};
use jarcade::{
    board_pan::{BoardPan, PinchZoom},
    feedback::Pulse,
    layout::touch_point,
    nonograms::{Cell, Game, PUZZLES, Size},
};
use macroquad::prelude::*;

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
        let landscape = w >= 540. && h < 500.;
        let viewport = if landscape {
            Rect::new(210., 58., w - 220., h - 70.)
        } else {
            Rect::new(
                (w - (w - 20.).min(720.)) * 0.5,
                106.,
                (w - 20.).min(720.),
                (h - 228.).max(62.),
            )
        };
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
        let pending = game.selected();
        Self {
            game,
            revision: 0,
            drawn_revision: 0,
            configuring: true,
            size,
            pending,
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
    pub fn announcement(&self) -> String {
        if self.help {
            return "Jarcade. Nonograms. Fill runs of squares to match the numbers. Leave at least one empty square between runs. Cross marks empty squares. Drag to paint; choose Move to pan, or pinch to zoom. Undo and hints are available. Close help to continue.".into();
        }
        if self.reset_confirm {
            return "Jarcade. Nonograms. Clear this picture? Your progress can still be restored with Undo. Clear or Cancel.".into();
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
                "Jarcade. Nonograms. Picture complete: {}. {} hints used. Choose the next picture or go back.",
                self.game.puzzle().name,
                self.game.hints()
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
        self.game.choose(self.pending);
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
        self.pan.zoom = self
            .pan
            .zoom
            .clamp(0.69, if screen_width() >= 600. { 2.5 } else { 1.6 });
        self.pan.clamp(l.viewport, l.base * self.pan.zoom);
        self.viewport = vec2(screen_width(), screen_height());
    }
    fn header(&mut self, ui: &mut Ui) {
        if ui.icon_button(Icon::Back, Rect::new(8., 6., 44., 44.), false) {
            self.home = self.back();
        }
        let title = if self.configuring {
            "Nonograms".into()
        } else if self.game.won() {
            self.game.puzzle().name.into()
        } else {
            format!("{} · {}", self.size.name(), self.pending % 4 + 1)
        };
        ui.centered(
            &title,
            Rect::new(58., 6., screen_width() - 116., 44.),
            if screen_width() < 330. { 17. } else { 20. },
            ui.theme.text,
            true,
        );
        if ui.button("?", Rect::new(screen_width() - 52., 6., 44., 44.), false) {
            self.cancel_gesture();
            self.help = true;
            self.revision += 1;
        }
    }
    fn setup(&mut self, ui: &mut Ui) {
        let width = (screen_width() - 24.).min(720.);
        let x = (screen_width() - width) * 0.5;
        let short = screen_height() < 500.;
        let sizes = Rect::new(x, 66., width, 52.);
        bordered(sizes, 19., ui.theme.line, ui.theme.bg);
        for (i, size) in Size::ALL.into_iter().enumerate() {
            let rect = Rect::new(
                sizes.x + 4. + i as f32 * (width - 8.) / 3.,
                sizes.y + 4.,
                (width - 8.) / 3.,
                44.,
            );
            if ui.tab(size.name(), rect, self.size == size) {
                self.size = size;
                self.pending = PUZZLES.iter().position(|p| p.size() == size).unwrap_or(0);
                self.revision += 1;
            }
        }
        if !short {
            ui.label(self.size.description(), x + 4., 148., 14., ui.theme.muted);
        }
        let top = if short { 130. } else { 164. };
        let bottom = screen_height() - if self.save_failed { 90. } else { 72. };
        let columns = if width >= 600. { 4 } else { 2 };
        let rows = if columns == 4 { 1 } else { 2 };
        let gap = 10.;
        let card_w = (width - gap * (columns - 1) as f32) / columns as f32;
        let card_h = ((bottom - top - gap * (rows - 1) as f32) / rows as f32)
            .min(if short { 100. } else { 170. })
            .max(46.);
        let indices = PUZZLES
            .iter()
            .enumerate()
            .filter_map(|(i, p)| (p.size() == self.size).then_some(i))
            .collect::<Vec<_>>();
        for (number, index) in indices.into_iter().enumerate() {
            let rect = Rect::new(
                x + (number % columns) as f32 * (card_w + gap),
                top + (number / columns) as f32 * (card_h + gap),
                card_w,
                card_h,
            );
            let active = self.pending == index;
            bordered(
                rect,
                18.,
                if active { CORAL } else { ui.theme.line },
                if active && !ui.theme.saver {
                    color_u8!(255, 246, 241, 255)
                } else {
                    ui.theme.bg
                },
            );
            let solved = self.game.completed(index);
            let marked = self.game.marked(index);
            let icon_side = (card_h - 36.).min(card_w - 36.).max(14.);
            let icon = Rect::new(
                rect.center().x - icon_side * 0.5,
                rect.y + 9.,
                icon_side,
                icon_side,
            );
            if solved {
                draw_picture(
                    icon,
                    index,
                    if ui.theme.saver {
                        ui.theme.accent
                    } else {
                        CORAL
                    },
                );
            } else {
                let unit = icon_side / 5.;
                for y in 0..5 {
                    for cx in 0..5 {
                        rounded(
                            Rect::new(
                                icon.x + cx as f32 * unit + 1.,
                                icon.y + y as f32 * unit + 1.,
                                unit - 2.,
                                unit - 2.,
                            ),
                            1.5,
                            if (cx + y) % 3 == 0 && marked > 0 {
                                ui.theme.muted
                            } else {
                                ui.theme.line
                            },
                        );
                    }
                }
            }
            ui.centered(
                if solved {
                    PUZZLES[index].name
                } else if marked > 0 {
                    "Continue"
                } else {
                    "Hidden picture"
                },
                Rect::new(rect.x + 2., rect.bottom() - 30., rect.w - 4., 22.),
                if card_w < 150. { 12. } else { 14. },
                ui.theme.text,
                true,
            );
            ui.label(
                &(number + 1).to_string(),
                rect.x + 12.,
                rect.y + 22.,
                12.,
                if active { CORAL } else { ui.theme.muted },
            );
            if ui.hit(rect) {
                self.pending = index;
                self.revision += 1;
            }
        }
        let button = Rect::new(x, screen_height() - 60., width, 48.);
        let label = if self.game.completed(self.pending) {
            "View picture"
        } else if self.game.marked(self.pending) > 0 {
            "Continue picture"
        } else {
            "Play"
        };
        if ui.button(label, button, true) {
            self.start();
            ui.reset_focus();
        }
        if self.save_failed {
            ui.centered(
                "Progress could not be saved",
                Rect::new(x, button.y - 27., width, 22.),
                12.,
                CORAL,
                false,
            );
        }
    }
    fn controls(&mut self, ui: &mut Ui, l: &BoardGeometry) -> Option<Pulse> {
        let landscape = screen_width() >= 540. && screen_height() < 500.;
        let area = if landscape {
            Rect::new(10., 60., 186., 42.)
        } else {
            Rect::new(
                (screen_width() - (screen_width() - 20.).min(560.)) * 0.5,
                56.,
                (screen_width() - 20.).min(560.),
                42.,
            )
        };
        let width = (area.w - 12.) / 3.;
        let mut pulse = None;
        for (i, label) in ["Undo", "Hint", "Reset"].into_iter().enumerate() {
            if ui.button(
                label,
                Rect::new(area.x + i as f32 * (width + 6.), area.y, width, area.h),
                false,
            ) {
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
                        self.revision += 1;
                    }
                }
            }
        }
        let zoom = if landscape {
            Rect::new(10., 112., 186., 44.)
        } else {
            Rect::new(
                (screen_width() - 186.) * 0.5,
                screen_height() - 112.,
                186.,
                44.,
            )
        };
        let decrease = ui.button("−", Rect::new(zoom.x, zoom.y, 48., 44.), false);
        let fit = ui.button("Fit", Rect::new(zoom.x + 54., zoom.y, 78., 44.), false);
        let increase = ui.button("+", Rect::new(zoom.x + 138., zoom.y, 48., 44.), false);
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
        let tools = if landscape {
            Rect::new(10., screen_height() - 60., 186., 48.)
        } else {
            Rect::new(
                (screen_width() - (screen_width() - 20.).min(560.)) * 0.5,
                screen_height() - 60.,
                (screen_width() - 20.).min(560.),
                48.,
            )
        };
        if self.game.won() {
            if landscape {
                ui.centered(
                    "Picture revealed!",
                    Rect::new(10., 170., 186., 32.),
                    15.,
                    ui.theme.accent,
                    true,
                );
            }
            if ui.button("Next picture", tools, true) {
                self.cancel_gesture();
                let next = (self.game.selected() + 1) % PUZZLES.len();
                self.pending = next;
                self.size = PUZZLES[next].size();
                self.configuring = true;
                self.revision += 1;
            }
        } else {
            bordered(tools, 17., ui.theme.line, ui.theme.bg);
            for (i, tool) in [Tool::Fill, Tool::Cross, Tool::Move]
                .into_iter()
                .enumerate()
            {
                let rect = Rect::new(
                    tools.x + 4. + i as f32 * (tools.w - 8.) / 3.,
                    tools.y + 4.,
                    (tools.w - 8.) / 3.,
                    tools.h - 8.,
                );
                if self.tool == tool {
                    bordered(
                        rect,
                        13.,
                        if ui.theme.saver {
                            ui.theme.accent
                        } else {
                            CORAL
                        },
                        if ui.theme.saver {
                            BLACK
                        } else {
                            color_u8!(255, 240, 232, 255)
                        },
                    );
                }
                let color = if self.tool == tool {
                    if ui.theme.saver {
                        ui.theme.accent
                    } else {
                        CORAL
                    }
                } else {
                    ui.theme.muted
                };
                let icon = vec2(
                    rect.x + if landscape { 13. } else { rect.w * 0.21 },
                    rect.center().y,
                );
                draw_tool(tool, icon, 12., color);
                ui.centered(
                    tool.label(),
                    Rect::new(
                        rect.x + if landscape { 20. } else { rect.w * 0.3 },
                        rect.y,
                        rect.w * if landscape { 0.65 } else { 0.66 },
                        rect.h,
                    ),
                    if landscape { 11. } else { 14. },
                    color,
                    true,
                );
                if ui.hit(rect) {
                    self.cancel_gesture();
                    self.tool = tool;
                    self.revision += 1;
                }
            }
            if landscape {
                ui.centered(
                    "Drag to paint",
                    Rect::new(10., 174., 186., 24.),
                    12.,
                    ui.theme.muted,
                    false,
                );
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
        bordered(
            Rect::new(
                l.viewport.x - 3.,
                l.viewport.y - 3.,
                l.viewport.w + 6.,
                l.viewport.h + 6.,
            ),
            14.,
            ui.theme.line,
            ui.theme.bg,
        );
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
        pulse
    }
    fn modal(&mut self, ui: &mut Ui) -> Option<Pulse> {
        let width = (screen_width() - 24.).min(460.);
        let x = (screen_width() - width) * 0.5;
        let height = if self.help { 282. } else { 198. };
        let top = (screen_height() - height) * 0.5;
        bordered(
            Rect::new(x, top, width, height),
            24.,
            ui.theme.line,
            ui.theme.bg,
        );
        ui.centered(
            if self.help {
                "Find the hidden picture"
            } else {
                "Clear this picture?"
            },
            Rect::new(x + 12., top + 14., width - 24., 34.),
            if width < 300. { 18. } else { 22. },
            ui.theme.text,
            true,
        );
        if self.help {
            for (i, line) in [
                "Numbers are runs of filled squares.",
                "Leave a gap between each run.",
                "Cross the squares you leave empty.",
                "Drag to paint; Move to scroll.",
                "Pinch or use + / − to zoom.",
                "Undo and hints are always here.",
            ]
            .into_iter()
            .enumerate()
            {
                ui.centered(
                    line,
                    Rect::new(x + 10., top + 58. + i as f32 * 26., width - 20., 22.),
                    if width < 300. { 12. } else { 14. },
                    ui.theme.muted,
                    false,
                );
            }
            if ui.button(
                "Got it",
                Rect::new(x + 16., top + height - 62., width - 32., 46.),
                true,
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
            if ui.button(
                "Clear",
                Rect::new(x + 26. + w, top + height - 62., w, 46.),
                true,
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
    let fill = if theme.saver { theme.accent } else { CORAL };
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
                unit * 0.48,
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
                unit * 0.48,
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
                unit * 0.48,
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
                unit * 0.48,
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
    draw_board(ui, game, grid, None, &ui.theme, None);
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
