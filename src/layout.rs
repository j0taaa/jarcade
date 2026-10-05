use macroquad::prelude::{Rect, Vec2, vec2};

/// All geometry is in logical points. The backing buffer stays at native DPR.
pub fn touch_point(position: Vec2, dpi: f32) -> Vec2 {
    position / dpi.max(1.0)
}

pub struct Layout {
    pub content: Rect,
    pub board: Rect,
    pub controls: Vec2,
    pub landscape: bool,
}

pub struct GameGrid {
    pub columns: usize,
    pub image_height: f32,
    origin: Vec2,
    width: f32,
    gap: f32,
}

impl GameGrid {
    pub fn content_height(&self, count: usize) -> f32 {
        let rows = count.max(1).div_ceil(self.columns);
        rows as f32 * (self.image_height + 60.) + (rows - 1) as f32 * self.gap
    }
    pub fn card(&self, index: usize) -> Rect {
        let height = self.image_height + 60.0;
        Rect::new(
            self.origin.x + (index % self.columns) as f32 * (self.width + self.gap),
            self.origin.y + (index / self.columns) as f32 * (height + self.gap),
            self.width,
            height,
        )
    }
}

impl Layout {
    pub fn game_grid(&self, top: f32) -> GameGrid {
        self.game_grid_for(top, 3)
    }
    pub fn game_grid_for(&self, top: f32, count: usize) -> GameGrid {
        let columns = if self.content.w >= 960.0
            || (count == 4 && self.landscape && self.content.w >= 480.)
        {
            4
        } else if self.content.w >= 600.0 || (self.landscape && self.content.w >= 480.) {
            3
        } else {
            2
        };
        let gap = if columns == 2 { 12.0 } else { 20.0 };
        let width = (self.content.w - gap * (columns - 1) as f32) / columns as f32;
        GameGrid {
            columns,
            width,
            gap,
            origin: vec2(self.content.x, top),
            image_height: (width * 0.88).min(
                ((self.content.h - top - 24. - gap * (count.max(1).div_ceil(columns) - 1) as f32)
                    / count.max(1).div_ceil(columns) as f32
                    - 60.)
                    .max(if count >= 5 { 88. } else { 44. }),
            ),
        }
    }

    pub fn new(width: f32, height: f32) -> Self {
        let margin = if width < 360. {
            16.
        } else if width < 600. {
            20.
        } else {
            40.
        };
        let content_width = (width - margin * 2.0).min(1040.0);
        let landscape =
            (width > height * 1.3 && height < 620.0) || (height < 360. && width >= 360.);
        let board_y = if landscape {
            if height < 300. { 80. } else { 96. }
        } else if height < 540. {
            104.
        } else {
            164.
        };
        let size = if landscape {
            (height - board_y - 20.0).min(width - 260.0)
        } else {
            (width - margin * 2.0).min(height - board_y - 140.0)
        }
        .clamp(96.0, 500.0);
        let board_x = (width - size - if landscape { 200.0 } else { 0.0 }) / 2.0;
        let board = Rect::new(board_x, board_y, size, size);
        Self {
            content: Rect::new((width - content_width) / 2.0, 0.0, content_width, height),
            board,
            landscape,
            controls: if landscape {
                vec2(board_x + size + 110.0, board_y + size / 2.0)
            } else {
                vec2(width / 2.0, board_y + size + 70.0)
            },
        }
    }
}

