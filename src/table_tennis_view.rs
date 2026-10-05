use crate::ui::{CORAL, Icon, Theme, Ui, bordered, rounded};
use jarcade::{
    feedback::Pulse,
    layout::{TableLayout, touch_point},
    table_tennis::{Difficulty, Event, Phase, RACKET_RADIUS, Side, TableTennis},
};
use macroquad::prelude::*;

const BLUE: Color = color_u8!(43, 94, 181, 255);
const TABLE: Color = color_u8!(34, 116, 131, 255);

pub struct TennisPage {
    pub game: TableTennis,
    last: Option<f64>,
    finger: Option<u64>,
    touch_mode: bool,
    mouse: Vec2,
    size: Vec2,
    hit: f64,
}
impl TennisPage {
    pub fn new(seed: u64) -> Self {
        Self {
            game: TableTennis::new(seed, Difficulty::Normal),
            last: None,
            finger: None,
            touch_mode: false,
            mouse: Vec2::ZERO,
            size: Vec2::ZERO,
            hit: -10.,
        }
    }
    pub fn enter(&mut self, seed: u64) {
        let difficulty = self.game.difficulty;
        *self = Self::new(seed);
        self.game.difficulty = difficulty;
        self.mouse = Vec2::from(mouse_position());
    }
    pub fn interrupt(&mut self) {
        self.game.pause();
        self.finger = None;
        self.last = None;
        self.mouse = Vec2::from(mouse_position());
    }
    pub fn needs_frame(&self) -> bool {
        self.game.phase == Phase::Rally
    }
    fn play(&mut self, now: f64) {
        match self.game.phase {
            Phase::Paused => self.game.resume(),
            Phase::Rally => self.game.pause(),
            Phase::Finished => {
                self.enter((now * 1_000_000.) as u64);
                self.game.serve();
            }
            Phase::Ready | Phase::Point => self.game.serve(),
        }
        self.last = Some(now);
        self.finger = None;
        self.mouse = Vec2::from(mouse_position());
    }
    fn controls(
        &mut self,
        l: &TableLayout,
        press: Option<Vec2>,
        keys: &[(KeyCode, miniquad::KeyMods, bool)],
        dt: f32,
    ) {
        let touches = touches();
        let live = touches
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled));
        if !touches.is_empty() {
            self.touch_mode = true;
        }
        if live.clone().count() > 1 || touches.iter().any(|t| t.phase == TouchPhase::Cancelled) {
            self.finger = None;
        } else if self.finger.is_none()
            && let Some(t) = live
                .clone()
                .next()
                .filter(|t| t.phase == TouchPhase::Started)
        {
            let p = touch_point(t.position, screen_dpi_scale());
            if l.table.contains(p) {
                self.finger = Some(t.id);
            }
        }
        if let Some(id) = self.finger {
            if let Some(t) = live.clone().find(|t| t.id == id) {
                self.game
                    .move_player(l.racket_local(touch_point(t.position, screen_dpi_scale()), true));
            } else {
                self.finger = None;
            }
        }
        let mouse = Vec2::from(mouse_position());
        if mouse.distance_squared(self.mouse) > 0.01 && touches.is_empty() {
            self.touch_mode = false;
            if l.table.contains(mouse) {
                self.game.move_player(l.racket_local(mouse, false));
            }
        }
        // A quick mouse click can arrive between frames without a motion event.
        if !self.touch_mode
            && let Some(p) = press.filter(|p| l.table.contains(*p))
        {
            self.game.move_player(l.racket_local(p, false));
        }
        self.mouse = mouse;
        let mut direction = Vec2::ZERO;
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            direction.x -= 1.;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            direction.x += 1.;
        }
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            direction.y -= 1.;
        }
        if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            direction.y += 1.;
        }
        if l.landscape {
            direction = vec2(-direction.y, direction.x);
        }
        self.game
            .move_player(self.game.player + direction * dt * 1.25);
        // Preserve taps whose key-down and key-up share a display frame.
        for &(key, _, repeat) in keys {
            if repeat {
                continue;
            }
            let mut step = match key {
                KeyCode::Left | KeyCode::A => vec2(-0.025, 0.),
                KeyCode::Right | KeyCode::D => vec2(0.025, 0.),
                KeyCode::Up | KeyCode::W => vec2(0., -0.025),
                KeyCode::Down | KeyCode::S => vec2(0., 0.025),
                _ => Vec2::ZERO,
            };
            if l.landscape {
                step = vec2(-step.y, step.x);
            }
            self.game.move_player(self.game.player + step);
        }
    }
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, miniquad::KeyMods, bool)],
        now: f64,
    ) -> (bool, Option<Pulse>) {
        let size = vec2(screen_width(), screen_height());
        if self.size != Vec2::ZERO && self.size != size {
            self.interrupt();
        }
        self.size = size;
        let l = TableLayout::new(size.x, size.y);
        let mut dt = self.last.replace(now).map_or(0., |last| now - last);
        for &(key, _, repeat) in keys {
            if !repeat && matches!(key, KeyCode::Space | KeyCode::P) {
                self.play(now);
                dt = 0.;
                break;
            }
        }
        if self.game.phase == Phase::Rally && (0.0..=0.25).contains(&dt) {
            self.controls(&l, press, keys, dt as f32);
        }
        let pulse = match self.game.advance(dt) {
            Some(Event::Return) => {
                self.hit = now;
                Some(Pulse::Tap)
            }
            Some(Event::Point(Side::You)) => Some(Pulse::Eat),
            Some(Event::Point(Side::Computer)) => Some(Pulse::Lost),
            Some(Event::Finished(Side::You)) => Some(Pulse::Won),
            Some(Event::Finished(Side::Computer)) => Some(Pulse::Lost),
            None => None,
        };
        if ui.icon_button(Icon::Back, Rect::new(12., 12., 44., 44.), false) {
            return (true, pulse);
        }
        ui.centered(
            "Table tennis",
            Rect::new(58., 12., size.x - 116., 44.),
            20.,
            ui.theme.text,
            true,
        );
        if ui.icon_button(
            if self.needs_frame() {
                Icon::Pause
            } else {
                Icon::Play
            },
            Rect::new(size.x - 56., 12., 44., 44.),
            false,
        ) {
            self.play(now);
        }
        self.score(ui, &l);
        draw_table(&l, &ui.theme);
        draw_racket(&l, self.game.computer, false, &ui.theme);
        draw_racket(&l, self.game.player, true, &ui.theme);
        let (ball, height) = self.game.ball();
        if matches!(self.game.phase, Phase::Rally | Phase::Paused) {
            draw_ball(
                &l,
                ball,
                height,
                &ui.theme,
                (1. - ((now - self.hit) / 0.18) as f32).clamp(0., 1.),
            );
        }
        if self.game.phase != Phase::Rally {
            self.overlay(ui, &l, now);
        }
        if !ui.theme.saver {
            let hint = if l.landscape {
                Rect::new(12., size.y - 72., 138., 40.)
            } else {
                Rect::new(16., size.y - 38., size.x - 32., 16.)
            };
            ui.centered(
                if l.landscape {
                    "Move to return"
                } else {
                    "Move to return · Aim with the racket’s edge"
                },
                hint,
                11.,
                ui.theme.muted,
                false,
            );
        }
        (false, pulse)
    }
    fn score(&self, ui: &Ui, l: &TableLayout) {
        let r = l.score;
        for (index, label, color) in [(0, "YOU", CORAL), (1, "CPU", BLUE)] {
            let w = r.w / 2.;
            let x = r.x + index as f32 * w;
            let label_y = if l.landscape { r.y } else { r.y - 4. };
            ui.centered(
                label,
                Rect::new(x, label_y, w, 18.),
                10.,
                ui.theme.muted,
                true,
            );
            ui.centered(
                &self.game.score[index].to_string(),
                Rect::new(x, label_y + 18., w, if l.landscape { 48. } else { 30. }),
                if l.landscape { 36. } else { 27. },
                color,
                true,
            );
            if self.game.server()
                == if index == 0 {
                    Side::You
                } else {
                    Side::Computer
                }
            {
                draw_circle(x + w / 2. + 20., label_y + 9., 2.5, color);
            }
        }
        if l.landscape {
            ui.centered(
                "FIRST TO 11",
                Rect::new(r.x, r.bottom() + 12., r.w, 20.),
                10.,
                ui.theme.muted,
                true,
            );
            if self.game.rally > 0 {
                ui.centered(
                    &format!(
                        "{} {}",
                        self.game.rally,
                        if self.game.rally == 1 {
                            "return"
                        } else {
                            "returns"
                        }
                    ),
                    Rect::new(r.x, r.bottom() + 34., r.w, 20.),
                    12.,
                    ui.theme.muted,
                    false,
                );
            }
        }
    }
    fn overlay(&mut self, ui: &mut Ui, l: &TableLayout, now: f64) {
        let ready = self.game.phase == Phase::Ready;
        let h = if ready { 186. } else { 134. };
        let w = (screen_width() - 32.).min(if l.landscape { l.table.w - 16. } else { 284. });
        let r = Rect::new(
            l.table.center().x - w / 2.,
            l.table.center().y - h / 2.,
            w,
            h,
        );
        bordered(r, 22., ui.theme.line, ui.theme.bg);
        let title = match self.game.phase {
            Phase::Ready => "First to 11",
            Phase::Paused => "Paused",
            Phase::Point => {
                if self.game.last_point == Some(Side::You) {
                    "Your point!"
                } else {
                    "Computer’s point"
                }
            }
            Phase::Finished => {
                if self.game.winner() == Some(Side::You) {
                    "You win!"
                } else {
                    "Good game!"
                }
            }
            Phase::Rally => unreachable!(),
        };
        ui.centered(
            title,
            Rect::new(r.x, r.y + 10., w, 32.),
            22.,
            ui.theme.text,
            true,
        );
        let caption = if ready {
            "Drag your racket. Return after the bounce.".to_owned()
        } else if self.game.phase == Phase::Paused {
            "Space to resume".to_owned()
        } else if self.game.phase == Phase::Finished {
            format!("{} – {}", self.game.score[0], self.game.score[1])
        } else {
            format!("{} serves", self.game.server().label())
        };
        ui.centered(
            &caption,
            Rect::new(r.x + 8., r.y + 44., w - 16., 22.),
            11.,
            ui.theme.muted,
            false,
        );
        if ready {
            let tab_w = (w - 24.) / 3.;
            for (i, level) in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard]
                .into_iter()
                .enumerate()
            {
                if ui.tab(
                    level.label(),
                    Rect::new(r.x + 12. + i as f32 * tab_w, r.y + 77., tab_w, 42.),
                    self.game.difficulty == level,
                ) {
                    self.game.difficulty = level;
                    self.game.revision += 1;
                }
            }
        }
        let label = match self.game.phase {
            Phase::Paused => "Resume",
            Phase::Finished => "Play again",
            _ => "Serve",
        };
        if ui.button(
            label,
            Rect::new(r.x + 12., r.bottom() - 58., w - 24., 46.),
            true,
        ) {
            self.play(now);
        }
    }
    pub fn announcement(&self) -> String {
        format!(
            "Jarcade. Table tennis. {:?}. {}. You {}. Computer {}. {} serves. Rally {}. Move your racket with finger, mouse, or arrow keys. Space to serve or pause.",
            self.game.phase,
            self.game.difficulty.label(),
            self.game.score[0],
            self.game.score[1],
            self.game.server().label(),
            self.game.rally
        )
    }
}

