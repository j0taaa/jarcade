//! Original, resolution-independent woodland court portraits.
use crate::ui::{Ui, bordered};
use jarcade::multiplayer::coup::Role;
use macroquad::prelude::*;
pub fn court_card(ui: &Ui, rect: Rect, role: Option<Role>, revealed: bool, small: bool) {
    let (base, ink) = match role {
        Some(Role::Regent) => (color_u8!(246, 224, 174, 255), color_u8!(167, 107, 52, 255)),
        Some(Role::Shade) => (color_u8!(222, 212, 248, 255), color_u8!(94, 75, 130, 255)),
        Some(Role::Corsair) => (color_u8!(188, 226, 238, 255), color_u8!(39, 106, 131, 255)),
        Some(Role::Envoy) => (color_u8!(222, 237, 196, 255), color_u8!(96, 127, 65, 255)),
        Some(Role::Sentinel) => (color_u8!(247, 208, 198, 255), color_u8!(169, 78, 65, 255)),
        None => (color_u8!(31, 82, 64, 255), color_u8!(152, 207, 175, 255)),
    };
    bordered(
        rect,
        12.,
        if revealed {
            ui.theme.muted
        } else {
            ui.theme.line
        },
        if ui.theme.saver { BLACK } else { base },
    );
    let center = vec2(rect.center().x, rect.y + rect.h * 0.42);
    let s = rect.w * 0.32;
    let shade = if ui.theme.saver {
        ink
    } else {
        Color::new(ink.r * 0.72, ink.g * 0.72, ink.b * 0.72, 1.)
    };
    if role.is_none() {
        draw_poly(center.x, center.y, 6, s, 30., ink);
        draw_poly(
            center.x,
            center.y,
            6,
            s * 0.82,
            30.,
            if ui.theme.saver { BLACK } else { base },
        );
        draw_circle(center.x, center.y, s * 0.38, ink);
        for a in 0..6 {
            let angle = a as f32 * std::f32::consts::TAU / 6.;
            draw_circle(
                center.x + angle.cos() * s * 0.62,
                center.y + angle.sin() * s * 0.62,
                s * 0.08,
                ink,
            );
        }
    } else {
        // Fox / owl / otter / hare / bear, with distinct silhouettes and regalia.
        draw_ellipse(center.x, center.y + s * 0.85, s * 0.98, s * 0.60, 0., shade);
        if role == Some(Role::Envoy) {
            for sign in [-1., 1.] {
                draw_ellipse(
                    center.x + sign * s * 0.45,
                    center.y - s * 0.78,
                    s * 0.24,
                    s * 0.85,
                    sign * 12.,
                    ink,
                );
                draw_ellipse(
                    center.x + sign * s * 0.45,
                    center.y - s * 0.78,
                    s * 0.1,
                    s * 0.6,
                    sign * 12.,
                    base,
                );
            }
        } else if role == Some(Role::Sentinel) || role == Some(Role::Corsair) {
            for sign in [-1., 1.] {
                draw_circle(
                    center.x + sign * s * 0.69,
                    center.y - s * 0.5,
                    s * 0.35,
                    ink,
                );
                draw_circle(
                    center.x + sign * s * 0.69,
                    center.y - s * 0.5,
                    s * 0.19,
                    base,
                );
            }
        } else {
            for sign in [-1., 1.] {
                draw_triangle(
                    center + vec2(sign * s * 0.88, -s * 0.05),
                    center + vec2(sign * s * 0.9, -s * 0.9),
                    center + vec2(sign * s * 0.2, -s * 0.56),
                    ink,
                );
            }
        }
        draw_ellipse(center.x, center.y, s * 0.87, s * 0.83, 0., ink);
        for sign in [-1., 1.] {
            draw_ellipse(
                center.x + sign * s * 0.29,
                center.y + s * 0.18,
                s * 0.43,
                s * 0.45,
                sign * 12.,
                color_u8!(255, 244, 219, 255),
            );
            draw_circle(
                center.x + sign * s * 0.32,
                center.y - s * 0.12,
                s * 0.13,
                shade,
            );
            draw_circle(
                center.x + sign * s * 0.29,
                center.y - s * 0.17,
                s * 0.04,
                WHITE,
            );
        }
        draw_triangle(
            center + vec2(-s * 0.12, s * 0.25),
            center + vec2(s * 0.12, s * 0.25),
            center + vec2(0., s * 0.4),
            shade,
        );
        draw_line(
            center.x,
            center.y + s * 0.4,
            center.x,
            center.y + s * 0.52,
            s * 0.04,
            shade,
        );
        if role == Some(Role::Regent) {
            let y = center.y - s * 0.74;
            draw_rectangle(
                center.x - s * 0.43,
                y,
                s * 0.86,
                s * 0.19,
                color_u8!(226, 168, 49, 255),
            );
            for i in -1..=1 {
                draw_triangle(
                    vec2(center.x + i as f32 * s * 0.3 - s * 0.13, y),
                    vec2(center.x + i as f32 * s * 0.3 + s * 0.13, y),
                    vec2(center.x + i as f32 * s * 0.3, y - s * 0.27),
                    color_u8!(226, 168, 49, 255),
                );
            }
        }
        if role == Some(Role::Shade) {
            for sign in [-1., 1.] {
                draw_circle_lines(
                    center.x + sign * s * 0.32,
                    center.y - s * 0.1,
                    s * 0.25,
                    s * 0.06,
                    shade,
                );
            }
        }
        if role == Some(Role::Corsair) {
            draw_line(
                center.x - s * 0.77,
                center.y - s * 0.22,
                center.x + s * 0.77,
                center.y - s * 0.32,
                s * 0.12,
                shade,
            );
            draw_ellipse(
                center.x + s * 0.32,
                center.y - s * 0.1,
                s * 0.2,
                s * 0.22,
                0.,
                shade,
            );
        }
        if role == Some(Role::Sentinel) {
            draw_poly(
                center.x,
                center.y + s * 0.89,
                5,
                s * 0.23,
                -90.,
                color_u8!(235, 183, 62, 255),
            );
        }
    }
    if !small {
        let title = role.map_or("COUPE", Role::title);
        ui.centered(
            title,
            Rect::new(rect.x + 4., rect.bottom() - 38., rect.w - 8., 28.),
            (rect.w * 0.14).clamp(10., 20.),
            if ui.theme.saver { ui.theme.text } else { shade },
            true,
        );
    }
    if revealed {
        draw_line(
            rect.x + 10.,
            rect.y + 10.,
            rect.right() - 10.,
            rect.bottom() - 10.,
            2.,
            ui.theme.muted,
        );
    }
}
pub fn preview(ui: &Ui, rect: Rect) {
    let w = rect.w * 0.39;
    let h = rect.h * 0.70;
    court_card(
        ui,
        Rect::new(rect.x + rect.w * 0.06, rect.y + rect.h * 0.15, w, h),
        Some(Role::Regent),
        false,
        true,
    );
    court_card(
        ui,
        Rect::new(rect.x + rect.w * 0.55, rect.y + rect.h * 0.15, w, h),
        Some(Role::Shade),
        false,
        true,
    );
    for i in 0..3 {
        draw_circle(
            rect.center().x + (i as f32 - 1.) * rect.w * 0.08,
            rect.y + rect.h * 0.90,
            rect.w * 0.035,
            color_u8!(226, 168, 49, 255),
        );
    }
}

