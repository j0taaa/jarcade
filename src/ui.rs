use jarcade::layout::touch_point;
use macroquad::prelude::*;
use std::{cell::RefCell, collections::HashSet};

pub const GREEN: Color = color_u8!(27, 109, 75, 255);
pub const MINT: Color = color_u8!(235, 246, 228, 255);
pub const CORAL: Color = color_u8!(235, 106, 67, 255);

pub struct Theme {
    pub bg: Color,
    pub panel: Color,
    pub line: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub saver: bool,
}

impl Theme {
    pub fn new(saver: bool) -> Self {
        Self {
            bg: if saver { BLACK } else { WHITE },
            panel: if saver {
                BLACK
            } else {
                color_u8!(246, 248, 245, 255)
            },
            line: if saver {
                color_u8!(47, 57, 51, 255)
            } else {
                color_u8!(231, 236, 231, 255)
            },
            text: if saver {
                color_u8!(245, 249, 243, 255)
            } else {
                color_u8!(24, 34, 28, 255)
            },
            muted: if saver {
                color_u8!(156, 169, 159, 255)
            } else {
                color_u8!(95, 112, 102, 255)
            },
            accent: if saver {
                color_u8!(164, 220, 133, 255)
            } else {
                GREEN
            },
            saver,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Icon {
    Back,
    Settings,
    Play,
    Pause,
    Arrow((i16, i16)),
}

pub struct Ui {
    pub font: Font,
    pub bold: Font,
    pub theme: Theme,
    pub keyboard_focus: bool,
    pub activated: bool,
    focus: usize,
    count: usize,
    previous_count: usize,
    cached_glyphs: RefCell<HashSet<(bool, u16, char)>>,
    pointer: Option<Vec2>,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            font: load_ttf_font_from_bytes(include_bytes!("../assets/Inter-Medium.ttf"))
                .expect("bundled Inter font"),
            bold: load_ttf_font_from_bytes(include_bytes!("../assets/Inter-SemiBold.ttf"))
                .expect("bundled Inter font"),
            theme: Theme::new(false),
            keyboard_focus: false,
            activated: false,
            focus: 0,
            count: 0,
            previous_count: 1,
            cached_glyphs: RefCell::new(HashSet::new()),
            pointer: None,
        }
    }

    pub fn begin(&mut self, saver: bool, press: Option<Vec2>) {
        self.theme = Theme::new(saver);
        self.count = 0;
        self.activated = false;
        if is_key_pressed(KeyCode::Tab) {
            if self.keyboard_focus {
                let backwards = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                self.focus = (self.focus
                    + if backwards {
                        self.previous_count - 1
                    } else {
                        1
                    })
                    % self.previous_count;
            }
            self.keyboard_focus = true;
        }
        self.pointer = press.or_else(|| {
            touches()
                .iter()
                .find(|t| t.phase == TouchPhase::Started)
                .map(|t| touch_point(t.position, screen_dpi_scale()))
        });
        if self.pointer.is_none() && is_mouse_button_pressed(MouseButton::Left) {
            let (x, y) = mouse_position();
            self.pointer = Some(vec2(x, y));
        }
        if self.pointer.is_some() {
            self.keyboard_focus = false;
        }
    }

    pub fn end(&mut self) {
        self.previous_count = self.count.max(1);
    }
    pub fn reset_focus(&mut self) {
        self.focus = 0;
        self.keyboard_focus = false;
    }

    pub fn focused_item(&self) -> Option<usize> {
        self.keyboard_focus.then_some(self.focus)
    }

    pub fn label(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        self.text(text, vec2(x, y), size, color, false);
    }
    pub fn heading(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        self.text(text, vec2(x, y), size, color, true);
    }
    // Macroquad may replace its glyph texture when an atlas grows. Flush queued
    // draws before caching new glyphs, so those draws never reference a deleted
    // texture. Existing text stays batched; there is no per-frame flush penalty.
    fn prepare_text(&self, text: &str, size: f32, bold: bool) {
        let pixels = ((size as u16) as f32 * screen_dpi_scale()).ceil() as u16;
        let mut cached = self.cached_glyphs.borrow_mut();
        if text.chars().any(|c| !cached.contains(&(bold, pixels, c))) {
            // SAFETY: called synchronously on the render thread; no GL handle escapes.
            unsafe {
                get_internal_gl().flush();
            }
            let characters: Vec<_> = text.chars().collect();
            let font = if bold { &self.bold } else { &self.font };
            font.populate_font_cache(&characters, pixels);
            cached.extend(characters.into_iter().map(|c| (bold, pixels, c)));
        }
    }

    pub fn text_width(&self, text: &str, size: f32, bold: bool) -> f32 {
        self.prepare_text(text, size, bold);
        measure_text(
            text,
            Some(if bold { &self.bold } else { &self.font }),
            size as u16,
            1.0,
        )
        .width
    }

    fn text(&self, text: &str, pos: Vec2, size: f32, color: Color, bold: bool) {
        self.prepare_text(text, size, bold);
        draw_text_ex(
            text,
            pos.x,
            pos.y,
            TextParams {
                font: Some(if bold { &self.bold } else { &self.font }),
                font_size: size as u16,
                color,
                ..Default::default()
            },
        );
    }
    pub fn centered(&self, text: &str, rect: Rect, size: f32, color: Color, bold: bool) {
        self.prepare_text(text, size, bold);
        let font = if bold { &self.bold } else { &self.font };
        let metrics = measure_text(text, Some(font), size as u16, 1.0);
        self.text(
            text,
            vec2(
                rect.x + (rect.w - metrics.width) / 2.0,
                rect.y + (rect.h - metrics.height) / 2.0 + metrics.offset_y,
            ),
            size,
            color,
            bold,
        );
    }

    pub fn button(&mut self, text: &str, rect: Rect, primary: bool) -> bool {
        let fill = if primary && !self.theme.saver {
            GREEN
        } else {
            self.theme.bg
        };
        bordered(
            rect,
            17.0,
            if primary {
                self.theme.accent
            } else {
                self.theme.line
            },
            fill,
        );
        let color = if primary && !self.theme.saver {
            WHITE
        } else if primary {
            self.theme.accent
        } else {
            self.theme.text
        };
        self.centered(text, rect, 16.0, color, true);
        self.hit(rect)
    }

    pub fn icon_button(&mut self, icon: Icon, rect: Rect, primary: bool) -> bool {
        let fill = if primary && !self.theme.saver {
            GREEN
        } else {
            self.theme.panel
        };
        if self.theme.saver {
            bordered(rect, rect.h / 2.0, self.theme.line, fill);
        } else {
            rounded(rect, rect.h / 2.0, fill);
        }
        let color = if primary && !self.theme.saver {
            WHITE
        } else {
            self.theme.text
        };
        draw_icon(icon, rect.center(), color);
        self.hit(rect)
    }

    pub fn tab(&mut self, text: &str, rect: Rect, active: bool) -> bool {
        if active {
            if self.theme.saver {
                bordered(rect, 22.0, self.theme.accent, BLACK);
            } else {
                rounded(rect, 22.0, MINT);
            }
        }
        self.centered(
            text,
            rect,
            14.0,
            if active {
                self.theme.accent
            } else {
                self.theme.muted
            },
            active,
        );
        self.hit(rect)
    }

    /// Gallery and scrolling-page controls activate only on release.
    pub fn override_pointer(&mut self, point: Option<Vec2>) -> Option<Vec2> {
        std::mem::replace(&mut self.pointer, point)
    }
    pub fn keyboard_hit(&mut self, rect: Rect) -> bool {
        let pointer = self.pointer.take();
        let hit = self.hit(rect);
        self.pointer = pointer;
        hit
    }

    pub fn hit(&mut self, rect: Rect) -> bool {
        let index = self.count;
        self.count += 1;
        let focused = self.keyboard_focus && self.focus == index;
        if focused {
            draw_rectangle_lines(
                rect.x - 3.0,
                rect.y - 3.0,
                rect.w + 6.0,
                rect.h + 6.0,
                2.0,
                self.theme.accent,
            );
        }
        let activated = self.pointer.is_some_and(|point| rect.contains(point))
            || (focused && is_key_pressed(KeyCode::Enter));
        self.activated |= activated;
        activated
    }

    pub fn toggle(
        &mut self,
        title: &str,
        subtitle: &str,
        rect: Rect,
        on: bool,
        enabled: bool,
    ) -> bool {
        self.heading(title, rect.x, rect.y + 22.0, 18.0, self.theme.text);
        self.label(subtitle, rect.x, rect.y + 48.0, 13.0, self.theme.muted);
        let switch = Rect::new(rect.right() - 50.0, rect.y + 1.0, 50.0, 30.0);
        rounded(
            switch,
            15.0,
            if on && enabled {
                self.theme.accent
            } else {
                self.theme.line
            },
        );
        draw_circle(
            switch.x + if on && enabled { 35.0 } else { 15.0 },
            switch.y + 15.0,
            11.0,
            if on && self.theme.saver { BLACK } else { WHITE },
        );
        enabled && self.hit(rect)
    }
}

pub fn rounded(rect: Rect, radius: f32, color: Color) {
    let r = radius.min(rect.w / 2.0).min(rect.h / 2.0).max(0.0);
    draw_rectangle(rect.x + r, rect.y, rect.w - r * 2.0, rect.h, color);
    draw_rectangle(rect.x, rect.y + r, rect.w, rect.h - r * 2.0, color);
    for (x, y) in [
        (rect.x + r, rect.y + r),
        (rect.right() - r, rect.y + r),
        (rect.x + r, rect.bottom() - r),
        (rect.right() - r, rect.bottom() - r),
    ] {
        draw_poly(x, y, 48, r, 0.0, color);
    }
}

pub fn bordered(rect: Rect, radius: f32, border: Color, fill: Color) {
    rounded(rect, radius, border);
    rounded(
        Rect::new(rect.x + 1.0, rect.y + 1.0, rect.w - 2.0, rect.h - 2.0),
        radius - 1.0,
        fill,
    );
}

pub fn draw_icon(icon: Icon, center: Vec2, color: Color) {
    let p = |x: f32, y: f32| center + vec2(x, y);
    let line = |a: Vec2, b: Vec2| draw_line(a.x, a.y, b.x, b.y, 1.8, color);
    match icon {
        Icon::Play => draw_triangle(p(-4.0, -8.0), p(-4.0, 8.0), p(8.0, 0.0), color),
        Icon::Pause => {
            for x in [-4.0, 4.0] {
                rounded(
                    Rect::new(center.x + x - 1.5, center.y - 8.0, 3.0, 16.0),
                    1.5,
                    color,
                );
            }
        }
        Icon::Back | Icon::Arrow(_) => {
            let (dx, dy) = if let Icon::Arrow(delta) = icon {
                delta
            } else {
                (-1, 0)
            };
            let axis = vec2(f32::from(dx), f32::from(dy));
            let side = vec2(-axis.y, axis.x);
            let tip = center + axis * 8.0;
            line(center - axis * 8.0, tip);
            line(tip, center + side * 7.0);
            line(tip, center - side * 7.0);
        }
        Icon::Settings => {
            for (y, x) in [(-7.0, 3.0), (0.0, -4.0), (7.0, 2.0)] {
                line(p(-9.0, y), p(9.0, y));
                draw_circle(center.x + x, center.y + y, 3.0, color);
            }
        }
    }
}

pub fn logo(rect: Rect, color: Color) {
    let unit = rect.w / 2.0;
    for (x, y) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        rounded(
            Rect::new(rect.x + x * unit, rect.y + y * unit, unit - 2.0, unit - 2.0),
            5.0,
            color,
        );
    }
}
