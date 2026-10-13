//! Classic 10×10 naval duel. Private projections, verified fleets and turn history.
use serde::{Deserialize, Serialize};
pub const LENGTHS: [usize; 5] = [5, 4, 3, 3, 2];
pub const NAMES: [&str; 5] = [
    "Porta-aviões",
    "Encouraçado",
    "Cruzador",
    "Submarino",
    "Patrulha",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ship {
    pub x: u8,
    pub y: u8,
    pub vertical: bool,
}
impl Ship {
    pub fn cells(self, length: usize) -> Option<Vec<usize>> {
        let x = self.x as usize;
        let y = self.y as usize;
        if x >= 10
            || y >= 10
            || length == 0
            || length > 5
            || if self.vertical {
                y + length > 10
            } else {
                x + length > 10
            }
        {
            return None;
        }
        Some(
            (0..length)
                .map(|i| {
                    (y + if self.vertical { i } else { 0 }) * 10
                        + x
                        + if self.vertical { 0 } else { i }
                })
                .collect(),
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resume {
    Setup,
    Turn,
    Report,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Cover { player: u8, resume: Resume },
    Setup(u8),
    Turn(u8),
    Report(u8),
    End(u8),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Water,
    Ship,
    Miss,
    Hit,
    Sunk,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Result {
    Miss,
    Hit,
    Sunk(usize),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Shot {
    player: u8,
    cell: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Naval {
    pub phase: Phase,
    pub scores: [u32; 2],
    pub round: u32,
    fleets: [[Option<Ship>; 5]; 2],
    shots: Vec<Shot>,
}
pub struct View {
    pub player: u8,
    pub own: Vec<Tile>,
    pub target: Vec<Tile>,
    pub last: Option<(usize, Result)>,
    pub sunk: [bool; 5],
}
impl Default for Naval {
    fn default() -> Self {
        Self {
            phase: Phase::Cover {
                player: 0,
                resume: Resume::Setup,
            },
            scores: [0; 2],
            round: 1,
            fleets: [[None; 5]; 2],
            shots: vec![],
        }
    }
}
impl Naval {
    pub fn player(&self) -> u8 {
        match self.phase {
            Phase::Cover { player, .. }
            | Phase::Setup(player)
            | Phase::Turn(player)
            | Phase::Report(player)
            | Phase::End(player) => player,
        }
    }
    pub fn reveal(&mut self) -> bool {
        if let Phase::Cover { player, resume } = self.phase {
            self.phase = match resume {
                Resume::Setup => Phase::Setup(player),
                Resume::Turn => Phase::Turn(player),
                Resume::Report => Phase::Report(player),
            };
            true
        } else {
            false
        }
    }
    pub fn conceal(&mut self) -> bool {
        let (player, resume) = match self.phase {
            Phase::Setup(p) => (p, Resume::Setup),
            Phase::Turn(p) => (p, Resume::Turn),
            Phase::Report(p) => (p, Resume::Report),
            _ => return false,
        };
        self.phase = Phase::Cover { player, resume };
        true
    }
    pub fn placements(&self) -> Option<&[Option<Ship>; 5]> {
        if let Phase::Setup(p) = self.phase {
            Some(&self.fleets[p as usize])
        } else {
            None
        }
    }
    pub fn can_place(&self, index: usize, ship: Ship) -> bool {
        let Some(fleet) = self.placements() else {
            return false;
        };
        let Some(&length) = LENGTHS.get(index) else {
            return false;
        };
        let Some(cells) = ship.cells(length) else {
            return false;
        };
        !fleet.iter().enumerate().any(|(j, s)| {
            j != index
                && s.is_some_and(|s| {
                    s.cells(LENGTHS[j])
                        .unwrap_or_default()
                        .iter()
                        .any(|c| cells.contains(c))
                })
        })
    }
    pub fn place(&mut self, index: usize, ship: Ship) -> bool {
        if !self.can_place(index, ship) {
            return false;
        }
        let p = self.player() as usize;
        self.fleets[p][index] = Some(ship);
        true
    }
    pub fn clear(&mut self) -> bool {
        if let Phase::Setup(p) = self.phase {
            self.fleets[p as usize] = [None; 5];
            true
        } else {
            false
        }
    }
    pub fn automatic(&mut self, mut seed: u64) -> bool {
        if !self.clear() {
            return false;
        }
        let p = self.player() as usize;
        for i in 0..5 {
            let mut choices = vec![];
            for vertical in [false, true] {
                for y in 0..10 {
                    for x in 0..10 {
                        let s = Ship { x, y, vertical };
                        if self.can_place(i, s) {
                            choices.push(s);
                        }
                    }
                }
            }
            if choices.is_empty() {
                self.fleets[p] = [None; 5];
                return false;
            }
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.fleets[p][i] = Some(choices[(seed >> 32) as usize % choices.len()]);
        }
        true
    }
    pub fn ready(&self) -> bool {
        self.placements()
            .is_some_and(|f| f.iter().all(Option::is_some))
    }
    pub fn seal(&mut self) -> bool {
        if !self.ready() {
            return false;
        }
        self.phase = if self.player() == 0 {
            Phase::Cover {
                player: 1,
                resume: Resume::Setup,
            }
        } else {
            Phase::Cover {
                player: ((self.round - 1) % 2) as u8,
                resume: Resume::Turn,
            }
        };
        true
    }
    fn cells(&self, p: usize, i: usize) -> Vec<usize> {
        self.fleets[p][i]
            .and_then(|s| s.cells(LENGTHS[i]))
            .unwrap_or_default()
    }
    fn attacked(&self, p: usize, cell: usize) -> bool {
        self.shots
            .iter()
            .any(|s| s.player as usize == p && s.cell as usize == cell)
    }
    fn sunk(&self, p: usize, i: usize) -> bool {
        let cells = self.cells(p, i);
        !cells.is_empty() && cells.iter().all(|&c| self.attacked(1 - p, c))
    }
    fn result(&self, attacker: usize, cell: usize) -> Result {
        let target = 1 - attacker;
        for i in 0..5 {
            if self.cells(target, i).contains(&cell) {
                return if self.sunk(target, i) {
                    Result::Sunk(i)
                } else {
                    Result::Hit
                };
            }
        }
        Result::Miss
    }
    pub fn can_fire(&self, cell: usize) -> bool {
        cell < 100 && matches!(self.phase,Phase::Turn(p)if !self.attacked(p as usize,cell))
    }
    pub fn fire(&mut self, cell: usize) -> Option<Result> {
        if !self.can_fire(cell) {
            return None;
        }
        let p = self.player();
        self.shots.push(Shot {
            player: p,
            cell: cell as u8,
        });
        let result = self.result(p as usize, cell);
        if (0..5).all(|i| self.sunk(1 - p as usize, i)) {
            self.scores[p as usize] = self.scores[p as usize].saturating_add(1);
            self.phase = Phase::End(p);
        } else {
            self.phase = Phase::Report(p);
        }
        Some(result)
    }
    pub fn pass(&mut self) -> bool {
        if let Phase::Report(p) = self.phase {
            self.phase = Phase::Cover {
                player: 1 - p,
                resume: Resume::Turn,
            };
            true
        } else {
            false
        }
    }
    pub fn rematch(&mut self) {
        let scores = self.scores;
        let round = self.round.saturating_add(1);
        *self = Self::default();
        self.scores = scores;
        self.round = round;
    }
    fn board(&self, p: usize, show: bool) -> Vec<Tile> {
        (0..100)
            .map(|c| {
                let ship = (0..5).find(|&i| self.cells(p, i).contains(&c));
                let attacked = self.attacked(1 - p, c);
                match (ship, attacked) {
                    (Some(i), true) => {
                        if self.sunk(p, i) {
                            Tile::Sunk
                        } else {
                            Tile::Hit
                        }
                    }
                    (Some(_), false) if show => Tile::Ship,
                    (_, true) => Tile::Miss,
                    _ => Tile::Water,
                }
            })
            .collect()
    }
    pub fn view(&self) -> Option<View> {
        if !matches!(
            self.phase,
            Phase::Setup(_) | Phase::Turn(_) | Phase::Report(_)
        ) {
            return None;
        }
        let p = self.player() as usize;
        Some(View {
            player: p as u8,
            own: self.board(p, true),
            target: self.board(1 - p, false),
            last: self
                .shots
                .last()
                .filter(|s| s.player as usize == p)
                .map(|s| (s.cell as usize, self.result(p, s.cell as usize))),
            sunk: std::array::from_fn(|i| self.sunk(1 - p, i)),
        })
    }
    pub fn revealed_board(&self, p: usize) -> Option<Vec<Tile>> {
        if p > 1 || !matches!(self.phase, Phase::End(_)) {
            None
        } else {
            Some(self.board(p, true))
        }
    }
    pub fn valid(&self) -> bool {
        if self.round == 0 || self.player() > 1 || self.shots.len() > 200 {
            return false;
        }
        for fleet in &self.fleets {
            let mut seen = [false; 100];
            for (i, s) in fleet.iter().enumerate() {
                if let Some(s) = s {
                    let Some(cells) = s.cells(LENGTHS[i]) else {
                        return false;
                    };
                    for c in cells {
                        if seen[c] {
                            return false;
                        }
                        seen[c] = true;
                    }
                }
            }
        }
        let setup = matches!(
            self.phase,
            Phase::Setup(_)
                | Phase::Cover {
                    resume: Resume::Setup,
                    ..
                }
        );
        if setup {
            return self.shots.is_empty()
                && if self.player() == 0 {
                    self.fleets[1].iter().all(Option::is_none)
                } else {
                    self.fleets[0].iter().all(Option::is_some)
                };
        }
        if !self.fleets.iter().all(|f| f.iter().all(Option::is_some)) {
            return false;
        }
        let mut replay = self.clone();
        replay.shots.clear();
        replay.phase = Phase::Turn(((self.round - 1) % 2) as u8);
        replay.scores = [0; 2];
        for s in &self.shots {
            if s.player != replay.player() || replay.fire(s.cell as usize).is_none() {
                return false;
            }
            if !matches!(replay.phase, Phase::End(_)) {
                replay.pass();
                replay.reveal();
            }
        }
        match self.phase {
            Phase::End(p) => replay.phase == Phase::End(p) && self.scores[p as usize] > 0,
            Phase::Turn(p)
            | Phase::Cover {
                player: p,
                resume: Resume::Turn,
            } => replay.phase == Phase::Turn(p),
            Phase::Report(p)
            | Phase::Cover {
                player: p,
                resume: Resume::Report,
            } => {
                matches!(replay.phase, Phase::Turn(_))
                    && self.shots.last().is_some_and(|s| s.player == p)
            }
            _ => false,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Naval {
        let mut g = Naval::default();
        for p in 0..2 {
            g.reveal();
            assert_eq!(g.player(), p);
            for i in 0..5 {
                assert!(g.place(
                    i,
                    Ship {
                        x: 0,
                        y: i as u8,
                        vertical: false
                    }
                ));
            }
            assert!(g.seal());
        }
        g.reveal();
        g
    }
    #[test]
    fn placement_rules_and_secret_projections() {
        let mut g = Naval::default();
        assert!(g.view().is_none());
        assert!(!g.place(
            0,
            Ship {
                x: 0,
                y: 0,
                vertical: false
            }
        ));
        g.reveal();
        assert!(!g.seal());
        assert!(!g.place(
            0,
            Ship {
                x: 7,
                y: 0,
                vertical: false
            }
        ));
        assert!(g.place(
            0,
            Ship {
                x: 0,
                y: 0,
                vertical: false
            }
        ));
        assert!(!g.place(
            1,
            Ship {
                x: 2,
                y: 0,
                vertical: true
            }
        ));
        assert!(g.place(
            1,
            Ship {
                x: 0,
                y: 1,
                vertical: false
            }
        ));
        assert!(g.valid());
        g.conceal();
        assert!(g.view().is_none());
        assert!(g.placements().is_none());
        assert!(g.revealed_board(0).is_none());
        let g = game();
        let v = g.view().unwrap();
        assert_eq!(v.own.iter().filter(|&&t| t == Tile::Ship).count(), 17);
        assert!(v.target.iter().all(|&t| t == Tile::Water));
        assert!(g.valid());
    }
    #[test]
    fn miss_hit_sunk_alternate_turns_and_prevent_duplicate_shots() {
        let mut g = game();
        assert_eq!(g.fire(0), Some(Result::Hit));
        assert!(g.fire(1).is_none());
        g.conceal();
        assert!(g.view().is_none());
        g.reveal();
        assert_eq!(g.phase, Phase::Report(0));
        g.pass();
        assert!(g.view().is_none());
        g.reveal();
        assert_eq!(g.fire(99), Some(Result::Miss));
        g.pass();
        g.reveal();
        assert!(g.fire(0).is_none());
        for c in 1..5 {
            assert_eq!(
                g.fire(c),
                Some(if c == 4 { Result::Sunk(0) } else { Result::Hit })
            );
            g.pass();
            g.reveal();
            g.fire(99 - c).unwrap();
            g.pass();
            g.reveal();
        }
        assert!(
            g.view().unwrap().target[..5]
                .iter()
                .all(|&t| t == Tile::Sunk)
        );
        assert!(g.valid());
    }
    #[test]
    fn full_victory_rematch_and_invalid_history() {
        let mut g = game();
        let cells: Vec<_> = (0..5).flat_map(|i| g.cells(1, i)).collect();
        for (i, c) in cells.into_iter().enumerate() {
            g.fire(c).unwrap();
            if matches!(g.phase, Phase::End(_)) {
                break;
            }
            g.pass();
            g.reveal();
            g.fire(99 - i).unwrap();
            g.pass();
            g.reveal();
        }
        assert_eq!(g.phase, Phase::End(0));
        assert_eq!(g.scores, [1, 0]);
        assert!(g.valid());
        assert!(g.fire(50).is_none());
        assert!(g.revealed_board(1).is_some());
        let mut bad = g.clone();
        bad.shots.push(Shot {
            player: 1,
            cell: 50,
        });
        assert!(!bad.valid());
        g.rematch();
        assert_eq!(g.scores, [1, 0]);
        assert_eq!(g.round, 2);
        assert!(g.valid());
    }
    #[test]
    fn automatic_fleets_are_legal_for_many_seeds() {
        for seed in 0..400 {
            let mut g = Naval::default();
            g.reveal();
            assert!(g.automatic(seed));
            assert!(g.ready());
            assert!(g.valid());
            g.seal();
            g.reveal();
            assert!(g.automatic(seed + 1));
            assert!(g.valid());
            g.seal();
            assert!(g.valid());
        }
    }
}
