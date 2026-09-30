//! Fih's mini-games use normalized positions and elapsed seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Catch,
    Pop,
    Memory,
    Hop,
    Swim,
    Rally,
    Dodge,
    Beats,
}
impl Kind {
    pub const ALL: [Self; 8] = [
        Self::Catch,
        Self::Pop,
        Self::Memory,
        Self::Hop,
        Self::Swim,
        Self::Rally,
        Self::Dodge,
        Self::Beats,
    ];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&k| k == self).unwrap()
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Catch => "Pearl Catch",
            Self::Pop => "Bubble Pop",
            Self::Memory => "Memory Reef",
            Self::Hop => "Reef Hop",
            Self::Swim => "Reef Dash",
            Self::Rally => "Shell Breaker",
            Self::Dodge => "Pearl Slalom",
            Self::Beats => "Tide Beats",
        }
    }
    pub fn instruction(self) -> &'static str {
        match self {
            Self::Catch => "Drag to catch pearls. Avoid the urchins!",
            Self::Pop => "Pearls only · Arrows aim · Space pops",
            Self::Memory => "Find all six matching pairs.",
            Self::Hop => "Jump left or right before the next stone sinks!",
            Self::Swim => "Tap or press Space to swim up.",
            Self::Rally => "Drag the paddle · Break all shells",
            Self::Dodge => "Drag to dodge · Collect pearls",
            Self::Beats => "Tap on the line · A / S / D",
        }
    }
    pub fn duration(self) -> f64 {
        match self {
            Self::Catch | Self::Hop => 30.,
            Self::Pop => 20.,
            Self::Memory => 60.,
            Self::Swim | Self::Rally | Self::Dodge | Self::Beats => 45.,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Gate {
    pub x: f32,
    pub gap: f32,
    pub hit: bool,
    pub passed: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Drop {
    pub x: f32,
    pub y: f32,
    pub danger: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct PopBubble {
    pub x: f32,
    pub y: f32,
    pub age: f32,
    pub danger: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Effect {
    pub x: f32,
    pub y: f32,
    pub age: f32,
    pub good: bool,
}
pub struct Round {
    pub kind: Kind,
    pub elapsed: f64,
    pub score: u32,
    pub lives: u8,
    pub player: f32,
    pub previous_player: f32,
    pub drops: Vec<Drop>,
    pub target: (f32, f32),
    pub paused: bool,
    pub finished: bool,
    pub cards: [u8; 12],
    pub shown: [bool; 12],
    pub matched: [bool; 12],
    pub lanes: [u8; 6],
    pub gates: Vec<Gate>,
    pub extras: crate::fih_extras::Extras,
    pub memory_stage: u32,
    pub swim_y: f32,
    pub bubbles: Vec<PopBubble>,
    pub effects: Vec<Effect>,
    pub combo: u32,
    pub aim: (f32, f32),
    pub hop_deadline: f64,
    pub flipped_at: [f64; 12],
    pop_at: f64,
    invincible_until: f64,
    velocity: f32,
    first: Option<usize>,
    hide_at: Option<f64>,
    hop_at: f64,
    spawn: f64,
    rng: u64,
    claimed: bool,
}
impl Round {
    pub fn new(kind: Kind, seed: u64) -> Self {
        let mut r = Self {
            kind,
            elapsed: 0.,
            score: 0,
            lives: 3,
            player: 0.5,
            previous_player: 0.5,
            drops: Vec::new(),
            target: (0.5, 0.5),
            paused: false,
            finished: false,
            cards: [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5],
            shown: [false; 12],
            matched: [false; 12],
            lanes: [0; 6],
            gates: Vec::new(),
            extras: crate::fih_extras::Extras::new(seed.wrapping_add(1)),
            memory_stage: 1,
            swim_y: 0.5,
            bubbles: Vec::new(),
            effects: Vec::new(),
            combo: 0,
            aim: (0.5, 0.5),
            hop_deadline: 2.4,
            flipped_at: [-10.; 12],
            pop_at: 0.,
            invincible_until: 0.,
            velocity: 0.,
            first: None,
            hide_at: None,
            hop_at: 0.,
            spawn: 0.,
            rng: seed.max(1),
            claimed: false,
        };
        r.new_target();
        if kind == Kind::Pop {
            r.bubbles.push(PopBubble {
                x: r.target.0,
                y: r.target.1,
                age: 0.,
                danger: false,
            });
            for _ in 0..3 {
                r.spawn_bubble();
            }
            r.spawn = 0.7;
        }
        for i in (1..12).rev() {
            let j = (r.random() * (i + 1) as f32) as usize;
            r.cards.swap(i, j);
        }
        for i in 0..6 {
            r.lanes[i] = u8::from(r.random() >= 0.5);
        }
        if kind == Kind::Memory {
            r.shown.fill(true);
            r.hide_at = Some(1.8);
        }
        r
    }
    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % 10000) as f32 / 10000.
    }
    fn new_target(&mut self) {
        self.target = (0.12 + self.random() * 0.76, 0.14 + self.random() * 0.62);
    }
    fn spawn_bubble(&mut self) {
        let x = 0.12 + self.random() * 0.76;
        let y = 0.35 + self.random() * 0.5;
        let danger = self.random() < (0.14 + self.elapsed as f32 * 0.01).min(0.34);
        self.bubbles.push(PopBubble {
            x,
            y,
            age: 0.,
            danger,
        });
    }
    pub fn hop_time(&self) -> f32 {
        ((self.hop_deadline - self.elapsed) / (2.4 - self.score as f64 * 0.025).max(0.85))
            .clamp(0., 1.) as f32
    }
    pub fn protected(&self) -> bool {
        self.elapsed < self.invincible_until
    }
    fn effect(&mut self, x: f32, y: f32, good: bool) {
        self.effects.push(Effect {
            x,
            y,
            age: 0.,
            good,
        });
    }
    pub fn level(&self) -> u32 {
        match self.kind {
            Kind::Memory => self.memory_stage,
            Kind::Rally => self.extras.wave,
            _ => 1 + (self.elapsed / 8.) as u32,
        }
    }
    pub fn beat(&mut self, lane: usize) -> bool {
        if self.kind != Kind::Beats || self.finished || self.paused {
            return false;
        }
        let (accepted, points, lost) = self.extras.beat(lane, self.elapsed);
        self.score = (self.score + points).min(100);
        self.lives = self.lives.saturating_sub(lost);
        if self.lives == 0 {
            self.finished = true;
        }
        accepted
    }
    pub fn remaining(&self) -> f64 {
        (self.kind.duration() - self.elapsed).max(0.)
    }
    pub fn steer(&mut self, x: f32) {
        if x.is_finite() {
            self.player = x.clamp(0.08, 0.92);
        }
    }
    pub fn pop(&mut self, x: f32, y: f32) -> bool {
        if self.kind != Kind::Pop
            || self.paused
            || self.finished
            || !x.is_finite()
            || !y.is_finite()
        {
            return false;
        }
        if self.elapsed < self.pop_at {
            return false;
        }
        self.pop_at = self.elapsed + 0.08;
        if let Some(index) = self
            .bubbles
            .iter()
            .position(|b| (x - b.x).hypot(y - b.y) <= 0.075)
        {
            let b = self.bubbles.remove(index);
            self.effect(b.x, b.y, !b.danger);
            if b.danger {
                self.lives = self.lives.saturating_sub(1);
                self.combo = 0;
            } else {
                self.combo += 1;
                self.score = (self.score + 1 + self.combo / 5).min(100);
            }
            if self.lives == 0 {
                self.finished = true;
            }
            self.target = self.bubbles.first().map_or((0.5, 0.5), |b| (b.x, b.y));
            true
        } else {
            self.combo = 0;
            false
        }
    }
    pub fn flip(&mut self, index: usize) -> bool {
        if self.kind != Kind::Memory
            || self.paused
            || self.finished
            || index >= 12
            || self.hide_at.is_some()
            || self.shown[index]
            || self.matched[index]
        {
            return false;
        }
        self.shown[index] = true;
        self.flipped_at[index] = self.elapsed;
        if let Some(first) = self.first.take() {
            if self.cards[first] == self.cards[index] {
                self.matched[first] = true;
                self.matched[index] = true;
                self.score += 2;
                self.combo += 1;
                if self.matched.iter().all(|&m| m) {
                    if self.memory_stage >= 3 {
                        self.finished = true;
                    } else {
                        self.memory_stage += 1;
                        self.shown.fill(true);
                        self.matched.fill(false);
                        for i in (1..12).rev() {
                            let j = (self.random() * (i + 1) as f32) as usize;
                            self.cards.swap(i, j);
                        }
                        self.hide_at =
                            Some(self.elapsed + if self.memory_stage == 2 { 1.1 } else { 0.55 });
                    }
                }
            } else {
                self.combo = 0;
                self.hide_at = Some(self.elapsed + (0.85 - self.memory_stage as f64 * 0.15));
            }
        } else {
            self.first = Some(index);
        }
        true
    }
    pub fn hop(&mut self, side: u8) -> bool {
        if self.kind != Kind::Hop
            || self.paused
            || self.finished
            || side > 1
            || self.elapsed < self.hop_at
        {
            return false;
        }
        self.previous_player = self.player;
        self.player = if side == 0 { 0.27 } else { 0.73 };
        if side == self.lanes[0] {
            self.combo += 1;
            self.score = (self.score + 1 + self.combo / 8).min(100);
        } else {
            self.combo = 0;
            self.lives = self.lives.saturating_sub(1);
        }
        self.lanes.rotate_left(1);
        self.lanes[5] = u8::from(self.random() >= 0.5);
        self.hop_at = self.elapsed + 0.32;
        self.hop_deadline = self.elapsed + (2.4 - self.score as f64 * 0.025).max(0.85);
        if self.lives == 0 {
            self.finished = true;
        }
        true
    }
    pub fn hop_phase(&self) -> f32 {
        ((self.elapsed - self.hop_at + 0.32) / 0.32).clamp(0., 1.) as f32
    }
    pub fn flap(&mut self) -> bool {
        if self.kind != Kind::Swim || self.paused || self.finished {
            return false;
        }
        self.velocity = -0.38;
        true
    }
    pub fn advance(&mut self, seconds: f64) -> u32 {
        if self.paused || self.finished || !seconds.is_finite() || seconds <= 0. {
            return 0;
        }
        if seconds > 0.75 {
            self.paused = true;
            return 0;
        }
        // Integrate small steps: catch collision and spawning do not depend on FPS.
        let mut left = seconds.min(self.remaining());
        let old = self.score;
        while left > 0.0000001 {
            let dt = left.min(1. / 240.);
            left -= dt;
            self.elapsed += dt;
            if matches!(self.kind, Kind::Rally | Kind::Dodge | Kind::Beats) {
                let (points, lost) = self
                    .extras
                    .advance(self.kind, dt, self.elapsed, self.player);
                self.score = (self.score + points).min(100);
                self.lives = self.lives.saturating_sub(lost);
                if self.lives == 0 {
                    self.finished = true;
                    break;
                }
            }
            for effect in &mut self.effects {
                effect.age += dt as f32;
            }
            self.effects.retain(|e| e.age < 0.65);
            if self.kind == Kind::Pop {
                self.spawn -= dt;
                if self.spawn <= 0. {
                    if self.bubbles.len() < 9 {
                        self.spawn_bubble();
                    }
                    self.spawn += (0.75 - self.elapsed * 0.012).max(0.35);
                }
                for bubble in &mut self.bubbles {
                    bubble.age += dt as f32;
                    bubble.y -= dt as f32 * 0.028;
                }
                if self.bubbles.iter().any(|b| b.age > 3.5 && !b.danger) {
                    self.combo = 0;
                }
                self.bubbles.retain(|b| b.age <= 3.5);
                self.target = self.bubbles.first().map_or((0.5, 0.5), |b| (b.x, b.y));
            }
            if self.kind == Kind::Hop && self.elapsed >= self.hop_deadline {
                self.lives = self.lives.saturating_sub(1);
                self.combo = 0;
                self.hop_deadline = self.elapsed + (2.4 - self.score as f64 * 0.025).max(0.85);
                self.lanes.rotate_left(1);
                self.lanes[5] = u8::from(self.random() >= 0.5);
                if self.lives == 0 {
                    self.finished = true;
                    break;
                }
            }
            if self.kind == Kind::Memory
                && self.hide_at.is_some_and(|t| self.elapsed + 0.000001 >= t)
            {
                for i in 0..12 {
                    if self.shown[i] && !self.matched[i] {
                        self.flipped_at[i] = self.elapsed;
                    }
                    self.shown[i] = self.matched[i];
                }
                self.hide_at = None;
            }
            if self.kind == Kind::Swim {
                self.velocity += dt as f32 * 0.65;
                self.swim_y += self.velocity * dt as f32;
                if !(0.04..=0.96).contains(&self.swim_y) {
                    self.lives = self.lives.saturating_sub(1);
                    self.swim_y = 0.5;
                    self.velocity = 0.;
                    self.invincible_until = self.elapsed + 0.8;
                }
                self.spawn -= dt;
                if self.spawn <= 0. {
                    let gap = 0.25 + self.random() * 0.5;
                    self.gates.push(Gate {
                        x: 1.1,
                        gap,
                        hit: false,
                        passed: false,
                    });
                    self.spawn += (2. - self.elapsed * 0.018).max(1.2);
                }
                for g in &mut self.gates {
                    g.x -= dt as f32 * (0.23 + self.elapsed as f32 * 0.0015);
                    if !g.hit
                        && self.elapsed >= self.invincible_until
                        && (g.x - 0.28).abs() < 0.075
                        && (self.swim_y - g.gap).abs()
                            > (0.20 - self.elapsed as f32 * 0.001).max(0.145)
                    {
                        g.hit = true;
                        self.lives = self.lives.saturating_sub(1);
                        self.invincible_until = self.elapsed + 0.8;
                    }
                    if !g.passed && g.x < 0.2 {
                        g.passed = true;
                        if !g.hit {
                            self.score = (self.score + 1).min(100);
                            self.effects.push(Effect {
                                x: 0.28,
                                y: self.swim_y,
                                age: 0.,
                                good: true,
                            });
                        }
                    }
                }
                self.gates.retain(|g| g.x > -0.12);
                if self.lives == 0 {
                    self.finished = true;
                    break;
                }
            }
            if self.kind == Kind::Catch {
                self.spawn -= dt;
                if self.spawn <= 0. {
                    let x = 0.08 + self.random() * 0.84;
                    let danger = self.random() < (0.20 + self.elapsed as f32 * 0.004).min(0.40);
                    self.drops.push(Drop {
                        x,
                        y: -0.04,
                        danger,
                    });
                    self.spawn += (0.65 - self.elapsed * 0.006).max(0.38);
                }
                let speed = 0.31 + self.elapsed as f32 * 0.002;
                for d in &mut self.drops {
                    d.y += speed * dt as f32;
                }
                let mut i = 0;
                while i < self.drops.len() {
                    let d = self.drops[i];
                    if d.y >= 0.83 && d.y <= 0.94 && (d.x - self.player).abs() < 0.11 {
                        if d.danger {
                            self.lives = self.lives.saturating_sub(1);
                            self.combo = 0;
                        } else {
                            self.combo += 1;
                            self.score = (self.score + 1 + self.combo / 5).min(100);
                        }
                        self.effect(d.x, d.y, !d.danger);
                        self.drops.remove(i);
                    } else if d.y > 1.08 {
                        if !d.danger {
                            self.combo = 0;
                        }
                        self.drops.remove(i);
                    } else {
                        i += 1;
                    }
                }
                if self.lives == 0 {
                    self.finished = true;
                    break;
                }
            }
        }
        if self.remaining() < 0.000001 {
            self.finished = true;
        }
        self.score - old
    }
    /// Results can be collected once, only after a completed round.
    pub fn claim(&mut self) -> Option<(usize, u32)> {
        if !self.finished || self.claimed {
            return None;
        }
        self.claimed = true;
        Some((self.kind.index(), self.score))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catch_is_frame_rate_independent() {
        let mut results = Vec::new();
        for fps in [15, 30, 60, 120, 240] {
            let mut r = Round::new(Kind::Catch, 42);
            for _ in 0..fps * 30 {
                r.advance(1. / f64::from(fps));
            }
            results.push((r.score, r.lives, r.finished));
        }
        assert!(results.windows(2).all(|r| r[0] == r[1]));
    }
    #[test]
    fn additional_games_have_refresh_independent_rules_and_single_rewards() {
        for kind in [Kind::Rally, Kind::Dodge, Kind::Beats] {
            let mut outcomes = Vec::new();
            for fps in [30, 60, 120, 240] {
                let mut round = Round::new(kind, 42);
                round.paused = true;
                round.advance(0.5);
                assert_eq!(round.elapsed, 0.);
                assert!(!round.beat(0));
                assert_eq!(round.claim(), None);
                round.paused = false;
                for _ in 0..fps * 45 {
                    round.advance(1. / f64::from(fps));
                }
                assert!(round.finished);
                outcomes.push((round.score, round.lives));
                assert_eq!(round.claim(), Some((kind.index(), round.score)));
                assert_eq!(round.claim(), None);
            }
            assert!(
                outcomes.windows(2).all(|pair| pair[0] == pair[1]),
                "{}: {:?}",
                kind.title(),
                outcomes
            );
        }
    }
    #[test]
    fn target_taps_pause_and_single_reward_work() {
        let mut r = Round::new(Kind::Pop, 42);
        let (x, y) = r.target;
        assert!(!r.pop(0., 0.));
        r.advance(0.1);
        assert!(r.pop(x, y));
        assert_eq!(r.score, 1);
        r.paused = true;
        r.advance(0.5);
        assert!((r.elapsed - 0.1).abs() < 1e-6);
        assert!(!r.pop(r.target.0, r.target.1));
        r.paused = false;
        for _ in 0..1200 {
            r.advance(1. / 60.);
        }
        assert!(r.finished);
        assert_eq!(r.claim(), Some((1, 1)));
        assert_eq!(r.claim(), None);
        assert!(!r.pop(r.target.0, r.target.1));
    }
    #[test]
    fn background_stalls_pause_without_unseen_progress() {
        let mut r = Round::new(Kind::Catch, 1);
        r.advance(30.);
        assert!(r.paused);
        assert_eq!(r.elapsed, 0.);
        assert_eq!(r.claim(), None);
        r.steer(100.);
        assert_eq!(r.player, 0.92);
    }
    #[test]
    fn memory_matches_and_mismatches_are_safe_and_complete_once() {
        let mut r = Round::new(Kind::Memory, 42);
        assert!(!r.flip(0));
        for _ in 0..120 {
            r.advance(1. / 60.);
        }
        let a = 0;
        let b = (1..12).find(|&i| r.cards[i] != r.cards[a]).unwrap();
        assert!(r.flip(a));
        assert!(!r.flip(a));
        assert!(r.flip(b));
        assert!(!r.flip(11));
        r.advance(0.7);
        assert!(!r.shown[a]);
        assert!(!r.shown[b]);
        for stage in 1..=3 {
            for value in 0..6 {
                let pair: Vec<_> = (0..12).filter(|&i| r.cards[i] == value).collect();
                assert!(r.flip(pair[0]));
                assert!(r.flip(pair[1]));
            }
            if stage < 3 {
                assert!(!r.finished);
                assert_eq!(r.memory_stage, stage + 1);
                for _ in 0..90 {
                    r.advance(1. / 60.);
                }
            }
        }
        assert!(r.finished);
        assert_eq!(r.score, 36);
        assert_eq!(r.claim(), Some((2, 36)));
        assert_eq!(r.claim(), None);
    }
    #[test]
    fn hops_require_safe_stones_and_respect_cooldown() {
        let mut r = Round::new(Kind::Hop, 9);
        assert!(r.hop(r.lanes[0]));
        assert_eq!(r.score, 1);
        assert!(!r.hop(r.lanes[0]));
        for _ in 0..3 {
            r.advance(0.35);
            assert!(r.hop(1 - r.lanes[0]));
        }
        assert!(r.finished);
        assert_eq!(r.lives, 0);
        assert!(!r.hop(0));
    }
    #[test]
    fn dash_collisions_charge_one_life_per_gate_and_pause_blocks_flaps() {
        let mut r = Round::new(Kind::Swim, 9);
        r.gates.push(Gate {
            x: 0.28,
            gap: 0.1,
            hit: false,
            passed: false,
        });
        r.advance(0.1);
        assert_eq!(r.lives, 2);
        r.advance(0.1);
        assert_eq!(r.lives, 2);
        r.paused = true;
        assert!(!r.flap());
        let y = r.swim_y;
        r.advance(0.5);
        assert_eq!(r.swim_y, y);
        r.paused = false;
        assert!(r.flap());
        r.advance(0.1);
        assert!(r.swim_y < y);
    }
    #[test]
    fn dash_collisions_and_scores_match_across_display_refresh_rates() {
        let mut results = Vec::new();
        for fps in [15, 30, 60, 120, 240] {
            let mut r = Round::new(Kind::Swim, 42);
            for frame in 0..fps * 45 {
                if frame % fps == 0 {
                    r.flap();
                }
                r.advance(1. / f64::from(fps));
            }
            results.push((r.score, r.lives, r.finished));
        }
        assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
    }
    #[test]
    fn bubbles_require_aiming_expire_and_hazards_break_streaks() {
        let mut r = Round::new(Kind::Pop, 42);
        r.bubbles = vec![PopBubble {
            x: 0.3,
            y: 0.4,
            age: 0.,
            danger: false,
        }];
        assert!(!r.pop(0.5, 0.5));
        assert_eq!(r.score, 0);
        r.advance(0.1);
        let b = r.bubbles[0];
        assert!(r.pop(b.x, b.y));
        assert_eq!(r.combo, 1);
        r.bubbles = vec![PopBubble {
            x: 0.3,
            y: 0.4,
            age: 0.,
            danger: true,
        }];
        r.advance(0.1);
        let b = r.bubbles[0];
        assert!(r.pop(b.x, b.y));
        assert_eq!(r.lives, 2);
        assert_eq!(r.combo, 0);
        r.bubbles = vec![PopBubble {
            x: 0.3,
            y: 0.4,
            age: 3.49,
            danger: false,
        }];
        r.combo = 5;
        r.advance(0.02);
        assert_eq!(r.combo, 0);
        assert!(!r.bubbles.iter().any(|b| b.age > 3.5));
        r.paused = true;
        let n = r.bubbles.len();
        r.advance(0.7);
        assert_eq!(r.bubbles.len(), n);
    }
    #[test]
    fn hopping_requires_a_decision_before_each_stone_sinks_at_all_refresh_rates() {
        for fps in [15, 30, 60, 120, 240] {
            let mut r = Round::new(Kind::Hop, 42);
            for _ in 0..fps * 8 {
                r.advance(1. / fps as f64);
            }
            assert!(r.finished);
            assert_eq!(r.lives, 0);
            assert_eq!(r.score, 0);
            assert!(r.claim().is_some());
            assert!(r.claim().is_none());
        }
    }
    #[test]
    fn catching_consecutive_pearls_builds_a_streak_and_hazards_reset_it() {
        let mut r = Round::new(Kind::Catch, 7);
        for _ in 0..5 {
            r.drops = vec![Drop {
                x: 0.5,
                y: 0.85,
                danger: false,
            }];
            r.advance(1. / 60.);
        }
        assert_eq!(r.combo, 5);
        assert_eq!(r.score, 6);
        assert!(!r.effects.is_empty());
        r.drops = vec![Drop {
            x: 0.5,
            y: 0.85,
            danger: true,
        }];
        r.advance(1. / 60.);
        assert_eq!(r.combo, 0);
        assert_eq!(r.lives, 2);
    }
    #[test]
    fn hazards_remove_lives_and_end_round() {
        let mut r = Round::new(Kind::Catch, 1);
        r.drops = vec![
            Drop {
                x: 0.5,
                y: 0.85,
                danger: true
            };
            3
        ];
        r.advance(1. / 60.);
        assert!(r.finished);
        assert_eq!(r.lives, 0);
        assert_eq!(r.claim(), Some((0, 0)));
    }
}
