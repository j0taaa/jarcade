//! Original characters, with the base game's seven actions and five role powers.
use super::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Regent,
    Shade,
    Corsair,
    Envoy,
    Sentinel,
}
impl Role {
    pub const ALL: [Self; 5] = [
        Self::Regent,
        Self::Shade,
        Self::Corsair,
        Self::Envoy,
        Self::Sentinel,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Regent => "Regent",
            Self::Shade => "Shade",
            Self::Corsair => "Corsair",
            Self::Envoy => "Envoy",
            Self::Sentinel => "Sentinel",
        }
    }
    pub fn power(self) -> &'static str {
        match self {
            Self::Regent => "Tax +3 · blocks Aid",
            Self::Shade => "Assassinate · costs 3",
            Self::Corsair => "Steal 2 · blocks Steal",
            Self::Envoy => "Exchange · blocks Steal",
            Self::Sentinel => "Blocks Assassination",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Income,
    Aid,
    Tax,
    Assassinate,
    Steal,
    Exchange,
    Coup,
}
impl Action {
    pub const ALL: [Self; 7] = [
        Self::Income,
        Self::Aid,
        Self::Tax,
        Self::Exchange,
        Self::Steal,
        Self::Assassinate,
        Self::Coup,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Income => "Income",
            Self::Aid => "Foreign aid",
            Self::Tax => "Tax",
            Self::Assassinate => "Assassinate",
            Self::Steal => "Steal",
            Self::Exchange => "Exchange",
            Self::Coup => "Coup",
        }
    }
    pub fn role(self) -> Option<Role> {
        match self {
            Self::Tax => Some(Role::Regent),
            Self::Assassinate => Some(Role::Shade),
            Self::Steal => Some(Role::Corsair),
            Self::Exchange => Some(Role::Envoy),
            _ => None,
        }
    }
    pub fn targeted(self) -> bool {
        matches!(self, Self::Assassinate | Self::Steal | Self::Coup)
    }
    pub fn cost(self) -> u32 {
        match self {
            Self::Assassinate => 3,
            Self::Coup => 7,
            _ => 0,
        }
    }
    pub fn blocks(self) -> &'static [Role] {
        match self {
            Self::Aid => &[Role::Regent],
            Self::Assassinate => &[Role::Sentinel],
            Self::Steal => &[Role::Corsair, Role::Envoy],
            _ => &[],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Move {
    Act {
        action: Action,
        target: Option<usize>,
    },
    Pass,
    Challenge,
    Block {
        role: Role,
    },
    Lose {
        card: usize,
    },
    Keep {
        cards: Vec<usize>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Influence {
    pub role: Role,
    pub revealed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub coins: u32,
    pub cards: Vec<Influence>,
}
impl Player {
    fn alive(&self) -> usize {
        self.cards.iter().filter(|c| !c.revealed).count()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pending {
    pub actor: usize,
    pub action: Action,
    pub target: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
enum Stage {
    Claim {
        role: Role,
        claimant: usize,
        block: bool,
    },
    Blocks,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
enum After {
    End,
    Resolve(Pending),
    Blocks(Pending),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
enum Phase {
    Turn,
    Respond {
        pending: Pending,
        stage: Stage,
        passed: Vec<usize>,
    },
    Loss {
        player: usize,
        after: After,
    },
    Exchange {
        player: usize,
        pool: Vec<Role>,
    },
    Finished {
        winner: usize,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub players: Vec<Player>,
    pub turn: usize,
    deck: Vec<Role>,
    rng: Rng,
    phase: Phase,
    log: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardView {
    pub role: Option<Role>,
    pub revealed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub name: String,
    pub coins: u32,
    pub cards: Vec<CardView>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub label: String,
    pub command: Move,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct View {
    pub players: Vec<PlayerView>,
    pub turn: usize,
    pub phase: String,
    pub prompt: String,
    pub pending: Option<Pending>,
    pub choices: Vec<Choice>,
    pub actions: Vec<Action>,
    pub exchange: Vec<Role>,
    pub keep: usize,
    pub winner: Option<usize>,
    pub log: Vec<String>,
}
impl Game {
    pub fn new(names: Vec<String>, seed: u64) -> Result<Self, &'static str> {
        if !(2..=6).contains(&names.len()) {
            return Err("Court needs 2–6 players");
        }
        let mut rng = Rng::new(seed);
        let mut deck = Role::ALL.repeat(3);
        rng.shuffle(&mut deck);
        let two = names.len() == 2;
        let players = names
            .into_iter()
            .enumerate()
            .map(|(i, name)| Player {
                name,
                coins: if two && i == 0 { 1 } else { 2 },
                cards: (0..2)
                    .map(|_| Influence {
                        role: deck.pop().unwrap(),
                        revealed: false,
                    })
                    .collect(),
            })
            .collect();
        Ok(Self {
            players,
            turn: 0,
            deck,
            rng,
            phase: Phase::Turn,
            log: vec!["Two influences. One survivor.".into()],
        })
    }
    pub fn finished(&self) -> bool {
        matches!(self.phase, Phase::Finished { .. })
    }
    fn event(&mut self, text: String) {
        self.log.push(text);
        if self.log.len() > 8 {
            self.log.remove(0);
        }
    }
    fn winner(&mut self) -> bool {
        let alive: Vec<_> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.alive() > 0)
            .map(|(i, _)| i)
            .collect();
        if alive.len() == 1 {
            self.phase = Phase::Finished { winner: alive[0] };
            true
        } else {
            false
        }
    }
    fn end(&mut self) {
        if self.winner() {
            return;
        }
        for _ in 0..self.players.len() {
            self.turn = (self.turn + 1) % self.players.len();
            if self.players[self.turn].alive() > 0 {
                break;
            }
        }
        self.phase = Phase::Turn;
    }
    fn loss(&mut self, player: usize, after: After) {
        if self.players[player].alive() == 0 {
            self.after(after);
        } else {
            self.phase = Phase::Loss { player, after };
        }
    }
    fn after(&mut self, after: After) {
        if self.winner() {
            return;
        }
        match after {
            After::End => self.end(),
            After::Resolve(p) => self.resolve(p),
            After::Blocks(p) => self.blocks(p),
        }
    }
    fn blocks(&mut self, pending: Pending) {
        if pending.action.blocks().is_empty()
            || pending.target.is_some_and(|p| self.players[p].alive() == 0)
        {
            self.resolve(pending);
            return;
        }
        self.phase = Phase::Respond {
            pending,
            stage: Stage::Blocks,
            passed: vec![],
        };
        self.finish_responses();
    }
    fn resolve(&mut self, pending: Pending) {
        if self.players[pending.actor].alive() == 0
            || pending.target.is_some_and(|p| self.players[p].alive() == 0)
        {
            self.end();
            return;
        }
        match pending.action {
            Action::Income => self.players[pending.actor].coins += 1,
            Action::Aid => self.players[pending.actor].coins += 2,
            Action::Tax => self.players[pending.actor].coins += 3,
            Action::Steal => {
                let target = pending.target.unwrap();
                let n = self.players[target].coins.min(2);
                self.players[target].coins -= n;
                self.players[pending.actor].coins += n;
            }
            Action::Assassinate | Action::Coup => {
                self.loss(pending.target.unwrap(), After::End);
                return;
            }
            Action::Exchange => {
                let mut pool: Vec<_> = self.players[pending.actor]
                    .cards
                    .iter()
                    .filter(|c| !c.revealed)
                    .map(|c| c.role)
                    .collect();
                for _ in 0..2 {
                    pool.push(
                        self.deck
                            .pop()
                            .expect("three undealt cards in a six-player game"),
                    );
                }
                self.phase = Phase::Exchange {
                    player: pending.actor,
                    pool,
                };
                return;
            }
        }
        self.end();
    }
    fn responders(&self, pending: &Pending, stage: &Stage) -> Vec<usize> {
        (0..self.players.len())
            .filter(|&p| {
                self.players[p].alive() > 0
                    && match stage {
                        Stage::Claim { claimant, .. } => p != *claimant,
                        Stage::Blocks => {
                            p != pending.actor
                                && (pending.action == Action::Aid || pending.target == Some(p))
                        }
                    }
            })
            .collect()
    }
    fn finish_responses(&mut self) {
        if let Phase::Respond {
            pending,
            stage,
            passed,
        } = self.phase.clone()
            && self
                .responders(&pending, &stage)
                .iter()
                .all(|p| passed.contains(p))
        {
            match stage {
                Stage::Claim { block: true, .. } => self.end(),
                Stage::Claim { block: false, .. } => self.blocks(pending),
                Stage::Blocks => self.resolve(pending),
            }
        }
    }
    pub fn play(&mut self, who: usize, movement: Move) -> Result<(), &'static str> {
        if who >= self.players.len() || self.players[who].alive() == 0 || self.finished() {
            return Err("You cannot act now");
        }
        match (self.phase.clone(), movement) {
            (Phase::Turn, Move::Act { action, target }) => {
                if who != self.turn {
                    return Err("Wait for your turn");
                }
                if self.players[who].coins >= 10 && action != Action::Coup {
                    return Err("At 10 coins you must Coup");
                }
                if self.players[who].coins < action.cost() {
                    return Err("Not enough coins");
                }
                if action.targeted() {
                    if target.is_none_or(|t| {
                        t >= self.players.len() || t == who || self.players[t].alive() == 0
                    }) {
                        return Err("Choose another active player");
                    }
                } else if target.is_some() {
                    return Err("This action has no target");
                }
                self.players[who].coins -= action.cost();
                self.event(format!(
                    "{}: {}{}",
                    self.players[who].name,
                    action.title(),
                    target.map_or(String::new(), |t| format!(" → {}", self.players[t].name))
                ));
                let pending = Pending {
                    actor: who,
                    action,
                    target,
                };
                if let Some(role) = action.role() {
                    self.phase = Phase::Respond {
                        pending,
                        stage: Stage::Claim {
                            role,
                            claimant: who,
                            block: false,
                        },
                        passed: vec![],
                    };
                } else if action == Action::Aid {
                    self.blocks(pending);
                } else {
                    self.resolve(pending);
                }
            }
            (
                Phase::Respond {
                    pending,
                    stage,
                    mut passed,
                },
                Move::Pass,
            ) => {
                if passed.contains(&who) || !self.responders(&pending, &stage).contains(&who) {
                    return Err("You already responded");
                }
                passed.push(who);
                self.phase = Phase::Respond {
                    pending,
                    stage,
                    passed,
                };
                self.finish_responses();
            }
            (
                Phase::Respond {
                    pending,
                    stage:
                        Stage::Claim {
                            role,
                            claimant,
                            block,
                        },
                    passed,
                },
                Move::Challenge,
            ) => {
                if who == claimant || passed.contains(&who) {
                    return Err("You cannot challenge now");
                }
                if let Some(card) = self.players[claimant]
                    .cards
                    .iter()
                    .position(|c| !c.revealed && c.role == role)
                {
                    self.deck.push(role);
                    self.rng.shuffle(&mut self.deck);
                    self.players[claimant].cards[card].role = self.deck.pop().unwrap();
                    self.event(format!(
                        "{} challenged {}. {} was shown.",
                        self.players[who].name,
                        self.players[claimant].name,
                        role.title()
                    ));
                    self.loss(
                        who,
                        if block {
                            After::End
                        } else {
                            After::Blocks(pending)
                        },
                    );
                } else {
                    self.event(format!(
                        "{} caught {} bluffing {}.",
                        self.players[who].name,
                        self.players[claimant].name,
                        role.title()
                    ));
                    self.loss(
                        claimant,
                        if block {
                            After::Resolve(pending)
                        } else {
                            After::End
                        },
                    );
                }
            }
            (
                Phase::Respond {
                    pending,
                    stage: Stage::Blocks,
                    passed,
                },
                Move::Block { role },
            ) => {
                if passed.contains(&who)
                    || !self.responders(&pending, &Stage::Blocks).contains(&who)
                    || !pending.action.blocks().contains(&role)
                {
                    return Err("That role cannot block this action");
                }
                self.event(format!(
                    "{} blocks as {}.",
                    self.players[who].name,
                    role.title()
                ));
                self.phase = Phase::Respond {
                    pending,
                    stage: Stage::Claim {
                        role,
                        claimant: who,
                        block: true,
                    },
                    passed: vec![],
                };
            }
            (Phase::Loss { player, after }, Move::Lose { card }) => {
                if player != who
                    || card >= self.players[who].cards.len()
                    || self.players[who].cards[card].revealed
                {
                    return Err("Choose a remaining influence");
                }
                self.players[who].cards[card].revealed = true;
                self.event(format!(
                    "{} reveals {}.",
                    self.players[who].name,
                    self.players[who].cards[card].role.title()
                ));
                self.after(after);
            }
            (Phase::Exchange { player, pool }, Move::Keep { cards }) => {
                let keep = self.players[who].alive();
                let mut indices = cards.clone();
                indices.sort_unstable();
                indices.dedup();
                if player != who
                    || cards.len() != keep
                    || indices.len() != keep
                    || indices.iter().any(|&i| i >= pool.len())
                {
                    return Err("Choose exactly your remaining influence count");
                }
                let mut selected = cards.iter().map(|&i| pool[i]);
                for card in self.players[who].cards.iter_mut().filter(|c| !c.revealed) {
                    card.role = selected.next().unwrap();
                }
                self.deck.extend(
                    pool.iter()
                        .enumerate()
                        .filter(|(i, _)| !cards.contains(i))
                        .map(|(_, r)| *r),
                );
                self.rng.shuffle(&mut self.deck);
                self.end();
            }
            _ => return Err("That move is no longer available"),
        }
        Ok(())
    }
    pub fn forfeit(&mut self, who: usize) {
        if who >= self.players.len() || self.finished() || self.players[who].alive() == 0 {
            return;
        }
        let before = self.players[who].alive();
        for c in &mut self.players[who].cards {
            c.revealed = true;
        }
        self.event(format!("{} left the table.", self.players[who].name));
        if let Phase::Exchange { player, pool } = &self.phase
            && *player == who
        {
            self.deck.extend(&pool[before..]);
            self.rng.shuffle(&mut self.deck);
        }
        if self.winner() {
            return;
        }
        match self.phase.clone() {
            Phase::Turn if self.turn == who => self.end(),
            Phase::Loss { player, after } if player == who => self.after(after),
            Phase::Exchange { player, .. } if player == who => self.end(),
            Phase::Respond {
                pending,
                stage:
                    Stage::Claim {
                        claimant,
                        block: true,
                        ..
                    },
                ..
            } if claimant == who => self.resolve(pending),
            Phase::Respond { pending, .. } if pending.actor == who => self.end(),
            Phase::Respond { .. } => self.finish_responses(),
            _ => {}
        }
    }
    pub fn view(&self, you: usize) -> View {
        let mut view = View {
            players: self
                .players
                .iter()
                .enumerate()
                .map(|(i, p)| PlayerView {
                    name: p.name.clone(),
                    coins: p.coins,
                    cards: p
                        .cards
                        .iter()
                        .map(|c| CardView {
                            role: if i == you || c.revealed {
                                Some(c.role)
                            } else {
                                None
                            },
                            revealed: c.revealed,
                        })
                        .collect(),
                })
                .collect(),
            turn: self.turn,
            phase: String::new(),
            prompt: String::new(),
            pending: None,
            choices: vec![],
            actions: vec![],
            exchange: vec![],
            keep: 0,
            winner: None,
            log: self.log.clone(),
        };
        match &self.phase {
            Phase::Turn => {
                view.phase = "turn".into();
                view.prompt = if self.turn == you {
                    "Your turn".into()
                } else {
                    format!("{}’s turn", self.players[self.turn].name)
                };
                if self.turn == you {
                    view.actions = Action::ALL
                        .into_iter()
                        .filter(|a| {
                            self.players[you].coins >= a.cost()
                                && (self.players[you].coins < 10 || *a == Action::Coup)
                        })
                        .collect();
                }
            }
            Phase::Respond {
                pending,
                stage,
                passed,
            } => {
                view.phase = "response".into();
                view.pending = Some(pending.clone());
                let eligible =
                    self.responders(pending, stage).contains(&you) && !passed.contains(&you);
                view.prompt = match stage {
                    Stage::Claim {
                        role,
                        claimant,
                        block,
                    } => format!(
                        "{} claims {}{}",
                        self.players[*claimant].name,
                        role.title(),
                        if *block { " to block" } else { "" }
                    ),
                    Stage::Blocks => format!("Block {}?", pending.action.title()),
                };
                if eligible {
                    view.choices.push(Choice {
                        label: "Allow".into(),
                        command: Move::Pass,
                    });
                    match stage {
                        Stage::Claim { .. } => view.choices.push(Choice {
                            label: "Challenge".into(),
                            command: Move::Challenge,
                        }),
                        Stage::Blocks => {
                            for &role in pending.action.blocks() {
                                view.choices.push(Choice {
                                    label: format!("Block · {}", role.title()),
                                    command: Move::Block { role },
                                });
                            }
                        }
                    }
                }
            }
            Phase::Loss { player, .. } => {
                view.phase = "loss".into();
                view.prompt = if *player == you {
                    "Choose an influence to reveal".into()
                } else {
                    format!("{} must reveal an influence", self.players[*player].name)
                };
                if *player == you {
                    for (card, c) in self.players[you]
                        .cards
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| !c.revealed)
                    {
                        view.choices.push(Choice {
                            label: format!("Reveal {}", c.role.title()),
                            command: Move::Lose { card },
                        });
                    }
                }
            }
            Phase::Exchange { player, pool } => {
                view.phase = "exchange".into();
                view.prompt = if *player == you {
                    "Choose the influences to keep".into()
                } else {
                    format!("{} is exchanging", self.players[*player].name)
                };
                if *player == you {
                    view.exchange = pool.clone();
                    view.keep = self.players[you].alive();
                }
            }
            Phase::Finished { winner } => {
                view.phase = "finished".into();
                view.winner = Some(*winner);
                view.prompt = format!("{} wins", self.players[*winner].name);
            }
        }
        view
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(n: usize) -> Game {
        Game::new((0..n).map(|i| format!("P{i}")).collect(), 42).unwrap()
    }
    fn act(g: &mut Game, action: Action, target: Option<usize>) {
        g.play(g.turn, Move::Act { action, target }).unwrap();
    }
    fn allow(g: &mut Game) {
        while let Phase::Respond {
            pending,
            stage,
            passed,
        } = g.phase.clone()
        {
            let p = g
                .responders(&pending, &stage)
                .into_iter()
                .find(|p| !passed.contains(p))
                .unwrap();
            g.play(p, Move::Pass).unwrap();
        }
    }
    #[test]
    fn deal_privacy_and_forced_coup() {
        for n in 2..=6 {
            let mut g = game(n);
            assert_eq!(g.deck.len(), 15 - 2 * n);
            for i in 0..n {
                let v = g.view(i);
                assert!(v.players[i].cards.iter().all(|c| c.role.is_some()));
                assert!(
                    v.players
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .all(|(_, p)| p.cards.iter().all(|c| c.role.is_none()))
                );
            }
            g.players[0].coins = 10;
            let before = serde_json::to_string(&g).unwrap();
            assert!(
                g.play(
                    0,
                    Move::Act {
                        action: Action::Tax,
                        target: None
                    }
                )
                .is_err()
            );
            assert_eq!(before, serde_json::to_string(&g).unwrap());
            assert_eq!(g.view(0).actions, vec![Action::Coup]);
            act(&mut g, Action::Coup, Some(1));
            assert_eq!(g.players[0].coins, 3);
            g.play(1, Move::Lose { card: 0 }).unwrap();
            assert!(g.view(0).players[1].cards[0].role.is_some());
        }
    }
    #[test]
    fn truthful_and_bluff_challenges() {
        let mut g = game(3);
        g.players[0].cards[0].role = Role::Regent;
        act(&mut g, Action::Tax, None);
        g.play(1, Move::Challenge).unwrap();
        assert!(matches!(g.phase, Phase::Loss { player: 1, .. }));
        g.play(1, Move::Lose { card: 0 }).unwrap();
        assert_eq!(g.players[0].coins, 5);
        assert_eq!(g.turn, 1);
        let mut g = game(3);
        for c in &mut g.players[0].cards {
            c.role = Role::Sentinel;
        }
        act(&mut g, Action::Tax, None);
        g.play(1, Move::Challenge).unwrap();
        g.play(0, Move::Lose { card: 1 }).unwrap();
        assert_eq!(g.players[0].coins, 2);
        assert_eq!(g.turn, 1);
    }
    #[test]
    fn assassination_cost_block_and_double_loss() {
        let mut g = game(3);
        g.players[0].coins = 3;
        g.players[0].cards[0].role = Role::Shade;
        act(&mut g, Action::Assassinate, Some(1));
        g.play(1, Move::Challenge).unwrap();
        g.play(1, Move::Lose { card: 0 }).unwrap();
        allow(&mut g);
        g.play(1, Move::Lose { card: 1 }).unwrap();
        assert_eq!(g.players[0].coins, 0);
        assert_eq!(g.players[1].alive(), 0);
        let mut g = game(3);
        g.players[0].coins = 3;
        act(&mut g, Action::Assassinate, Some(1));
        allow_claim_only(&mut g);
        assert!(
            g.play(
                2,
                Move::Block {
                    role: Role::Sentinel
                }
            )
            .is_err()
        );
        g.play(
            1,
            Move::Block {
                role: Role::Sentinel,
            },
        )
        .unwrap();
        allow(&mut g);
        assert_eq!(g.players[1].alive(), 2);
        assert_eq!(g.players[0].coins, 0);
    }
    fn allow_claim_only(g: &mut Game) {
        while let Phase::Respond {
            pending,
            stage: stage @ Stage::Claim { .. },
            passed,
        } = g.phase.clone()
        {
            let p = g
                .responders(&pending, &stage)
                .into_iter()
                .find(|p| !passed.contains(p))
                .unwrap();
            g.play(p, Move::Pass).unwrap();
        }
    }
    #[test]
    fn false_block_continues_and_foreign_aid_anyone_can_block() {
        let mut g = game(3);
        g.players[1]
            .cards
            .iter_mut()
            .for_each(|c| c.role = Role::Shade);
        act(&mut g, Action::Aid, None);
        g.play(1, Move::Block { role: Role::Regent }).unwrap();
        g.play(2, Move::Challenge).unwrap();
        g.play(1, Move::Lose { card: 0 }).unwrap();
        assert_eq!(g.players[0].coins, 4);
        assert_eq!(g.turn, 1);
    }
    #[test]
    fn exchange_conserves_cards_rejects_duplicates_and_survives_serialization() {
        let mut g = game(6);
        act(&mut g, Action::Exchange, None);
        allow(&mut g);
        let before = serde_json::to_string(&g).unwrap();
        assert!(g.play(0, Move::Keep { cards: vec![0, 0] }).is_err());
        assert_eq!(before, serde_json::to_string(&g).unwrap());
        let mut restored: Game = serde_json::from_str(&before).unwrap();
        g.play(0, Move::Keep { cards: vec![2, 3] }).unwrap();
        restored.play(0, Move::Keep { cards: vec![2, 3] }).unwrap();
        assert_eq!(
            serde_json::to_string(&g).unwrap(),
            serde_json::to_string(&restored).unwrap()
        );
        assert_eq!(
            g.deck.len() + g.players.iter().map(|p| p.cards.len()).sum::<usize>(),
            15
        );
        let mut roles = g.deck.clone();
        roles.extend(
            g.players
                .iter()
                .flat_map(|p| p.cards.iter().map(|c| c.role)),
        );
        for role in Role::ALL {
            assert_eq!(roles.iter().filter(|r| **r == role).count(), 3);
        }
    }
    #[test]
    fn complete_matches_never_leak_or_stall() {
        for n in 2..=6 {
            for seed in 0..12 {
                let mut g = Game::new((0..n).map(|i| format!("P{i}")).collect(), seed).unwrap();
                for _ in 0..500 {
                    if g.finished() {
                        break;
                    }
                    let mut moved = false;
                    for who in 0..n {
                        let v = g.view(who);
                        let movement = if !v.actions.is_empty() {
                            Some(Move::Act {
                                action: if g.players[who].coins >= 7 {
                                    Action::Coup
                                } else {
                                    Action::Income
                                },
                                target: (g.players[who].coins >= 7).then(|| {
                                    (0..n)
                                        .find(|&i| i != who && g.players[i].alive() > 0)
                                        .unwrap()
                                }),
                            })
                        } else {
                            v.choices.first().map(|c| c.command.clone())
                        };
                        if let Some(m) = movement {
                            g.play(who, m).unwrap();
                            moved = true;
                            break;
                        }
                    }
                    assert!(moved);
                    assert_eq!(
                        g.deck.len() + g.players.iter().map(|p| p.cards.len()).sum::<usize>(),
                        15
                    );
                }
                assert!(g.finished());
            }
        }
    }
}
