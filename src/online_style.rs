//! Small vector pieces for the card table and the dream gallery.
use crate::ui::{Ui, bordered, rounded};
use jarcade::multiplayer::{
    GameKind,
    coup::{Action, Role},
};
use macroquad::prelude::*;

pub fn palette(game: GameKind, saver: bool) -> (Color, Color, Color) {
    match (game, saver) {
        (GameKind::Court, false) => (
            color_u8!(66, 81, 66, 255),
            color_u8!(247, 244, 234, 255),
            color_u8!(232, 227, 210, 255),
        ),
        (GameKind::Reverie, false) => (
            color_u8!(113, 86, 177, 255),
            color_u8!(246, 242, 252, 255),
            color_u8!(230, 221, 244, 255),
        ),
        (GameKind::Court, true) => (
            color_u8!(207, 213, 163, 255),
            BLACK,
            color_u8!(57, 63, 47, 255),
        ),
        (GameKind::Wolves, false) => (
            color_u8!(81, 83, 135, 255),
            color_u8!(246, 245, 253, 255),
            color_u8!(225, 225, 242, 255),
        ),
        (GameKind::Wolves, true) => (
            color_u8!(189, 196, 244, 255),
            BLACK,
            color_u8!(49, 51, 72, 255),
        ),
        (GameKind::Reverie, true) => (
            color_u8!(203, 182, 244, 255),
            BLACK,
            color_u8!(58, 47, 73, 255),
        ),
    }
}

pub fn coin(center: Vec2, radius: f32) {
    draw_circle(
        center.x,
        center.y + radius * 0.12,
        radius,
        color_u8!(193, 137, 45, 255),
    );
    draw_circle(center.x, center.y, radius, color_u8!(244, 192, 78, 255));
    draw_circle_lines(
        center.x,
        center.y,
        radius * 0.70,
        radius * 0.10,
        color_u8!(255, 222, 132, 255),
    );
    draw_line(
        center.x,
        center.y - radius * 0.32,
        center.x,
        center.y + radius * 0.32,
        radius * 0.14,
        color_u8!(162, 109, 31, 255),
    );
}

pub fn spark(center: Vec2, radius: f32, ink: Color) {
    for sign in [-1., 1.] {
        draw_triangle(
            center + vec2(0., sign * radius),
            center + vec2(-radius * 0.27, 0.),
            center + vec2(radius * 0.27, 0.),
            ink,
        );
        draw_triangle(
            center + vec2(sign * radius, 0.),
            center + vec2(0., -radius * 0.27),
            center + vec2(0., radius * 0.27),
            ink,
        );
    }
}

pub fn crown(center: Vec2, radius: f32, ink: Color) {
    rounded(
        Rect::new(center.x - radius, center.y, radius * 2., radius * 0.60),
        radius * 0.16,
        ink,
    );
    for i in -1..=1 {
        draw_triangle(
            center + vec2(i as f32 * radius * 0.65 - radius * 0.4, radius * 0.12),
            center + vec2(i as f32 * radius * 0.65 + radius * 0.4, radius * 0.12),
            center
                + vec2(
                    i as f32 * radius * 0.65,
                    -radius * if i == 0 { 0.85 } else { 0.5 },
                ),
            ink,
        );
    }
}

pub fn emblem(game: GameKind, center: Vec2, radius: f32, ink: Color) {
    if game == GameKind::Court {
        crown(center, radius, ink);
    } else if game == GameKind::Wolves {
        crate::wolves_art::moon(center, radius, ink, if ink.r > 0.6 { BLACK } else { WHITE });
    } else {
        spark(center, radius, ink);
    }
}

pub fn avatar(ui: &Ui, name: &str, index: usize, center: Vec2, radius: f32) {
    const COLORS: [Color; 8] = [
        color_u8!(236, 214, 170, 255),
        color_u8!(219, 208, 247, 255),
        color_u8!(194, 226, 225, 255),
        color_u8!(249, 211, 200, 255),
        color_u8!(216, 231, 182, 255),
        color_u8!(208, 218, 246, 255),
        color_u8!(242, 213, 235, 255),
        color_u8!(240, 228, 184, 255),
    ];
    draw_circle(
        center.x,
        center.y,
        radius,
        if ui.theme.saver {
            BLACK
        } else {
            COLORS[index % COLORS.len()]
        },
    );
    if ui.theme.saver {
        draw_circle_lines(center.x, center.y, radius, 1., ui.theme.accent);
    }
    let initial = name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    ui.centered(
        &initial,
        Rect::new(
            center.x - radius,
            center.y - radius,
            radius * 2.,
            radius * 2.,
        ),
        radius,
        ui.theme.text,
        true,
    );
}

