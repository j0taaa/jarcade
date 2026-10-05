//! Original picture puzzles and rendering-independent Nonogram rules.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Blank,
    Filled,
    Cross,
}
impl Cell {
    fn code(self) -> u8 {
        match self {
            Self::Blank => 0,
            Self::Filled => 1,
            Self::Cross => 2,
        }
    }
    fn decode(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Blank),
            1 => Some(Self::Filled),
            2 => Some(Self::Cross),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Small,
    Medium,
    Large,
}
impl Size {
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];
    pub fn side(self) -> usize {
        match self {
            Self::Small => 5,
            Self::Medium => 10,
            Self::Large => 15,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Small => "5 × 5",
            Self::Medium => "10 × 10",
            Self::Large => "15 × 15",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Small => "Little pictures",
            Self::Medium => "A little challenge",
            Self::Large => "Take your time",
        }
    }
}

pub struct Puzzle {
    pub id: &'static str,
    pub name: &'static str,
    pub rows: &'static [&'static str],
}
impl Puzzle {
    pub fn size(&self) -> Size {
        match self.rows.len() {
            5 => Size::Small,
            10 => Size::Medium,
            _ => Size::Large,
        }
    }
    pub fn filled(&self, index: usize) -> bool {
        let side = self.rows.len();
        index < side * side && self.rows[index / side].as_bytes()[index % side] == b'#'
    }
    pub fn clues(&self) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
        let side = self.rows.len();
        let rows = (0..side)
            .map(|y| runs((0..side).map(|x| self.filled(y * side + x))))
            .collect();
        let columns = (0..side)
            .map(|x| runs((0..side).map(|y| self.filled(y * side + x))))
            .collect();
        (rows, columns)
    }
}

// These are hand-authored pixel pictures, never sampled from copyrighted art.
pub static PUZZLES: &[Puzzle] = &[
    Puzzle {
        id: "heart",
        name: "Heart",
        rows: &[".#.#.", "#####", "#####", ".###.", "..#.."],
    },
    Puzzle {
        id: "cup",
        name: "Tea time",
        rows: &["####.", "#.###", "#.#.#", "#####", ".##.."],
    },
    Puzzle {
        id: "tree",
        name: "Little tree",
        rows: &["..#..", ".###.", "#####", "..#..", ".###."],
    },
    Puzzle {
        id: "boat",
        name: "Sailboat",
        rows: &["..#..", ".##..", "####.", "#####", ".###."],
    },
    Puzzle {
        id: "house",
        name: "Home",
        rows: &[
            "....##....",
            "...####...",
            "..######..",
            ".########.",
            "##########",
            ".########.",
            ".##..####.",
            ".##..#..#.",
            ".##..#..#.",
            ".########.",
        ],
    },
    Puzzle {
        id: "tulip",
        name: "Tulip",
        rows: &[
            "..##..##..",
            "..##..##..",
            "..######..",
            "..######..",
            "...####...",
            "....##....",
            ".##.##....",
            ".#####.##.",
            "..#######.",
            "....###...",
        ],
    },
    Puzzle {
        id: "whale",
        name: "Whale",
        rows: &[
            "..........",
            ".....##...",
            ".....#....",
            "...#####..",
            "..########",
            "#####.####",
            "##.#######",
            "#########.",
            "..######..",
            "...####...",
        ],
    },
    Puzzle {
        id: "star",
        name: "Star",
        rows: &[
            "....##....",
            "....##....",
            "...####...",
            "##########",
            ".########.",
            "..######..",
            "..######..",
            ".########.",
            ".###..###.",
            "##......##",
        ],
    },
    Puzzle {
        id: "rocket",
        name: "Lift off",
        rows: &[
            ".......#.......",
            "......###......",
            ".....#####.....",
            ".....#####.....",
            "....#######....",
            "....##...##....",
            "....##...##....",
            "....##...##....",
            "....#######....",
            "...#########...",
            "..###########..",
            ".####.###.####.",
            ".###..###..###.",
            "......###......",
            ".......#.......",
        ],
    },
    Puzzle {
        id: "fox",
        name: "Woodland fox",
        rows: &[
            "..#.........#..",
            "..##.......##..",
            "..###.....###..",
            "..####...####..",
            "..###########..",
            ".#############.",
            ".#############.",
            "##..#######..##",
            "##..#######..##",
            "###..#####..###",
            ".###..###..###.",
            "..###..#..###..",
            "...####.####...",
            "....#######....",
            "......###......",
        ],
    },
    Puzzle {
        id: "cactus",
        name: "Desert friend",
        rows: &[
            "......###......",
            ".....#####.....",
            ".....#####.....",
            "..##.#####.....",
            ".###.#####.##..",
            ".###.#####.###.",
            ".###.#####.###.",
            ".#########.###.",
            "..########.###.",
            ".....#########.",
            ".....########..",
            ".....#####.....",
            ".....#####.....",
            "...#########...",
            "...#########...",
        ],
    },
    Puzzle {
        id: "butterfly",
        name: "Butterfly",
        rows: &[
            ".##.........##.",
            ".##.........##.",
            ".###...#...###.",
            ".####.###.####.",
            ".#############.",
            "..###########..",
            "...#########...",
            ".....#####.....",
            "...#########...",
            "..###########..",
            ".#############.",
            ".####.###.####.",
            ".###..###..###.",
            "..##...#...##..",
            ".......#.......",
        ],
    },
];

