//! Offline Sudoku: original generated grids, variant constraints and pencil marks.
mod generator;
mod solver;
pub use generator::Generator;
use serde::{Deserialize, Serialize};
pub use solver::{Deduction, LogicResult, logical_solve, solution_count};

pub const ALL_DIGITS: u16 = 0x1ff;
pub fn bit(digit: u8) -> u16 {
    if (1..=9).contains(&digit) {
        1 << (digit - 1)
    } else {
        0
    }
}
pub fn digit(mask: u16) -> u8 {
    if mask.count_ones() == 1 {
        mask.trailing_zeros() as u8 + 1
    } else {
        0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Variant {
    #[default]
    Classic,
    Killer,
    Xv,
    Kropki,
    Thermo,
    Diagonal,
}
impl Variant {
    pub const ALL: [Self; 6] = [
        Self::Classic,
        Self::Killer,
        Self::Xv,
        Self::Kropki,
        Self::Thermo,
        Self::Diagonal,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Killer => "Killer",
            Self::Xv => "V & X",
            Self::Kropki => "Kropki",
            Self::Thermo => "Thermo",
            Self::Diagonal => "Diagonal",
        }
    }
    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Classic => "The original nine",
            Self::Killer => "Little sums, big ideas",
            Self::Xv => "Five and ten",
            Self::Kropki => "Connect the dots",
            Self::Thermo => "Follow the warmth",
            Self::Diagonal => "Across both diagonals",
        }
    }
    pub fn rules(self) -> &'static [&'static str] {
        match self {
            Self::Classic => &[
                "Each row, column and 3 × 3 box",
                "contains the digits 1–9 once.",
            ],
            Self::Killer => &[
                "Normal Sudoku rules apply.",
                "Digits in a dotted cage add to its",
                "small total. No repeats in a cage.",
            ],
            Self::Xv => &[
                "Normal Sudoku rules apply.",
                "V joins digits adding to 5; X to 10.",
                "Unmarked edges have no extra rule.",
            ],
            Self::Kropki => &[
                "Normal Sudoku rules apply.",
                "Hollow dots: consecutive digits.",
                "Filled dots: one digit is twice the other.",
                "Unmarked edges have no extra rule.",
            ],
            Self::Thermo => &[
                "Normal Sudoku rules apply.",
                "Digits strictly increase from the bulb",
                "to the tip of each thermometer.",
            ],
            Self::Diagonal => &[
                "Normal Sudoku rules apply.",
                "Both long diagonals also contain",
                "the digits 1–9 once.",
            ],
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    #[default]
    Easy,
    Medium,
    Hard,
}
impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Medium, Self::Hard];
    pub fn name(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Medium => "Medium",
            Self::Hard => "Hard",
        }
    }
    pub fn level(self) -> u8 {
        match self {
            Self::Easy => 0,
            Self::Medium => 1,
            Self::Hard => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relation {
    Five,
    Ten,
    Consecutive,
    Double,
}
impl Relation {
    pub fn accepts(self, a: u8, b: u8) -> bool {
        match self {
            Self::Five => a + b == 5,
            Self::Ten => a + b == 10,
            Self::Consecutive => a.abs_diff(b) == 1,
            Self::Double => a == 2 * b || b == 2 * a,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub relation: Relation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cage {
    pub cells: Vec<usize>,
    pub sum: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Puzzle {
    pub seed: u64,
    pub variant: Variant,
    pub difficulty: Difficulty,
    pub givens: Vec<u8>,
    pub solution: Vec<u8>,
    pub cages: Vec<Cage>,
    pub edges: Vec<Edge>,
    pub thermos: Vec<Vec<usize>>,
    pub effort: u32,
}
impl Puzzle {
    pub fn units(&self) -> Vec<Vec<usize>> {
        let mut units = Vec::with_capacity(29);
        for i in 0..9 {
            units.push((0..9).map(|x| i * 9 + x).collect());
            units.push((0..9).map(|y| y * 9 + i).collect());
            units.push(
                (0..9)
                    .map(|j| (i / 3 * 3 + j / 3) * 9 + i % 3 * 3 + j % 3)
                    .collect(),
            );
        }
        if self.variant == Variant::Diagonal {
            units.push((0..9).map(|i| i * 10).collect());
            units.push((0..9).map(|i| i * 9 + 8 - i).collect());
        }
        units
    }
    pub fn complete_valid(&self, values: &[u8]) -> bool {
        if values.len() != 81 || values.iter().any(|&v| !(1..=9).contains(&v)) {
            return false;
        }
        if self
            .units()
            .iter()
            .any(|u| u.iter().fold(0, |m, &i| m | bit(values[i])) != ALL_DIGITS)
        {
            return false;
        }
        if self.cages.iter().any(|c| {
            let mask = c.cells.iter().fold(0u16, |m, &i| m | bit(values[i]));
            mask.count_ones() != c.cells.len() as u32
                || c.cells.iter().map(|&i| values[i] as u16).sum::<u16>() != c.sum as u16
        }) {
            return false;
        }
        self.edges
            .iter()
            .all(|e| e.relation.accepts(values[e.a], values[e.b]))
            && self
                .thermos
                .iter()
                .all(|t| t.windows(2).all(|p| values[p[0]] < values[p[1]]))
    }
    pub fn validated(&self) -> bool {
        if self.givens.len() != 81
            || self.solution.len() != 81
            || self
                .givens
                .iter()
                .zip(&self.solution)
                .any(|(&a, &b)| a > 9 || (a != 0 && a != b))
        {
            return false;
        }
        if self.cages.len() > 81 || self.edges.len() > 144 || self.thermos.len() > 12 {
            return false;
        }
        let adjacent = |a: usize, b: usize| {
            a < 81
                && b < 81
                && (a / 9 == b / 9 && a.abs_diff(b) == 1 || a % 9 == b % 9 && a.abs_diff(b) == 9)
        };
        let mut covered = [false; 81];
        for cage in &self.cages {
            if cage.cells.is_empty() || cage.cells.len() > 4 {
                return false;
            }
            for &i in &cage.cells {
                if i >= 81 || covered[i] {
                    return false;
                }
                covered[i] = true;
            }
            let mut connected = vec![cage.cells[0]];
            loop {
                let next =
                    cage.cells.iter().copied().find(|i| {
                        !connected.contains(i) && connected.iter().any(|&j| adjacent(*i, j))
                    });
                if let Some(i) = next {
                    connected.push(i);
                } else {
                    break;
                }
            }
            if connected.len() != cage.cells.len() {
                return false;
            }
        }
        if self.variant == Variant::Killer && covered.contains(&false) {
            return false;
        }
        if self.variant != Variant::Killer && !self.cages.is_empty() {
            return false;
        }
        let mut seen = Vec::new();
        for e in &self.edges {
            if !adjacent(e.a, e.b) || seen.contains(&(e.a.min(e.b), e.a.max(e.b))) {
                return false;
            }
            if !matches!(
                (self.variant, e.relation),
                (Variant::Xv, Relation::Five | Relation::Ten)
                    | (Variant::Kropki, Relation::Consecutive | Relation::Double)
            ) {
                return false;
            }
            seen.push((e.a.min(e.b), e.a.max(e.b)));
        }
        if self.variant != Variant::Thermo && !self.thermos.is_empty() {
            return false;
        }
        for t in &self.thermos {
            if !(2..=6).contains(&t.len())
                || t.windows(2).any(|p| !adjacent(p[0], p[1]))
                || t.iter().enumerate().any(|(j, i)| t[..j].contains(i))
            {
                return false;
            }
        }
        self.complete_valid(&self.solution)
            && logical_solve(self, &self.givens, self.difficulty.level()).solved
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[u16; 4]", into = "[u16; 4]")]
pub struct Mark {
    pub value: u8,
    pub corner: u16,
    pub centre: u16,
    pub colour: u8,
}
impl From<Mark> for [u16; 4] {
    fn from(m: Mark) -> Self {
        [m.value as u16, m.corner, m.centre, m.colour as u16]
    }
}
impl TryFrom<[u16; 4]> for Mark {
    type Error = &'static str;
    fn try_from(m: [u16; 4]) -> Result<Self, Self::Error> {
        if m[0] > 9 || m[1] & !ALL_DIGITS != 0 || m[2] & !ALL_DIGITS != 0 || m[3] > 9 {
            return Err("invalid pencil mark");
        }
        Ok(Self {
            value: m[0] as u8,
            corner: m[1],
            centre: m[2],
            colour: m[3] as u8,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Digit,
    Corner,
    Centre,
    Colour,
}
impl Tool {
    pub const ALL: [Self; 4] = [Self::Digit, Self::Corner, Self::Centre, Self::Colour];
    pub fn name(self) -> &'static str {
        match self {
            Self::Digit => "Digit",
            Self::Corner => "Corner notes",
            Self::Centre => "Centre notes",
            Self::Colour => "Colour",
        }
    }
    pub fn next(self) -> Self {
        Self::ALL[(Self::ALL.iter().position(|&t| t == self).unwrap() + 1) % 4]
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Change {
    before: Vec<Mark>,
    after: Vec<Mark>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub puzzle: Puzzle,
    pub marks: Vec<Mark>,
    undo: Vec<Change>,
    redo: Vec<Change>,
    pub hints: u32,
}
impl Game {
    pub fn new(puzzle: Puzzle) -> Self {
        Self {
            puzzle,
            marks: vec![Mark::default(); 81],
            undo: Vec::new(),
            redo: Vec::new(),
            hints: 0,
        }
    }
    pub fn values(&self) -> Vec<u8> {
        self.puzzle
            .givens
            .iter()
            .zip(&self.marks)
            .map(|(&g, m)| if g > 0 { g } else { m.value })
            .collect()
    }
    pub fn filled(&self) -> usize {
        self.values().iter().filter(|&&d| d > 0).count()
    }
    pub fn won(&self) -> bool {
        self.puzzle.complete_valid(&self.values())
    }
    fn finish(&mut self, before: Vec<Mark>) -> bool {
        if before == self.marks {
            return false;
        }
        self.undo.push(Change {
            before,
            after: self.marks.clone(),
        });
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
        true
    }
    pub fn enter(&mut self, selected: &[usize], number: u8, tool: Tool) -> bool {
        if !(1..=9).contains(&number) {
            return false;
        }
        let before = self.marks.clone();
        let valid: Vec<_> = selected
            .iter()
            .copied()
            .filter(|&i| i < 81 && (tool == Tool::Colour || self.puzzle.givens[i] == 0))
            .collect();
        let remove = !valid.is_empty()
            && valid.iter().all(|&i| match tool {
                Tool::Digit => self.marks[i].value == number,
                Tool::Corner => self.marks[i].corner & bit(number) != 0,
                Tool::Centre => self.marks[i].centre & bit(number) != 0,
                Tool::Colour => self.marks[i].colour == number,
            });
        for i in valid {
            let m = &mut self.marks[i];
            match tool {
                Tool::Digit => {
                    m.value = if remove { 0 } else { number };
                    m.corner = 0;
                    m.centre = 0;
                }
                Tool::Corner if m.value == 0 => {
                    if remove {
                        m.corner &= !bit(number)
                    } else {
                        m.corner |= bit(number)
                    }
                }
                Tool::Centre if m.value == 0 => {
                    if remove {
                        m.centre &= !bit(number)
                    } else {
                        m.centre |= bit(number)
                    }
                }
                Tool::Colour => m.colour = if remove { 0 } else { number },
                _ => {}
            }
        }
        self.finish(before)
    }
    pub fn erase(&mut self, selected: &[usize], tool: Tool) -> bool {
        let before = self.marks.clone();
        for &i in selected {
            if i >= 81 {
                continue;
            }
            let m = &mut self.marks[i];
            if tool == Tool::Colour {
                m.colour = 0;
            } else if self.puzzle.givens[i] == 0 {
                match tool {
                    Tool::Digit => {
                        *m = Mark {
                            colour: m.colour,
                            ..Mark::default()
                        }
                    }
                    Tool::Corner => m.corner = 0,
                    Tool::Centre => m.centre = 0,
                    _ => {}
                }
            }
        }
        self.finish(before)
    }
    pub fn undo(&mut self) -> bool {
        if let Some(c) = self.undo.pop() {
            self.marks = c.before.clone();
            self.redo.push(c);
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self) -> bool {
        if let Some(c) = self.redo.pop() {
            self.marks = c.after.clone();
            self.undo.push(c);
            true
        } else {
            false
        }
    }
    pub fn reset(&mut self) -> bool {
        let before = self.marks.clone();
        self.marks.fill(Mark::default());
        self.finish(before)
    }
    pub fn mistakes(&self) -> Vec<usize> {
        self.values()
            .iter()
            .zip(&self.puzzle.solution)
            .enumerate()
            .filter_map(|(i, (&a, &b))| (a != 0 && a != b).then_some(i))
            .collect()
    }
    pub fn hint(&mut self) -> Option<(usize, String)> {
        if let Some(i) = self.mistakes().first().copied() {
            return Some((
                i,
                "This digit contradicts the puzzle. Clear it and try again.".into(),
            ));
        }
        let result = logical_solve(&self.puzzle, &self.values(), 2);
        let step = result.steps.first()?;
        let before = self.marks.clone();
        self.marks[step.cell].value = step.value;
        self.marks[step.cell].corner = 0;
        self.marks[step.cell].centre = 0;
        self.finish(before);
        self.hints += 1;
        Some((
            step.cell,
            format!(
                "{} → {} at r{}c{}.",
                step.reason,
                step.value,
                step.cell / 9 + 1,
                step.cell % 9 + 1
            ),
        ))
    }
    pub fn encode(&self) -> String {
        serde_json::to_string(&Save {
            version: 1,
            game: self.clone(),
        })
        .unwrap_or_default()
    }
    pub fn restore(data: &str) -> Option<Self> {
        if data.len() > 262_144 {
            return None;
        }
        let save: Save = serde_json::from_str(data).ok()?;
        let mut g = save.game;
        if save.version != 1
            || !g.puzzle.validated()
            || !g.valid_marks(&g.marks)
            || g.undo.len() > 100
            || g.redo.len() > 100
        {
            return None;
        }
        if g.undo
            .iter()
            .chain(&g.redo)
            .any(|c| !g.valid_marks(&c.before) || !g.valid_marks(&c.after))
        {
            return None;
        }
        // History must be a coherent chain, not an opportunity to mutate givens.
        let mut current = &g.marks;
        for c in g.undo.iter().rev() {
            if current != &c.after {
                return None;
            }
            current = &c.before;
        }
        current = &g.marks;
        for c in g.redo.iter().rev() {
            if current != &c.before {
                return None;
            }
            current = &c.after;
        }
        g.hints = g.hints.min(100_000);
        Some(g)
    }
    fn valid_marks(&self, m: &[Mark]) -> bool {
        m.len() == 81
            && m.iter().enumerate().all(|(i, m)| {
                m.value <= 9
                    && m.corner & !ALL_DIGITS == 0
                    && m.centre & !ALL_DIGITS == 0
                    && m.colour <= 9
                    && (self.puzzle.givens[i] == 0
                        || m.value == 0 && m.corner == 0 && m.centre == 0)
            })
    }
}
#[derive(Serialize, Deserialize)]
struct Save {
    version: u8,
    game: Game,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_modes_and_levels_generate_unique_logical_puzzles() {
        for variant in Variant::ALL {
            for level in Difficulty::ALL {
                for seed in [0, 17, 901] {
                    let p = Generator::new(seed, variant, level).finish();
                    assert!(p.validated(), "{variant:?} {level:?} {seed}");
                    let proof = logical_solve(&p, &p.givens, level.level());
                    assert!(proof.solved);
                    assert_eq!(
                        proof.masks.iter().map(|&m| digit(m)).collect::<Vec<_>>(),
                        p.solution
                    );
                    assert_eq!(solution_count(&p, &p.givens, 10_000), Some(1));
                    assert!(p.givens.contains(&0));
                    assert!(p.givens.iter().filter(|&&v| v == 0).count() >= 30);
                    if variant == Variant::Xv || variant == Variant::Kropki {
                        assert!(!p.edges.is_empty());
                    }
                    if variant == Variant::Thermo {
                        assert!(!p.thermos.is_empty());
                    }
                }
            }
        }
    }
    #[test]
    fn seed_is_reproducible_and_new_seeds_produce_new_games() {
        for v in Variant::ALL {
            let a = Generator::new(81, v, Difficulty::Medium).finish();
            let b = Generator::new(81, v, Difficulty::Medium).finish();
            let c = Generator::new(82, v, Difficulty::Medium).finish();
            assert_eq!(a.givens, b.givens);
            assert_eq!(a.solution, b.solution);
            assert_ne!(a.solution, c.solution);
        }
    }
    #[test]
    fn difficulty_removes_more_clues_and_expands_allowed_logic() {
        for v in Variant::ALL {
            let puzzles = Difficulty::ALL.map(|d| Generator::new(123, v, d).finish());
            let counts = puzzles
                .each_ref()
                .map(|p| p.givens.iter().filter(|&&v| v > 0).count());
            assert!(counts[0] > counts[1], "{v:?}: {counts:?}");
            assert!(counts[1] > counts[2], "{v:?}: {counts:?}");
            for p in puzzles {
                assert!(logical_solve(&p, &p.givens, p.difficulty.level()).solved);
            }
        }
    }
    #[test]
    fn solver_rejects_duplicates_and_bad_variant_constraints() {
        for v in Variant::ALL {
            let p = Generator::new(11, v, Difficulty::Easy).finish();
            let mut wrong = p.solution.clone();
            wrong[0] = wrong[1];
            assert!(!p.complete_valid(&wrong));
            assert!(!logical_solve(&p, &wrong, 2).valid);
            assert_eq!(solution_count(&p, &wrong, 10), Some(0));
        }
        for r in [
            Relation::Five,
            Relation::Ten,
            Relation::Consecutive,
            Relation::Double,
        ] {
            assert!(!r.accepts(9, 9));
        }
    }
    #[test]
    fn marked_edges_do_not_impose_negative_constraints() {
        let mut p = Generator::new(7, Variant::Classic, Difficulty::Easy).finish();
        p.variant = Variant::Kropki;
        assert!(p.complete_valid(&p.solution));
        p.variant = Variant::Xv;
        assert!(p.complete_valid(&p.solution));
    }
    #[test]
    fn killer_cages_are_connected_cover_grid_and_forbid_repeats() {
        let mut p = Generator::new(7, Variant::Killer, Difficulty::Hard).finish();
        assert!(p.validated());
        let c = p.cages.iter().find(|c| c.cells.len() > 1).unwrap();
        let mut wrong = p.solution.clone();
        wrong[c.cells[0]] = wrong[c.cells[1]];
        assert!(!p.complete_valid(&wrong));
        p.cages[0].cells = vec![0, 80];
        assert!(!p.validated());
    }
    fn game() -> Game {
        Game::new(Generator::new(25, Variant::Classic, Difficulty::Medium).finish())
    }
    #[test]
    fn givens_are_immutable_and_notes_and_colours_have_independent_history() {
        let mut g = game();
        let i = g.puzzle.givens.iter().position(|&v| v == 0).unwrap();
        let given = g.puzzle.givens.iter().position(|&v| v > 0).unwrap();
        assert!(!g.enter(&[given], 1, Tool::Digit));
        assert!(!g.erase(&[given], Tool::Digit));
        assert!(g.enter(&[i], 3, Tool::Corner));
        assert!(g.enter(&[i], 4, Tool::Centre));
        assert!(g.enter(&[i, given], 2, Tool::Colour));
        assert_eq!(g.marks[i].corner, bit(3));
        assert_eq!(g.marks[i].centre, bit(4));
        assert!(g.enter(&[i], 5, Tool::Digit));
        assert_eq!(
            g.marks[i],
            Mark {
                value: 5,
                colour: 2,
                ..Mark::default()
            }
        );
        g.undo();
        assert_eq!(g.marks[i].corner, bit(3));
        g.redo();
        assert_eq!(g.marks[i].value, 5);
        assert_eq!(g.values()[given], g.puzzle.givens[given]);
    }
    #[test]
    fn grouped_notes_toggle_as_one_action_and_new_actions_clear_redo() {
        let mut g = game();
        let cells: Vec<_> = g
            .puzzle
            .givens
            .iter()
            .enumerate()
            .filter_map(|(i, &d)| (d == 0).then_some(i))
            .take(3)
            .collect();
        g.enter(&cells, 7, Tool::Corner);
        assert!(cells.iter().all(|&i| g.marks[i].corner == bit(7)));
        g.undo();
        assert!(cells.iter().all(|&i| g.marks[i].corner == 0));
        g.redo();
        g.enter(&cells, 7, Tool::Corner);
        assert!(cells.iter().all(|&i| g.marks[i].corner == 0));
        g.undo();
        g.enter(&cells, 4, Tool::Centre);
        assert!(!g.redo());
    }
    #[test]
    fn logical_hints_finish_each_variant_and_wins_validate_every_rule() {
        for v in Variant::ALL {
            let mut g = Game::new(Generator::new(14, v, Difficulty::Hard).finish());
            for _ in 0..81 {
                if g.won() {
                    break;
                }
                assert!(g.hint().is_some(), "{v:?}");
            }
            assert!(g.won());
            assert!(g.hint().is_none());
            let before = g.marks.clone();
            g.reset();
            assert!(!g.won());
            g.undo();
            assert_eq!(g.marks, before);
            assert!(g.won());
        }
    }
    #[test]
    fn hint_reports_wrong_digit_without_silently_replacing_it() {
        let mut g = game();
        let i = g.puzzle.givens.iter().position(|&d| d == 0).unwrap();
        let wrong = g.puzzle.solution[i] % 9 + 1;
        g.enter(&[i], wrong, Tool::Digit);
        let before = g.marks.clone();
        assert_eq!(g.hint().unwrap().0, i);
        assert_eq!(g.marks, before);
        assert_eq!(g.mistakes(), vec![i]);
    }
    #[test]
    fn offline_save_restores_progress_and_history_and_rejects_corruption() {
        let mut g = game();
        let i = g.puzzle.givens.iter().position(|&d| d == 0).unwrap();
        g.enter(&[i], 3, Tool::Corner);
        g.enter(&[i], 2, Tool::Centre);
        g.enter(&[i], 1, Tool::Colour);
        g.undo();
        let data = g.encode();
        let mut restored = Game::restore(&data).unwrap();
        assert_eq!(restored.encode(), data);
        assert!(restored.redo());
        assert_eq!(restored.marks[i].colour, 1);
        let mut json: serde_json::Value = serde_json::from_str(&data).unwrap();
        json["game"]["puzzle"]["givens"][0] = serde_json::json!(99);
        assert!(Game::restore(&json.to_string()).is_none());
        assert!(Game::restore("{}").is_none());
        assert!(Game::restore(&"x".repeat(300_000)).is_none());
    }
    #[test]
    fn full_history_fits_storage_buffer_and_remains_undoable_after_reload() {
        let mut g = game();
        let cells: Vec<_> = (0..81).collect();
        for d in 1..=9 {
            g.enter(&cells, d, Tool::Corner);
            g.enter(&cells, d, Tool::Centre);
        }
        for n in 0..130 {
            g.enter(&cells, n % 9 + 1, Tool::Colour);
        }
        let data = g.encode();
        assert!(data.len() < 262_144);
        let mut restored = Game::restore(&data).unwrap();
        for _ in 0..100 {
            assert!(restored.undo());
        }
        assert!(!restored.undo());
        for _ in 0..100 {
            assert!(restored.redo());
        }
        assert!(!restored.redo());
        assert_eq!(restored.marks, g.marks);
    }
    #[test]
    fn impossible_and_ambiguous_puzzles_are_not_accepted() {
        let mut p = Generator::new(1, Variant::Classic, Difficulty::Easy).finish();
        p.givens.fill(0);
        assert!(!p.validated());
        assert_eq!(solution_count(&p, &p.givens, 1000), Some(2));
        p.givens[0] = 1;
        p.givens[1] = 1;
        assert_eq!(solution_count(&p, &p.givens, 1000), Some(0));
    }
}
