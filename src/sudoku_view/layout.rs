use macroquad::prelude::*;
pub struct Layout {
    pub board: Rect,
    pub pad: Rect,
    pub actions: Rect,
    pub message: Rect,
}
impl Layout {
    pub fn new(w: f32, h: f32) -> Self {
        let side = w >= 850. || w >= 480. && h < 550.;
        if side {
            let pad_w = if h < 450. { 194. } else { 256. };
            let size = (h - 112.).min(w - pad_w - 64.).clamp(96., 630.);
            let x = (w - size - pad_w - 28.) * 0.5;
            let top = if h < 450. { 64. } else { 100. };
            let board = Rect::new(x, top, size, size);
            let pad = Rect::new(board.right() + 28., top, pad_w, 0.);
            let row = (pad_w - 18.) / 4.;
            let pad = Rect {
                h: row * 4. + 18.,
                ..pad
            };
            let actions = if h < 450. {
                Rect::new((w - 340.) / 2., h - 48., 340., 44.)
            } else {
                Rect::new(pad.x - 4., pad.bottom() + 12., pad.w + 8., 44.)
            };
            Self {
                board,
                pad,
                actions,
                message: Rect::new(
                    pad.x,
                    if h < 450. { 0. } else { actions.bottom() + 12. },
                    pad.w,
                    if h < 450. {
                        0.
                    } else {
                        (h - actions.bottom() - 24.).max(24.)
                    },
                ),
            }
        } else {
            let pad_w = (w - 32.).min(if h < 700. { 194. } else { 288. });
            let row = (pad_w - 18.) / 4.;
            let pad_h = row * 4. + 18.;
            let size = (w - 24.).min(h - 72. - pad_h - 98.).clamp(96., 600.);
            let board = Rect::new((w - size) * 0.5, 72., size, size);
            let pad = Rect::new((w - pad_w) * 0.5, board.bottom() + 16., pad_w, pad_h);
            let actions = Rect::new(
                (w - (w - 24.).min(340.)) * 0.5,
                pad.bottom() + 10.,
                (w - 24.).min(340.),
                44.,
            );
            Self {
                board,
                pad,
                actions,
                message: Rect::new(16., actions.bottom() + 8., w - 32., 36.),
            }
        }
    }
    pub fn reserve_clues(&mut self) {
        let margin = (self.board.w / 9. * 0.75).clamp(14., 48.);
        self.board = Rect::new(
            self.board.x + margin,
            self.board.y + margin,
            self.board.w - 2. * margin,
            self.board.h - 2. * margin,
        );
    }
    pub fn number(&self, d: usize) -> Rect {
        let unit = (self.pad.w - 18.) / 4.;
        let i = d - 1;
        Rect::new(
            self.pad.x + (i % 3) as f32 * (unit + 6.),
            self.pad.y + (i / 3) as f32 * (unit + 6.),
            unit,
            unit,
        )
    }
    pub fn tool(&self, i: usize) -> Rect {
        let u = (self.pad.w - 18.) / 4.;
        Rect::new(
            self.pad.x + 3. * (u + 6.),
            self.pad.y + i as f32 * (u + 6.),
            u,
            u,
        )
    }
    pub fn erase(&self, notes_available: bool) -> Rect {
        let u = (self.pad.w - 18.) / 4.;
        Rect::new(
            self.pad.x,
            self.pad.y + 3. * (u + 6.),
            if notes_available {
                u * 2. + 6.
            } else {
                u * 3. + 12.
            },
            u,
        )
    }
    pub fn candidates(&self) -> Rect {
        let u = (self.pad.w - 18.) / 4.;
        Rect::new(self.pad.x + 2. * (u + 6.), self.pad.y + 3. * (u + 6.), u, u)
    }
    pub fn action(&self, i: usize) -> Rect {
        let u = self.actions.w / 6.;
        Rect::new(
            self.actions.x + i as f32 * u,
            self.actions.y,
            u,
            self.actions.h,
        )
    }
}

