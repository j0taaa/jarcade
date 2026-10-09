//! Four offline games for exactly two people. No room transport or renderer.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Faces,
    Mastermind,
    TicTacToe,
    ConnectFour,
}
impl Kind {
    pub const ALL: [Self; 4] = [
        Self::Faces,
        Self::Mastermind,
        Self::TicTacToe,
        Self::ConnectFour,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Faces => "Cara a Cara",
            Self::Mastermind => "Mastermind",
            Self::TicTacToe => "Jogo da Velha",
            Self::ConnectFour => "Ligue 4",
        }
    }
    pub fn path(self) -> &'static str {
        match self {
            Self::Faces => "/games/guess-who",
            Self::Mastermind => "/games/mastermind",
            Self::TicTacToe => "/games/tic-tac-toe",
            Self::ConnectFour => "/games/connect-four",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Won(u8),
    Draw,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grid {
    kind: Kind,
    moves: Vec<usize>,
    starter: u8,
    scores: [u32; 2],
    round: u32,
}
#[derive(Clone, Debug)]
pub struct Position {
    pub cells: Vec<u8>,
    pub turn: u8,
    pub outcome: Outcome,
    pub line: Vec<usize>,
}
impl Grid {
    pub fn new(kind: Kind) -> Self {
        assert!(matches!(kind, Kind::TicTacToe | Kind::ConnectFour));
        Self {
            kind,
            moves: vec![],
            starter: 0,
            scores: [0; 2],
            round: 1,
        }
    }
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn size(&self) -> (usize, usize) {
        if self.kind == Kind::TicTacToe {
            (3, 3)
        } else {
            (7, 6)
        }
    }
    pub fn scores(&self) -> [u32; 2] {
        self.scores
    }
    pub fn round(&self) -> u32 {
        self.round
    }
    fn empty(&self) -> Position {
        let (w, h) = self.size();
        Position {
            cells: vec![0; w * h],
            turn: self.starter,
            outcome: Outcome::Playing,
            line: vec![],
        }
    }
    fn apply(&self, p: &mut Position, action: usize) -> bool {
        if p.outcome != Outcome::Playing {
            return false;
        }
        let (w, h) = self.size();
        let index = if self.kind == Kind::TicTacToe {
            if action >= 9 || p.cells[action] != 0 {
                return false;
            }
            action
        } else {
            if action >= w {
                return false;
            }
            let Some(row) = (0..h).rev().find(|&row| p.cells[row * w + action] == 0) else {
                return false;
            };
            row * w + action
        };
        p.cells[index] = p.turn + 1;
        let target = if self.kind == Kind::TicTacToe { 3 } else { 4 };
        for y in 0..h {
            for x in 0..w {
                for (dx, dy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                    let line: Vec<_> = (0..target)
                        .filter_map(|i| {
                            let xx = x as isize + dx * i as isize;
                            let yy = y as isize + dy * i as isize;
                            (xx >= 0 && yy >= 0 && xx < w as isize && yy < h as isize)
                                .then(|| yy as usize * w + xx as usize)
                        })
                        .collect();
                    if line.len() == target && line.iter().all(|&i| p.cells[i] == p.turn + 1) {
                        p.line = line;
                        p.outcome = Outcome::Won(p.turn);
                        return true;
                    }
                }
            }
        }
        p.turn = 1 - p.turn;
        if p.cells.iter().all(|&c| c != 0) {
            p.outcome = Outcome::Draw;
        }
        true
    }
    pub fn position(&self) -> Position {
        let mut p = self.empty();
        for &action in &self.moves {
            self.apply(&mut p, action);
        }
        p
    }
    pub fn play(&mut self, action: usize) -> bool {
        let mut p = self.position();
        if !self.apply(&mut p, action) {
            return false;
        }
        self.moves.push(action);
        if let Outcome::Won(player) = p.outcome {
            self.scores[player as usize] = self.scores[player as usize].saturating_add(1);
        }
        true
    }
    pub fn next_round(&mut self) {
        self.moves.clear();
        self.starter = 1 - self.starter;
        self.round = self.round.saturating_add(1);
    }
    fn valid(&self, kind: Kind) -> bool {
        if self.kind != kind
            || self.starter > 1
            || self.round == 0
            || self.moves.len() > self.empty().cells.len()
        {
            return false;
        }
        let mut p = self.empty();
        self.moves.iter().all(|&action| self.apply(&mut p, action))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MmPhase {
    CodeCover,
    Code,
    GuessCover,
    Guess,
    Result,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Feedback {
    pub exact: u8,
    pub colour: u8,
}
pub fn feedback(code: [u8; 4], guess: [u8; 4]) -> Feedback {
    let mut exact = 0;
    let mut a = [0u8; 6];
    let mut b = [0u8; 6];
    for i in 0..4 {
        if code[i] == guess[i] {
            exact += 1;
        } else {
            if let Some(n) = a.get_mut(code[i] as usize) {
                *n += 1;
            }
            if let Some(n) = b.get_mut(guess[i] as usize) {
                *n += 1;
            }
        }
    }
    Feedback {
        exact,
        colour: (0..6).map(|i| a[i].min(b[i])).sum(),
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mastermind {
    phase: MmPhase,
    maker: u8,
    code: [Option<u8>; 4],
    draft: [Option<u8>; 4],
    guesses: Vec<[u8; 4]>,
    scores: [u32; 2],
    round: u32,
}
impl Default for Mastermind {
    fn default() -> Self {
        Self {
            phase: MmPhase::CodeCover,
            maker: 0,
            code: [None; 4],
            draft: [None; 4],
            guesses: vec![],
            scores: [0; 2],
            round: 1,
        }
    }
}
impl Mastermind {
    pub const LIMIT: usize = 10;
    pub fn phase(&self) -> MmPhase {
        self.phase
    }
    pub fn maker(&self) -> u8 {
        self.maker
    }
    pub fn player(&self) -> u8 {
        if matches!(self.phase, MmPhase::CodeCover | MmPhase::Code) {
            self.maker
        } else {
            1 - self.maker
        }
    }
    pub fn scores(&self) -> [u32; 2] {
        self.scores
    }
    pub fn round(&self) -> u32 {
        self.round
    }
    pub fn slots(&self) -> [Option<u8>; 4] {
        match self.phase {
            MmPhase::Code | MmPhase::Result => self.code,
            MmPhase::Guess => self.draft,
            _ => [None; 4],
        }
    }
    pub fn history(&self) -> Vec<([u8; 4], Feedback)> {
        let Some(code) = complete(self.code) else {
            return vec![];
        };
        self.guesses
            .iter()
            .map(|&g| (g, feedback(code, g)))
            .collect()
    }
    pub fn solved(&self) -> bool {
        self.guesses
            .last()
            .is_some_and(|&g| Some(g) == complete(self.code))
    }
    pub fn reveal(&mut self) -> bool {
        match self.phase {
            MmPhase::CodeCover => self.phase = MmPhase::Code,
            MmPhase::GuessCover => self.phase = MmPhase::Guess,
            _ => return false,
        };
        true
    }
    pub fn conceal(&mut self) -> bool {
        match self.phase {
            MmPhase::Code => self.phase = MmPhase::CodeCover,
            MmPhase::Guess => self.phase = MmPhase::GuessCover,
            _ => return false,
        };
        true
    }
    pub fn set(&mut self, slot: usize, value: Option<u8>) -> bool {
        if slot >= 4 || value.is_some_and(|v| v >= 6) {
            return false;
        }
        let slots = match self.phase {
            MmPhase::Code => &mut self.code,
            MmPhase::Guess => &mut self.draft,
            _ => return false,
        };
        if slots[slot] == value {
            return false;
        }
        slots[slot] = value;
        true
    }
    pub fn ready(&self) -> bool {
        matches!(self.phase, MmPhase::Code | MmPhase::Guess) && complete(self.slots()).is_some()
    }
    pub fn seal(&mut self) -> bool {
        if self.phase != MmPhase::Code || !self.ready() {
            return false;
        }
        self.phase = MmPhase::GuessCover;
        true
    }
    pub fn submit(&mut self) -> bool {
        if self.phase != MmPhase::Guess {
            return false;
        }
        let Some(guess) = complete(self.draft) else {
            return false;
        };
        self.guesses.push(guess);
        self.draft = [None; 4];
        if self.solved() || self.guesses.len() == Self::LIMIT {
            self.phase = MmPhase::Result;
            self.scores[self.maker as usize] =
                self.scores[self.maker as usize].saturating_add(self.guesses.len() as u32);
        }
        true
    }
    pub fn next_round(&mut self) {
        let scores = self.scores;
        let round = self.round.saturating_add(1);
        let maker = 1 - self.maker;
        *self = Self {
            scores,
            round,
            maker,
            ..Self::default()
        };
    }
    fn valid(&self) -> bool {
        if self.maker > 1
            || self.round == 0
            || self.guesses.len() > Self::LIMIT
            || self
                .code
                .iter()
                .chain(self.draft.iter())
                .any(|v| v.is_some_and(|v| v >= 6))
            || self.guesses.iter().flatten().any(|&v| v >= 6)
        {
            return false;
        }
        if matches!(self.phase, MmPhase::CodeCover | MmPhase::Code) {
            return self.guesses.is_empty() && self.draft == [None; 4];
        }
        let Some(code) = complete(self.code) else {
            return false;
        };
        if self
            .guesses
            .iter()
            .take(self.guesses.len().saturating_sub(1))
            .any(|&g| g == code)
        {
            return false;
        }
        let ended = self.solved() || self.guesses.len() == Self::LIMIT;
        (self.phase == MmPhase::Result) == ended
            && (self.phase != MmPhase::Result || self.draft == [None; 4])
    }
}
fn complete(a: [Option<u8>; 4]) -> Option<[u8; 4]> {
    Some([a[0]?, a[1]?, a[2]?, a[3]?])
}

pub const FACE_COUNT: usize = 24;
pub const FACE_NAMES: [&str; FACE_COUNT] = [
    "Alex", "Bia", "Caio", "Dani", "Eva", "Felipe", "Gabi", "Hugo", "Isa", "João", "Kai", "Lia",
    "Maya", "Nico", "Olívia", "Pedro", "Ravi", "Sara", "Theo", "Uma", "Vini", "Yara", "Zeca",
    "Zoe",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FacePhase {
    Cover { player: u8, choosing: bool },
    Choose { player: u8 },
    Turn { player: u8 },
    End { winner: u8 },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Faces {
    phase: FacePhase,
    identities: [Option<usize>; 2],
    eliminated: [Vec<bool>; 2],
    scores: [u32; 2],
    round: u32,
}
impl Default for Faces {
    fn default() -> Self {
        Self {
            phase: FacePhase::Cover {
                player: 0,
                choosing: true,
            },
            identities: [None; 2],
            eliminated: [vec![false; FACE_COUNT], vec![false; FACE_COUNT]],
            scores: [0; 2],
            round: 1,
        }
    }
}
impl Faces {
    pub fn phase(&self) -> FacePhase {
        self.phase
    }
    pub fn scores(&self) -> [u32; 2] {
        self.scores
    }
    pub fn round(&self) -> u32 {
        self.round
    }
    pub fn own(&self) -> Option<usize> {
        match self.phase {
            FacePhase::Choose { player } | FacePhase::Turn { player } => {
                self.identities[player as usize]
            }
            _ => None,
        }
    }
    pub fn result(&self) -> Option<[usize; 2]> {
        if matches!(self.phase, FacePhase::End { .. }) {
            Some([self.identities[0]?, self.identities[1]?])
        } else {
            None
        }
    }
    pub fn marks(&self) -> Vec<bool> {
        if let FacePhase::Turn { player } = self.phase {
            self.eliminated[player as usize].clone()
        } else {
            vec![false; FACE_COUNT]
        }
    }
    pub fn reveal(&mut self) -> bool {
        if let FacePhase::Cover { player, choosing } = self.phase {
            self.phase = if choosing {
                FacePhase::Choose { player }
            } else {
                FacePhase::Turn { player }
            };
            true
        } else {
            false
        }
    }
    pub fn conceal(&mut self) -> bool {
        match self.phase {
            FacePhase::Choose { player } => {
                self.phase = FacePhase::Cover {
                    player,
                    choosing: true,
                }
            }
            FacePhase::Turn { player } => {
                self.phase = FacePhase::Cover {
                    player,
                    choosing: false,
                }
            }
            _ => return false,
        };
        true
    }
    pub fn choose(&mut self, id: usize) -> bool {
        if id >= FACE_COUNT {
            return false;
        }
        if let FacePhase::Choose { player } = self.phase {
            if self.identities[player as usize] == Some(id) {
                return false;
            }
            self.identities[player as usize] = Some(id);
            true
        } else {
            false
        }
    }
    pub fn seal(&mut self) -> bool {
        let FacePhase::Choose { player } = self.phase else {
            return false;
        };
        if self.own().is_none() {
            return false;
        }
        self.phase = if player == 0 {
            FacePhase::Cover {
                player: 1,
                choosing: true,
            }
        } else {
            FacePhase::Cover {
                player: ((self.round - 1) % 2) as u8,
                choosing: false,
            }
        };
        true
    }
    pub fn toggle(&mut self, id: usize) -> bool {
        if let FacePhase::Turn { player } = self.phase
            && let Some(m) = self.eliminated[player as usize].get_mut(id)
        {
            *m = !*m;
            true
        } else {
            false
        }
    }
    pub fn pass(&mut self) -> bool {
        if let FacePhase::Turn { player } = self.phase {
            self.phase = FacePhase::Cover {
                player: 1 - player,
                choosing: false,
            };
            true
        } else {
            false
        }
    }
    pub fn guess(&mut self, id: usize) -> bool {
        let FacePhase::Turn { player } = self.phase else {
            return false;
        };
        if id >= FACE_COUNT {
            return false;
        }
        let winner = if self.identities[(1 - player) as usize] == Some(id) {
            player
        } else {
            1 - player
        };
        self.phase = FacePhase::End { winner };
        self.scores[winner as usize] = self.scores[winner as usize].saturating_add(1);
        true
    }
    pub fn next_round(&mut self) {
        let scores = self.scores;
        let round = self.round.saturating_add(1);
        *self = Self {
            scores,
            round,
            ..Self::default()
        };
    }
    fn valid(&self) -> bool {
        if self.round == 0
            || self
                .identities
                .iter()
                .any(|id| id.is_some_and(|v| v >= FACE_COUNT))
            || self.eliminated.iter().any(|m| m.len() != FACE_COUNT)
        {
            return false;
        }
        match self.phase {
            FacePhase::Cover {
                player,
                choosing: true,
            }
            | FacePhase::Choose { player } => {
                player < 2
                    && (player == 0 && self.identities[1].is_none()
                        || player == 1 && self.identities[0].is_some())
                    && self.eliminated.iter().flatten().all(|&v| !v)
            }
            FacePhase::Cover {
                player,
                choosing: false,
            }
            | FacePhase::Turn { player } => {
                player < 2 && self.identities.iter().all(Option::is_some)
            }
            FacePhase::End { winner } => winner < 2 && self.identities.iter().all(Option::is_some),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    version: u8,
    pub tic: Grid,
    pub connect: Grid,
    pub mastermind: Mastermind,
    pub faces: Faces,
}
impl Default for State {
    fn default() -> Self {
        Self {
            version: 1,
            tic: Grid::new(Kind::TicTacToe),
            connect: Grid::new(Kind::ConnectFour),
            mastermind: Mastermind::default(),
            faces: Faces::default(),
        }
    }
}
impl State {
    pub fn conceal(&mut self) -> bool {
        let a = self.mastermind.conceal();
        let b = self.faces.conceal();
        a || b
    }
    pub fn encode(&self) -> String {
        let mut s = self.clone();
        s.conceal();
        serde_json::to_string(&s).unwrap()
    }
    pub fn decode(data: &str) -> Self {
        let Ok(mut s) = serde_json::from_str::<Self>(data) else {
            return Self::default();
        };
        if s.version != 1 {
            return Self::default();
        }
        if !s.tic.valid(Kind::TicTacToe) {
            s.tic = Grid::new(Kind::TicTacToe);
        }
        if !s.connect.valid(Kind::ConnectFour) {
            s.connect = Grid::new(Kind::ConnectFour);
        }
        if !s.mastermind.valid() {
            s.mastermind = Mastermind::default();
        }
        if !s.faces.valid() {
            s.faces = Faces::default();
        }
        s.conceal();
        s
    }
    pub fn reset(&mut self, kind: Kind) {
        match kind {
            Kind::TicTacToe => self.tic = Grid::new(kind),
            Kind::ConnectFour => self.connect = Grid::new(kind),
            Kind::Mastermind => self.mastermind = Mastermind::default(),
            Kind::Faces => self.faces = Faces::default(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tic_wins_rejects_overwrite_and_late_moves_and_alternates_start() {
        let mut g = Grid::new(Kind::TicTacToe);
        assert!(g.play(0));
        assert!(!g.play(0));
        assert!(!g.play(9));
        for i in [3, 1, 4, 2] {
            assert!(g.play(i));
        }
        assert_eq!(g.position().outcome, Outcome::Won(0));
        assert_eq!(g.position().line, vec![0, 1, 2]);
        assert!(!g.play(5));
        assert_eq!(g.scores(), [1, 0]);
        g.next_round();
        assert_eq!(g.position().turn, 1);
        assert_eq!(g.scores(), [1, 0]);
    }
    #[test]
    fn tic_all_reachable_games_end_correctly_without_double_scoring() {
        fn visit(g: Grid, seen: &mut std::collections::HashSet<Vec<u8>>) {
            let p = g.position();
            if !seen.insert(p.cells.clone()) {
                return;
            }
            if p.outcome != Outcome::Playing {
                assert!(
                    p.outcome == Outcome::Draw && p.cells.iter().all(|&c| c != 0)
                        || !p.line.is_empty()
                );
                return;
            }
            for i in 0..9 {
                let mut next = g.clone();
                if next.play(i) {
                    assert!(next.valid(Kind::TicTacToe));
                    visit(next, seen);
                }
            }
        }
        visit(Grid::new(Kind::TicTacToe), &mut Default::default());
    }
    #[test]
    fn connect_gravity_full_columns_and_all_win_directions() {
        let mut g = Grid::new(Kind::ConnectFour);
        for _ in 0..6 {
            assert!(g.play(0));
        }
        assert!(!g.play(0));
        assert!(!g.play(7));
        assert_eq!(g.position().cells[0], 2);
        for (moves, winner) in [
            (vec![0, 1, 0, 1, 0, 1, 0], 0),
            (vec![0, 0, 1, 1, 2, 2, 3], 0),
            (vec![0, 1, 1, 2, 3, 2, 2, 3, 4, 3, 3], 0),
            (vec![6, 5, 5, 4, 3, 4, 4, 3, 2, 3, 3], 0),
        ] {
            let mut g = Grid::new(Kind::ConnectFour);
            for i in moves {
                assert!(g.play(i));
            }
            assert_eq!(g.position().outcome, Outcome::Won(winner));
            assert_eq!(g.position().line.len(), 4);
            assert!(!g.play(6));
        }
    }
    #[test]
    fn connect_full_board_draw_is_final_and_replayable() {
        let mut g = Grid::new(Kind::ConnectFour);
        for c in [
            5, 3, 6, 5, 5, 0, 4, 6, 1, 0, 4, 3, 3, 2, 4, 6, 3, 1, 5, 0, 5, 4, 1, 6, 2, 5, 4, 3, 4,
            2, 6, 2, 0, 1, 2, 6, 2, 3, 0, 1, 0, 1,
        ] {
            assert!(g.play(c));
        }
        assert_eq!(g.position().outcome, Outcome::Draw);
        assert_eq!(g.scores(), [0, 0]);
        assert!(!g.play(0));
        assert!(g.valid(Kind::ConnectFour));
    }
    #[test]
    fn mastermind_feedback_counts_duplicates_only_once() {
        assert_eq!(
            feedback([0, 0, 1, 2], [0, 1, 0, 0]),
            Feedback {
                exact: 1,
                colour: 2
            }
        );
        assert_eq!(
            feedback([0, 0, 0, 0], [1, 0, 1, 0]),
            Feedback {
                exact: 2,
                colour: 0
            }
        );
        assert_eq!(
            feedback([0, 1, 2, 3], [3, 2, 1, 0]),
            Feedback {
                exact: 0,
                colour: 4
            }
        );
        for a in 0..1296 {
            let c = [
                (a / 216) as u8,
                (a / 36 % 6) as u8,
                (a / 6 % 6) as u8,
                (a % 6) as u8,
            ];
            for b in [0, 1, 42, 1295] {
                let g = [
                    (b / 216) as u8,
                    (b / 36 % 6) as u8,
                    (b / 6 % 6) as u8,
                    (b % 6) as u8,
                ];
                let f = feedback(c, g);
                assert!(f.exact + f.colour <= 4);
                assert_eq!(f, feedback(g, c));
            }
        }
    }
    #[test]
    fn mastermind_handoffs_never_reveal_code_and_win_scores_once() {
        let mut g = Mastermind::default();
        assert_eq!(g.slots(), [None; 4]);
        assert!(!g.set(0, Some(0)));
        g.reveal();
        assert!(!g.seal());
        for i in 0..4 {
            g.set(i, Some(i as u8));
        }
        assert!(!g.set(4, Some(0)));
        assert!(!g.set(0, Some(6)));
        assert!(g.seal());
        assert_eq!(g.slots(), [None; 4]);
        g.reveal();
        assert_eq!(g.slots(), [None; 4]);
        assert!(!g.submit());
        for i in 0..4 {
            g.set(i, Some(i as u8));
        }
        assert!(g.submit());
        assert!(g.solved());
        assert_eq!(g.phase(), MmPhase::Result);
        assert_eq!(g.scores(), [1, 0]);
        assert!(!g.submit());
        g.next_round();
        assert_eq!(g.maker(), 1);
        assert_eq!(g.scores(), [1, 0]);
        assert!(g.valid());
    }
    #[test]
    fn mastermind_attempt_limit_and_saved_drafts_survive_concealment() {
        let mut s = State::default();
        s.mastermind.reveal();
        for i in 0..4 {
            s.mastermind.set(i, Some(0));
        }
        s.mastermind.seal();
        s.mastermind.reveal();
        s.mastermind.set(1, Some(2));
        let mut r = State::decode(&s.encode());
        assert_eq!(r.mastermind.phase(), MmPhase::GuessCover);
        assert_eq!(r.mastermind.slots(), [None; 4]);
        r.mastermind.reveal();
        assert_eq!(r.mastermind.slots()[1], Some(2));
        for _ in 0..10 {
            for i in 0..4 {
                r.mastermind.set(i, Some(1));
            }
            assert!(r.mastermind.submit());
        }
        assert_eq!(r.mastermind.phase(), MmPhase::Result);
        assert!(!r.mastermind.solved());
        assert_eq!(r.mastermind.scores(), [10, 0]);
        assert!(r.mastermind.valid());
    }
    #[test]
    fn faces_private_setup_separate_elimination_boards_and_wrong_guess_loses() {
        let mut g = Faces::default();
        assert!(!g.choose(0));
        g.reveal();
        assert!(!g.seal());
        g.choose(0);
        g.seal();
        assert_eq!(g.own(), None);
        g.reveal();
        assert_eq!(g.own(), None);
        g.choose(1);
        g.seal();
        g.reveal();
        assert_eq!(g.own(), Some(0));
        g.toggle(1);
        g.pass();
        assert!(g.marks().iter().all(|&b| !b));
        g.reveal();
        assert!(g.marks().iter().all(|&b| !b));
        g.pass();
        g.reveal();
        assert!(g.marks()[1]);
        g.toggle(1);
        assert!(!g.marks()[1]);
        assert!(!g.guess(24));
        assert!(g.guess(3));
        assert_eq!(g.phase(), FacePhase::End { winner: 1 });
        assert_eq!(g.result(), Some([0, 1]));
        assert!(!g.guess(1));
        assert!(g.valid());
        g.next_round();
        assert_eq!(g.scores(), [0, 1]);
    }
    #[test]
    fn faces_correct_guess_and_reload_privacy() {
        let mut s = State::default();
        for id in [5, 9] {
            s.faces.reveal();
            s.faces.choose(id);
            s.faces.seal();
        }
        s.faces.reveal();
        assert_eq!(s.faces.own(), Some(5));
        let mut r = State::decode(&s.encode());
        assert_eq!(r.faces.own(), None);
        assert_eq!(r.faces.result(), None);
        r.faces.reveal();
        assert!(r.faces.guess(9));
        assert_eq!(r.faces.phase(), FacePhase::End { winner: 0 });
        assert_eq!(r.faces.scores(), [1, 0]);
    }
    #[test]
    fn corrupted_saves_reset_only_the_affected_game_and_bound_arrays() {
        let mut s = State::default();
        s.tic.play(0);
        let mut json = serde_json::to_value(&s).unwrap();
        json["connect"]["moves"] = serde_json::json!([99]);
        let r = State::decode(&json.to_string());
        assert_eq!(r.tic.position().cells[0], 1);
        assert!(r.connect.moves.is_empty());
        json["mastermind"]["maker"] = serde_json::json!(2);
        json["faces"]["identities"] = serde_json::json!([24, null]);
        let r = State::decode(&json.to_string());
        assert_eq!(r.mastermind.maker(), 0);
        assert_eq!(r.faces.phase(), Faces::default().phase());
        json["tic"]["moves"] = serde_json::json!([0, 3, 1, 4, 2, 5]);
        assert!(State::decode(&json.to_string()).tic.moves.is_empty());
        assert_eq!(State::decode("broken").tic.position().cells, vec![0; 9]);
        assert!(s.encode().len() < 16384);
    }
}