pub fn runs(cells: impl IntoIterator<Item = bool>) -> Vec<usize> {
    let mut result = Vec::new();
    let mut run = 0;
    for filled in cells {
        if filled {
            run += 1;
        } else if run > 0 {
            result.push(run);
            run = 0;
        }
    }
    if run > 0 {
        result.push(run);
    }
    result
}

#[derive(Clone, Default)]
struct Progress {
    cells: Vec<Cell>,
    hints: usize,
}
#[derive(Serialize, Deserialize)]
struct Record {
    id: String,
    cells: Vec<u8>,
    #[serde(default)]
    hints: usize,
}
#[derive(Serialize, Deserialize)]
struct Save {
    version: u8,
    selected: String,
    boards: Vec<Record>,
}

#[derive(Clone)]
struct Stroke {
    target: Cell,
    changes: Vec<(usize, Cell)>,
}
pub struct Game {
    selected: usize,
    boards: Vec<Progress>,
    undo: Vec<Vec<(usize, Cell)>>,
    stroke: Option<Stroke>,
    row_clues: Vec<Vec<usize>>,
    column_clues: Vec<Vec<usize>>,
}
impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}
impl Game {
    pub fn new() -> Self {
        let (row_clues, column_clues) = PUZZLES[0].clues();
        Self {
            selected: 0,
            boards: PUZZLES
                .iter()
                .map(|p| Progress {
                    cells: vec![Cell::Blank; p.rows.len().pow(2)],
                    hints: 0,
                })
                .collect(),
            undo: Vec::new(),
            stroke: None,
            row_clues,
            column_clues,
        }
    }
    pub fn restore(data: &str) -> Self {
        let mut game = Self::new();
        if data.len() > 64 * 1024 {
            return game;
        }
        let Ok(save) = serde_json::from_str::<Save>(data) else {
            return game;
        };
        if save.version != 1 {
            return game;
        }
        for record in save.boards.into_iter().take(PUZZLES.len()) {
            let Some(index) = PUZZLES.iter().position(|p| p.id == record.id) else {
                continue;
            };
            if record.cells.len() != game.boards[index].cells.len() {
                continue;
            }
            if let Some(cells) = record
                .cells
                .into_iter()
                .map(Cell::decode)
                .collect::<Option<Vec<_>>>()
            {
                game.boards[index] = Progress {
                    cells,
                    hints: record.hints.min(9999),
                };
            }
        }
        if let Some(index) = PUZZLES.iter().position(|p| p.id == save.selected) {
            game.choose(index);
        }
        game
    }
    pub fn encode(&self) -> String {
        let save = Save {
            version: 1,
            selected: self.puzzle().id.into(),
            boards: PUZZLES
                .iter()
                .zip(&self.boards)
                .filter(|(_, p)| p.cells.iter().any(|c| *c != Cell::Blank) || p.hints > 0)
                .map(|(p, progress)| Record {
                    id: p.id.into(),
                    cells: progress.cells.iter().map(|c| c.code()).collect(),
                    hints: progress.hints,
                })
                .collect(),
        };
        serde_json::to_string(&save).expect("Nonogram save contains only finite integer data")
    }
    pub fn selected(&self) -> usize {
        self.selected
    }
    pub fn puzzle(&self) -> &'static Puzzle {
        &PUZZLES[self.selected]
    }
    pub fn side(&self) -> usize {
        self.puzzle().rows.len()
    }
    pub fn cells(&self) -> &[Cell] {
        &self.boards[self.selected].cells
    }
    pub fn row_clues(&self) -> &[Vec<usize>] {
        &self.row_clues
    }
    pub fn column_clues(&self) -> &[Vec<usize>] {
        &self.column_clues
    }
    pub fn hints(&self) -> usize {
        self.boards[self.selected].hints
    }
    pub fn marked(&self, index: usize) -> usize {
        self.boards
            .get(index)
            .map_or(0, |p| p.cells.iter().filter(|c| **c != Cell::Blank).count())
    }
    pub fn completed(&self, index: usize) -> bool {
        self.boards
            .get(index)
            .zip(PUZZLES.get(index))
            .is_some_and(|(progress, puzzle)| {
                progress
                    .cells
                    .iter()
                    .enumerate()
                    .all(|(i, c)| (*c == Cell::Filled) == puzzle.filled(i))
            })
    }
    pub fn won(&self) -> bool {
        self.completed(self.selected)
    }
    pub fn choose(&mut self, index: usize) -> bool {
        if index >= PUZZLES.len() {
            return false;
        }
        self.cancel_stroke();
        self.selected = index;
        self.undo.clear();
        (self.row_clues, self.column_clues) = self.puzzle().clues();
        true
    }
    /// A drag always paints the state chosen at its first cell. Revisiting a cell
    /// never toggles it again, and the complete stroke is one undo operation.
    pub fn begin_stroke(&mut self, index: usize, tool: Cell) -> bool {
        if self.won() || index >= self.cells().len() {
            return false;
        }
        self.cancel_stroke();
        let target = if self.cells()[index] == tool {
            Cell::Blank
        } else {
            tool
        };
        self.stroke = Some(Stroke {
            target,
            changes: Vec::new(),
        });
        self.paint(index)
    }
    pub fn paint(&mut self, index: usize) -> bool {
        let Some(stroke) = &mut self.stroke else {
            return false;
        };
        let Some(cell) = self.boards[self.selected].cells.get_mut(index) else {
            return false;
        };
        if stroke.changes.iter().any(|(i, _)| *i == index) || *cell == stroke.target {
            return false;
        }
        stroke.changes.push((index, *cell));
        *cell = stroke.target;
        true
    }
    pub fn finish_stroke(&mut self) -> bool {
        if let Some(stroke) = self.stroke.take()
            && !stroke.changes.is_empty()
        {
            self.push_undo(stroke.changes);
            return true;
        }
        false
    }
    pub fn cancel_stroke(&mut self) {
        if let Some(stroke) = self.stroke.take() {
            for (index, cell) in stroke.changes {
                self.boards[self.selected].cells[index] = cell;
            }
        }
    }
    fn push_undo(&mut self, changes: Vec<(usize, Cell)>) {
        if self.undo.len() == 100 {
            self.undo.remove(0);
        }
        self.undo.push(changes);
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn undo(&mut self) -> bool {
        self.cancel_stroke();
        if let Some(changes) = self.undo.pop() {
            for (i, cell) in changes {
                self.boards[self.selected].cells[i] = cell;
            }
            true
        } else {
            false
        }
    }
    pub fn reset(&mut self) -> bool {
        self.cancel_stroke();
        let changes = self
            .cells()
            .iter()
            .enumerate()
            .filter_map(|(i, c)| (*c != Cell::Blank).then_some((i, *c)))
            .collect::<Vec<_>>();
        if changes.is_empty() {
            return false;
        }
        self.boards[self.selected].cells.fill(Cell::Blank);
        self.push_undo(changes);
        true
    }
    pub fn hint(&mut self) -> Option<usize> {
        self.cancel_stroke();
        if self.won() {
            return None;
        }
        // Correct a contradiction before offering another cell. Never penalize
        // experimentation: the hint and reset are both undoable.
        let index =
            self.cells()
                .iter()
                .enumerate()
                .find_map(|(i, c)| match c {
                    Cell::Filled if !self.puzzle().filled(i) => Some(i),
                    Cell::Cross if self.puzzle().filled(i) => Some(i),
                    _ => None,
                })
                .or_else(|| {
                    self.cells().iter().enumerate().find_map(|(i, c)| {
                        (*c == Cell::Blank && self.puzzle().filled(i)).then_some(i)
                    })
                })?;
        let previous = self.cells()[index];
        self.boards[self.selected].cells[index] = if self.puzzle().filled(index) {
            Cell::Filled
        } else {
            Cell::Cross
        };
        self.boards[self.selected].hints += 1;
        self.push_undo(vec![(index, previous)]);
        Some(index)
    }
    pub fn line_complete(&self, index: usize, column: bool) -> bool {
        let side = self.side();
        if index >= side {
            return false;
        }
        let filled = runs((0..side).map(|i| {
            self.cells()[if column {
                i * side + index
            } else {
                index * side + i
            }] == Cell::Filled
        }));
        filled
            == if column {
                self.column_clues[index].clone()
            } else {
                self.row_clues[index].clone()
            }
    }
}

