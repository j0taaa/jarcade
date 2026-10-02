//! Offline, same-device clue/guess game. Hidden targets never enter the public view.
use crate::multiplayer::{Rng, clean_text};
use serde::{Deserialize, Serialize};

pub const MAX_EXTREME: usize = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Ready,
    Peek,
    Handoff,
    Guess,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Deck {
    Everyday,
    Playful,
    Portuguese,
    Custom,
}
impl Deck {
    pub const ALL: [Self; 4] = [
        Self::Everyday,
        Self::Playful,
        Self::Portuguese,
        Self::Custom,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Everyday => "Everyday",
            Self::Playful => "Playful",
            Self::Portuguese => "Em português",
            Self::Custom => "Your spectrum",
        }
    }
    fn pairs(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Everyday => &EVERYDAY,
            Self::Playful => &PLAYFUL,
            Self::Portuguese => &PORTUGUESE,
            Self::Custom => &[],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proximity {
    InTune,
    VeryClose,
    Close,
    Almost,
    Different,
}
impl Proximity {
    pub fn title(self) -> &'static str {
        match self {
            Self::InTune => "On the same wavelength!",
            Self::VeryClose => "Very close",
            Self::Close => "Close",
            Self::Almost => "Almost there",
            Self::Different => "A different wavelength",
        }
    }
}
pub fn proximity(target: f32, guess: f32) -> Proximity {
    let distance = (target - guess).abs();
    if !distance.is_finite() {
        return Proximity::Different;
    }
    if distance <= 0.027 + f32::EPSILON {
        Proximity::InTune
    } else if distance <= 0.062 + f32::EPSILON {
        Proximity::VeryClose
    } else if distance <= 0.10 + f32::EPSILON {
        Proximity::Close
    } else if distance <= 0.14 + f32::EPSILON {
        Proximity::Almost
    } else {
        Proximity::Different
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    phase: Phase,
    target: f32,
    guess: f32,
    deck: Deck,
    pair: usize,
    reversed: bool,
    custom: [String; 2],
    round: u32,
    rng: Rng,
}
pub struct View<'a> {
    pub phase: Phase,
    pub target: Option<f32>,
    pub guess: f32,
    pub left: &'a str,
    pub right: &'a str,
    pub deck: Deck,
    pub round: u32,
    pub result: Option<Proximity>,
}
impl Game {
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            phase: Phase::Ready,
            target: 0.5,
            guess: 0.5,
            deck: Deck::Everyday,
            pair: 0,
            reversed: false,
            custom: ["An everyday thing".into(), "Something extraordinary".into()],
            round: 1,
            rng: Rng::new(seed),
        };
        game.randomize(true);
        game
    }
    fn randomize(&mut self, new_pair: bool) {
        self.target = (0.16 + (self.rng.next() % 680_001) as f32 / 1_000_000.).clamp(0.16, 0.84);
        self.guess = 0.5;
        self.phase = Phase::Ready;
        let len = self.deck.pairs().len();
        if new_pair && len > 1 {
            self.pair = (self.pair + 1 + (self.rng.next() % (len - 1) as u64) as usize) % len;
            self.reversed = self.rng.next() & 1 == 1;
        }
    }
    pub fn view(&self) -> View<'_> {
        let (left, right) = if self.deck == Deck::Custom {
            (&*self.custom[0], &*self.custom[1])
        } else {
            self.deck.pairs()[self.pair]
        };
        let (left, right) = if self.reversed && self.deck != Deck::Custom {
            (right, left)
        } else {
            (left, right)
        };
        View {
            phase: self.phase,
            target: matches!(self.phase, Phase::Peek | Phase::Result).then_some(self.target),
            guess: self.guess,
            left,
            right,
            deck: self.deck,
            round: self.round,
            result: (self.phase == Phase::Result).then(|| proximity(self.target, self.guess)),
        }
    }
    /// Exactly one transition per deliberate button press; no way back to Peek from Guess.
    pub fn advance(&mut self) {
        self.phase = match self.phase {
            Phase::Ready => Phase::Peek,
            Phase::Peek => Phase::Handoff,
            Phase::Handoff => Phase::Guess,
            Phase::Guess => Phase::Result,
            Phase::Result => {
                self.round = self.round.saturating_add(1);
                self.randomize(true);
                Phase::Ready
            }
        };
    }
    pub fn conceal(&mut self) {
        if self.phase == Phase::Peek {
            self.phase = Phase::Ready;
        }
    }
    pub fn set_guess(&mut self, position: f32) -> bool {
        if self.phase != Phase::Guess || !position.is_finite() {
            return false;
        }
        let position = position.clamp(0.02, 0.98);
        let changed = self.guess != position;
        self.guess = position;
        changed
    }
    pub fn shuffle(&mut self) -> bool {
        if !matches!(self.phase, Phase::Ready | Phase::Result) {
            return false;
        }
        self.randomize(true);
        true
    }
    pub fn choose_deck(&mut self, deck: Deck) -> bool {
        if !matches!(self.phase, Phase::Ready | Phase::Result) {
            return false;
        }
        self.deck = deck;
        self.pair = 0;
        self.randomize(true);
        true
    }
    pub fn set_custom(&mut self, left: &str, right: &str) -> bool {
        if !matches!(self.phase, Phase::Ready | Phase::Result) {
            return false;
        }
        let pair = [
            clean_text(left, MAX_EXTREME),
            clean_text(right, MAX_EXTREME),
        ];
        if pair.iter().any(String::is_empty) {
            return false;
        }
        self.custom = pair;
        self.deck = Deck::Custom;
        self.reversed = false;
        self.randomize(false);
        true
    }
    /// Edit the displayed side without re-rolling the target or resetting the turn.
    pub fn edit_side(&mut self, side: usize, text: &str) -> bool {
        if side > 1 || self.phase == Phase::Handoff {
            return false;
        }
        let text = clean_text(text, MAX_EXTREME);
        if text.is_empty() {
            return false;
        }
        let view = self.view();
        let mut pair = [view.left.to_owned(), view.right.to_owned()];
        if pair[side] == text {
            return false;
        }
        pair[side] = text;
        self.custom = pair;
        self.deck = Deck::Custom;
        self.reversed = false;
        true
    }
    pub fn custom(&self) -> &[String; 2] {
        &self.custom
    }
    pub fn encode(&self) -> String {
        serde_json::to_string(self).expect("finite game state")
    }
    pub fn decode(data: &str, seed: u64) -> Self {
        let Ok(mut game) = serde_json::from_str::<Self>(data) else {
            return Self::new(seed);
        };
        if !game.target.is_finite()
            || !(0.16..=0.84).contains(&game.target)
            || !game.guess.is_finite()
            || !(0.02..=0.98).contains(&game.guess)
            || game.round == 0
            || (game.deck != Deck::Custom && game.pair >= game.deck.pairs().len())
            || game
                .custom
                .iter()
                .any(|s| s.is_empty() || clean_text(s, MAX_EXTREME) != *s)
        {
            return Self::new(seed);
        }
        // Restoring a device must never display a private peek or bypass the handoff.
        game.conceal();
        if game.phase == Phase::Guess {
            game.phase = Phase::Handoff;
        }
        game
    }
}

