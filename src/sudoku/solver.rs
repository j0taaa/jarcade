mod advanced;
use super::{ALL_DIGITS, Puzzle, bit, digit};
#[derive(Clone, Debug)]
pub struct Deduction {
    pub cell: usize,
    pub value: u8,
    pub reason: String,
}
#[derive(Clone, Debug)]
pub struct LogicResult {
    pub solved: bool,
    pub valid: bool,
    pub steps: Vec<Deduction>,
    pub masks: Vec<u16>,
    pub effort: u32,
    pub advanced_steps: u32,
    pub forcing_steps: u32,
    pub longest_chain: u32,
}
struct Logic<'a> {
    puzzle: &'a Puzzle,
    masks: Vec<u16>,
    steps: Vec<Deduction>,
    effort: u32,
    valid: bool,
    advanced_steps: u32,
    context: Option<String>,
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
                reason: self.context.as_ref().map_or_else(
                    || reason.into(),
                    |context| format!("{context} Then: {reason}"),
                ),
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
    fn pass(&mut self, level: u8) -> bool {
        let units = self.puzzle.units();
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
        for &(a, b, nc) in self.puzzle.rules().global_pairs() {
            self.pair(
                a,
                b,
                |x, y| if nc { x.abs_diff(y) != 1 } else { x != y },
                if nc {
                    "Orthogonal neighbours cannot be consecutive"
                } else {
                    "The chess restriction excludes an equal digit"
                },
            );
        }
        for line in &self.puzzle.lines {
            let support = super::constraints::line_support(line, &self.masks);
            for (&i, &allowed) in line.cells.iter().zip(&support) {
                self.restrict(i, allowed, line.kind.reason(), 2);
            }
        }
        for clue in &self.puzzle.sandwiches {
            let support = super::constraints::sandwich_support(clue, &self.masks);
            for (&i, &allowed) in clue.cells().iter().zip(&support) {
                self.restrict(
                    i,
                    allowed,
                    "The sandwich total fixes the digits between 1 and 9",
                    2,
                );
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
        for (a, b, xv, dots) in self.puzzle.negative_pairs() {
            self.pair(
                a,
                b,
                |x, y| Puzzle::unmarked_accepts(x, y, xv, dots),
                "Full marking: this unmarked edge excludes the relationship",
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
                                self.restrict(j, !pair, "A naked pair removes other candidates", 8);
                            }
                        }
                    }
                }
            }
        }
        if level >= 3 && self.valid && self.masks == before {
            self.patterns(&units);
        }
        self.valid && self.masks != before
    }
    fn run(mut self, level: u8) -> LogicResult {
        while self.pass(level) {}
        self.result()
    }
    fn result(self) -> LogicResult {
        let values: Vec<_> = self.masks.iter().map(|&m| digit(m)).collect();
        LogicResult {
            solved: self.valid && self.puzzle.complete_valid(&values),
            valid: self.valid,
            steps: self.steps,
            masks: self.masks,
            effort: self.effort,
            advanced_steps: self.advanced_steps,
            forcing_steps: 0,
            longest_chain: 0,
        }
    }
}
fn pass(
    puzzle: &Puzzle,
    masks: Vec<u16>,
    level: u8,
    context: Option<String>,
) -> (LogicResult, bool, Option<String>) {
    let mut logic = Logic {
        puzzle,
        masks,
        steps: Vec::new(),
        effort: 0,
        valid: true,
        advanced_steps: 0,
        context,
    };
    let changed = logic.pass(level);
    let context = logic.context.clone();
    (logic.result(), changed, context)
}
/// Resumable proof: each step runs propagation or one bounded implication
/// branch. No guesses are committed; only contradictions/common consequences.
pub(super) struct LogicalProof {
    puzzle: Puzzle,
    level: u8,
    result: LogicResult,
    sources: Vec<usize>,
    source: usize,
    candidate: u8,
    union: Vec<u16>,
    supported: bool,
    chain: u32,
    propagate: bool,
    done: bool,
    branch: Option<LogicResult>,
    context: Option<String>,
}
impl LogicalProof {
    pub fn new(puzzle: Puzzle, values: Vec<u8>, level: u8) -> Self {
        Self {
            puzzle,
            level,
            result: LogicResult {
                solved: false,
                valid: true,
                steps: Vec::new(),
                masks: values
                    .iter()
                    .map(|&v| if v == 0 { ALL_DIGITS } else { bit(v) })
                    .collect(),
                effort: 0,
                advanced_steps: 0,
                forcing_steps: 0,
                longest_chain: 0,
            },
            sources: Vec::new(),
            source: 0,
            candidate: 1,
            union: vec![0; 81],
            supported: false,
            chain: 0,
            propagate: true,
            done: false,
            branch: None,
            context: None,
        }
    }
    pub fn result(self) -> LogicResult {
        self.result
    }
    pub fn step(&mut self) -> bool {
        if self.done {
            return true;
        }
        if self.propagate {
            let prior = &mut self.result;
            let (next, changed, context) = pass(
                &self.puzzle,
                prior.masks.clone(),
                self.level,
                self.context.clone(),
            );
            self.context = context;
            prior.steps.extend(next.steps);
            prior.masks = next.masks;
            prior.effort += next.effort;
            prior.advanced_steps += next.advanced_steps;
            prior.solved = next.solved;
            prior.valid = next.valid;
            if prior.solved || !prior.valid {
                self.done = true;
                return true;
            }
            if changed {
                return false;
            }
            self.propagate = false;
            if self.level < 3 {
                self.done = true;
                return true;
            }
            self.sources = (0..81)
                .filter(|&i| (2..=4).contains(&prior.masks[i].count_ones()))
                .collect();
            self.sources
                .sort_by_key(|&i| (prior.masks[i].count_ones(), i));
            self.source = 0;
            self.candidate = 1;
            self.union.fill(0);
            self.supported = false;
            self.chain = 0;
            return false;
        }
        let Some(&cell) = self.sources.get(self.source) else {
            self.done = true;
            return true;
        };
        while self.candidate <= 9 && self.result.masks[cell] & bit(self.candidate) == 0 {
            self.candidate += 1;
        }
        if let Some(branch) = &mut self.branch {
            let (next, changed, _) = pass(&self.puzzle, branch.masks.clone(), 3, None);
            branch.steps.extend(next.steps);
            branch.masks = next.masks;
            branch.valid = next.valid;
            branch.solved = next.solved;
            if changed && branch.valid && !branch.solved {
                return false;
            }
            let branch = self.branch.take().unwrap();
            self.chain = self.chain.max(branch.steps.len() as u32);
            if branch.valid {
                self.supported = true;
                for (union, mask) in self.union.iter_mut().zip(branch.masks) {
                    *union |= mask;
                }
            }
            return false;
        }
        if self.candidate <= 9 {
            let d = self.candidate;
            self.candidate += 1;
            let mut masks = self.result.masks.clone();
            masks[cell] = bit(d);
            self.branch = Some(LogicResult {
                masks,
                steps: Vec::new(),
                solved: false,
                valid: true,
                effort: 0,
                advanced_steps: 0,
                forcing_steps: 0,
                longest_chain: 0,
            });
            return false;
        }
        if !self.supported {
            self.result.valid = false;
            self.done = true;
            return true;
        }
        let before = self.result.masks.clone();
        let reason = format!(
            "Forcing net at r{}c{}: every viable candidate leads to these eliminations (up to {} linked placements).",
            cell / 9 + 1,
            cell % 9 + 1,
            self.chain
        );
        for (i, &old) in before.iter().enumerate() {
            let new = old & self.union[i];
            if new != old {
                self.result.masks[i] = new;
                self.result.effort += 100;
                if old.count_ones() > 1 && new.count_ones() == 1 {
                    self.result.steps.push(Deduction {
                        cell: i,
                        value: digit(new),
                        reason: reason.clone(),
                    });
                }
            }
        }
        if before != self.result.masks {
            self.result.forcing_steps += 1;
            self.result.advanced_steps += 1;
            self.result.longest_chain = self.result.longest_chain.max(self.chain);
            self.context = Some(reason);
            self.propagate = true;
        } else {
            self.source += 1;
            self.candidate = 1;
            self.union.fill(0);
            self.supported = false;
            self.chain = 0;
        }
        false
    }
}
/// Independent, bounded uniqueness search. Each node has a resumable proof;
/// reaching the budget reports unproven rather than accepting a puzzle.
pub(super) struct UniquenessProof {
    puzzle: Puzzle,
    pending: Vec<Vec<u16>>,
    proof: Option<LogicalProof>,
    remaining: usize,
    count: usize,
    done: bool,
    exhausted: bool,
}
impl UniquenessProof {
    pub fn new(puzzle: Puzzle, budget: usize) -> Self {
        let masks = puzzle
            .givens
            .iter()
            .map(|&v| if v == 0 { ALL_DIGITS } else { bit(v) })
            .collect();
        Self {
            puzzle,
            pending: vec![masks],
            proof: None,
            remaining: budget,
            count: 0,
            done: false,
            exhausted: false,
        }
    }
    pub fn count(self) -> Option<usize> {
        (!self.exhausted).then_some(self.count)
    }
    pub fn step(&mut self) -> bool {
        if self.done {
            return true;
        }
        if let Some(proof) = &mut self.proof {
            if !proof.step() {
                return false;
            }
            let result = self.proof.take().unwrap().result();
            if result.solved {
                self.count += 1;
            } else if result.valid {
                let i = (0..81)
                    .filter(|&i| result.masks[i].count_ones() > 1)
                    .min_by_key(|&i| result.masks[i].count_ones())
                    .unwrap();
                for d in (1..=9).rev() {
                    if result.masks[i] & bit(d) != 0 {
                        let mut masks = result.masks.clone();
                        masks[i] = bit(d);
                        self.pending.push(masks);
                    }
                }
            }
            if self.count >= 2 {
                self.done = true;
                return true;
            }
            return false;
        }
        let Some(masks) = self.pending.pop() else {
            self.done = true;
            return true;
        };
        if self.remaining == 0 {
            self.exhausted = true;
            self.done = true;
            return true;
        }
        self.remaining -= 1;
        let mut proof = LogicalProof::new(self.puzzle.clone(), vec![0; 81], 2);
        proof.result.masks = masks;
        self.proof = Some(proof);
        false
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
            advanced_steps: 0,
            forcing_steps: 0,
            longest_chain: 0,
        };
    }
    let mut proof = LogicalProof::new(puzzle.clone(), values.to_vec(), level);
    while !proof.step() {}
    proof.result()
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
            advanced_steps: 0,
            context: None,
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
