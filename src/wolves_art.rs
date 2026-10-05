//! Original, resolution-independent village portraits and tabletop preview.
use crate::{
    online_view::fit,
    ui::{Ui, bordered, rounded},
};
use jarcade::multiplayer::wolves::Role;
use macroquad::prelude::*;

pub fn moon(c: Vec2, r: f32, ink: Color, background: Color) {
    draw_circle(c.x, c.y, r, ink);
    draw_circle(c.x + r * 0.45, c.y - r * 0.3, r * 0.85, background);
}
pub fn portrait(c: Vec2, r: f32, seat: usize, role: Option<Role>, saver: bool) {
    let wolf = role.is_some_and(Role::wolf);
    let coat = [
        color_u8!(152, 172, 216, 255),
        color_u8!(176, 157, 200, 255),
        color_u8!(129, 183, 164, 255),
        color_u8!(229, 170, 145, 255),
    ][seat % 4];
    let ink = color_u8!(53, 53, 80, 255);
    let skin = color_u8!(250, 218, 188, 255);
    let hair = [
        color_u8!(97, 67, 62, 255),
        color_u8!(216, 158, 77, 255),
        color_u8!(69, 66, 97, 255),
    ][seat % 3];
    draw_circle(
        c.x,
        c.y,
        r,
        if saver {
            BLACK
        } else {
            color_u8!(239, 237, 249, 255)
        },
    );
    draw_ellipse(c.x, c.y + r * 0.6, r * 0.76, r * 0.35, 0., coat);
    let face = c + vec2(0., -r * 0.12);
    if wolf {
        let fur = color_u8!(133, 145, 186, 255);
        for sign in [-1., 1.] {
            draw_triangle(
                face + vec2(sign * r * 0.7, -r * 0.12),
                face + vec2(sign * r * 0.72, -r * 0.9),
                face + vec2(sign * r * 0.08, -r * 0.35),
                fur,
            );
            draw_triangle(
                face + vec2(sign * r * 0.57, -r * 0.3),
                face + vec2(sign * r * 0.62, -r * 0.72),
                face + vec2(sign * r * 0.3, -r * 0.34),
                color_u8!(216, 184, 203, 255),
            );
        }
        draw_ellipse(face.x, face.y, r * 0.65, r * 0.65, 0., fur);
        draw_ellipse(
            face.x,
            face.y + r * 0.27,
            r * 0.4,
            r * 0.26,
            0.,
            color_u8!(233, 231, 241, 255),
        );
        draw_triangle(
            face + vec2(-r * 0.12, r * 0.13),
            face + vec2(r * 0.12, r * 0.13),
            face + vec2(0., r * 0.3),
            ink,
        );
    } else {
        draw_circle(face.x, face.y - r * 0.1, r * 0.66, hair);
        draw_ellipse(face.x, face.y + r * 0.06, r * 0.57, r * 0.6, 0., skin);
        for i in 0..3 {
            draw_circle(
                face.x + (i as f32 - 1.) * r * 0.28,
                face.y - r * 0.41,
                r * 0.22,
                hair,
            );
        }
        draw_line(
            face.x - r * 0.13,
            face.y + r * 0.31,
            face.x + r * 0.13,
            face.y + r * 0.31,
            r * 0.06,
            ink,
        );
    }
    for sign in [-1., 1.] {
        draw_circle(face.x + sign * r * 0.23, face.y + r * 0.01, r * 0.06, ink);
    }
    if matches!(
        role,
        Some(Role::Seer | Role::AuraSeer | Role::WolfSeer | Role::Witch | Role::Medium)
    ) {
        draw_triangle(
            c + vec2(-r * 0.68, -r * 0.58),
            c + vec2(r * 0.68, -r * 0.58),
            c + vec2(r * 0.12, -r * 1.06),
            coat,
        );
        draw_ellipse(c.x, c.y - r * 0.58, r * 0.8, r * 0.14, 0., coat);
    }
    if matches!(role, Some(Role::Doctor)) {
        rounded(
            Rect::new(c.x - r * 0.5, c.y - r * 0.72, r, r * 0.33),
            r * 0.1,
            WHITE,
        );
        draw_line(
            c.x,
            c.y - r * 0.68,
            c.x,
            c.y - r * 0.45,
            r * 0.07,
            color_u8!(195, 101, 124, 255),
        );
        draw_line(
            c.x - r * 0.11,
            c.y - r * 0.565,
            c.x + r * 0.11,
            c.y - r * 0.565,
            r * 0.07,
            color_u8!(195, 101, 124, 255),
        );
    }

    if matches!(role, Some(Role::Bodyguard | Role::ToughGuy)) {
        let shield = c + vec2(0., r * 0.52);
        draw_triangle(
            shield + vec2(-r * 0.26, -r * 0.18),
            shield + vec2(r * 0.26, -r * 0.18),
            shield + vec2(0., r * 0.28),
            ink,
        );
        draw_line(
            shield.x,
            shield.y - r * 0.12,
            shield.x,
            shield.y + r * 0.1,
            r * 0.07,
            coat,
        );
    }
    if role == Some(Role::Gunner) {
        draw_line(
            c.x - r * 0.55,
            c.y + r * 0.39,
            c.x + r * 0.5,
            c.y + r * 0.79,
            r * 0.15,
            hair,
        );
        for i in 0..3 {
            draw_circle(
                c.x + (i as f32 - 1.) * r * 0.25,
                c.y + r * (0.6 + (i as f32 - 1.) * 0.09),
                r * 0.05,
                color_u8!(239, 189, 92, 255),
            );
        }
    }
    if role == Some(Role::Fool) {
        for sign in [-1., 1.] {
            draw_triangle(
                c + vec2(-r * 0.55, -r * 0.5),
                c + vec2(r * 0.55, -r * 0.5),
                c + vec2(sign * r * 0.6, -r * 0.9),
                if sign < 0. {
                    coat
                } else {
                    color_u8!(224, 145, 176, 255)
                },
            );
            draw_circle(
                c.x + sign * r * 0.6,
                c.y - r * 0.9,
                r * 0.1,
                color_u8!(242, 190, 94, 255),
            );
        }
    }
    if role == Some(Role::SerialKiller) {
        rounded(
            Rect::new(c.x - r * 0.55, c.y - r * 0.28, r * 1.1, r * 0.26),
            r * 0.08,
            ink,
        );
        for sign in [-1., 1.] {
            draw_circle(c.x + sign * r * 0.23, c.y - r * 0.15, r * 0.07, WHITE);
        }
    }
    if role == Some(Role::Avenger) {
        draw_line(
            c.x - r * 0.6,
            c.y - r * 0.36,
            c.x + r * 0.6,
            c.y - r * 0.36,
            r * 0.17,
            color_u8!(181, 89, 115, 255),
        );
    }
    if role == Some(Role::AlphaWolf) {
        crate::online_style::crown(
            c + vec2(0., -r * 0.82),
            r * 0.3,
            color_u8!(230, 182, 97, 255),
        );
    }
}
pub fn preview(ui: &Ui, r: Rect) {
    let ink = if ui.theme.saver {
        color_u8!(189, 196, 244, 255)
    } else {
        color_u8!(81, 83, 135, 255)
    };
    let board = Rect::new(r.x + r.w * 0.05, r.y + r.h * 0.1, r.w * 0.9, r.h * 0.8);
    bordered(
        board,
        r.w * 0.08,
        ink,
        if ui.theme.saver {
            BLACK
        } else {
            color_u8!(246, 245, 253, 255)
        },
    );
    moon(
        vec2(board.x + board.w * 0.15, board.y + board.h * 0.16),
        r.w * 0.045,
        ink,
        if ui.theme.saver {
            BLACK
        } else {
            color_u8!(246, 245, 253, 255)
        },
    );
    fit(
        ui,
        "NIGHT 1",
        Rect::new(
            board.x + board.w * 0.24,
            board.y + board.h * 0.06,
            board.w * 0.6,
            board.h * 0.2,
        ),
        r.w * 0.065,
        ink,
        true,
    );
    for i in 0..6 {
        let p = vec2(
            board.x + board.w * (0.19 + (i % 3) as f32 * 0.31),
            board.y + board.h * (0.42 + (i / 3) as f32 * 0.32),
        );
        portrait(
            p,
            r.w * 0.087,
            i,
            if i == 0 { Some(Role::Werewolf) } else { None },
            ui.theme.saver,
        );
        if i == 4 {
            draw_circle_lines(p.x, p.y, r.w * 0.106, 2., ink);
        }
    }
}
