//! Offline Ito-style cooperation. Hidden numbers are projected only to the current hand.
use crate::multiplayer::{Rng, clean_text};
use serde::{Deserialize, Serialize};

pub const MAX_CLUE: usize = 80;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    English,
    Portuguese,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub players: u8,
    pub cards_each: u8,
    pub language: Language,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            players: 2,
            cards_each: 2,
            language: Language::English,
        }
    }
}
impl Config {
    pub fn valid(self) -> bool {
        (2..=10).contains(&self.players) && (1..=3).contains(&self.cards_each)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Phase {
    Setup,
    Handoff { player: usize, review: bool },
    Hand { player: usize, review: bool },
    Arrange,
    Reveal,
    Result { won: bool },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Card {
    number: u8,
    clue: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    version: u8,
    config: Config,
    phase: Phase,
    category: usize,
    cards: Vec<Card>,
    order: Vec<usize>,
    revealed: usize,
    round: u32,
    rng: Rng,
}
pub struct CardView<'a> {
    pub id: usize,
    pub owner: usize,
    pub letter: char,
    pub clue: &'a str,
    pub number: Option<u8>,
}
impl Game {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let category = (rng.next() % CATEGORIES.len() as u64) as usize;
        Self {
            version: 1,
            config: Config::default(),
            phase: Phase::Setup,
            category,
            cards: vec![],
            order: vec![],
            revealed: 0,
            round: 0,
            rng,
        }
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn config(&self) -> Config {
        self.config
    }
    pub fn order(&self) -> &[usize] {
        &self.order
    }
    pub fn round(&self) -> u32 {
        self.round
    }
    pub fn category(&self) -> (&'static str, &'static str, &'static str) {
        let c = CATEGORIES[self.category];
        if self.config.language == Language::English {
            c.0
        } else {
            c.1
        }
    }
    pub fn configure(&mut self, config: Config) -> bool {
        if self.phase != Phase::Setup || !config.valid() || config == self.config {
            return false;
        }
        self.config = config;
        true
    }
    pub fn shuffle_category(&mut self) -> bool {
        if self.phase != Phase::Setup {
            return false;
        }
        self.category =
            (self.category + 1 + (self.rng.next() % (CATEGORIES.len() - 1) as u64) as usize)
                % CATEGORIES.len();
        true
    }
    pub fn start(&mut self) -> bool {
        if self.phase != Phase::Setup || !self.config.valid() {
            return false;
        }
        let count = usize::from(self.config.players) * usize::from(self.config.cards_each);
        let mut numbers: Vec<u8> = (1..=100).collect();
        self.rng.shuffle(&mut numbers);
        self.cards = numbers[..count]
            .iter()
            .map(|&number| Card {
                number,
                clue: String::new(),
            })
            .collect();
        self.order = (0..count).collect();
        self.rng.shuffle(&mut self.order);
        self.phase = Phase::Handoff {
            player: 0,
            review: false,
        };
        self.revealed = 0;
        self.round = self.round.saturating_add(1);
        true
    }
    pub fn owner(&self, id: usize) -> Option<usize> {
        (id < self.cards.len()).then(|| id / usize::from(self.config.cards_each))
    }
    pub fn card(&self, id: usize) -> Option<CardView<'_>> {
        let c = self.cards.get(id)?;
        let owner = self.owner(id)?;
        let visible = match self.phase {
            Phase::Hand { player, .. } => player == owner,
            Phase::Reveal => self.order[..self.revealed].contains(&id),
            Phase::Result { .. } => true,
            _ => false,
        };
        Some(CardView {
            id,
            owner,
            letter: (b'A' + (id % usize::from(self.config.cards_each)) as u8) as char,
            clue: &c.clue,
            number: visible.then_some(c.number),
        })
    }
    pub fn show_hand(&mut self) -> bool {
        if let Phase::Handoff { player, review } = self.phase {
            self.phase = Phase::Hand { player, review };
            true
        } else {
            false
        }
    }
    pub fn clue(&mut self, id: usize, text: &str) -> Result<bool, &'static str> {
        let Phase::Hand { player, .. } = self.phase else {
            return Err("Reveal your own hand first");
        };
        if self.owner(id) != Some(player) {
            return Err("That is another player's card");
        }
        let text = clean_text(text, MAX_CLUE);
        if text.chars().any(char::is_numeric) {
            return Err("Use an example, without numbers");
        }
        if self.cards[id].clue == text {
            return Ok(false);
        }
        self.cards[id].clue = text;
        Ok(true)
    }
    pub fn hand_ready(&self) -> bool {
        if let Phase::Hand { player, .. } = self.phase {
            self.cards
                .iter()
                .enumerate()
                .filter(|(i, _)| self.owner(*i) == Some(player))
                .all(|(_, c)| !c.clue.is_empty())
        } else {
            false
        }
    }
    pub fn pass(&mut self) -> bool {
        if !self.hand_ready() {
            return false;
        }
        let Phase::Hand { player, review } = self.phase else {
            return false;
        };
        self.phase = if review || player + 1 == usize::from(self.config.players) {
            Phase::Arrange
        } else {
            Phase::Handoff {
                player: player + 1,
                review: false,
            }
        };
        true
    }
    pub fn review_hand(&mut self, player: usize) -> bool {
        if self.phase != Phase::Arrange || player >= usize::from(self.config.players) {
            return false;
        }
        self.phase = Phase::Handoff {
            player,
            review: true,
        };
        true
    }
    pub fn conceal(&mut self) -> bool {
        if let Phase::Hand { player, review } = self.phase {
            self.phase = Phase::Handoff { player, review };
            true
        } else {
            false
        }
    }
    pub fn move_card(&mut self, id: usize, to: usize) -> bool {
        if self.phase != Phase::Arrange || to >= self.order.len() {
            return false;
        }
        let Some(from) = self.order.iter().position(|&i| i == id) else {
            return false;
        };
        if from == to {
            return false;
        }
        self.order.remove(from);
        self.order.insert(to, id);
        true
    }
    pub fn begin_reveal(&mut self) -> bool {
        if self.phase != Phase::Arrange || self.cards.iter().any(|c| c.clue.is_empty()) {
            return false;
        }
        self.phase = Phase::Reveal;
        self.revealed = 0;
        true
    }
    pub fn reveal_next(&mut self) -> bool {
        if self.phase != Phase::Reveal {
            return false;
        }
        self.revealed += 1;
        if self.revealed > 1
            && self.cards[self.order[self.revealed - 2]].number
                > self.cards[self.order[self.revealed - 1]].number
        {
            self.phase = Phase::Result { won: false };
        } else if self.revealed == self.order.len() {
            self.phase = Phase::Result { won: true };
        }
        true
    }
    pub fn revealed(&self) -> usize {
        self.revealed
    }
    pub fn new_round(&mut self) -> bool {
        if !matches!(self.phase, Phase::Result { .. }) {
            return false;
        }
        self.phase = Phase::Setup;
        self.cards.clear();
        self.order.clear();
        self.revealed = 0;
        self.shuffle_category();
        true
    }
    pub fn abandon(&mut self) -> bool {
        if self.phase == Phase::Setup {
            return false;
        }
        self.phase = Phase::Setup;
        self.cards.clear();
        self.order.clear();
        self.revealed = 0;
        true
    }
    pub fn encode(&self) -> String {
        let mut saved = self.clone();
        saved.conceal();
        serde_json::to_string(&saved).unwrap()
    }
    pub fn decode(data: &str, seed: u64) -> Self {
        let Some(mut game) = serde_json::from_str::<Self>(data).ok().filter(Self::valid) else {
            return Self::new(seed);
        };
        game.conceal();
        game
    }
    fn valid(&self) -> bool {
        if self.version != 1 || !self.config.valid() || self.category >= CATEGORIES.len() {
            return false;
        }
        let n = usize::from(self.config.players) * usize::from(self.config.cards_each);
        if self.phase == Phase::Setup {
            return self.cards.is_empty() && self.order.is_empty() && self.revealed == 0;
        }
        if self.cards.len() != n || self.order.len() != n || self.revealed > n {
            return false;
        }
        let mut order = self.order.clone();
        order.sort_unstable();
        if order != (0..n).collect::<Vec<_>>() {
            return false;
        }
        let mut nums: Vec<_> = self.cards.iter().map(|c| c.number).collect();
        nums.sort_unstable();
        nums.dedup();
        if nums.len() != n || nums.iter().any(|&v| !(1..=100).contains(&v)) {
            return false;
        }
        if self.cards.iter().any(|c| {
            c.clue.chars().count() > MAX_CLUE
                || clean_text(&c.clue, MAX_CLUE) != c.clue
                || c.clue.chars().any(char::is_numeric)
        }) {
            return false;
        }
        match self.phase {
            Phase::Handoff { player, review } | Phase::Hand { player, review } => {
                player < usize::from(self.config.players)
                    && self.revealed == 0
                    && self
                        .cards
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| {
                            if review {
                                self.owner(*i) != Some(player)
                            } else {
                                self.owner(*i).unwrap() < player
                            }
                        })
                        .all(|(_, c)| !c.clue.is_empty())
            }
            Phase::Arrange => self.revealed == 0 && self.cards.iter().all(|c| !c.clue.is_empty()),
            Phase::Reveal => {
                self.revealed < n
                    && self.cards.iter().all(|c| !c.clue.is_empty())
                    && self.order[..self.revealed]
                        .windows(2)
                        .all(|p| self.cards[p[0]].number < self.cards[p[1]].number)
            }
            Phase::Result { won } => {
                self.cards.iter().all(|c| !c.clue.is_empty())
                    && if won {
                        self.revealed == n
                            && self
                                .order
                                .windows(2)
                                .all(|p| self.cards[p[0]].number < self.cards[p[1]].number)
                    } else {
                        self.revealed >= 2
                            && self.order[..self.revealed - 1]
                                .windows(2)
                                .all(|p| self.cards[p[0]].number < self.cards[p[1]].number)
                            && self.cards[self.order[self.revealed - 2]].number
                                > self.cards[self.order[self.revealed - 1]].number
                    }
            }
            Phase::Setup => false,
        }
    }
}

