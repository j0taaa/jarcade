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
#[cfg(test)]
mod tests {
    use super::*;
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
