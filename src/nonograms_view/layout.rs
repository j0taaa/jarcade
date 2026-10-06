use macroquad::prelude::*;

pub(super) fn landscape(w: f32, h: f32) -> bool {
    w >= 540. && h < 500.
}

pub(super) struct SetupLayout {
    pub modes: Rect,
    pub sizes: Rect,
    pub content: Rect,
    pub play: Rect,
}
impl SetupLayout {
    pub fn new(w: f32, h: f32) -> Self {
        if landscape(w, h) {
            Self {
                modes: Rect::new(16., 66., 180., 44.),
                sizes: Rect::new(16., 122., 180., 48.),
                content: Rect::new(220., 66., w - 236., h - 82.),
                play: Rect::new(16., h - 64., 180., 48.),
            }
        } else {
            let width = (w - 32.).min(840.);
            let x = (w - width) * 0.5;
            Self {
                modes: Rect::new(x, 62., width, 44.),
                sizes: Rect::new(x, 120., width, 48.),
                content: Rect::new(x, 182., width, h - 262.),
                play: Rect::new((w - width.min(440.)) * 0.5, h - 64., width.min(440.), 48.),
            }
        }
    }
}
pub(super) fn board_viewport(w: f32, h: f32) -> Rect {
    if landscape(w, h) {
        Rect::new(174., 62., w - 190., h - 78.)
    } else {
        let width = (w - 24.).min(780.);
        Rect::new((w - width) * 0.5, 112., width, h - 192.)
    }
}

pub(super) fn action_rects(w: f32, h: f32) -> [Rect; 6] {
    if landscape(w, h) {
        std::array::from_fn(|i| {
            Rect::new(
                16. + (i % 3) as f32 * 48.,
                if i < 3 { 64. } else { 120. },
                44.,
                44.,
            )
        })
    } else {
        let width = (w - 16.).min(600.);
        let x = (w - width) * 0.5;
        std::array::from_fn(|i| {
            Rect::new(
                if i < 3 {
                    x + i as f32 * 44.
                } else {
                    x + width - 132. + (i - 3) as f32 * 44.
                },
                58.,
                44.,
                44.,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selectors_and_board_leave_room_for_fixed_controls_at_every_size() {
        for (w, h) in [
            (280., 360.),
            (320., 480.),
            (390., 844.),
            (568., 320.),
            (768., 1024.),
            (1440., 900.),
        ] {
            let l = SetupLayout::new(w, h);
            for r in [l.modes, l.sizes, l.content, l.play, board_viewport(w, h)] {
                assert!(r.x >= 0. && r.y >= 0. && r.right() <= w && r.bottom() <= h);
                assert!(r.w > 0. && r.h > 0.);
            }
            for r in action_rects(w, h) {
                assert!(
                    r.x >= 0.
                        && r.y >= 0.
                        && r.right() <= w
                        && r.bottom() <= h
                        && r.w >= 44.
                        && r.h >= 44.
                );
                assert!(!r.overlaps(&board_viewport(w, h)));
            }
            assert!(!l.content.overlaps(&l.play));
            assert!(!l.sizes.overlaps(&l.content));
            assert!((l.sizes.w - 12.) / 3. >= 44. && l.modes.h >= 44. && l.play.h >= 44.);
            assert!(board_viewport(w, h).bottom() <= h - if landscape(w, h) { 16. } else { 80. });
        }
    }
}
