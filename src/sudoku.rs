//! Offline Sudoku: original generated grids, variant constraints and pencil marks.
mod constraints;
mod generator;
pub use constraints::{Line, LineKind, Sandwich};
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
    Arrow,
    Renban,
    Whispers,
    RegionSum,
    Palindrome,
    Between,
    Entropic,
    Sandwich,
    AntiKnight,
    AntiKing,
    NonConsecutive,
    Miracle,
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
    pub const OPTIONS: [Self; 18] = [
        Self::Classic,
        Self::Killer,
        Self::Xv,
        Self::Kropki,
        Self::Thermo,
        Self::Diagonal,
        Self::Arrow,
        Self::Renban,
        Self::Whispers,
        Self::RegionSum,
        Self::Palindrome,
        Self::Between,
        Self::Entropic,
        Self::Sandwich,
        Self::AntiKnight,
        Self::AntiKing,
        Self::NonConsecutive,
        Self::Miracle,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Killer => "Killer",
            Self::Xv => "V & X",
            Self::Kropki => "Kropki",
            Self::Thermo => "Thermo",
            Self::Diagonal => "Diagonal",
            Self::Arrow => "Arrow",
            Self::Renban => "Renban",
            Self::Whispers => "Whispers",
            Self::RegionSum => "Region sum",
            Self::Palindrome => "Palindrome",
            Self::Between => "Between",
            Self::Entropic => "Entropic",
            Self::Sandwich => "Sandwich",
            Self::AntiKnight => "Anti-knight",
            Self::AntiKing => "Anti-king",
            Self::NonConsecutive => "Non-consecutive",
            Self::Miracle => "Miracle",
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
            _ => "More ways to reason",
        }
    }
    pub fn rules(self) -> Vec<String> {
        vec![Rules::from_variant(self).explanation()]
    }
}
/// Independent constraints. An absent rules field keeps legacy single-mode saves intact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClueMode {
    #[default]
    Off,
    Partial,
    Full,
}
impl ClueMode {
    pub fn enabled(self) -> bool {
        self != Self::Off
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Partial => "Partial",
            Self::Full => "Full",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rules {
    pub killer: bool,
    pub xv: ClueMode,
    pub kropki: ClueMode,
    pub thermo: bool,
    pub diagonal: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub arrow: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub renban: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub whispers: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub region_sum: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub palindrome: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub between: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub entropic: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub sandwich: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub anti_knight: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub anti_king: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub non_consecutive: bool,
}
impl Rules {
    pub fn from_variant(v: Variant) -> Self {
        let mut rules = Self::default();
        rules.toggle(v);
        rules
    }
    pub fn contains(self, v: Variant) -> bool {
        match v {
            Variant::Classic => self == Self::default(),
            Variant::Killer => self.killer,
            Variant::Xv => self.xv.enabled(),
            Variant::Kropki => self.kropki.enabled(),
            Variant::Thermo => self.thermo,
            Variant::Diagonal => self.diagonal,
            Variant::Arrow => self.arrow,
            Variant::Renban => self.renban,
            Variant::Whispers => self.whispers,
            Variant::RegionSum => self.region_sum,
            Variant::Palindrome => self.palindrome,
            Variant::Between => self.between,
            Variant::Entropic => self.entropic,
            Variant::Sandwich => self.sandwich,
            Variant::AntiKnight => self.anti_knight,
            Variant::AntiKing => self.anti_king,
            Variant::NonConsecutive => self.non_consecutive,
            Variant::Miracle => self.anti_knight && self.anti_king && self.non_consecutive,
        }
    }
    pub fn toggle(&mut self, v: Variant) {
        match v {
            Variant::Classic => *self = Self::default(),
            Variant::Killer => self.killer = !self.killer,
            Variant::Xv => {
                self.xv = if self.xv.enabled() {
                    ClueMode::Off
                } else {
                    ClueMode::Partial
                }
            }
            Variant::Kropki => {
                self.kropki = if self.kropki.enabled() {
                    ClueMode::Off
                } else {
                    ClueMode::Partial
                }
            }
            Variant::Thermo => self.thermo = !self.thermo,
            Variant::Diagonal => self.diagonal = !self.diagonal,
            Variant::Arrow => self.arrow = !self.arrow,
            Variant::Renban => self.renban = !self.renban,
            Variant::Whispers => self.whispers = !self.whispers,
            Variant::RegionSum => self.region_sum = !self.region_sum,
            Variant::Palindrome => self.palindrome = !self.palindrome,
            Variant::Between => self.between = !self.between,
            Variant::Entropic => self.entropic = !self.entropic,
            Variant::Sandwich => self.sandwich = !self.sandwich,
            Variant::AntiKnight => self.anti_knight = !self.anti_knight,
            Variant::AntiKing => self.anti_king = !self.anti_king,
            Variant::NonConsecutive => self.non_consecutive = !self.non_consecutive,
            Variant::Miracle => {
                let enabled = !self.contains(Variant::Miracle);
                self.anti_knight = enabled;
                self.anti_king = enabled;
                self.non_consecutive = enabled;
            }
        }
    }
    pub fn primary(self) -> Variant {
        if self == Self::from_variant(Variant::Miracle) {
            return Variant::Miracle;
        }
        Variant::OPTIONS
            .into_iter()
            .skip(1)
            .find(|&v| v != Variant::Miracle && self.contains(v))
            .unwrap_or(Variant::Classic)
    }
    pub fn name(self) -> String {
        let names: Vec<_> = Variant::OPTIONS
            .into_iter()
            .skip(1)
            .filter(|&v| {
                self.contains(v)
                    && (v == Variant::Miracle
                        || !self.contains(Variant::Miracle)
                        || !matches!(
                            v,
                            Variant::AntiKnight | Variant::AntiKing | Variant::NonConsecutive
                        ))
            })
            .map(|v| {
                let full = v == Variant::Xv && self.xv == ClueMode::Full
                    || v == Variant::Kropki && self.kropki == ClueMode::Full;
                format!("{}{}", v.name(), if full { " (full)" } else { "" })
            })
            .collect();
        if names.is_empty() {
            "Classic".into()
        } else {
            names.join(" + ")
        }
    }
    pub fn descriptions(self) -> Vec<String> {
        let mut lines = vec!["Rows, columns and 3 × 3 boxes contain 1–9 once."];
        if self.killer {
            lines.push("Cages sum to their total, without repeats.");
        }
        if self.xv.enabled() {
            lines.push("V pairs sum to 5; X pairs sum to 10.");
            lines.push(if self.xv == ClueMode::Full {
                "Full XV: an edge without V/X cannot sum to 5 or 10."
            } else {
                "Partial XV: unmarked edges have no XV restriction."
            });
        }
        if self.kropki.enabled() {
            lines.push("White dots: consecutive. Black dots: double. 1/2 may use either dot.");
            lines.push(if self.kropki == ClueMode::Full {
                "Full dots: an edge without a dot is neither consecutive nor double."
            } else {
                "Partial dots: unmarked edges have no dot restriction."
            });
        }
        if self.thermo {
            lines.push("Thermometers increase from bulb to tip.");
        }
        if self.diagonal {
            lines.push("Both long diagonals contain 1–9 once.");
        }
        if self.arrow {
            lines.push("Arrow shafts sum to the digit in their circle; repeats are allowed by normal rules.");
        }
        if self.renban {
            lines.push("Purple Renban lines contain distinct consecutive digits, in any order.");
        }
        if self.whispers {
            lines.push("Neighbours on green Whispers lines differ by at least 5.");
        }
        if self.region_sum {
            lines.push("Box boundaries split blue region-sum lines into segments with equal sums.");
        }
        if self.palindrome {
            lines.push("Grey palindrome lines read the same forwards and backwards.");
        }
        if self.between {
            lines.push("Digits on orange between lines lie strictly between their two endpoints.");
        }
        if self.entropic {
            lines.push("Every three neighbours on gold entropic lines include one digit from each of 1–3, 4–6 and 7–9.");
        }
        if self.sandwich {
            lines.push("Outside sandwich clues sum the digits strictly between 1 and 9 in that row or column.");
        }
        if self.anti_knight {
            lines.push("Equal digits cannot be a chess knight's move apart.");
        }
        if self.anti_king {
            lines.push("Equal digits cannot touch diagonally.");
        }
        if self.non_consecutive {
            lines.push("Orthogonal neighbours cannot be consecutive.");
        }
        lines.into_iter().map(str::to_owned).collect()
    }
    pub fn explanation(self) -> String {
        self.descriptions().join(" ")
    }
    pub(super) fn pair_kind(self, a: usize, b: usize) -> Option<bool> {
        let dr = (a / 9).abs_diff(b / 9);
        let dc = (a % 9).abs_diff(b % 9);
        if self.anti_knight && matches!((dr, dc), (1, 2) | (2, 1))
            || self.anti_king && dr == 1 && dc == 1
        {
            Some(false)
        } else if self.non_consecutive && dr + dc == 1 {
            Some(true)
        } else {
            None
        }
    }
    pub(super) fn global_pairs(self) -> &'static [(usize, usize, bool)] {
        static PAIRS: std::sync::OnceLock<[Vec<(usize, usize, bool)>; 8]> =
            std::sync::OnceLock::new();
        let sets = PAIRS.get_or_init(|| {
            std::array::from_fn(|flags| {
                let rules = Self {
                    anti_knight: flags & 1 != 0,
                    anti_king: flags & 2 != 0,
                    non_consecutive: flags & 4 != 0,
                    ..Self::default()
                };
                (0..81)
                    .flat_map(|a| {
                        (a + 1..81).filter_map(move |b| rules.pair_kind(a, b).map(|nc| (a, b, nc)))
                    })
                    .collect()
            })
        });
        &sets[usize::from(self.anti_knight)
            + 2 * usize::from(self.anti_king)
            + 4 * usize::from(self.non_consecutive)]
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
            Self::Hard => 3,
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub relation: Relation,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cage {
    pub cells: Vec<usize>,
    pub sum: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HardRating {
    pub stalled_cells: usize,
    pub advanced_steps: u32,
    pub forcing_steps: u32,
    pub longest_chain: u32,
}
impl HardRating {
    pub fn qualifies(&self) -> bool {
        self.stalled_cells >= 45
            && self.advanced_steps >= 5
            && self.forcing_steps >= 3
            && self.longest_chain >= 30
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Puzzle {
    pub seed: u64,
    pub variant: Variant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Rules>,
    pub difficulty: Difficulty,
    pub givens: Vec<u8>,
    pub solution: Vec<u8>,
    pub cages: Vec<Cage>,
    pub edges: Vec<Edge>,
    pub thermos: Vec<Vec<usize>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<Line>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sandwiches: Vec<Sandwich>,
    pub effort: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hard_rating: Option<HardRating>,
}
impl Puzzle {
    pub fn rules(&self) -> Rules {
        self.rules
            .unwrap_or_else(|| Rules::from_variant(self.variant))
    }
    pub fn name(&self) -> String {
        self.rules().name()
    }
    /// Negative constraints apply separately to XV and dots, even when both
    /// marks share an edge. A V is not a dot and a dot is not a V.
    pub(super) fn negative_pairs(&self) -> Vec<(usize, usize, bool, bool)> {
        let rules = self.rules();
        if rules.xv != ClueMode::Full && rules.kropki != ClueMode::Full {
            return Vec::new();
        }
        let mut marks = [[0u8; 2]; 81];
        for edge in &self.edges {
            let a = edge.a.min(edge.b);
            let direction = usize::from(edge.a.abs_diff(edge.b) == 9);
            if a < 81 {
                marks[a][direction] |= if matches!(edge.relation, Relation::Five | Relation::Ten) {
                    1
                } else {
                    2
                };
            }
        }
        let mut pairs = Vec::new();
        for (a, mark) in marks.iter().enumerate() {
            for (direction, &marked) in mark.iter().enumerate() {
                if direction == 0 && a % 9 == 8 || direction == 1 && a >= 72 {
                    continue;
                }
                let xv = rules.xv == ClueMode::Full && marked & 1 == 0;
                let dots = rules.kropki == ClueMode::Full && marked & 2 == 0;
                if xv || dots {
                    pairs.push((a, a + if direction == 0 { 1 } else { 9 }, xv, dots));
                }
            }
        }
        pairs
    }
    pub(super) fn unmarked_accepts(a: u8, b: u8, xv: bool, dots: bool) -> bool {
        (!xv || a + b != 5 && a + b != 10)
            && (!dots || a.abs_diff(b) != 1 && a != 2 * b && b != 2 * a)
    }

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
        if self.rules().diagonal {
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
        self.rules().global_pairs().iter().all(|&(a, b, nc)| {
            if nc {
                values[a].abs_diff(values[b]) != 1
            } else {
                values[a] != values[b]
            }
        }) && self.lines.iter().all(|l| l.valid(values))
            && self.sandwiches.iter().all(|c| c.valid(values))
            && self
                .negative_pairs()
                .iter()
                .all(|&(a, b, xv, dots)| Self::unmarked_accepts(values[a], values[b], xv, dots))
            && self
                .edges
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
        if self.cages.len() > 81 || self.edges.len() > 288 || self.thermos.len() > 12 {
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
        if self.rules.is_none() && self.variant == Variant::Killer && covered.contains(&false) {
            return false;
        }
        if !self.rules().killer && !self.cages.is_empty() {
            return false;
        }
        let mut seen = Vec::new();
        for e in &self.edges {
            if !adjacent(e.a, e.b)
                || seen.contains(&(
                    e.a.min(e.b),
                    e.a.max(e.b),
                    matches!(e.relation, Relation::Five | Relation::Ten),
                ))
            {
                return false;
            }
            let allowed = match e.relation {
                Relation::Five | Relation::Ten => self.rules().xv.enabled(),
                Relation::Consecutive | Relation::Double => self.rules().kropki.enabled(),
            };
            if !allowed {
                return false;
            }
            seen.push((
                e.a.min(e.b),
                e.a.max(e.b),
                matches!(e.relation, Relation::Five | Relation::Ten),
            ));
        }
        if !self.rules().thermo && !self.thermos.is_empty() {
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
        if self.lines.len() > 56
            || self.sandwiches.len() > 18
            || self
                .lines
                .iter()
                .any(|l| !l.valid_shape() || !self.rules().contains(l.kind.variant()))
        {
            return false;
        }
        let mut sandwich_seen = Vec::new();
        for c in &self.sandwiches {
            if !self.rules().sandwich
                || c.unit >= 18
                || c.sum > 35
                || sandwich_seen.contains(&c.unit)
            {
                return false;
            }
            sandwich_seen.push(c.unit);
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
    pub fn toggle_entry(self) -> Self {
        if self == Self::Digit {
            Self::Corner
        } else {
            Self::Digit
        }
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
    #[serde(default = "fresh_notes_available")]
    initial_notes_available: bool,
    #[serde(skip)]
    hint_cache: Option<(Vec<u8>, Vec<Deduction>)>,
}
fn fresh_notes_available() -> bool {
    true
}
impl Game {
    pub fn new(puzzle: Puzzle) -> Self {
        Self {
            puzzle,
            marks: vec![Mark::default(); 81],
            undo: Vec::new(),
            redo: Vec::new(),
            hints: 0,
            initial_notes_available: true,
            hint_cache: None,
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
    /// Visible givens and player digits only; never solution cells or pencil notes.
    pub fn revealed_cells(&self, number: u8) -> Vec<usize> {
        if !(1..=9).contains(&number) {
            return Vec::new();
        }
        self.values()
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| (v == number).then_some(i))
            .collect()
    }
    /// Player-marked possibilities in empty cells, from either pencil-note style.
    /// This is a visual search, not a deduction from the hidden solution.
    pub fn candidate_cells(&self, number: u8) -> Vec<usize> {
        if !(1..=9).contains(&number) {
            return Vec::new();
        }
        self.puzzle
            .givens
            .iter()
            .zip(&self.marks)
            .enumerate()
            .filter_map(|(i, (&given, mark))| {
                (given == 0 && mark.value == 0 && (mark.corner | mark.centre) & bit(number) != 0)
                    .then_some(i)
            })
            .collect()
    }
    /// A single distinct digit in the user's notes, regardless of its correctness.
    pub fn single_candidate(&self, cell: usize) -> Option<u8> {
        let mark = self.marks.get(cell)?;
        if *self.puzzle.givens.get(cell)? != 0 || mark.value != 0 {
            return None;
        }
        let notes = mark.corner | mark.centre;
        (notes.count_ones() == 1).then(|| notes.trailing_zeros() as u8 + 1)
    }
    /// One optional setup action, before any progress has been made.
    pub fn can_fill_classic_candidates(&self) -> bool {
        self.initial_notes_available
            && self.hints == 0
            && self.undo.is_empty()
            && self.redo.is_empty()
            && self.marks.iter().all(|m| *m == Mark::default())
    }
    /// Fill starting notes in every empty cell with simple classic candidates.
    /// Read only visible digits: no solution, variant constraints or deductions.
    pub fn fill_classic_candidates(&mut self) -> bool {
        if !self.can_fill_classic_candidates() {
            return false;
        }
        self.initial_notes_available = false;
        let values = self.values();
        let mut rows = [0u16; 9];
        let mut columns = [0u16; 9];
        let mut boxes = [0u16; 9];
        for (i, &value) in values.iter().enumerate() {
            rows[i / 9] |= bit(value);
            columns[i % 9] |= bit(value);
            boxes[(i / 27) * 3 + (i % 9) / 3] |= bit(value);
        }
        let before = self.marks.clone();
        for (i, mark) in self.marks.iter_mut().enumerate() {
            if values[i] == 0 {
                mark.corner = ALL_DIGITS
                    & !(rows[i / 9] | columns[i % 9] | boxes[(i / 27) * 3 + (i % 9) / 3]);
                mark.centre = 0;
            }
        }
        self.finish(before);
        true
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
        self.initial_notes_available = false;
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
    /// Mechanical cleanup only: no variant peers, deductions or new notes.
    fn clear_peer_notes(&mut self, cell: usize, number: u8) {
        let keep = !bit(number);
        for (i, mark) in self.marks.iter_mut().enumerate() {
            if i / 9 == cell / 9
                || i % 9 == cell % 9
                || (i / 27 == cell / 27 && i % 9 / 3 == cell % 9 / 3)
            {
                mark.corner &= keep;
                mark.centre &= keep;
            }
        }
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
            if tool == Tool::Digit && !remove {
                self.clear_peer_notes(i, number);
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
        let values = self.values();
        let cached = self
            .hint_cache
            .as_ref()
            .is_some_and(|(snapshot, steps)| *snapshot == values && !steps.is_empty());
        if !cached {
            let result =
                logical_solve(&self.puzzle, &values, self.puzzle.difficulty.level().max(2));
            self.hint_cache = Some((values, result.steps));
        }
        let steps = &mut self.hint_cache.as_mut()?.1;
        if steps.is_empty() {
            return None;
        }
        let step = steps.remove(0);
        let before = self.marks.clone();
        self.marks[step.cell].value = step.value;
        self.marks[step.cell].corner = 0;
        self.marks[step.cell].centre = 0;
        self.clear_peer_notes(step.cell, step.value);
        self.finish(before);
        self.hints += 1;
        let values = self.values();
        if let Some((snapshot, _)) = &mut self.hint_cache {
            *snapshot = values;
        }
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
        // Older saves lack the flag: preserve untouched puzzles, but never
        // reopen setup after progress or an undo, even if the board is empty.
        g.initial_notes_available = g.can_fill_classic_candidates();
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
    fn additional_variants_have_unique_logical_puzzles_at_every_difficulty() {
        for variant in Variant::OPTIONS.into_iter().skip(6) {
            for difficulty in Difficulty::ALL {
                let p = Generator::new(17, variant, difficulty)
                    .try_finish()
                    .unwrap();
                assert!(p.validated(), "{variant:?} {difficulty:?}");
                let proof = logical_solve(&p, &p.givens, difficulty.level());
                assert!(proof.solved);
                assert_eq!(solution_count(&p, &p.givens, 10_000), Some(1));
                assert_eq!(
                    proof.masks.iter().map(|&m| digit(m)).collect::<Vec<_>>(),
                    p.solution
                );
                for kind in LineKind::ALL {
                    if p.rules().contains(kind.variant()) {
                        assert!(p.lines.iter().any(|l| l.kind == kind));
                    }
                }
                if p.rules().sandwich {
                    assert!(!p.sandwiches.is_empty());
                }
                if difficulty == Difficulty::Hard {
                    assert!(p.hard_rating.as_ref().unwrap().qualifies());
                    assert!(
                        logical_solve(&p, &p.givens, 2)
                            .masks
                            .iter()
                            .filter(|m| m.count_ones() > 1)
                            .count()
                            >= 45
                    );
                }
                let mut hidden = p.clone();
                hidden.solution.fill(9);
                assert_eq!(
                    logical_solve(&hidden, &p.givens, difficulty.level()).masks,
                    proof.masks
                );
                let saved = Game::new(p).encode();
                assert_eq!(Game::restore(&saved).unwrap().encode(), saved);
            }
        }
    }
    #[test]
    fn lines_outside_clues_and_old_rules_combine_without_hidden_answers() {
        for rules in [
            Rules {
                arrow: true,
                killer: true,
                diagonal: true,
                xv: ClueMode::Partial,
                ..Rules::default()
            },
            Rules {
                renban: true,
                sandwich: true,
                whispers: true,
                ..Rules::default()
            },
            Rules {
                arrow: true,
                renban: true,
                whispers: true,
                region_sum: true,
                palindrome: true,
                between: true,
                entropic: true,
                ..Rules::default()
            },
            Rules {
                anti_knight: true,
                thermo: true,
                ..Rules::default()
            },
        ] {
            let p = Generator::with_rules(91, rules, Difficulty::Medium)
                .try_finish()
                .unwrap();
            assert_eq!(p.rules(), rules);
            assert!(p.validated());
            assert_eq!(solution_count(&p, &p.givens, 10_000), Some(1));
            let mut g = Game::new(p);
            assert!(g.fill_classic_candidates());
            let values = g.values();
            for i in 0..81 {
                if values[i] != 0 {
                    continue;
                }
                let expected = (1..=9)
                    .filter(|&d| {
                        !values.iter().enumerate().any(|(j, &v)| {
                            v == d
                                && (i / 9 == j / 9
                                    || i % 9 == j % 9
                                    || i / 27 == j / 27 && i % 9 / 3 == j % 9 / 3)
                        })
                    })
                    .fold(0, |m, d| m | bit(d));
                assert_eq!(g.marks[i].corner, expected);
            }
            let (i, _) = g.hint().unwrap();
            assert_eq!(g.marks[i].value, g.puzzle.solution[i]);
        }
    }
    #[test]
    fn new_save_constraints_reject_malformed_shapes_and_keep_legacy_json() {
        let p = Generator::new(17, Variant::Arrow, Difficulty::Easy).finish();
        for cells in [
            vec![],
            vec![0, 1, 81],
            vec![0, 1, 1],
            vec![0, 1, 8],
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
        ] {
            let mut bad = p.clone();
            bad.lines[0].cells = cells;
            assert!(Game::restore(&Game::new(bad).encode()).is_none());
        }
        let mut bad = p;
        bad.rules.as_mut().unwrap().arrow = false;
        assert!(!bad.validated());
        let p = Generator::new(17, Variant::Sandwich, Difficulty::Easy).finish();
        for (unit, sum) in [(18, 0), (0, 36)] {
            let mut bad = p.clone();
            bad.sandwiches[0] = Sandwich { unit, sum };
            assert!(!bad.validated());
        }
        let mut bad = p;
        bad.sandwiches.push(bad.sandwiches[0].clone());
        assert!(!bad.validated());
        let old = Game::new(
            Generator::with_rules(
                17,
                Rules {
                    diagonal: true,
                    xv: ClueMode::Partial,
                    ..Rules::default()
                },
                Difficulty::Easy,
            )
            .finish(),
        )
        .encode();
        assert!(!old.contains("anti_knight"));
        assert!(!old.contains("sandwiches"));
        assert!(!old.contains("\"lines\""));
        assert_eq!(Game::restore(&old).unwrap().encode(), old);
    }
    #[test]
    fn miracle_is_a_preset_and_global_completion_search_remains_bounded() {
        let mut r = Rules::default();
        r.toggle(Variant::Miracle);
        assert!(r.anti_knight && r.anti_king && r.non_consecutive);
        assert_eq!(r.name(), "Miracle");
        assert_eq!(r.primary(), Variant::Miracle);
        r.toggle(Variant::AntiKing);
        assert!(!r.contains(Variant::Miracle));
        r.toggle(Variant::Miracle);
        assert_eq!(r.name(), "Miracle");
        r.toggle(Variant::Miracle);
        assert_eq!(r, Rules::default());
        let rules = Rules {
            diagonal: true,
            anti_knight: true,
            ..Rules::default()
        };
        let mut generator = Generator::with_rules(17, rules, Difficulty::Easy);
        // Filling a constrained solution is resumable from the first step.
        assert!(!generator.step());
        let mut steps = 1;
        while !generator.step() {
            steps += 1;
            assert!(steps < 2_000_000);
        }
        if let Ok(p) = generator.try_finish() {
            assert!(p.validated());
            assert_eq!(p.rules(), rules);
        }
        for seed in [0, 1, 91] {
            let p = Generator::new(seed, Variant::Miracle, Difficulty::Easy).finish();
            assert!(p.complete_valid(&p.solution));
            assert_eq!(
                p.solution,
                Generator::new(seed, Variant::Miracle, Difficulty::Easy)
                    .finish()
                    .solution
            );
        }
    }
    #[test]
    fn combined_rules_generate_unique_puzzles_at_all_supported_clue_settings() {
        for killer in [false, true] {
            for xv in [ClueMode::Off, ClueMode::Partial, ClueMode::Full] {
                for kropki in [ClueMode::Off, ClueMode::Partial, ClueMode::Full] {
                    for thermo in [false, true] {
                        for diagonal in [false, true] {
                            let rules = Rules {
                                killer,
                                xv,
                                kropki,
                                thermo,
                                diagonal,
                                ..Rules::default()
                            };
                            for difficulty in [Difficulty::Easy, Difficulty::Medium] {
                                let p = Generator::with_rules(17, rules, difficulty)
                                    .try_finish()
                                    .unwrap();
                                assert_eq!(p.rules(), rules);
                                assert!(p.validated(), "{rules:?} {difficulty:?}");
                                assert_eq!(solution_count(&p, &p.givens, 10_000), Some(1));
                                let saved = Game::new(p.clone()).encode();
                                assert_eq!(Game::restore(&saved).unwrap().encode(), saved);
                                let mut unseen = p.clone();
                                unseen.solution.fill(9);
                                assert_eq!(
                                    logical_solve(&unseen, &p.givens, difficulty.level()).masks,
                                    logical_solve(&p, &p.givens, difficulty.level()).masks
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn full_markings_enforce_absence_separately_and_allow_either_dot_for_one_two() {
        let rules = Rules {
            xv: ClueMode::Full,
            kropki: ClueMode::Full,
            ..Rules::default()
        };
        let p = Generator::with_rules(17, rules, Difficulty::Easy).finish();
        assert!(p.complete_valid(&p.solution));
        let overlap = p
            .edges
            .iter()
            .find(|e| {
                matches!(e.relation, Relation::Five | Relation::Ten)
                    && p.edges.iter().any(|d| {
                        d.a == e.a
                            && d.b == e.b
                            && matches!(d.relation, Relation::Consecutive | Relation::Double)
                    })
            })
            .unwrap();
        for xv in [false, true] {
            let mut missing = p.clone();
            missing.edges.retain(|e| {
                !(e.a == overlap.a
                    && e.b == overlap.b
                    && matches!(e.relation, Relation::Five | Relation::Ten) == xv)
            });
            assert!(!missing.complete_valid(&p.solution));
            assert!(!logical_solve(&missing, &p.solution, 0).valid);
            if xv {
                missing.rules.as_mut().unwrap().xv = ClueMode::Partial;
            } else {
                missing.rules.as_mut().unwrap().kropki = ClueMode::Partial;
            }
            assert!(missing.complete_valid(&p.solution));
        }
        let mut white = p.clone();
        let e = white
            .edges
            .iter_mut()
            .find(|e| {
                matches!((p.solution[e.a], p.solution[e.b]), (1, 2) | (2, 1))
                    && e.relation == Relation::Double
            })
            .unwrap();
        e.relation = Relation::Consecutive;
        assert!(white.complete_valid(&white.solution));
        assert!(white.validated());
        white.edges.push(white.edges.last().unwrap().clone());
        assert!(!white.validated());
    }
    #[test]
    fn sparse_hard_combinations_keep_the_expert_floor_and_prune_optional_clues() {
        let cases = [
            Rules::default(),
            Rules::from_variant(Variant::Killer),
            Rules::from_variant(Variant::Xv),
            Rules::from_variant(Variant::Kropki),
            Rules::from_variant(Variant::Thermo),
            Rules::from_variant(Variant::Diagonal),
            Rules {
                killer: true,
                thermo: true,
                ..Rules::default()
            },
            Rules {
                xv: ClueMode::Partial,
                kropki: ClueMode::Full,
                ..Rules::default()
            },
            Rules {
                xv: ClueMode::Partial,
                diagonal: true,
                ..Rules::default()
            },
            Rules {
                xv: ClueMode::Full,
                diagonal: true,
                ..Rules::default()
            },
            Rules {
                kropki: ClueMode::Full,
                ..Rules::default()
            },
            Rules {
                killer: true,
                xv: ClueMode::Partial,
                kropki: ClueMode::Partial,
                thermo: true,
                diagonal: true,
                ..Rules::default()
            },
        ];
        for rules in cases {
            let p = Generator::with_rules(17, rules, Difficulty::Hard)
                .try_finish()
                .unwrap();
            assert!(p.validated());
            assert!(p.hard_rating.as_ref().unwrap().qualifies());
            assert_eq!(solution_count(&p, &p.givens, 10_000), Some(1));
            for i in 0..81 {
                if p.givens[i] > 0 {
                    let mut less = p.clone();
                    less.givens[i] = 0;
                    assert_ne!(
                        solution_count(&less, &less.givens, 10_000),
                        Some(1),
                        "Remove unnecessary digit {i}"
                    );
                }
            }
            if rules.killer {
                assert!(!p.cages.is_empty());
                assert!(p.cages.iter().map(|c| c.cells.len()).sum::<usize>() < 81);
            }
            if rules.thermo {
                assert!(!p.thermos.is_empty());
                assert!(p.thermos.len() < 7);
            }
            let encoded = Game::new(p).encode();
            assert_eq!(Game::restore(&encoded).unwrap().encode(), encoded);
        }
    }
    #[test]
    fn overinformative_hard_combinations_stop_without_an_easy_fallback() {
        let rules = Rules {
            xv: ClueMode::Full,
            kropki: ClueMode::Full,
            ..Rules::default()
        };
        let mut generator = Generator::with_rules(17, rules, Difficulty::Hard);
        let mut steps = 0;
        while !generator.step() {
            steps += 1;
            assert!(steps < 10_000_000);
        }
        assert!(generator.attempts() <= 256);
        match generator.try_finish() {
            Ok(p) => assert!(p.hard_rating.as_ref().unwrap().qualifies()),
            Err(message) => assert!(message.contains("partial")),
        }
    }
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
    fn every_new_hard_puzzle_has_a_real_advanced_logic_floor() {
        for variant in Variant::ALL {
            for seed in [1, 17, 333] {
                let p = Generator::new(seed, variant, Difficulty::Hard).finish();
                let simple = logical_solve(&p, &p.givens, 2);
                let proof = logical_solve(&p, &p.givens, 3);
                assert!(proof.solved, "{variant:?} {seed}");
                assert!(
                    !simple.solved,
                    "Hard cannot be solved by the previous techniques"
                );
                assert!(simple.masks.iter().filter(|m| m.count_ones() > 1).count() >= 45);
                assert!(proof.advanced_steps >= 5);
                assert!(proof.forcing_steps >= 3);
                assert!(proof.longest_chain >= 30);
                let rating = p.hard_rating.as_ref().unwrap();
                assert!(rating.qualifies());
                assert_eq!(rating.advanced_steps, proof.advanced_steps);
                assert_eq!(rating.forcing_steps, proof.forcing_steps);
                assert_eq!(rating.longest_chain, proof.longest_chain);
                assert_eq!(p.seed, seed);
                assert_eq!(
                    proof.masks.iter().map(|&m| digit(m)).collect::<Vec<_>>(),
                    p.solution
                );
                assert_eq!(solution_count(&p, &p.givens, 10_000), Some(1));
                let mut changed = p.clone();
                changed.solution.fill(9);
                let independently = logical_solve(&changed, &changed.givens, 3);
                assert_eq!(
                    independently.masks, proof.masks,
                    "Never consult the stored answer"
                );
            }
        }
    }
    #[test]
    fn advanced_logic_never_eliminates_a_valid_completion_of_sparse_variant_grids() {
        for variant in Variant::ALL {
            for seed in 0..8 {
                let mut p = Generator::new(seed, variant, Difficulty::Easy).finish();
                for i in 0..81 {
                    if !(i as u64 + seed).is_multiple_of(4) {
                        p.givens[i] = 0;
                    }
                }
                let result = logical_solve(&p, &p.givens, 3);
                assert!(result.valid, "{variant:?} {seed}");
                for (&mask, &value) in result.masks.iter().zip(&p.solution) {
                    assert_ne!(mask & bit(value), 0, "{variant:?} {seed}");
                }
                if result.solved {
                    assert_eq!(solution_count(&p, &p.givens, 10_000), Some(1));
                }
            }
        }
    }
    #[test]
    fn resumable_uniqueness_rejects_multiple_solutions_and_exhausted_budgets() {
        use super::solver::UniquenessProof;
        let p = Generator::new(8, Variant::Classic, Difficulty::Easy).finish();
        let mut proof = UniquenessProof::new(p.clone(), 10_000);
        while !proof.step() {}
        assert_eq!(proof.count(), Some(1));
        let mut exhausted = UniquenessProof::new(p.clone(), 0);
        while !exhausted.step() {}
        assert_eq!(exhausted.count(), None);
        let mut blank = p;
        blank.givens.fill(0);
        let mut proof = UniquenessProof::new(blank, 10_000);
        while !proof.step() {}
        assert_eq!(proof.count(), Some(2));
    }
    #[test]
    fn hard_hints_cache_only_the_current_visible_state_and_survive_edits() {
        let p = Generator::new(1, Variant::Classic, Difficulty::Hard).finish();
        let mut g = Game::new(p);
        let (cell, _) = g.hint().unwrap();
        assert!(g.hint_cache.as_ref().unwrap().1.len() > 1);
        assert_eq!(g.hint_cache.as_ref().unwrap().0, g.values());
        let save = g.encode();
        assert!(!save.contains("hint_cache"));
        let mut restored = Game::restore(&save).unwrap();
        assert!(restored.hint_cache.is_none());
        let next = g.hint().unwrap().0;
        assert_eq!(restored.hint().unwrap().0, next);
        g.undo();
        assert_eq!(g.hint().unwrap().0, next);
        g.enter(&[cell], g.puzzle.solution[cell] % 9 + 1, Tool::Digit);
        let before = g.marks.clone();
        assert_eq!(g.hint().unwrap().0, cell);
        assert_eq!(g.marks, before);
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
    fn entering_digits_clears_both_note_styles_in_classic_peers_only() {
        for variant in Variant::ALL {
            for number in 1..=9 {
                let mut g = game();
                g.puzzle.variant = variant;
                g.puzzle.givens.fill(0);
                // Variant-only peers (diagonal r5c5, a cage and a thermometer)
                // must not affect this mechanical row/column/box cleanup.
                g.puzzle.cages = vec![Cage {
                    cells: vec![0, 40],
                    sum: 10,
                }];
                g.puzzle.thermos = vec![vec![0, 40]];
                g.marks.fill(Mark {
                    corner: ALL_DIGITS,
                    centre: ALL_DIGITS,
                    colour: 4,
                    value: 0,
                });
                let before = g.marks.clone();
                assert!(g.enter(&[0], number, Tool::Digit));
                assert_eq!(g.undo.len(), 1);
                for i in 0..81 {
                    let peer = i / 9 == 0 || i % 9 == 0 || (i / 27 == 0 && i % 9 / 3 == 0);
                    let expected = if i == 0 {
                        0
                    } else if peer {
                        ALL_DIGITS & !bit(number)
                    } else {
                        ALL_DIGITS
                    };
                    assert_eq!(
                        g.marks[i].corner, expected,
                        "{variant:?} digit {number} cell {i}"
                    );
                    assert_eq!(g.marks[i].centre, expected);
                    assert_eq!(g.marks[i].colour, 4);
                    assert_eq!(g.marks[i].value, if i == 0 { number } else { 0 });
                }
                let after = g.marks.clone();
                assert!(g.undo());
                assert_eq!(g.marks, before);
                assert!(g.redo());
                assert_eq!(g.marks, after);
            }
        }
    }
    #[test]
    fn note_cleanup_groups_multi_cell_entries_and_handles_replacement_and_removal() {
        let mut g = game();
        g.puzzle.givens.fill(0);
        g.marks.fill(Mark {
            corner: ALL_DIGITS,
            centre: ALL_DIGITS,
            ..Mark::default()
        });
        let before = g.marks.clone();
        assert!(g.enter(&[0, 40, 80, 40, 81], 5, Tool::Digit));
        assert_eq!(g.undo.len(), 1);
        for i in 0..81 {
            let selected = [0, 40, 80].contains(&i);
            let peer = [0, 40, 80].iter().any(|&j| {
                i / 9 == j / 9 || i % 9 == j % 9 || (i / 27 == j / 27 && i % 9 / 3 == j % 9 / 3)
            });
            let expected = if selected {
                0
            } else if peer {
                ALL_DIGITS & !bit(5)
            } else {
                ALL_DIGITS
            };
            assert_eq!(g.marks[i].corner, expected);
            assert_eq!(g.marks[i].centre, expected);
        }
        let after = g.marks.clone();
        g.undo();
        assert_eq!(g.marks, before);
        g.redo();
        assert_eq!(g.marks, after);
        // Removing a digit never recreates notes or performs more eliminations.
        assert!(g.enter(&[0, 40, 80], 5, Tool::Digit));
        for (a, b) in g.marks.iter().zip(&after) {
            assert_eq!((a.corner, a.centre), (b.corner, b.centre));
        }
        g.enter(&[0], 5, Tool::Digit);
        assert!(g.enter(&[0], 6, Tool::Digit));
        assert_eq!(g.marks[1].corner, ALL_DIGITS & !(bit(5) | bit(6)));
        assert_eq!(g.marks[1].centre, ALL_DIGITS & !(bit(5) | bit(6)));
    }
    #[test]
    fn notes_and_colours_do_not_clean_peers_and_givens_and_lookup_are_inert() {
        let mut g = game();
        let given = g.puzzle.givens.iter().position(|&v| v != 0).unwrap();
        let i = g.puzzle.givens.iter().position(|&v| v == 0).unwrap();
        g.marks[i].corner = ALL_DIGITS;
        g.marks[i].centre = ALL_DIGITS;
        let before = g.marks.clone();
        assert!(!g.enter(&[], 5, Tool::Digit));
        assert!(!g.enter(&[given, 81], 5, Tool::Digit));
        assert_eq!(g.marks, before);
        for tool in [Tool::Corner, Tool::Centre, Tool::Colour] {
            let before = g.marks.clone();
            g.enter(&[i], 5, tool);
            for (j, mark) in before.iter().enumerate() {
                if j != i {
                    assert_eq!(g.marks[j], *mark);
                }
            }
        }
    }
    #[test]
    fn hints_clean_notes_in_one_saved_action_with_undo_and_redo() {
        for variant in Variant::ALL {
            let mut g = Game::new(Generator::new(51, variant, Difficulty::Medium).finish());
            let blanks: Vec<_> = (0..81).filter(|&i| g.puzzle.givens[i] == 0).collect();
            for number in 1..=9 {
                g.enter(&blanks, number, Tool::Corner);
                g.enter(&blanks, number, Tool::Centre);
            }
            let before = g.marks.clone();
            let count = g.undo.len();
            let (cell, _) = g.hint().unwrap();
            let number = g.marks[cell].value;
            for &i in &blanks {
                let peer = i / 9 == cell / 9
                    || i % 9 == cell % 9
                    || (i / 27 == cell / 27 && i % 9 / 3 == cell % 9 / 3);
                let expected = if i == cell {
                    0
                } else if peer {
                    ALL_DIGITS & !bit(number)
                } else {
                    ALL_DIGITS
                };
                assert_eq!(g.marks[i].corner, expected);
                assert_eq!(g.marks[i].centre, expected);
            }
            assert_eq!(g.undo.len(), count + 1);
            let after = g.marks.clone();
            let mut restored = Game::restore(&g.encode()).unwrap();
            assert_eq!(restored.marks, after);
            restored.undo();
            assert_eq!(restored.marks, before);
            restored.redo();
            assert_eq!(restored.marks, after);
        }
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
    fn revealed_digit_search_includes_visible_entries_but_never_hidden_digits_or_notes() {
        let mut g = game();
        let cells: Vec<_> = (0..81).filter(|&i| g.puzzle.givens[i] == 0).collect();
        let hidden = *cells.iter().find(|&&i| g.puzzle.solution[i] == 1).unwrap();
        let wrong = *cells.iter().find(|&&i| g.puzzle.solution[i] != 1).unwrap();
        g.enter(&[hidden], 1, Tool::Corner);
        g.enter(&[hidden], 1, Tool::Centre);
        g.enter(&[wrong], 1, Tool::Digit);
        let matches = g.revealed_cells(1);
        assert!(matches.contains(&wrong));
        assert!(!matches.contains(&hidden));
        assert_eq!(
            matches,
            (0..81)
                .filter(|&i| g.puzzle.givens[i] == 1 || i == wrong)
                .collect::<Vec<_>>()
        );
        g.enter(&[hidden], 1, Tool::Digit);
        assert!(g.revealed_cells(1).contains(&hidden));
        assert!(g.revealed_cells(0).is_empty());
        assert!(g.revealed_cells(10).is_empty());
    }
    #[test]
    fn candidate_search_combines_both_note_styles_without_revealing_or_editing_answers() {
        let mut g = game();
        let cells: Vec<_> = (0..81).filter(|&i| g.puzzle.givens[i] == 0).collect();
        g.enter(&[cells[0], cells[2]], 2, Tool::Corner);
        g.enter(&[cells[1], cells[2]], 2, Tool::Centre);
        g.enter(&[cells[3]], 3, Tool::Centre);
        g.enter(&[cells[4]], 2, Tool::Colour);
        let before = g.encode();
        assert_eq!(g.candidate_cells(2), cells[..3]);
        assert_eq!(g.candidate_cells(3), vec![cells[3]]);
        assert!(g.candidate_cells(0).is_empty());
        assert!(g.candidate_cells(10).is_empty());
        assert_eq!(g.encode(), before);
        let mut restored = Game::restore(&before).unwrap();
        assert_eq!(restored.candidate_cells(2), cells[..3]);
        restored.enter(&[cells[2]], 2, Tool::Digit);
        let remaining: Vec<_> = cells[..2]
            .iter()
            .copied()
            .filter(|&i| {
                i / 9 != cells[2] / 9
                    && i % 9 != cells[2] % 9
                    && (i / 27 != cells[2] / 27 || i % 9 / 3 != cells[2] % 9 / 3)
            })
            .collect();
        assert_eq!(restored.candidate_cells(2), remaining);
        assert!(restored.revealed_cells(2).contains(&cells[2]));
        restored.undo();
        assert_eq!(restored.candidate_cells(2), cells[..3]);
        restored.erase(&[cells[0]], Tool::Corner);
        assert_eq!(restored.candidate_cells(2), cells[1..3]);
    }
    #[test]
    fn single_candidate_uses_distinct_user_notes_and_never_the_solution() {
        let mut g = game();
        let i = g.puzzle.givens.iter().position(|&v| v == 0).unwrap();
        let given = g.puzzle.givens.iter().position(|&v| v != 0).unwrap();
        let wrong = g.puzzle.solution[i] % 9 + 1;
        assert_eq!(g.single_candidate(i), None);
        g.enter(&[i], wrong, Tool::Corner);
        g.enter(&[i], wrong, Tool::Centre);
        assert_eq!(g.single_candidate(i), Some(wrong));
        let before = g.encode();
        assert_eq!(g.single_candidate(81), None);
        assert_eq!(g.single_candidate(given), None);
        assert_eq!(g.encode(), before);
        g.enter(&[i], wrong % 9 + 1, Tool::Centre);
        assert_eq!(g.single_candidate(i), None);
        g.enter(&[i], wrong, Tool::Digit);
        assert_eq!(g.single_candidate(i), None);
    }
    #[test]
    fn fill_notes_ignores_variant_rules_and_hidden_answers() {
        for variant in Variant::ALL {
            let mut g = game();
            g.puzzle.variant = variant;
            g.puzzle.givens.fill(0);
            // Only 1, 2 and 3 are visible classic peers of r1c1.
            g.puzzle.givens[1] = 1;
            g.puzzle.givens[72] = 2;
            g.puzzle.givens[10] = 3;
            g.puzzle.givens[40] = 4; // Diagonal only: must not exclude 4.
            // Extra clues deliberately restrict r1c1 further. Ignore them all.
            g.puzzle.cages = vec![Cage {
                cells: vec![0],
                sum: 4,
            }];
            g.puzzle.edges = vec![Edge {
                a: 0,
                b: 1,
                relation: Relation::Five,
            }];
            g.puzzle.thermos = vec![vec![0, 1]];
            g.puzzle.solution[0] = 9;
            assert!(g.fill_classic_candidates());
            assert_eq!(g.marks[0].corner, ALL_DIGITS & !(bit(1) | bit(2) | bit(3)));
            assert_eq!(g.marks[0].value, 0);
            assert_eq!(g.hints, 0);
        }
    }
    #[test]
    fn fill_notes_never_fills_singles_cascades_or_resolves_contradictions() {
        let mut g = game();
        g.puzzle.givens.fill(0);
        for i in 1..9 {
            g.puzzle.givens[i] = i as u8;
        }
        g.puzzle.givens[9] = 9;
        for i in 0..8 {
            g.puzzle.givens[27 + i] = i as u8 + 1;
        }
        let visible = g.values();
        assert!(g.fill_classic_candidates());
        assert_eq!(g.marks[0].corner, 0); // No mechanically possible digit.
        assert_eq!(g.marks[35].corner, bit(9)); // A single remains a note.
        assert_ne!(g.marks[44].corner & bit(9), 0); // No cascading eliminations.
        assert_eq!(g.values(), visible);
        assert_eq!(g.hints, 0);
    }
    #[test]
    fn starting_notes_are_classic_only_one_saved_action_and_never_reopen() {
        for variant in Variant::ALL {
            let mut g = Game::new(Generator::new(27, variant, Difficulty::Medium).finish());
            assert!(g.can_fill_classic_candidates());
            let values = g.values();
            let before = g.marks.clone();
            assert!(g.fill_classic_candidates());
            assert!(!g.can_fill_classic_candidates());
            assert_eq!(g.undo.len(), 1);
            assert_eq!(g.values(), values);
            for i in 0..81 {
                if values[i] != 0 {
                    assert_eq!(g.marks[i], before[i]);
                    continue;
                }
                let expected = (1..=9)
                    .filter(|&digit| {
                        !(0..81).any(|peer| {
                            (peer / 9 == i / 9
                                || peer % 9 == i % 9
                                || (peer / 27 == i / 27 && peer % 9 / 3 == i % 9 / 3))
                                && values[peer] == digit
                        })
                    })
                    .fold(0, |mask, digit| mask | bit(digit));
                assert_eq!(g.marks[i].corner, expected, "{variant:?}, cell {i}");
                assert_eq!(g.marks[i].centre, 0);
            }
            let after = g.marks.clone();
            let saved = g.encode();
            assert!(!g.fill_classic_candidates());
            assert_eq!(g.encode(), saved);
            let mut restored = Game::restore(&saved).unwrap();
            assert!(!restored.can_fill_classic_candidates());
            assert_eq!(restored.hints, 0);
            assert!(restored.undo());
            assert_eq!(restored.marks, before);
            assert!(!restored.fill_classic_candidates());
            assert!(
                !Game::restore(&restored.encode())
                    .unwrap()
                    .can_fill_classic_candidates()
            );
            assert!(restored.redo());
            assert_eq!(restored.marks, after);
            assert!(restored.reset());
            assert!(!restored.can_fill_classic_candidates());
            assert!(!restored.fill_classic_candidates());
        }
    }
    #[test]
    fn first_edit_disables_starting_notes_even_after_undo_or_reset() {
        for tool in Tool::ALL {
            let mut g = game();
            let i = g.puzzle.givens.iter().position(|&v| v == 0).unwrap();
            let given = g.puzzle.givens.iter().position(|&v| v > 0).unwrap();
            assert!(!g.enter(&[], 1, tool));
            assert!(!g.erase(&[i], tool));
            if tool != Tool::Colour {
                assert!(!g.enter(&[given], 1, tool));
            }
            assert!(g.can_fill_classic_candidates());
            assert!(g.enter(&[i], 1, tool));
            let before = g.encode();
            assert!(!g.fill_classic_candidates());
            assert_eq!(g.encode(), before);
            assert!(g.undo());
            assert!(!g.can_fill_classic_candidates());
            assert!(g.redo());
            assert!(g.reset());
            assert!(
                !Game::restore(&g.encode())
                    .unwrap()
                    .can_fill_classic_candidates()
            );
        }
        let mut g = game();
        assert!(g.hint().is_some());
        assert!(!g.can_fill_classic_candidates());
        assert!(g.undo());
        assert!(!g.fill_classic_candidates());
        assert!(Game::new(g.puzzle).can_fill_classic_candidates());
    }
    #[test]
    fn older_saves_only_offer_starting_notes_if_untouched() {
        let mut g = game();
        let legacy = |g: &Game| {
            let mut data: serde_json::Value = serde_json::from_str(&g.encode()).unwrap();
            data["game"]
                .as_object_mut()
                .unwrap()
                .remove("initial_notes_available");
            Game::restore(&data.to_string()).unwrap()
        };
        assert!(legacy(&g).can_fill_classic_candidates());
        let i = g.puzzle.givens.iter().position(|&v| v == 0).unwrap();
        g.enter(&[i], 1, Tool::Corner);
        assert!(!legacy(&g).can_fill_classic_candidates());
        g.undo();
        assert!(!legacy(&g).can_fill_classic_candidates());
    }
    #[test]
    fn space_entry_toggle_only_uses_digit_and_corner_notes() {
        assert_eq!(Tool::Digit.toggle_entry(), Tool::Corner);
        for tool in [Tool::Corner, Tool::Centre, Tool::Colour] {
            assert_eq!(tool.toggle_entry(), Tool::Digit);
            assert_eq!(tool.toggle_entry().toggle_entry(), Tool::Corner);
        }
    }
    #[test]
    fn mistake_feedback_tracks_correction_undo_redo_and_restore_in_every_variant() {
        for v in Variant::ALL {
            let mut g = Game::new(Generator::new(31, v, Difficulty::Medium).finish());
            let i = g.puzzle.givens.iter().position(|&v| v == 0).unwrap();
            let correct = g.puzzle.solution[i];
            g.enter(&[i], correct % 9 + 1, Tool::Corner);
            assert!(g.mistakes().is_empty());
            g.enter(&[i], correct % 9 + 1, Tool::Digit);
            assert_eq!(g.mistakes(), vec![i]);
            let mut restored = Game::restore(&g.encode()).unwrap();
            assert_eq!(restored.mistakes(), vec![i]);
            restored.enter(&[i], correct, Tool::Digit);
            assert!(restored.mistakes().is_empty());
            restored.undo();
            assert_eq!(restored.mistakes(), vec![i]);
            restored.redo();
            assert!(restored.mistakes().is_empty());
        }
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
