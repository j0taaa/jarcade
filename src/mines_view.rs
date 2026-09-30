use crate::ui::{CORAL, GREEN, Theme, Ui, bordered, rounded};
use jarcade::{
    board_pan::{BoardPan, PinchZoom},
    feedback::Pulse,
    layout::{MinesLayout, touch_point},
    minesweeper::{BoardSize, Minesweeper, Status},
};
use macroquad::prelude::*;

fn flag(center: Vec2, size: f32, color: Color) {
    let pole = center.x - size * 0.22;
    draw_line(
        pole,
        center.y - size * 0.38,
        pole,
        center.y + size * 0.4,
        size * 0.09,
        color,
    );
    draw_triangle(
        vec2(pole, center.y - size * 0.38),
        vec2(center.x + size * 0.38, center.y - size * 0.13),
        vec2(pole, center.y + size * 0.07),
        color,
    );
    draw_line(
        pole - size * 0.16,
        center.y + size * 0.4,
        pole + size * 0.25,
        center.y + size * 0.4,
        size * 0.09,
        color,
    );
}
fn mine(center: Vec2, size: f32, color: Color) {
    for angle in [0.0_f32, 45., 90., 135.] {
        let (s, c) = angle.to_radians().sin_cos();
        let delta = vec2(c, s) * size * 0.42;
        draw_line(
            center.x - delta.x,
            center.y - delta.y,
            center.x + delta.x,
            center.y + delta.y,
            size * 0.08,
            color,
        );
    }
    draw_circle(center.x, center.y, size * 0.29, color);
}

pub fn draw_board(
    ui: &Ui,
    board: Rect,
    game: &Minesweeper,
    theme: &Theme,
    selected: Option<usize>,
    clip: Option<Rect>,
) {
    let unit = board.w / game.width() as f32;
    let gap = unit * 0.065;
    bordered(
        Rect::new(board.x - 7., board.y - 7., board.w + 14., board.h + 14.),
        18.,
        theme.line,
        theme.bg,
    );
    for (i, tile) in game.tiles().iter().enumerate() {
        let rect = Rect::new(
            board.x + (i % game.width()) as f32 * unit + gap / 2.,
            board.y + (i / game.width()) as f32 * unit + gap / 2.,
            unit - gap,
            unit - gap,
        );
        if clip.is_some_and(|view| !rect.overlaps(&view)) {
            continue;
        }
        let exploded = game.exploded() == Some(i);
        let fill = if theme.saver {
            BLACK
        } else if exploded {
            color_u8!(255, 223, 208, 255)
        } else if tile.revealed {
            color_u8!(248, 250, 247, 255)
        } else {
            color_u8!(220, 236, 211, 255)
        };
        bordered(
            rect,
            unit * 0.13,
            if exploded {
                CORAL
            } else if theme.saver {
                if tile.revealed {
                    color_u8!(14, 19, 15, 255)
                } else {
                    color_u8!(66, 88, 71, 255)
                }
            } else if tile.revealed {
                theme.line
            } else {
                color_u8!(207, 225, 198, 255)
            },
            fill,
        );
        if tile.flagged {
            flag(
                rect.center(),
                unit * 0.57,
                if game.status() == Status::Lost && !tile.mine {
                    CORAL
                } else {
                    theme.accent
                },
            );
            if game.status() == Status::Lost && !tile.mine {
                let p = rect.center();
                let d = unit * 0.3;
                draw_line(p.x - d, p.y + d, p.x + d, p.y - d, unit * 0.06, CORAL);
            }
        } else if tile.mine && game.finished() {
            mine(
                rect.center(),
                unit * 0.62,
                if exploded { CORAL } else { theme.text },
            );
        } else if tile.revealed && tile.adjacent > 0 {
            let color = if theme.saver {
                theme.text
            } else {
                match tile.adjacent {
                    1 => color_u8!(56, 116, 161, 255),
                    2 => theme.accent,
                    3 => CORAL,
                    _ => color_u8!(133, 89, 160, 255),
                }
            };
            ui.centered(&tile.adjacent.to_string(), rect, unit * 0.48, color, true);
        }
        if selected == Some(i) {
            draw_rectangle_lines(
                rect.x + 1.,
                rect.y + 1.,
                rect.w - 2.,
                rect.h - 2.,
                2.,
                theme.accent,
            );
        }
    }
}

