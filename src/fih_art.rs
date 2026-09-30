//! Shared artwork for the room, wardrobe previews, mini-games and arcade card.
use crate::ui::{Ui, bordered, rounded};
use jarcade::fih::{Fih, Room};
use jarcade::{
    fih_interaction::{Pose, body_half_width},
    fih_svg::{Scene, Shape},
};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub const INK: Color = color_u8!(74, 62, 78, 255);
pub const PINK: Color = color_u8!(240, 160, 180, 255);
pub const LILAC: Color = color_u8!(164, 145, 213, 255);
pub const MINT: Color = color_u8!(125, 191, 175, 255);
pub const GOLD: Color = color_u8!(235, 183, 95, 255);
pub const BLUE: Color = color_u8!(129, 182, 213, 255);
pub const COLORS: [Color; 8] = [
    color_u8!(255, 165, 68, 255),
    color_u8!(184, 173, 234, 255),
    color_u8!(169, 208, 237, 255),
    color_u8!(177, 222, 193, 255),
    color_u8!(248, 215, 145, 255),
    color_u8!(243, 179, 207, 255),
    color_u8!(233, 226, 208, 255),
    color_u8!(180, 201, 219, 255),
];
pub const CLOTHES: [&str; 12] = [
    "Bare fins",
    "Sailor",
    "Cozy scarf",
    "Hoodie",
    "Raincoat",
    "Striped tee",
    "Overalls",
    "Princess",
    "Superhero",
    "Pajamas",
    "Formal",
    "Astronaut",
];
pub const HATS: [&str; 12] = [
    "No hat",
    "Bow",
    "Crown",
    "Cap",
    "Beanie",
    "Sunhat",
    "Pirate",
    "Chef",
    "Headphones",
    "Flower",
    "Wizard",
    "Party",
];
pub const WALLS: [&str; 8] = [
    "Original", "Coral", "Twilight", "Garden", "Peach", "Mint", "Lilac", "Sand",
];

