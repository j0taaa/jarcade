use super::{Cage, Difficulty, Edge, Puzzle, Relation, Variant, bit, logical_solve};
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn index(&mut self, n: usize) -> usize {
        self.next() as usize % n
    }
    fn shuffle<T>(&mut self, s: &mut [T]) {
        for i in (1..s.len()).rev() {
            let j = self.index(i + 1);
            s.swap(i, j);
        }
    }
}
/// One clue-removal attempt per step keeps web and phone input responsive.
pub struct Generator {
    puzzle: Puzzle,
    order: Vec<usize>,
    at: usize,
    target: usize,
    done: bool,
}
impl Generator {
    pub fn new(seed: u64, variant: Variant, difficulty: Difficulty) -> Self {
        let mut rng = Rng(seed);
        let solution = solved_grid(&mut rng, variant == Variant::Diagonal);
        let mut puzzle = Puzzle {
            seed,
            variant,
            difficulty,
            givens: solution.clone(),
            solution,
            cages: Vec::new(),
            edges: Vec::new(),
            thermos: Vec::new(),
            effort: 0,
        };
        match variant {
            Variant::Killer => make_cages(&mut puzzle, &mut rng),
            Variant::Xv | Variant::Kropki => {
                for a in 0..81 {
                    for b in neighbours(a).into_iter().filter(|&b| b > a) {
                        let x = puzzle.solution[a];
                        let y = puzzle.solution[b];
                        let relation = if variant == Variant::Xv {
                            if x + y == 5 {
                                Some(Relation::Five)
                            } else if x + y == 10 {
                                Some(Relation::Ten)
                            } else {
                                None
                            }
                        } else if x == 2 * y || y == 2 * x {
                            Some(Relation::Double)
                        } else if x.abs_diff(y) == 1 {
                            Some(Relation::Consecutive)
                        } else {
                            None
                        };
                        if let Some(relation) = relation {
                            puzzle.edges.push(Edge { a, b, relation });
                        }
                    }
                }
            }
            Variant::Thermo => make_thermos(&mut puzzle, &mut rng),
            _ => {}
        }
        let mut order: Vec<_> = (0..81).collect();
        rng.shuffle(&mut order);
        let targets = match variant {
            Variant::Killer => [28, 16, 4],
            Variant::Kropki => [36, 26, 16],
            Variant::Xv => [42, 32, 24],
            Variant::Thermo => [40, 30, 22],
            Variant::Diagonal => [40, 30, 24],
            Variant::Classic => [44, 34, 26],
        };
        Self {
            puzzle,
            order,
            at: 0,
            target: targets[difficulty.level() as usize],
            done: false,
        }
    }
    pub fn progress(&self) -> f32 {
        self.at as f32 / 81.
    }
    pub fn step(&mut self) -> bool {
        if self.done {
            return true;
        }
        if self.at == self.order.len()
            || self.puzzle.givens.iter().filter(|&&v| v > 0).count() <= self.target
        {
            self.puzzle.effort = logical_solve(
                &self.puzzle,
                &self.puzzle.givens,
                self.puzzle.difficulty.level(),
            )
            .effort;
            self.done = true;
            return true;
        }
        let i = self.order[self.at];
        self.at += 1;
        let old = self.puzzle.givens[i];
        self.puzzle.givens[i] = 0;
        // Every accepted removal has a full logical proof of the unique solution.
        if !logical_solve(
            &self.puzzle,
            &self.puzzle.givens,
            self.puzzle.difficulty.level(),
        )
        .solved
        {
            self.puzzle.givens[i] = old;
        }
        false
    }
    pub fn finish(mut self) -> Puzzle {
        while !self.step() {}
        self.puzzle
    }
}
fn neighbours(i: usize) -> Vec<usize> {
    let mut n = Vec::with_capacity(4);
    if !i.is_multiple_of(9) {
        n.push(i - 1);
    }
    if i % 9 < 8 {
        n.push(i + 1);
    }
    if i >= 9 {
        n.push(i - 9);
    }
    if i < 72 {
        n.push(i + 9);
    }
    n
}
fn solved_grid(rng: &mut Rng, diagonal: bool) -> Vec<u8> {
    // Randomized backtracking rather than shuffling a single fixed puzzle.
    fn fill(grid: &mut [u8], rng: &mut Rng, diagonal: bool) -> bool {
        let mut best = None;
        let mut options = Vec::new();
        for i in 0..81 {
            if grid[i] != 0 {
                continue;
            }
            let mut used = 0;
            for (j, &value) in grid.iter().enumerate() {
                if i / 9 == j / 9
                    || i % 9 == j % 9
                    || i / 27 == j / 27 && i % 9 / 3 == j % 9 / 3
                    || diagonal
                        && (i / 9 == i % 9 && j / 9 == j % 9
                            || i / 9 + i % 9 == 8 && j / 9 + j % 9 == 8)
                {
                    used |= bit(value);
                }
            }
            let candidates: Vec<_> = (1..=9).filter(|&d| used & bit(d) == 0).collect();
            if candidates.is_empty() {
                return false;
            }
            if best.is_none() || candidates.len() < options.len() {
                best = Some(i);
                options = candidates;
                if options.len() == 1 {
                    break;
                }
            }
        }
        let Some(i) = best else {
            return true;
        };
        rng.shuffle(&mut options);
        for d in options {
            grid[i] = d;
            if fill(grid, rng, diagonal) {
                return true;
            }
        }
        grid[i] = 0;
        false
    }
    let mut grid = vec![0; 81];
    assert!(fill(&mut grid, rng, diagonal));
    grid
}
fn make_cages(p: &mut Puzzle, rng: &mut Rng) {
    let mut cells: Vec<_> = (0..81).collect();
    rng.shuffle(&mut cells);
    let mut used = [false; 81];
    for start in cells {
        if used[start] {
            continue;
        }
        let mut cage = vec![start];
        used[start] = true;
        let target = 2 + rng.index(3);
        let mut digits = bit(p.solution[start]);
        while cage.len() < target {
            let mut choices: Vec<_> = cage
                .iter()
                .flat_map(|&i| neighbours(i))
                .filter(|&i| !used[i] && digits & bit(p.solution[i]) == 0)
                .collect();
            choices.sort_unstable();
            choices.dedup();
            if choices.is_empty() {
                break;
            }
            let next = choices[rng.index(choices.len())];
            cage.push(next);
            used[next] = true;
            digits |= bit(p.solution[next]);
        }
        cage.sort_unstable();
        let sum = cage.iter().map(|&i| p.solution[i]).sum();
        p.cages.push(Cage { cells: cage, sum });
    }
}
fn make_thermos(p: &mut Puzzle, rng: &mut Rng) {
    let mut starts: Vec<_> = (0..81).collect();
    rng.shuffle(&mut starts);
    for start in starts {
        let mut path = vec![start];
        let target = 3 + rng.index(3);
        while path.len() < target {
            let last = *path.last().unwrap();
            let choices: Vec<_> = neighbours(last)
                .into_iter()
                .filter(|i| !path.contains(i) && p.solution[*i] > p.solution[last])
                .collect();
            if choices.is_empty() {
                break;
            }
            path.push(choices[rng.index(choices.len())]);
        }
        if path.len() >= 3 && !p.thermos.iter().any(|t| t == &path) {
            p.thermos.push(path);
        }
        if p.thermos.len() == 7 {
            break;
        }
    }
}