/// Rotate the table on short landscape screens so the play area stays large.
pub struct TableLayout {
    pub table: Rect,
    pub score: Rect,
    pub landscape: bool,
}
impl TableLayout {
    pub fn new(width: f32, height: f32) -> Self {
        let landscape = width > height * 1.3 && height < 620.;
        let (table, score) = if landscape {
            let w = (width - 174.).min((height - 82.) / 0.62).min(900.);
            (
                Rect::new(
                    162. + (width - 174. - w) / 2.,
                    68. + (height - 82. - w * 0.62) / 2.,
                    w,
                    w * 0.62,
                ),
                Rect::new(12., 85., 138., 78.),
            )
        } else {
            let w = (width - 32.).min((height - 154.) * 0.62).min(520.);
            (
                Rect::new(
                    (width - w) / 2.,
                    114. + (height - 154. - w / 0.62) / 2.,
                    w,
                    w / 0.62,
                ),
                Rect::new((width - 240.) / 2., 64., 240., 40.),
            )
        };
        Self {
            table,
            score,
            landscape,
        }
    }
    pub fn point(&self, point: Vec2) -> Vec2 {
        if self.landscape {
            self.table.point() + vec2(point.y * self.table.w, (1. - point.x) * self.table.h)
        } else {
            self.table.point() + vec2(point.x * self.table.w, point.y * self.table.h)
        }
    }
    pub fn local(&self, point: Vec2, finger: bool) -> Vec2 {
        let p = point - self.table.point();
        if self.landscape {
            vec2(
                1. - p.y / self.table.h,
                (p.x - if finger { 26. } else { 0. }) / self.table.w,
            )
        } else {
            vec2(
                p.x / self.table.w,
                (p.y - if finger { 26. } else { 0. }) / self.table.h,
            )
        }
    }
    pub fn unit(&self) -> f32 {
        if self.landscape {
            self.table.h
        } else {
            self.table.w
        }
    }
    pub fn racket_point(&self, point: Vec2) -> Vec2 {
        self.point(point) - vec2(0., crate::table_tennis::RACKET_HEIGHT * self.unit())
    }
    pub fn racket_local(&self, point: Vec2, finger: bool) -> Vec2 {
        self.local(
            point + vec2(0., crate::table_tennis::RACKET_HEIGHT * self.unit()),
            finger,
        )
    }
}

