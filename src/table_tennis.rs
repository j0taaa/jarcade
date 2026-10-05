//! Offline table tennis rules. Shots cross the net, bounce once, and can only
//! be returned after that bounce. No side walls; missing a return loses a point.
use macroquad::prelude::{Vec2, vec2};

const STEP: f64 = 1.0 / 240.0;
pub const RACKET_RADIUS: f32 = 0.085;
pub const RACKET_HEIGHT: f32 = 0.06;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}
impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
        }
    }
    fn speed(self) -> f32 {
        match self {
            Self::Easy => 0.30,
            Self::Normal => 0.49,
            Self::Hard => 0.64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ready,
    Rally,
    Paused,
    Point,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    You,
    Computer,
}
impl Side {
    pub fn label(self) -> &'static str {
        match self {
            Self::You => "You",
            Self::Computer => "Computer",
        }
    }
    fn other(self) -> Self {
        match self {
            Self::You => Self::Computer,
            Self::Computer => Self::You,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Return,
    Point(Side),
    Finished(Side),
}

#[derive(Clone, Copy)]
struct Shot {
    from: Vec2,
    to: Vec2,
    progress: f32,
    duration: f32,
    height: f32,
    receiver: Side,
    aim: f32,
}
impl Shot {
    fn position(self, extra: f32) -> (Vec2, f32) {
        let t = (self.progress + extra / self.duration).clamp(0., 1.);
        // Two distinct arcs: the first clears the net, the second follows the
        // bounce on the receiving half. Height is in table-width units.
        let height = if t <= 0.68 {
            let u = t / 0.68;
            self.height * (1. - u) + 4. * 0.15 * u * (1. - u)
        } else {
            let u = (t - 0.68) / 0.32;
            4. * 0.07 * u * (1. - u)
        };
        (self.from.lerp(self.to, t), height)
    }
}

pub struct TableTennis {
    pub phase: Phase,
    pub difficulty: Difficulty,
    pub player: Vec2,
    pub computer: Vec2,
    pub score: [u16; 2],
    pub rally: u32,
    pub last_point: Option<Side>,
    pub revision: u64,
    shot: Option<Shot>,
    rng: u64,
    accumulated: f64,
    player_motion: f32,
    previous_player: Vec2,
}

impl TableTennis {
    pub fn new(seed: u64, difficulty: Difficulty) -> Self {
        Self {
            phase: Phase::Ready,
            difficulty,
            player: vec2(0.5, 0.86),
            computer: vec2(0.5, 0.14),
            score: [0, 0],
            rally: 0,
            last_point: None,
            revision: 0,
            shot: None,
            rng: seed.max(1),
            accumulated: 0.,
            player_motion: 0.,
            previous_player: vec2(0.5, 0.86),
        }
    }
    pub fn server(&self) -> Side {
        let total = u32::from(self.score[0]) + u32::from(self.score[1]);
        let turn = if self.score[0] >= 10 && self.score[1] >= 10 {
            total
        } else {
            total / 2
        };
        if turn.is_multiple_of(2) {
            Side::You
        } else {
            Side::Computer
        }
    }
    pub fn winner(&self) -> Option<Side> {
        if self.score[0].max(self.score[1]) >= 11 && self.score[0].abs_diff(self.score[1]) >= 2 {
            Some(if self.score[0] > self.score[1] {
                Side::You
            } else {
                Side::Computer
            })
        } else {
            None
        }
    }
    pub fn move_player(&mut self, point: Vec2) {
        if point.is_finite() && self.phase == Phase::Rally {
            // Direct input, with no easing or queued movement before contact.
            self.player = vec2(point.x.clamp(0.045, 0.955), point.y.clamp(0.56, 0.94));
        }
    }
    pub fn serve(&mut self) {
        if !matches!(self.phase, Phase::Ready | Phase::Point) {
            return;
        }
        self.phase = Phase::Rally;
        self.rally = 0;
        self.accumulated = 0.;
        self.player_motion = 0.;
        self.previous_player = self.player;
        let server = self.server();
        let from = if server == Side::You {
            self.player
        } else {
            self.computer
        };
        let aim = 0.25 + self.random() * 0.5;
        self.launch(from, server.other(), aim);
        self.revision += 1;
    }
    pub fn pause(&mut self) {
        if self.phase == Phase::Rally {
            self.phase = Phase::Paused;
            self.accumulated = 0.;
            self.revision += 1;
        }
    }
    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = Phase::Rally;
            self.previous_player = self.player;
            self.player_motion = 0.;
            self.revision += 1;
        }
    }
    pub fn ball(&self) -> (Vec2, f32) {
        self.shot.map_or((self.player, 0.035), |shot| {
            shot.position(if self.phase == Phase::Rally {
                self.accumulated as f32
            } else {
                0.
            })
        })
    }
    pub fn advance(&mut self, elapsed: f64) -> Option<Event> {
        if self.phase != Phase::Rally {
            return None;
        }
        if !elapsed.is_finite() || !(0.0..=0.25).contains(&elapsed) {
            self.pause();
            return None;
        }
        self.accumulated += elapsed;
        let mut event = None;
        while self.accumulated + 1e-10 >= STEP && self.phase == Phase::Rally {
            self.accumulated = (self.accumulated - STEP).max(0.);
            if let Some(next) = self.step(STEP as f32) {
                event = Some(next);
            }
        }
        event
    }
    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u32 << 24) as f32
    }
    fn launch(&mut self, from: Vec2, receiver: Side, x: f32) {
        let to = vec2(
            x.clamp(0.045, 0.955),
            if receiver == Side::You { 1.04 } else { -0.04 },
        );
        let accuracy = match self.difficulty {
            Difficulty::Easy => 0.10,
            Difficulty::Normal => 0.045,
            Difficulty::Hard => 0.016,
        };
        let intercept = from.lerp(to, (self.computer.y - from.y) / (to.y - from.y));
        let aim = (intercept.x + (self.random() - 0.5) * accuracy * 2.).clamp(0.045, 0.955);
        self.shot = Some(Shot {
            from,
            to,
            progress: 0.,
            receiver,
            aim,
            duration: (1.25 - self.rally.min(18) as f32 * 0.025).max(0.80),
            height: self.shot.map_or(RACKET_HEIGHT, |shot| shot.position(0.).1),
        });
    }
    fn step(&mut self, dt: f32) -> Option<Event> {
        self.player_motion = self.player_motion * 0.94
            + ((self.player.x - self.previous_player.x) / dt).clamp(-3., 3.) * 0.06;
        self.previous_player = self.player;
        let mut shot = self.shot?;
        shot.progress += dt / shot.duration;
        // The CPU reacts after a short delay and moves at a bounded speed. It
        // never teleports to the ball and can be beaten with angled returns.
        let target = if shot.receiver == Side::Computer && shot.progress > 0.12 {
            shot.aim
        } else {
            0.5
        };
        let speed = self.difficulty.speed() + self.rally.min(12) as f32 * 0.006;
        // Recover gradually after a stroke. Instantly recovering to the middle
        // on every outgoing shot would make even Normal cover both corners.
        let speed = speed
            * if shot.receiver == Side::Computer {
                1.
            } else {
                0.22
            };
        self.computer.x += (target - self.computer.x).clamp(-speed * dt, speed * dt);
        self.shot = Some(shot);
        let (ball, height) = shot.position(0.);
        let racket = if shot.receiver == Side::You {
            self.player
        } else {
            self.computer
        };
        // Circular paddles in physical table proportions (width / length .62).
        let distance = vec2(ball.x - racket.x, (ball.y - racket.y) / 0.62).length();
        if shot.progress >= 0.68 && height <= 0.08 && distance <= RACKET_RADIUS + 0.012 {
            self.rally += 1;
            let target = if shot.receiver == Side::You {
                let offset = ((ball.x - racket.x) / RACKET_RADIUS).clamp(-1., 1.);
                0.5 + offset * 0.43 + self.player_motion * 0.07
            } else {
                // Vary placement instead of always feeding the player's paddle.
                0.12 + self.random() * 0.76
            };
            self.launch(ball, shot.receiver.other(), target);
            self.revision += 1;
            return Some(Event::Return);
        }
        if shot.progress >= 1. {
            return Some(self.point(shot.receiver.other()));
        }
        None
    }
    fn point(&mut self, side: Side) -> Event {
        let index = usize::from(side == Side::Computer);
        self.score[index] = self.score[index].saturating_add(1);
        self.last_point = Some(side);
        self.shot = None;
        self.accumulated = 0.;
        self.revision += 1;
        if let Some(winner) = self.winner() {
            self.phase = Phase::Finished;
            Event::Finished(winner)
        } else {
            self.phase = Phase::Point;
            Event::Point(side)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_changes_every_two_points_and_every_point_at_deuce() {
        let mut game = TableTennis::new(1, Difficulty::Normal);
        for (score, server) in [
            ([0, 0], Side::You),
            ([1, 0], Side::You),
            ([1, 1], Side::Computer),
            ([3, 1], Side::You),
            ([10, 10], Side::You),
            ([11, 10], Side::Computer),
            ([11, 11], Side::You),
        ] {
            game.score = score;
            assert_eq!(game.server(), server);
        }
    }
    #[test]
    fn match_requires_eleven_and_a_two_point_lead() {
        let mut game = TableTennis::new(1, Difficulty::Normal);
        for score in [[10, 0], [11, 10], [12, 12]] {
            game.score = score;
            assert_eq!(game.winner(), None);
        }
        game.score = [11, 9];
        assert_eq!(game.winner(), Some(Side::You));
        assert_eq!(game.point(Side::You), Event::Finished(Side::You));
        assert_eq!(game.phase, Phase::Finished);
        game.serve();
        assert_eq!(game.phase, Phase::Finished);
    }
    #[test]
    fn shots_clear_net_and_bounce_once_on_receiving_half() {
        let mut game = TableTennis::new(77, Difficulty::Normal);
        for receiver in [Side::You, Side::Computer] {
            let from = vec2(0.5, if receiver == Side::You { 0.14 } else { 0.86 });
            game.launch(from, receiver, 0.7);
            let mut shot = game.shot.unwrap();
            shot.progress = (0.5 - from.y) / (shot.to.y - from.y);
            assert!(shot.position(0.).1 > 0.05);
            shot.progress = 0.68;
            let (bounce, height) = shot.position(0.);
            assert!(height.abs() < 0.00001);
            assert_eq!(bounce.y > 0.5, receiver == Side::You);
            shot.progress = 0.83;
            assert!(shot.position(0.).1 > 0.);
        }
    }
    #[test]
    fn legal_return_and_miss_award_only_one_point() {
        let mut game = TableTennis::new(12, Difficulty::Normal);
        game.phase = Phase::Rally;
        game.launch(vec2(0.5, 0.14), Side::You, 0.5);
        let mut returned = false;
        for _ in 0..270 {
            returned |= game.advance(STEP) == Some(Event::Return);
        }
        assert!(returned);
        assert_eq!(game.score, [0, 0]);
        assert_eq!(game.shot.unwrap().receiver, Side::Computer);
        game.rally = 0;
        game.player.x = 0.1;
        game.launch(vec2(0.8, 0.14), Side::You, 0.8);
        for _ in 0..400 {
            game.advance(STEP);
        }
        assert_eq!(game.score, [0, 1]);
        assert_eq!(game.phase, Phase::Point);
        assert_eq!(game.advance(0.1), None);
        assert_eq!(game.score, [0, 1]);
    }
    #[test]
    fn no_volley_before_bounce_and_no_side_wall_reflection() {
        let mut game = TableTennis::new(8, Difficulty::Normal);
        game.phase = Phase::Rally;
        game.launch(vec2(0.5, 0.14), Side::You, 0.95);
        game.shot.as_mut().unwrap().progress = 0.50;
        game.player = game.ball().0;
        assert_eq!(game.advance(STEP), None);
        assert_eq!(game.rally, 0);
        game.player = vec2(0.05, 0.86);
        for _ in 0..200 {
            game.advance(STEP);
        }
        assert_eq!(game.score, [0, 1]);
    }

    #[test]
    fn returns_preserve_ball_position_and_height_without_a_visual_jump() {
        let mut game = TableTennis::new(1, Difficulty::Normal);
        game.phase = Phase::Rally;
        game.launch(vec2(0.5, 0.14), Side::You, 0.5);
        for _ in 0..300 {
            let (before, height) = game.ball();
            if game.advance(STEP) == Some(Event::Return) {
                let (after, next_height) = game.ball();
                assert!(before.distance(after) < 0.005);
                assert!((height - next_height).abs() < 0.002);
                return;
            }
        }
        panic!("Return did not occur");
    }
    #[test]
    fn physics_and_cpu_do_not_depend_on_display_refresh_rate() {
        let mut samples = Vec::new();
        for fps in [30, 60, 90, 120, 144, 240] {
            let mut game = TableTennis::new(123, Difficulty::Normal);
            game.serve();
            for _ in 0..fps {
                game.advance(1. / f64::from(fps));
            }
            samples.push((game.ball().0, game.computer, game.rally, game.score));
        }
        for sample in &samples[1..] {
            assert!(sample.0.distance(samples[0].0) < 0.00001);
            assert!(sample.1.distance(samples[0].1) < 0.00001);
            assert_eq!(sample.2, samples[0].2);
            assert_eq!(sample.3, samples[0].3);
        }
    }
    #[test]
    fn pointer_is_direct_clamped_and_rejects_invalid_coordinates() {
        let mut game = TableTennis::new(1, Difficulty::Normal);
        game.serve();
        game.move_player(vec2(0.75, 0.81));
        assert_eq!(game.player, vec2(0.75, 0.81));
        game.move_player(vec2(-10., 100.));
        assert_eq!(game.player, vec2(0.045, 0.94));
        game.move_player(vec2(f32::NAN, 0.8));
        assert_eq!(game.player, vec2(0.045, 0.94));
    }
    #[test]
    fn interruptions_pause_without_scoring_or_unseen_catchup() {
        let mut game = TableTennis::new(55, Difficulty::Normal);
        game.serve();
        game.advance(0.1);
        let ball = game.ball().0;
        for elapsed in [30., f64::NAN, -1.] {
            game.advance(elapsed);
            assert_eq!(game.phase, Phase::Paused);
            assert_eq!(game.score, [0, 0]);
            game.resume();
        }
        assert!(game.ball().0.distance(ball) < 0.004);
        assert_eq!(game.advance(STEP), None);
        assert_eq!(game.phase, Phase::Rally);
    }
    #[test]
    fn cpu_speed_is_bounded_and_difficulty_changes_reach() {
        let mut positions = Vec::new();
        for level in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
            let mut game = TableTennis::new(123, level);
            game.phase = Phase::Rally;
            game.launch(vec2(0.95, 0.86), Side::Computer, 0.95);
            let before = game.computer;
            game.advance(0.15);
            assert!(game.computer.distance(before) <= level.speed() * 0.15 + 0.001);
            game.advance(0.15);
            positions.push(game.computer.x);
        }
        assert!(positions[0] < positions[1] && positions[1] < positions[2]);
    }

    #[test]
    fn complete_matches_finish_and_angled_returns_can_beat_every_cpu_level() {
        for level in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
            let mut game = TableTennis::new(881, level);
            for _ in 0..240 * 360 {
                if matches!(game.phase, Phase::Ready | Phase::Point) {
                    game.serve();
                }
                if game.shot.is_some_and(|s| s.receiver == Side::You) {
                    let ball = game.ball().0;
                    let offset = if game.computer.x > 0.5 { -0.075 } else { 0.075 };
                    game.move_player(vec2(ball.x - offset, 0.84));
                }
                game.advance(STEP);
                if game.phase == Phase::Finished {
                    break;
                }
            }
            assert_eq!(
                game.phase,
                Phase::Finished,
                "{level:?} stalled: score {:?}, rally {}, cpu {:?}, player {:?}",
                game.score,
                game.rally,
                game.computer,
                game.player
            );
            assert_eq!(game.winner(), Some(Side::You), "{level:?} cannot be beaten");
            assert!(game.score[0] >= 11 && game.score[0].abs_diff(game.score[1]) >= 2);
        }
    }
}
