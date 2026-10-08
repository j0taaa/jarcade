use super::solver::{LogicalProof, UniquenessProof};
use super::{
    Cage, ClueMode, Difficulty, Edge, HardRating, Line, LineKind, Puzzle, Relation, Rules,
    Sandwich, Variant, bit, logical_solve,
};
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
#[derive(Clone)]
enum Removal {
    Digit(usize, u8),
    Edge(Edge),
    Cage(Cage),
    Thermo(Vec<usize>),
    Line(Line),
    Sandwich(Sandwich),
}
impl Removal {
    fn remove(&self, p: &mut Puzzle) -> bool {
        match self {
            Self::Digit(i, _) => p.givens[*i] = 0,
            Self::Edge(edge) => {
                let xv = matches!(edge.relation, Relation::Five | Relation::Ten);
                if p.edges
                    .iter()
                    .filter(|e| matches!(e.relation, Relation::Five | Relation::Ten) == xv)
                    .count()
                    <= 1
                {
                    return false;
                }
                p.edges.retain(|e| e != edge);
            }
            Self::Cage(cage) => {
                if p.cages.len() <= 1 {
                    return false;
                }
                p.cages.retain(|c| c != cage);
            }
            Self::Line(line) => {
                if p.lines.iter().filter(|l| l.kind == line.kind).count() <= 1 {
                    return false;
                }
                p.lines.retain(|l| l != line);
            }
            Self::Sandwich(clue) => {
                if p.sandwiches.len() <= 1 {
                    return false;
                }
                p.sandwiches.retain(|c| c != clue);
            }
            Self::Thermo(thermo) => {
                if p.thermos.len() <= 1 {
                    return false;
                }
                p.thermos.retain(|t| t != thermo);
            }
        }
        true
    }
    fn restore(self, p: &mut Puzzle) {
        match self {
            Self::Digit(i, value) => p.givens[i] = value,
            Self::Edge(e) => p.edges.push(e),
            Self::Cage(c) => p.cages.push(c),
            Self::Thermo(t) => p.thermos.push(t),
            Self::Line(l) => p.lines.push(l),
            Self::Sandwich(c) => p.sandwiches.push(c),
        }
    }
}
/// A resumable logical proof keeps web and phone input responsive.
pub struct Generator {
    puzzle: Puzzle,
    order: Vec<Removal>,
    at: usize,
    target: usize,
    done: bool,
    failed: bool,
    proof: Option<LogicalProof>,
    pending: Option<Removal>,
    uniqueness: Option<UniquenessProof>,
    base_seed: u64,
    attempts: u64,
    completion: Option<Completion>,
}
impl Generator {
    pub fn new(seed: u64, variant: Variant, difficulty: Difficulty) -> Self {
        Self::build(
            seed,
            variant,
            difficulty,
            if Variant::ALL.contains(&variant) {
                None
            } else {
                Some(Rules::from_variant(variant))
            },
        )
    }
    pub fn with_rules(seed: u64, rules: Rules, difficulty: Difficulty) -> Self {
        Self::build(seed, rules.primary(), difficulty, Some(rules))
    }
    fn build(seed: u64, variant: Variant, difficulty: Difficulty, custom: Option<Rules>) -> Self {
        let rules = custom.unwrap_or_else(|| Rules::from_variant(variant));
        let mut rng = Rng(seed);
        let global = rules.anti_knight || rules.anti_king || rules.non_consecutive;
        let completion = (global && rules.diagonal).then(|| Completion::new(seed, rules));
        let solution = if completion.is_some() {
            vec![0; 81]
        } else if global {
            global_grid(&mut rng, rules)
        } else {
            solved_grid(&mut rng, rules.diagonal)
        };
        let puzzle = Puzzle {
            seed,
            variant,
            rules: custom,
            difficulty,
            givens: solution.clone(),
            solution,
            cages: Vec::new(),
            edges: Vec::new(),
            thermos: Vec::new(),
            lines: Vec::new(),
            sandwiches: Vec::new(),
            effort: 0,
            hard_rating: None,
        };
        let targets = match variant {
            Variant::Killer => [28, 16],
            Variant::Kropki => [36, 26],
            Variant::Xv => [42, 32],
            Variant::Thermo => [40, 30],
            Variant::Diagonal => [40, 30],
            Variant::Classic => [44, 34],
            _ => [40, 30],
        };
        let mut generator = Self {
            puzzle,
            order: Vec::new(),
            at: 0,
            target: if difficulty == Difficulty::Hard {
                0
            } else {
                targets[difficulty.level() as usize]
            },
            done: false,
            failed: false,
            proof: None,
            pending: None,
            uniqueness: None,
            base_seed: seed,
            attempts: 0,
            completion,
        };
        if generator.completion.is_none() {
            generator.prepare(&mut rng);
        }
        generator
    }
    fn prepare(&mut self, rng: &mut Rng) {
        let rules = self.puzzle.rules();
        if rules.killer {
            make_cages(&mut self.puzzle, rng);
        }
        for a in 0..81 {
            for b in neighbours(a).into_iter().filter(|&b| b > a) {
                let x = self.puzzle.solution[a];
                let y = self.puzzle.solution[b];
                if rules.xv.enabled() {
                    let relation = if x + y == 5 {
                        Some(Relation::Five)
                    } else if x + y == 10 {
                        Some(Relation::Ten)
                    } else {
                        None
                    };
                    if let Some(relation) = relation {
                        self.puzzle.edges.push(Edge { a, b, relation });
                    }
                }
                if rules.kropki.enabled() {
                    let relation = if x == 2 * y || y == 2 * x {
                        Some(Relation::Double)
                    } else if x.abs_diff(y) == 1 {
                        Some(Relation::Consecutive)
                    } else {
                        None
                    };
                    if let Some(relation) = relation {
                        self.puzzle.edges.push(Edge { a, b, relation });
                    }
                }
            }
        }
        if rules.thermo {
            make_thermos(&mut self.puzzle, rng);
        }
        make_lines(&mut self.puzzle, rng);
        if rules.sandwich {
            for unit in 0..18 {
                let mut clue = Sandwich { unit, sum: 0 };
                let cells = clue.cells();
                let a = cells
                    .iter()
                    .position(|&i| self.puzzle.solution[i] == 1)
                    .unwrap();
                let b = cells
                    .iter()
                    .position(|&i| self.puzzle.solution[i] == 9)
                    .unwrap();
                clue.sum = cells[a.min(b) + 1..a.max(b)]
                    .iter()
                    .map(|&i| self.puzzle.solution[i])
                    .sum();
                self.puzzle.sandwiches.push(clue);
            }
        }
        let mut order: Vec<_> = (0..81)
            .map(|i| Removal::Digit(i, self.puzzle.solution[i]))
            .collect();
        // Full symbols are mandatory. All other clues are candidates for
        // removal, including cage totals; uncaged cells use normal Sudoku.
        // Legacy generators retain their exact clue format and seeded output.
        if self.puzzle.rules.is_some() && self.puzzle.difficulty == Difficulty::Hard {
            order.extend(
                self.puzzle
                    .edges
                    .iter()
                    .filter(|e| {
                        if matches!(e.relation, Relation::Five | Relation::Ten) {
                            rules.xv == ClueMode::Partial
                        } else {
                            rules.kropki == ClueMode::Partial
                        }
                    })
                    .cloned()
                    .map(Removal::Edge),
            );
            order.extend(self.puzzle.cages.iter().cloned().map(Removal::Cage));
            order.extend(self.puzzle.thermos.iter().cloned().map(Removal::Thermo));
            order.extend(self.puzzle.lines.iter().cloned().map(Removal::Line));
            order.extend(
                self.puzzle
                    .sandwiches
                    .iter()
                    .cloned()
                    .map(Removal::Sandwich),
            );
        }
        rng.shuffle(&mut order);
        self.order = order;
    }
    pub fn preview(seed: u64, variant: Variant) -> Puzzle {
        let mut puzzle = Self::new(seed, variant, Difficulty::Easy).puzzle;
        for i in 0..81 {
            if i % 3 != 0 {
                puzzle.givens[i] = 0;
            }
        }
        puzzle
    }
    pub fn attempts(&self) -> u64 {
        self.attempts + 1
    }
    pub fn progress(&self) -> f32 {
        if self.done {
            1.
        } else {
            if self.order.is_empty() {
                0.
            } else {
                self.at as f32 / self.order.len() as f32 * 0.9
            }
        }
    }
    pub fn step(&mut self) -> bool {
        if self.done {
            return true;
        }
        if let Some(completion) = &mut self.completion {
            if let Some(result) = completion.step() {
                self.completion = None;
                match result {
                    Some(solution) => {
                        self.puzzle.givens = solution.clone();
                        self.puzzle.solution = solution;
                        let mut rng = Rng(self.puzzle.seed);
                        self.prepare(&mut rng);
                    }
                    None => {
                        self.failed = true;
                        self.done = true;
                    }
                }
            }
            return self.done;
        }
        if let Some(search) = &mut self.uniqueness {
            if !search.step() {
                return false;
            }
            let unique = self.uniqueness.take().unwrap().count() == Some(1);
            let removal = self.pending.take().unwrap();
            if !unique {
                removal.restore(&mut self.puzzle);
            }
            return false;
        }
        if let Some(proof) = &mut self.proof {
            if !proof.step() {
                return false;
            }
            let result = self.proof.take().unwrap().result();
            if let Some(removal) = self.pending.take() {
                if !result.solved {
                    removal.restore(&mut self.puzzle);
                } else if self.puzzle.difficulty == Difficulty::Hard
                    && self.puzzle.rules().non_consecutive
                {
                    self.uniqueness = Some(UniquenessProof::new(self.puzzle.clone(), 2_000));
                    self.pending = Some(removal);
                }
                return false;
            }
            let rating = (self.puzzle.difficulty == Difficulty::Hard).then(|| {
                let simple = logical_solve(&self.puzzle, &self.puzzle.givens, 2);
                HardRating {
                    stalled_cells: simple.masks.iter().filter(|m| m.count_ones() > 1).count(),
                    advanced_steps: result.advanced_steps,
                    forcing_steps: result.forcing_steps,
                    longest_chain: result.longest_chain,
                }
            });
            let rules = self.puzzle.rules();
            let visible = LineKind::ALL.iter().all(|kind| {
                !rules.contains(kind.variant())
                    || self.puzzle.lines.iter().any(|line| line.kind == *kind)
            }) && (!rules.sandwich || !self.puzzle.sandwiches.is_empty());
            let qualifies =
                visible && result.solved && rating.as_ref().is_none_or(HardRating::qualifies);
            if qualifies {
                self.puzzle.hard_rating = rating;
                self.puzzle.effort = result.effort;
                self.puzzle.seed = self.base_seed;
                self.done = true;
                return true;
            }
            // A full grid isn't Hard just because it has few givens. Discard
            // easy or unproved candidates rather than silently downgrading.
            if self.puzzle.rules.is_some() && self.attempts >= 255 {
                self.failed = true;
                self.done = true;
                return true;
            }
            let base_seed = self.base_seed;
            let attempts = self.attempts + 1;
            let seed = base_seed.wrapping_add(attempts.wrapping_mul(0x9e3779b97f4a7c15));
            *self = Self::build(
                seed,
                self.puzzle.variant,
                self.puzzle.difficulty,
                self.puzzle.rules,
            );
            self.base_seed = base_seed;
            self.attempts = attempts;
            return false;
        }
        if self.at == self.order.len()
            || self.target > 0
                && self.puzzle.givens.iter().filter(|&&v| v > 0).count() <= self.target
        {
            self.proof = Some(LogicalProof::new(
                self.puzzle.clone(),
                self.puzzle.givens.clone(),
                self.puzzle.difficulty.level(),
            ));
            return false;
        }
        let removal = self.order[self.at].clone();
        self.at += 1;
        if !removal.remove(&mut self.puzzle) {
            return false;
        }
        self.pending = Some(removal);
        if self.puzzle.difficulty == Difficulty::Hard && !self.puzzle.rules().non_consecutive {
            self.uniqueness = Some(UniquenessProof::new(self.puzzle.clone(), 2_000));
        } else {
            self.proof = Some(LogicalProof::new(
                self.puzzle.clone(),
                self.puzzle.givens.clone(),
                self.puzzle.difficulty.level(),
            ));
        }
        false
    }

