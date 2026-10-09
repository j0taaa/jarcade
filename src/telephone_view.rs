//! A shared vector sketchbook: no DOM drawing surface or bitmap assets.
use crate::{
    online_style::primary,
    online_view::{clip, fit, wrap, wrapped_lines},
    ui::{Ui, bordered, rounded},
};
use jarcade::{
    layout::touch_point,
    multiplayer::{
        Command, RoomView,
        telephone::{
            Content, Drawing, MAX_POINTS, MAX_STROKES, Move, PALETTE, Phase, Stroke, View,
        },
    },
};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Draft {
    pub key: String,
    pub drawing: Drawing,
    pub text: String,
}
#[derive(Default)]
pub struct Outcome {
    pub command: Option<Command>,
    pub editor: Option<Rect>,
}
#[derive(Default)]
pub struct TelephoneUi {
    pub drawing: Drawing,
    history: Vec<Drawing>,
    redo: Vec<Drawing>,
    color: u8,
    width: u8,
    gesture: Option<Option<u64>>,
    canvas: Option<Rect>,
    blocked: bool,
    dirty: bool,
    confirm: bool,
}
impl TelephoneUi {
    pub fn new() -> Self {
        Self {
            width: 6,
            ..Self::default()
        }
    }
    pub fn active(&self) -> bool {
        self.gesture.is_some()
    }
    pub fn cancel(&mut self) {
        self.gesture = None;
        self.blocked = true;
    }
    pub fn restore(&mut self, draft: Option<&Draft>) {
        *self = Self::new();
        if let Some(d) = draft
            && (d.drawing.strokes.is_empty() || d.drawing.validate().is_ok())
        {
            self.drawing = d.drawing.clone();
        }
    }
    pub fn key(room: &RoomView) -> String {
        format!(
            "{}:{}:{}",
            room.code,
            room.you,
            room.telephone.as_ref().map_or(0, |g| g.stage)
        )
    }
    pub fn take_dirty(&mut self) -> bool {
        if self.active() {
            return false;
        }
        std::mem::take(&mut self.dirty)
    }
    fn remember(&mut self) {
        if self.history.len() >= 40 {
            self.history.remove(0);
        }
        self.history.push(self.drawing.clone());
        self.redo.clear();
        self.dirty = true;
    }
    fn undo(&mut self) {
        if let Some(d) = self.history.pop() {
            self.redo.push(std::mem::replace(&mut self.drawing, d));
            self.dirty = true;
        }
    }
    fn redo(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.history.push(std::mem::replace(&mut self.drawing, d));
            self.dirty = true;
        }
    }
    fn point(rect: Rect, point: Vec2) -> [u16; 2] {
        [
            ((point.x - rect.x) / rect.w * 1000.)
                .clamp(0., 1000.)
                .round() as u16,
            ((point.y - rect.y) / rect.h * 750.).clamp(0., 750.).round() as u16,
        ]
    }
    fn input(&mut self, rect: Rect, press: Option<Vec2>) {
        let events = touches();
        let live: Vec<_> = events
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .collect();
        if self.canvas != Some(rect) {
            self.cancel();
            self.canvas = Some(rect);
        }
        if live.len() > 1 || events.iter().any(|t| t.phase == TouchPhase::Cancelled) {
            self.cancel();
        }
        if self.blocked {
            if live.is_empty() && !is_mouse_button_down(MouseButton::Left) {
                self.blocked = false;
            }
            return;
        }
        if self.gesture.is_none()
            && let Some(p) = press
            && rect.contains(p)
            && self.drawing.strokes.len() < MAX_STROKES
            && self
                .drawing
                .strokes
                .iter()
                .map(|s| s.points.len())
                .sum::<usize>()
                < MAX_POINTS
        {
            self.remember();
            self.drawing.strokes.push(Stroke {
                color: self.color,
                width: self.width,
                points: vec![Self::point(rect, p)],
            });
            self.gesture = Some(live.first().map(|t| t.id));
        }
        if let Some(owner) = self.gesture {
            let (point, ended) = if let Some(id) = owner {
                match events.iter().find(|t| t.id == id) {
                    Some(t) => (
                        touch_point(t.position, screen_dpi_scale()),
                        matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled),
                    ),
                    None => {
                        self.gesture = None;
                        return;
                    }
                }
            } else {
                let (x, y) = mouse_position();
                (vec2(x, y), !is_mouse_button_down(MouseButton::Left))
            };
            let p = Self::point(rect, point);
            let count = self
                .drawing
                .strokes
                .iter()
                .map(|s| s.points.len())
                .sum::<usize>();
            if let Some(s) = self.drawing.strokes.last_mut() {
                let last = *s.points.last().unwrap();
                if count < MAX_POINTS
                    && (i32::from(p[0]) - i32::from(last[0])).pow(2)
                        + (i32::from(p[1]) - i32::from(last[1])).pow(2)
                        >= 4
                {
                    s.points.push(p);
                    self.dirty = true;
                }
            }
            if ended {
                self.gesture = None;
            }
        }
    }
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, miniquad::KeyMods, bool)],
        room: &RoomView,
        text: (&str, bool),
        bounds: Rect,
    ) -> Outcome {
        let mut out = Outcome::default();
        let Some(game) = room.telephone.as_ref() else {
            return out;
        };
        if game.phase != Phase::Play {
            self.reveal(ui, room, game, bounds, &mut out);
            return out;
        }
        if game.submitted[room.you] {
            self.waiting(ui, room, game, bounds);
            return out;
        }
        let draw = !game.stage.is_multiple_of(2);
        if self.confirm {
            let area = Rect::new(bounds.x, bounds.y + 72., bounds.w, bounds.h - 132.);
            if draw {
                let r = canvas_rect(area);
                paper(ui, r);
                render(&self.drawing, r);
            }
            self.cancel();
            let r = Rect::new(
                bounds.center().x - bounds.w.min(420.) / 2.,
                bounds.center().y - 88.,
                bounds.w.min(420.),
                176.,
            );
            // A solid sheet keeps the work underneath private while confirming.
            bordered(r, 20., ui.theme.accent, ui.theme.bg);
            ui.centered(
                "Ready to pass it on?",
                Rect::new(r.x + 8., r.y + 16., r.w - 16., 30.),
                19.,
                ui.theme.text,
                true,
            );
            ui.centered(
                "Your contribution will be sealed.",
                Rect::new(r.x + 8., r.y + 52., r.w - 16., 26.),
                12.,
                ui.theme.muted,
                false,
            );
            if ui.button(
                "Keep editing",
                Rect::new(r.x + 12., r.y + 110., (r.w - 36.) / 2., 48.),
                false,
            ) {
                self.confirm = false;
            }
            if primary(
                ui,
                "Pass it on",
                Rect::new(r.center().x + 6., r.y + 110., (r.w - 36.) / 2., 48.),
                true,
            ) {
                out.command = Some(Command::Telephone(if draw {
                    Move::Draw {
                        drawing: self.drawing.clone(),
                    }
                } else {
                    Move::Text {
                        text: text.0.into(),
                    }
                }));
                self.confirm = false;
            }
            return out;
        }
        let landscape = bounds.w >= 480. && bounds.h < 480.;
        let work = if landscape {
            Rect::new(bounds.x, bounds.y, bounds.w - 196., bounds.h)
        } else {
            bounds
        };
        fit(
            ui,
            &format!("TURN {} / {}", game.stage + 1, game.rounds),
            Rect::new(work.x, work.y, work.w, 18.),
            11.,
            ui.theme.muted,
            true,
        );
        let title = if draw {
            "Draw this"
        } else if game.stage == 0 {
            "Start a little chaos"
        } else {
            "What do you see?"
        };
        fit(
            ui,
            title,
            Rect::new(work.x, work.y + 24., work.w, 30.),
            24.,
            ui.theme.text,
            true,
        );
        if draw {
            let prompt = match &game.task {
                Some(Content::Text(t)) => t.as_str(),
                _ => "The previous player left. Draw a wonderful creature!",
            };
            let prompt_h = wrapped_lines(ui, prompt, work.w, 14.).len() as f32 * 23.;
            let title_h = 59. + prompt_h + 8.;
            wrap(
                ui,
                prompt,
                Rect::new(work.x, work.y + 59., work.w, prompt_h),
                14.,
                ui.theme.accent,
                true,
            );
            let tool_h = if bounds.w < 352. { 204. } else { 160. };
            let tools_y = bounds.bottom() - tool_h;
            let area = Rect::new(
                work.x,
                work.y + title_h,
                work.w,
                (if landscape {
                    work.h - title_h - 6.
                } else {
                    tools_y - work.y - title_h - 10.
                })
                .max(32.),
            );
            let canvas = canvas_rect(area);
            if !self.confirm {
                self.input(canvas, press);
            }
            paper(ui, canvas);
            render(&self.drawing, canvas);
            if self.drawing.strokes.is_empty() {
                pencil(canvas.center() - vec2(0., 13.), 20., ui.theme.muted);
                ui.centered(
                    "Make your mark",
                    Rect::new(canvas.x, canvas.center().y + 16., canvas.w, 22.),
                    13.,
                    ui.theme.muted,
                    false,
                );
            }
            let tool_bounds = if landscape {
                Rect::new(bounds.right() - 180., bounds.y, 180., bounds.h)
            } else {
                Rect::new(bounds.x, tools_y, bounds.w, tool_h)
            };
            self.tools(ui, tool_bounds, landscape);
            let submit = Rect::new(
                tool_bounds.x,
                tool_bounds.bottom() - 48.,
                tool_bounds.w,
                48.,
            );
            if primary(
                ui,
                "Seal drawing",
                submit,
                self.drawing.validate().is_ok() && !self.active(),
            ) {
                self.confirm = true;
            }
            for &(key, mods, repeat) in keys {
                if key == KeyCode::Z && (mods.ctrl || mods.logo) && !repeat {
                    if mods.shift {
                        self.redo();
                    } else {
                        self.undo();
                    }
                }
            }
        } else {
            let field = if landscape {
                Rect::new(bounds.right() - 180., bounds.y + 38., 180., 76.)
            } else {
                Rect::new(bounds.x, bounds.bottom() - 144., bounds.w, 76.)
            };
            let area = Rect::new(
                work.x,
                work.y + 68.,
                work.w,
                (if landscape {
                    work.h - 80.
                } else {
                    field.y - work.y - 90.
                })
                .max(40.),
            );
            if let Some(Content::Drawing(d)) = &game.task {
                let r = canvas_rect(area);
                paper(ui, r);
                render(d, r);
            } else if game.stage == 0 {
                let size = area.w.min(area.h).min(270.);
                let r = Rect::new(
                    area.center().x - size / 2.,
                    area.center().y - size / 2.,
                    size,
                    size,
                );
                preview(ui, r);
            } else {
                ui.centered(
                    "A missing sketch… imagine your own!",
                    area,
                    13.,
                    ui.theme.muted,
                    false,
                );
            }
            bordered(
                field,
                16.,
                if text.1 {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                ui.theme.panel,
            );
            wrap(
                ui,
                if text.0.is_empty() {
                    "Write a short sentence…"
                } else {
                    text.0
                },
                Rect::new(field.x + 14., field.y + 10., field.w - 28., field.h - 16.),
                16.,
                if text.0.is_empty() {
                    ui.theme.muted
                } else {
                    ui.theme.text
                },
                false,
            );
            if ui.hit(field) {
                out.editor = Some(field);
            }
            if text.1 {
                crate::platform::editor_position(7, field);
            }
            let r = Rect::new(field.x, bounds.bottom() - 48., field.w, 48.);
            if primary(ui, "Seal sentence", r, !text.0.trim().is_empty()) {
                self.confirm = true;
            }
        }

        out
    }
    fn tools(&mut self, ui: &mut Ui, b: Rect, landscape: bool) {
        // Swatches are full touch targets even when the visible ink dot is small.
        let cols = if landscape || b.w < 352. { 4 } else { 8 };
        let sw = (b.w / cols as f32).min(48.);
        let sx = b.x + (b.w - sw * cols as f32) / 2.;
        for (i, rgb) in PALETTE.iter().enumerate() {
            let r = Rect::new(
                sx + (i % cols) as f32 * sw,
                b.y + (i / cols) as f32 * 44.,
                sw,
                44.,
            );
            let color = Color::from_rgba(rgb[0], rgb[1], rgb[2], 255);
            draw_circle(r.center().x, r.center().y, 11., color);
            if i == 7 {
                draw_circle_lines(r.center().x, r.center().y, 11., 1., ui.theme.line);
            }
            if self.color as usize == i {
                draw_circle_lines(r.center().x, r.center().y, 16., 2., ui.theme.accent);
            }
            if ui.hit(r) {
                self.color = i as u8;
            }
        }
        let y = b.y + if cols == 4 { 88. } else { 48. };
        let cw = b.w / if landscape { 3. } else { 6. };
        for (i, width) in [3, 6, 12].into_iter().enumerate() {
            let r = Rect::new(b.x + i as f32 * cw, y, cw, 44.);
            if width == self.width {
                rounded(r, 10., ui.theme.panel);
                draw_rectangle_lines(r.x + 2., r.y + 2., r.w - 4., r.h - 4., 1., ui.theme.accent);
            }
            draw_circle(r.center().x, r.center().y, width as f32 / 2., ui.theme.text);
            if ui.hit(r) {
                self.width = width;
            }
        }
        for i in 0..3 {
            let r = if landscape {
                Rect::new(b.x + i as f32 * cw, y + 44., cw, 44.)
            } else {
                Rect::new(b.x + (i + 3) as f32 * cw, y, cw, 44.)
            };
            tool_icon(
                ui,
                r,
                i,
                match i {
                    0 => !self.history.is_empty(),
                    1 => !self.redo.is_empty(),
                    _ => !self.drawing.strokes.is_empty(),
                },
            );
            if ui.hit(r) && !self.active() {
                match i {
                    0 => self.undo(),
                    1 => self.redo(),
                    _ => {
                        self.remember();
                        self.drawing = Drawing::default();
                    }
                }
            }
        }
        if !landscape {
            fit(
                ui,
                if self.color == 7 {
                    "Eraser · undo / redo / clear"
                } else {
                    "Brush · undo / redo / clear"
                },
                Rect::new(b.x, y + 46., b.w, 16.),
                10.,
                ui.theme.muted,
                false,
            );
        }
        if self.drawing.strokes.len() >= MAX_STROKES
            || self
                .drawing
                .strokes
                .iter()
                .map(|s| s.points.len())
                .sum::<usize>()
                >= MAX_POINTS
        {
            fit(
                ui,
                "Sketch full · undo or seal",
                Rect::new(b.x, b.bottom() - 72., b.w, 20.),
                11.,
                ui.theme.accent,
                true,
            );
        }
    }
    fn waiting(&self, ui: &mut Ui, room: &RoomView, g: &View, b: Rect) {
        let w = b.w.min(520.);
        let x = b.center().x - w / 2.;
        let compact = b.h < 420.;
        let cols = if compact && b.w >= 480. { 4 } else { 2 };
        let header = if compact { 86. } else { 140. };
        let total = header + room.members.len().div_ceil(cols) as f32 * 42.;
        let y = b.y + ((b.h - total) / 2.).max(0.);
        if !compact {
            pencil(vec2(b.center().x, y + 26.), 23., ui.theme.accent);
        }
        ui.centered(
            "Sealed with a scribble",
            Rect::new(x, y + if compact { 32. } else { 68. }, w, 30.),
            21.,
            ui.theme.text,
            true,
        );
        ui.centered(
            "Waiting for your friends",
            Rect::new(x, y + if compact { 62. } else { 106. }, w, 22.),
            13.,
            ui.theme.muted,
            false,
        );
        for (i, m) in room.members.iter().enumerate() {
            let r = Rect::new(
                x + (i % cols) as f32 * w / cols as f32,
                y + header + (i / cols) as f32 * 42.,
                w / cols as f32 - 8.,
                34.,
            );
            fit(
                ui,
                &format!(
                    "{} {}",
                    if g.left[i] {
                        "—"
                    } else if g.submitted[i] {
                        "✓"
                    } else {
                        "○"
                    },
                    m.name
                ),
                r,
                13.,
                if g.submitted[i] {
                    ui.theme.accent
                } else {
                    ui.theme.muted
                },
                g.submitted[i],
            );
        }
    }
    fn reveal(&self, ui: &mut Ui, room: &RoomView, g: &View, b: Rect, out: &mut Outcome) {
        let landscape = b.w >= 480. && b.h < 480.;
        let info = if landscape {
            Rect::new(b.right() - 180., b.y, 180., b.h)
        } else {
            b
        };
        fit(
            ui,
            &format!("ALBUM {} / {}", g.album + 1, g.rounds),
            Rect::new(info.x, info.y, info.w, 18.),
            11.,
            ui.theme.muted,
            true,
        );
        fit(
            ui,
            &format!("{}’s story", room.members[g.owner].name),
            Rect::new(info.x, info.y + 24., info.w, 30.),
            23.,
            ui.theme.text,
            true,
        );
        let author = g
            .entry
            .as_ref()
            .map_or("", |e| room.members[e.author].name.as_str());
        fit(
            ui,
            &format!("{} / {} · {}", g.step + 1, g.rounds, author),
            Rect::new(info.x, info.y + 60., info.w, 20.),
            13.,
            ui.theme.accent,
            true,
        );
        let area = if landscape {
            Rect::new(b.x, b.y + 2., b.w - 196., b.h - 4.)
        } else {
            Rect::new(b.x, b.y + 96., b.w, (b.h - 164.).max(32.))
        };
        match g.entry.as_ref().map(|e| &e.content) {
            Some(Content::Drawing(d)) => {
                let r = canvas_rect(area);
                paper(ui, r);
                render(d, r);
            }
            Some(Content::Text(t)) => {
                let w = area.w.min(660.);
                let h = area.h.min(220.);
                let r = Rect::new(area.center().x - w / 2., area.center().y - h / 2., w, h);
                bordered(r, 22., ui.theme.line, ui.theme.panel);
                fitted_wrap(
                    ui,
                    &format!("“{t}”"),
                    Rect::new(r.x + 14., r.y + 12., r.w - 28., r.h - 24.),
                    22.,
                    ui.theme.text,
                    true,
                );
            }
            _ => {
                ui.centered(
                    "This player left a blank page",
                    area,
                    14.,
                    ui.theme.muted,
                    false,
                );
            }
        }
        let by = b.bottom() - 48.;
        if room.you == room.host {
            if ui.button(
                "Previous",
                if landscape {
                    Rect::new(info.x, by - 56., info.w, 48.)
                } else {
                    Rect::new(b.x, by, 100., 48.)
                },
                false,
            ) && g.album * g.rounds + g.step > 0
            {
                out.command = Some(Command::Telephone(Move::Previous));
            }
            let last = g.album + 1 == g.rounds && g.step + 1 == g.rounds;
            let label = if g.phase == Phase::Finished && last {
                "New round"
            } else if last {
                "That’s a wrap!"
            } else if g.step + 1 == g.rounds {
                "Next album"
            } else {
                "Next page"
            };
            if primary(
                ui,
                label,
                if landscape {
                    Rect::new(info.x, by, info.w, 48.)
                } else {
                    Rect::new(b.x + 112., by, b.w - 112., 48.)
                },
                true,
            ) {
                out.command = Some(if g.phase == Phase::Finished && last {
                    Command::Rematch
                } else {
                    Command::Telephone(Move::Next)
                });
            }
        } else {
            ui.centered(
                if g.phase == Phase::Finished {
                    "All stories revealed"
                } else {
                    "The host turns the pages"
                },
                Rect::new(info.x, by, info.w, 48.),
                13.,
                ui.theme.muted,
                false,
            );
        }
    }
}
fn fitted_wrap(ui: &Ui, text: &str, r: Rect, size: f32, color: Color, bold: bool) {
    let mut size = size;
    while size > 10. && wrapped_lines(ui, text, r.w, size).len() as f32 * (size + 9.) > r.h {
        size -= 1.;
    }
    wrap(ui, text, r, size, color, bold);
}
fn canvas_rect(area: Rect) -> Rect {
    let w = area.w.min(area.h * 4. / 3.).clamp(1., 900.);
    Rect::new(
        area.center().x - w / 2.,
        area.center().y - w * 0.375,
        w,
        w * 0.75,
    )
}
fn paper(ui: &Ui, r: Rect) {
    // White ink is the eraser. The paper alone stays white in OLED mode.
    bordered(
        Rect::new(r.x - 2., r.y - 2., r.w + 4., r.h + 4.),
        3.,
        ui.theme.line,
        WHITE,
    );
}
pub fn render(drawing: &Drawing, r: Rect) {
    clip(Some(r));
    for s in &drawing.strokes {
        let rgb = PALETTE[s.color as usize];
        let ink = Color::from_rgba(rgb[0], rgb[1], rgb[2], 255);
        let width = (s.width as f32 * r.w / 1000.).max(0.7);
        let point = |p: [u16; 2]| {
            vec2(
                r.x + p[0] as f32 * r.w / 1000.,
                r.y + p[1] as f32 * r.h / 750.,
            )
        };
        for pair in s.points.windows(2) {
            let a = point(pair[0]);
            let b = point(pair[1]);
            draw_line(a.x, a.y, b.x, b.y, width, ink);
        }
        for p in &s.points {
            let p = point(*p);
            draw_circle(p.x, p.y, width / 2., ink);
        }
    }
    clip(None);
}
fn tool_icon(ui: &Ui, r: Rect, index: usize, enabled: bool) {
    let c = r.center();
    let ink = if enabled {
        ui.theme.text
    } else {
        ui.theme.line
    };
    if index < 2 {
        let sign = if index == 0 { -1. } else { 1. };
        draw_line(
            c.x - sign * 7.,
            c.y + 7.,
            c.x - sign * 7.,
            c.y - 5.,
            2.,
            ink,
        );
        draw_line(
            c.x - sign * 7.,
            c.y - 5.,
            c.x + sign * 7.,
            c.y - 5.,
            2.,
            ink,
        );
        draw_line(
            c.x + sign * 7.,
            c.y - 5.,
            c.x + sign * 1.,
            c.y - 11.,
            2.,
            ink,
        );
        draw_line(
            c.x + sign * 7.,
            c.y - 5.,
            c.x + sign * 1.,
            c.y + 1.,
            2.,
            ink,
        );
    } else {
        draw_rectangle_lines(c.x - 6., c.y - 5., 12., 15., 1.5, ink);
        draw_line(c.x - 9., c.y - 8., c.x + 9., c.y - 8., 2., ink);
        draw_line(c.x - 3., c.y - 11., c.x + 3., c.y - 11., 2., ink);
    }
}
pub fn pencil(c: Vec2, r: f32, ink: Color) {
    let a = c + vec2(-r * 0.55, r * 0.55);
    let b = c + vec2(r * 0.65, -r * 0.65);
    draw_line(a.x, a.y, b.x, b.y, r * 0.4, ink);
    draw_triangle(
        a + vec2(-r * 0.18, 0.),
        a + vec2(0., r * 0.18),
        a - vec2(r * 0.33, -r * 0.33),
        ink,
    );
    draw_circle(b.x, b.y, r * 0.2, ink);
}
pub fn preview(ui: &Ui, r: Rect) {
    let s = r.w / 200.;
    let at = |x: f32, y: f32| r.point() + vec2(x * s, y * s);
    let sheet = Rect::new(r.x + 18. * s, r.y + 26. * s, 154. * s, 122. * s);
    bordered(sheet, 9. * s, ui.theme.line, ui.theme.bg);
    let ink = ui.theme.accent;
    // A real chain: the written prompt, the sketch, then a transformed caption.
    let cat = at(95., 91.);
    draw_circle_lines(cat.x, cat.y, 25. * s, 2.5 * s, ink);
    for sign in [-1., 1.] {
        let p = cat + vec2(sign * 17. * s, -18. * s);
        draw_line(p.x, p.y, p.x + sign * 4. * s, p.y - 17. * s, 2.5 * s, ink);
        draw_line(
            p.x + sign * 4. * s,
            p.y - 17. * s,
            p.x - sign * 9. * s,
            p.y - 4. * s,
            2.5 * s,
            ink,
        );
        draw_circle(cat.x + sign * 9. * s, cat.y - 3. * s, 2. * s, ink);
        for dy in [-1., 1.] {
            draw_line(
                cat.x + sign * 20. * s,
                cat.y + 8. * s,
                cat.x + sign * 34. * s,
                cat.y + (8. + dy * 5.) * s,
                2. * s,
                ink,
            );
        }
    }
    draw_line(
        cat.x - 5. * s,
        cat.y + 13. * s,
        cat.x,
        cat.y + 17. * s,
        2. * s,
        ink,
    );
    draw_line(
        cat.x,
        cat.y + 17. * s,
        cat.x + 5. * s,
        cat.y + 13. * s,
        2. * s,
        ink,
    );
    let bubble = Rect::new(r.x + 4. * s, r.y + 4. * s, 98. * s, 34. * s);
    bordered(bubble, 10. * s, ui.theme.line, ui.theme.bg);
    ui.centered("Space cat", bubble, 11. * s, ui.theme.text, true);
    let caption = Rect::new(r.x + 70. * s, r.y + 140. * s, 124. * s, 36. * s);
    bordered(caption, 11. * s, ui.theme.line, ui.theme.panel);
    ui.centered("Moon whiskers", caption, 11. * s, ui.theme.accent, true);
    pencil(at(156., 107.), 24. * s, ink);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn undo_redo_and_clear_preserve_vector_work() {
        let mut ui = TelephoneUi::new();
        ui.remember();
        ui.drawing.strokes.push(Stroke {
            color: 1,
            width: 6,
            points: vec![[200, 300]],
        });
        let d = ui.drawing.clone();
        ui.remember();
        ui.drawing = Drawing::default();
        ui.undo();
        assert_eq!(ui.drawing, d);
        ui.undo();
        assert_eq!(ui.drawing, Drawing::default());
        ui.redo();
        assert_eq!(ui.drawing, d);
        ui.redo();
        assert_eq!(ui.drawing, Drawing::default());
        ui.undo();
        ui.remember();
        assert!(ui.redo.is_empty());
    }
    #[test]
    fn canvas_fits_phones_tablets_and_landscape() {
        for (w, h) in [
            (288., 486.),
            (358., 762.),
            (536., 238.),
            (788., 1098.),
            (1408., 818.),
        ] {
            let r = canvas_rect(Rect::new(16., 66., w, h));
            assert!(r.x >= 16. && r.y >= 66. && r.right() <= w + 16.01 && r.bottom() <= h + 66.01);
            assert_eq!(TelephoneUi::point(r, r.point()), [0, 0]);
            assert_eq!(TelephoneUi::point(r, r.point() + r.size()), [1000, 750]);
        }
    }
}
