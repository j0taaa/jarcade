//! Additional mini-game rules. Positions are normalized; integration lives in Round.
use crate::fih_games::Kind;
#[derive(Clone, Copy, Debug)]
pub struct Obstacle {
    pub x: f32,
    pub y: f32,
    pub good: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub lane: usize,
    pub y: f32,
}
pub struct Extras {
    pub ball: (f32, f32),
    pub velocity: (f32, f32),
    pub bricks: [bool; 24],
    pub wave: u32,
    pub obstacles: Vec<Obstacle>,
    pub notes: Vec<Note>,
    pub combo: u32,
    spawn: f64,
    rng: u64,
    last_beat: f64,
}
impl Extras {
    pub fn new(seed: u64) -> Self {
        Self {
            ball: (0.5, 0.72),
            velocity: (0.3, -0.46),
            bricks: [true; 24],
            wave: 1,
            obstacles: Vec::new(),
            notes: Vec::new(),
            combo: 0,
            spawn: 0.,
            rng: seed.max(1),
            last_beat: -1.,
        }
    }
    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % 10000) as f32 / 10000.
    }
    pub fn level(&self, elapsed: f64) -> u32 {
        self.wave.max(1 + (elapsed / 8.) as u32)
    }
    /// Return earned points and lost hearts for this integration step.
    pub fn advance(&mut self, kind: Kind, dt: f64, elapsed: f64, player: f32) -> (u32, u8) {
        let mut points = 0;
        let mut lost = 0;
        let level = self.level(elapsed) as f32;
        match kind {
            Kind::Rally => {
                self.ball.0 += self.velocity.0 * dt as f32;
                self.ball.1 += self.velocity.1 * dt as f32;
                if self.ball.0 < 0.035 {
                    self.ball.0 = 0.035;
                    self.velocity.0 = self.velocity.0.abs();
                }
                if self.ball.0 > 0.965 {
                    self.ball.0 = 0.965;
                    self.velocity.0 = -self.velocity.0.abs();
                }
                if self.ball.1 < 0.03 {
                    self.ball.1 = 0.03;
                    self.velocity.1 = self.velocity.1.abs();
                }
                let speed = (0.48 + self.wave as f32 * 0.08 + elapsed as f32 * 0.003).min(1.1);
                if self.velocity.1 > 0.
                    && (0.84..0.88).contains(&self.ball.1)
                    && (self.ball.0 - player).abs() < self.paddle_width(elapsed) / 2. + 0.025
                {
                    let offset = (self.ball.0 - player) / (self.paddle_width(elapsed) / 2.);
                    self.velocity.0 = offset.clamp(-0.85, 0.85) * speed;
                    self.velocity.1 = -(speed * speed - self.velocity.0 * self.velocity.0)
                        .max(0.05)
                        .sqrt();
                    self.ball.1 = 0.839;
                }
                for i in 0..24 {
                    if !self.bricks[i] {
                        continue;
                    }
                    let x = 0.10 + (i % 6) as f32 * 0.16;
                    let y = 0.13 + (i / 6) as f32 * 0.065;
                    if (self.ball.0 - x).abs() < 0.072 && (self.ball.1 - y).abs() < 0.04 {
                        self.bricks[i] = false;
                        self.velocity.1 = -self.velocity.1;
                        points += 1;
                        break;
                    }
                }
                if self.ball.1 > 1.03 {
                    lost = 1;
                    self.ball = (player, 0.72);
                    self.velocity = (0.25, -speed);
                }
                if self.bricks.iter().all(|b| !*b) {
                    self.wave += 1;
                    self.bricks.fill(true);
                    points += 4;
                    self.ball = (player, 0.72);
                    self.velocity = (0.32, -speed);
                }
            }
            Kind::Dodge => {
                self.spawn -= dt;
                if self.spawn <= 0. {
                    let x = 0.08 + self.random() * 0.84;
                    let good = self.random() < 0.28;
                    self.obstacles.push(Obstacle { x, y: -0.05, good });
                    self.spawn += (0.72 - level as f64 * 0.075).max(0.25);
                }
                for o in &mut self.obstacles {
                    o.y += dt as f32 * (0.27 + level * 0.055);
                }
                let mut i = 0;
                while i < self.obstacles.len() {
                    let o = self.obstacles[i];
                    if (o.y - 0.82).abs() < 0.065 && (o.x - player).abs() < 0.07 {
                        if o.good {
                            points += 2;
                        } else {
                            lost += 1;
                        }
                        self.obstacles.remove(i);
                    } else if o.y > 1.05 {
                        self.obstacles.remove(i);
                    } else {
                        i += 1;
                    }
                }
                // Surviving each five-second marker earns points even without pearls.
                if (elapsed / 5.).floor() > ((elapsed - dt) / 5.).floor() {
                    points += 2;
                }
            }
            Kind::Beats => {
                self.spawn -= dt;
                if self.spawn <= 0. {
                    let lane = (self.random() * 3.) as usize;
                    self.notes.push(Note { lane, y: 0.05 });
                    self.spawn += (0.9 - level as f64 * 0.08).max(0.36);
                }
                for n in &mut self.notes {
                    n.y += dt as f32 * (0.28 + level * 0.035);
                }
                let missed = self.notes.iter().filter(|n| n.y > 0.96).count() as u8;
                if missed > 0 {
                    self.combo = 0;
                    lost = missed;
                }
                self.notes.retain(|n| n.y <= 0.96);
            }
            _ => {}
        }
        (points, lost)
    }
    pub fn paddle_width(&self, elapsed: f64) -> f32 {
        (0.30 - elapsed as f32 * 0.0012 - (self.wave - 1) as f32 * 0.02).max(0.16)
    }
    pub fn beat(&mut self, lane: usize, elapsed: f64) -> (bool, u32, u8) {
        if lane >= 3 || elapsed - self.last_beat < 0.10 {
            return (false, 0, 0);
        }
        self.last_beat = elapsed;
        if let Some(i) = self
            .notes
            .iter()
            .position(|n| n.lane == lane && (n.y - 0.82).abs() <= 0.085)
        {
            let note = self.notes.remove(i);
            self.combo += 1;
            (
                true,
                if (note.y - 0.82).abs() < 0.025 {
                    2 + self.combo / 5
                } else {
                    1
                },
                0,
            )
        } else {
            self.combo = 0;
            (true, 0, 1)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rally_paddle_requires_contact_and_each_shell_breaks_once() {
        let mut e = Extras::new(42);
        e.ball = (0.5, 0.85);
        e.velocity = (0.1, 0.5);
        e.advance(Kind::Rally, 1. / 240., 1., 0.5);
        assert!(e.velocity.1 < 0.);
        e.ball = (0.10, 0.13);
        e.velocity = (0., 0.3);
        assert_eq!(e.advance(Kind::Rally, 1. / 240., 1., 0.5).0, 1);
        assert!(!e.bricks[0]);
        assert_eq!(e.advance(Kind::Rally, 1. / 240., 1., 0.5).0, 0);
        assert!(e.paddle_width(30.) < e.paddle_width(0.));
    }
    #[test]
    fn dodge_rewards_collectibles_and_hazards_cost_once() {
        let mut e = Extras::new(1);
        e.spawn = 10.;
        e.obstacles = vec![
            Obstacle {
                x: 0.5,
                y: 0.82,
                good: true,
            },
            Obstacle {
                x: 0.5,
                y: 0.82,
                good: false,
            },
        ];
        assert_eq!(e.advance(Kind::Dodge, 1. / 240., 1., 0.5), (2, 1));
        assert_eq!(e.advance(Kind::Dodge, 1. / 240., 1., 0.5), (0, 0));
        assert!(e.level(24.) > e.level(0.));
    }
    #[test]
    fn beats_require_lane_and_timing_and_never_reward_double_taps() {
        let mut e = Extras::new(1);
        e.notes = vec![Note { lane: 1, y: 0.82 }];
        assert_eq!(e.beat(1, 1.), (true, 2, 0));
        assert_eq!(e.beat(1, 1.01), (false, 0, 0));
        assert_eq!(e.beat(1, 1.2), (true, 0, 1));
        e.notes = vec![Note { lane: 0, y: 0.3 }];
        assert_eq!(e.beat(0, 2.), (true, 0, 1));
        assert_eq!(e.notes.len(), 1);
    }
}
