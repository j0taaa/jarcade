use macroquad::prelude::{Rect, Vec2, vec2};

#[derive(Clone, Copy)]
struct Drag {
    start: Vec2,
    offset: Vec2,
    moved: bool,
}
pub struct BoardPan {
    pub offset: Vec2,
    pub zoom: f32,
    drag: Option<Drag>,
}
impl Default for BoardPan {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
            drag: None,
        }
    }
}
impl BoardPan {
    /// Keep the board point under the gesture anchored while scaling and moving.
    pub fn transform(&mut self, from: Vec2, to: Vec2, zoom: f32, viewport: Rect, base: Vec2) {
        if !zoom.is_finite() {
            return;
        }
        let old = self.board(viewport, base * self.zoom);
        let point = (from - old.point()) / self.zoom;
        self.zoom = zoom.clamp(0.2, 2.5);
        let content = base * self.zoom;
        let centered = viewport.point() + (viewport.size() - content).max(Vec2::ZERO) * 0.5;
        self.offset = centered - to + point * self.zoom;
        self.cancel();
        self.clamp(viewport, content);
    }
    pub fn zoom_at(&mut self, anchor: Vec2, zoom: f32, viewport: Rect, base: Vec2) {
        self.transform(anchor, anchor, zoom, viewport, base);
    }
    pub fn fit(&mut self, viewport: Rect, base: Vec2) {
        self.zoom_at(
            viewport.center(),
            (viewport.w / base.x).min(viewport.h / base.y),
            viewport,
            base,
        );
        self.offset = Vec2::ZERO;
    }