fn draw_table(l: &TableLayout, theme: &Theme) {
    let r = l.table;
    if !theme.saver {
        rounded(
            Rect::new(r.x - 5., r.y + 6., r.w + 10., r.h + 4.),
            12.,
            color_u8!(224, 237, 237, 255),
        );
    }
    bordered(
        r,
        7.,
        if theme.saver {
            theme.line
        } else {
            color_u8!(20, 77, 88, 255)
        },
        if theme.saver { BLACK } else { TABLE },
    );
    let line = if theme.saver {
        theme.muted
    } else {
        color_u8!(218, 243, 241, 255)
    };
    let edge = 0.015;
    let corners = [
        vec2(edge, edge),
        vec2(1. - edge, edge),
        vec2(1. - edge, 1. - edge),
        vec2(edge, 1. - edge),
    ];
    for i in 0..4 {
        let a = l.point(corners[i]);
        let b = l.point(corners[(i + 1) % 4]);
        draw_line(a.x, a.y, b.x, b.y, 1.8, line);
    }
    let a = l.point(vec2(0.5, edge));
    let b = l.point(vec2(0.5, 1. - edge));
    draw_line(a.x, a.y, b.x, b.y, 1., Color { a: 0.45, ..line });
    let a = l.point(vec2(-0.015, 0.5));
    let b = l.point(vec2(1.015, 0.5));
    draw_line(
        a.x,
        a.y,
        b.x,
        b.y,
        7.,
        if theme.saver {
            theme.line
        } else {
            color_u8!(18, 70, 83, 255)
        },
    );
    draw_line(a.x, a.y, b.x, b.y, 1.8, line);
    if !theme.saver {
        for i in 0..32 {
            let c = a.lerp(b, i as f32 / 31.);
            draw_circle(c.x, c.y, 1.1, line);
        }
    }
    for p in [a, b] {
        draw_circle(p.x, p.y, 4., line);
    }
}
fn draw_racket(l: &TableLayout, point: Vec2, player: bool, theme: &Theme) {
    let p = l.racket_point(point);
    let radius = (l.unit() * RACKET_RADIUS).max(3.);
    let d = if l.landscape {
        vec2(if player { 1. } else { -1. }, 0.)
    } else {
        vec2(0., if player { 1. } else { -1. })
    };
    let end = p + d * radius * 1.8;
    if !theme.saver {
        circle(p.x + 2., p.y + 4., radius + 1., color_u8!(20, 83, 95, 255));
    }
    draw_line(
        p.x,
        p.y,
        end.x,
        end.y,
        radius * 0.42,
        color_u8!(229, 183, 125, 255),
    );
    circle(p.x, p.y, radius + 2., color_u8!(252, 224, 176, 255));
    circle(p.x, p.y, radius, if player { CORAL } else { BLUE });
    if !theme.saver {
        draw_poly_lines(
            p.x,
            p.y,
            64,
            radius * 0.77,
            0.,
            1.,
            Color::new(1., 1., 1., 0.20),
        );
        let q = p - d * radius * 0.3;
        circle(q.x, q.y, radius * 0.12, Color::new(1., 1., 1., 0.8));
    }
}
fn circle(x: f32, y: f32, radius: f32, color: Color) {
    draw_poly(x, y, if radius > 8. { 64 } else { 32 }, radius, 0., color);
}
fn draw_ball(l: &TableLayout, point: Vec2, height: f32, theme: &Theme, hit: f32) {
    let floor = l.point(point);
    let p = floor - vec2(0., height * l.unit());
    let radius = (l.unit() * 0.017).clamp(4., 8.);
    if !theme.saver {
        draw_ellipse(
            floor.x + 2.,
            floor.y + 3.,
            radius * 0.95,
            radius * 0.5,
            0.,
            Color::new(0.02, 0.15, 0.17, 0.40),
        );
        if hit > 0. {
            draw_circle_lines(
                p.x,
                p.y,
                radius + (1. - hit) * 18.,
                1.5,
                Color::new(1., 1., 1., hit * 0.6),
            );
        }
    }
    draw_circle(p.x, p.y, radius + 1., color_u8!(225, 186, 112, 255));
    draw_circle(p.x, p.y, radius, color_u8!(255, 249, 223, 255));
    draw_circle(
        p.x - radius * 0.25,
        p.y - radius * 0.25,
        radius * 0.25,
        WHITE,
    );
}

/// The home card uses the same vector table, rackets and ball as gameplay.
pub fn preview(ui: &Ui, rect: Rect) {
    let w = rect.w * 0.68;
    let l = TableLayout {
        table: Rect::new(rect.center().x - w / 2., rect.y + 3., w, rect.h - 6.),
        score: rect,
        landscape: false,
    };
    draw_table(&l, &ui.theme);
    draw_racket(&l, vec2(0.65, 0.16), false, &ui.theme);
    draw_racket(&l, vec2(0.32, 0.83), true, &ui.theme);
    draw_ball(&l, vec2(0.60, 0.59), 0.08, &ui.theme, 0.);
}