pub struct MinesPage {
    pub game: Minesweeper,
    pub flag_mode: bool,
    pub selected: usize,
    pub configuring: bool,
    pub size: BoardSize,
    keyboard: bool,
    pan: BoardPan,
    pinch: PinchZoom,
    touch_id: Option<u64>,
}
impl MinesPage {
    pub fn new(seed: u64) -> Self {
        Self {
            game: Minesweeper::new(seed),
            flag_mode: false,
            selected: 0,
            configuring: true,
            size: BoardSize::Small,
            keyboard: false,
            pan: BoardPan::default(),
            pinch: PinchZoom::default(),
            touch_id: None,
        }
    }
    pub fn zoom_percent(&self) -> u16 {
        (self.pan.zoom * 100.).round() as u16
    }

    pub fn cancel_gesture(&mut self) {
        self.pinch = PinchZoom::default();
        self.pan.cancel();
        self.touch_id = None;
    }
    pub fn choose_size(&mut self) {
        self.configuring = true;
        self.cancel_gesture();
    }
    pub fn needs_frame(&self) -> bool {
        self.pan.active() && self.touch_id.is_none()
    }
    fn start(&mut self) {
        self.game = Minesweeper::with_size(super::seed(), self.size);
        self.configuring = false;
        self.flag_mode = false;
        self.selected = 0;
        self.keyboard = false;
        self.pan = BoardPan::default();
        let layout = MinesLayout::new(screen_width(), screen_height());
        // Small boards use the available height instead of floating in empty space.
        self.pan.zoom =
            (layout.viewport.h / (self.game.height() as f32 * layout.tile_size)).clamp(1.0, 1.5);
        self.pinch = PinchZoom::default();
        self.touch_id = None;
    }
    pub fn announcement(&self) -> String {
        if self.configuring {
            let (w, h, mines) = self.size.dimensions();
            return format!(
                "Jarcade. Minesweeper. Choose board size. {} selected. {w} by {h}, {mines} mines. Start to play.",
                self.size.name()
            );
        }
        let tile = self.game.tiles()[self.selected];
        let tile_text = if tile.mine && self.game.finished() {
            "Mine.".into()
        } else if tile.flagged {
            "Flagged tile.".into()
        } else if tile.revealed {
            format!("{} adjacent mines.", tile.adjacent)
        } else {
            "Covered tile.".into()
        };
        format!(
            "Jarcade. Minesweeper. {:?}. {} by {}. Zoom {} percent. {} mines left. {} of {} tiles cleared. {} mode. Row {}, column {}. {tile_text}",
            self.game.status(),
            self.game.width(),
            self.game.height(),
            self.zoom_percent(),
            self.game.mine_count() as isize - self.game.flags() as isize,
            self.game.revealed(),
            self.game.safe_count(),
            if self.flag_mode { "Flag" } else { "Reveal" },
            self.selected / self.game.width() + 1,
            self.selected % self.game.width() + 1
        )
    }
    fn setup(&mut self, ui: &mut Ui) {
        let width = (screen_width() - 40.).min(560.);
        let x = (screen_width() - width) / 2.;
        let short = screen_height() < 500.;
        let wide = short && width >= 480.;
        ui.heading("Board size", x, 110., 28., ui.theme.text);
        for (i, size) in BoardSize::ALL.into_iter().enumerate() {
            let card = if wide {
                Rect::new(
                    x + i as f32 * (width + 12.) / 3.,
                    132.,
                    (width - 24.) / 3.,
                    80.,
                )
            } else {
                Rect::new(
                    x,
                    136. + i as f32 * if short { 56. } else { 82. },
                    width,
                    if short { 48. } else { 72. },
                )
            };
            let active = size == self.size;
            bordered(
                card,
                18.,
                if active {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                if active && !ui.theme.saver {
                    color_u8!(239, 247, 234, 255)
                } else {
                    ui.theme.bg
                },
            );
            ui.heading(
                size.name(),
                card.x + 16.,
                card.y + if short && !wide { 20. } else { 27. },
                18.,
                ui.theme.text,
            );
            let (w, h, mines) = size.dimensions();
            ui.label(
                &format!("{w} × {h} · {mines} mines"),
                card.x + 16.,
                card.y + if short && !wide { 38. } else { 51. },
                if wide { 10. } else { 13. },
                ui.theme.muted,
            );
            let center = vec2(card.right() - 24., card.y + 24.);
            draw_circle_lines(
                center.x,
                center.y,
                8.,
                1.5,
                if active {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
            );
            if active {
                draw_circle(center.x, center.y, 4., ui.theme.accent);
            }
            if ui.hit(card) {
                self.size = size;
            }
        }
        if ui.button(
            "Start game",
            Rect::new(
                x,
                if wide {
                    228.
                } else if short {
                    (screen_height() - 60.).min(312.)
                } else {
                    400.
                },
                width,
                52.,
            ),
            true,
        ) {
            self.start();
            ui.reset_focus();
        }
    }
    fn input(&mut self, ui: &mut Ui, layout: &MinesLayout, base: Vec2) -> bool {
        let viewport = layout.viewport;
        let mut tap = None;
        let touches = touches();
        let active: Vec<_> = touches
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .map(|t| (t.id, touch_point(t.position, screen_dpi_scale())))
            .collect();
        let suppress = self.pinch.update(&active, &mut self.pan, viewport, base);
        let content = base * self.pan.zoom;
        let unit = layout.tile_size * self.pan.zoom;
        self.pan.clamp(viewport, content);
        if suppress {
            self.pan.cancel();
            self.touch_id = None;
        } else {
            for touch in &touches {
                let p = touch_point(touch.position, screen_dpi_scale());
                match touch.phase {
                    TouchPhase::Started if viewport.contains(p) => {
                        self.touch_id = Some(touch.id);
                        self.pan.begin(p);
                        self.keyboard = false;
                    }
                    TouchPhase::Moved if self.touch_id == Some(touch.id) => {
                        self.pan.update(p, viewport, content)
                    }
                    TouchPhase::Ended if self.touch_id == Some(touch.id) => {
                        tap = self.pan.end(p, viewport, content);
                        self.touch_id = None;
                    }
                    TouchPhase::Cancelled => self.cancel_gesture(),
                    _ => {}
                }
            }
        }
        let (mx, my) = mouse_position();
        let mouse = vec2(mx, my);
        if !suppress && self.touch_id.is_none() && touches.is_empty() {
            if is_mouse_button_pressed(MouseButton::Left) && viewport.contains(mouse) {
                self.pan.begin(mouse);
                self.keyboard = false;
            }
            if self.pan.active() {
                if is_mouse_button_released(MouseButton::Left) {
                    tap = self.pan.end(mouse, viewport, content);
                } else if is_mouse_button_down(MouseButton::Left) {
                    self.pan.update(mouse, viewport, content);
                } else {
                    self.pan.cancel();
                }
            }
        }
        if viewport.contains(mouse) {
            let (wx, wy) = mouse_wheel();
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
            if delta != Vec2::ZERO {
                self.pan.scroll(delta * scale, viewport, content);
                self.keyboard = false;
            }
        }
        let mut changed = false;
        let right = is_mouse_button_pressed(MouseButton::Right);
        if let Some(point) = tap.or_else(|| right.then_some(mouse))
            && let Some(index) =
                self.pan
                    .cell_at(point, viewport, self.game.width(), self.game.height(), unit)
        {
            self.selected = index;
            self.keyboard = false;
            changed = if right || self.flag_mode {
                self.game.toggle_flag(index)
            } else {
                self.game.reveal(index)
            };
        }
        for (key, dx, dy) in [
            (KeyCode::Left, -1, 0),
            (KeyCode::Right, 1, 0),
            (KeyCode::Up, 0, -1),
            (KeyCode::Down, 0, 1),
        ] {
            if is_key_pressed(key) {
                let w = self.game.width();
                let h = self.game.height();
                let x = (self.selected % w) as i32;
                let y = (self.selected / w) as i32;
                self.selected = (y + dy).clamp(0, h as i32 - 1) as usize * w
                    + (x + dx).clamp(0, w as i32 - 1) as usize;
                self.keyboard = true;
                ui.reset_focus();
                self.pan
                    .show_cell(self.selected, w, unit, viewport, content);
            }
        }
        if !ui.keyboard_focus
            && (is_key_pressed(KeyCode::F)
                || is_key_pressed(KeyCode::Enter)
                || is_key_pressed(KeyCode::Space))
        {
            self.keyboard = true;
            changed |= if is_key_pressed(KeyCode::F) || self.flag_mode {
                self.game.toggle_flag(self.selected)
            } else {
                self.game.reveal(self.selected)
            };
            self.pan
                .show_cell(self.selected, self.game.width(), unit, viewport, content);
        }
        changed
    }
    fn selector(&mut self, ui: &mut Ui, rect: Rect) {
        bordered(rect, 20., ui.theme.line, ui.theme.panel);
        for (i, label) in ["Reveal", "Flag"].into_iter().enumerate() {
            let segment = Rect::new(
                rect.x + 5. + i as f32 * (rect.w - 10.) / 2.,
                rect.y + 5.,
                (rect.w - 10.) / 2.,
                rect.h - 10.,
            );
            let active = self.flag_mode == (i == 1);
            let accent = if i == 1 && !ui.theme.saver {
                CORAL
            } else {
                ui.theme.accent
            };
            if active {
                bordered(
                    segment,
                    15.,
                    accent,
                    if ui.theme.saver {
                        BLACK
                    } else if i == 1 {
                        CORAL
                    } else {
                        GREEN
                    },
                );
            }
            let color = if active {
                if ui.theme.saver { accent } else { WHITE }
            } else {
                ui.theme.muted
            };
            let center = segment.center();
            let icon = vec2(center.x - if rect.w < 260. { 27. } else { 35. }, center.y);
            if i == 1 {
                flag(icon, 20., color);
            } else {
                draw_circle_lines(icon.x, icon.y, 6., 1.7, color);
                draw_line(
                    icon.x + 4.,
                    icon.y + 4.,
                    icon.x + 10.,
                    icon.y + 10.,
                    2.,
                    color,
                );
            }
            ui.centered(
                label,
                Rect::new(
                    center.x - 15.,
                    segment.y,
                    if rect.w < 260. { 48. } else { 65. },
                    segment.h,
                ),
                if rect.w < 260. { 13. } else { 16. },
                color,
                true,
            );
            if ui.hit(segment) {
                self.flag_mode = i == 1;
            }
        }
    }
    fn zoom_controls(&mut self, ui: &mut Ui, layout: &MinesLayout, base: Vec2) {
        let r = layout.zoom_controls;
        let minus = ui.button("−", Rect::new(r.x, r.y, 44., 44.), false);
        let fit = ui.button("Fit", Rect::new(r.x + 48., r.y, 54., 44.), false);
        let plus = ui.button("+", Rect::new(r.x + 106., r.y, 44., 44.), false);
        let decrease =
            minus || is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract);
        let increase = plus || is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd);
        if decrease || increase {
            self.cancel_gesture();
            self.pan.zoom_at(
                layout.viewport.center(),
                self.pan.zoom * if increase { 1.25 } else { 0.8 },
                layout.viewport,
                base,
            );
        }
        if fit || is_key_pressed(KeyCode::Key0) {
            self.cancel_gesture();
            self.pan.fit(layout.viewport, base);
        }
    }
    pub fn draw(&mut self, ui: &mut Ui) -> Option<Pulse> {
        if self.configuring {
            self.setup(ui);
            return None;
        }
        let l = MinesLayout::new(screen_width(), screen_height());
        let base = vec2(self.game.width() as f32, self.game.height() as f32) * l.tile_size;
        self.zoom_controls(ui, &l, base);
        let changed = self.input(ui, &l, base);
        let content = base * self.pan.zoom;
        let board = self.pan.board(l.viewport, content);
        bordered(
            Rect::new(
                l.viewport.x - 6.,
                l.viewport.y - 6.,
                l.viewport.w + 12.,
                l.viewport.h + 12.,
            ),
            16.,
            ui.theme.line,
            ui.theme.bg,
        );
        let dpi = screen_dpi_scale();
        // Keep board clipping isolated from the surrounding controls.
        // SAFETY: flush and set the clip synchronously on the render thread.
        unsafe {
            get_internal_gl().flush();
            get_internal_gl().quad_gl.scissor(Some((
                (l.viewport.x * dpi) as i32,
                (l.viewport.y * dpi) as i32,
                (l.viewport.w * dpi) as i32,
                (l.viewport.h * dpi) as i32,
            )));
        }
        draw_board(
            ui,
            board,
            &self.game,
            &ui.theme,
            self.keyboard.then_some(self.selected),
            Some(l.viewport),
        );
        unsafe {
            get_internal_gl().flush();
            get_internal_gl().quad_gl.scissor(None);
        }
        // Persistent scroll thumbs make overflow visible without an animation loop.
        for vertical in [false, true] {
            let (length, total, offset) = if vertical {
                (l.viewport.h, content.y, self.pan.offset.y)
            } else {
                (l.viewport.w, content.x, self.pan.offset.x)
            };
            if total > length {
                let thumb = (length * length / total).max(24.);
                let start = offset / (total - length) * (length - thumb);
                let bar = if vertical {
                    Rect::new(l.viewport.right() + 2., l.viewport.y + start, 3., thumb)
                } else {
                    Rect::new(l.viewport.x + start, l.viewport.bottom() + 2., thumb, 3.)
                };
                rounded(bar, 1.5, ui.theme.muted);
            }
        }
        let stats = l.stats;
        if self.game.finished() {
            let won = self.game.status() == Status::Won;
            ui.centered(
                if won { "Cleared!" } else { "Mine hit" },
                stats,
                18.,
                if won { ui.theme.accent } else { CORAL },
                true,
            );
            if ui.button("New board", l.tools, true) {
                self.choose_size();
                ui.reset_focus();
            }
        } else {
            ui.centered(
                &format!(
                    "{} left",
                    self.game.mine_count() as isize - self.game.flags() as isize
                ),
                Rect::new(stats.x, stats.y, stats.w - 62., stats.h),
                17.,
                ui.theme.text,
                true,
            );
            ui.centered(
                &format!("{}/{}", self.game.revealed(), self.game.safe_count()),
                Rect::new(stats.right() - 60., stats.y, 60., stats.h),
                12.,
                ui.theme.muted,
                false,
            );
            self.selector(ui, l.tools);
        }
        if changed {
            Some(match self.game.status() {
                Status::Won => Pulse::Won,
                Status::Lost => Pulse::Lost,
                _ => Pulse::Tap,
            })
        } else {
            None
        }
    }
}

pub struct MinesPreview {
    target: RenderTarget,
}
impl MinesPreview {
    pub fn new(ui: &Ui, saver: bool) -> Self {
        let target = render_target(1024, 1024);
        target.texture.set_filter(FilterMode::Linear);
        set_camera(&Camera2D {
            render_target: Some(target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0., 0., 1024., 1024.))
        });
        clear_background(Color::new(0., 0., 0., 0.));
        let mut game = Minesweeper::new(2026);
        game.reveal(40);
        let flags: Vec<_> = game
            .tiles()
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.mine.then_some(i))
            .take(3)
            .collect();
        for i in flags {
            game.toggle_flag(i);
        }
        draw_board(
            ui,
            Rect::new(32., 32., 960., 960.),
            &game,
            &Theme::new(saver),
            None,
            None,
        );
        set_default_camera();
        Self { target }
    }
    pub fn draw(&self, rect: Rect) {
        draw_texture_ex(
            &self.target.texture,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                flip_y: true,
                ..Default::default()
            },
        );
    }
}
