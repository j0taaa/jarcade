//! Secret submissions, simultaneous votes, and the 3-player/two-decoy variant.
use super::{Rng, clean_text};
use serde::{Deserialize, Serialize};
pub const CARD_COUNT: u8 = 84;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Move {
    Story { card: u8, clue: String },
    Submit { cards: Vec<u8> },
    Vote { card: u8 },
    Next,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
enum Phase {
    Story,
    Submit,
    Vote,
    Results,
    Finished,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub score: u32,
    hand: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub players: Vec<Player>,
    pub storyteller: usize,
    pub round: u32,
    phase: Phase,
    deck: Vec<u8>,
    discard: Vec<u8>,
    rng: Rng,
    clue: String,
    submissions: Vec<Vec<u8>>,
    table: Vec<(u8, usize)>,
    votes: Vec<Option<u8>>,
    ready: Vec<bool>,
    gained: Vec<u32>,
    ended: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub name: String,
    pub score: u32,
    pub done: bool,
    pub gained: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableCard {
    pub card: u8,
    pub own: bool,
    pub owner: Option<usize>,
    pub story: bool,
    pub votes: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct View {
    pub players: Vec<PlayerView>,
    pub storyteller: usize,
    pub round: u32,
    pub phase: String,
    pub clue: String,
    pub hand: Vec<u8>,
    pub table: Vec<TableCard>,
    pub pick: usize,
    pub can_play: bool,
    pub vote: Option<u8>,
    pub winners: Vec<usize>,
    pub ended: Option<String>,
}
impl Game {
    pub fn new(names: Vec<String>, seed: u64) -> Result<Self, &'static str> {
        let n = names.len();
        if !(3..=8).contains(&n) {
            return Err("Reverie needs 3–8 players");
        }
        let mut rng = Rng::new(seed);
        let mut deck: Vec<_> = (0..CARD_COUNT).collect();
        rng.shuffle(&mut deck);
        let players = names
            .into_iter()
            .map(|name| Player {
                name,
                score: 0,
                hand: (0..if n == 3 { 7 } else { 6 })
                    .map(|_| deck.pop().unwrap())
                    .collect(),
            })
            .collect();
        Ok(Self {
            players,
            storyteller: 0,
            round: 1,
            phase: Phase::Story,
            deck,
            discard: vec![],
            rng,
            clue: String::new(),
            submissions: vec![vec![]; n],
            table: vec![],
            votes: vec![None; n],
            ready: vec![false; n],
            gained: vec![0; n],
            ended: None,
        })
    }
    pub fn finished(&self) -> bool {
        matches!(self.phase, Phase::Finished)
    }
    pub fn play(&mut self, who: usize, movement: Move) -> Result<(), &'static str> {
        if who >= self.players.len() || self.finished() {
            return Err("This table has ended");
        }
        match (&self.phase, movement) {
            (Phase::Story, Move::Story { card, clue }) => {
                let clue = clean_text(&clue, 160);
                if who != self.storyteller
                    || clue.is_empty()
                    || !self.players[who].hand.contains(&card)
                {
                    return Err("Choose your card and enter a clue");
                }
                self.clue = clue;
                self.submissions[who] = vec![card];
                self.players[who].hand.retain(|c| *c != card);
                self.phase = Phase::Submit;
            }
            (Phase::Submit, Move::Submit { cards }) => {
                let n = if self.players.len() == 3 { 2 } else { 1 };
                let mut unique = cards.clone();
                unique.sort_unstable();
                unique.dedup();
                if who == self.storyteller
                    || !self.submissions[who].is_empty()
                    || cards.len() != n
                    || unique.len() != n
                    || cards.iter().any(|c| !self.players[who].hand.contains(c))
                {
                    return Err("Choose the requested cards from your hand");
                }
                self.players[who].hand.retain(|c| !cards.contains(c));
                self.submissions[who] = cards;
                if self.submissions.iter().all(|s| !s.is_empty()) {
                    self.table = self
                        .submissions
                        .iter()
                        .enumerate()
                        .flat_map(|(i, cards)| cards.iter().map(move |&c| (c, i)))
                        .collect();
                    self.rng.shuffle(&mut self.table);
                    self.phase = Phase::Vote;
                }
            }
            (Phase::Vote, Move::Vote { card }) => {
                if who == self.storyteller
                    || self.votes[who].is_some()
                    || !self.table.iter().any(|&(c, p)| c == card && p != who)
                {
                    return Err("Vote for another player’s card");
                }
                self.votes[who] = Some(card);
                if self
                    .votes
                    .iter()
                    .enumerate()
                    .all(|(i, v)| i == self.storyteller || v.is_some())
                {
                    self.score();
                }
            }
            (Phase::Results, Move::Next) => {
                if self.ready[who] {
                    return Err("You are already ready");
                }
                self.ready[who] = true;
                if self.ready.iter().all(|r| *r) {
                    self.next();
                }
            }
            _ => return Err("That move is no longer available"),
        }
        Ok(())
    }
    fn score(&mut self) {
        let story = self.submissions[self.storyteller][0];
        let correct = self.votes.iter().filter(|v| **v == Some(story)).count();
        self.gained.fill(0);
        if correct == 0 || correct == self.players.len() - 1 {
            for (i, g) in self.gained.iter_mut().enumerate() {
                if i != self.storyteller {
                    *g = 2;
                }
            }
        } else {
            self.gained[self.storyteller] = 3;
            for (i, v) in self.votes.iter().enumerate() {
                if *v == Some(story) {
                    self.gained[i] = 3;
                }
            }
        }
        for &vote in self.votes.iter().flatten() {
            let owner = self.table.iter().find(|&&(c, _)| c == vote).unwrap().1;
            if owner != self.storyteller {
                self.gained[owner] += 1;
            }
        }
        for (p, g) in self.players.iter_mut().zip(&self.gained) {
            p.score += g;
        }
        self.phase = if self.players.iter().any(|p| p.score >= 30) {
            Phase::Finished
        } else {
            Phase::Results
        };
    }
    fn next(&mut self) {
        self.discard.extend(self.table.iter().map(|&(c, _)| c));
        let target = if self.players.len() == 3 { 7 } else { 6 };
        let need = self
            .players
            .iter()
            .map(|p| target - p.hand.len())
            .sum::<usize>();
        if self.deck.len() < need {
            self.deck.append(&mut self.discard);
            self.rng.shuffle(&mut self.deck);
        }
        for p in &mut self.players {
            while p.hand.len() < target {
                p.hand.push(
                    self.deck
                        .pop()
                        .expect("84 cards exceed the largest combined hand"),
                );
            }
        }
        self.storyteller = (self.storyteller + 1) % self.players.len();
        self.round += 1;
        self.clue.clear();
        self.table.clear();
        self.submissions.iter_mut().for_each(Vec::clear);
        self.votes.fill(None);
        self.ready.fill(false);
        self.gained.fill(0);
        self.phase = Phase::Story;
    }
    pub fn end_on_leave(&mut self, who: usize) {
        if !self.finished() {
            self.ended = Some(format!("{} left the table", self.players[who].name));
            self.phase = Phase::Finished;
        }
    }
    pub fn view(&self, you: usize) -> View {
        let revealed = matches!(self.phase, Phase::Results | Phase::Finished);
        let story = self.submissions[self.storyteller].first().copied();
        let phase = match self.phase {
            Phase::Story => "story",
            Phase::Submit => "submit",
            Phase::Vote => "vote",
            Phase::Results => "results",
            Phase::Finished => "finished",
        };
        let max = self.players.iter().map(|p| p.score).max().unwrap_or(0);
        View {
            players: self
                .players
                .iter()
                .enumerate()
                .map(|(i, p)| PlayerView {
                    name: p.name.clone(),
                    score: p.score,
                    done: match self.phase {
                        Phase::Story => false,
                        Phase::Submit => !self.submissions[i].is_empty(),
                        Phase::Vote => i == self.storyteller || self.votes[i].is_some(),
                        Phase::Results => self.ready[i],
                        Phase::Finished => true,
                    },
                    gained: if revealed { self.gained[i] } else { 0 },
                })
                .collect(),
            storyteller: self.storyteller,
            round: self.round,
            phase: phase.into(),
            clue: self.clue.clone(),
            hand: self.players[you].hand.clone(),
            table: if matches!(self.phase, Phase::Story | Phase::Submit) {
                vec![]
            } else {
                self.table
                    .iter()
                    .map(|&(card, owner)| TableCard {
                        card,
                        own: owner == you,
                        owner: revealed.then_some(owner),
                        story: revealed && story == Some(card),
                        votes: if revealed {
                            self.votes
                                .iter()
                                .enumerate()
                                .filter(|(_, v)| **v == Some(card))
                                .map(|(i, _)| i)
                                .collect()
                        } else {
                            vec![]
                        },
                    })
                    .collect()
            },
            pick: if matches!(self.phase, Phase::Submit) && self.players.len() == 3 {
                2
            } else {
                1
            },
            can_play: match self.phase {
                Phase::Story => you == self.storyteller,
                Phase::Submit => you != self.storyteller && self.submissions[you].is_empty(),
                Phase::Vote => you != self.storyteller && self.votes[you].is_none(),
                Phase::Results => !self.ready[you],
                Phase::Finished => false,
            },
            vote: self.votes[you],
            winners: if self.finished() && self.ended.is_none() {
                self.players
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.score == max)
                    .map(|(i, _)| i)
                    .collect()
            } else {
                vec![]
            },
            ended: self.ended.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(n: usize) -> Game {
        Game::new((0..n).map(|i| format!("P{i}")).collect(), 9).unwrap()
    }
    fn submit(g: &mut Game) -> u8 {
        let teller = g.storyteller;
        let story = g.players[teller].hand[0];
        g.play(
            teller,
            Move::Story {
                card: story,
                clue: "The last light".into(),
            },
        )
        .unwrap();
        for p in 0..g.players.len() {
            if p != teller {
                let cards = g.players[p].hand[..if g.players.len() == 3 { 2 } else { 1 }].to_vec();
                g.play(p, Move::Submit { cards }).unwrap();
            }
        }
        story
    }
    #[test]
    fn three_player_variant_and_private_votes() {
        let mut g = game(3);
        assert!(g.players.iter().all(|p| p.hand.len() == 7));
        let card = g.players[0].hand[0];
        g.play(
            0,
            Move::Story {
                card,
                clue: "A quiet sea".into(),
            },
        )
        .unwrap();
        assert!(g.view(1).table.is_empty());
        assert!(
            g.play(
                1,
                Move::Submit {
                    cards: vec![g.players[1].hand[0]]
                }
            )
            .is_err()
        );
        for p in 1..3 {
            g.play(
                p,
                Move::Submit {
                    cards: g.players[p].hand[..2].to_vec(),
                },
            )
            .unwrap();
        }
        assert_eq!(g.table.len(), 5);
        let own = g.submissions[1][0];
        assert!(g.play(1, Move::Vote { card: own }).is_err());
        g.play(1, Move::Vote { card }).unwrap();
        let v = g.view(2);
        assert!(
            v.table
                .iter()
                .all(|c| c.owner.is_none() && !c.story && c.votes.is_empty())
        );
        assert_eq!(v.vote, None);
        g.play(2, Move::Vote { card }).unwrap();
        assert_eq!(g.gained, vec![0, 2, 2]);
        assert!(g.view(1).table.iter().all(|c| c.owner.is_some()));
    }
    #[test]
    fn mixed_all_and_none_scoring_with_decoy_bonuses() {
        let mut g = game(4);
        let story = submit(&mut g);
        let decoy = g.submissions[1][0];
        g.play(1, Move::Vote { card: story }).unwrap();
        g.play(2, Move::Vote { card: decoy }).unwrap();
        g.play(3, Move::Vote { card: decoy }).unwrap();
        assert_eq!(g.gained, vec![3, 5, 0, 0]);
        let mut g = game(4);
        let story = submit(&mut g);
        for p in 1..4 {
            g.play(p, Move::Vote { card: story }).unwrap();
        }
        assert_eq!(g.gained, vec![0, 2, 2, 2]);
        let mut g = game(4);
        submit(&mut g);
        g.play(
            1,
            Move::Vote {
                card: g.submissions[2][0],
            },
        )
        .unwrap();
        g.play(
            2,
            Move::Vote {
                card: g.submissions[1][0],
            },
        )
        .unwrap();
        g.play(
            3,
            Move::Vote {
                card: g.submissions[1][0],
            },
        )
        .unwrap();
        assert_eq!(g.gained, vec![0, 4, 3, 2]);
    }
    #[test]
    fn invalid_choices_are_atomic_and_serialized_progress_is_identical() {
        let mut g = game(3);
        let before = serde_json::to_string(&g).unwrap();
        assert!(
            g.play(
                1,
                Move::Story {
                    card: g.players[1].hand[0],
                    clue: "X".into()
                }
            )
            .is_err()
        );
        assert_eq!(before, serde_json::to_string(&g).unwrap());
        let story = submit(&mut g);
        let mut copy: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        for p in 1..3 {
            g.play(p, Move::Vote { card: story }).unwrap();
            copy.play(p, Move::Vote { card: story }).unwrap();
        }
        assert_eq!(
            serde_json::to_string(&g).unwrap(),
            serde_json::to_string(&copy).unwrap()
        );
    }
    #[test]
    fn deck_recycles_and_teller_rotates_until_thirty() {
        for n in 3..=8 {
            let mut g = game(n);
            let mut rounds = 0;
            while !g.finished() {
                let story = submit(&mut g);
                for p in 0..n {
                    if p != g.storyteller {
                        g.play(p, Move::Vote { card: story }).unwrap();
                    }
                }
                rounds += 1;
                let mut cards = g.deck.clone();
                cards.extend(&g.discard);
                cards.extend(g.players.iter().flat_map(|p| p.hand.iter()).copied());
                cards.extend(g.table.iter().map(|&(c, _)| c));
                cards.sort_unstable();
                assert_eq!(cards, (0..CARD_COUNT).collect::<Vec<_>>());
                if !g.finished() {
                    for p in 0..n {
                        g.play(p, Move::Next).unwrap();
                    }
                    assert_eq!(g.storyteller, rounds % n);
                    assert!(
                        g.players
                            .iter()
                            .all(|p| p.hand.len() == if n == 3 { 7 } else { 6 })
                    );
                }
                assert!(rounds < 30);
            }
            assert!(
                g.view(0).winners.iter().all(
                    |&p| g.players[p].score == g.players.iter().map(|p| p.score).max().unwrap()
                )
            );
        }
    }
}
