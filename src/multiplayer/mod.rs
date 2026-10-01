//! Shared wire protocol and authoritative, renderer-independent card game rules.
pub mod coup;
pub mod reverie;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameKind {
    Court,
    Reverie,
}
impl GameKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::Court => "Coupe",
            Self::Reverie => "Dicksit",
        }
    }
    pub fn limits(self) -> (usize, usize) {
        match self {
            Self::Court => (2, 6),
            Self::Reverie => (3, 8),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Create { game: GameKind, name: String },
    Join { room: String, name: String },
    Resume { room: String, token: String },
    Play { revision: u64, command: Command },
    Leave,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "move", rename_all = "snake_case")]
pub enum Command {
    Ready(bool),
    Start,
    Rematch,
    Court(coup::Move),
    Reverie(reverie::Move),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub room: String,
    pub token: String,
    pub game: GameKind,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Connected,
    Disconnected { reason: String },
    Welcome { session: Session },
    State { room: Box<RoomView> },
    Error { message: String },
    Left,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberView {
    pub name: String,
    pub connected: bool,
    pub ready: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomView {
    pub code: String,
    pub game: GameKind,
    pub revision: u64,
    pub epoch: u64,
    pub you: usize,
    pub host: usize,
    pub members: Vec<MemberView>,
    pub court: Option<coup::View>,
    pub reverie: Option<reverie::View>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Rng(rand_chacha::ChaCha12Rng);
impl Rng {
    pub fn new(seed: u64) -> Self {
        use rand_chacha::rand_core::SeedableRng;
        Self(rand_chacha::ChaCha12Rng::seed_from_u64(seed))
    }
    pub fn next(&mut self) -> u64 {
        use rand_chacha::rand_core::RngCore;
        self.0.next_u64()
    }
    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            values.swap(i, (self.next() % (i + 1) as u64) as usize);
        }
    }
}
pub fn clean_text(text: &str, max: usize) -> String {
    text.chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .take(max)
        .collect::<String>()
        .trim()
        .to_owned()
}