pub fn check(center: Vec2, size: f32, ink: Color) {
    draw_line(
        center.x - size * 0.5,
        center.y,
        center.x - size * 0.12,
        center.y + size * 0.35,
        2.,
        ink,
    );
    draw_line(
        center.x - size * 0.12,
        center.y + size * 0.35,
        center.x + size * 0.6,
        center.y - size * 0.4,
        2.,
        ink,
    );
}

pub fn primary(ui: &mut Ui, text: &str, rect: Rect, enabled: bool) -> bool {
    let fill = if enabled && !ui.theme.saver {
        ui.theme.accent
    } else {
        ui.theme.bg
    };
    bordered(
        rect,
        18.,
        if enabled {
            ui.theme.accent
        } else {
            ui.theme.line
        },
        fill,
    );
    let size = (16. * (rect.w - 24.) / ui.text_width(text, 16., true).max(1.)).clamp(10., 16.);
    ui.centered(
        text,
        rect,
        size,
        if enabled && !ui.theme.saver {
            WHITE
        } else if enabled {
            ui.theme.accent
        } else {
            ui.theme.muted
        },
        true,
    );
    // Keep disabled controls in the focus order, without sending a move.
    ui.hit(rect) && enabled
}

pub fn action_tile(
    ui: &mut Ui,
    action: Action,
    rect: Rect,
    tap: Option<Vec2>,
    viewport: Rect,
) -> bool {
    let (wash, ink) = match action.role() {
        Some(Role::Regent) => (color_u8!(253, 241, 207, 255), color_u8!(151, 107, 30, 255)),
        Some(Role::Shade) => (color_u8!(240, 231, 253, 255), color_u8!(118, 81, 159, 255)),
        Some(Role::Corsair) => (color_u8!(225, 243, 246, 255), color_u8!(47, 114, 129, 255)),
        Some(Role::Envoy) => (color_u8!(233, 244, 217, 255), color_u8!(94, 126, 55, 255)),
        _ if action == Action::Coup => (color_u8!(253, 231, 222, 255), color_u8!(175, 82, 62, 255)),
        _ => (ui.theme.panel, ui.theme.accent),
    };
    bordered(
        rect,
        16.,
        if ui.theme.saver { ink } else { wash },
        if ui.theme.saver { BLACK } else { wash },
    );
    let center = vec2(rect.x + 23., rect.center().y);
    match action {
        Action::Income | Action::Aid => coin(center, 9.),
        Action::Tax => crown(center, 9., ink),
        Action::Exchange => {
            for sign in [-1., 1.] {
                let y = center.y + sign * 5.;
                draw_line(center.x - 8., y, center.x + 8., y, 2., ink);
                draw_triangle(
                    vec2(center.x + sign * 9., y),
                    vec2(center.x + sign * 3., y - 4.),
                    vec2(center.x + sign * 3., y + 4.),
                    ink,
                );
            }
        }
        Action::Steal => {
            coin(center + vec2(-3., 3.), 7.);
            draw_line(
                center.x - 5.,
                center.y - 9.,
                center.x + 9.,
                center.y - 9.,
                2.,
                ink,
            );
            draw_triangle(
                center + vec2(10., -9.),
                center + vec2(4., -13.),
                center + vec2(4., -5.),
                ink,
            );
        }
        Action::Assassinate | Action::Coup => {
            draw_line(
                center.x - 7.,
                center.y + 9.,
                center.x + 7.,
                center.y - 8.,
                3.,
                ink,
            );
            draw_line(
                center.x - 9.,
                center.y + 1.,
                center.x,
                center.y + 8.,
                2.,
                ink,
            );
            spark(center + vec2(8., -9.), 4., ink);
        }
    }
    let name = action.title();
    let w = (rect.w - 53.).max(20.);
    let size = (14. * w / ui.text_width(name, 14., true).max(1.)).clamp(9., 14.);
    ui.heading(
        name,
        rect.x + 45.,
        rect.y + 23.,
        size,
        if ui.theme.saver { ui.theme.text } else { ink },
    );
    let detail = match action {
        Action::Income => "+1 coin",
        Action::Aid => "+2 coins",
        Action::Tax => "Regent · +3",
        Action::Exchange => "Envoy · swap",
        Action::Steal => "Corsair · take 2",
        Action::Assassinate => "Shade · pay 3",
        Action::Coup => "Pay 7 · no block",
    };
    let size = (11. * w / ui.text_width(detail, 11., false).max(1.)).clamp(8., 11.);
    ui.label(detail, rect.x + 45., rect.y + 39., size, ui.theme.muted);
    let hit = rect.overlaps(&viewport)
        && (tap.is_some_and(|p| rect.contains(p)) || ui.keyboard_hit(rect));
    ui.activated |= hit;
    hit
}
