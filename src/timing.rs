pub const STEP_SECONDS: f64 = 0.14;

/// A fixed-step clock shared by both render modes. A long interruption pauses
/// play instead of advancing the snake through unseen steps.
#[derive(Default)]
pub struct TickClock {
    last: Option<f64>,
    accumulated: f64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Advance {
    None,
    Steps(u32),
    Interrupted,
}

impl TickClock {
    /// Commit the first segment immediately so starting has no idle tick.
    pub fn start(&mut self, now: f64) {
        self.reset(now);
        self.accumulated = STEP_SECONDS;
    }

    /// Resume the unfinished segment without jumping forward or waiting again.
    pub fn resume(&mut self, now: f64) {
        self.last = Some(now);
    }

    pub fn reset(&mut self, now: f64) {
        self.last = Some(now);
        self.accumulated = 0.0;
    }

    pub fn advance(&mut self, now: f64) -> Advance {
        let Some(last) = self.last.replace(now) else {
            return Advance::None;
        };
        let elapsed = now - last;
        if !(0.0..=0.75).contains(&elapsed) {
            self.accumulated = 0.0;
            return Advance::Interrupted;
        }
        self.accumulated += elapsed;
        let steps = ((self.accumulated + 1e-9) / STEP_SECONDS).floor() as u32;
        self.accumulated = (self.accumulated - f64::from(steps) * STEP_SECONDS).max(0.0);
        if steps == 0 {
            Advance::None
        } else {
            Advance::Steps(steps)
        }
    }

    pub fn until_tick(&self) -> f64 {
        (STEP_SECONDS - self.accumulated).max(0.001)
    }
    pub fn fraction(&self) -> f32 {
        (self.accumulated / STEP_SECONDS) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rendering_frequency_does_not_change_game_speed() {
        for fps in [7, 15, 30, 60, 90, 120, 144, 165, 240] {
            let mut clock = TickClock::default();
            clock.reset(0.0);
            let mut ticks = 0;
            for frame in 1..=fps * 14 {
                if let Advance::Steps(n) = clock.advance(f64::from(frame) / f64::from(fps)) {
                    ticks += n;
                }
            }
            assert_eq!(ticks, 100);
        }
    }
    #[test]
    fn interruption_never_catches_up_unseen_gameplay() {
        let mut clock = TickClock::default();
        clock.reset(1.0);
        assert_eq!(clock.advance(30.0), Advance::Interrupted);
        assert_eq!(clock.advance(30.14), Advance::Steps(1));
    }
    #[test]
    fn resuming_starts_with_a_full_step() {
        let mut clock = TickClock::default();
        clock.reset(1.0);
        clock.advance(1.1);
        clock.reset(5.0);
        assert_eq!(clock.advance(5.1), Advance::None);
        assert_eq!(clock.advance(5.14), Advance::Steps(1));
    }

    #[test]
    fn start_animates_immediately_and_resume_preserves_motion_phase() {
        let mut clock = TickClock::default();
        clock.start(1.);
        assert_eq!(clock.advance(1.), Advance::Steps(1));
        assert_eq!(clock.advance(1.07), Advance::None);
        assert!((clock.fraction() - 0.5).abs() < 0.00001);
        clock.resume(30.);
        assert_eq!(clock.advance(30.), Advance::None);
        assert!((clock.fraction() - 0.5).abs() < 0.00001);
        assert_eq!(clock.advance(30.07), Advance::Steps(1));
    }
}
