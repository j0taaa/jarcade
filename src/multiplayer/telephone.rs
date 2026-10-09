//! Private simultaneous prompt/drawing chains. Drawings use bounded vector data.
use super::{Rng, clean_text};
use serde::{Deserialize, Serialize};

pub const MAX_STROKES: usize = 128;
pub const MAX_POINTS: usize = 4096;
pub const PALETTE: [[u8; 3]; 8] = [
    [39, 43, 59],
    [236, 102, 119],
    [248, 179, 64],
    [84, 170, 133],
    [83, 145, 213],
    [147, 114, 205],
    [168, 114, 84],
    [255, 255, 255],
];
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stroke {
    pub color: u8,
    pub width: u8,
    /// Coordinates in a 1000 × 750 canvas, independent of display density.
    pub points: Vec<[u16; 2]>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Drawing {
    pub strokes: Vec<Stroke>,
}
impl Drawing {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.strokes.is_empty() || self.strokes.len() > MAX_STROKES {
            return Err("Draw something first (up to 128 strokes)");
        }
        if self.strokes.iter().map(|s| s.points.len()).sum::<usize>() > MAX_POINTS {
            return Err("This drawing has too many points");
        }
        for s in &self.strokes {
            if s.color as usize >= PALETTE.len()
                || !(2..=24).contains(&s.width)
                || s.points.is_empty()
                || s.points.iter().any(|p| p[0] > 1000 || p[1] > 750)
            {
                return Err("Invalid brush or drawing coordinates");
            }
        }
        if self.strokes.iter().all(|s| s.color == 7) {
            return Err("Draw something in colour first");
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Content {
    Text(String),
    Drawing(Drawing),
    Skipped,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub author: usize,
    pub content: Content,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Play,
    Reveal,
    Finished,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Move {
    Text { text: String },
    Draw { drawing: Drawing },
    Next,
    Previous,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct View {
    pub phase: Phase,
    pub stage: usize,
    pub rounds: usize,
    pub submitted: Vec<bool>,
    pub left: Vec<bool>,
    /// Only the preceding contribution in your current chain, never other chains.
    pub task: Option<Content>,
    pub album: usize,
    pub step: usize,
    pub owner: usize,
    /// Only the currently revealed contribution, keeping payloads bounded.
    pub entry: Option<Entry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    order: Vec<usize>,
    chains: Vec<Vec<Entry>>,
    submitted: Vec<bool>,
    left: Vec<bool>,
    pub stage: usize,
    pub phase: Phase,
    pub cursor: usize,
}
impl Game {
    pub fn new(players: usize, seed: u64) -> Result<Self, &'static str> {
        if !(3..=12).contains(&players) {
            return Err("Invite 3–12 players");
        }
        let mut order: Vec<_> = (0..players).collect();
        Rng::new(seed).shuffle(&mut order);
        Ok(Self {
            order,
            chains: vec![vec![]; players],
            submitted: vec![false; players],
            left: vec![false; players],
            stage: 0,
            phase: Phase::Play,
            cursor: 0,
        })
    }
    fn chain(&self, player: usize) -> usize {
        let n = self.order.len();
        (self.order.iter().position(|&p| p == player).unwrap() + n - self.stage % n) % n
    }
    pub fn finished(&self) -> bool {
        self.phase == Phase::Finished
    }
    pub fn phase_key(&self) -> String {
        format!("{:?}:{}:{}", self.phase, self.stage, self.cursor)
    }
    pub fn view(&self, player: usize) -> View {
        let n = self.order.len();
        let album = self.cursor / n;
        let step = self.cursor % n;
        View {
            phase: self.phase,
            stage: self.stage,
            rounds: n,
            submitted: self.submitted.clone(),
            left: self.left.clone(),
            task: if self.phase == Phase::Play && !self.submitted[player] && self.stage > 0 {
                self.chains[self.chain(player)]
                    .get(self.stage - 1)
                    .map(|e| e.content.clone())
            } else {
                None
            },
            album,
            step,
            owner: self.order[album],
            entry: if self.phase != Phase::Play {
                self.chains[album].get(step).cloned()
            } else {
                None
            },
        }
    }
    pub fn play(&mut self, player: usize, movement: Move) -> Result<(), &'static str> {
        if player >= self.order.len() || self.left[player] {
            return Err("Your seat is not active");
        }
        match movement {
            Move::Next | Move::Previous => {
                if self.phase == Phase::Play {
                    return Err("The albums are still secret");
                }
                match movement {
                    Move::Next if self.cursor + 1 < self.order.len().pow(2) => self.cursor += 1,
                    Move::Next => self.phase = Phase::Finished,
                    Move::Previous => self.cursor = self.cursor.saturating_sub(1),
                    _ => unreachable!(),
                }
            }
            _ => {
                if self.phase != Phase::Play || self.submitted[player] {
                    return Err("This turn is already sealed");
                }
                let content = match movement {
                    Move::Text { text } if self.stage.is_multiple_of(2) => {
                        let text = clean_text(&text, 160);
                        if text.is_empty() {
                            return Err("Write a short sentence first");
                        }
                        Content::Text(text)
                    }
                    Move::Draw { drawing } if !self.stage.is_multiple_of(2) => {
                        drawing.validate()?;
                        Content::Drawing(drawing)
                    }
                    _ => return Err("That is not your task this turn"),
                };
                let chain = self.chain(player);
                self.chains[chain].push(Entry {
                    author: player,
                    content,
                });
                self.submitted[player] = true;
                self.advance();
            }
        }
        Ok(())
    }
    fn advance(&mut self) {
        loop {
            for player in 0..self.order.len() {
                if self.left[player] && !self.submitted[player] {
                    let chain = self.chain(player);
                    self.chains[chain].push(Entry {
                        author: player,
                        content: Content::Skipped,
                    });
                    self.submitted[player] = true;
                }
            }
            if !self.submitted.iter().all(|&s| s) {
                break;
            }
            self.stage += 1;
            if self.stage == self.order.len() {
                self.phase = Phase::Reveal;
                break;
            }
            self.submitted.fill(false);
        }
    }
    pub fn forfeit(&mut self, player: usize) {
        if player >= self.left.len() {
            return;
        }
        self.left[player] = true;
        if self.phase == Phase::Play {
            self.advance();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn drawing() -> Drawing {
        Drawing {
            strokes: vec![Stroke {
                color: 1,
                width: 6,
                points: vec![[0, 0], [1000, 750]],
            }],
        }
    }
    fn submit(g: &mut Game, p: usize) {
        let m = if g.stage.is_multiple_of(2) {
            Move::Text {
                text: format!("private player {p}"),
            }
        } else {
            Move::Draw { drawing: drawing() }
        };
        g.play(p, m).unwrap();
    }
    #[test]
    fn chains_visit_every_player_and_hide_submissions() {
        for n in 3..=12 {
            let mut g = Game::new(n, 9).unwrap();
            for stage in 0..n {
                let phase = g.phase_key();
                for p in 0..n {
                    let v = g.view(p);
                    assert!(v.entry.is_none());
                    assert_eq!(v.task.is_some(), stage > 0);
                    submit(&mut g, p);
                    if p + 1 < n || stage + 1 == n {
                        assert!(g.view(p).task.is_none());
                    }
                    if p + 1 < n {
                        assert!(
                            g.play(
                                p,
                                Move::Text {
                                    text: "again".into()
                                }
                            )
                            .is_err()
                        );
                    }
                    if p + 1 < n {
                        assert_eq!(g.phase_key(), phase);
                    }
                }
            }
            assert_eq!(g.phase, Phase::Reveal);
            for chain in &g.chains {
                let mut authors: Vec<_> = chain.iter().map(|e| e.author).collect();
                authors.sort();
                assert_eq!(authors, (0..n).collect::<Vec<_>>());
            }
            for _ in 0..n * n {
                g.play(0, Move::Next).unwrap();
            }
            assert!(g.finished());
        }
    }
    #[test]
    fn invalid_moves_are_atomic_and_drawings_bounded() {
        let mut g = Game::new(3, 1).unwrap();
        for m in [
            Move::Next,
            Move::Draw { drawing: drawing() },
            Move::Text { text: " \n".into() },
        ] {
            assert!(g.play(0, m).is_err());
            assert!(!g.submitted[0]);
        }
        assert!(Drawing::default().validate().is_err());
        let mut d = drawing();
        d.strokes[0].points[0] = [1001, 0];
        assert!(d.validate().is_err());
        d = drawing();
        d.strokes[0].color = 8;
        assert!(d.validate().is_err());
        d = drawing();
        d.strokes[0].width = 0;
        assert!(d.validate().is_err());
        d = drawing();
        d.strokes[0].points = vec![[1000, 750]; MAX_POINTS];
        assert!(d.validate().is_ok());
        assert!(serde_json::to_vec(&d).unwrap().len() < 65536);
        d.strokes[0].points.push([0, 0]);
        assert!(d.validate().is_err());
        let d = Drawing {
            strokes: vec![
                Stroke {
                    color: 1,
                    width: 24,
                    points: vec![[1000, 750]; MAX_POINTS / MAX_STROKES]
                };
                MAX_STROKES
            ],
        };
        assert!(d.validate().is_ok());
        let wire = super::super::ClientMessage::Play {
            revision: u64::MAX,
            command: super::super::Command::Telephone(Move::Draw { drawing: d }),
        };
        assert!(serde_json::to_vec(&wire).unwrap().len() < 65536);
    }
    #[test]
    fn explicit_leavers_are_skipped_and_saved_tasks_resume() {
        let mut g = Game::new(3, 9).unwrap();
        submit(&mut g, 0);
        g.forfeit(1);
        submit(&mut g, 2);
        let mut g: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(g.stage, 1);
        assert!(g.view(0).task.is_some());
        submit(&mut g, 0);
        submit(&mut g, 2);
        submit(&mut g, 0);
        submit(&mut g, 2);
        assert_eq!(g.phase, Phase::Reveal);
        assert_eq!(
            g.chains
                .iter()
                .flatten()
                .filter(|e| e.content == Content::Skipped)
                .count(),
            3
        );
        let mut g = Game::new(3, 0).unwrap();
        for p in 0..3 {
            g.forfeit(p);
        }
        assert_eq!(g.phase, Phase::Reveal);
    }
}
