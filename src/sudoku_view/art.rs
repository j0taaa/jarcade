use super::*;
pub const BLUE: Color = color_u8!(76, 89, 193, 255);
pub fn colour(n: u8, saver: bool) -> Color {
    let colours = [
        color_u8!(233, 190, 183, 255),
        color_u8!(245, 216, 159, 255),
        color_u8!(198, 217, 183, 255),
        color_u8!(175, 220, 217, 255),
        color_u8!(180, 202, 232, 255),
        color_u8!(199, 189, 231, 255),
        color_u8!(228, 192, 219, 255),
        color_u8!(213, 210, 202, 255),
        color_u8!(163, 197, 202, 255),
    ];
    let c = colours[(n.saturating_sub(1) as usize).min(8)];
    if saver {
        Color::new(c.r, c.g, c.b, 0.23)
    } else {
        c
    }
}
#[derive(Clone, Copy)]
pub enum Glyph {
    Back,
    Help,
    Undo,
    Redo,
    Erase,
    Check,
    Hint,
    Reset,
}
pub fn glyph(ui: &Ui, g: Glyph, r: Rect) {
    let c = r.center();
    let ink = ui.theme.text;
    let line = |a: Vec2, b: Vec2| draw_line(c.x + a.x, c.y + a.y, c.x + b.x, c.y + b.y, 1.9, ink);
    match g {
        Glyph::Back => {
            line(vec2(8., 0.), vec2(-8., 0.));
            line(vec2(-8., 0.), vec2(-2., -6.));
            line(vec2(-8., 0.), vec2(-2., 6.));
        }
        Glyph::Help => {
            draw_circle_lines(c.x, c.y, 10., 1.6, ink);
            ui.centered("?", r, 17., ink, true);
        }
        Glyph::Undo | Glyph::Redo => {
            let d = if matches!(g, Glyph::Redo) { -1. } else { 1. };
            line(vec2(-8. * d, -3.), vec2(1. * d, -3.));
            line(vec2(-8. * d, -3.), vec2(-3. * d, -8.));
            line(vec2(-8. * d, -3.), vec2(-3. * d, 2.));
            for i in 0..16 {
                let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / 16.;
                let b = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * (i + 1) as f32 / 16.;
                line(
                    vec2((1. + a.cos() * 5.) * d, 2. + a.sin() * 5.),
                    vec2((1. + b.cos() * 5.) * d, 2. + b.sin() * 5.),
                );
            }
            line(vec2(1. * d, 7.), vec2(-4. * d, 7.));
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
        Glyph::Erase => {
            let p = [
                vec2(-10., -5.),
                vec2(-5., -9.),
                vec2(10., 6.),
                vec2(5., 10.),
            ];
            for i in 0..4 {
                line(p[i], p[(i + 1) % 4]);
            }
            line(vec2(-2., 3.), vec2(3., -2.));
            line(vec2(-10., 11.), vec2(11., 11.));
        }
        Glyph::Check => {
            line(vec2(-9., 0.), vec2(-3., 6.));
            line(vec2(-3., 6.), vec2(10., -7.));
        }
        Glyph::Hint => {
            draw_circle_lines(c.x, c.y - 3., 6., 1.7, ink);
            line(vec2(-3., 3.), vec2(-3., 7.));
            line(vec2(3., 3.), vec2(3., 7.));
            line(vec2(-3., 7.), vec2(3., 7.));
            line(vec2(-2., 10.), vec2(2., 10.));
        }
    }
}
pub fn tool_icon(ui: &Ui, tool: Tool, r: Rect, active: bool) {
    let c = if active {
        if ui.theme.saver {
            ui.theme.accent
        } else {
            WHITE
        }
    } else {
        ui.theme.text
    };
    match tool {
        Tool::Digit => ui.centered("9", r, 23., c, true),
        Tool::Corner => {
            draw_rectangle_lines(r.center().x - 12., r.center().y - 12., 24., 24., 1.2, c);
            ui.label("1 2", r.center().x - 10., r.center().y - 3., 10., c);
            ui.label("3", r.center().x - 10., r.center().y + 10., 10., c);
        }
        Tool::Centre => {
            draw_rectangle_lines(r.center().x - 12., r.center().y - 12., 24., 24., 1.2, c);
            ui.centered("123", r, 10., c, true);
        }
        Tool::Colour => {
            for i in 0..4 {
                draw_circle(
                    r.center().x - 5. + (i % 2) as f32 * 10.,
                    r.center().y - 5. + (i / 2) as f32 * 10.,
                    4.,
                    colour(i + 2, false),
                );
            }
        }
    }
}
#[derive(Default)]
pub struct Highlights<'a> {
    pub selected: &'a [usize],
    pub matches: &'a [usize],
    pub candidates: &'a [usize],
    pub mistakes: &'a [usize],
}
pub fn draw_board(
    ui: &Ui,
    p: &Puzzle,
    marks: &[Mark],
    highlights: Highlights<'_>,
    r: Rect,
    preview: bool,
) {
    let u = r.w / 9.;
    draw_rectangle(r.x, r.y, r.w, r.h, ui.theme.bg);
    for i in 0..81 {
        let cell = Rect::new(r.x + (i % 9) as f32 * u, r.y + (i / 9) as f32 * u, u, u);
        let mark = marks.get(i).copied().unwrap_or_default();
        if mark.colour > 0 {
            draw_rectangle(
                cell.x,
                cell.y,
                cell.w,
                cell.h,
                colour(mark.colour, ui.theme.saver),
            );
        }
        if highlights.candidates.contains(&i) {
            draw_rectangle(
                cell.x,
                cell.y,
                cell.w,
                cell.h,
                if ui.theme.saver {
                    Color::new(0.43, 0.32, 0.82, 0.13)
                } else {
                    color_u8!(246, 242, 255, 255)
                },
            );
        }
        if highlights.matches.contains(&i) {
            draw_rectangle(
                cell.x,
                cell.y,
                cell.w,
                cell.h,
                if ui.theme.saver {
                    Color::new(0.43, 0.32, 0.82, 0.27)
                } else {
                    color_u8!(234, 226, 254, 255)
                },
            );
        }
        if highlights.selected.contains(&i) {
            draw_rectangle(
                cell.x,
                cell.y,
                cell.w,
                cell.h,
                if ui.theme.saver {
                    Color::new(0.32, 0.38, 0.8, 0.23)
                } else {
                    color_u8!(230, 234, 254, 255)
                },
            );
        }
        if highlights.mistakes.contains(&i) {
            draw_rectangle(
                cell.x,
                cell.y,
                cell.w,
                cell.h,
                if ui.theme.saver {
                    Color::new(0.9, 0.3, 0.3, 0.2)
                } else {
                    color_u8!(255, 228, 224, 255)
                },
            );
        }
    }
    if p.variant == Variant::Diagonal {
        let c = if ui.theme.saver {
            Color::new(0.4, 0.45, 0.8, 0.3)
        } else {
            color_u8!(225, 222, 245, 255)
        };
        draw_line(r.x, r.y, r.right(), r.bottom(), u * 0.13, c);
        draw_line(r.right(), r.y, r.x, r.bottom(), u * 0.13, c);
    }
    for t in &p.thermos {
        let point = |i: usize| {
            vec2(
                r.x + (i % 9) as f32 * u + u / 2.,
                r.y + (i / 9) as f32 * u + u / 2.,
            )
        };
        let c = if ui.theme.saver {
            color_u8!(66, 56, 88, 255)
        } else {
            color_u8!(221, 208, 242, 255)
        };
        for pair in t.windows(2) {
            let a = point(pair[0]);
            let b = point(pair[1]);
            draw_line(a.x, a.y, b.x, b.y, u * 0.23, c);
            draw_circle(b.x, b.y, u * 0.115, c);
        }
        let bulb = point(t[0]);
        draw_circle(bulb.x, bulb.y, u * 0.26, c);
    }
    for i in 0..=9 {
        let pos = i as f32 * u;
        let thick = if i % 3 == 0 {
            (u * 0.05).clamp(1.3, 2.8)
        } else {
            0.8
        };
        let c = if i % 3 == 0 {
            ui.theme.text
        } else {
            ui.theme.line
        };
        draw_line(r.x + pos, r.y, r.x + pos, r.bottom(), thick, c);
        draw_line(r.x, r.y + pos, r.right(), r.y + pos, thick, c);
    }
    for cage in &p.cages {
        let inset = u * 0.09;
        let c = ui.theme.muted;
        for &i in &cage.cells {
            let x = r.x + (i % 9) as f32 * u;
            let y = r.y + (i / 9) as f32 * u;
            let boundaries = [
                (
                    i < 9 || !cage.cells.contains(&(i - 9)),
                    vec2(x + inset, y + inset),
                    vec2(x + u - inset, y + inset),
                ),
                (
                    i >= 72 || !cage.cells.contains(&(i + 9)),
                    vec2(x + inset, y + u - inset),
                    vec2(x + u - inset, y + u - inset),
                ),
                (
                    i % 9 == 0 || !cage.cells.contains(&(i - 1)),
                    vec2(x + inset, y + inset),
                    vec2(x + inset, y + u - inset),
                ),
                (
                    i % 9 == 8 || !cage.cells.contains(&(i + 1)),
                    vec2(x + u - inset, y + inset),
                    vec2(x + u - inset, y + u - inset),
                ),
            ];
            for (edge, a, b) in boundaries {
                if edge {
                    let length = a.distance(b);
                    let pieces = (length / 4.).ceil() as usize;
                    for j in 0..pieces {
                        let start = a.lerp(b, j as f32 / pieces as f32);
                        let end = a.lerp(b, (j as f32 + 0.5) / pieces as f32);
                        draw_line(start.x, start.y, end.x, end.y, 0.9, c);
                    }
                }
            }
        }
        let i = cage.cells[0];
        let x = r.x + (i % 9) as f32 * u;
        let y = r.y + (i / 9) as f32 * u;
        let label = cage.sum.to_string();
        let size = (u * 0.21).clamp(if preview { 2. } else { 7. }, 13.);
        let width = ui.text_width(&label, size, false) + 3.;
        draw_rectangle(x + u * 0.09, y + u * 0.04, width, u * 0.23, ui.theme.bg);
        ui.label(&label, x + u * 0.12, y + u * 0.24, size, c);
    }
    for i in 0..81 {
        let cell = Rect::new(r.x + (i % 9) as f32 * u, r.y + (i / 9) as f32 * u, u, u);
        let mark = marks.get(i).copied().unwrap_or_default();
        let given = p.givens[i];
        let value = if given > 0 { given } else { mark.value };
        if value > 0 {
            ui.centered(
                &value.to_string(),
                cell,
                (u * 0.56).clamp(if preview { 2. } else { 8. }, 38.),
                if highlights.mistakes.contains(&i) {
                    if ui.theme.saver {
                        color_u8!(255, 151, 142, 255)
                    } else {
                        color_u8!(196, 58, 55, 255)
                    }
                } else if given > 0 {
                    ui.theme.text
                } else {
                    if ui.theme.saver {
                        color_u8!(160, 175, 255, 255)
                    } else {
                        BLUE
                    }
                },
                given > 0,
            );
        } else if !preview {
            if mark.centre > 0 {
                let s = (1..=9)
                    .filter(|&d| mark.centre & bit(d) != 0)
                    .map(|d| d.to_string())
                    .collect::<String>();
                let font = (u * 0.25)
                    .min(u * 0.82 / s.len() as f32 * 1.65)
                    .clamp(6., 17.);
                ui.centered(&s, cell, font, ui.theme.text, false);
            }
            for d in 1..=9 {
                if mark.corner & bit(d) != 0 {
                    let j = d - 1;
                    let cy = if p.variant == Variant::Killer {
                        0.37 + j as f32 / 3. * 0.26
                    } else {
                        0.2 + (j / 3) as f32 * 0.3
                    };
                    let cy = if p.variant == Variant::Killer {
                        0.34 + (j / 3) as f32 * 0.24
                    } else {
                        cy
                    };
                    ui.centered(
                        &d.to_string(),
                        Rect::new(
                            cell.x + (j % 3) as f32 * u / 3.,
                            cell.y + cy * u - u * 0.13,
                            u / 3.,
                            u * 0.26,
                        ),
                        (u * 0.22).clamp(6., 14.),
                        ui.theme.muted,
                        false,
                    );
                }
            }
        }
        if highlights.selected.contains(&i) {
            draw_rectangle_lines(
                cell.x + 1.5,
                cell.y + 1.5,
                cell.w - 3.,
                cell.h - 3.,
                1.6,
                ui.theme.accent,
            );
        }
    }
    for e in &p.edges {
        let a = vec2(
            r.x + (e.a % 9) as f32 * u + u / 2.,
            r.y + (e.a / 9) as f32 * u + u / 2.,
        );
        let b = vec2(
            r.x + (e.b % 9) as f32 * u + u / 2.,
            r.y + (e.b / 9) as f32 * u + u / 2.,
        );
        let c = (a + b) * 0.5;
        match e.relation {
            Relation::Consecutive | Relation::Double => {
                let filled = e.relation == Relation::Double;
                draw_circle(
                    c.x,
                    c.y,
                    u * 0.105,
                    if filled { ui.theme.text } else { ui.theme.bg },
                );
                draw_circle_lines(c.x, c.y, u * 0.105, 1.1, ui.theme.text);
            }
            Relation::Five | Relation::Ten => {
                draw_circle(c.x, c.y, u * 0.15, ui.theme.bg);
                ui.centered(
                    if e.relation == Relation::Five {
                        "V"
                    } else {
                        "X"
                    },
                    Rect::new(c.x - u * 0.2, c.y - u * 0.2, u * 0.4, u * 0.4),
                    (u * 0.31).clamp(if preview { 3. } else { 7. }, 18.),
                    ui.theme.text,
                    true,
                );
            }
        }
    }
}
