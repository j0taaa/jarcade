use crate::{air_hockey::Hockey, battleship::Naval};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Hockey,
    Naval,
}
impl Kind {
    pub fn title(self) -> &'static str {
        match self {
            Self::Hockey => "Air Hockey",
            Self::Naval => "Batalha Naval",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    version: u8,
    pub hockey: Hockey,
    pub naval: Naval,
}
impl Default for State {
    fn default() -> Self {
        Self {
            version: 1,
            hockey: Hockey::default(),
            naval: Naval::default(),
        }
    }
}
impl State {
    pub fn interrupt(&mut self) {
        self.hockey.pause();
        self.naval.conceal();
    }
    pub fn encode(&self) -> String {
        let mut s = self.clone();
        s.interrupt();
        serde_json::to_string(&s).unwrap_or_default()
    }
    pub fn decode(data: &str) -> Self {
        if data.len() > 16384 {
            return Self::default();
        }
        let Ok(mut s) = serde_json::from_str::<Self>(data) else {
            return Self::default();
        };
        if s.version != 1 {
            return Self::default();
        }
        if !s.hockey.valid() {
            s.hockey = Hockey::default();
        }
        if !s.naval.valid() {
            s.naval = Naval::default();
        }
        s.interrupt();
        s
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        air_hockey::Phase as H,
        battleship::{Phase as N, Resume},
    };
    #[test]
    fn saves_conceal_and_pause_without_resetting_other_games() {
        let mut s = State::default();
        s.hockey.play();
        s.naval.reveal();
        s.naval.automatic(7);
        let save = s.encode();
        assert!(save.len() < 16384);
        let r = State::decode(&save);
        assert_eq!(r.hockey.phase, H::Paused);
        assert_eq!(
            r.naval.phase,
            N::Cover {
                player: 0,
                resume: Resume::Setup
            }
        );
        let mut json: serde_json::Value = serde_json::from_str(&save).unwrap();
        json["naval"]["round"] = 0.into();
        let repaired = State::decode(&json.to_string());
        assert_eq!(repaired.naval.round, 1);
        assert_eq!(repaired.hockey.phase, H::Paused);
        assert_eq!(State::decode("invalid").hockey.phase, H::Ready);
    }
}
