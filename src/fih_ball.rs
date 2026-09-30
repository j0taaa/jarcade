//! Playroom ball in units of room width. No screen-dependent timing or background work.
#[derive(Clone, Debug)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub held: bool,
    pub radius: f32,
    height: f32,
    sampled_at: f64,
}
impl Default for Ball {
    fn default() -> Self {
        Self {
            x: 0.75,
            y: 0.8,
            vx: 0.,
            vy: 0.,
            held: false,
            radius: 0.065,
            height: 1.,
            sampled_at: 0.,
        }
    }
}
impl Ball {
    pub fn resize(&mut self, height: f32) {
        let next = height.max(self.radius * 2. + 0.01);
        self.y *= next / self.height;
        self.height = next;
        self.x = self.x.clamp(self.radius, 1. - self.radius);
        self.y = self.y.clamp(self.radius, next - self.radius);
    }
    pub fn hit(&self, x: f32, y: f32) -> bool {
        (x - self.x).hypot(y - self.y) <= self.radius * 1.4
    }
    pub fn grab(&mut self, now: f64) {
        self.held = true;
        self.vx = 0.;
        self.vy = 0.;
        self.sampled_at = now;
    }
    pub fn drag(&mut self, x: f32, y: f32, now: f64) {
        if !self.held || !x.is_finite() || !y.is_finite() || !now.is_finite() {
            return;
        }
        let x = x.clamp(self.radius, 1. - self.radius);
        let y = y.clamp(self.radius, self.height - self.radius);
        if (x - self.x).hypot(y - self.y) < 0.000001 {
            return;
        }
        let dt = (now - self.sampled_at) as f32;
        if dt > 0.001 && dt < 0.25 {
            let previous = (-dt / 0.025).exp();
            self.vx = self.vx * previous + ((x - self.x) / dt).clamp(-3., 3.) * (1. - previous);
            self.vy = self.vy * previous + ((y - self.y) / dt).clamp(-3., 3.) * (1. - previous);
        }
        self.x = x;
        self.y = y;
        self.sampled_at = now;
    }
    pub fn release(&mut self, now: f64) -> bool {
        if !self.held {
            return false;
        }
        self.held = false;
        if now - self.sampled_at > 0.15 {
            self.vx = 0.;
            self.vy = 0.;
        }
        self.moving()
    }
    pub fn cancel(&mut self) {
        self.held = false;
        self.vx = 0.;
        self.vy = 0.;
    }
    pub fn moving(&self) -> bool {
        self.held
            || self.vx.abs() + self.vy.abs() > 0.025
            || self.y < self.height - self.radius - 0.001
    }
    pub fn advance(&mut self, seconds: f64) {
        if self.held || !seconds.is_finite() || seconds <= 0. || seconds > 0.75 || !self.moving() {
            return;
        }
        let mut left = seconds;
        while left > 1e-7 {
            let dt = left.min(1. / 240.) as f32;
            left -= f64::from(dt);
            self.vy += dt * 1.35;
            self.x += self.vx * dt;
            self.y += self.vy * dt;
            if self.x < self.radius {
                self.x = self.radius;
                self.vx = self.vx.abs() * 0.88;
            }
            if self.x > 1. - self.radius {
                self.x = 1. - self.radius;
                self.vx = -self.vx.abs() * 0.88;
            }
            if self.y < self.radius {
                self.y = self.radius;
                self.vy = self.vy.abs() * 0.88;
            }
            if self.y > self.height - self.radius {
                self.y = self.height - self.radius;
                self.vy = -self.vy.abs() * 0.76;
                self.vx *= 0.93;
                if self.vy.abs() < 0.10 {
                    self.vy = 0.;
                    if self.vx.abs() < 0.03 {
                        self.vx = 0.;
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn throw_bounces_inside_boundaries_and_stops_without_background_work() {
        let mut b = Ball::default();
        b.resize(0.6);
        b.grab(1.);
        b.drag(0.8, 0.15, 1.05);
        assert!(b.release(1.05));
        for _ in 0..60 * 30 {
            b.advance(1. / 60.);
            assert!(b.x >= b.radius && b.x <= 1. - b.radius);
            assert!(b.y >= b.radius && b.y <= b.height - b.radius);
        }
        assert!(!b.moving());
        let x = b.x;
        b.advance(60.);
        assert_eq!(b.x, x);
    }
    #[test]
    fn throw_velocity_does_not_depend_on_render_rate_or_stationary_frames() {
        for fps in [30, 60, 120, 240] {
            let mut ball = Ball {
                x: 0.5,
                y: 0.5,
                ..Ball::default()
            };
            ball.grab(0.);
            for frame in 1..=fps / 5 {
                let t = f64::from(frame) / f64::from(fps);
                ball.drag(0.5 + t as f32, 0.5 - t as f32, t);
            }
            assert!((ball.vx - 1.).abs() < 0.001);
            assert!((ball.vy + 1.).abs() < 0.001);
            let velocity = (ball.vx, ball.vy);
            ball.drag(ball.x, ball.y, 0.21);
            assert_eq!((ball.vx, ball.vy), velocity);
            assert!(ball.release(0.21));
            ball.grab(0.3);
            ball.drag(0.6, 0.4, 0.35);
            ball.release(0.6);
            assert_eq!((ball.vx, ball.vy), (0., 0.));
        }
    }
    #[test]
    fn physics_are_consistent_across_refresh_rates_and_cancel_does_not_throw() {
        let mut results = Vec::new();
        for fps in [30, 60, 120, 240] {
            let mut b = Ball {
                vx: 1.,
                vy: -1.,
                ..Ball::default()
            };
            for _ in 0..fps {
                b.advance(1. / fps as f64);
            }
            results.push((b.x, b.y));
        }
        assert!(results.iter().all(|&(x,y)|(x-results[0].0).abs()<0.001 && (y-results[0].1).abs()<0.001));
        let mut b = Ball::default();
        b.grab(1.);
        b.drag(0.3, 0.3, 1.05);
        b.cancel();
        assert!(!b.release(1.05));
        assert_eq!(b.vx, 0.);
    }
}