// Original prompts; each spectrum explicitly defines what a high number means.
type Category = (
    (&'static str, &'static str, &'static str),
    (&'static str, &'static str, &'static str),
);
const CATEGORIES: [Category; 32] = [
    (
        ("Animals you would cuddle", "Keep away", "Best cuddle"),
        ("Animais que você abraçaria", "Bem longe", "Melhor abraço"),
    ),
    (
        ("Things to find in your pocket", "Disappointing", "Amazing"),
        ("Coisas para achar no bolso", "Decepcionante", "Incrível"),
    ),
    (
        (
            "Food for a midnight snack",
            "Terrible choice",
            "Perfect snack",
        ),
        (
            "Comida para a madrugada",
            "Péssima escolha",
            "Lanche perfeito",
        ),
    ),
    (
        ("Superpowers you would want", "No thanks", "Dream power"),
        (
            "Superpoderes que você quer",
            "Não, obrigado",
            "Poder dos sonhos",
        ),
    ),
    (
        ("Sounds to wake up to", "Unbearable", "Wonderful"),
        ("Sons para acordar", "Insuportável", "Maravilhoso"),
    ),
    (
        ("Places for a first date", "Awkward", "Romantic"),
        ("Lugares para um encontro", "Constrangedor", "Romântico"),
    ),
    (
        ("Things that belong in a museum", "Ordinary", "Priceless"),
        ("Coisas para um museu", "Comum", "Inestimável"),
    ),
    (
        (
            "Animals in a tiny apartment",
            "Bad roommate",
            "Ideal roommate",
        ),
        ("Animais num apartamento", "Péssimo colega", "Colega ideal"),
    ),
    (
        ("Ways to spend a rainy day", "Boring", "Brilliant"),
        ("Jeitos de passar um dia chuvoso", "Chato", "Excelente"),
    ),
    (
        ("Things to bring to a desert island", "Useless", "Essential"),
        ("Coisas para uma ilha deserta", "Inútil", "Essencial"),
    ),
    (
        ("Jobs you would try for a day", "Dreadful", "Exciting"),
        ("Trabalhos por um dia", "Terrível", "Empolgante"),
    ),
    (
        ("Things that make you nervous", "Relaxing", "Terrifying"),
        ("Coisas que dão nervosismo", "Tranquilo", "Aterrorizante"),
    ),
    (
        ("Gifts for a close friend", "Bad gift", "Thoughtful gift"),
        ("Presentes para um amigo", "Presente ruim", "Muito especial"),
    ),
    (
        ("Foods to eat with your hands", "Messy disaster", "Easy"),
        ("Comidas para comer com a mão", "Desastre", "Fácil"),
    ),
    (
        ("Places to hide a secret", "Obvious", "Impossible to find"),
        (
            "Lugares para esconder um segredo",
            "Óbvio",
            "Impossível achar",
        ),
    ),
    (
        ("Things you could build a house from", "Flimsy", "Sturdy"),
        ("Materiais para construir uma casa", "Frágil", "Resistente"),
    ),
    (
        (
            "Fictional characters as babysitters",
            "Disaster",
            "Perfect sitter",
        ),
        ("Personagens como babás", "Desastre", "Babá perfeita"),
    ),
    (
        ("Ways to travel to work", "Uncomfortable", "Comfortable"),
        ("Jeitos de ir ao trabalho", "Desconfortável", "Confortável"),
    ),
    (
        ("Things to see through a window", "Unpleasant", "Beautiful"),
        ("Coisas para ver pela janela", "Desagradável", "Lindo"),
    ),
    (
        (
            "Things that would surprise an alien",
            "Unremarkable",
            "Mind-blowing",
        ),
        ("Coisas que surpreendem um alien", "Normal", "Surpreendente"),
    ),
    (
        ("Activities before bedtime", "Bad idea", "Peaceful"),
        ("Atividades antes de dormir", "Má ideia", "Relaxante"),
    ),
    (
        (
            "Things you would save from a flood",
            "Replaceable",
            "Irreplaceable",
        ),
        (
            "Coisas para salvar de uma enchente",
            "Substituível",
            "Insubstituível",
        ),
    ),
    (
        (
            "Things that make a party better",
            "Party killer",
            "Party maker",
        ),
        (
            "Coisas que melhoram uma festa",
            "Acaba com a festa",
            "Anima a festa",
        ),
    ),
    (
        ("Things to have as a pet", "Impossible", "Wonderful pet"),
        (
            "Coisas para ter como bichinho",
            "Impossível",
            "Ótimo bichinho",
        ),
    ),
    (
        ("Foods you would eat every day", "Never", "Gladly"),
        ("Comidas para comer todo dia", "Nunca", "Com prazer"),
    ),
    (
        ("Things you would carry uphill", "Light", "Heavy"),
        ("Coisas para levar ladeira acima", "Leve", "Pesado"),
    ),
    (
        ("Places to take a nap", "Impossible", "Perfect"),
        ("Lugares para tirar uma soneca", "Impossível", "Perfeito"),
    ),
    (
        ("Things to wear to a wedding", "Inappropriate", "Elegant"),
        ("Coisas para vestir num casamento", "Inadequado", "Elegante"),
    ),
    (
        (
            "Things that smell strong",
            "Barely noticeable",
            "Overwhelming",
        ),
        (
            "Coisas com cheiro forte",
            "Quase sem cheiro",
            "Muito intenso",
        ),
    ),
    (
        (
            "Things that are difficult to learn",
            "Easy",
            "Very difficult",
        ),
        ("Coisas difíceis de aprender", "Fácil", "Muito difícil"),
    ),
    (
        ("Things to put on a pizza", "Please don't", "Delicious"),
        (
            "Coisas para colocar na pizza",
            "Por favor, não",
            "Delicioso",
        ),
    ),
    (
        (
            "Things that could be a band name",
            "Forgettable",
            "Memorable",
        ),
        ("Coisas que seriam nomes de banda", "Esquecível", "Marcante"),
    ),
];
#[cfg(test)]
mod tests {
    use super::*;
    fn arranged(players: u8, cards: u8) -> Game {
        let mut g = Game::new(9);
        g.configure(Config {
            players,
            cards_each: cards,
            ..Config::default()
        });
        assert!(g.start());
        for player in 0..usize::from(players) {
            assert_eq!(
                g.phase(),
                Phase::Handoff {
                    player,
                    review: false
                }
            );
            assert!(g.show_hand());
            for id in player * usize::from(cards)..(player + 1) * usize::from(cards) {
                g.clue(id, &format!("Clue {}", (b'A' + id as u8) as char))
                    .unwrap();
            }
            assert!(g.pass());
        }
        assert_eq!(g.phase(), Phase::Arrange);
        g
    }
    #[test]
    fn all_sizes_deal_unique_numbers_and_hide_every_other_hand() {
        for players in 2..=10 {
            for each in 1..=3 {
                let mut g = Game::new(players as u64);
                g.configure(Config {
                    players,
                    cards_each: each,
                    ..Config::default()
                });
                g.start();
                assert!((0..g.cards.len()).all(|i| g.card(i).unwrap().number.is_none()));
                g.show_hand();
                assert_eq!(
                    (0..g.cards.len())
                        .filter(|&i| g.card(i).unwrap().number.is_some())
                        .count(),
                    each as usize
                );
                assert!(g.clue(each as usize, "cheat").is_err());
                assert!(!g.pass());
                assert!(!g.begin_reveal());
                assert!(g.valid());
            }
        }
    }
    #[test]
    fn correct_order_wins_and_inversion_loses_without_reordering_during_reveal() {
        let mut g = arranged(2, 3);
        let mut ids = g.order.clone();
        ids.sort_by_key(|&i| g.cards[i].number);
        for (to, id) in ids.into_iter().enumerate() {
            g.move_card(id, to);
        }
        assert!((0..6).all(|id| g.card(id).unwrap().number.is_none()));
        assert!(g.begin_reveal());
        for i in 0..6 {
            assert!(g.reveal_next());
            assert_eq!(g.revealed(), i + 1);
            assert!(g.card(g.order[i]).unwrap().number.is_some());
            assert!(!g.move_card(g.order[0], 5));
        }
        assert_eq!(g.phase(), Phase::Result { won: true });
        assert!(!g.reveal_next());
        assert!(g.valid());
        let mut g = arranged(2, 2);
        g.order
            .sort_by_key(|&i| std::cmp::Reverse(g.cards[i].number));
        g.begin_reveal();
        g.reveal_next();
        g.reveal_next();
        assert_eq!(g.phase(), Phase::Result { won: false });
        assert!(g.valid());
    }
    #[test]
    fn clues_can_change_but_numbers_are_never_clues_or_leaked_on_review() {
        let mut g = arranged(2, 2);
        let before = g.order.clone();
        assert!(g.review_hand(1));
        assert!((0..4).all(|i| g.card(i).unwrap().number.is_none()));
        g.show_hand();
        assert!(g.clue(2, "  A gentle giant  ").unwrap());
        assert_eq!(g.card(2).unwrap().clue, "A gentle giant");
        for bad in ["size 90", "ninety ９０", "٥"] {
            assert!(g.clue(2, bad).is_err());
        }
        assert!(!g.move_card(0, 3));
        assert!(g.pass());
        assert_eq!(g.order, before);
        assert!((0..4).all(|i| g.card(i).unwrap().number.is_none()));
    }
    #[test]
    fn saves_conceal_hands_reject_corruption_and_keep_order_progress() {
        let mut g = arranged(10, 3);
        g.move_card(0, 29);
        assert!(g.review_hand(3));
        g.show_hand();
        let saved = g.encode();
        let restored = Game::decode(&saved, 3);
        assert_eq!(
            restored.phase(),
            Phase::Handoff {
                player: 3,
                review: true
            }
        );
        assert_eq!(restored.order, g.order);
        assert!(restored.valid());
        assert!(saved.len() < 32768);
        let mut bad = serde_json::to_value(&g).unwrap();
        bad["order"][0] = serde_json::json!(999);
        assert_eq!(Game::decode(&bad.to_string(), 0).phase(), Phase::Setup);
        let mut bad = serde_json::to_value(&g).unwrap();
        bad["cards"][0]["number"] = serde_json::json!(0);
        assert_eq!(Game::decode(&bad.to_string(), 0).phase(), Phase::Setup);
        let mut bad = serde_json::to_value(&g).unwrap();
        bad["phase"] = serde_json::json!({"kind":"handoff","player":999,"review":false});
        assert_eq!(Game::decode(&bad.to_string(), 0).phase(), Phase::Setup);
        assert_eq!(Game::decode("bad data", 0).phase(), Phase::Setup);
    }
    #[test]
    fn review_with_cleared_clue_survives_reload_but_cannot_finish() {
        let mut g = arranged(2, 2);
        g.review_hand(0);
        g.show_hand();
        g.clue(0, "").unwrap();
        assert!(!g.hand_ready());
        assert!(!g.pass());
        let mut restored = Game::decode(&g.encode(), 0);
        assert_eq!(
            restored.phase(),
            Phase::Handoff {
                player: 0,
                review: true
            }
        );
        restored.show_hand();
        assert_eq!(restored.card(0).unwrap().clue, "");
        restored.clue(0, "A new idea").unwrap();
        assert!(restored.pass());
        assert_eq!(restored.phase(), Phase::Arrange);
    }
    #[test]
    fn moves_are_atomic_categories_never_repeat_and_languages_do_not_change_numbers() {
        let mut g = Game::new(1);
        let old = g.category();
        assert!(g.shuffle_category());
        assert_ne!(g.category(), old);
        assert!(!g.configure(Config {
            players: 1,
            ..Config::default()
        }));
        g.configure(Config {
            language: Language::Portuguese,
            ..Config::default()
        });
        g.start();
        let nums: Vec<_> = g.cards.iter().map(|c| c.number).collect();
        assert!(!g.configure(Config::default()));
        assert!(!g.shuffle_category());
        assert_eq!(nums, g.cards.iter().map(|c| c.number).collect::<Vec<_>>());
        let mut g = arranged(2, 2);
        let old = g.order.clone();
        assert!(!g.move_card(999, 0));
        assert!(!g.move_card(0, 999));
        assert_eq!(g.order, old);
        g.conceal();
        assert_eq!(g.phase(), Phase::Arrange);
    }
}