    pub fn try_finish(mut self) -> Result<Puzzle, &'static str> {
        while !self.step() {}
        if self.failed {
            Err(
                "No puzzle matching these rules and difficulty was found. Try fewer rules, partial markings, or retry.",
            )
        } else {
            Ok(self.puzzle)
        }
    }
    pub fn finish(self) -> Puzzle {
        self.try_finish()
            .expect("No puzzle satisfies the selected difficulty")
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

fn global_grid(rng: &mut Rng, rules: Rules) -> Vec<u8> {
    // This Latin pattern obeys both chess restrictions. Non-consecutive
    // relabelling puts successive digits two positions apart on the cycle.
    let mut labels = if rules.non_consecutive {
        vec![1, 6, 2, 7, 3, 8, 4, 9, 5]
    } else {
        (1..=9).collect()
    };
    if !rules.non_consecutive {
        rng.shuffle(&mut labels);
    } else if rng.index(2) == 0 {
        for d in &mut labels {
            *d = 10 - *d;
        }
    }
    if rules.non_consecutive {
        let shift = rng.index(9);
        labels.rotate_left(shift);
    }
    let transform = rng.index(8);
    (0..81)
        .map(|i| {
            let (mut r, mut c) = (i / 9, i % 9);
            if transform & 1 != 0 {
                std::mem::swap(&mut r, &mut c);
            }
            if transform & 2 != 0 {
                r = 8 - r;
            }
            if transform & 4 != 0 {
                c = 8 - c;
            }
            labels[(r % 3 * 3 + r / 3 + c) % 9]
        })
        .collect()
}
struct Completion {
    pending: Vec<Vec<u8>>,
    rules: Rules,
    rng: Rng,
    remaining: usize,
}
impl Completion {
    fn new(seed: u64, rules: Rules) -> Self {
        Self {
            pending: vec![vec![0; 81]],
            rules,
            rng: Rng(seed),
            remaining: 20_000,
        }
    }
    fn step(&mut self) -> Option<Option<Vec<u8>>> {
        if self.remaining == 0 {
            return Some(None);
        }
        self.remaining -= 1;
        let Some(grid) = self.pending.pop() else {
            return Some(None);
        };
        let mut best = None;
        let mut options = Vec::new();
        for i in 0..81 {
            if grid[i] != 0 {
                continue;
            }
            let choices: Vec<_> = (1..=9)
                .filter(|&d| {
                    grid.iter().enumerate().all(|(j, &v)| {
                        if v == 0 {
                            return true;
                        }
                        let peers = i / 9 == j / 9
                            || i % 9 == j % 9
                            || super::constraints::box_id(i) == super::constraints::box_id(j)
                            || self.rules.diagonal
                                && (i / 9 == i % 9 && j / 9 == j % 9
                                    || i / 9 + i % 9 == 8 && j / 9 + j % 9 == 8);
                        (!peers || d != v)
                            && self
                                .rules
                                .pair_kind(i, j)
                                .is_none_or(|nc| if nc { d.abs_diff(v) != 1 } else { d != v })
                    })
                })
                .collect();
            if choices.is_empty() {
                return None;
            }
            if best.is_none() || choices.len() < options.len() {
                best = Some(i);
                options = choices;
                if options.len() == 1 {
                    break;
                }
            }
        }
        let Some(i) = best else {
            return Some(Some(grid));
        };
        self.rng.shuffle(&mut options);
        for d in options {
            let mut child = grid.clone();
            child[i] = d;
            self.pending.push(child);
        }
        None
    }
}
fn make_lines(p: &mut Puzzle, rng: &mut Rng) {
    let neighbours = |i: usize| -> Vec<usize> {
        (0..81)
            .filter(|&j| j != i && (i / 9).abs_diff(j / 9) <= 1 && (i % 9).abs_diff(j % 9) <= 1)
            .collect()
    };
    for kind in LineKind::ALL {
        if !p.rules().contains(kind.variant()) {
            continue;
        }
        for _ in 0..1200 {
            let mut cells = vec![rng.index(81)];
            let target = 3 + rng.index(if kind == LineKind::Arrow { 3 } else { 6 });
            while cells.len() < target {
                let choices: Vec<_> = neighbours(*cells.last().unwrap())
                    .into_iter()
                    .filter(|i| !cells.contains(i))
                    .collect();
                if choices.is_empty() {
                    break;
                }
                cells.push(choices[rng.index(choices.len())]);
            }
            let line = Line { kind, cells };
            if line.valid_shape() && line.valid(&p.solution) && !p.lines.contains(&line) {
                p.lines.push(line);
            }
            if p.lines.iter().filter(|l| l.kind == kind).count()
                >= if p.difficulty == Difficulty::Hard {
                    5
                } else {
                    3
                }
            {
                break;
            }
        }
    }
}
