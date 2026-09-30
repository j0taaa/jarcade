#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoardSize {
    #[default]
    Small,
    Medium,
    Large,
}
impl BoardSize {
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];
    pub fn dimensions(self) -> (usize, usize, usize) {
        match self {
            Self::Small => (9, 9, 10),
            Self::Medium => (16, 16, 40),
            Self::Large => (30, 16, 99),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tile {
    pub mine: bool,
    pub adjacent: u8,
    pub revealed: bool,
    pub flagged: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ready,
    Playing,
    Won,
    Lost,
}

/// Rules only. Mines are placed after the first reveal, with a safe 3x3 opening.
pub struct Minesweeper {
    tiles: Vec<Tile>,
    size: BoardSize,
    status: Status,
    rng: u64,
    exploded: Option<usize>,
}

impl Minesweeper {
    pub fn new(seed: u64) -> Self {
        Self::with_size(seed, BoardSize::Small)
    }
    pub fn with_size(seed: u64, size: BoardSize) -> Self {
        let (width, height, _) = size.dimensions();
        Self {
            tiles: vec![Tile::default(); width * height],
            size,
            status: Status::Ready,
            rng: seed.max(1),
            exploded: None,
        }
    }
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
    pub fn width(&self) -> usize {
        self.size.dimensions().0
    }
    pub fn height(&self) -> usize {
        self.size.dimensions().1
    }
    pub fn mine_count(&self) -> usize {
        self.size.dimensions().2
    }
    pub fn safe_count(&self) -> usize {
        self.tiles.len() - self.mine_count()
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn exploded(&self) -> Option<usize> {
        self.exploded
    }
    pub fn flags(&self) -> usize {
        self.tiles.iter().filter(|t| t.flagged).count()
    }
    pub fn revealed(&self) -> usize {
        self.tiles.iter().filter(|t| t.revealed && !t.mine).count()
    }
    pub fn finished(&self) -> bool {
        matches!(self.status, Status::Won | Status::Lost)
    }

    fn neighbors(&self, index: usize) -> impl Iterator<Item = usize> + use<> {
        let (width, height) = (self.width(), self.height());
        let (x, y) = ((index % width) as i32, (index / width) as i32);
        (-1..=1).flat_map(move |dy| {
            (-1..=1).filter_map(move |dx| {
                let (nx, ny) = (x + dx, y + dy);
                ((dx != 0 || dy != 0)
                    && (0..width as i32).contains(&nx)
                    && (0..height as i32).contains(&ny))
                .then_some((ny * width as i32 + nx) as usize)
            })
        })
    }
    fn place(&mut self, first: usize) {
        let safe: Vec<_> = self
            .neighbors(first)
            .chain(std::iter::once(first))
            .collect();
        let mut candidates: Vec<_> = (0..self.tiles.len())
            .filter(|i| !safe.contains(i))
            .collect();
        for i in 0..self.mine_count() {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 7;
            self.rng ^= self.rng << 17;
            let j = i + (self.rng % (candidates.len() - i) as u64) as usize;
            candidates.swap(i, j);
            self.tiles[candidates[i]].mine = true;
        }
        for i in 0..self.tiles.len() {
            self.tiles[i].adjacent =
                self.neighbors(i).filter(|&j| self.tiles[j].mine).count() as u8;
        }
        self.status = Status::Playing;
    }
    pub fn toggle_flag(&mut self, index: usize) -> bool {
        if index >= self.tiles.len() || self.finished() || self.tiles[index].revealed {
            return false;
        }
        self.tiles[index].flagged = !self.tiles[index].flagged;
        true
    }
    pub fn reveal(&mut self, index: usize) -> bool {
        if index >= self.tiles.len() || self.finished() || self.tiles[index].flagged {
            return false;
        }
        if self.status == Status::Ready {
            self.place(index);
        }
        if self.tiles[index].revealed {
            // Clicking a number opens its neighbors when the flag count matches.
            let neighbors: Vec<_> = self.neighbors(index).collect();
            if self.tiles[index].adjacent == 0
                || neighbors.iter().filter(|&&i| self.tiles[i].flagged).count()
                    != self.tiles[index].adjacent as usize
            {
                return false;
            }
            let mut changed = false;
            for i in neighbors {
                if self.finished() {
                    break;
                }
                changed |= self.open(i);
            }
            self.check_win();
            changed
        } else {
            let changed = self.open(index);
            self.check_win();
            changed
        }
    }
    fn open(&mut self, index: usize) -> bool {
        if self.tiles[index].revealed || self.tiles[index].flagged {
            return false;
        }
        if self.tiles[index].mine {
            self.tiles[index].revealed = true;
            self.exploded = Some(index);
            self.status = Status::Lost;
            return true;
        }
        let mut pending = vec![index];
        while let Some(i) = pending.pop() {
            if self.tiles[i].revealed || self.tiles[i].flagged || self.tiles[i].mine {
                continue;
            }
            self.tiles[i].revealed = true;
            if self.tiles[i].adjacent == 0 {
                pending.extend(self.neighbors(i));
            }
        }
        true
    }
    fn check_win(&mut self) {
        if self.status == Status::Playing && self.revealed() == self.safe_count() {
            self.status = Status::Won;
            for tile in &mut self.tiles {
                if tile.mine {
                    tile.flagged = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SIZE: usize = 9;
    const CELLS: usize = 81;
    const MINES: usize = 10;
    #[test]
    fn every_first_cell_has_a_safe_opening_and_exact_mine_count() {
        for seed in 0..30 {
            for first in 0..CELLS {
                let mut game = Minesweeper::new(seed);
                assert!(game.reveal(first));
                assert_ne!(game.status(), Status::Lost);
                assert_eq!(game.tiles.iter().filter(|t| t.mine).count(), MINES);
                assert_eq!(game.tiles[first].adjacent, 0);
                for i in (0..CELLS).filter(|i| {
                    (i % SIZE).abs_diff(first % SIZE) <= 1 && (i / SIZE).abs_diff(first / SIZE) <= 1
                }) {
                    assert!(!game.tiles[i].mine);
                }
                for i in 0..CELLS {
                    assert_eq!(
                        game.tiles[i].adjacent as usize,
                        game.neighbors(i).filter(|&n| game.tiles[n].mine).count()
                    );
                }
            }
        }
    }
    #[test]
    fn flags_before_start_do_not_generate_board_and_block_reveal() {
        let mut game = Minesweeper::new(7);
        assert!(game.toggle_flag(40));
        assert!(!game.reveal(40));
        assert_eq!(game.status(), Status::Ready);
        assert_eq!(game.flags(), 1);
        game.toggle_flag(40);
        game.reveal(40);
        assert!(!game.toggle_flag(40));
        assert!(game.revealed() > 1);
    }
    #[test]
    fn flood_fill_respects_flags_and_edges_do_not_wrap() {
        let mut game = Minesweeper::new(1);
        game.toggle_flag(41);
        game.reveal(40);
        assert!(!game.tiles[41].revealed);
        assert_eq!(game.neighbors(0).collect::<Vec<_>>(), [1, 9, 10]);
        assert_eq!(game.neighbors(8).collect::<Vec<_>>(), [7, 16, 17]);
        game.toggle_flag(41);
        game.reveal(41);
        assert!(game.tiles[41].revealed);
    }
    #[test]
    fn revealing_all_safe_tiles_wins_without_requiring_flags() {
        let mut game = Minesweeper::new(90);
        game.reveal(40);
        for i in 0..CELLS {
            if !game.tiles[i].mine {
                game.reveal(i);
            }
        }
        assert_eq!(game.status(), Status::Won);
        assert_eq!(game.revealed(), CELLS - MINES);
        assert_eq!(game.flags(), MINES);
        assert!(!game.toggle_flag(0));
        assert!(!game.reveal(0));
    }
    #[test]
    fn mine_loses_and_freezes_input_and_invalid_indices_are_safe() {
        let mut game = Minesweeper::new(2);
        assert!(!game.reveal(CELLS));
        assert!(!game.toggle_flag(usize::MAX));
        game.reveal(40);
        let mine = game.tiles.iter().position(|t| t.mine).unwrap();
        game.reveal(mine);
        assert_eq!(game.status(), Status::Lost);
        assert_eq!(game.exploded(), Some(mine));
        let snapshot = game.tiles.clone();
        for i in 0..CELLS {
            game.reveal(i);
            game.toggle_flag(i);
        }
        assert_eq!(game.tiles, snapshot);
    }
    #[test]
    fn chording_opens_neighbors_and_wrong_flags_can_lose() {
        for correct in [true, false] {
            let mut game = Minesweeper::new(1);
            game.status = Status::Playing;
            game.tiles[0].mine = true;
            game.tiles[10].revealed = true;
            game.tiles[10].adjacent = 1;
            // Nonzero boundary prevents a flood from hiding chord behavior.
            for i in [1, 2, 9, 11, 18, 19, 20] {
                game.tiles[i].adjacent = 1;
            }
            assert!(!game.reveal(10));
            game.toggle_flag(if correct { 0 } else { 1 });
            assert!(game.reveal(10));
            if correct {
                assert!(game.tiles[20].revealed);
                assert_eq!(game.status(), Status::Playing);
            } else {
                assert_eq!(game.status(), Status::Lost);
            }
        }
    }
    #[test]
    fn seeds_repeat_across_platforms_and_restarts_are_clean() {
        let mut a = Minesweeper::new(123);
        let mut b = Minesweeper::new(123);
        a.reveal(40);
        b.reveal(40);
        assert_eq!(a.tiles, b.tiles);
        let fresh = Minesweeper::new(123);
        assert_eq!(fresh.revealed(), 0);
        assert_eq!(fresh.flags(), 0);
        assert_eq!(fresh.status(), Status::Ready);
    }
    #[test]
    fn all_sizes_generate_safe_openings_win_and_have_correct_edges() {
        for size in BoardSize::ALL {
            let (width, height, mines) = size.dimensions();
            for first in [
                0,
                width - 1,
                width * height - 1,
                width * (height / 2) + width / 2,
            ] {
                for seed in 0..20 {
                    let mut game = Minesweeper::with_size(seed, size);
                    game.reveal(first);
                    assert_ne!(game.status(), Status::Lost);
                    assert_eq!(game.tiles.len(), width * height);
                    assert_eq!(game.tiles.iter().filter(|t| t.mine).count(), mines);
                    assert_eq!(game.tiles[first].adjacent, 0);
                    for i in 0..game.tiles.len() {
                        let count = (0..game.tiles.len())
                            .filter(|&j| {
                                j != i
                                    && (j % width).abs_diff(i % width) <= 1
                                    && (j / width).abs_diff(i / width) <= 1
                                    && game.tiles[j].mine
                            })
                            .count();
                        assert_eq!(game.tiles[i].adjacent as usize, count);
                    }
                    for i in 0..game.tiles.len() {
                        if !game.tiles[i].mine {
                            game.reveal(i);
                        }
                    }
                    assert_eq!(game.status(), Status::Won);
                    assert_eq!(game.flags(), mines);
                }
            }
        }
    }
}