#[cfg(target_arch = "wasm32")]
use macroquad::experimental::coroutines::{Coroutine, start_coroutine, stop_coroutine};
use std::collections::HashMap;
pub struct DeckArt {
    textures: HashMap<u8, Texture2D>,
    #[cfg(target_arch = "wasm32")]
    pending: HashMap<u8, (Coroutine<Option<Texture2D>>, f64)>,
    failed: Vec<u8>,
}
impl DeckArt {
    pub fn new() -> Self {
        let first = Texture2D::from_file_with_format(
            include_bytes!("../assets/reverie/atlas-01.jpg"),
            Some(ImageFormat::Jpeg),
        );
        first.set_filter(FilterMode::Linear);
        Self {
            textures: HashMap::from([(0, first)]),
            #[cfg(target_arch = "wasm32")]
            pending: HashMap::new(),
            failed: vec![],
        }
    }
    pub fn update(&mut self, cards: &[u8]) {
        let mut needed: Vec<_> = cards.iter().map(|c| c / 4).collect();
        needed.push(0);
        needed.sort_unstable();
        needed.dedup();
        self.textures.retain(|id, _| needed.contains(id));
        #[cfg(target_arch = "wasm32")]
        {
            let ids: Vec<_> = self.pending.keys().copied().collect();
            for id in ids {
                let (task, time) = &self.pending[&id];
                if let Some(result) = task.retrieve() {
                    self.pending.remove(&id);
                    if let Some(t) = result {
                        t.set_filter(FilterMode::Linear);
                        self.textures.insert(id, t);
                    } else {
                        self.failed.push(id);
                    }
                } else if get_time() - *time > 15. {
                    stop_coroutine(task.clone());
                    self.pending.remove(&id);
                    self.failed.push(id);
                }
            }
        }
        for id in needed {
            if self.textures.contains_key(&id) || self.failed.contains(&id) {
                continue;
            }
            #[cfg(target_arch = "wasm32")]
            if let std::collections::hash_map::Entry::Vacant(entry) = self.pending.entry(id) {
                let task = start_coroutine(async move {
                    load_texture(&format!("assets/reverie/atlas-{:02}.jpg", id + 1))
                        .await
                        .ok()
                });
                entry.insert((task, get_time()));
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let texture =
                    Texture2D::from_file_with_format(native_atlas(id), Some(ImageFormat::Jpeg));
                texture.set_filter(FilterMode::Linear);
                self.textures.insert(id, texture);
            }
        }
    }
    pub fn loading(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            !self.pending.is_empty()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            false
        }
    }
    pub fn retry(&mut self) {
        self.failed.clear();
    }
    pub fn failed(&self) -> bool {
        !self.failed.is_empty()
    }
    pub fn draw(&self, ui: &Ui, card: u8, rect: Rect) {
        bordered(rect, 10., ui.theme.line, ui.theme.panel);
        if let Some(t) = self.textures.get(&(card / 4)) {
            let unit = vec2(t.width() / 2., t.height() / 2.);
            draw_texture_ex(
                t,
                rect.x + 2.,
                rect.y + 2.,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(rect.size() - Vec2::splat(4.)),
                    source: Some(Rect::new(
                        (card % 2) as f32 * unit.x,
                        ((card % 4) / 2) as f32 * unit.y,
                        unit.x,
                        unit.y,
                    )),
                    ..Default::default()
                },
            );
        } else {
            ui.centered("…", rect, 22., ui.theme.muted, false);
        }
    }
    pub fn preview(&self, ui: &Ui, rect: Rect) {
        let w = rect.w * 0.4;
        let h = (w * 1.5).min(rect.h * 0.94);
        self.draw(
            ui,
            0,
            Rect::new(rect.x + rect.w * 0.05, rect.center().y - h / 2., w, h),
        );
        self.draw(
            ui,
            1,
            Rect::new(rect.x + rect.w * 0.55, rect.center().y - h / 2., w, h),
        );
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn native_atlas(id: u8) -> &'static [u8] {
    const FILES: [&[u8]; 21] = [
        include_bytes!("../assets/reverie/atlas-01.jpg"),
        include_bytes!("../assets/reverie/atlas-02.jpg"),
        include_bytes!("../assets/reverie/atlas-03.jpg"),
        include_bytes!("../assets/reverie/atlas-04.jpg"),
        include_bytes!("../assets/reverie/atlas-05.jpg"),
        include_bytes!("../assets/reverie/atlas-06.jpg"),
        include_bytes!("../assets/reverie/atlas-07.jpg"),
        include_bytes!("../assets/reverie/atlas-08.jpg"),
        include_bytes!("../assets/reverie/atlas-09.jpg"),
        include_bytes!("../assets/reverie/atlas-10.jpg"),
        include_bytes!("../assets/reverie/atlas-11.jpg"),
        include_bytes!("../assets/reverie/atlas-12.jpg"),
        include_bytes!("../assets/reverie/atlas-13.jpg"),
        include_bytes!("../assets/reverie/atlas-14.jpg"),
        include_bytes!("../assets/reverie/atlas-15.jpg"),
        include_bytes!("../assets/reverie/atlas-16.jpg"),
        include_bytes!("../assets/reverie/atlas-17.jpg"),
        include_bytes!("../assets/reverie/atlas-18.jpg"),
        include_bytes!("../assets/reverie/atlas-19.jpg"),
        include_bytes!("../assets/reverie/atlas-20.jpg"),
        include_bytes!("../assets/reverie/atlas-21.jpg"),
    ];
    FILES[usize::from(id)]
}