const EVERYDAY: [(&str, &str); 20] = [
    ("Cold", "Hot"),
    ("Quiet", "Loud"),
    ("Ordinary", "Extraordinary"),
    ("Tiny", "Enormous"),
    ("Cheap", "Expensive"),
    ("Easy", "Difficult"),
    ("Slow", "Fast"),
    ("Sweet", "Bitter"),
    ("Soft", "Hard"),
    ("Old-fashioned", "Futuristic"),
    ("Useful", "Useless"),
    ("Relaxing", "Stressful"),
    ("Delicate", "Sturdy"),
    ("Simple", "Complicated"),
    ("A little risky", "Very risky"),
    ("Casual", "Fancy"),
    ("A quick visit", "A whole-day adventure"),
    ("Everyday treat", "Special occasion"),
    ("At home", "Out in the world"),
    ("A small surprise", "A huge surprise"),
];
const PLAYFUL: [(&str, &str); 20] = [
    ("Sidekick energy", "Main character energy"),
    ("Tiny inconvenience", "Total disaster"),
    ("A boring superpower", "An amazing superpower"),
    ("Cuddly creature", "Spooky creature"),
    ("A sensible purchase", "A ridiculous purchase"),
    ("Secret talent", "Party trick"),
    ("A gentle prank", "An outrageous prank"),
    ("Picnic food", "Midnight snack"),
    ("Friendly alien", "Terrifying alien"),
    ("One more episode", "Stay up all night"),
    ("An easy quest", "A final boss"),
    ("A cozy hideout", "An epic castle"),
    ("Keep it forever", "Forget it tomorrow"),
    ("A tiny celebration", "A giant party"),
    ("A quiet hobby", "An adventure sport"),
    ("Believable story", "Wild fairy tale"),
    ("A bad band name", "A great band name"),
    ("Lucky accident", "Genius plan"),
    ("One suitcase", "A moving truck"),
    ("A silly fear", "A reasonable fear"),
];
const PORTUGUESE: [(&str, &str); 20] = [
    ("Frio", "Quente"),
    ("Silencioso", "Barulhento"),
    ("Comum", "Extraordinário"),
    ("Pequeno", "Enorme"),
    ("Barato", "Caro"),
    ("Fácil", "Difícil"),
    ("Devagar", "Rápido"),
    ("Doce", "Amargo"),
    ("Macio", "Duro"),
    ("Antigo", "Futurista"),
    ("Útil", "Inútil"),
    ("Relaxante", "Estressante"),
    ("Lanche da tarde", "Comida de madrugada"),
    ("Passeio tranquilo", "Grande aventura"),
    ("Ideia sensata", "Ideia maluca"),
    ("Pouco arriscado", "Muito arriscado"),
    ("Talento escondido", "Truque de festa"),
    ("Pequena surpresa", "Enorme surpresa"),
    ("História realista", "Conto de fadas"),
    ("Sorte", "Planejamento"),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handoff_hides_target_and_guess_cannot_reopen_it() {
        let mut g = Game::new(1);
        assert_eq!(g.view().target, None);
        assert!(!g.set_guess(0.2));
        g.advance();
        let target = g.view().target.unwrap();
        g.advance();
        assert_eq!(g.view().phase, Phase::Handoff);
        assert_eq!(g.view().target, None);
        g.advance();
        assert!(!g.shuffle());
        assert!(!g.choose_deck(Deck::Playful));
        assert!(!g.set_custom("A", "B"));
        assert_eq!(g.view().target, None);
        assert!(g.set_guess(target));
        g.advance();
        assert_eq!(g.view().target, Some(target));
        assert_eq!(g.view().result, Some(Proximity::InTune));
        g.advance();
        assert_eq!(g.view().round, 2);
        assert_eq!(g.view().target, None);
    }
    #[test]
    fn privacy_survives_background_exit_and_reload() {
        let mut g = Game::new(9);
        g.advance();
        let target = g.view().target;
        let restored = Game::decode(&g.encode(), 0);
        assert_eq!(restored.view().target, None);
        assert_eq!(restored.view().phase, Phase::Ready);
        g.conceal();
        assert_eq!(g.view().target, None);
        g.advance();
        assert_eq!(g.view().target, target);
        g.advance();
        g.advance();
        g.set_guess(0.76);
        let mut restored = Game::decode(&g.encode(), 0);
        assert_eq!(restored.view().phase, Phase::Handoff);
        assert_eq!(restored.view().target, None);
        restored.advance();
        assert_eq!(restored.view().guess, 0.76);
    }
    #[test]
    fn proximity_boundaries_are_symmetric_and_unscored() {
        for sign in [-1., 1.] {
            for (d, result) in [
                (0.027, Proximity::InTune),
                (0.028, Proximity::VeryClose),
                (0.062, Proximity::VeryClose),
                (0.063, Proximity::Close),
                (0.1, Proximity::Close),
                (0.101, Proximity::Almost),
                (0.14, Proximity::Almost),
                (0.141, Proximity::Different),
            ] {
                assert_eq!(proximity(0.5, 0.5 + sign * d), result);
            }
        }
        assert_eq!(proximity(f32::NAN, 0.5), Proximity::Different);
    }
    #[test]
    fn seeds_are_repeatable_and_decks_do_not_repeat_adjacent_pairs() {
        for deck in Deck::ALL {
            let mut a = Game::new(777);
            a.choose_deck(deck);
            let mut b = a.clone();
            for _ in 0..100 {
                let previous = (a.view().left.to_owned(), a.view().right.to_owned());
                a.shuffle();
                b.shuffle();
                assert_eq!(a.encode(), b.encode());
                a.advance();
                assert!((0.16..=0.84).contains(&a.view().target.unwrap()));
                a.conceal();
                if deck != Deck::Custom {
                    assert_ne!(previous, (a.view().left.into(), a.view().right.into()));
                }
            }
        }
    }
    #[test]
    fn saves_reject_corruption_and_custom_labels_are_bounded() {
        let mut g = Game::new(8);
        assert!(!g.set_custom("  ", "B"));
        assert!(g.set_custom(&"界".repeat(200), "  Night\n  "));
        assert_eq!(g.view().left.chars().count(), MAX_EXTREME);
        assert_eq!(g.view().right, "Night");
        let data = g.encode();
        assert_eq!(Game::decode(&data, 0).encode(), data);
        for value in ["", "null", "{}"] {
            assert_eq!(Game::decode(value, 12).encode(), Game::new(12).encode());
        }
        for (key, value) in [
            ("target", serde_json::json!(1.1)),
            ("guess", serde_json::json!(-0.1)),
            ("round", serde_json::json!(0)),
            ("custom", serde_json::json!(["", "Night"])),
        ] {
            let mut v: serde_json::Value = serde_json::from_str(&data).unwrap();
            v[key] = value;
            assert_eq!(
                Game::decode(&v.to_string(), 12).encode(),
                Game::new(12).encode()
            );
        }
        let mut v: serde_json::Value = serde_json::from_str(&data).unwrap();
        v["deck"] = serde_json::json!("Everyday");
        v["pair"] = serde_json::json!(1000);
        assert_eq!(
            Game::decode(&v.to_string(), 12).encode(),
            Game::new(12).encode()
        );
    }
    #[test]
    fn guess_clamps_and_ignores_nonfinite_input() {
        let mut g = Game::new(2);
        for _ in 0..3 {
            g.advance();
        }
        assert!(g.set_guess(-2.));
        assert_eq!(g.view().guess, 0.02);
        assert!(g.set_guess(2.));
        assert_eq!(g.view().guess, 0.98);
        assert!(!g.set_guess(f32::NAN));
        assert_eq!(g.view().guess, 0.98);
    }

    #[test]
    fn editing_displayed_sides_preserves_opposite_side_target_guess_and_phase() {
        let mut g = Game::new(4);
        g.reversed = true;
        let right = g.view().right.to_owned();
        let target = g.target;
        assert!(g.edit_side(0, "  Anything I want  "));
        assert_eq!(g.view().left, "Anything I want");
        assert_eq!(g.view().right, right);
        assert_eq!(g.view().phase, Phase::Ready);
        assert_eq!(g.target, target);
        assert_eq!(g.view().target, None);
        g.advance();
        assert!(g.edit_side(1, "A different idea"));
        assert_eq!(g.view().target, Some(target));
        assert_eq!(g.view().phase, Phase::Peek);
        g.advance();
        let handoff = g.encode();
        assert!(!g.edit_side(0, "Blocked during handoff"));
        assert_eq!(g.encode(), handoff);
        g.advance();
        g.set_guess(0.73);
        assert!(g.edit_side(0, "Frio de inverno"));
        assert_eq!(g.view().phase, Phase::Guess);
        assert_eq!(g.view().target, None);
        assert_eq!(g.view().guess, 0.73);
        assert_eq!(g.view().round, 1);
        g.advance();
        assert_eq!(g.view().target, Some(target));
        assert!(g.edit_side(1, "Calor de verão"));
        assert_eq!(g.view().phase, Phase::Result);
        assert_eq!(Game::decode(&g.encode(), 0).encode(), g.encode());
    }

    #[test]
    fn empty_unchanged_and_invalid_side_edits_do_not_mutate_a_round() {
        let mut g = Game::new(5);
        let original = g.encode();
        let left = g.view().left.to_owned();
        for (side, text) in [(0, " \n\t "), (2, "Invalid"), (0, left.as_str())] {
            assert!(!g.edit_side(side, text));
            assert_eq!(g.encode(), original);
        }
        assert!(g.edit_side(1, &"🌞".repeat(500)));
        assert_eq!(g.view().right.chars().count(), MAX_EXTREME);
        assert!(g.encode().len() < 4096);
    }
}