    pub fn active(&self) -> bool {
        self.drag.is_some()
    }
    pub fn cancel(&mut self) {
        self.drag = None;
    }
    pub fn clamp(&mut self, viewport: Rect, content: Vec2) {
        self.offset = self
            .offset
            .clamp(Vec2::ZERO, (content - viewport.size()).max(Vec2::ZERO));
    }
    pub fn board(&self, viewport: Rect, content: Vec2) -> Rect {
        let origin =
            viewport.point() + (viewport.size() - content).max(Vec2::ZERO) * 0.5 - self.offset;
        Rect::new(origin.x, origin.y, content.x, content.y)
    }
    pub fn begin(&mut self, point: Vec2) {
        self.drag = Some(Drag {
            start: point,
            offset: self.offset,
            moved: false,
        });
    }
    pub fn update(&mut self, point: Vec2, viewport: Rect, content: Vec2) {
        if let Some(drag) = self.drag.as_mut() {
            let delta = point - drag.start;
            drag.moved |= delta.length_squared() > 64.0;
            if drag.moved {
                self.offset = drag.offset - delta;
            }
        }
        self.clamp(viewport, content);
    }
    pub fn end(&mut self, point: Vec2, viewport: Rect, content: Vec2) -> Option<Vec2> {
        self.update(point, viewport, content);
        self.drag
            .take()
            .filter(|d| !d.moved && viewport.contains(point))
            .map(|_| point)
    }
    pub fn scroll(&mut self, delta: Vec2, viewport: Rect, content: Vec2) {
        self.cancel();
        self.offset += delta;
        self.clamp(viewport, content);
    }
    pub fn cell_at(
        &self,
        point: Vec2,
        viewport: Rect,
        columns: usize,
        rows: usize,
        unit: f32,
    ) -> Option<usize> {
        let board = self.board(viewport, vec2(columns as f32, rows as f32) * unit);
        let p = point - board.point();
        if !viewport.contains(point) || p.x < 0. || p.y < 0. || p.x >= board.w || p.y >= board.h {
            return None;
        }
        Some((p.y / unit) as usize * columns + (p.x / unit) as usize)
    }
    pub fn show_cell(
        &mut self,
        index: usize,
        columns: usize,
        unit: f32,
        viewport: Rect,
        content: Vec2,
    ) {
        let p = vec2((index % columns) as f32, (index / columns) as f32) * unit;
        self.offset = self
            .offset
            .min(p)
            .max(p + Vec2::splat(unit) - viewport.size());
        self.clamp(viewport, content);
    }
}
/// Once two fingers participate, suppress taps until every finger is lifted.
#[derive(Default)]
pub struct PinchZoom {
    previous: Option<([u64; 2], Vec2, f32)>,
    blocked: bool,
}
impl PinchZoom {
    pub fn update(
        &mut self,
        touches: &[(u64, Vec2)],
        pan: &mut BoardPan,
        viewport: Rect,
        base: Vec2,
    ) -> bool {
        if touches.len() >= 2 {
            self.blocked = true;
            pan.cancel();
            let mut pair = [touches[0], touches[1]];
            pair.sort_by_key(|p| p.0);
            let ids = [pair[0].0, pair[1].0];
            let center = (pair[0].1 + pair[1].1) * 0.5;
            let distance = pair[0].1.distance(pair[1].1).max(8.0);
            if let Some((old_ids, old_center, old_distance)) = self.previous {
                if old_ids == ids {
                    pan.transform(
                        old_center,
                        center,
                        pan.zoom * distance / old_distance,
                        viewport,
                        base,
                    );
                }
            } else if !pair.iter().all(|p| viewport.contains(p.1)) {
                return true;
            }
            self.previous = Some((ids, center, distance));
            return true;
        }
        let suppress = self.blocked;
        self.previous = None;
        if touches.is_empty() {
            self.blocked = false;
        }
        suppress
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const VIEW: Rect = Rect::new(20., 140., 350., 400.);
    fn content() -> Vec2 {
        vec2(30., 16.) * 48.
    }
    #[test]
    fn taps_reveal_on_release_but_drags_never_become_taps() {
        let mut pan = BoardPan::default();
        pan.begin(vec2(100., 200.));
        assert_eq!(
            pan.end(vec2(103., 202.), VIEW, content()),
            Some(vec2(103., 202.))
        );
        pan.begin(vec2(250., 350.));
        pan.update(vec2(150., 250.), VIEW, content());
        assert_eq!(pan.offset, vec2(100., 100.));
        // Returning to the starting point still counts as a drag.
        assert_eq!(pan.end(vec2(250., 350.), VIEW, content()), None);
        pan.begin(vec2(100., 200.));
        pan.cancel();
        assert_eq!(pan.end(vec2(100., 200.), VIEW, content()), None);
    }
    #[test]
    fn scrolling_reaches_last_cell_clamps_edges_and_maps_retina_logical_points() {
        let mut pan = BoardPan::default();
        pan.scroll(vec2(10000., 10000.), VIEW, content());
        assert_eq!(pan.offset, content() - VIEW.size());
        assert_eq!(
            pan.cell_at(
                vec2(VIEW.right() - 24., VIEW.bottom() - 24.),
                VIEW,
                30,
                16,
                48.
            ),
            Some(479)
        );
        pan.scroll(vec2(-10000., -10000.), VIEW, content());
        assert_eq!(pan.offset, Vec2::ZERO);
        for dpi in [1., 2., 3., 4.] {
            let p = crate::layout::touch_point(vec2(44., 164.) * dpi, dpi);
            assert_eq!(pan.cell_at(p, VIEW, 30, 16, 48.), Some(0));
        }
        assert_eq!(pan.cell_at(vec2(44., 600.), VIEW, 30, 16, 48.), None);
    }
    #[test]
    fn keyboard_selection_scrolls_into_view_and_resize_keeps_bounds_valid() {
        let mut pan = BoardPan::default();
        pan.show_cell(479, 30, 48., VIEW, content());
        let b = pan.board(VIEW, content());
        assert!(VIEW.contains(b.point() + content() - Vec2::splat(24.)));
        pan.show_cell(0, 30, 48., VIEW, content());
        assert_eq!(pan.offset, Vec2::ZERO);
        pan.scroll(content(), VIEW, content());
        let large = Rect::new(0., 0., 2000., 1000.);
        pan.clamp(large, content());
        assert_eq!(pan.offset, Vec2::ZERO);
        assert_eq!(pan.board(large, content()).center(), large.center());
    }
    #[test]
    fn zoom_keeps_cell_under_anchor_and_hit_testing_matches() {
        let mut pan = BoardPan::default();
        pan.scroll(vec2(200., 100.), VIEW, content());
        let anchor = VIEW.center();
        let cell = pan.cell_at(anchor, VIEW, 30, 16, 48.);
        let logical = (anchor - pan.board(VIEW, content()).point()) / 48.;
        pan.zoom_at(anchor, 1.8, VIEW, content());
        let after = (anchor - pan.board(VIEW, content() * pan.zoom).point()) / (48. * pan.zoom);
        assert!(logical.distance(after) < 0.0001);
        assert_eq!(pan.cell_at(anchor, VIEW, 30, 16, 48. * pan.zoom), cell);
        pan.zoom_at(anchor, 1., VIEW, content());
        assert!(pan.offset.distance(vec2(200., 100.)) < 0.0001);
    }
    #[test]
    fn fit_and_zoom_limits_work_for_all_board_sizes_and_viewports() {
        for size in crate::minesweeper::BoardSize::ALL {
            let (w, h, _) = size.dimensions();
            let base = vec2(w as f32, h as f32) * 48.;
            for view in [
                VIEW,
                Rect::new(8., 96., 304., 298.),
                Rect::new(8., 58., 552., 176.),
            ] {
                let mut pan = BoardPan::default();
                pan.fit(view, base);
                let board = pan.board(view, base * pan.zoom);
                assert!(board.w <= view.w + 0.001 && board.h <= view.h + 0.001);
                assert_eq!(pan.offset, Vec2::ZERO);
                pan.zoom_at(view.center(), 100., view, base);
                assert_eq!(pan.zoom, 2.5);
                pan.zoom_at(view.center(), 0., view, base);
                assert_eq!(pan.zoom, 0.2);
                pan.zoom_at(view.center(), f32::NAN, view, base);
                assert_eq!(pan.zoom, 0.2);
            }
        }
    }
    #[test]
    fn pinch_scales_moves_and_never_turns_into_a_tap_after_one_finger_lifts() {
        let mut pan = BoardPan::default();
        let mut pinch = PinchZoom::default();
        pan.begin(vec2(120., 240.));
        assert!(pinch.update(
            &[(1, vec2(120., 240.)), (2, vec2(220., 240.))],
            &mut pan,
            VIEW,
            content()
        ));
        assert!(!pan.active());
        // Reverse event order: stable touch IDs keep the same gesture.
        pinch.update(
            &[(2, vec2(270., 250.)), (1, vec2(70., 250.))],
            &mut pan,
            VIEW,
            content(),
        );
        assert_eq!(pan.zoom, 2.);
        assert!(pinch.update(&[(1, vec2(70., 250.))], &mut pan, VIEW, content()));
        assert!(pinch.update(&[], &mut pan, VIEW, content()));
        assert_eq!(pan.end(vec2(70., 250.), VIEW, content() * pan.zoom), None);
        assert!(!pinch.update(&[(3, vec2(150., 250.))], &mut pan, VIEW, content()));
    }
}