#[derive(Clone)]
pub struct Art {
    rooms: Rc<[Scene; 5]>,
}
thread_local! { static ART: RefCell<Option<Art>> = const { RefCell::new(None) }; }
impl Art {
    pub fn shared() -> Self {
        ART.with(|slot| {
            slot.borrow_mut()
                .get_or_insert_with(|| Self {
                    rooms: Rc::new([
                        Scene::parse(include_str!("../assets/fih/kitchen.svg"))
                            .expect("kitchen SVG"),
                        Scene::parse(include_str!("../assets/fih/bathroom.svg"))
                            .expect("bathroom SVG"),
                        Scene::parse(include_str!("../assets/fih/bedroom.svg"))
                            .expect("bedroom SVG"),
                        Scene::parse(include_str!("../assets/fih/playroom.svg"))
                            .expect("playroom SVG"),
                        Scene::parse(include_str!("../assets/fih/clinic.svg")).expect("clinic SVG"),
                    ]),
                })
                .clone()
        })
    }
    pub fn room(&self, rect: Rect, room: Room, wallpaper: u8, saver: bool, asleep: bool) {
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, BLACK);
        if saver {
            return;
        }
        let scene = &self.rooms[room.index()];
        // The wall/floor spans the viewport; furniture retains its proportions.
        let sy = rect.h / 844.;
        let sx = sy.min(rect.w / 390.);
        let origin = vec2(rect.center().x - 195. * sx, rect.y);
        for (index, element) in scene.elements.iter().enumerate() {
            let [r, g, b] = element.color;
            let color = Color::from_rgba(r, g, b, 255);
            match &element.shape {
                Shape::Rect { x, y, w, h, radius } => {
                    let shape = if index < 3 {
                        Rect::new(rect.x, rect.y + y * sy, rect.w, h * sy)
                    } else {
                        Rect::new(origin.x + x * sx, origin.y + y * sy, w * sx, h * sy)
                    };
                    rounded(shape, radius * sx, color);
                }
                Shape::Ellipse { x, y, rx, ry } => {
                    ellipse(origin + vec2(x * sx, y * sy), vec2(rx * sx, ry * sy), color)
                }
                Shape::Polygon(points) => {
                    // SVG polygons are triangulated with ear clipping (stars are concave).
                    let polygon: Vec<_> = points
                        .iter()
                        .map(|&(x, y)| origin + vec2(x * sx, y * sy))
                        .collect();
                    let mut remaining: Vec<_> = (0..polygon.len()).collect();
                    while remaining.len() > 2 {
                        let mut found = false;
                        for j in 0..remaining.len() {
                            let a = polygon[remaining[(j + remaining.len() - 1) % remaining.len()]];
                            let b = polygon[remaining[j]];
                            let c = polygon[remaining[(j + 1) % remaining.len()]];
                            let cross = |a: Vec2, b: Vec2, c: Vec2| (b - a).perp_dot(c - a);
                            if cross(a, b, c) <= 0. {
                                continue;
                            }
                            if remaining.iter().any(|&i| {
                                polygon[i] != a
                                    && polygon[i] != b
                                    && polygon[i] != c
                                    && cross(a, b, polygon[i]) >= 0.
                                    && cross(b, c, polygon[i]) >= 0.
                                    && cross(c, a, polygon[i]) >= 0.
                            }) {
                                continue;
                            }
                            draw_triangle(a, b, c, color);
                            remaining.remove(j);
                            found = true;
                            break;
                        }
                        if !found {
                            break;
                        }
                    }
                }
            }
        }
        if wallpaper != 0 {
            let c = COLORS[usize::from(wallpaper)];
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::new(c.r, c.g, c.b, 0.16),
            );
        }
        if asleep {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::new(0.08, 0.05, 0.16, 0.66),
            );
        }
    }
    pub fn fish(&self, p: Vec2, r: f32, pet: &Fih, phase: f32) {
        self.fish_pose(
            p,
            r,
            pet,
            Pose {
                phase,
                ..Pose::default()
            },
        );
    }
    pub fn fish_pose(&self, p: Vec2, r: f32, pet: &Fih, pose: Pose) {
        crate::fih_character::fish(p, r, pet, pose, self);
    }
    pub fn clothes(&self, p: Vec2, r: f32, index: u8) {
        if index == 0 {
            return;
        }
        let base = [
            WHITE,
            BLUE,
            PINK,
            LILAC,
            GOLD,
            MINT,
            BLUE,
            PINK,
            color_u8!(218, 110, 104, 255),
            LILAC,
            INK,
            WHITE,
        ][usize::from(index)];
        // Clothing follows the exact lower body ellipse, with a curved neckline.
        // Every decoration is contained in this silhouette, below the mouth.
        const ROWS: usize = 28;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for row in 0..=ROWS {
            let y = if index == 2 {
                0.48 + row as f32 / ROWS as f32 * 0.13
            } else {
                0.52 + row as f32 / ROWS as f32 * 0.377
            };
            let half = body_half_width(y);
            for col in 0..=16 {
                let x = half * (col as f32 / 8. - 1.);
                let neckline =
                    0.04 * (1. - (x / 0.81).powi(2)).max(0.) * (1. - row as f32 / ROWS as f32);
                let mut c = base;
                if matches!(index, 1 | 5) && (((y - 0.52) * 40.) as u8).is_multiple_of(4) {
                    c = WHITE;
                }
                let shade = 1. - 0.18 * (x.abs() / 0.94);
                c.r *= shade;
                c.g *= shade;
                c.b *= shade;
                let local_y = y + neckline;
                let half = body_half_width(local_y);
                let v = p + vec2(x.clamp(-half, half), local_y) * r;
                vertices.push(Vertex::new(v.x, v.y, 0., 0., 0., c));
                if row < ROWS && col < 16 {
                    let a = (row * 17 + col) as u16;
                    indices.extend([a, a + 1, a + 17, a + 1, a + 18, a + 17]);
                }
            }
        }
        draw_mesh(&Mesh {
            vertices,
            indices,
            texture: None,
        });
        match index {
            1 => {
                draw_triangle(
                    p + vec2(-0.38, 0.55) * r,
                    p + vec2(0., 0.74) * r,
                    p + vec2(0.38, 0.55) * r,
                    WHITE,
                );
                heart(p + vec2(0., 0.65) * r, r * 0.055, PINK);
            }
            2 => {
                rounded(
                    Rect::new(p.x - r * 0.30, p.y + r * 0.55, r * 0.15, r * 0.24),
                    r * 0.035,
                    PINK,
                );
                for i in 0..3 {
                    draw_line(
                        p.x - r * 0.27,
                        p.y + r * (0.68 + i as f32 * 0.035),
                        p.x - r * 0.18,
                        p.y + r * (0.68 + i as f32 * 0.035),
                        r * 0.012,
                        WHITE,
                    );
                }
            }
            3 => {
                for x in [-0.16, 0.16] {
                    draw_line(
                        p.x + x * r,
                        p.y + r * 0.57,
                        p.x + x * r,
                        p.y + r * 0.65,
                        r * 0.018,
                        WHITE,
                    );
                }
                rounded(
                    Rect::new(p.x - r * 0.22, p.y + r * 0.71, r * 0.44, r * 0.085),
                    r * 0.035,
                    color_u8!(201, 188, 229, 255),
                );
            }
            4 => {
                draw_line(p.x, p.y + r * 0.56, p.x, p.y + r * 0.87, r * 0.013, WHITE);
                for y in [0.63, 0.73, 0.82] {
                    draw_circle(p.x + r * 0.04, p.y + y * r, r * 0.019, INK);
                }
            }
            6 => {
                for x in [-0.33, 0.33] {
                    draw_line(
                        p.x + x * r,
                        p.y + r * 0.54,
                        p.x + x * r,
                        p.y + r * 0.70,
                        r * 0.07,
                        MINT,
                    );
                    draw_circle(p.x + x * r, p.y + r * 0.67, r * 0.024, GOLD);
                }
                rounded(
                    Rect::new(p.x - r * 0.20, p.y + r * 0.72, r * 0.40, r * 0.08),
                    r * 0.02,
                    MINT,
                );
            }
            7 => {
                for x in [-0.44, -0.22, 0., 0.22, 0.44] {
                    draw_line(
                        p.x + x * r,
                        p.y + r * 0.59,
                        p.x + x * r * 0.65,
                        p.y + r * 0.81,
                        r * 0.017,
                        WHITE,
                    );
                }
                heart(p + vec2(0., 0.60) * r, r * 0.048, GOLD);
            }
            8 => draw_poly(p.x, p.y + r * 0.68, 5, r * 0.10, -90., GOLD),
            9 => {
                for v in [
                    vec2(-0.32, 0.61),
                    vec2(0.28, 0.64),
                    vec2(-0.12, 0.77),
                    vec2(0.18, 0.80),
                ] {
                    draw_poly(p.x + v.x * r, p.y + v.y * r, 5, r * 0.028, -90., WHITE);
                }
            }
            10 => {
                draw_triangle(
                    p + vec2(-0.27, 0.55) * r,
                    p + vec2(0., 0.82) * r,
                    p + vec2(0.27, 0.55) * r,
                    WHITE,
                );
                for side in [-1., 1.] {
                    draw_triangle(
                        p + vec2(0., 0.60) * r,
                        p + vec2(side * 0.13, 0.55) * r,
                        p + vec2(side * 0.13, 0.66) * r,
                        PINK,
                    );
                }
            }
            11 => {
                rounded(
                    Rect::new(p.x - r * 0.22, p.y + r * 0.62, r * 0.44, r * 0.14),
                    r * 0.035,
                    BLUE,
                );
                draw_circle(p.x - r * 0.09, p.y + r * 0.69, r * 0.026, PINK);
                draw_circle(p.x + r * 0.09, p.y + r * 0.69, r * 0.026, GOLD);
            }
            _ => {}
        }
    }
    pub fn hat(&self, p: Vec2, r: f32, index: u8) {
        let c = p + vec2(0., -0.86 * r);
        match index {
            1 => {
                ellipse(c + vec2(-0.16 * r, 0.), vec2(r * 0.20, r * 0.13), PINK);
                ellipse(c + vec2(0.16 * r, 0.), vec2(r * 0.20, r * 0.13), PINK);
                draw_circle(c.x, c.y, r * 0.09, LILAC);
            }
            2 => {
                let a = c + vec2(-0.33 * r, 0.1 * r);
                let b = c + vec2(0.33 * r, 0.1 * r);
                draw_triangle(a, b, c + vec2(0., -0.34 * r), GOLD);
                draw_triangle(a, a + vec2(-0.07 * r, -0.36 * r), c, GOLD);
                draw_triangle(b, b + vec2(0.07 * r, -0.36 * r), c, GOLD);
                rounded(Rect::new(a.x, a.y, r * 0.66, r * 0.13), r * 0.035, GOLD);
                draw_circle(c.x, c.y, r * 0.055, PINK);
            }
            3 | 4 | 5 | 7 => {
                let color = match index {
                    3 => BLUE,
                    4 => LILAC,
                    5 => GOLD,
                    _ => WHITE,
                };
                ellipse(c, vec2(r * 0.39, r * 0.16), color);
                ellipse(c + vec2(0., -r * 0.09), vec2(r * 0.28, r * 0.22), color);
                if index == 3 {
                    rounded(
                        Rect::new(c.x, c.y + r * 0.02, r * 0.49, r * 0.09),
                        r * 0.04,
                        BLUE,
                    );
                }
                if index == 4 {
                    draw_circle(c.x, c.y - r * 0.30, r * 0.08, PINK);
                }
                if index == 7 {
                    for dx in [-0.22, 0., 0.22] {
                        draw_circle(c.x + dx * r, c.y - r * 0.2, r * 0.15, WHITE);
                    }
                }
                if index == 5 {
                    rounded(
                        Rect::new(c.x - r * 0.30, c.y - r * 0.02, r * 0.60, r * 0.07),
                        r * 0.03,
                        PINK,
                    );
                }
            }
            6 => {
                ellipse(c, vec2(r * 0.43, r * 0.20), INK);
                draw_triangle(
                    c + vec2(-0.40, 0.03) * r,
                    c + vec2(0.40, 0.03) * r,
                    c + vec2(0., -0.39) * r,
                    INK,
                );
                draw_circle(c.x, c.y - r * 0.06, r * 0.07, WHITE);
            }
            8 => {
                draw_arc(
                    p.x,
                    p.y - r * 0.2,
                    64,
                    r * 0.68,
                    180.,
                    r * 0.08,
                    180.,
                    LILAC,
                );
                for dx in [-0.62, 0.63] {
                    ellipse(p + vec2(dx * r, -r * 0.16), vec2(r * 0.10, r * 0.22), PINK);
                }
            }
            9 => {
                let c = c + vec2(r * 0.36, r * 0.1);
                for i in 0..5 {
                    let a = i as f32 * std::f32::consts::TAU / 5.;
                    draw_circle(
                        c.x + a.cos() * r * 0.12,
                        c.y + a.sin() * r * 0.12,
                        r * 0.10,
                        PINK,
                    );
                }
                draw_circle(c.x, c.y, r * 0.085, GOLD);
            }
            10 | 11 => {
                let color = if index == 10 { LILAC } else { PINK };
                draw_triangle(
                    c + vec2(-0.33, 0.1) * r,
                    c + vec2(0.33, 0.1) * r,
                    c + vec2(-0.07, -0.57) * r,
                    color,
                );
                for v in [vec2(0., -0.23), vec2(0.1, 0.01), vec2(-0.1, -0.05)] {
                    draw_poly(c.x + v.x * r, c.y + v.y * r, 5, r * 0.035, 0., GOLD);
                }
            }
            _ => {}
        }
    }
}
pub fn ellipse(p: Vec2, r: Vec2, color: Color) {
    for i in 0..72 {
        let a = i as f32 * std::f32::consts::TAU / 72.;
        let b = (i + 1) as f32 * std::f32::consts::TAU / 72.;
        draw_triangle(
            p,
            p + vec2(a.cos() * r.x, a.sin() * r.y),
            p + vec2(b.cos() * r.x, b.sin() * r.y),
            color,
        );
    }
}
pub fn heart(p: Vec2, r: f32, color: Color) {
    draw_circle(p.x - r * 0.35, p.y - r * 0.18, r * 0.46, color);
    draw_circle(p.x + r * 0.35, p.y - r * 0.18, r * 0.46, color);
    draw_triangle(
        p + vec2(-0.77, -0.06) * r,
        p + vec2(0.77, -0.06) * r,
        p + vec2(0., 0.84) * r,
        color,
    );
}
pub fn bubble(p: Vec2, r: f32, color: Color) {
    draw_poly_lines(p.x, p.y, 64, r, 0., 2., color);
    draw_circle(p.x - r * 0.3, p.y - r * 0.4, r * 0.12, WHITE);
}
pub fn pearl(p: Vec2, r: f32) {
    draw_circle(p.x, p.y, r, BLUE);
    draw_circle(p.x, p.y, r * 0.87, color_u8!(235, 247, 253, 255));
    draw_circle(p.x - r * 0.3, p.y - r * 0.3, r * 0.23, WHITE);
}
pub fn glass(ui: &Ui, rect: Rect, r: f32) {
    bordered(
        rect,
        r,
        if ui.theme.saver {
            ui.theme.line
        } else {
            color_u8!(234, 227, 221, 255)
        },
        if ui.theme.saver {
            BLACK
        } else {
            color_u8!(255, 255, 255, 245)
        },
    );
}

