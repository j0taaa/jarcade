//! Two human players, bounded paddles and display-independent fixed-step physics.
use macroquad::prelude::{Vec2, vec2};
use serde::{Deserialize, Serialize};
pub const HEIGHT: f32 = 1.6;
pub const MALLET: f32 = 0.085;
pub const PUCK: f32 = 0.035;
const STEP: f32 = 1. / 240.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Ready,
    Playing,
    Paused,
    Goal(u8),
    Finished(u8),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Hit,
    Goal(u8),
    Won(u8),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hockey {
    pub phase: Phase,
    pub scores: [u8; 2],
    pub matches: [u32; 2],
    pub round: u32,
    puck: [f32; 2],
    velocity: [f32; 2],
    paddles: [[f32; 2]; 2],
    targets: [[f32; 2]; 2],
    server: u8,
    #[serde(skip)]
    accumulator: f32,
    #[serde(skip)]
    previous_puck: [f32; 2],
    #[serde(skip)]
    previous_paddles: [[f32; 2]; 2],
}
impl Default for Hockey {
    fn default() -> Self {
        let mut g = Self {
            phase: Phase::Ready,
            scores: [0; 2],
            matches: [0; 2],
            round: 1,
            puck: [0.5, 1.12],
            velocity: [0.; 2],
            paddles: [[0.; 2]; 2],
            targets: [[0.; 2]; 2],
            server: 0,
            accumulator: 0.,
            previous_puck: [0.; 2],
            previous_paddles: [[0.; 2]; 2],
        };
        g.positions();
        g
    }
}
impl Hockey {
    fn positions(&mut self) {
        self.paddles = [[0.5, HEIGHT - 0.22], [0.5, 0.22]];
        self.targets = self.paddles;
        self.puck = [
            0.5,
            if self.server == 0 {
                HEIGHT - 0.48
            } else {
                0.48
            },
        ];
        self.velocity = [0.; 2];
        self.accumulator = 0.;
        self.previous_puck = self.puck;
        self.previous_paddles = self.paddles;
    }
    pub fn puck(&self) -> Vec2 {
        Vec2::from(self.puck)
    }
    pub fn paddle(&self, player: usize) -> Vec2 {
        Vec2::from(self.paddles[player.min(1)])
    }
    pub fn target(&self, player: usize) -> Vec2 {
        Vec2::from(self.targets[player.min(1)])
    }
    pub fn render_puck(&self) -> Vec2 {
        if self.phase == Phase::Playing {
            Vec2::from(self.previous_puck)
                .lerp(self.puck(), (self.accumulator / STEP).clamp(0., 1.))
        } else {
            self.puck()
        }
    }
    pub fn render_paddle(&self, player: usize) -> Vec2 {
        if self.phase == Phase::Playing {
            Vec2::from(self.previous_paddles[player.min(1)])
                .lerp(self.paddle(player), (self.accumulator / STEP).clamp(0., 1.))
        } else {
            self.paddle(player)
        }
    }
    pub fn bounds(player: usize, p: Vec2) -> Vec2 {
        vec2(
            p.x.clamp(MALLET, 1. - MALLET),
            p.y.clamp(
                if player == 0 {
                    HEIGHT / 2. + MALLET
                } else {
                    MALLET
                },
                if player == 0 {
                    HEIGHT - MALLET
                } else {
                    HEIGHT / 2. - MALLET
                },
            ),
        )
    }
    pub fn move_to(&mut self, player: usize, p: Vec2) -> bool {
        if player > 1 || !p.is_finite() || self.phase != Phase::Playing {
            return false;
        }
        let target = Self::bounds(player, p);
        if self.target(player).distance_squared(target) < 0.000001 {
            return false;
        }
        self.targets[player] = target.to_array();
        true
    }
    pub fn play(&mut self) -> bool {
        match self.phase {
            Phase::Ready | Phase::Goal(_) => {
                self.positions();
                self.velocity = [0.15, if self.server == 0 { -0.6 } else { 0.6 }];
            }
            Phase::Paused => {
                self.previous_puck = self.puck;
                self.previous_paddles = self.paddles;
            }
            Phase::Playing | Phase::Finished(_) => return false,
        }
        self.phase = Phase::Playing;
        self.accumulator = 0.;
        true
    }
    pub fn pause(&mut self) -> bool {
        if self.phase != Phase::Playing {
            return false;
        }
        self.phase = Phase::Paused;
        self.accumulator = 0.;
        self.targets = self.paddles;
        true
    }
    pub fn rematch(&mut self) {
        self.round = self.round.saturating_add(1);
        self.server = ((self.round - 1) % 2) as u8;
        self.scores = [0; 2];
        self.phase = Phase::Ready;
        self.positions();
    }
    pub fn advance(&mut self, dt: f32) -> Vec<Event> {
        let mut events = vec![];
        if self.phase != Phase::Playing || !dt.is_finite() || dt <= 0. {
            return events;
        }
        // Interruptions cannot fast-forward a match or leave it simulating offscreen.
        if dt > 0.25 {
            self.pause();
            return events;
        }
        self.accumulator += dt;
        while self.accumulator + 0.0000001 >= STEP && self.phase == Phase::Playing {
            self.accumulator = (self.accumulator - STEP).max(0.);
            self.step(&mut events);
        }
        events
    }
    fn step(&mut self, events: &mut Vec<Event>) {
        self.previous_puck = self.puck;
        self.previous_paddles = self.paddles;
        let mut pv = [Vec2::ZERO; 2];
        for (i, v) in pv.iter_mut().enumerate() {
            let old = self.paddle(i);
            let d = self.target(i) - old;
            let new = old + d.clamp_length_max(8. * STEP);
            *v = (new - old) / STEP;
            self.paddles[i] = new.to_array();
        }
        let mut p = self.puck();
        let mut v = Vec2::from(self.velocity);
        p += v * STEP;
        if p.x < PUCK {
            p.x = PUCK;
            v.x = v.x.abs();
        } else if p.x > 1. - PUCK {
            p.x = 1. - PUCK;
            v.x = -v.x.abs();
        }
        let opening = (0.3 + PUCK..=0.7 - PUCK).contains(&p.x);
        if !opening {
            if p.y < PUCK {
                p.y = PUCK;
                v.y = v.y.abs();
            } else if p.y > HEIGHT - PUCK {
                p.y = HEIGHT - PUCK;
                v.y = -v.y.abs();
            }
        }
        for (i, paddle_v) in pv.into_iter().enumerate() {
            let d = p - self.paddle(i);
            let distance = d.length();
            if distance < MALLET + PUCK {
                let n = if distance > 0.00001 {
                    d / distance
                } else {
                    vec2(0., if i == 0 { -1. } else { 1. })
                };
                p = self.paddle(i) + n * (MALLET + PUCK + 0.0001);
                let approach = (v - paddle_v).dot(n);
                if approach < 0. {
                    v -= n * 2. * approach;
                    v = v.clamp_length_max(4.);
                    events.push(Event::Hit);
                }
            }
        }
        p.x = p.x.clamp(PUCK, 1. - PUCK);
        if p.y < -PUCK || p.y > HEIGHT + PUCK {
            let winner = if p.y < 0. { 0 } else { 1 };
            self.scores[winner] += 1;
            self.server = 1 - winner as u8;
            self.phase = if self.scores[winner] >= 7 {
                self.matches[winner] = self.matches[winner].saturating_add(1);
                Phase::Finished(winner as u8)
            } else {
                Phase::Goal(winner as u8)
            };
            self.positions();
            events.push(if self.scores[winner] >= 7 {
                Event::Won(winner as u8)
            } else {
                Event::Goal(winner as u8)
            });
            return;
        }
        v *= (-0.045 * STEP).exp();
        self.puck = p.to_array();
        self.velocity = v.to_array();
    }
    pub fn valid(&self) -> bool {
        self.round > 0
            && self.server < 2
            && self.scores.iter().all(|&n| n <= 7)
            && self.puck().is_finite()
            && self.puck().x >= PUCK
            && self.puck().x <= 1. - PUCK
            && self.puck().y >= -PUCK
            && self.puck().y <= HEIGHT + PUCK
            && Vec2::from(self.velocity).is_finite()
            && Vec2::from(self.velocity).length() <= 4.01
            && (0..2).all(|i| {
                self.paddle(i).is_finite()
                    && self.target(i).is_finite()
                    && self.paddle(i).distance(Self::bounds(i, self.paddle(i))) < 0.0001
                    && self.target(i).distance(Self::bounds(i, self.target(i))) < 0.0001
            })
            && match self.phase {
                Phase::Finished(p) => {
                    p < 2 && self.scores[p as usize] == 7 && self.scores[1 - p as usize] < 7
                }
                Phase::Goal(p) => {
                    p < 2 && self.scores[p as usize] > 0 && self.scores.iter().all(|&n| n < 7)
                }
                _ => self.scores.iter().all(|&n| n < 7),
            }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interpolated_motion_is_uniform_at_144_hz_and_pause_draws_current_state() {
        let mut g = Hockey::default();
        g.play();
        g.puck = [0.5, 0.8];
        g.previous_puck = g.puck;
        g.velocity = [0.2, 0.];
        let mut previous = g.render_puck();
        let mut distances = vec![];
        for i in 0..144 {
            g.advance(1. / 144.);
            let p = g.render_puck();
            if i > 2 {
                distances.push(p.x - previous.x);
            }
            previous = p;
        }
        let min = distances.iter().copied().fold(f32::INFINITY, f32::min);
        let max = distances.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min < 0.00008, "uneven display motion: {min} to {max}");
        g.pause();
        assert_eq!(g.render_puck(), g.puck());
        assert_eq!(g.render_paddle(0), g.paddle(0));
    }
    #[test]
    fn goals_posts_wins_pause_and_rematch() {
        let mut g = Hockey::default();
        g.play();
        g.paddles = [[0.12, 1.4], [0.12, 0.2]];
        g.targets = g.paddles;
        g.puck = [0.5, 0.04];
        g.velocity = [0., -3.];
        assert_eq!(g.advance(0.05), vec![Event::Goal(0)]);
        assert_eq!(g.scores, [1, 0]);
        g.advance(0.2);
        assert_eq!(g.scores, [1, 0]);
        g.play();
        g.puck = [0.15, 0.04];
        g.velocity = [0., -2.];
        g.advance(0.025);
        assert!(g.velocity[1] > 0.);
        assert_eq!(g.scores, [1, 0]);
        g.scores = [6, 0];
        g.paddles = [[0.12, 1.4], [0.12, 0.2]];
        g.targets = g.paddles;
        g.puck = [0.5, 0.04];
        g.velocity = [0., -3.];
        assert_eq!(g.advance(0.05), vec![Event::Won(0)]);
        assert_eq!(g.matches, [1, 0]);
        assert!(g.valid());
        assert!(!g.play());
        g.rematch();
        assert_eq!(g.matches, [1, 0]);
        assert_eq!(g.scores, [0, 0]);
        assert_eq!(g.server, 1);
        g.play();
        let puck = g.puck;
        g.advance(2.);
        assert_eq!(g.phase, Phase::Paused);
        assert_eq!(g.puck, puck);
    }
    #[test]
    fn physics_is_independent_of_frame_rate_and_never_tunnels() {
        let mut a = Hockey::default();
        a.play();
        a.puck = [0.5, 0.8];
        a.velocity = [3., 0.];
        let mut b = a.clone();
        let mut c = a.clone();
        for _ in 0..30 {
            a.advance(1. / 30.);
        }
        for _ in 0..120 {
            b.advance(1. / 120.);
        }
        for _ in 0..144 {
            c.advance(1. / 144.);
        }
        assert!(a.puck().distance(b.puck()) < 0.0001);
        assert!(a.puck().distance(c.puck()) < 0.0001);
        assert!(a.valid());
        a.puck = [0.5, 1.21];
        a.velocity = [0., 4.];
        a.paddles[0] = [0.5, 1.38];
        a.targets = a.paddles;
        assert!(a.advance(0.05).contains(&Event::Hit));
        assert!(a.velocity[1] < 0.);
    }
    #[test]
    fn independent_paddle_targets_stay_in_their_halves() {
        let mut g = Hockey::default();
        g.play();
        g.move_to(0, vec2(-5., -5.));
        g.move_to(1, vec2(5., 5.));
        for _ in 0..100 {
            g.advance(0.01);
        }
        assert!(g.valid());
        assert!(g.paddle(0).y >= HEIGHT / 2. + MALLET);
        assert!(g.paddle(1).y <= HEIGHT / 2. - MALLET);
        assert!(!g.move_to(2, vec2(0., 0.)));
        assert!(!g.move_to(0, vec2(f32::NAN, 0.)));
    }
}
