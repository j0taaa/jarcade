#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum Pulse {
    Tap,
    Eat,
    Lost,
    Won,
}

#[derive(Default)]
pub struct Feedback {
    last: Option<f64>,
}

impl Feedback {
    /// Never buzz every frame, nor flood the motor with repeated input.
    pub fn request(&mut self, pulse: Pulse, enabled: bool, now: f64) -> Option<Pulse> {
        if !enabled {
            return None;
        }
        if pulse == Pulse::Tap && self.last.is_some_and(|last| now - last < 0.07) {
            return None;
        }
        self.last = Some(now);
        Some(pulse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn respects_opt_out_and_limits_repeat_taps() {
        let mut feedback = Feedback::default();
        assert_eq!(feedback.request(Pulse::Tap, false, 0.0), None);
        assert_eq!(feedback.request(Pulse::Tap, true, 0.0), Some(Pulse::Tap));
        assert_eq!(feedback.request(Pulse::Tap, true, 0.03), None);
        assert_eq!(feedback.request(Pulse::Eat, true, 0.04), Some(Pulse::Eat));
        assert_eq!(feedback.request(Pulse::Lost, true, 0.05), Some(Pulse::Lost));
        assert_eq!(feedback.request(Pulse::Won, false, 1.0), None);
    }
}
