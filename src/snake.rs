use std::collections::VecDeque;

pub const BOARD_SIZE: i16 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    pub fn delta(self) -> (i16, i16) {
        match self {
            Self::Up => (0, -1),
            Self::Right => (1, 0),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
        }
    }

    fn opposite(self, other: Self) -> bool {
        let (x, y) = self.delta();
        let (ox, oy) = other.delta();
        x == -ox && y == -oy
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ready,
    Running,
    Paused,
    Lost,
    Won,
}

/// Pure rules, independent of graphics, input devices, and wall-clock time.
pub struct Snake {
    body: VecDeque<Cell>,
    direction: Direction,
    turns: VecDeque<Direction>,
    food: Option<Cell>,
    status: Status,
    score: u32,
    rng: u64,
}

impl Snake {
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            body: VecDeque::from([
                Cell { x: 9, y: 10 },
                Cell { x: 8, y: 10 },
                Cell { x: 7, y: 10 },
            ]),
            direction: Direction::Right,
            turns: VecDeque::with_capacity(2),
            food: None,
            status: Status::Ready,
            score: 0,
            rng: seed.max(1),
        };
        game.place_food();
        game
    }

    pub fn body(&self) -> &VecDeque<Cell> {
        &self.body
    }
    pub fn food(&self) -> Option<Cell> {
        self.food
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn score(&self) -> u32 {
        self.score
    }
    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn start_or_resume(&mut self) {
        if matches!(self.status, Status::Ready | Status::Paused) {
            self.status = Status::Running;
        }
    }

    pub fn pause(&mut self) {
        if self.status == Status::Running {
            self.status = Status::Paused;
            self.turns.clear();
        }
    }

    /// Buffer two turns to preserve quick corner inputs, checking each against
    /// the previous queued direction rather than allowing a same-tick reversal.
    pub fn turn(&mut self, direction: Direction) -> bool {
        if !matches!(self.status, Status::Ready | Status::Running) || self.turns.len() == 2 {
            return false;
        }
        let previous = self.turns.back().copied().unwrap_or(self.direction);
        if direction != previous && !direction.opposite(previous) {
            self.turns.push_back(direction);
            return true;
        }
        false
    }

    pub fn tick(&mut self) {
        if self.status != Status::Running {
            return;
        }
        if let Some(direction) = self.turns.pop_front() {
            self.direction = direction;
        }
        let head = self.body[0];
        let (dx, dy) = self.direction.delta();
        let next = Cell {
            x: head.x + dx,
            y: head.y + dy,
        };
        let growing = self.food == Some(next);
        // The tail vacates its cell on a non-growing step.
        let occupied = self.body.len() - usize::from(!growing);
        if next.x < 0
            || next.y < 0
            || next.x >= BOARD_SIZE
            || next.y >= BOARD_SIZE
            || self.body.iter().take(occupied).any(|&cell| cell == next)
        {
            self.status = Status::Lost;
            self.turns.clear();
            return;
        }
        self.body.push_front(next);
        if growing {
            self.score += 1;
            self.place_food();
        } else {
            self.body.pop_back();
        }
    }

    fn place_food(&mut self) {
        let free = (BOARD_SIZE * BOARD_SIZE) as usize - self.body.len();
        if free == 0 {
            self.food = None;
            self.status = Status::Won;
            return;
        }
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let mut index = (self.rng % free as u64) as usize;
        // Bounded scan, including when only one free cell remains.
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                let cell = Cell { x, y };
                if !self.body.contains(&cell) {
                    if index == 0 {
                        self.food = Some(cell);
                        return;
                    }
                    index -= 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running() -> Snake {
        let mut game = Snake::new(42);
        game.food = Some(Cell { x: 0, y: 0 });
        game.start_or_resume();
        game
    }

    #[test]
    fn initial_state_and_movement() {
        let mut game = Snake::new(0);
        assert_eq!(game.status(), Status::Ready);
        game.tick();
        assert_eq!(game.body()[0], Cell { x: 9, y: 10 });
        game.start_or_resume();
        game.tick();
        assert_eq!(game.body()[0], Cell { x: 10, y: 10 });
        assert_eq!(game.body().len(), 3);
    }

    #[test]
    fn eating_grows_and_scores_once() {
        let mut game = running();
        game.food = Some(Cell { x: 10, y: 10 });
        game.tick();
        assert_eq!(game.body().len(), 4);
        assert_eq!(game.score(), 1);
        assert!(!game.body.contains(&game.food.unwrap()));
    }

    #[test]
    fn reversals_are_rejected_and_fast_turns_are_buffered() {
        let mut game = running();
        game.turn(Direction::Left);
        game.turn(Direction::Up);
        game.turn(Direction::Down);
        game.turn(Direction::Left);
        game.turn(Direction::Down); // Third valid turn exceeds buffer.
        game.tick();
        assert_eq!(game.body()[0], Cell { x: 9, y: 9 });
        game.tick();
        assert_eq!(game.body()[0], Cell { x: 8, y: 9 });
        game.tick();
        assert_eq!(game.body()[0], Cell { x: 7, y: 9 });
    }

    #[test]
    fn wall_collision_freezes_game() {
        let mut game = running();
        for _ in 0..11 {
            game.tick();
        }
        assert_eq!(game.status(), Status::Lost);
        let body = game.body.clone();
        game.tick();
        game.start_or_resume();
        assert_eq!(game.body, body);
        assert_eq!(game.status(), Status::Lost);
    }

    #[test]
    fn self_collision_loses() {
        let mut game = running();
        game.body = VecDeque::from([
            Cell { x: 2, y: 2 },
            Cell { x: 2, y: 3 },
            Cell { x: 3, y: 3 },
            Cell { x: 3, y: 2 },
            Cell { x: 4, y: 2 },
        ]);
        game.tick();
        assert_eq!(game.status(), Status::Lost);
    }

    #[test]
    fn moving_into_departing_tail_is_legal() {
        let mut game = running();
        game.body = VecDeque::from([
            Cell { x: 2, y: 2 },
            Cell { x: 2, y: 3 },
            Cell { x: 3, y: 3 },
            Cell { x: 3, y: 2 },
        ]);
        game.tick();
        assert_eq!(game.status(), Status::Running);
        assert_eq!(game.body()[0], Cell { x: 3, y: 2 });
    }

    #[test]
    fn pause_freezes_and_clears_pending_turns() {
        let mut game = running();
        game.turn(Direction::Up);
        game.pause();
        let body = game.body.clone();
        game.turn(Direction::Down);
        game.tick();
        assert_eq!(game.body, body);
        game.start_or_resume();
        game.tick();
        assert_eq!(game.body()[0], Cell { x: 10, y: 10 });
    }

    #[test]
    fn food_never_overlaps_snake_and_seeds_are_repeatable() {
        for seed in 0..500 {
            let game = Snake::new(seed);
            let food = game.food.unwrap();
            assert!(!game.body.contains(&food));
            assert!((0..BOARD_SIZE).contains(&food.x));
            assert!((0..BOARD_SIZE).contains(&food.y));
            assert_eq!(game.food, Snake::new(seed).food);
        }
    }

    #[test]
    fn filling_board_wins_without_food_search_loop() {
        let mut game = running();
        game.body = VecDeque::from([Cell { x: 0, y: 0 }]);
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                if y != 0 || x > 1 {
                    game.body.push_back(Cell { x, y });
                }
            }
        }
        game.place_food();
        assert_eq!(game.food, Some(Cell { x: 1, y: 0 }));
        game.tick();
        assert_eq!(game.status(), Status::Won);
        assert_eq!(game.body.len(), 400);
        assert_eq!(game.food, None);
    }
}
