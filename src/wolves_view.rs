//! Village UI. Selections remain local until explicitly confirmed.
use crate::{
    online_style::primary,
    online_view::{button, clip, fit, page_input, wrap, wrapped_lines},
    ui::{Ui, bordered},
    wolves_art,
};
use jarcade::{
    board_pan::BoardPan,
    multiplayer::{
        Command, RoomView,
        wolves::{self, Channel, Move, Phase, Preset, Role, Setup, View},
    },
};
use macroquad::prelude::*;

#[derive(Clone, Copy, Default, PartialEq)]
enum Tab {
    #[default]
    Village,
    Chat,
    Role,
}
#[derive(Clone, Copy, Default, PartialEq)]
enum Mode {
    #[default]
    Ability,
    Hunt,
    Shoot,
    Mark,
}
#[derive(Default)]
pub struct WolvesUi {
    tab: Tab,
    mode: Mode,
    target: Option<usize>,
    kill: Option<usize>,
    mark: Option<usize>,
    channel: Option<Channel>,
    pan: BoardPan,
    blocked: bool,
    pub setup: Option<Setup>,
}
#[derive(Default)]
pub struct Outcome {
    pub command: Option<Command>,
    pub editor: Option<Rect>,
    pub clear_chat: bool,
}
impl Outcome {
    fn movement(&mut self, movement: Move) {
        self.command = Some(Command::Wolves(movement));
    }
}
impl WolvesUi {
    pub fn phase_changed(&mut self) {
        self.target = None;
        self.kill = None;
        self.mark = None;
        self.mode = Mode::Ability;
        self.pan = BoardPan::default();
    }
    pub fn active(&self) -> bool {
        self.pan.active()
    }
    pub fn close_setup(&mut self) -> bool {
        self.setup.take().is_some()
    }
    pub fn open_setup(&mut self, room: &RoomView) {
        self.setup = Some(room.wolves_setup.clone().unwrap_or_default());
        self.pan = BoardPan::default();
    }
    pub fn announcement(&self) -> String {
        if let Some(s) = &self.setup {
            format!(
                "Role setup. {:?}. {} selected roles. Classic, Advanced, Custom. Apply setup.",
                s.preset,
                s.roles.len()
            )
        } else {
            format!(
                "{} tab. {}",
                match self.tab {
                    Tab::Village => "Village",
                    Tab::Chat => "Chat",
                    Tab::Role => "Role",
                },
                if self.target.is_some() || self.kill.is_some() || self.mark.is_some() {
                    "Player selected. Confirm your choice."
                } else {
                    ""
                }
            )
        }
    }
    pub fn draw_setup(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        room: &RoomView,
        x: f32,
        w: f32,
    ) -> Outcome {
        let mut out = Outcome::default();
        let host = room.host == room.you;
        let n = room.members.len().max(6);
        let setup = self.setup.as_mut().unwrap();
        if setup.preset != Preset::Custom {
            setup.roles = setup.roles_for(n).unwrap_or_default();
        }
        fit(
            ui,
            if host {
                "Build your village"
            } else {
                "Village roles"
            },
            Rect::new(x, 65., w, 32.),
            25.,
            ui.theme.text,
            true,
        );
        let gap = 8.;
        let cw = (w - gap * 2.) / 3.;
        for (i, (preset, label)) in [
            (Preset::Classic, "Classic"),
            (Preset::Advanced, "Advanced"),
            (Preset::Custom, "Custom"),
        ]
        .iter()
        .enumerate()
        {
            let r = Rect::new(x + i as f32 * (cw + gap), 106., cw, 44.);
            if pill(ui, label, r, setup.preset == *preset) && host {
                setup.preset = *preset;
                self.pan = BoardPan::default();
            }
        }
        let count = setup.roles.len();
        let wolves = setup.roles.iter().filter(|r| r.wolf()).count();
        let summary = format!(
            "{count} roles  ·  {wolves} wolves  ·  {} players here",
            room.members.len()
        );
        fit(
            ui,
            &summary,
            Rect::new(x, 157., w, 28.),
            13.,
            ui.theme.muted,
            false,
        );
        let row_h = 116.;
        let cols = if w >= 700. { 2 } else { 1 };
        let rw = (w - (cols - 1) as f32 * 12.) / cols as f32;
        let viewport = Rect::new(x, 194., w, (screen_height() - 276.).max(40.));
        let (origin, tap) = page_input(
            &mut self.pan,
            &mut self.blocked,
            press,
            viewport,
            wolves::ROLES.len().div_ceil(cols) as f32 * row_h,
        );
        let old = ui.override_pointer(tap);
        clip(Some(viewport));
        for (i, role) in wolves::ROLES.iter().enumerate() {
            let r = Rect::new(
                x + (i % cols) as f32 * (rw + 12.),
                origin.y + (i / cols) as f32 * row_h,
                rw,
                row_h - 10.,
            );
            bordered(r, 18., ui.theme.line, ui.theme.bg);
            wolves_art::portrait(
                vec2(r.x + 29., r.y + 28.),
                20.,
                i,
                Some(*role),
                ui.theme.saver,
            );
            fit(
                ui,
                role.title(),
                Rect::new(
                    r.x + 59.,
                    r.y + 7.,
                    r.w - if host && setup.preset == Preset::Custom {
                        220.
                    } else {
                        125.
                    },
                    25.,
                ),
                15.,
                ui.theme.text,
                true,
            );
            wrap(
                ui,
                role.description(),
                Rect::new(r.x + 12., r.y + 55., r.w - 24., 44.),
                11.,
                ui.theme.muted,
                false,
            );
            let num = setup.roles.iter().filter(|v| **v == *role).count();
            if host && setup.preset == Preset::Custom {
                if button(
                    ui,
                    "−",
                    Rect::new(r.right() - 150., r.y + 5., 44., 44.),
                    false,
                    tap,
                    viewport,
                ) && let Some(at) = setup.roles.iter().position(|v| v == role)
                {
                    setup.roles.remove(at);
                }
                fit(
                    ui,
                    &num.to_string(),
                    Rect::new(r.right() - 104., r.y + 5., 40., 44.),
                    17.,
                    ui.theme.accent,
                    true,
                );
                if button(
                    ui,
                    "+",
                    Rect::new(r.right() - 56., r.y + 5., 44., 44.),
                    false,
                    tap,
                    viewport,
                ) && count < 16
                    && (num == 0 || matches!(role, Role::Villager | Role::Werewolf))
                {
                    setup.roles.push(*role);
                }
            } else {
                fit(
                    ui,
                    &format!("×{num}"),
                    Rect::new(r.right() - 58., r.y + 5., 46., 44.),
                    17.,
                    ui.theme.accent,
                    true,
                );
            }
        }
        clip(None);
        ui.override_pointer(old);
        let validation = if setup.preset == Preset::Custom {
            setup.roles_for(setup.roles.len())
        } else {
            setup.roles_for(n)
        };
        let bottom = screen_height() - 68.;
        let error = validation
            .as_ref()
            .err()
            .copied()
            .unwrap_or("One role per player. Special roles are unique.");
        fit(
            ui,
            error,
            Rect::new(x, bottom - 20., w, 18.),
            10.,
            ui.theme.muted,
            false,
        );
        let mut close = false;
        if ui.button("Back", Rect::new(x, bottom, 78., 48.), false) {
            close = true;
        }
        if primary(
            ui,
            if host { "Apply setup" } else { "Done" },
            Rect::new(x + 90., bottom, w - 90., 48.),
            !host || validation.is_ok(),
        ) {
            if host {
                out.command = Some(Command::WolvesSetup(setup.clone()));
            }
            close = true;
        }
        if close {
            self.setup = None;
            self.pan = BoardPan::default();
        }
        out
    }
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        room: &RoomView,
        g: &View,
        chat: (&str, bool),
        area: Rect,
    ) -> Outcome {
        let mut out = Outcome::default();
        let w = area.w.min(850.);
        let x = area.x + (area.w - w) / 2.;
        let compact = screen_height() < 600.;
        let sideways = compact && w >= 500.;
        let hw = if sideways { w * 0.4 } else { w };
        let hy = if compact { 60. } else { 66. };
        let body = Rect::new(
            x,
            if sideways {
                116.
            } else if compact {
                172.
            } else {
                230.
            },
            w,
            screen_height(),
        );
        if self.target.is_some_and(|i| {
            if g.phase == Phase::Vote && self.mode != Mode::Shoot {
                !g.players[i].alive
            } else {
                !g.targets.contains(&i)
            }
        }) {
            self.target = None;
        }
        if self.kill.is_some_and(|i| !g.victims.contains(&i)) {
            self.kill = None;
        }
        if self.mark.is_some_and(|i| !g.players[i].alive) {
            self.mark = None;
        }
        let night = g.phase == Phase::Night;
        let header = Rect::new(x, hy, hw, if compact { 48. } else { 96. });
        bordered(header, 22., ui.theme.line, ui.theme.panel);
        if night {
            wolves_art::moon(
                vec2(x + 28., hy + 28.),
                12.,
                ui.theme.accent,
                ui.theme.panel,
            );
        } else {
            draw_circle(x + 28., hy + 28., 9., ui.theme.accent);
        }
        let title = g.winner.map_or_else(
            || format!("{} {}", g.phase.title(), g.day),
            |t| t.title().into(),
        );
        fit(
            ui,
            &title,
            Rect::new(x + 51., hy + 9., hw - 132., 32.),
            23.,
            ui.theme.text,
            true,
        );
        if !g.phase.eq(&Phase::Finished) {
            let left = g.deadline.saturating_sub(unix());
            fit(
                ui,
                &format!("{left}s"),
                Rect::new(x + hw - 69., hy + 11., 57., 30.),
                19.,
                ui.theme.accent,
                true,
            );
        }
        let sub = if g.phase == Phase::Finished {
            "Every role is revealed. Ready for another village?".into()
        } else {
            format!(
                "{} alive  ·  {}/{} locked in",
                g.alive, g.submitted, g.alive
            )
        };
        if !compact {
            fit(
                ui,
                &sub,
                Rect::new(x + 16., 119., w - 32., 26.),
                13.,
                ui.theme.muted,
                false,
            );
        }
        let tx = if sideways { x + w * 0.42 } else { x };
        let tabs_w = if sideways { w * 0.58 } else { w };
        let tabs_y = if sideways {
            62.
        } else if compact {
            116.
        } else {
            174.
        };
        let tw = (tabs_w - 16.) / 3.;
        for (i, (tab, label)) in [
            (Tab::Village, "Village"),
            (Tab::Chat, "Chat"),
            (Tab::Role, "My role"),
        ]
        .iter()
        .enumerate()
        {
            if pill(
                ui,
                label,
                Rect::new(tx + i as f32 * (tw + 8.), tabs_y, tw, 44.),
                self.tab == *tab,
            ) {
                if self.tab != *tab {
                    self.pan = BoardPan::default();
                }
                self.tab = *tab;
            }
        }
        match self.tab {
            Tab::Village => self.village(ui, press, room, g, body, &mut out),
            Tab::Chat => self.chat(ui, press, g, (chat.0, chat.1, room.you), body, &mut out),
            Tab::Role => self.role(ui, press, g, body),
        }
        out
    }
    fn village(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        room: &RoomView,
        g: &View,
        area: Rect,
        out: &mut Outcome,
    ) {
        let x = area.x;
        let w = area.w;
        let alive = g.players[room.you].alive;
        let mut modes = if alive && g.phase != Phase::Finished {
            vec![(
                Mode::Ability,
                if g.phase == Phase::Night {
                    "Night"
                } else if g.phase == Phase::Vote {
                    "Vote"
                } else {
                    "Continue"
                },
            )]
        } else {
            vec![]
        };
        if g.phase == Phase::Night
            && !g.targets.is_empty()
            && !matches!(g.role, Role::Avenger | Role::JuniorWolf)
        {
            modes[0] = (Mode::Ability, ability(g.role));
        }
        if g.phase == Phase::Night && !g.victims.is_empty() {
            modes.push((
                Mode::Hunt,
                if g.role == Role::Witch {
                    "Poison"
                } else {
                    "Hunt"
                },
            ));
        }
        if matches!(g.phase, Phase::Discussion | Phase::Vote)
            && g.role == Role::Gunner
            && !g.targets.is_empty()
        {
            modes.push((Mode::Shoot, "Shoot"));
        }
        if g.can_mark {
            modes.push((Mode::Mark, "Revenge"));
        }
        if self.mode != Mode::Ability && !modes.iter().any(|(m, _)| *m == self.mode) {
            self.mode = Mode::Ability;
        }
        if self.mode == Mode::Ability
            && g.targets.is_empty()
            && g.phase == Phase::Night
            && !g.victims.is_empty()
        {
            self.mode = Mode::Hunt;
        }
        let mode_y = area.y;
        if !modes.is_empty() {
            let mw = (w - 8. * (modes.len() - 1) as f32) / modes.len() as f32;
            for (i, (mode, label)) in modes.iter().enumerate() {
                if pill(
                    ui,
                    label,
                    Rect::new(x + i as f32 * (mw + 8.), mode_y, mw, 44.),
                    self.mode == *mode,
                ) {
                    self.mode = *mode;
                }
            }
        } else {
            fit(
                ui,
                if !alive {
                    "You are a ghost. Watch the village or join ghost chat."
                } else {
                    g.role.title()
                },
                Rect::new(x, mode_y, w, 40.),
                14.,
                ui.theme.accent,
                true,
            );
        }
        let compact = screen_height() < 600.;
        let gap_y = if compact { 54. } else { 57. };
        let viewport = Rect::new(
            x,
            area.y + gap_y,
            w,
            (screen_height() - area.y - gap_y - 90.).max(44.),
        );
        let cols = if compact && w < 500. {
            2
        } else if w >= 700. {
            6
        } else {
            4
        };
        let cell_w = (w - 8. * (cols - 1) as f32) / cols as f32;
        let cell_h = if compact {
            64.
        } else if w >= 700. {
            120.
        } else {
            104.
        };
        let grid_h = g.players.len().div_ceil(cols) as f32 * cell_h;
        let event_h: f32 = g
            .events
            .iter()
            .rev()
            .take(4)
            .map(|e| wrapped_lines(ui, e, w - 24., 13.).len() as f32 * 20. + 9.)
            .sum();
        let (origin, tap) = page_input(
            &mut self.pan,
            &mut self.blocked,
            press,
            viewport,
            grid_h + event_h + 48.,
        );
        let old = ui.override_pointer(tap);
        clip(Some(viewport));
        for (i, player) in g.players.iter().enumerate() {
            let r = Rect::new(
                x + (i % cols) as f32 * (cell_w + 8.),
                origin.y + (i / cols) as f32 * cell_h,
                cell_w,
                cell_h - 9.,
            );
            let selected = match self.mode {
                Mode::Hunt => self.kill == Some(i),
                Mode::Mark => self.mark.or(g.mark) == Some(i),
                _ => self.target == Some(i),
            };
            let legal = alive
                && !g.phase.eq(&Phase::Finished)
                && match self.mode {
                    Mode::Hunt => !g.locked && g.victims.contains(&i),
                    Mode::Shoot => !g.locked && g.targets.contains(&i),
                    Mode::Mark => g.can_mark && player.alive && i != room.you,
                    Mode::Ability => {
                        !g.locked
                            && if g.phase == Phase::Vote {
                                player.alive && i != room.you
                            } else {
                                g.targets.contains(&i)
                            }
                    }
                };
            bordered(
                r,
                16.,
                if selected {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                if selected && !ui.theme.saver {
                    ui.theme.panel
                } else {
                    ui.theme.bg
                },
            );
            let c = if compact {
                vec2(r.x + 23., r.y + 22.)
            } else {
                vec2(r.center().x, r.y + 29.)
            };
            wolves_art::portrait(
                c,
                if compact { 17. } else { 23. },
                i,
                player.role,
                ui.theme.saver,
            );
            if !player.alive {
                draw_line(
                    c.x - 16.,
                    c.y - 16.,
                    c.x + 16.,
                    c.y + 16.,
                    2.,
                    ui.theme.muted,
                );
                draw_line(
                    c.x + 16.,
                    c.y - 16.,
                    c.x - 16.,
                    c.y + 16.,
                    2.,
                    ui.theme.muted,
                );
            }
            fit(
                ui,
                &player.name,
                if compact {
                    Rect::new(r.x + 46., r.y + 6., r.w - 52., 22.)
                } else {
                    Rect::new(r.x + 4., r.y + 55., r.w - 8., 21.)
                },
                13.,
                if player.alive {
                    ui.theme.text
                } else {
                    ui.theme.muted
                },
                true,
            );
            fit(
                ui,
                player
                    .role
                    .map_or(if i == room.you { "you" } else { "?" }, Role::title),
                if compact {
                    Rect::new(r.x + 46., r.y + 28., r.w - 52., 15.)
                } else {
                    Rect::new(r.x + 3., r.y + 77., r.w - 6., 15.)
                },
                9.,
                ui.theme.muted,
                false,
            );
            let hit =
                r.overlaps(&viewport) && (tap.is_some_and(|p| r.contains(p)) || ui.keyboard_hit(r));
            if hit && legal {
                ui.activated = true;
                let value = if selected { None } else { Some(i) };
                match self.mode {
                    Mode::Hunt => self.kill = value,
                    Mode::Mark => self.mark = value,
                    _ => self.target = value,
                }
            }
        }
        let mut y = origin.y + grid_h + 13.;
        for event in g.events.iter().rev().take(4) {
            let h = wrapped_lines(ui, event, w - 24., 13.).len() as f32 * 20. + 9.;
            wrap(
                ui,
                event,
                Rect::new(x + 12., y, w - 24., h),
                13.,
                ui.theme.muted,
                false,
            );
            y += h;
        }
        clip(None);
        ui.override_pointer(old);
        let bottom = screen_height() - 68.;
        let selection = match self.mode {
            Mode::Hunt => self.kill,
            Mode::Mark => self.mark.or(g.mark),
            _ => self.target,
        };
        let hint = if self.mode == Mode::Mark {
            selection.map_or_else(
                || "Choose your revenge target".into(),
                |i| format!("Revenge → {}", g.players[i].name),
            )
        } else if g.phase == Phase::Night {
            format!(
                "{} · {}",
                self.target
                    .map_or("No ability", |i| g.players[i].name.as_str()),
                self.kill.map_or("No hunt", |i| g.players[i].name.as_str())
            )
        } else {
            selection.map_or_else(
                || {
                    if g.phase == Phase::Vote {
                        "No selection · abstain"
                    } else {
                        "Discuss in village chat"
                    }
                    .into()
                },
                |i| format!("Selected: {}", g.players[i].name),
            )
        };
        fit(
            ui,
            &hint,
            Rect::new(x, bottom - 21., w, 18.),
            11.,
            ui.theme.muted,
            false,
        );
        let label = if self.mode == Mode::Mark && g.can_mark {
            "Set revenge mark"
        } else if g.phase == Phase::Finished {
            "Play again"
        } else if !alive {
            "Watching as a ghost"
        } else if g.locked {
            "Choice locked"
        } else if self.mode == Mode::Shoot {
            "Fire shot"
        } else {
            match g.phase {
                Phase::Night => "Lock night choices",
                Phase::Dawn => "Continue",
                Phase::Discussion => "Ready to vote",
                Phase::Vote => {
                    if self.target.is_some() {
                        "Confirm vote"
                    } else {
                        "Abstain"
                    }
                }
                Phase::Finished => "Play again",
            }
        };
        let enabled = if g.phase == Phase::Finished {
            room.host == room.you
        } else if self.mode == Mode::Mark {
            g.can_mark && self.mark.is_some()
        } else {
            alive && !g.locked && (self.mode != Mode::Shoot || self.target.is_some())
        };
        if primary(ui, label, Rect::new(x, bottom, w, 48.), enabled) {
            if g.phase == Phase::Finished {
                out.command = Some(Command::Rematch);
            } else if self.mode == Mode::Mark {
                out.movement(Move::Mark {
                    target: self.mark.unwrap(),
                });
                self.mode = Mode::Ability;
            } else if self.mode == Mode::Shoot {
                out.movement(Move::Shoot {
                    target: self.target.unwrap(),
                });
            } else {
                out.movement(match g.phase {
                    Phase::Night => Move::Night {
                        target: self.target,
                        kill: self.kill,
                    },
                    Phase::Vote => Move::Vote {
                        target: self.target,
                    },
                    _ => Move::Ready,
                });
            }
        }
    }
    fn role(&mut self, ui: &mut Ui, press: Option<Vec2>, g: &View, area: Rect) {
        let x = area.x;
        let w = area.w;
        let viewport = Rect::new(x, area.y, w, screen_height() - area.y - 20.);
        let desc_h = wrapped_lines(ui, g.role.description(), w - 36., 16.).len() as f32 * 25.;
        let total = 240.
            + desc_h
            + g.findings.len() as f32 * 47.
            + g.pool.len() as f32 * 36.
            + g.players.len() as f32 * 54.
            + 200.;
        let (origin, _) = page_input(&mut self.pan, &mut self.blocked, press, viewport, total);
        clip(Some(viewport));
        let card = Rect::new(x, origin.y, w, 135. + desc_h);
        bordered(card, 22., ui.theme.line, ui.theme.panel);
        wolves_art::portrait(
            vec2(x + 49., card.y + 45.),
            32.,
            0,
            Some(g.role),
            ui.theme.saver,
        );
        fit(
            ui,
            g.role.title(),
            Rect::new(x + 98., card.y + 15., w - 114., 36.),
            24.,
            ui.theme.text,
            true,
        );
        fit(
            ui,
            if g.role.wolf() {
                "WOLF TEAM"
            } else if g.role.village() {
                "VILLAGE TEAM"
            } else {
                "SOLO ROLE"
            },
            Rect::new(x + 98., card.y + 57., w - 114., 22.),
            11.,
            ui.theme.accent,
            true,
        );
        wrap(
            ui,
            g.role.description(),
            Rect::new(x + 18., card.y + 101., w - 36., desc_h),
            16.,
            ui.theme.text,
            false,
        );
        let power = match g.role {
            Role::Witch => format!(
                "Shield: {}  ·  Poison: {}",
                if g.power.heal { "1" } else { "0" },
                if g.power.poison { "1" } else { "0" }
            ),
            Role::Medium => format!("Revival: {}", u8::from(g.power.revive)),
            Role::Gunner => format!("{} bullets left", g.bullets),
            Role::Bodyguard => format!("{} hits remaining", 2_u8.saturating_sub(g.power.hits)),
            _ => String::new(),
        };
        let mut y = card.bottom() + 18.;
        if !power.is_empty() {
            fit(
                ui,
                &power,
                Rect::new(x, y, w, 28.),
                14.,
                ui.theme.accent,
                true,
            );
            y += 42.;
        }
        if g.power.injured.is_some() {
            fit(
                ui,
                "Injured · you fall at the end of this day",
                Rect::new(x, y, w, 30.),
                14.,
                ui.theme.accent,
                true,
            );
            y += 42.;
        }
        for f in g.findings.iter().rev() {
            let result = f.role.map_or_else(
                || format!("{:?} aura", f.aura.unwrap_or(wolves::Aura::Unknown)),
                |r| r.title().to_owned(),
            );
            fit(
                ui,
                &format!(
                    "Night {} · {} → {}",
                    f.day, g.players[f.target].name, result
                ),
                Rect::new(x + 8., y, w - 16., 35.),
                14.,
                ui.theme.text,
                false,
            );
            y += 47.;
        }
        if let Some(votes) = &g.pack_votes {
            fit(
                ui,
                "PRIVATE PACK CHOICES",
                Rect::new(x + 8., y, w - 16., 28.),
                11.,
                ui.theme.accent,
                true,
            );
            y += 34.;
            for (who, target) in votes {
                fit(
                    ui,
                    &format!(
                        "{} → {}",
                        g.players[*who].name,
                        target.map_or("resting", |i| g.players[i].name.as_str())
                    ),
                    Rect::new(x + 8., y, w - 16., 25.),
                    13.,
                    ui.theme.text,
                    false,
                );
                y += 27.;
            }
        }
        fit(
            ui,
            "IN THIS VILLAGE",
            Rect::new(x + 8., y, w - 16., 28.),
            11.,
            ui.theme.accent,
            true,
        );
        y += 38.;
        for (role, count) in &g.pool {
            fit(
                ui,
                &format!("{}  ×{}", role.title(), count),
                Rect::new(x + 8., y, w - 16., 28.),
                14.,
                ui.theme.text,
                false,
            );
            y += 36.;
        }
        if let Some(votes) = &g.ballots {
            fit(
                ui,
                "LAST VOTE · REVEALED",
                Rect::new(x + 8., y + 12., w - 16., 28.),
                11.,
                ui.theme.accent,
                true,
            );
            y += 46.;
            for (i, target) in votes.iter().enumerate() {
                fit(
                    ui,
                    &format!(
                        "{} → {}",
                        g.players[i].name,
                        target.map_or("abstained", |t| g.players[t].name.as_str())
                    ),
                    Rect::new(x + 8., y, w - 16., 25.),
                    13.,
                    ui.theme.text,
                    false,
                );
                y += 27.;
            }
        }
        clip(None);
    }
    fn chat(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        g: &View,
        compose: (&str, bool, usize),
        area: Rect,
        out: &mut Outcome,
    ) {
        let (text, editing, you) = compose;
        let x = area.x;
        let w = area.w;
        let alive = g.players[you].alive;
        let mut channels = vec![(Channel::Village, "Village")];
        if g.role.wolf() {
            channels.push((Channel::Pack, "Pack"));
        }
        if !alive || g.role == Role::Medium {
            channels.push((Channel::Ghosts, "Ghosts"));
        }
        let current = self
            .channel
            .filter(|c| channels.iter().any(|(v, _)| v == c))
            .unwrap_or(Channel::Village);
        self.channel = Some(current);
        let cw = (w - 8. * (channels.len() - 1) as f32) / channels.len() as f32;
        for (i, (channel, label)) in channels.iter().enumerate() {
            if pill(
                ui,
                label,
                Rect::new(x + i as f32 * (cw + 8.), area.y, cw, 44.),
                current == *channel,
            ) {
                self.channel = Some(*channel);
                self.pan = BoardPan::default();
            }
        }
        let current = self.channel.unwrap();
        let messages: Vec<_> = g.chat.iter().filter(|c| c.channel == current).collect();
        let heights: Vec<_> = messages
            .iter()
            .map(|c| wrapped_lines(ui, &c.text, w - 36., 15.).len() as f32 * 22. + 47.)
            .collect();
        let viewport = Rect::new(
            x,
            area.y + 57.,
            w,
            (screen_height() - area.y - 147.).max(30.),
        );
        let total = heights.iter().sum::<f32>().max(130.);
        let (origin, _) = page_input(&mut self.pan, &mut self.blocked, press, viewport, total);
        clip(Some(viewport));
        let mut y = origin.y;
        if messages.is_empty() {
            fit(
                ui,
                match current {
                    Channel::Village => "A quiet village. Start a conversation.",
                    Channel::Pack => "Private to the pack. Coordinate your hunt.",
                    Channel::Ghosts => "Ghosts + the Medium. Living messages are anonymous.",
                },
                Rect::new(x + 8., y + 24., w - 16., 36.),
                14.,
                ui.theme.muted,
                false,
            );
        }
        for (message, h) in messages.iter().zip(heights) {
            bordered(Rect::new(x, y, w, h - 8.), 16., ui.theme.line, ui.theme.bg);
            fit(
                ui,
                &format!(
                    "{} · day {}",
                    message
                        .who
                        .map_or("The Medium", |i| g.players[i].name.as_str()),
                    message.day
                ),
                Rect::new(x + 14., y + 7., w - 28., 22.),
                11.,
                ui.theme.accent,
                true,
            );
            wrap(
                ui,
                &message.text,
                Rect::new(x + 14., y + 33., w - 28., h - 37.),
                15.,
                ui.theme.text,
                false,
            );
            y += h;
        }
        clip(None);
        let allowed = match current {
            Channel::Village => {
                alive
                    && matches!(
                        g.phase,
                        Phase::Dawn | Phase::Discussion | Phase::Vote | Phase::Finished
                    )
            }
            Channel::Pack => {
                alive
                    && g.role.wolf()
                    && (matches!(g.phase, Phase::Night | Phase::Finished)
                        || (g.role == Role::AlphaWolf && g.power.alpha_day != g.day))
            }
            Channel::Ghosts => !alive || (g.role == Role::Medium && g.phase == Phase::Night),
        };
        let bottom = screen_height() - 68.;
        fit(
            ui,
            if allowed {
                "Messages stay inside this channel"
            } else {
                "You can read here. Speaking is closed in this phase."
            },
            Rect::new(x, bottom - 21., w, 18.),
            11.,
            ui.theme.muted,
            false,
        );
        let field = Rect::new(x, bottom, w - 76., 48.);
        bordered(
            field,
            14.,
            if editing {
                ui.theme.accent
            } else {
                ui.theme.line
            },
            ui.theme.bg,
        );
        fit(
            ui,
            if text.is_empty() { "Message…" } else { text },
            Rect::new(field.x + 12., field.y, field.w - 24., field.h),
            15.,
            ui.theme.text,
            false,
        );
        if allowed && ui.hit(field) {
            out.editor = Some(field);
        }
        if editing {
            crate::platform::editor_position(5, field);
        }
        if primary(
            ui,
            "Send",
            Rect::new(field.right() + 8., bottom, 68., 48.),
            allowed && !text.is_empty(),
        ) {
            out.movement(Move::Chat {
                channel: current,
                text: text.to_owned(),
            });
            out.clear_chat = true;
            self.pan.offset.y = (total + 90. - viewport.h).max(0.);
        }
    }
}
pub fn unix() -> u64 {
    macroquad::miniquad::date::now().max(0.) as u64
}
fn ability(role: Role) -> &'static str {
    match role {
        Role::Seer | Role::WolfSeer | Role::AuraSeer => "Inspect",
        Role::Medium => "Revive",
        Role::Witch | Role::Doctor | Role::Bodyguard | Role::ToughGuy => "Protect",
        Role::SerialKiller => "Hunt",
        Role::JuniorWolf | Role::Avenger => "Revenge",
        _ => "Ability",
    }
}

fn pill(ui: &mut Ui, text: &str, r: Rect, selected: bool) -> bool {
    bordered(
        r,
        22.,
        if selected {
            ui.theme.accent
        } else {
            ui.theme.line
        },
        if selected {
            ui.theme.panel
        } else {
            ui.theme.bg
        },
    );
    fit(
        ui,
        text,
        Rect::new(r.x + 6., r.y, r.w - 12., r.h),
        14.,
        if selected {
            ui.theme.accent
        } else {
            ui.theme.muted
        },
        selected,
    );
    ui.hit(r)
}
