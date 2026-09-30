/// Measures rendered frames, never schedules extra work just to show a counter.
#[derive(Default)]
pub struct FpsCounter {
    last: Option<f64>,
    elapsed: f64,
    frames: u32,
    value: Option<u32>,
}
impl FpsCounter {
    pub fn record(&mut self, now: f64, continuous: bool) -> Option<u32> {
        if !continuous {
            *self = Self::default();
            return None;
        }
        if let Some(last) = self.last {
            let dt = now - last;
            if !(0.0..=0.75).contains(&dt) {
                *self = Self::default();
            } else if dt > 0.0 {
                self.elapsed += dt;
                self.frames += 1;
                if self.elapsed >= 0.5 {
                    self.value = Some((f64::from(self.frames) / self.elapsed).round() as u32);
                    self.frames = 0;
                    self.elapsed = 0.0;
                }
            }
        }
        self.last = Some(now);
        self.value
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_render_rate_including_saver_and_high_refresh() {
        for fps in [7, 30, 60, 90, 120, 144, 165, 240] {
            let mut meter = FpsCounter::default();
            for frame in 0..=fps * 2 {
                meter.record(f64::from(frame) / f64::from(fps), true);
            }
            assert_eq!(meter.value, Some(fps));
        }
    }
    #[test]
    fn idle_and_background_gaps_never_leave_a_stale_reading() {
        let mut meter = FpsCounter::default();
        for frame in 0..=60 {
            meter.record(f64::from(frame) / 60., true);
        }
        assert_eq!(meter.record(1.01, false), None);
        assert_eq!(meter.record(50., true), None);
        assert_eq!(meter.record(70., true), None);
    }
}