#[cfg(test)]
mod table_tests {
    use super::*;
    #[test]
    fn table_and_score_fit_and_pointer_mapping_survives_rotation() {
        for (w, h) in [
            (280., 360.),
            (320., 480.),
            (390., 844.),
            (568., 320.),
            (844., 390.),
            (768., 1024.),
            (1440., 900.),
        ] {
            let l = TableLayout::new(w, h);
            for r in [l.table, l.score] {
                assert!(r.x >= 0. && r.y >= 64. && r.right() <= w && r.bottom() <= h - 12.);
            }
            assert!(!l.table.overlaps(&l.score));
            let point = vec2(0.7, 0.84);
            assert!(l.local(l.point(point), false).distance(point) < 0.00001);
            let offset = if l.landscape {
                vec2(26., 0.)
            } else {
                vec2(0., 26.)
            };
            assert!(l.local(l.point(point) + offset, true).distance(point) < 0.00001);
            assert!(l.racket_local(l.racket_point(point), false).distance(point) < 0.00001);
            assert!(
                l.racket_local(l.racket_point(point) + offset, true)
                    .distance(point)
                    < 0.00001
            );
        }
    }
    #[test]
    fn four_solo_cards_fit_short_landscape_and_portrait_screens() {
        for (w, h, top) in [
            (280., 360., 132.),
            (320., 480., 132.),
            (390., 844., 238.),
            (568., 320., 132.),
            (844., 390., 132.),
            (768., 1024., 238.),
            (1440., 900., 238.),
        ] {
            let l = Layout::new(w, h);
            let grid = l.game_grid_for(top, 4);
            for index in 0..4 {
                let card = grid.card(index);
                assert!(card.bottom() <= h && card.right() <= w);
                assert!(card.w >= 112. && card.h >= 100.);
            }
        }
    }
    #[test]
    fn five_cards_keep_readable_previews_and_can_all_be_reached_by_scrolling() {
        use crate::board_pan::BoardPan;
        for (w, h, top) in [
            (280., 360., 132.),
            (320., 480., 132.),
            (390., 844., 238.),
            (568., 320., 132.),
            (768., 1024., 238.),
            (1440., 900., 238.),
        ] {
            let l = Layout::new(w, h);
            let grid = l.game_grid_for(top, 5);
            if w < 600. && w < h {
                assert_eq!(grid.columns, 2);
            }
            let viewport = Rect::new(l.content.x, top, l.content.w, h - top - 16.);
            let content = vec2(viewport.w, grid.content_height(5).max(viewport.h));
            let mut pan = BoardPan::default();
            for index in 0..5 {
                let card = grid.card(index);
                assert!(card.w >= 112. && grid.image_height >= 88.);
                pan.offset.y = (card.bottom() - top - viewport.h).max(0.);
                pan.clamp(viewport, content);
                let origin = pan.board(viewport, content).point();
                let shown = Rect::new(card.x, card.y + origin.y - top, card.w, card.h);
                assert!(shown.y >= viewport.y && shown.bottom() <= viewport.bottom() + 0.001);
                assert!(shown.x >= viewport.x && shown.right() <= viewport.right() + 0.001);
                pan.begin(shown.center());
                pan.update(shown.center() - vec2(0., 40.), viewport, content);
                assert_eq!(pan.end(shown.center(), viewport, content), None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_grid_fits_two_phone_cards_and_more_on_larger_screens() {
        for (width, height, columns) in [
            (280., 360., 2),
            (320., 480., 2),
            (390., 844., 2),
            (430., 932., 2),
            (568., 320., 3),
            (768., 1024., 3),
            (1440., 900., 4),
        ] {
            let layout = Layout::new(width, height);
            let top = if height < 500. {
                132.
            } else if height < 640. {
                208.
            } else {
                238.
            };
            let grid = layout.game_grid(top);
            assert_eq!(grid.columns, columns);
            for index in 0..3 {
                let card = grid.card(index);
                assert!(
                    card.x >= layout.content.x && card.right() <= layout.content.right() + 0.001
                );
                assert!(card.bottom() <= height);
                assert!(card.w >= 112. && card.h >= 100.);
                if index % columns > 0 {
                    assert!(grid.card(index - 1).right() < card.x);
                }
            }
            assert_eq!(grid.card(columns).x, grid.card(0).x);
            assert!(grid.card(columns).y > grid.card(0).bottom());
        }
    }

    #[test]
    fn retina_touches_match_logical_targets() {
        for dpi in [1.0, 2.0, 3.0, 4.0] {
            let logical = vec2(250.0, 320.0);
            assert_eq!(touch_point(logical * dpi, dpi), logical);
        }
    }

    #[test]
    fn board_and_touch_controls_fit_supported_phone_and_desktop_sizes() {
        for (width, height) in [
            (280., 600.),
            (320., 360.),
            (360., 280.),
            (320.0, 480.0),
            (320.0, 568.0),
            (390.0, 664.0),
            (390.0, 844.0),
            (430.0, 932.0),
            (568.0, 320.0),
            (844.0, 390.0),
            (768.0, 1024.0),
            (1440.0, 900.0),
        ] {
            let layout = Layout::new(width, height);
            assert!(layout.board.x >= 0.0 && layout.board.right() <= width);
            assert!(layout.board.bottom() <= height);
            assert!(layout.controls.x - 90.0 >= 0.0 && layout.controls.x + 90.0 <= width);
            assert!(layout.controls.y + 58.0 <= height, "{width} x {height}");
        }
    }
}

pub struct MinesLayout {
    pub zoom_controls: Rect,
    pub viewport: Rect,
    pub tools: Rect,
    pub stats: Rect,
    pub tile_size: f32,
}
impl MinesLayout {
    pub fn new(width: f32, height: f32) -> Self {
        let landscape = width > height * 1.3 && height < 620.0;
        let content = (width - 16.0).min(1440.0);
        let x = (width - content) / 2.0;
        let (viewport, tools, stats, zoom_controls) = if landscape {
            (
                Rect::new(x, 58.0, content, height - 144.0),
                Rect::new(x, height - 76.0, 200.0, 52.0),
                Rect::new(x + 212.0, height - 72.0, content - 374.0, 40.0),
                Rect::new(x + content - 150.0, height - 72.0, 150.0, 44.0),
            )
        } else {
            (
                Rect::new(x, 100.0, content, height - 186.0),
                Rect::new(
                    (width - content.min(440.0)) / 2.0,
                    height - 76.0,
                    content.min(440.0),
                    52.0,
                ),
                Rect::new(x + 4.0, 52.0, content - 170.0, 36.0),
                Rect::new(x + content - 150.0, 48.0, 150.0, 44.0),
            )
        };
        Self {
            viewport,
            tools,
            stats,
            zoom_controls,
            tile_size: if width < 700.0 { 48.0 } else { 52.0 },
        }
    }
}

#[cfg(test)]
mod mines_tests {
    use super::*;
    #[test]
    fn viewport_and_fixed_tools_fit_without_shrinking_tiles() {
        for (w, h) in [
            (320., 480.),
            (390., 844.),
            (430., 932.),
            (568., 320.),
            (844., 390.),
            (768., 1024.),
            (1440., 900.),
        ] {
            let l = MinesLayout::new(w, h);
            for rect in [l.viewport, l.tools, l.stats, l.zoom_controls] {
                assert!(rect.x >= 0. && rect.right() <= w && rect.y >= 48. && rect.bottom() <= h);
            }
            assert!(l.viewport.w >= 200. && l.viewport.h >= 176.);
            assert!(l.tile_size >= 48.);
            assert!(!l.viewport.overlaps(&l.tools));
            assert!(!l.viewport.overlaps(&l.zoom_controls));
            assert!(!l.stats.overlaps(&l.zoom_controls));
            assert!(l.viewport.w * l.viewport.h / (w * h) > 0.52);
        }
    }
}

pub struct FihLayout {
    pub viewport: Rect,
    pub stats: Rect,
    pub room_nav: Rect,
    pub pet: Rect,
    pub tools: Rect,
}
impl FihLayout {
    pub fn new(width: f32, height: f32) -> Self {
        let landscape = height < 520. && width > height * 1.3;
        let stats_width = (width - 90.).clamp(200., 310.).min(width - 64.);
        let stats = if width < 360. {
            Rect::new(64., 6., width - 76., 44.)
        } else {
            Rect::new((width - stats_width) / 2., 6., stats_width, 44.)
        };
        let room_nav = Rect::new((width - 240.) / 2., 64., 240., 44.);
        let tools = if landscape {
            Rect::new(width - 152., 112., 140., height - 124.)
        } else {
            Rect::new(
                (width - (width - 24.).min(570.)) / 2.,
                height - 108.,
                (width - 24.).min(570.),
                92.,
            )
        };
        let top = room_nav.bottom() + 8.;
        let pet = if landscape {
            Rect::new(30., top, width - 194., height - top - 16.)
        } else {
            Rect::new(
                (width - (width - 20.).min(600.)) / 2.,
                top,
                (width - 20.).min(600.),
                (tools.y - top - 8.).max(40.),
            )
        };
        Self {
            viewport: Rect::new(0., 0., width, height),
            stats,
            room_nav,
            pet,
            tools,
        }
    }
}
#[cfg(test)]
mod fih_layout_tests {
    use super::*;
    #[test]
    fn fullscreen_rooms_keep_overlay_controls_inside_the_viewport() {
        for (w, h) in [
            (280., 360.),
            (568., 320.),
            (320., 480.),
            (390., 844.),
            (430., 932.),
            (768., 1024.),
            (1440., 900.),
        ] {
            let l = FihLayout::new(w, h);
            assert_eq!(l.viewport, Rect::new(0., 0., w, h));
            for r in [l.stats, l.room_nav, l.tools, l.pet] {
                assert!(r.x >= 0. && r.right() <= w && r.y >= 0. && r.bottom() <= h);
            }
            assert!(l.tools.w >= 44. && l.tools.h >= 44.);
            assert!(!l.pet.overlaps(&l.tools));
            assert!(l.room_nav.h >= 44.);
        }
    }
}
