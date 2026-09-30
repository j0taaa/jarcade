use crate::ui::{CORAL, GREEN, MINT, Theme, bordered};
use jarcade::curve::rounded_path;
use jarcade::snake::{BOARD_SIZE, Cell, Direction};
use macroquad::prelude::*;

pub fn cell_center(cell: Cell, board: Rect) -> Vec2 {
    let unit = board.w / f32::from(BOARD_SIZE);
    vec2(
        board.x + (f32::from(cell.x) + 0.5) * unit,
        board.y + (f32::from(cell.y) + 0.5) * unit,
    )
}

/// The same renderer supplies the live board and the launch-card image.
/// Body positions may be interpolated; rules still move on fixed ticks.
pub fn draw_board(
    board: Rect,
    body: &[Vec2],
    food: Option<Cell>,
    direction: Direction,
    heading: Option<Vec2>,
    theme: &Theme,
) {
    let unit = board.w / f32::from(BOARD_SIZE);
    bordered(
        Rect::new(board.x - 8.0, board.y - 8.0, board.w + 16.0, board.h + 16.0),
        23.0,
        if theme.saver {
            theme.line
        } else {
            color_u8!(219, 233, 211, 255)
        },
        if theme.saver { BLACK } else { MINT },
    );
    if !theme.saver {
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                if (x + y) % 2 == 0 {
                    draw_rectangle(
                        board.x + f32::from(x) * unit,
                        board.y + f32::from(y) * unit,
                        unit,
                        unit,
                        color_u8!(226, 238, 218, 255),
                    );
                }
            }
        }
    }
    let body_color = if theme.saver {
        color_u8!(156, 215, 124, 255)
    } else {
        GREEN
    };
    let path = rounded_path(body, unit);
    let radius = unit * 0.385;
    for pair in path.windows(2) {
        let a = pair[0];
        let b = pair[1];
        let na = vec2(-a.tangent.y, a.tangent.x) * radius;
        let nb = vec2(-b.tangent.y, b.tangent.x) * radius;
        draw_triangle(
            a.position + na,
            a.position - na,
            b.position + nb,
            body_color,
        );
        draw_triangle(
            a.position - na,
            b.position - nb,
            b.position + nb,
            body_color,
        );
    }
    for &pos in body.first().into_iter().chain(body.last()) {
        draw_circle(pos.x, pos.y, radius, body_color);
    }
    if let Some(&head) = body.first() {
        let (dx, dy) = direction.delta();
        let forward = heading.unwrap_or_else(|| {
            path.first()
                .map_or(vec2(f32::from(dx), f32::from(dy)), |p| -p.tangent)
        });
        let side = vec2(-forward.y, forward.x);
        for sign in [-1.0, 1.0] {
            let eye = head + forward * unit * 0.12 + side * unit * 0.20 * sign;
            draw_circle(eye.x, eye.y, unit * 0.12, WHITE);
            let pupil = eye + forward * unit * 0.035;
            draw_circle(pupil.x, pupil.y, unit * 0.055, color_u8!(22, 42, 29, 255));
        }
    }
    if let Some(cell) = food {
        let pos = cell_center(cell, board);
        draw_circle(pos.x, pos.y + unit * 0.035, unit * 0.34, CORAL);
        draw_ellipse(
            pos.x + unit * 0.08,
            pos.y - unit * 0.30,
            unit * 0.15,
            unit * 0.07,
            -35.0,
            body_color,
        );
        draw_circle(
            pos.x - unit * 0.09,
            pos.y - unit * 0.08,
            unit * 0.065,
            color_u8!(255, 204, 174, 255),
        );
    }
}

pub struct Preview {
    target: RenderTarget,
}
impl Preview {
    pub fn new(saver: bool) -> Self {
        // Fixed high-resolution screenshot texture, rendered once and reused.
        // 1024 pixels exceeds the card's normal display size even at DPR 3.
        let target = render_target(1024, 1024);
        target.texture.set_filter(FilterMode::Linear);
        set_camera(&Camera2D {
            render_target: Some(target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0.0, 0.0, 1024.0, 1024.0))
        });
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        let board = Rect::new(32.0, 32.0, 960.0, 960.0);
        let cells = [
            Cell { x: 14, y: 5 },
            Cell { x: 13, y: 5 },
            Cell { x: 12, y: 5 },
            Cell { x: 11, y: 5 },
            Cell { x: 10, y: 5 },
            Cell { x: 10, y: 6 },
            Cell { x: 10, y: 7 },
            Cell { x: 10, y: 8 },
            Cell { x: 10, y: 9 },
            Cell { x: 9, y: 9 },
            Cell { x: 8, y: 9 },
            Cell { x: 7, y: 9 },
            Cell { x: 6, y: 9 },
            Cell { x: 5, y: 9 },
            Cell { x: 5, y: 10 },
            Cell { x: 5, y: 11 },
            Cell { x: 5, y: 12 },
            Cell { x: 5, y: 13 },
            Cell { x: 6, y: 13 },
            Cell { x: 7, y: 13 },
            Cell { x: 8, y: 13 },
        ];
        let body: Vec<_> = cells.iter().map(|&cell| cell_center(cell, board)).collect();
        draw_board(
            board,
            &body,
            Some(Cell { x: 14, y: 12 }),
            Direction::Right,
            None,
            &Theme::new(saver),
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
