//! Authoritative word-association rules; the unrevealed key never enters an operative view.
//! Reference: https://czechgames.com/files/rules/codenames-rules-en.pdf
use super::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    Red,
    Blue,
}
impl Team {
    pub fn other(self) -> Self {
        match self {
            Self::Red => Self::Blue,
            Self::Blue => Self::Red,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Blue => "Blue",
        }
    }
    fn index(self) -> usize {
        if self == Self::Red { 0 } else { 1 }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Spymaster,
    Operative,
}
impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Self::Spymaster => "Spymaster",
            Self::Operative => "Operative",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    English,
    Portuguese,
}
impl Language {
    pub fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Portuguese => "Português",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seat {
    pub team: Team,
    pub role: Role,
}
impl Seat {
    pub fn default_for(i: usize) -> Self {
        Self {
            team: if i.is_multiple_of(2) {
                Team::Red
            } else {
                Team::Blue
            },
            role: if i < 2 {
                Role::Spymaster
            } else {
                Role::Operative
            },
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setup {
    pub language: Language,
    pub seats: Vec<Seat>,
}
impl Setup {
    pub fn seat(&self, i: usize) -> Seat {
        self.seats
            .get(i)
            .copied()
            .unwrap_or_else(|| Seat::default_for(i))
    }
    pub fn resize(&mut self, n: usize) {
        while self.seats.len() < n {
            self.seats.push(Seat::default_for(self.seats.len()));
        }
        self.seats.truncate(n);
    }
    pub fn validate(&self, n: usize) -> Result<(), &'static str> {
        if !(4..=16).contains(&n) {
            return Err("Invite 4–16 players");
        }
        for team in [Team::Red, Team::Blue] {
            let spies = (0..n)
                .filter(|&i| {
                    self.seat(i)
                        == Seat {
                            team,
                            role: Role::Spymaster,
                        }
                })
                .count();
            if spies != 1 {
                return Err("Each team needs exactly one spymaster");
            }
            if !(0..n).any(|i| {
                self.seat(i)
                    == Seat {
                        team,
                        role: Role::Operative,
                    }
            }) {
                return Err("Each team needs at least one operative");
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Identity {
    Red,
    Blue,
    Neutral,
    Assassin,
}
impl Identity {
    pub fn team(self) -> Option<Team> {
        match self {
            Self::Red => Some(Team::Red),
            Self::Blue => Some(Team::Blue),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Clue,
    Guess,
    Finished,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clue {
    pub word: String,
    /// None = unlimited; zero also allows unlimited guesses.
    pub number: Option<u8>,
    pub team: Team,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Move {
    Clue { word: String, number: Option<u8> },
    Guess { card: usize },
    Pass,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardView {
    pub word: String,
    pub identity: Option<Identity>,
    pub revealed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct View {
    pub cards: Vec<CardView>,
    pub seats: Vec<Seat>,
    pub team: Team,
    pub phase: Phase,
    pub clue: Option<Clue>,
    pub history: Vec<Clue>,
    pub remaining: [usize; 2],
    pub guesses_left: Option<u8>,
    pub guesses: u8,
    pub turn: u32,
    pub winner: Option<Team>,
    pub can_clue: bool,
    pub can_guess: bool,
    pub can_pass: bool,
    pub reason: String,
    pub language: Language,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Card {
    word: String,
    identity: Identity,
    revealed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    cards: Vec<Card>,
    seats: Vec<Seat>,
    team: Team,
    phase: Phase,
    clue: Option<Clue>,
    history: Vec<Clue>,
    guesses_left: Option<u8>,
    guesses: u8,
    turn: u32,
    winner: Option<Team>,
    reason: String,
    language: Language,
}
impl Game {
    pub fn new(n: usize, seed: u64, setup: &Setup) -> Result<Self, &'static str> {
        setup.validate(n)?;
        let mut rng = Rng::new(seed);
        let team = if rng.next().is_multiple_of(2) {
            Team::Red
        } else {
            Team::Blue
        };
        let mut identities = vec![Identity::Neutral; 7];
        identities.push(Identity::Assassin);
        for t in [Team::Red, Team::Blue] {
            identities.extend(std::iter::repeat_n(
                if t == Team::Red {
                    Identity::Red
                } else {
                    Identity::Blue
                },
                if t == team { 9 } else { 8 },
            ));
        }
        rng.shuffle(&mut identities);
        let mut words: Vec<_> = deck(setup.language).split_whitespace().collect();
        rng.shuffle(&mut words);
        Ok(Self {
            cards: words
                .into_iter()
                .take(25)
                .zip(identities)
                .map(|(word, identity)| Card {
                    word: word.into(),
                    identity,
                    revealed: false,
                })
                .collect(),
            seats: (0..n).map(|i| setup.seat(i)).collect(),
            team,
            phase: Phase::Clue,
            clue: None,
            history: vec![],
            guesses_left: None,
            guesses: 0,
            turn: 1,
            winner: None,
            reason: String::new(),
            language: setup.language,
        })
    }
    pub fn finished(&self) -> bool {
        self.phase == Phase::Finished
    }
    pub fn phase_key(&self) -> String {
        format!("{:?}:{}:{}", self.phase, self.turn, self.guesses)
    }
    fn remaining(&self) -> [usize; 2] {
        [Team::Red, Team::Blue].map(|t| {
            self.cards
                .iter()
                .filter(|c| !c.revealed && c.identity.team() == Some(t))
                .count()
        })
    }
    pub fn view(&self, you: usize) -> View {
        let seat = self.seats.get(you).copied();
        let authorized = seat.is_some_and(|s| s.team == self.team);
        let can_clue = authorized
            && seat.is_some_and(|s| s.role == Role::Spymaster)
            && self.phase == Phase::Clue;
        let can_guess = authorized
            && seat.is_some_and(|s| s.role == Role::Operative)
            && self.phase == Phase::Guess;
        let key = self.finished() || seat.is_some_and(|s| s.role == Role::Spymaster);
        View {
            cards: self
                .cards
                .iter()
                .map(|c| CardView {
                    word: c.word.clone(),
                    identity: (c.revealed || key).then_some(c.identity),
                    revealed: c.revealed,
                })
                .collect(),
            seats: self.seats.clone(),
            team: self.team,
            phase: self.phase,
            clue: self.clue.clone(),
            history: self.history.clone(),
            remaining: self.remaining(),
            guesses_left: self.guesses_left,
            guesses: self.guesses,
            turn: self.turn,
            winner: self.winner,
            can_clue,
            can_guess,
            can_pass: can_guess && self.guesses > 0,
            reason: self.reason.clone(),
            language: self.language,
        }
    }
    pub fn play(&mut self, you: usize, movement: Move) -> Result<(), &'static str> {
        let seat = self.seats.get(you).ok_or("Unknown player")?;
        if self.finished() {
            return Err("This match has ended");
        }
        if seat.team != self.team {
            return Err("Wait for your team's turn");
        }
        match movement {
            Move::Clue { word, number } => {
                if self.phase != Phase::Clue || seat.role != Role::Spymaster {
                    return Err("Only the active spymaster can give a clue");
                }
                let word = word.trim();
                if word.is_empty()
                    || word.chars().count() > 32
                    || !word.chars().all(char::is_alphabetic)
                {
                    return Err("Use one word, letters only (up to 32)");
                }
                if number.is_some_and(|n| n > 9) {
                    return Err("Choose a number from 0 to 9, or unlimited");
                }
                let normalized = normalize(word);
                if self.cards.iter().any(|c| {
                    !c.revealed && {
                        let w = normalize(&c.word);
                        w.contains(&normalized) || normalized.contains(&w)
                    }
                }) {
                    return Err("The clue cannot contain an unrevealed board word or part of one");
                }
                let clue = Clue {
                    word: word.to_owned(),
                    number,
                    team: self.team,
                };
                self.clue = Some(clue.clone());
                self.history.push(clue);
                self.guesses_left = match number {
                    Some(1..=9) => number.map(|n| n + 1),
                    _ => None,
                };
                self.guesses = 0;
                self.phase = Phase::Guess;
            }
            Move::Guess { card } => {
                if self.phase != Phase::Guess || seat.role != Role::Operative {
                    return Err("Only active operatives can reveal cards");
                }
                let card = self
                    .cards
                    .get_mut(card)
                    .ok_or("Choose a card on the board")?;
                if card.revealed {
                    return Err("That card is already revealed");
                }
                card.revealed = true;
                let identity = card.identity;
                self.guesses += 1;
                if let Some(n) = &mut self.guesses_left {
                    *n = n.saturating_sub(1);
                }
                if identity == Identity::Assassin {
                    self.end(self.team.other(), "The assassin was uncovered.");
                } else if let Some(t) = identity.team()
                    && self.remaining()[t.index()] == 0
                {
                    self.end(t, "All agents found.");
                } else if identity.team() != Some(self.team) || self.guesses_left == Some(0) {
                    self.next_turn();
                }
            }
            Move::Pass => {
                if self.phase != Phase::Guess || seat.role != Role::Operative || self.guesses == 0 {
                    return Err("Make at least one guess before ending the turn");
                }
                self.next_turn();
            }
        }
        Ok(())
    }
    fn next_turn(&mut self) {
        self.team = self.team.other();
        self.phase = Phase::Clue;
        self.guesses = 0;
        self.guesses_left = None;
        self.turn += 1;
    }
    fn end(&mut self, winner: Team, reason: &str) {
        self.winner = Some(winner);
        self.phase = Phase::Finished;
        self.reason = reason.into();
    }
    pub fn forfeit(&mut self, you: usize) {
        if !self.finished()
            && let Some(s) = self.seats.get(you)
        {
            self.end(s.team.other(), "A player left the mission.");
        }
    }
}
fn normalize(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'ã' | 'â' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' => 'u',
            'ç' => 'c',
            _ => c,
        })
        .collect()
}
// Original common-word decks; no publisher card artwork or proprietary word list.
fn deck(language: Language) -> &'static str {
    match language {
        Language::English => {
            "APPLE BEACH BRIDGE CLOCK CLOUD CROWN DANCE DESERT DRAGON DREAM EAGLE ENGINE FEATHER FIELD FIRE FLOWER FOREST FROG GARDEN GHOST GIANT GLASS GLOVE GOLD GRAPE GUITAR HARBOR HEART HONEY HORSE HOTEL ICE ISLAND JACKET JELLY JUDGE KITE KNIGHT LADDER LASER LEMON LIGHT LION LOCK MAGIC MAP MARBLE MASK MOON MOUNTAIN MOUSE MUSIC NEEDLE NIGHT OCEAN OLIVE ORANGE OWL PALACE PAPER PEARL PIANO PILOT PLANET POCKET POLICE RABBIT RAIN RAINBOW RIVER ROBOT ROCK ROCKET ROSE SAIL SATELLITE SCHOOL SHADOW SHARK SHIP SHOE SILVER SNAKE SNOW SPACE SPIDER SPRING STAR STONE STORM SUGAR SUN TABLE TEMPLE THUNDER TIGER TOAST TOWER TRAIN TREE TURTLE UMBRELLA VALLEY VELVET VIOLIN VOLCANO WATER WAVE WHALE WHEEL WIND WINDOW WING WIZARD WOLF WOOL ZEBRA ANCHOR ANT AXE BALLOON BANK BELL BONE BOTTLE BREAD BUTTER CAMERA CAMP CANDLE CAVE CHAIR CHEESE CHERRY CHOCOLATE COIN COMET COMPASS CORAL CRAB CRYSTAL CUP DEER DIAMOND DOCTOR DOLPHIN DOOR DUCK EARTH ELEPHANT FENCE FISH FLAME FLUTE FOUNTAIN FOX FRUIT GATE HAMMER HELMET HILL HOSPITAL INK LEAF LIBRARY LIZARD MIRROR MONKEY NET NURSE OPERA PAINT PANDA PEACH PENCIL PENGUIN PIZZA POND POSTCARD PUMPKIN QUEEN RADIO RING ROAD SAND SCARF SEED SHELL SOCK SOUP SPOON SQUIRREL STAMP STRAWBERRY SWORD TENT THREAD TICKET TOMATO TOOTH TREASURE VILLAGE WATCH WITCH YOGURT ZIPPER"
        }
        Language::Portuguese => {
            "ABELHA ABÓBORA ÂNCORA ANEL ANJO AREIA ARTE ASA ÁRVORE AVIÃO BALEIA BALÃO BANCO BANDEIRA BARCO BARRIGA BATATA BIBLIOTECA BICICLETA BISCOITO BOLHA BONECA BORBOLETA BOSQUE BRUXA CABELO CABRA CAFÉ CAIXA CAMA CAMELO CAMISA CAMPO CANETA CANGURU CAPA CARACOL CARANGUEJO CARTA CASA CASTELO CAVALO CAVERNA CEBOLA CEREJA CHAVE CHOCOLATE CHUVA CIDADE CINEMA COBRA COELHO COMETA CORAÇÃO COROA CORUJA COZINHA CRISTAL CUBO DANÇA DENTE DESERTO DIAMANTE DINHEIRO DINOSSAURO DOCE DRAGÃO ELEFANTE ESCADA ESCOLA ESCUDO ESPELHO ESPINHO ESPUMA ESTRELA FACA FAROL FANTASMA FEIJÃO FOGO FOLHA FLORESTA FLOR FOGUETE FORMIGA FORTALEZA FRUTA GARFO GARRAFA GATO GELO GIRAFA GLOBO GOLFINHO GOTA GUARDA GUITARRA ILHA ÍMÃ JANELA JARDIM JOANINHA JOGO JUÍZ LÂMPADA LARANJA LEÃO LEITE LIVRO LUA LUVA MACACO MADEIRA MAGIA MALA MANGA MAPA MAR MARTE MEL MÉDICO MESA METAL MOEDA MOLHO MONSTRO MONTANHA MORANGO MOTOR MURALHA MÚSICA NARIZ NAVIO NEVE NINHO NOITE NUVEM OCEANO OLHO ONDA OSSO OVO PANDA PANELA PÃO PAPEL PARQUE PASSARINHO PATO PEDRA PEIXE PENA PENTE PÉROLA PIANO PICOLÉ PIMENTA PINGUIM PIRATA PLANETA PLUMA PONTE PORTA PORTO PRATA PRAIA PRINCESA PRISÃO QUEIJO RAIO RAINHA RAPOSA REI RELÓGIO RIO ROBÔ ROCHA RODA ROSA SABÃO SAL SAPATO SAPO SATÉLITE SELO SEMENTE SEREIA SOL SOM SOMBRA SONHO SORVETE SOPA TARTARUGA TELEFONE TEMPLO TESOURA TESOURO TIGRE TINTA TOMATE TORRE TREM TRIÂNGULO TROVÃO TUBARÃO TÚNEL UVA VALE VELA VENTO VIDRO VIOLINO VULCÃO XÍCARA ZEBRA"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(seed: u64) -> Game {
        Game::new(4, seed, &Setup::default()).unwrap()
    }
    fn actor(g: &Game, role: Role) -> usize {
        g.seats
            .iter()
            .position(|s| s.team == g.team && s.role == role)
            .unwrap()
    }
    fn clue(g: &mut Game, number: Option<u8>) {
        g.play(
            actor(g, Role::Spymaster),
            Move::Clue {
                word: "quintessential".into(),
                number,
            },
        )
        .unwrap();
    }
    #[test]
    fn decks_are_large_unique_and_board_is_balanced() {
        for language in [Language::English, Language::Portuguese] {
            let words: Vec<_> = deck(language).split_whitespace().collect();
            let unique: std::collections::HashSet<_> = words.iter().map(|s| normalize(s)).collect();
            assert_eq!(words.len(), unique.len());
            assert!(words.len() >= 150);
            for seed in 0..32 {
                let g = Game::new(
                    4,
                    seed,
                    &Setup {
                        language,
                        seats: vec![],
                    },
                )
                .unwrap();
                assert_eq!(g.cards.len(), 25);
                assert_eq!(g.remaining()[g.team.index()], 9);
                assert_eq!(g.remaining()[g.team.other().index()], 8);
                assert_eq!(
                    g.cards
                        .iter()
                        .filter(|c| c.identity == Identity::Assassin)
                        .count(),
                    1
                );
            }
        }
    }
    #[test]
    fn setup_validates_every_team_role_and_count() {
        for n in 4..=16 {
            assert!(Setup::default().validate(n).is_ok());
        }
        assert!(Setup::default().validate(3).is_err());
        assert!(Setup::default().validate(17).is_err());
        let mut s = Setup::default();
        s.resize(4);
        s.seats[2].role = Role::Spymaster;
        assert!(s.validate(4).is_err());
        s.seats[2].role = Role::Operative;
        s.seats[3].team = Team::Red;
        assert!(s.validate(4).is_err());
    }
    #[test]
    fn operative_and_invalid_view_never_contain_unrevealed_key() {
        let mut g = game(1);
        for you in [2, 3, 99] {
            assert!(g.view(you).cards.iter().all(|c| c.identity.is_none()));
            let json = serde_json::to_value(g.view(you)).unwrap();
            assert!(
                json["cards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|c| c["identity"].is_null())
            );
        }
        assert!(g.view(0).cards.iter().all(|c| c.identity.is_some()));
        assert!(g.view(1).cards.iter().all(|c| c.identity.is_some()));
        clue(&mut g, Some(1));
        let op = actor(&g, Role::Operative);
        let own = g
            .cards
            .iter()
            .position(|c| c.identity.team() == Some(g.team))
            .unwrap();
        g.play(op, Move::Guess { card: own }).unwrap();
        assert_eq!(
            g.view(op)
                .cards
                .iter()
                .filter(|c| c.identity.is_some())
                .count(),
            1
        );
        let restored: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(restored.view(op)).unwrap(),
            serde_json::to_value(g.view(op)).unwrap()
        );
    }
    #[test]
    fn rejects_bad_clues_wrong_roles_and_wrong_team_without_changes() {
        let mut g = game(3);
        let spy = actor(&g, Role::Spymaster);
        let op = actor(&g, Role::Operative);
        for word in ["", "two words", "hello!", "123", &g.cards[0].word.clone()] {
            let before = serde_json::to_string(&g).unwrap();
            assert!(
                g.play(
                    spy,
                    Move::Clue {
                        word: word.into(),
                        number: Some(2)
                    }
                )
                .is_err()
            );
            assert_eq!(before, serde_json::to_string(&g).unwrap());
        }
        assert!(
            g.play(
                op,
                Move::Clue {
                    word: "abc".into(),
                    number: Some(2)
                }
            )
            .is_err()
        );
        assert!(
            g.play(
                spy,
                Move::Clue {
                    word: "abc".into(),
                    number: Some(10)
                }
            )
            .is_err()
        );
        clue(&mut g, Some(1));
        assert!(g.play(spy, Move::Guess { card: 0 }).is_err());
        assert!(
            g.play(actor(&g, Role::Operative) ^ 1, Move::Guess { card: 0 })
                .is_err()
        );
        assert!(g.play(op, Move::Guess { card: 25 }).is_err());
    }
    #[test]
    fn numbered_clue_limits_guesses_and_pass_needs_first_guess() {
        let mut g = game(4);
        clue(&mut g, Some(1));
        let op = actor(&g, Role::Operative);
        let t = g.team;
        assert!(g.play(op, Move::Pass).is_err());
        for _ in 0..2 {
            let card = g
                .cards
                .iter()
                .position(|c| !c.revealed && c.identity.team() == Some(t))
                .unwrap();
            g.play(op, Move::Guess { card }).unwrap();
        }
        assert_eq!(g.phase, Phase::Clue);
        assert_eq!(g.team, t.other());
        assert_eq!(g.turn, 2);
    }
    #[test]
    fn zero_and_unlimited_allow_extra_guesses_and_reject_revealed_card() {
        for number in [Some(0), None] {
            let mut g = game(5);
            clue(&mut g, number);
            let op = actor(&g, Role::Operative);
            for _ in 0..3 {
                let card = g
                    .cards
                    .iter()
                    .position(|c| !c.revealed && c.identity.team() == Some(g.team))
                    .unwrap();
                g.play(op, Move::Guess { card }).unwrap();
                assert!(g.play(op, Move::Guess { card }).is_err());
            }
            assert_eq!(g.guesses_left, None);
            g.play(op, Move::Pass).unwrap();
            assert_eq!(g.phase, Phase::Clue);
        }
    }
    #[test]
    fn neutral_and_enemy_cards_end_turn() {
        for identity in [Identity::Neutral, Identity::Red, Identity::Blue] {
            let mut g = game(6);
            if identity.team() == Some(g.team) {
                continue;
            }
            clue(&mut g, Some(3));
            let t = g.team;
            let op = actor(&g, Role::Operative);
            let card = g.cards.iter().position(|c| c.identity == identity).unwrap();
            g.play(op, Move::Guess { card }).unwrap();
            assert_eq!(g.team, t.other());
            assert_eq!(g.phase, Phase::Clue);
        }
    }
    #[test]
    fn assassin_loses_and_finished_views_reveal_key() {
        let mut g = game(7);
        clue(&mut g, Some(1));
        let t = g.team;
        let card = g
            .cards
            .iter()
            .position(|c| c.identity == Identity::Assassin)
            .unwrap();
        g.play(actor(&g, Role::Operative), Move::Guess { card })
            .unwrap();
        assert_eq!(g.winner, Some(t.other()));
        assert!(g.finished());
        assert!(g.view(2).cards.iter().all(|c| c.identity.is_some()));
        assert!(g.play(0, Move::Pass).is_err());
    }
    #[test]
    fn finding_last_agent_can_win_either_team() {
        for enemy in [false, true] {
            let mut g = game(8);
            clue(&mut g, None);
            let t = if enemy { g.team.other() } else { g.team };
            let last = g
                .cards
                .iter()
                .position(|c| c.identity.team() == Some(t))
                .unwrap();
            for (i, c) in g.cards.iter_mut().enumerate() {
                if c.identity.team() == Some(t) && i != last {
                    c.revealed = true;
                }
            }
            g.play(actor(&g, Role::Operative), Move::Guess { card: last })
                .unwrap();
            assert_eq!(g.winner, Some(t));
        }
    }
    #[test]
    fn forfeit_ends_match_but_invalid_seat_does_nothing() {
        let mut g = game(9);
        g.forfeit(99);
        assert!(!g.finished());
        g.forfeit(0);
        assert_eq!(g.winner, Some(Team::Blue));
    }
    #[test]
    fn portuguese_clues_are_accent_insensitive() {
        assert_eq!(normalize("CORAÇÃO"), "coracao");
        let mut g = game(1);
        g.cards[0].word = "CORAÇÃO".into();
        let spy = actor(&g, Role::Spymaster);
        assert!(
            g.play(
                spy,
                Move::Clue {
                    word: "coracao".into(),
                    number: Some(1)
                }
            )
            .is_err()
        );
    }
}
