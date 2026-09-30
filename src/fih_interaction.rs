//! Care gestures and animation poses, independent of rendering and platform input.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pose {
    pub phase: f32,
    pub look: (f32, f32),
    pub mouth: f32,
    pub chew: f32,
    pub delight: f32,
    pub blink: f32,
}
impl Pose {
    pub fn idle(seconds: f64) -> Self {
        let cycle = (seconds % 4.7) as f32;
        Self {
            phase: seconds as f32 * 2.,
            blink: (1. - (cycle - 4.4).abs() / 0.10).clamp(0., 1.),
            ..Self::default()
        }
    }
    pub fn watch(&mut self, food: (f32, f32)) {
        self.look = (food.0.clamp(-1., 1.), food.1.clamp(-1., 1.));
        self.mouth = (1.2 - food.0.hypot(food.1 - 0.28) * 0.6).clamp(0.15, 1.);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Feed,
    Wash,
    Love,
    Heal,
    Dress,
    Sleep,
    Wake,
}
#[derive(Clone, Copy, Debug)]
pub struct Animation {
    pub action: Action,
    pub started: f64,
}
impl Animation {
    pub fn duration(self) -> f64 {
        if self.action == Action::Feed {
            1.3
        } else {
            1.6
        }
    }
    pub fn progress(self, now: f64) -> f32 {
        ((now - self.started) / self.duration()).clamp(0., 1.) as f32
    }
    pub fn active(self, now: f64) -> bool {
        now - self.started < self.duration()
    }
    pub fn apply(self, pose: &mut Pose, now: f64) {
        let t = self.progress(now);
        if t >= 1. {
            return;
        }
        let pulse = (t * std::f32::consts::PI).sin();
        match self.action {
            Action::Feed => {
                pose.chew = (t * std::f32::consts::TAU * 4.).sin() * pulse;
                pose.mouth = (1. - t * 5.).max(0.);
                pose.delight = pulse * 0.35;
            }
            Action::Wash => {
                pose.blink = pulse * 0.6;
                pose.delight = pulse * 0.4;
            }
            Action::Love | Action::Heal | Action::Dress | Action::Wake => pose.delight = pulse,
            Action::Sleep => pose.blink = t,
        }
    }
}
pub fn mouth_hit(point: (f32, f32)) -> bool {
    (point.0 / 0.30).powi(2) + ((point.1 - 0.28) / 0.23).powi(2) <= 1.
}
pub fn can_feed(start: (f32, f32), drop: (f32, f32), cancelled: bool) -> bool {
    !cancelled && mouth_hit(drop) && (drop.0 - start.0).hypot(drop.1 - start.1) > 0.15
}
pub fn body_half_width(y: f32) -> f32 {
    0.94 * (1. - (y / 0.90).powi(2)).max(0.).sqrt()
}
pub fn body_hit(point: (f32, f32)) -> bool {
    (point.0 / 0.94).powi(2) + (point.1 / 0.90).powi(2) <= 1.
}
#[derive(Clone, Debug, Default)]
pub struct Lather {
    pub amount: f32,
    previous: Option<(f32, f32)>,
    sectors: u8,
}
impl Lather {
    pub fn rub(&mut self, point: (f32, f32)) -> bool {
        if !point.0.is_finite() || !point.1.is_finite() || !body_hit(point) {
            self.previous = None;
            return false;
        }
        let Some(prev) = self.previous.replace(point) else {
            return false;
        };
        let distance = (point.0 - prev.0).hypot(point.1 - prev.1);
        // A teleport or a stationary hold is not rubbing. Reset after leaving the body.
        if !(0.005..=0.45).contains(&distance) {
            return false;
        }
        self.amount = (self.amount + distance / 5.).min(1.);
        self.sectors |= 1 << (u8::from(point.0 > 0.) + 2 * u8::from(point.1 > 0.));
        true
    }
    pub fn end_stroke(&mut self) {
        self.previous = None;
    }
    pub fn ready(&self) -> bool {
        self.amount >= 0.65 && self.sectors.count_ones() >= 3
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn feeding_requires_the_mouth_instead_of_the_whole_body() {
        assert!(mouth_hit((0., 0.28)));
        for p in [(0.7, 0.3), (0., -0.5), (0., 0.8), (f32::NAN, 0.)] {
            assert!(!mouth_hit(p));
        }
    }
    #[test]
    fn feeding_taps_cancellation_and_wrong_drops_never_consume_food() {
        assert!(!can_feed((0., 0.28), (0., 0.28), false));
        assert!(!can_feed((0., 3.), (0.7, 0.3), false));
        assert!(!can_feed((0., 3.), (0., 0.28), true));
        assert!(can_feed((0., 3.), (0., 0.28), false));
    }
    #[test]
    fn garment_silhouette_stays_inside_the_shared_body() {
        for i in 0..100 {
            let y = 0.48 + i as f32 / 100. * 0.419;
            let w = body_half_width(y);
            assert!(body_hit((w * 0.999, y)));
        }
        assert_eq!(body_half_width(1.), 0.);
    }
    #[test]
    fn rubbing_progress_is_consistent_at_different_frame_rates() {
        let mut amounts = Vec::new();
        for fps in [15, 30, 60, 120, 240] {
            let mut lather = Lather::default();
            for frame in 0..fps * 3 {
                let a = frame as f32 / fps as f32 * std::f32::consts::TAU;
                lather.rub((a.cos() * 0.5, a.sin() * 0.5));
            }
            assert!(lather.ready());
            amounts.push(lather.amount);
        }
        assert!(amounts.iter().all(|&a| a == 1.));
    }
    #[test]
    fn bathing_requires_real_motion_and_body_coverage() {
        let mut soap = Lather::default();
        for _ in 0..100 {
            soap.rub((0., 0.));
        }
        assert!(!soap.ready());
        assert_eq!(soap.amount, 0.);
        soap.rub((100., 100.));
        soap.rub((0., 0.));
        assert_eq!(soap.amount, 0.);
        for _ in 0..8 {
            for i in 0..64 {
                let a = i as f32 * std::f32::consts::TAU / 64.;
                soap.rub((a.cos() * 0.5, a.sin() * 0.5));
            }
        }
        assert!(soap.ready());
        soap.clear();
        assert!(!soap.ready());
    }
    #[test]
    fn idle_gaze_and_actions_are_time_based_and_finish() {
        let mut pose = Pose::idle(2.);
        pose.watch((0.6, 0.1));
        assert!(pose.look.0 > 0.);
        assert!(pose.mouth > 0.);
        for action in [
            Action::Feed,
            Action::Wash,
            Action::Love,
            Action::Heal,
            Action::Dress,
            Action::Sleep,
            Action::Wake,
        ] {
            let animation = Animation {
                action,
                started: 10.,
            };
            assert!(animation.active(10.5));
            assert!(!animation.active(12.));
            let mut pose = Pose::default();
            animation.apply(&mut pose, 12.);
            assert_eq!(pose.delight, 0.);
            assert_eq!(pose.chew, 0.);
        }
    }
}