/// Candidate bitmasks for a numbered line. Used to verify every shipped puzzle
/// is uniquely solvable, and can be solved through line deductions alone.
pub fn line_patterns(side: usize, clues: &[usize]) -> Vec<u16> {
    fn place(side: usize, clues: &[usize], at: usize, mask: u16, out: &mut Vec<u16>) {
        if clues.is_empty() {
            out.push(mask);
            return;
        }
        let required = clues.iter().sum::<usize>() + clues.len() - 1;
        if at + required > side {
            return;
        }
        for start in at..=side - required {
            let bits = ((1u16 << clues[0]) - 1) << start;
            place(side, &clues[1..], start + clues[0] + 1, mask | bits, out);
        }
    }
    if side > 15 || clues.len() > side || clues.contains(&0) || clues.iter().any(|c| *c > side) {
        return Vec::new();
    }
    let mut out = Vec::new();
    place(side, clues, 0, 0, &mut out);
    out
}

pub fn solves_by_lines(puzzle: &Puzzle) -> bool {
    let side = puzzle.rows.len();
    let (row_clues, col_clues) = puzzle.clues();
    let mut rows = row_clues
        .iter()
        .map(|c| line_patterns(side, c))
        .collect::<Vec<_>>();
    let mut cols = col_clues
        .iter()
        .map(|c| line_patterns(side, c))
        .collect::<Vec<_>>();
    loop {
        let mut changed = false;
        for axis in [false, true] {
            for line in 0..side {
                let (source, target) = if axis {
                    (&cols, &mut rows)
                } else {
                    (&rows, &mut cols)
                };
                if source[line].is_empty() {
                    return false;
                }
                let all = source[line].iter().fold(u16::MAX, |a, b| a & b);
                let any = source[line].iter().fold(0u16, |a, b| a | b);
                for (position, candidates) in target.iter_mut().enumerate() {
                    let must_fill = all & (1 << position) != 0;
                    let must_empty = any & (1 << position) == 0;
                    if must_fill || must_empty {
                        let before = candidates.len();
                        candidates.retain(|mask| (mask & (1 << line) != 0) == must_fill);
                        changed |= candidates.len() != before;
                    }
                }
            }
        }
        if rows.iter().chain(&cols).any(Vec::is_empty) {
            return false;
        }
        if rows.iter().chain(&cols).all(|line| line.len() == 1) {
            return true;
        }
        if !changed {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_picture_has_valid_dimensions_and_a_unique_logical_solution() {
        for puzzle in PUZZLES {
            assert!(
                puzzle
                    .rows
                    .iter()
                    .all(|r| r.len() == puzzle.rows.len()
                        && r.bytes().all(|b| b == b'.' || b == b'#')),
                "{}",
                puzzle.id
            );
            assert!(
                solves_by_lines(puzzle),
                "{} is ambiguous or requires guessing",
                puzzle.id
            );
        }
    }
    #[test]
    fn empty_lines_full_lines_and_separate_runs_are_numbered_correctly() {
        assert_eq!(runs([false; 5]), Vec::<usize>::new());
        assert_eq!(runs([true; 5]), vec![5]);
        assert_eq!(runs([true, true, false, true, false]), vec![2, 1]);
        assert_eq!(line_patterns(5, &[]), vec![0]);
        assert_eq!(line_patterns(5, &[5]), vec![31]);
        assert_eq!(line_patterns(5, &[2, 2]), vec![27]);
        assert!(line_patterns(5, &[3, 3]).is_empty());
        assert!(line_patterns(20, &[1]).is_empty());
    }
    #[test]
    fn ambiguous_picture_is_not_mistaken_for_a_unique_solution() {
        let ambiguous = Puzzle {
            id: "test",
            name: "test",
            rows: &["#.", ".#"],
        };
        assert!(!solves_by_lines(&ambiguous));
    }
    #[test]
    fn stroke_paints_consistently_and_undoes_as_one_action() {
        let mut game = Game::new();
        game.begin_stroke(0, Cell::Filled);
        game.paint(1);
        game.paint(0);
        game.paint(2);
        assert!(game.finish_stroke());
        assert_eq!(&game.cells()[..3], &[Cell::Filled; 3]);
        assert!(game.undo());
        assert_eq!(&game.cells()[..3], &[Cell::Blank; 3]);
        assert!(!game.undo());
        game.begin_stroke(0, Cell::Filled);
        game.finish_stroke();
        game.begin_stroke(0, Cell::Filled);
        game.paint(1);
        game.finish_stroke();
        assert_eq!(&game.cells()[..2], &[Cell::Blank; 2]);
    }
    #[test]
    fn interrupted_or_multitouch_stroke_rolls_back_every_cell() {
        let mut game = Game::new();
        game.begin_stroke(4, Cell::Cross);
        game.paint(5);
        game.cancel_stroke();
        assert_eq!(game.marked(0), 0);
        assert!(!game.can_undo());
        assert!(!game.paint(0));
        assert!(!game.begin_stroke(1000, Cell::Filled));
    }
    #[test]
    fn win_requires_every_filled_cell_and_no_extra_fill_but_crosses_are_optional() {
        let mut game = Game::new();
        game.begin_stroke(0, Cell::Filled);
        for i in 0..25 {
            if game.puzzle().filled(i) {
                game.paint(i);
            }
        }
        game.finish_stroke();
        assert!(!game.won());
        game.begin_stroke(0, Cell::Filled);
        game.finish_stroke();
        assert!(game.won());
        assert!(game.cells().contains(&Cell::Blank));
        assert!(!game.begin_stroke(1, Cell::Cross));
        assert!(game.undo());
        assert!(!game.won());
    }
    #[test]
    fn hint_corrects_mistakes_and_reset_can_be_undone() {
        let mut game = Game::new();
        game.begin_stroke(0, Cell::Filled);
        game.finish_stroke();
        assert_eq!(game.hint(), Some(0));
        assert_eq!(game.cells()[0], Cell::Cross);
        assert_eq!(game.hints(), 1);
        game.reset();
        assert_eq!(game.marked(0), 0);
        game.undo();
        assert_eq!(game.cells()[0], Cell::Cross);
        game.undo();
        assert_eq!(game.cells()[0], Cell::Filled);
    }
    #[test]
    fn saves_preserve_each_board_without_trusting_malformed_data() {
        let mut game = Game::new();
        game.begin_stroke(0, Cell::Cross);
        game.finish_stroke();
        game.choose(8);
        game.begin_stroke(224, Cell::Cross);
        game.finish_stroke();
        let mut restored = Game::restore(&game.encode());
        assert_eq!(restored.selected(), 8);
        assert_eq!(restored.cells()[224], Cell::Cross);
        assert!(!restored.can_undo());
        restored.choose(0);
        assert_eq!(restored.cells()[0], Cell::Cross);
        for data in [
            "",
            "{}",
            r#"{"version":1,"selected":"heart","boards":[{"id":"heart","cells":[9]}]}"#,
            r#"{"version":2,"selected":"fox","boards":[]}"#,
        ] {
            assert_eq!(Game::restore(data).marked(0), 0);
        }
    }
}
