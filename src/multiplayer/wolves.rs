//! Authoritative classic social deduction. Only per-seat projections cross the wire.
use super::{Rng, clean_text};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Villager,
    Werewolf,
    WolfSeer,
    Seer,
    Doctor,
    Bodyguard,
    Gunner,
    Fool,
    SerialKiller,
    AuraSeer,
    Medium,
    Witch,
    Avenger,
    AlphaWolf,
    JuniorWolf,
    ToughGuy,
}
impl Role {
    pub fn title(self) -> &'static str {
        match self {
            Self::Villager => "Villager",
            Self::Werewolf => "Werewolf",
            Self::WolfSeer => "Wolf seer",
            Self::Seer => "Seer",
            Self::Doctor => "Doctor",
            Self::Bodyguard => "Bodyguard",
            Self::Gunner => "Gunner",
            Self::Fool => "Fool",
            Self::SerialKiller => "Serial killer",
            Self::AuraSeer => "Aura seer",
            Self::Medium => "Medium",
            Self::Witch => "Witch",
            Self::Avenger => "Avenger",
            Self::AlphaWolf => "Alpha werewolf",
            Self::JuniorWolf => "Junior werewolf",
            Self::ToughGuy => "Tough guy",
        }
    }
    pub fn wolf(self) -> bool {
        matches!(
            self,
            Self::Werewolf | Self::WolfSeer | Self::AlphaWolf | Self::JuniorWolf
        )
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Villager => "Find the wolves. Discuss by day and vote carefully.",
            Self::Werewolf => "Hunt with your pack at night. Stay hidden by day.",
            Self::WolfSeer => "Inspect a role at night and choose a victim with your pack.",
            Self::Seer => "Inspect one player’s exact role each night. Your findings are private.",
            Self::Doctor => "Shield another player from night attacks.",
            Self::Bodyguard => {
                "Take attacks for a neighbor. Survive one hit; the second kills you."
            }
            Self::Gunner => {
                "Two shots, one per day. Firing reveals you. First-day discussion is too early to shoot."
            }
            Self::Fool => "Get yourself voted out to win alone. Night attacks do not win for you.",
            Self::SerialKiller => {
                "Hunt alone at night. Outlive everyone. Wolf attacks cannot kill you."
            }
            Self::AuraSeer => "Read a player’s aura at night: village, wolf, or unknown.",
            Self::Medium => {
                "Speak with ghosts anonymously. Revive one fallen villager at night, once."
            }
            Self::Witch => {
                "One shield potion, spent only when attacked. One poison, available after night one."
            }
            Self::Avenger => "After the first night, mark a player. They fall when you die.",
            Self::AlphaWolf => {
                "Your night hunt vote counts twice. Send one private pack message each day."
            }
            Self::JuniorWolf => "Hunt with the pack. Mark someone to take with you when you die.",
            Self::ToughGuy => {
                "Guard a neighbor. An attack exposes you and the attacker to each other; you die after the day ends."
            }
        }
    }
}
pub const ROLES: [Role; 16] = [
    Role::Villager,
    Role::Werewolf,
    Role::Seer,
    Role::Doctor,
    Role::Bodyguard,
    Role::Gunner,
    Role::Fool,
    Role::WolfSeer,
    Role::SerialKiller,
    Role::AuraSeer,
    Role::Medium,
    Role::Witch,
    Role::Avenger,
    Role::AlphaWolf,
    Role::JuniorWolf,
    Role::ToughGuy,
];
impl Role {
    pub fn village(self) -> bool {
        !self.wolf() && !matches!(self, Self::Fool | Self::SerialKiller)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Classic,
    #[default]
    Advanced,
    Custom,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setup {
    pub preset: Preset,
    pub roles: Vec<Role>,
}
impl Setup {
    pub fn roles_for(&self, n: usize) -> Result<Vec<Role>, &'static str> {
        if !(6..=16).contains(&n) {
            return Err("Invite 6–16 players");
        }
        let roles = if self.preset == Preset::Custom {
            if self.roles.len() != n {
                return Err("Choose exactly one role per player");
            }
            self.roles.clone()
        } else {
            let mut roles = vec![Role::Werewolf; (n / 4).max(2)];
            if self.preset == Preset::Advanced {
                roles[0] = Role::AlphaWolf;
                roles[1] = Role::JuniorWolf;
                if roles.len() > 2 {
                    roles[2] = Role::WolfSeer;
                }
            } else if n >= 10 {
                roles[0] = Role::WolfSeer;
            }
            roles.extend(if self.preset == Preset::Advanced {
                [Role::AuraSeer, Role::Witch]
            } else {
                [Role::Seer, Role::Doctor]
            });
            if n >= 7 {
                roles.push(if self.preset == Preset::Advanced {
                    Role::Avenger
                } else {
                    Role::Gunner
                });
            }
            if n >= 8 {
                roles.push(if self.preset == Preset::Advanced {
                    Role::Medium
                } else {
                    Role::Bodyguard
                });
            }
            if n >= 9 {
                roles.push(Role::Fool);
            }
            if n >= 10 && self.preset == Preset::Advanced {
                roles.push(Role::ToughGuy);
            }
            if n >= 12 {
                roles.push(Role::SerialKiller);
            }
            roles.resize(n, Role::Villager);
            roles
        };
        let wolves = roles.iter().filter(|r| r.wolf()).count();
        if wolves == 0 || wolves * 2 >= n {
            return Err("Choose at least one wolf and a larger non-wolf team");
        }
        for role in ROLES {
            if !matches!(role, Role::Villager | Role::Werewolf)
                && roles.iter().filter(|r| **r == role).count() > 1
            {
                return Err("Special roles can appear only once");
            }
        }
        Ok(roles)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Night,
    Dawn,
    Discussion,
    Vote,
    Finished,
}
impl Phase {
    pub fn title(self) -> &'static str {
        match self {
            Self::Night => "Night",
            Self::Dawn => "Dawn",
            Self::Discussion => "Discussion",
            Self::Vote => "Vote",
            Self::Finished => "Finished",
        }
    }
    fn seconds(self) -> u64 {
        match self {
            Self::Night => 45,
            Self::Dawn => 12,
            Self::Discussion => 75,
            Self::Vote => 35,
            Self::Finished => 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    Village,
    Wolves,
    Fool,
    SerialKiller,
    Draw,
}
impl Team {
    pub fn title(self) -> &'static str {
        match self {
            Self::Village => "The village wins",
            Self::Wolves => "The wolves win",
            Self::Fool => "The fool wins",
            Self::SerialKiller => "The lone killer wins",
            Self::Draw => "A quiet draw",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Village,
    Pack,
    Ghosts,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Move {
    Night {
        target: Option<usize>,
        kill: Option<usize>,
    },
    Vote {
        target: Option<usize>,
    },
    Ready,
    Shoot {
        target: usize,
    },
    Mark {
        target: usize,
    },
    Chat {
        channel: Channel,
        text: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chat {
    pub who: Option<usize>,
    pub text: String,
    pub channel: Channel,
    pub day: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Finding {
    pub day: u32,
    pub target: usize,
    pub role: Option<Role>,
    pub aura: Option<Aura>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Aura {
    Village,
    Wolf,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Power {
    pub heal: bool,
    pub poison: bool,
    pub revive: bool,
    pub hits: u8,
    pub shot_day: u32,
    pub alpha_day: u32,
    pub injured: Option<u32>,
}
impl Default for Power {
    fn default() -> Self {
        Self {
            heal: true,
            poison: true,
            revive: true,
            hits: 0,
            shot_day: 0,
            alpha_day: 0,
            injured: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub name: String,
    pub alive: bool,
    pub role: Option<Role>,
    pub revealed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct View {
    pub phase: Phase,
    pub day: u32,
    pub deadline: u64,
    pub players: Vec<PlayerView>,
    pub role: Role,
    pub locked: bool,
    pub submitted: usize,
    pub alive: usize,
    pub targets: Vec<usize>,
    pub victims: Vec<usize>,
    pub bullets: u8,
    pub power: Power,
    pub mark: Option<usize>,
    pub can_mark: bool,
    pub pool: Vec<(Role, usize)>,
    pub findings: Vec<Finding>,
    pub chat: Vec<Chat>,
    pub events: Vec<String>,
    pub ballots: Option<Vec<Option<usize>>>,
    pub pack_votes: Option<Vec<(usize, Option<usize>)>>,
    pub winner: Option<Team>,
    pub winner_seat: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    names: Vec<String>,
    roles: Vec<Role>,
    alive: Vec<bool>,
    revealed: Vec<bool>,
    pub phase: Phase,
    pub day: u32,
    pub deadline: u64,
    ready: Vec<bool>,
    targets: Vec<Option<usize>>,
    kills: Vec<Option<usize>>,
    votes: Vec<Option<usize>>,
    ballots: Option<Vec<Option<usize>>>,
    bullets: Vec<u8>,
    power: Vec<Power>,
    marks: Vec<Option<usize>>,
    findings: Vec<Vec<Finding>>,
    chat: Vec<Chat>,
    last_chat: Vec<u64>,
    events: Vec<String>,
    winner: Option<Team>,
    winner_seat: Option<usize>,
}
impl Game {
    pub fn new(names: Vec<String>, seed: u64, now: u64) -> Result<Self, &'static str> {
        Self::with_setup(
            names,
            seed,
            now,
            &Setup {
                preset: Preset::Classic,
                roles: vec![],
            },
        )
    }
    pub fn with_setup(
        names: Vec<String>,
        seed: u64,
        now: u64,
        setup: &Setup,
    ) -> Result<Self, &'static str> {
        let n = names.len();
        let mut roles = setup.roles_for(n)?;
        Rng::new(seed).shuffle(&mut roles);
        Ok(Self {
            names,
            roles,
            alive: vec![true; n],
            revealed: vec![false; n],
            phase: Phase::Night,
            day: 1,
            deadline: now.saturating_add(Phase::Night.seconds()),
            ready: vec![false; n],
            targets: vec![None; n],
            kills: vec![None; n],
            votes: vec![None; n],
            ballots: None,
            bullets: vec![2; n],
            power: vec![Power::default(); n],
            marks: vec![None; n],
            findings: vec![vec![]; n],
            chat: vec![],
            last_chat: vec![0; n],
            events: vec!["The village falls asleep. Keep your role secret.".into()],
            winner: None,
            winner_seat: None,
        })
    }
    pub fn finished(&self) -> bool {
        self.phase == Phase::Finished
    }
    pub fn view(&self, you: usize) -> View {
        let role = self.roles[you];
        View {
            phase: self.phase,
            day: self.day,
            deadline: self.deadline,
            players: self
                .names
                .iter()
                .enumerate()
                .map(|(i, name)| PlayerView {
                    name: name.clone(),
                    alive: self.alive[i],
                    revealed: self.revealed[i],
                    role: (i == you
                        || !self.alive[i]
                        || self.revealed[i]
                        || self.finished()
                        || (role.wolf() && self.roles[i].wolf()))
                    .then_some(self.roles[i]),
                })
                .collect(),
            role,
            locked: self.ready[you] || !self.alive[you],
            submitted: self
                .ready
                .iter()
                .zip(&self.alive)
                .filter(|(r, a)| **r && **a)
                .count(),
            alive: self.alive.iter().filter(|a| **a).count(),
            targets: self.ability_targets(you),
            victims: if (role.wolf()
                || (role == Role::Witch && self.day > 1 && self.power[you].poison))
                && self.alive[you]
            {
                self.alive
                    .iter()
                    .enumerate()
                    .filter(|(i, a)| **a && *i != you && (!role.wolf() || !self.roles[*i].wolf()))
                    .map(|(i, _)| i)
                    .collect()
            } else {
                vec![]
            },
            bullets: if role == Role::Gunner {
                self.bullets[you]
            } else {
                0
            },
            power: self.power[you].clone(),
            mark: self.marks[you],
            can_mark: self.alive[you]
                && !self.finished()
                && (role == Role::JuniorWolf
                    || (role == Role::Avenger && (self.day > 1 || self.phase != Phase::Night))),
            pool: ROLES
                .into_iter()
                .filter_map(|r| {
                    let n = self.roles.iter().filter(|role| **role == r).count();
                    (n > 0).then_some((r, n))
                })
                .collect(),
            findings: self.findings[you].clone(),
            chat: self
                .chat
                .iter()
                .filter(|c| match c.channel {
                    Channel::Village => true,
                    Channel::Pack => role.wolf(),
                    Channel::Ghosts => !self.alive[you] || role == Role::Medium,
                })
                .cloned()
                .collect(),
            events: self.events.clone(),
            ballots: self.ballots.clone(),
            pack_votes: role.wolf().then(|| {
                self.roles
                    .iter()
                    .enumerate()
                    .filter(|(i, r)| r.wolf() && self.alive[*i])
                    .map(|(i, _)| (i, self.kills[i]))
                    .collect()
            }),
            winner: self.winner,
            winner_seat: self.winner_seat,
        }
    }
    fn ability_targets(&self, you: usize) -> Vec<usize> {
        if !self.alive[you] {
            return vec![];
        }
        let role = self.roles[you];
        if self.phase == Phase::Night && role == Role::Medium {
            return self
                .alive
                .iter()
                .enumerate()
                .filter(|(i, a)| !**a && self.roles[*i].village() && self.power[you].revive)
                .map(|(i, _)| i)
                .collect();
        }
        let active = if matches!(self.phase, Phase::Discussion | Phase::Vote) {
            role == Role::Gunner
                && self.bullets[you] > 0
                && self.power[you].shot_day != self.day
                && (self.day > 1 || self.phase == Phase::Vote)
        } else {
            self.phase == Phase::Night
                && (matches!(
                    role,
                    Role::Seer
                        | Role::WolfSeer
                        | Role::AuraSeer
                        | Role::Doctor
                        | Role::Bodyguard
                        | Role::SerialKiller
                        | Role::ToughGuy
                        | Role::JuniorWolf
                ) || (role == Role::Avenger && self.day > 1)
                    || (role == Role::Witch && self.power[you].heal))
        };
        if !active {
            return vec![];
        }
        self.alive
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                **a && (*i != you || role == Role::Witch)
                    && !(role == Role::WolfSeer && self.roles[*i].wolf())
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn play(&mut self, you: usize, movement: Move, now: u64) -> Result<(), &'static str> {
        if you >= self.names.len() {
            return Err("Unknown player");
        }
        if let Move::Chat { channel, text } = movement {
            return self.say(you, channel, &text, now);
        }
        if self.finished() {
            return Err("This match has finished");
        }
        if now >= self.deadline {
            return Err("This phase has ended. Wait for the village to update");
        }
        if !self.alive[you] {
            return Err("Ghosts cannot act or vote");
        }
        if let Move::Mark { target } = movement {
            if target >= self.roles.len()
                || !self.alive[target]
                || target == you
                || !self.view(you).can_mark
            {
                return Err("That revenge mark is not available");
            }
            self.marks[you] = Some(target);
            return Ok(());
        }
        if self.ready[you] {
            return Err("Your choice is locked for this phase");
        }
        match movement {
            Move::Night { target, kill } if self.phase == Phase::Night => {
                if target.is_some_and(|t| !self.ability_targets(you).contains(&t)) {
                    return Err("That ability target is not available");
                }
                if kill.is_some_and(|t| {
                    t >= self.roles.len()
                        || (!self.roles[you].wolf()
                            && !(self.roles[you] == Role::Witch
                                && self.day > 1
                                && self.power[you].poison))
                        || !self.alive[t]
                        || t == you
                        || (self.roles[you].wolf() && self.roles[t].wolf())
                }) {
                    return Err("The pack can only hunt a living non-wolf");
                }
                self.targets[you] = target;
                self.kills[you] = kill;
                if matches!(self.roles[you], Role::Avenger | Role::JuniorWolf) && target.is_some() {
                    self.marks[you] = target;
                }
                self.ready[you] = true;
            }
            Move::Vote { target } if self.phase == Phase::Vote => {
                if target.is_some_and(|t| t >= self.roles.len() || !self.alive[t] || t == you) {
                    return Err("Vote for another living player, or abstain");
                }
                self.votes[you] = target;
                self.ready[you] = true;
            }
            Move::Ready if matches!(self.phase, Phase::Dawn | Phase::Discussion) => {
                self.ready[you] = true
            }
            Move::Shoot { target } if matches!(self.phase, Phase::Discussion | Phase::Vote) => {
                if self.roles[you] != Role::Gunner
                    || self.bullets[you] == 0
                    || !self.ability_targets(you).contains(&target)
                {
                    return Err("That shot is not available");
                }
                self.bullets[you] -= 1;
                self.power[you].shot_day = self.day;
                self.revealed[you] = true;
                self.event(format!(
                    "{} revealed as Gunner and shot {}.",
                    self.names[you], self.names[target]
                ));
                self.eliminate(target);
                self.check_winner();
            }
            _ => return Err("That action is not available in this phase"),
        }
        if !self.finished() && self.all_ready() {
            self.advance(now);
        }
        Ok(())
    }
    fn say(
        &mut self,
        you: usize,
        channel: Channel,
        text: &str,
        now: u64,
    ) -> Result<(), &'static str> {
        let allowed = match channel {
            Channel::Village => {
                self.alive[you]
                    && (self.finished()
                        || matches!(self.phase, Phase::Dawn | Phase::Discussion | Phase::Vote))
            }
            Channel::Pack => {
                self.alive[you]
                    && self.roles[you].wolf()
                    && (self.phase == Phase::Night
                        || self.finished()
                        || (self.roles[you] == Role::AlphaWolf
                            && self.power[you].alpha_day != self.day))
            }
            Channel::Ghosts => {
                !self.alive[you] || (self.roles[you] == Role::Medium && self.phase == Phase::Night)
            }
        };
        if !allowed {
            return Err("You cannot speak in that channel now");
        }
        let text = clean_text(text, 160);
        if text.is_empty() {
            return Err("Write a message first");
        }
        if self.last_chat[you] != 0 && now.saturating_sub(self.last_chat[you]) < 1 {
            return Err("Give the village a moment between messages");
        }
        self.last_chat[you] = now;
        if channel == Channel::Pack && !matches!(self.phase, Phase::Night | Phase::Finished) {
            self.power[you].alpha_day = self.day;
        }
        self.chat.push(Chat {
            who: if channel == Channel::Ghosts && self.alive[you] {
                None
            } else {
                Some(you)
            },
            text,
            channel,
            day: self.day,
        });
        if self.chat.len() > 120 {
            self.chat.remove(0);
        }
        Ok(())
    }
    fn all_ready(&self) -> bool {
        self.alive
            .iter()
            .enumerate()
            .all(|(i, a)| !*a || self.ready[i])
    }
    pub fn tick(&mut self, now: u64) -> bool {
        if self.finished() || now < self.deadline {
            return false;
        }
        self.advance(now);
        true
    }
    fn phase(&mut self, phase: Phase, now: u64) {
        self.phase = phase;
        self.deadline = now.saturating_add(phase.seconds());
        self.ready.fill(false);
    }
    fn advance(&mut self, now: u64) {
        match self.phase {
            Phase::Night => {
                self.resolve_night();
                if !self.finished() {
                    self.phase(Phase::Dawn, now);
                }
            }
            Phase::Dawn => self.phase(Phase::Discussion, now),
            Phase::Discussion => self.phase(Phase::Vote, now),
            Phase::Vote => {
                let votes: Vec<_> = self
                    .votes
                    .iter()
                    .enumerate()
                    .map(|(i, v)| v.filter(|&t| self.alive[i] && self.alive[t]))
                    .collect();
                self.ballots = Some(votes.clone());
                let alive = self.alive.iter().filter(|a| **a).count();
                if let Some(target) = plurality(&votes, self.roles.len())
                    .filter(|&t| votes.iter().filter(|v| **v == Some(t)).count() > alive / 2)
                {
                    self.event(format!("The village voted out {}.", self.names[target]));
                    self.eliminate(target);
                    if self.roles[target] == Role::Fool {
                        self.finish(Team::Fool, Some(target));
                    }
                } else {
                    self.event("No majority. Nobody was voted out.".into());
                }
                for i in 0..self.roles.len() {
                    if self.alive[i] && self.power[i].injured == Some(self.day) {
                        self.event(format!("{} succumbed to their injuries.", self.names[i]));
                        self.eliminate(i);
                    }
                }
                if !self.finished() {
                    self.check_winner();
                }
                if !self.finished() {
                    if self.day >= 40 {
                        self.finish(Team::Draw, None);
                    } else {
                        self.day += 1;
                        self.targets.fill(None);
                        self.kills.fill(None);
                        self.votes.fill(None);
                        self.phase(Phase::Night, now);
                    }
                }
            }
            Phase::Finished => {}
        }
    }
    fn resolve_night(&mut self) {
        // Snapshot all living actors: death cannot cancel an already locked action.
        let actors = self.alive.clone();
        for (i, &alive) in actors.iter().enumerate() {
            if alive
                && matches!(self.roles[i], Role::Seer | Role::WolfSeer | Role::AuraSeer)
                && let Some(t) = self.targets[i]
            {
                self.findings[i].push(Finding {
                    day: self.day,
                    target: t,
                    role: (self.roles[i] != Role::AuraSeer).then_some(self.roles[t]),
                    aura: (self.roles[i] == Role::AuraSeer).then_some(if self.roles[t].wolf() {
                        Aura::Wolf
                    } else if self.roles[t].village() {
                        Aura::Village
                    } else {
                        Aura::Unknown
                    }),
                });
            }
        }
        let doctor = self
            .roles
            .iter()
            .position(|r| *r == Role::Doctor)
            .filter(|&i| actors[i]);
        let protected = doctor.and_then(|i| self.targets[i]);
        let witch = self
            .roles
            .iter()
            .position(|r| *r == Role::Witch)
            .filter(|&i| actors[i]);
        let healed = witch
            .filter(|&i| self.power[i].heal)
            .and_then(|i| self.targets[i]);
        let guard = self
            .roles
            .iter()
            .position(|r| *r == Role::Bodyguard)
            .filter(|&i| actors[i]);
        let guarded = guard.and_then(|i| self.targets[i]);
        let tough = self
            .roles
            .iter()
            .position(|r| *r == Role::ToughGuy)
            .filter(|&i| actors[i]);
        let guarded_tough = tough.and_then(|i| self.targets[i]);
        let mut pack = vec![];
        for (i, &alive) in actors.iter().enumerate() {
            if alive && self.roles[i].wolf() {
                pack.push(self.kills[i]);
                if self.roles[i] == Role::AlphaWolf {
                    pack.push(self.kills[i]);
                }
            }
        }
        let wolf_target = plurality(&pack, self.roles.len());
        let wolf_attacker = self
            .roles
            .iter()
            .enumerate()
            .find(|(i, r)| actors[*i] && r.wolf())
            .map(|(i, _)| i);
        let killer = self
            .roles
            .iter()
            .position(|r| *r == Role::SerialKiller)
            .filter(|&i| actors[i]);
        let poisoned = witch
            .filter(|&i| self.day > 1 && self.power[i].poison)
            .and_then(|i| self.kills[i]);
        if poisoned.is_some()
            && let Some(i) = witch
        {
            self.power[i].poison = false;
        }
        let mut deaths = vec![];
        for (target, attacker, wolf_attack, poison) in [
            (wolf_target, wolf_attacker, true, false),
            (killer.and_then(|i| self.targets[i]), killer, false, false),
            (poisoned, witch, false, true),
        ] {
            let Some(target) = target else {
                continue;
            };
            if !actors[target] {
                continue;
            }
            if wolf_attack && self.roles[target] == Role::SerialKiller {
                continue;
            }
            if !poison && (protected == Some(target) || healed == Some(target)) {
                if healed == Some(target)
                    && let Some(i) = witch
                {
                    self.power[i].heal = false;
                }
                continue;
            }
            if !poison
                && (guarded_tough == Some(target) || tough == Some(target))
                && let Some(i) = tough
            {
                self.power[i].injured.get_or_insert(self.day);
                if let Some(a) = attacker {
                    self.findings[i].push(Finding {
                        day: self.day,
                        target: a,
                        role: Some(self.roles[a]),
                        aura: None,
                    });
                    self.findings[a].push(Finding {
                        day: self.day,
                        target: i,
                        role: Some(self.roles[i]),
                        aura: None,
                    });
                }
                continue;
            }
            let victim = if !poison && (guarded == Some(target) || guard == Some(target)) {
                guard.unwrap()
            } else {
                target
            };
            if victim != target && !poison && (protected == Some(victim) || healed == Some(victim))
            {
                if healed == Some(victim)
                    && let Some(i) = witch
                {
                    self.power[i].heal = false;
                }
                continue;
            }
            if !poison && self.roles[victim] == Role::Bodyguard {
                self.power[victim].hits += 1;
                if self.power[victim].hits < 2 {
                    continue;
                }
            }
            if !deaths.contains(&victim) {
                deaths.push(victim);
            }
        }
        if deaths.is_empty() {
            self.event("Dawn breaks. Everyone survived the night.".into());
        }
        for target in deaths {
            if self.alive[target] {
                self.event(format!("{} was lost during the night.", self.names[target]));
                self.eliminate(target);
            }
        }
        for (i, &alive) in actors.iter().enumerate() {
            if alive
                && self.roles[i] == Role::Medium
                && self.power[i].revive
                && let Some(t) = self.targets[i]
                && !actors[t]
                && self.roles[t].village()
            {
                self.alive[t] = true;
                self.revealed[t] = true;
                self.ready[t] = false;
                self.power[t].injured = None;
                self.marks[t] = None;
                self.power[i].revive = false;
                self.event(format!("{} returned to the village.", self.names[t]));
            }
        }
        for findings in &mut self.findings {
            if findings.len() > 40 {
                findings.drain(..findings.len() - 40);
            }
        }
        self.check_winner();
    }
    fn eliminate(&mut self, target: usize) {
        if !self.alive[target] {
            return;
        }
        self.alive[target] = false;
        self.ready[target] = false;
        if matches!(self.roles[target], Role::Avenger | Role::JuniorWolf)
            && let Some(mark) = self.marks[target].take()
            && self.alive[mark]
        {
            self.event(format!(
                "{} took {} with them.",
                self.names[target], self.names[mark]
            ));
            self.eliminate(mark);
        }
    }
    fn event(&mut self, text: String) {
        self.events.push(text);
        if self.events.len() > 16 {
            self.events.remove(0);
        }
    }
    fn finish(&mut self, team: Team, seat: Option<usize>) {
        self.winner = Some(team);
        self.winner_seat = seat;
        self.phase = Phase::Finished;
        self.deadline = 0;
        self.event(team.title().into());
    }
    fn check_winner(&mut self) {
        let living = self.alive.iter().filter(|a| **a).count();
        let wolves = self
            .roles
            .iter()
            .enumerate()
            .filter(|(i, r)| self.alive[*i] && r.wolf())
            .count();
        let killer = self
            .roles
            .iter()
            .enumerate()
            .find(|(i, r)| self.alive[*i] && **r == Role::SerialKiller)
            .map(|(i, _)| i);
        if wolves == 1
            && let Some(i) = self
                .roles
                .iter()
                .enumerate()
                .find(|(i, r)| self.alive[*i] && **r == Role::WolfSeer)
                .map(|(i, _)| i)
        {
            self.roles[i] = Role::Werewolf;
        }
        if living == 0 {
            self.finish(Team::Draw, None);
        } else if living == 1 && killer.is_some() {
            self.finish(Team::SerialKiller, killer);
        } else if wolves == 0 && killer.is_none() {
            self.finish(Team::Village, None);
        } else if killer.is_none() && wolves >= living - wolves {
            self.finish(Team::Wolves, None);
        }
    }
    pub fn forfeit(&mut self, you: usize, now: u64) {
        if you >= self.roles.len() || !self.alive[you] || self.finished() {
            return;
        }
        self.event(format!("{} left the village.", self.names[you]));
        self.eliminate(you);
        self.targets[you] = None;
        self.kills[you] = None;
        self.votes[you] = None;
        self.check_winner();
        if !self.finished() && self.all_ready() {
            self.advance(now);
        }
    }
}
fn plurality(votes: &[Option<usize>], n: usize) -> Option<usize> {
    let mut counts = vec![0; n];
    for &v in votes.iter().flatten() {
        if v < n {
            counts[v] += 1;
        }
    }
    let best = *counts.iter().max()?;
    if best == 0 {
        return None;
    }
    let mut leaders = counts
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == best)
        .map(|(i, _)| i);
    let first = leaders.next()?;
    leaders.next().is_none().then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(roles: &[Role]) -> Game {
        let mut g = Game::new((0..roles.len()).map(|i| format!("P{i}")).collect(), 1, 100).unwrap();
        g.roles = roles.to_vec();
        g
    }
    const BASE: [Role; 6] = [
        Role::Werewolf,
        Role::Werewolf,
        Role::Seer,
        Role::Doctor,
        Role::Villager,
        Role::Villager,
    ];
    fn night(g: &mut Game, choices: &[(usize, Option<usize>, Option<usize>)]) {
        for &(i, t, k) in choices {
            g.play(i, Move::Night { target: t, kill: k }, 101).unwrap();
        }
        for i in 0..g.roles.len() {
            if g.phase == Phase::Night && g.alive[i] && !g.ready[i] {
                g.play(
                    i,
                    Move::Night {
                        target: None,
                        kill: None,
                    },
                    101,
                )
                .unwrap();
            }
        }
    }
    fn vote(g: &mut Game, target: Option<usize>) {
        g.phase(Phase::Vote, 100);
        for i in 0..g.roles.len() {
            if g.alive[i] && !g.finished() {
                g.play(
                    i,
                    Move::Vote {
                        target: target.filter(|t| *t != i),
                    },
                    101,
                )
                .unwrap();
            }
        }
    }
    #[test]
    fn presets_and_custom_setups_validate_all_room_sizes() {
        for n in 6..=16 {
            for preset in [Preset::Classic, Preset::Advanced] {
                let roles = Setup {
                    preset,
                    roles: vec![],
                }
                .roles_for(n)
                .unwrap();
                assert_eq!(roles.len(), n);
                assert!(roles.iter().filter(|r| r.wolf()).count() * 2 < n);
            }
        }
        assert!(Setup::default().roles_for(5).is_err());
        assert!(Setup::default().roles_for(17).is_err());
        let mut custom = Setup {
            preset: Preset::Custom,
            roles: BASE.to_vec(),
        };
        assert!(custom.roles_for(6).is_ok());
        assert!(custom.roles_for(7).is_err());
        custom.roles.fill(Role::Villager);
        assert!(custom.roles_for(6).is_err());
        custom.roles[..3].fill(Role::Werewolf);
        assert!(custom.roles_for(6).is_err());
        custom.roles = BASE.to_vec();
        custom.roles[5] = Role::Seer;
        assert!(custom.roles_for(6).is_err());
    }
    #[test]
    fn projections_keep_roles_actions_votes_and_channels_private() {
        let mut g = game(&BASE);
        g.play(
            0,
            Move::Chat {
                channel: Channel::Pack,
                text: "P4 tonight".into(),
            },
            101,
        )
        .unwrap();
        g.play(
            0,
            Move::Night {
                target: None,
                kill: Some(4),
            },
            102,
        )
        .unwrap();
        let village = g.view(4);
        assert!(village.chat.is_empty());
        assert!(village.pack_votes.is_none());
        assert!(
            village
                .players
                .iter()
                .enumerate()
                .all(|(i, p)| p.role.is_some() == (i == 4))
        );
        assert_eq!(g.view(1).players[0].role, Some(Role::Werewolf));
        assert_eq!(g.view(1).pack_votes.unwrap()[0], (0, Some(4)));
        g.phase(Phase::Vote, 103);
        g.play(4, Move::Vote { target: Some(0) }, 104).unwrap();
        assert!(g.view(0).ballots.is_none());
        let wire = serde_json::to_value(g.view(4)).unwrap();
        assert!(wire.get("kills").is_none());
        assert!(wire.get("votes").is_none());
        assert!(wire.get("roles").is_none());
    }
    #[test]
    fn illegal_targets_deadline_and_double_actions_are_rejected() {
        let mut g = game(&BASE);
        for movement in [
            Move::Night {
                target: Some(4),
                kill: None,
            },
            Move::Night {
                target: None,
                kill: Some(0),
            },
            Move::Vote { target: Some(0) },
        ] {
            assert!(g.play(4, movement, 101).is_err());
        }
        assert!(
            g.play(
                0,
                Move::Night {
                    target: None,
                    kill: Some(1)
                },
                101
            )
            .is_err()
        );
        assert!(
            g.play(
                2,
                Move::Night {
                    target: Some(99),
                    kill: None
                },
                101
            )
            .is_err()
        );
        assert!(
            g.play(
                2,
                Move::Night {
                    target: Some(2),
                    kill: None
                },
                101
            )
            .is_err()
        );
        assert!(
            g.play(
                2,
                Move::Night {
                    target: Some(0),
                    kill: None
                },
                145
            )
            .is_err()
        );
        g.play(
            2,
            Move::Night {
                target: Some(0),
                kill: None,
            },
            101,
        )
        .unwrap();
        assert!(
            g.play(
                2,
                Move::Night {
                    target: Some(1),
                    kill: None
                },
                101
            )
            .is_err()
        );
        g.alive[4] = false;
        assert!(g.play(4, Move::Ready, 101).is_err());
    }
    #[test]
    fn doctor_blocks_attacks_and_findings_belong_only_to_inspector() {
        let mut g = game(&BASE);
        night(
            &mut g,
            &[
                (0, None, Some(4)),
                (1, None, Some(4)),
                (2, Some(0), None),
                (3, Some(4), None),
            ],
        );
        assert!(g.alive[4]);
        assert_eq!(g.phase, Phase::Dawn);
        assert_eq!(g.view(2).findings[0].role, Some(Role::Werewolf));
        assert!(g.view(4).findings.is_empty());
    }
    #[test]
    fn locked_night_abilities_resolve_even_when_actor_is_killed() {
        let mut g = game(&BASE);
        night(
            &mut g,
            &[(0, None, Some(2)), (1, None, Some(2)), (2, Some(0), None)],
        );
        assert!(!g.alive[2]);
        assert_eq!(g.findings[2][0].role, Some(Role::Werewolf));
        assert_eq!(g.view(4).players[2].role, Some(Role::Seer));
    }
    #[test]
    fn alpha_double_vote_breaks_ties_but_equal_pack_votes_spare_targets() {
        let mut g = game(&BASE);
        night(&mut g, &[(0, None, Some(4)), (1, None, Some(5))]);
        assert!(g.alive[4] && g.alive[5]);
        let mut g = game(&[
            Role::AlphaWolf,
            Role::Werewolf,
            Role::Seer,
            Role::Doctor,
            Role::Villager,
            Role::Villager,
        ]);
        night(&mut g, &[(0, None, Some(4)), (1, None, Some(5))]);
        assert!(!g.alive[4] && g.alive[5]);
    }
    #[test]
    fn bodyguard_redirects_and_survives_one_hit() {
        let mut g = game(&[
            Role::Werewolf,
            Role::Werewolf,
            Role::Seer,
            Role::Bodyguard,
            Role::Villager,
            Role::Villager,
        ]);
        night(
            &mut g,
            &[(0, None, Some(4)), (1, None, Some(4)), (3, Some(4), None)],
        );
        assert!(g.alive[3] && g.alive[4]);
        assert_eq!(g.power[3].hits, 1);
        g.day = 2;
        g.phase(Phase::Night, 100);
        night(
            &mut g,
            &[(0, None, Some(4)), (1, None, Some(4)), (3, Some(4), None)],
        );
        assert!(!g.alive[3] && g.alive[4]);
    }
    #[test]
    fn witch_potions_only_spend_on_use_and_poison_bypasses_protection() {
        let mut g = game(&[
            Role::Werewolf,
            Role::Werewolf,
            Role::Witch,
            Role::Doctor,
            Role::Villager,
            Role::Villager,
        ]);
        assert!(
            g.play(
                2,
                Move::Night {
                    target: None,
                    kill: Some(4)
                },
                101
            )
            .is_err()
        );
        night(&mut g, &[(2, Some(4), None)]);
        assert!(g.power[2].heal && g.power[2].poison);
        g.day = 2;
        g.phase(Phase::Night, 100);
        night(
            &mut g,
            &[
                (0, None, Some(4)),
                (1, None, Some(4)),
                (2, Some(4), Some(5)),
                (3, Some(5), None),
            ],
        );
        assert!(g.alive[4]);
        assert!(!g.alive[5]);
        assert!(!g.power[2].heal && !g.power[2].poison);
    }
    #[test]
    fn aura_seer_reads_teams_without_exact_roles() {
        let mut g = game(&[
            Role::Werewolf,
            Role::Werewolf,
            Role::AuraSeer,
            Role::Doctor,
            Role::Fool,
            Role::Villager,
        ]);
        night(&mut g, &[(2, Some(4), None)]);
        assert_eq!(g.findings[2][0].aura, Some(Aura::Unknown));
        assert!(g.findings[2][0].role.is_none());
    }
    #[test]
    fn medium_chat_is_anonymous_and_revives_a_villager_only_once() {
        let mut g = game(&[
            Role::Werewolf,
            Role::Werewolf,
            Role::Medium,
            Role::Doctor,
            Role::Fool,
            Role::Villager,
        ]);
        g.alive[5] = false;
        g.alive[4] = false;
        assert_eq!(g.view(2).targets, vec![5]);
        g.play(
            2,
            Move::Chat {
                channel: Channel::Ghosts,
                text: "Come back".into(),
            },
            101,
        )
        .unwrap();
        assert_eq!(g.view(5).chat[0].who, None);
        assert!(g.view(0).chat.is_empty());
        night(
            &mut g,
            &[(0, None, Some(2)), (1, None, Some(2)), (2, Some(5), None)],
        );
        assert!(!g.alive[2] && g.alive[5]);
        assert!(!g.power[2].revive);
        assert_eq!(g.view(3).players[5].role, Some(Role::Villager));
    }
    #[test]
    fn tough_guy_exposes_attacker_privately_and_dies_at_day_end() {
        let mut g = game(&[
            Role::Werewolf,
            Role::Werewolf,
            Role::ToughGuy,
            Role::Doctor,
            Role::Villager,
            Role::Villager,
        ]);
        night(
            &mut g,
            &[(0, None, Some(4)), (1, None, Some(4)), (2, Some(4), None)],
        );
        assert!(g.alive[2] && g.alive[4]);
        assert_eq!(g.power[2].injured, Some(1));
        assert_eq!(g.view(2).findings[0].role, Some(Role::Werewolf));
        assert!(g.view(4).findings.is_empty());
        vote(&mut g, None);
        assert!(!g.alive[2]);
    }
    #[test]
    fn revenge_chains_terminate_and_avenger_waits_until_after_first_night() {
        let mut g = game(&[
            Role::Werewolf,
            Role::JuniorWolf,
            Role::Avenger,
            Role::Doctor,
            Role::Villager,
            Role::Villager,
        ]);
        assert!(g.play(2, Move::Mark { target: 1 }, 101).is_err());
        g.play(1, Move::Mark { target: 2 }, 101).unwrap();
        g.phase(Phase::Dawn, 100);
        g.play(2, Move::Mark { target: 1 }, 101).unwrap();
        g.eliminate(1);
        assert!(!g.alive[1] && !g.alive[2]);
    }
    #[test]
    fn gunner_has_two_shots_one_per_day_and_reveals_their_role() {
        let mut g = game(&[
            Role::Werewolf,
            Role::Werewolf,
            Role::Gunner,
            Role::Doctor,
            Role::Fool,
            Role::Villager,
        ]);
        g.phase(Phase::Discussion, 100);
        assert!(g.play(2, Move::Shoot { target: 4 }, 101).is_err());
        g.phase(Phase::Vote, 100);
        g.play(2, Move::Shoot { target: 4 }, 101).unwrap();
        assert_eq!(g.view(5).players[2].role, Some(Role::Gunner));
        assert!(!g.finished());
        assert!(g.play(2, Move::Shoot { target: 5 }, 101).is_err());
        g.day = 2;
        g.phase(Phase::Discussion, 100);
        g.play(2, Move::Shoot { target: 0 }, 101).unwrap();
        assert_eq!(g.bullets[2], 0);
        g.day = 3;
        assert!(g.play(2, Move::Shoot { target: 1 }, 101).is_err());
    }
    #[test]
    fn voting_requires_strict_majority_and_fool_wins_only_when_executed() {
        let mut g = game(&[
            Role::Werewolf,
            Role::Werewolf,
            Role::Seer,
            Role::Doctor,
            Role::Fool,
            Role::Villager,
        ]);
        vote(&mut g, Some(4));
        assert_eq!(g.winner, Some(Team::Fool));
        let mut g = game(&BASE);
        g.phase(Phase::Vote, 100);
        for i in 0..6 {
            g.play(
                i,
                Move::Vote {
                    target: if i < 3 { Some(4) } else { Some(0) },
                },
                101,
            )
            .unwrap();
        }
        assert!(g.alive.iter().all(|a| *a));
        assert_eq!(g.day, 2);
    }
    #[test]
    fn dead_or_departed_ballots_cannot_execute_a_dead_fool() {
        let mut g = game(&[
            Role::Werewolf,
            Role::Werewolf,
            Role::Seer,
            Role::Doctor,
            Role::Fool,
            Role::Villager,
        ]);
        g.phase(Phase::Vote, 100);
        g.votes = vec![Some(4); 6];
        g.alive[4] = false;
        g.advance(101);
        assert_ne!(g.winner, Some(Team::Fool));
        assert!(g.ballots.unwrap().iter().all(Option::is_none));
    }
    #[test]
    fn last_wolf_seer_becomes_a_wolf_and_killer_blocks_parity_win() {
        let mut g = game(&[
            Role::WolfSeer,
            Role::Werewolf,
            Role::SerialKiller,
            Role::Doctor,
            Role::Villager,
            Role::Villager,
        ]);
        g.alive[1] = false;
        g.check_winner();
        assert_eq!(g.roles[0], Role::Werewolf);
        g.alive[3..].fill(false);
        g.check_winner();
        assert!(!g.finished());
        g.phase(Phase::Night, 100);
        night(&mut g, &[(0, None, Some(2)), (2, Some(0), None)]);
        assert!(g.alive[2]);
        assert_eq!(g.winner, Some(Team::SerialKiller));
    }
    #[test]
    fn all_terminal_teams_and_forty_day_limit_resolve() {
        for (living, result) in [
            (vec![2, 3, 4], Team::Village),
            (vec![0, 1, 4, 5], Team::Wolves),
            (vec![], Team::Draw),
        ] {
            let mut g = game(&BASE);
            g.alive.fill(false);
            for i in living {
                g.alive[i] = true;
            }
            g.check_winner();
            assert_eq!(g.winner, Some(result));
        }
        let mut g = game(&BASE);
        g.day = 40;
        vote(&mut g, None);
        assert_eq!(g.winner, Some(Team::Draw));
    }
    #[test]
    fn timers_do_not_catch_up_multiple_phases_after_offline_restore() {
        let mut g = game(&BASE);
        assert!(!g.tick(144));
        assert!(g.tick(10000));
        assert_eq!(g.phase, Phase::Dawn);
        assert_eq!(g.deadline, 10012);
        assert!(!g.tick(10000));
        let restored: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(g.view(0)).unwrap(),
            serde_json::to_value(restored.view(0)).unwrap()
        );
    }
    #[test]
    fn alpha_day_chat_is_one_private_message_and_village_sleeps_at_night() {
        let mut g = game(&[
            Role::AlphaWolf,
            Role::Werewolf,
            Role::Seer,
            Role::Doctor,
            Role::Villager,
            Role::Villager,
        ]);
        assert!(
            g.play(
                4,
                Move::Chat {
                    channel: Channel::Village,
                    text: "hello".into()
                },
                101
            )
            .is_err()
        );
        g.phase(Phase::Discussion, 100);
        g.play(
            0,
            Move::Chat {
                channel: Channel::Pack,
                text: "stay quiet".into(),
            },
            101,
        )
        .unwrap();
        assert!(
            g.play(
                0,
                Move::Chat {
                    channel: Channel::Pack,
                    text: "again".into()
                },
                102
            )
            .is_err()
        );
        assert!(
            g.play(
                1,
                Move::Chat {
                    channel: Channel::Pack,
                    text: "hi".into()
                },
                102
            )
            .is_err()
        );
        assert!(g.view(4).chat.is_empty());
    }
    #[test]
    fn simultaneous_night_deaths_resolve_before_the_winner_is_decided() {
        let mut g = game(&[
            Role::Werewolf,
            Role::SerialKiller,
            Role::Witch,
            Role::Villager,
            Role::Villager,
            Role::Villager,
        ]);
        g.alive[3..].fill(false);
        g.day = 2;
        night(
            &mut g,
            &[(0, None, Some(2)), (1, Some(0), None), (2, None, Some(1))],
        );
        assert!(g.alive.iter().all(|a| !*a));
        assert_eq!(g.winner, Some(Team::Draw));
    }
    #[test]
    fn witch_can_shield_herself_without_enabling_self_poison() {
        let mut g = game(&[
            Role::Werewolf,
            Role::Werewolf,
            Role::Witch,
            Role::Doctor,
            Role::Villager,
            Role::Villager,
        ]);
        assert!(g.view(2).targets.contains(&2));
        night(
            &mut g,
            &[(0, None, Some(2)), (1, None, Some(2)), (2, Some(2), None)],
        );
        assert!(g.alive[2]);
        assert!(!g.power[2].heal);
        g.day = 2;
        g.phase(Phase::Night, 100);
        assert!(
            g.play(
                2,
                Move::Night {
                    target: None,
                    kill: Some(2)
                },
                101
            )
            .is_err()
        );
    }
    #[test]
    fn seeded_games_finish_and_serialized_views_preserve_privacy() {
        for n in 6..=16 {
            for seed in 0..6 {
                let mut g = Game::with_setup(
                    (0..n).map(|i| format!("P{i}")).collect(),
                    seed,
                    100,
                    &Setup::default(),
                )
                .unwrap();
                for _ in 0..200 {
                    if g.finished() {
                        break;
                    }
                    if g.phase == Phase::Night {
                        for i in 0..n {
                            let v = g.view(i);
                            if g.phase != Phase::Night || !g.alive[i] || v.locked {
                                continue;
                            }
                            let target = v.targets.first().copied();
                            let kill = v.victims.first().copied();
                            g.play(i, Move::Night { target, kill }, g.deadline - 1)
                                .unwrap();
                        }
                    } else if g.phase == Phase::Vote {
                        let suspect = (0..n).find(|&i| g.alive[i] && g.roles[i].wolf());
                        for i in 0..n {
                            if g.alive[i] && !g.finished() {
                                g.play(
                                    i,
                                    Move::Vote {
                                        target: suspect.filter(|t| *t != i),
                                    },
                                    g.deadline - 1,
                                )
                                .unwrap();
                            }
                        }
                    } else {
                        g.tick(g.deadline);
                    }
                    let restored: Game =
                        serde_json::from_slice(&serde_json::to_vec(&g).unwrap()).unwrap();
                    for i in 0..n {
                        assert_eq!(
                            serde_json::to_value(g.view(i)).unwrap(),
                            serde_json::to_value(restored.view(i)).unwrap()
                        );
                    }
                }
                assert!(g.finished(), "seed {seed}, players {n}");
            }
        }
    }
}
