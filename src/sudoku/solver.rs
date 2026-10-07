use super::{ALL_DIGITS, Puzzle, bit, digit};
#[derive(Clone, Debug)]
pub struct Deduction {
    pub cell: usize,
    pub value: u8,
    pub reason: &'static str,
}
#[derive(Clone, Debug)]
pub struct LogicResult {
    pub solved: bool,
    pub valid: bool,
    pub steps: Vec<Deduction>,
    pub masks: Vec<u16>,
    pub effort: u32,
}
struct Logic<'a> {
    puzzle: &'a Puzzle,
    masks: Vec<u16>,
    steps: Vec<Deduction>,
    effort: u32,
    valid: bool,
}
impl Logic<'_> {
    fn restrict(&mut self, i: usize, allowed: u16, reason: &'static str, weight: u32) {
        let before = self.masks[i];
        let after = before & allowed;
        if before == after {
            return;
        }
        self.masks[i] = after;
        self.effort += weight;
        if after == 0 {
            self.valid = false;
        }
        if after.count_ones() == 1 && before.count_ones() > 1 {
            self.steps.push(Deduction {
                cell: i,
                value: digit(after),
                reason,
            });
        }
    }
    fn pair(&mut self, a: usize, b: usize, accepts: impl Fn(u8, u8) -> bool, reason: &'static str) {
        let mut ma = 0;
        let mut mb = 0;
        for x in 1..=9 {
            if self.masks[a] & bit(x) == 0 {
                continue;
            }
            for y in 1..=9 {
                if self.masks[b] & bit(y) != 0 && accepts(x, y) {
                    ma |= bit(x);
                    mb |= bit(y);
                }
            }
        }
        self.restrict(a, ma, reason, 2);
        self.restrict(b, mb, reason, 2);
    }
    fn run(mut self, level: u8) -> LogicResult {
        let units = self.puzzle.units();
        loop {
            let before = self.masks.clone();
            for u in &units {
                let mut used = 0;
                for &i in u {
                    let m = self.masks[i];
                    if m.count_ones() == 1 {
                        if used & m != 0 {
                            self.valid = false;
                        }
                        used |= m;
                    }
                }
                for &i in u {
                    if self.masks[i].count_ones() > 1 {
                        self.restrict(i, !used, "Only candidate in this cell", 1);
                    }
                }
                for d in 1..=9 {
                    let places: Vec<_> = u
                        .iter()
                        .copied()
                        .filter(|&i| self.masks[i] & bit(d) != 0)
                        .collect();
                    if places.is_empty() {
                        self.valid = false;
                    }
                    if places.len() == 1 {
                        self.restrict(places[0], bit(d), "Only place for this digit in a unit", 2);
                    }
                }
            }
            for edge in &self.puzzle.edges {
                self.pair(
                    edge.a,
                    edge.b,
                    |a, b| edge.relation.accepts(a, b),
                    "The marked pair fixes this digit",
                );
            }
            for thermo in &self.puzzle.thermos {
                for (j, &i) in thermo.iter().enumerate() {
                    let min = j as u8 + 1;
                    let max = 9 - (thermo.len() - 1 - j) as u8;
                    let allowed = (min..=max).fold(0, |m, d| m | bit(d));
                    self.restrict(i, allowed, "Increasing thermometer bounds", 2);
                }
                for p in thermo.windows(2) {
                    self.pair(
                        p[0],
                        p[1],
                        |a, b| a < b,
                        "Digits increase along the thermometer",
                    );
                }
            }
            for cage in &self.puzzle.cages {
                let masks: Vec<_> = cage.cells.iter().map(|&i| self.masks[i]).collect();
                let mut support = vec![0; cage.cells.len()];
                let mut chosen = vec![0; cage.cells.len()];
                cage_support(&masks, 0, 0, cage.sum as u16, &mut chosen, &mut support);
                for (&i, &m) in cage.cells.iter().zip(&support) {
                    self.restrict(i, m, "The cage total and no repeats fix this digit", 2);
                }
            }
            if level >= 1 {
                // Locked candidates work for boxes, rows, columns and diagonals.
                for a in &units {
                    for d in 1..=9 {
                        let places: Vec<_> = a
                            .iter()
                            .copied()
                            .filter(|&i| self.masks[i] & bit(d) != 0)
                            .collect();
                        if places.len() < 2 {
                            continue;
                        }
                        for b in &units {
                            if a == b || !places.iter().all(|i| b.contains(i)) {
                                continue;
                            }
                            for &i in b {
                                if !a.contains(&i) {
                                    self.restrict(i, !bit(d), "Locked candidates", 5);
                                }
                            }
                        }
                    }
                }
            }
            if level >= 2 {
                for u in &units {
                    for &i in u {
                        let pair = self.masks[i];
                        if pair.count_ones() != 2 {
                            continue;
                        }
                        if u.iter().filter(|&&j| self.masks[j] == pair).count() == 2 {
                            for &j in u {
                                if self.masks[j] != pair {
                                    self.restrict(
                                        j,
                                        !pair,
                                        "A naked pair removes other candidates",
                                        8,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            if !self.valid || self.masks == before {
                break;
            }
        }
        let values: Vec<_> = self.masks.iter().map(|&m| digit(m)).collect();
        LogicResult {
            solved: self.valid && self.puzzle.complete_valid(&values),
            valid: self.valid,
            steps: self.steps,
            masks: self.masks,
            effort: self.effort,
        }
    }
}
fn cage_support(
    masks: &[u16],
    at: usize,
    used: u16,
    remaining: u16,
    chosen: &mut [u16],
    support: &mut [u16],
) {
    if at == masks.len() {
        if remaining == 0 {
            for (s, &c) in support.iter_mut().zip(chosen.iter()) {
                *s |= c;
            }
        }
        return;
    }
    let left = masks.len() - at - 1;
    for d in 1..=9 {
        let b = bit(d);
        if masks[at] & b == 0 || used & b != 0 || d as u16 > remaining {
            continue;
        }
        let rest = remaining - d as u16;
        if rest < left as u16 || rest > left as u16 * 9 {
            continue;
        }
        chosen[at] = b;
        cage_support(masks, at + 1, used | b, rest, chosen, support);
    }
}
pub fn logical_solve(puzzle: &Puzzle, values: &[u8], level: u8) -> LogicResult {
    if values.len() != 81 || values.iter().any(|&v| v > 9) {
        return LogicResult {
            solved: false,
            valid: false,
            steps: Vec::new(),
            masks: Vec::new(),
            effort: 0,
        };
    }
    Logic {
        puzzle,
        masks: values
            .iter()
            .map(|&v| if v == 0 { ALL_DIGITS } else { bit(v) })
            .collect(),
        steps: Vec::new(),
        effort: 0,
        valid: true,
    }
    .run(level)
}
/// Independent search with a conservative work bound. None means unproven.
pub fn solution_count(puzzle: &Puzzle, values: &[u8], budget: usize) -> Option<usize> {
    fn search(p: &Puzzle, masks: Vec<u16>, budget: &mut usize) -> Option<usize> {
        if *budget == 0 {
            return None;
        }
        *budget -= 1;
        let result = Logic {
            puzzle: p,
            masks,
            steps: Vec::new(),
            effort: 0,
            valid: true,
        }
        .run(2);
        if !result.valid {
            return Some(0);
        }
        if result.solved {
            return Some(1);
        }
        let i = result
            .masks
            .iter()
            .enumerate()
            .filter(|(_, m)| m.count_ones() > 1)
            .min_by_key(|(_, m)| m.count_ones())
            .map(|(i, _)| i)?;
        let mut count = 0;
        for d in 1..=9 {
            if result.masks[i] & bit(d) != 0 {
                let mut next = result.masks.clone();
                next[i] = bit(d);
                count += search(p, next, budget)?;
                if count >= 2 {
                    return Some(2);
                }
            }
        }
        Some(count)
    }
    if values.len() != 81 || values.iter().any(|&v| v > 9) {
        return Some(0);
    }
    search(
        puzzle,
        values
            .iter()
            .map(|&v| if v == 0 { ALL_DIGITS } else { bit(v) })
            .collect(),
        &mut budget.clone(),
    )
}