pub struct SetupLayout {
    pub tabs: [Rect; 3],
    pub cards: [Rect; 6],
    pub markings: [Rect; 2],
    pub difficulty: [Rect; 3],
    pub new: Rect,
    pub resume: Rect,
}
impl SetupLayout {
    pub fn new(w: f32, h: f32, saved: bool) -> Self {
        let width = (w - 32.).min(780.);
        let x = (w - width) / 2.;
        let side = w >= 480. && h < 500.;
        let gap = if side { 6. } else { 10. };
        let gallery = if side { width - 218. } else { width };
        let cols = if side {
            2
        } else if w >= 680. {
            3
        } else {
            2
        };
        let rows = 6 / cols;
        let top = 114.;
        let ch = if side {
            ((h - top - 10. - gap * rows as f32) / rows as f32).clamp(44., 100.)
        } else {
            ((h - top - 166. - gap * rows as f32) / rows as f32).clamp(44., 156.)
        };
        let cw = (gallery - gap * (cols - 1) as f32) / cols as f32;
        let cards = std::array::from_fn(|i| {
            Rect::new(
                x + (i % cols) as f32 * (cw + gap),
                top + (i / cols) as f32 * (ch + gap),
                cw,
                ch,
            )
        });
        let sx = if side { x + gallery + 18. } else { x };
        let sw = if side { 200. } else { width };
        let sy = if side {
            top
        } else {
            top + rows as f32 * (ch + gap) + 6.
        };
        let markings = std::array::from_fn(|i| {
            Rect::new(sx + i as f32 * (sw + 10.) / 2., sy, (sw - 10.) / 2., 44.)
        });
        let difficulty = std::array::from_fn(|i| {
            Rect::new(sx + i as f32 * sw / 3., sy + 50., sw / 3. - 4., 42.)
        });
        let nw = if saved { (sw - 10.) / 2. } else { sw };
        let new = Rect::new(sx, sy + 106., nw, 46.);
        let resume = Rect::new(new.right() + 10., new.y, nw, 46.);
        let tabs = std::array::from_fn(|i| {
            Rect::new(x + i as f32 * width / 3., 64., width / 3. - 4., 44.)
        });
        Self {
            tabs,
            cards,
            markings,
            difficulty,
            new,
            resume,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paged_rule_controls_and_outside_clues_fit_all_screen_shapes() {
        for (w, h) in [
            (320., 568.),
            (390., 844.),
            (768., 1024.),
            (1440., 900.),
            (568., 320.),
            (1024., 600.),
        ] {
            for saved in [false, true] {
                let l = SetupLayout::new(w, h, saved);
                let mut controls = Vec::from(l.tabs);
                controls.extend(l.cards);
                controls.extend(l.markings);
                controls.extend(l.difficulty);
                controls.push(l.new);
                if saved {
                    controls.push(l.resume);
                }
                for (i, r) in controls.iter().enumerate() {
                    assert!(r.w >= 44. && r.h >= 42.);
                    assert!(
                        r.x >= 0. && r.y >= 0. && r.right() <= w && r.bottom() <= h,
                        "{w} {h} {r:?}"
                    );
                    assert!(controls[i + 1..].iter().all(|s| !r.overlaps(s)));
                }
            }
            let mut l = Layout::new(w, h);
            let outer = l.board;
            l.reserve_clues();
            let u = l.board.w / 9.;
            assert!(l.board.x - u * 0.73 >= outer.x && l.board.y - u * 0.73 >= outer.y);
            assert!(l.board.right() <= outer.right() && l.board.bottom() <= outer.bottom());
        }
    }
    #[test]
    fn board_and_number_pad_fit_phones_tablets_and_landscape() {
        for (w, h) in [
            (320., 568.),
            (390., 844.),
            (768., 1024.),
            (1440., 900.),
            (568., 320.),
            (1024., 600.),
        ] {
            let l = Layout::new(w, h);
            assert!(l.board.w >= 140., "{w} {h}");
            assert!(l.board.x >= 0. && l.board.right() <= w && l.board.bottom() <= h);
            assert!(l.pad.x >= 0. && l.pad.right() <= w && l.pad.bottom() <= h);
            assert!(!l.board.overlaps(&l.pad));
            for i in 1..=9 {
                let r = l.number(i);
                assert!(r.w >= 44. && r.h >= 44.);
                assert!(r.bottom() <= h);
            }
            for i in 0..4 {
                assert!(l.tool(i).bottom() <= h);
            }
            assert!(l.actions.bottom() <= h);
            for r in [l.erase(true), l.erase(false), l.candidates()] {
                assert!(r.w >= 44. && r.h >= 44.);
                assert!(r.x >= 0. && r.right() <= w && r.bottom() <= h);
                assert!(!r.overlaps(&l.board));
            }
            assert!(!l.erase(true).overlaps(&l.candidates()));
            assert!(!l.erase(false).overlaps(&l.tool(3)));
            assert!(!l.candidates().overlaps(&l.tool(3)));
        }
    }
}