#[derive(Clone, Copy)]
pub enum Glyph {
    Back,
    Next,
    Settings,
    Food,
    Soap,
    Shower,
    Moon,
    Heart,
    Play,
    Shirt,
    Shop,
    Fridge,
    Home,
    Palette,
    Hat,
    Wall,
    Pause,
    Close,
    Sun,
    Potion,
    Swim,
}
pub fn glyph(kind: Glyph, p: Vec2, r: f32) {
    let line = |a: Vec2, b: Vec2| {
        draw_line(
            p.x + a.x * r,
            p.y + a.y * r,
            p.x + b.x * r,
            p.y + b.y * r,
            2.5,
            LILAC,
        )
    };
    match kind {
        Glyph::Back | Glyph::Next => {
            let d = if matches!(kind, Glyph::Back) { -1. } else { 1. };
            line(vec2(-d * 0.3, -0.5), vec2(d * 0.3, 0.));
            line(vec2(d * 0.3, 0.), vec2(-d * 0.3, 0.5));
        }
        Glyph::Close => {
            line(vec2(-0.4, -0.4), vec2(0.4, 0.4));
            line(vec2(-0.4, 0.4), vec2(0.4, -0.4));
        }
        Glyph::Settings => {
            draw_poly(p.x, p.y, 8, r * 0.7, 22., LILAC);
            draw_circle(p.x, p.y, r * 0.35, WHITE);
            draw_circle(p.x, p.y, r * 0.16, INK);
        }
        Glyph::Food => food_icon(1, p, r),
        Glyph::Heart => heart(p, r * 0.7, PINK),
        Glyph::Soap => {
            rounded(
                Rect::new(p.x - r * 0.7, p.y - r * 0.35, r * 1.4, r * 0.8),
                r * 0.3,
                PINK,
            );
            bubble(p + vec2(r * 0.3, -r * 0.4), r * 0.3, BLUE);
            bubble(p + vec2(-r * 0.5, -r * 0.5), r * 0.18, BLUE);
        }
        Glyph::Shower => {
            line(vec2(-0.4, 0.5), vec2(-0.4, -0.5));
            line(vec2(-0.4, -0.5), vec2(0.4, -0.5));
            ellipse(p + vec2(r * 0.35, -r * 0.32), vec2(r * 0.4, r * 0.2), BLUE);
            for i in 0..3 {
                let x = 0.03 + i as f32 * 0.3;
                draw_line(
                    p.x + x * r,
                    p.y,
                    p.x + (x - 0.1) * r,
                    p.y + r * 0.5,
                    2.,
                    BLUE,
                );
            }
        }
        Glyph::Moon => {
            draw_circle(p.x, p.y, r * 0.65, LILAC);
            draw_circle(
                p.x + r * 0.3,
                p.y - r * 0.25,
                r * 0.50,
                color_u8!(255, 252, 248, 255),
            );
            draw_poly(p.x + r * 0.55, p.y + r * 0.4, 5, r * 0.18, -90., GOLD);
        }
        Glyph::Sun => {
            draw_circle(p.x, p.y, r * 0.4, GOLD);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.;
                let v = vec2(a.cos(), a.sin());
                draw_line(
                    p.x + v.x * r * 0.55,
                    p.y + v.y * r * 0.55,
                    p.x + v.x * r * 0.78,
                    p.y + v.y * r * 0.78,
                    2.,
                    GOLD,
                );
            }
        }
        Glyph::Play => {
            rounded(
                Rect::new(p.x - r * 0.8, p.y - r * 0.4, r * 1.6, r * 0.9),
                r * 0.3,
                LILAC,
            );
            line(vec2(-0.45, -0.08), vec2(-0.45, 0.28));
            line(vec2(-0.65, 0.10), vec2(-0.25, 0.10));
            draw_circle(p.x + r * 0.36, p.y, r * 0.10, PINK);
            draw_circle(p.x + r * 0.57, p.y + r * 0.2, r * 0.10, GOLD);
        }
        Glyph::Pause => {
            for dx in [-0.3, 0.15] {
                rounded(
                    Rect::new(p.x + dx * r, p.y - r * 0.5, r * 0.2, r),
                    r * 0.07,
                    LILAC,
                );
            }
        }
        Glyph::Shirt => {
            draw_triangle(
                p + vec2(-0.8, -0.25) * r,
                p + vec2(-0.35, -0.65) * r,
                p + vec2(0., 0.1) * r,
                PINK,
            );
            draw_triangle(
                p + vec2(0.8, -0.25) * r,
                p + vec2(0.35, -0.65) * r,
                p + vec2(0., 0.1) * r,
                PINK,
            );
            rounded(
                Rect::new(p.x - r * 0.43, p.y - r * 0.5, r * 0.86, r * 1.1),
                r * 0.15,
                PINK,
            );
            heart(p, r * 0.23, WHITE);
        }
        Glyph::Shop => {
            rounded(
                Rect::new(p.x - r * 0.6, p.y - r * 0.2, r * 1.2, r),
                r * 0.14,
                GOLD,
            );
            draw_poly_lines(p.x, p.y - r * 0.23, 32, r * 0.32, 0., 2., INK);
            heart(p + vec2(0., r * 0.26), r * 0.23, WHITE);
        }
        Glyph::Fridge => {
            rounded(
                Rect::new(p.x - r * 0.5, p.y - r * 0.75, r, r * 1.5),
                r * 0.16,
                MINT,
            );
            line(vec2(-0.43, -0.2), vec2(0.43, -0.2));
            line(vec2(0.25, -0.5), vec2(0.25, -0.3));
            line(vec2(0.25, 0.03), vec2(0.25, 0.35));
        }
        Glyph::Home => {
            draw_triangle(
                p + vec2(-0.75, -0.1) * r,
                p + vec2(0.75, -0.1) * r,
                p + vec2(0., -0.75) * r,
                PINK,
            );
            rounded(
                Rect::new(p.x - r * 0.55, p.y - r * 0.12, r * 1.1, r * 0.9),
                r * 0.1,
                GOLD,
            );
            rounded(
                Rect::new(p.x - r * 0.18, p.y + r * 0.2, r * 0.36, r * 0.57),
                r * 0.08,
                WHITE,
            );
        }
        Glyph::Palette => {
            ellipse(p, vec2(r * 0.75, r * 0.6), PINK);
            for (v, c) in [
                (vec2(-0.35, -0.1), BLUE),
                (vec2(0., -0.3), GOLD),
                (vec2(0.35, -0.1), MINT),
            ] {
                draw_circle(p.x + v.x * r, p.y + v.y * r, r * 0.16, c);
            }
        }
        Glyph::Hat => {
            ellipse(p + vec2(0., r * 0.3), vec2(r * 0.8, r * 0.18), GOLD);
            rounded(
                Rect::new(p.x - r * 0.44, p.y - r * 0.5, r * 0.88, r * 0.8),
                r * 0.2,
                GOLD,
            );
            draw_line(
                p.x - r * 0.4,
                p.y + r * 0.1,
                p.x + r * 0.4,
                p.y + r * 0.1,
                3.,
                PINK,
            );
        }
        Glyph::Wall => {
            rounded(
                Rect::new(p.x - r * 0.6, p.y - r * 0.65, r * 1.2, r * 1.3),
                r * 0.1,
                BLUE,
            );
            for x in [-0.3, 0.3] {
                for y in [-0.3, 0.3] {
                    draw_poly(p.x + x * r, p.y + y * r, 5, r * 0.15, 0., WHITE);
                }
            }
        }
        Glyph::Potion => {
            rounded(
                Rect::new(p.x - r * 0.18, p.y - r * 0.7, r * 0.36, r * 0.5),
                r * 0.06,
                BLUE,
            );
            draw_circle(p.x, p.y + r * 0.15, r * 0.57, MINT);
            heart(p + vec2(0., r * 0.16), r * 0.28, WHITE);
        }
        Glyph::Swim => {
            ellipse(p, vec2(r * 0.6, r * 0.4), PINK);
            draw_triangle(
                p + vec2(-0.4, 0.) * r,
                p + vec2(-0.9, -0.4) * r,
                p + vec2(-0.9, 0.4) * r,
                PINK,
            );
            draw_circle(p.x + r * 0.3, p.y - r * 0.1, r * 0.1, INK);
        }
    }
}
pub fn icon_button(ui: &mut Ui, kind: Glyph, rect: Rect, selected: bool) -> bool {
    glass(ui, rect, rect.h * 0.34);
    if selected {
        draw_rectangle_lines(
            rect.x + 1.,
            rect.y + 1.,
            rect.w - 2.,
            rect.h - 2.,
            2.,
            LILAC,
        );
    }
    glyph(kind, rect.center(), rect.w.min(rect.h) * 0.32);
    ui.hit(rect)
}
pub fn food_icon(index: usize, p: Vec2, r: f32) {
    match index {
        0 => {
            for (dx, dy, c) in [
                (-0.3, -0.1, GOLD),
                (0.3, 0.2, PINK),
                (0., 0.5, GOLD),
                (0.2, -0.4, GOLD),
                (-0.4, 0.4, MINT),
            ] {
                ellipse(p + vec2(dx, dy) * r, vec2(r * 0.2, r * 0.13), c);
            }
        }
        1 => {
            draw_circle(p.x - r * 0.2, p.y, r * 0.53, color_u8!(228, 120, 114, 255));
            draw_circle(p.x + r * 0.2, p.y, r * 0.53, color_u8!(228, 120, 114, 255));
            draw_line(p.x, p.y - r * 0.4, p.x + r * 0.1, p.y - r * 0.8, 2.5, INK);
            ellipse(
                p + vec2(r * 0.28, -r * 0.55),
                vec2(r * 0.25, r * 0.12),
                MINT,
            );
            draw_circle(
                p.x - r * 0.28,
                p.y - r * 0.22,
                r * 0.12,
                color_u8!(255, 204, 189, 255),
            );
        }
        2 => {
            for v in [vec2(-0.3, 0.2), vec2(0.3, 0.2), vec2(0., -0.3)] {
                draw_circle(p.x + v.x * r, p.y + v.y * r, r * 0.35, LILAC);
                draw_poly(
                    p.x + v.x * r,
                    p.y + v.y * r - r * 0.15,
                    5,
                    r * 0.10,
                    0.,
                    BLUE,
                );
            }
        }
        3 => {
            for dx in [-0.4, 0., 0.4] {
                draw_line(
                    p.x + dx * r,
                    p.y + r * 0.7,
                    p.x + (dx + 0.12) * r,
                    p.y - r * 0.6,
                    r * 0.19,
                    MINT,
                );
            }
        }
        4 => {
            rounded(
                Rect::new(p.x - r * 0.62, p.y - r * 0.35, r * 1.24, r * 0.8),
                r * 0.25,
                INK,
            );
            ellipse(p + vec2(0., -r * 0.28), vec2(r * 0.62, r * 0.33), WHITE);
            ellipse(p + vec2(0., -r * 0.28), vec2(r * 0.31, r * 0.19), PINK);
            draw_circle(p.x + r * 0.06, p.y - r * 0.29, r * 0.10, MINT);
        }
        5 => {
            draw_arc(p.x, p.y, 32, r * 0.48, 20., r * 0.32, 255., PINK);
            for i in 0..3 {
                draw_line(
                    p.x - r * 0.3 + i as f32 * r * 0.2,
                    p.y - r * 0.3,
                    p.x - r * 0.1 + i as f32 * r * 0.2,
                    p.y + r * 0.05,
                    1.5,
                    WHITE,
                );
            }
            draw_triangle(
                p + vec2(0.4, 0.4) * r,
                p + vec2(0.8, 0.1) * r,
                p + vec2(0.8, 0.65) * r,
                PINK,
            );
        }
        6 => {
            draw_triangle(
                p + vec2(-0.65, 0.5) * r,
                p + vec2(0.65, 0.5) * r,
                p + vec2(-0.65, -0.6) * r,
                GOLD,
            );
            draw_line(
                p.x - r * 0.55,
                p.y + r * 0.35,
                p.x + r * 0.45,
                p.y + r * 0.35,
                4.,
                MINT,
            );
            draw_line(
                p.x - r * 0.55,
                p.y + r * 0.20,
                p.x + r * 0.28,
                p.y + r * 0.20,
                3.,
                PINK,
            );
        }
        7 => {
            ellipse(p, vec2(r * 0.7, r * 0.55), BLUE);
            ellipse(p + vec2(0., -r * 0.15), vec2(r * 0.62, r * 0.25), GOLD);
            for dx in [-0.3, 0.2] {
                draw_line(
                    p.x + dx * r,
                    p.y - r * 0.5,
                    p.x + (dx + 0.1) * r,
                    p.y - r * 0.8,
                    1.5,
                    INK,
                );
            }
        }
        8 => {
            rounded(
                Rect::new(p.x - r * 0.65, p.y - r * 0.18, r * 1.3, r * 0.8),
                r * 0.06,
                PINK,
            );
            rounded(
                Rect::new(p.x - r * 0.65, p.y - r * 0.35, r * 1.3, r * 0.25),
                r * 0.1,
                WHITE,
            );
            draw_line(
                p.x - r * 0.6,
                p.y + r * 0.23,
                p.x + r * 0.6,
                p.y + r * 0.23,
                3.,
                WHITE,
            );
            draw_circle(p.x, p.y - r * 0.51, r * 0.16, PINK);
        }
        9 => {
            draw_circle(p.x, p.y, r * 0.68, GOLD);
            draw_circle(p.x, p.y, r * 0.54, PINK);
            draw_circle(p.x, p.y, r * 0.22, color_u8!(255, 252, 248, 255));
            for v in [vec2(-0.35, -0.2), vec2(0.3, 0.25), vec2(0.2, -0.35)] {
                draw_line(
                    p.x + v.x * r,
                    p.y + v.y * r,
                    p.x + (v.x + 0.15) * r,
                    p.y + (v.y + 0.1) * r,
                    2.,
                    WHITE,
                );
            }
        }
        10 => {
            draw_triangle(
                p + vec2(-0.45, -0.1) * r,
                p + vec2(0.45, -0.1) * r,
                p + vec2(0., 0.85) * r,
                GOLD,
            );
            draw_circle(p.x, p.y - r * 0.35, r * 0.49, PINK);
            draw_circle(p.x + r * 0.26, p.y - r * 0.27, r * 0.2, WHITE);
        }
        _ => {
            rounded(
                Rect::new(p.x - r * 0.4, p.y - r * 0.45, r * 0.8, r * 1.1),
                r * 0.12,
                GOLD,
            );
            draw_line(
                p.x + r * 0.1,
                p.y - r * 0.3,
                p.x + r * 0.3,
                p.y - r * 0.85,
                3.,
                PINK,
            );
            ellipse(p + vec2(0., r * 0.2), vec2(r * 0.37, r * 0.19), PINK);
        }
    }
}
