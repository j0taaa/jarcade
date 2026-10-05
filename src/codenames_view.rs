//! Original vector word-table UI for private online teams.
use crate::{
    online_style::primary,
    online_view::{button, clip, fit, page_input},
    ui::{Ui, bordered, rounded},
};
use jarcade::{
    board_pan::BoardPan,
    multiplayer::{
        Command, RoomView,
        codenames::{Identity, Language, Move, Phase, Role, Seat, Team, View},
    },
};
use macroquad::prelude::*;

#[derive(Default)]
pub struct CodenamesUi {
    pan: BoardPan,
    blocked: bool,
    selected: Option<usize>,
    pub number: Option<u8>,
}
#[derive(Default)]
pub struct Outcome {
    pub command: Option<Command>,
    pub editor: Option<Rect>,
    pub leave: bool,
}
impl CodenamesUi {
    pub fn new() -> Self {
        Self {
            number: Some(2),
            ..Self::default()
        }
    }
    pub fn phase_changed(&mut self) {
        self.selected = None;
    }
    pub fn active(&self) -> bool {
        self.pan.active()
    }
    pub fn announcement(&self) -> String {
        if let Some(card) = self.selected {
            format!("Card {} selected. Confirm reveal or cancel.", card + 1)
        } else {
            String::new()
        }
    }
    pub fn lobby(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        room: &RoomView,
        bounds: Rect,
    ) -> Outcome {
        let mut out = Outcome::default();
        let setup = room.codenames_setup.clone().unwrap_or_default();
        let me = setup.seat(room.you);
        let team_counts = [Team::Red, Team::Blue].map(|t| {
            (0..room.members.len())
                .filter(|&i| setup.seat(i).team == t)
                .count()
        });
        let roster_h = team_counts.into_iter().max().unwrap_or(0).max(2) as f32 * 48. + 58.;
        let total = 290. + roster_h + 206.;
        let (origin, tap) = page_input(&mut self.pan, &mut self.blocked, press, bounds, total);
        clip(Some(bounds));
        let x = bounds.x;
        let w = bounds.w;
        let y = origin.y;
        let ticket = Rect::new(x, y + 4., w, 76.);
        bordered(ticket, 18., ui.theme.line, ui.theme.panel);
        fit(
            ui,
            "MISSION ROOM",
            Rect::new(x + 16., ticket.y + 8., w - 32., 16.),
            10.,
            ui.theme.muted,
            true,
        );
        fit(
            ui,
            &room.code,
            Rect::new(x + 16., ticket.y + 28., w - 115., 35.),
            28.,
            ui.theme.accent,
            true,
        );
        agent(
            vec2(ticket.right() - 38., ticket.center().y),
            19.,
            ui.theme.accent,
            ui.theme.bg,
        );
        fit(
            ui,
            "Choose your side",
            Rect::new(x, y + 90., w, 24.),
            17.,
            ui.theme.text,
            true,
        );
        let gap = 10.;
        let half = (w - gap) / 2.;
        for (i, t) in [Team::Red, Team::Blue].into_iter().enumerate() {
            let r = Rect::new(x + i as f32 * (half + gap), y + 122., half, 44.);
            let old = ui.theme.accent;
            ui.theme.accent = team_color(t, ui.theme.saver);
            if button(
                ui,
                &format!("{}  {}", if me.team == t { "●" } else { "○" }, t.label()),
                r,
                me.team == t,
                tap,
                bounds,
            ) && me.team != t
            {
                out.command = Some(Command::CodenamesSeat(Seat {
                    team: t,
                    role: me.role,
                }));
            }
            ui.theme.accent = old;
        }
        for (i, role) in [Role::Operative, Role::Spymaster].into_iter().enumerate() {
            if button(
                ui,
                role.label(),
                Rect::new(x + i as f32 * (half + gap), y + 175., half, 44.),
                me.role == role,
                tap,
                bounds,
            ) && me.role != role
            {
                out.command = Some(Command::CodenamesSeat(Seat {
                    team: me.team,
                    role,
                }));
            }
        }
        fit(
            ui,
            if me.role == Role::Spymaster {
                "You see the key. Give one-word clues."
            } else {
                "You see the words. Find your team's agents."
            },
            Rect::new(x, y + 222., w, 29.),
            12.,
            ui.theme.muted,
            false,
        );
        for (i, language) in [Language::English, Language::Portuguese]
            .into_iter()
            .enumerate()
        {
            let r = Rect::new(x + i as f32 * (half + gap), y + 255., half, 36.);
            if button(
                ui,
                language.label(),
                r,
                language == setup.language,
                tap,
                bounds,
            ) && room.host == room.you
                && setup.language != language
            {
                out.command = Some(Command::CodenamesLanguage(language));
            }
        }
        let roster_y = y + 308.;
        for (col, t) in [Team::Red, Team::Blue].into_iter().enumerate() {
            let r = Rect::new(x + col as f32 * (half + gap), roster_y, half, roster_h);
            let accent = team_color(t, ui.theme.saver);
            bordered(r, 16., accent, ui.theme.bg);
            agent(vec2(r.x + 24., r.y + 25.), 10., accent, ui.theme.bg);
            fit(
                ui,
                t.label(),
                Rect::new(r.x + 42., r.y + 6., r.w - 52., 36.),
                17.,
                accent,
                true,
            );
            let members: Vec<_> = room
                .members
                .iter()
                .enumerate()
                .filter(|(i, _)| setup.seat(*i).team == t)
                .collect();
            for (row, (i, member)) in members.iter().enumerate() {
                let sy = r.y + 51. + row as f32 * 48.;
                fit(
                    ui,
                    &format!(
                        "{}{}",
                        member.name,
                        if *i == room.you { " · you" } else { "" }
                    ),
                    Rect::new(r.x + 12., sy, r.w - 24., 22.),
                    13.,
                    ui.theme.text,
                    true,
                );
                fit(
                    ui,
                    &format!(
                        "{} · {}",
                        setup.seat(*i).role.label(),
                        if !member.connected {
                            "offline"
                        } else if member.ready {
                            "ready"
                        } else {
                            "waiting"
                        }
                    ),
                    Rect::new(r.x + 12., sy + 20., r.w - 24., 16.),
                    10.,
                    if member.ready { accent } else { ui.theme.muted },
                    false,
                );
            }
            if members.is_empty() {
                fit(
                    ui,
                    "Invite a teammate",
                    Rect::new(r.x + 12., r.y + 51., r.w - 24., 36.),
                    12.,
                    ui.theme.muted,
                    false,
                );
            }
        }
        let footer_y = roster_y + roster_h + 12.;
        if let Err(message) = setup.validate(room.members.len()) {
            fit(
                ui,
                message,
                Rect::new(x, footer_y, w, 28.),
                12.,
                ui.theme.muted,
                false,
            );
        }
        if button(
            ui,
            if room.members[room.you].ready {
                "✓  Ready"
            } else {
                "I'm ready"
            },
            Rect::new(x, footer_y + 34., w, 44.),
            !room.members[room.you].ready,
            tap,
            bounds,
        ) {
            out.command = Some(Command::Ready(!room.members[room.you].ready));
        }
        if room.host == room.you {
            let pointer = ui.override_pointer(tap);
            if primary(
                ui,
                "Open the mission",
                Rect::new(x, footer_y + 88., w, 44.),
                setup.validate(room.members.len()).is_ok()
                    && room.members.iter().all(|m| m.connected && m.ready),
            ) {
                out.command = Some(Command::Start);
            }
            ui.override_pointer(pointer);
        } else {
            fit(
                ui,
                "The host starts when both teams are ready",
                Rect::new(x, footer_y + 88., w, 44.),
                12.,
                ui.theme.muted,
                false,
            );
        }
        if button(
            ui,
            "Leave room",
            Rect::new(x, footer_y + 142., w, 44.),
            false,
            tap,
            bounds,
        ) {
            out.leave = true;
        }
        clip(None);
        out
    }
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        room: &RoomView,
        game: &View,
        field: (&str, bool),
        bounds: Rect,
    ) -> Outcome {
        let mut out = Outcome::default();
        let me = game.seats[room.you];
        let saver = ui.theme.saver;
        let x = bounds.x;
        let w = bounds.w;
        let y = bounds.y;
        let compact = bounds.h < 430.;
        let sidecar = compact && w >= 480.;
        let footer_h = if game.can_clue && (!compact || sidecar) {
            112.
        } else {
            60.
        };
        let top_h = if compact { 108. } else { 124. };
        let bw = if sidecar {
            (w - 206.).min(840.)
        } else {
            w.min(840.)
        };
        let bx = if sidecar { x } else { x + (w - bw) / 2. };
        let info_x = if sidecar { bx + bw + 16. } else { bx };
        let info_w = if sidecar { w - bw - 16. } else { bw };
        let team_gap = if sidecar { 6. } else { 10. };
        let team_w = (info_w - team_gap) / 2.;
        for (i, t) in [Team::Red, Team::Blue].into_iter().enumerate() {
            let r = Rect::new(info_x + i as f32 * (team_w + team_gap), y, team_w, 40.);
            let color = team_color(t, saver);
            bordered(
                r,
                12.,
                if game.team == t { color } else { ui.theme.line },
                ui.theme.bg,
            );
            if !sidecar {
                agent(vec2(r.x + 21., r.center().y), 8., color, ui.theme.bg);
            }
            fit(
                ui,
                &format!("{}   {}", t.label(), game.remaining[i]),
                Rect::new(
                    r.x + if sidecar { 6. } else { 37. },
                    r.y,
                    r.w - if sidecar { 12. } else { 49. },
                    r.h,
                ),
                18.,
                color,
                true,
            );
        }
        let title = if let Some(winner) = game.winner {
            format!("{} team wins", winner.label())
        } else if game.phase == Phase::Clue {
            format!("{} spymaster's clue", game.team.label())
        } else {
            let clue = game.clue.as_ref().unwrap();
            format!("{}  {}", clue.word, number_label(clue.number))
        };
        fit(
            ui,
            &title,
            Rect::new(info_x, y + 48., info_w, 31.),
            24.,
            if game.finished() {
                team_color(game.winner.unwrap(), saver)
            } else {
                ui.theme.text
            },
            true,
        );
        {
            let subtitle = if game.finished() {
                game.reason.clone()
            } else if me.role == Role::Spymaster {
                format!("{} spymaster · private key", me.team.label())
            } else if game.can_guess {
                match game.guesses_left {
                    Some(n) => format!("{} operative · up to {n} guesses left", me.team.label()),
                    None => format!("{} operative · unlimited guesses", me.team.label()),
                }
            } else {
                format!("{} operative · listen and discuss", me.team.label())
            };
            fit(
                ui,
                &subtitle,
                Rect::new(info_x, y + 82., info_w, if compact { 20. } else { 26. }),
                12.,
                ui.theme.muted,
                false,
            );
        }
        let board_viewport = if sidecar {
            Rect::new(bx, y, bw, bounds.h - 8.)
        } else {
            Rect::new(
                bx,
                y + top_h,
                bw,
                (bounds.h - top_h - footer_h - 14.).max(36.),
            )
        };
        let gap = if bw < 400. { 5. } else { 9. };
        let cell_w = (bw - gap * 4.) / 5.;
        let cell_h = if sidecar {
            ((board_viewport.h - gap * 4.) / 5.).clamp(44., 103.)
        } else {
            (cell_w * 0.9 + 10.).clamp(52., 103.)
        };
        let board_h = cell_h * 5. + gap * 4.;
        let (origin, tap) = page_input(
            &mut self.pan,
            &mut self.blocked,
            press,
            board_viewport,
            board_h,
        );
        clip(Some(board_viewport));
        let board_y = if board_h < board_viewport.h {
            board_viewport.y + (board_viewport.h - board_h) * 0.35
        } else {
            origin.y
        };
        for (i, card) in game.cards.iter().enumerate() {
            let rect = Rect::new(
                bx + (i % 5) as f32 * (cell_w + gap),
                board_y + (i / 5) as f32 * (cell_h + gap),
                cell_w,
                cell_h,
            );
            let (fill, ink, edge) = card_palette(ui, card.identity, card.revealed);
            let chosen = self.selected == Some(i);
            bordered(
                rect,
                (cell_w * 0.13).min(13.),
                if chosen { ui.theme.accent } else { edge },
                fill,
            );
            if !saver && !card.revealed {
                draw_line(
                    rect.x + 8.,
                    rect.bottom() - 7.,
                    rect.right() - 8.,
                    rect.bottom() - 7.,
                    1.,
                    edge,
                );
            }
            fit(
                ui,
                &card.word,
                Rect::new(
                    rect.x + 3.,
                    rect.y + cell_h * 0.29,
                    rect.w - 6.,
                    cell_h * 0.35,
                ),
                if bw < 400. { 14. } else { 18. },
                ink,
                true,
            );
            if let Some(id) = card.identity {
                let mark_y = rect.y + cell_h * 0.17;
                match id {
                    Identity::Assassin => {
                        draw_line(
                            rect.center().x - 3.,
                            mark_y - 3.,
                            rect.center().x + 3.,
                            mark_y + 3.,
                            1.6,
                            ink,
                        );
                        draw_line(
                            rect.center().x + 3.,
                            mark_y - 3.,
                            rect.center().x - 3.,
                            mark_y + 3.,
                            1.6,
                            ink,
                        );
                    }
                    Identity::Neutral => {
                        draw_circle_lines(rect.center().x, mark_y, 3., 1., ink);
                    }
                    Identity::Red => {
                        draw_triangle(
                            vec2(rect.center().x, mark_y - 3.5),
                            vec2(rect.center().x - 3.5, mark_y + 3.),
                            vec2(rect.center().x + 3.5, mark_y + 3.),
                            ink,
                        );
                    }
                    Identity::Blue => {
                        draw_circle(rect.center().x, mark_y, 3., ink);
                    }
                }
            }
            if card.revealed {
                fit(
                    ui,
                    "FOUND",
                    Rect::new(rect.x, rect.y + cell_h * 0.72, rect.w, cell_h * 0.19),
                    8.,
                    ink,
                    true,
                );
            }
            if game.can_guess
                && !card.revealed
                && rect.overlaps(&board_viewport)
                && (tap.is_some_and(|p| rect.contains(p)) || ui.keyboard_hit(rect))
            {
                self.selected = Some(i);
                ui.activated = true;
            }
        }
        clip(None);
        let fy = bounds.bottom() - footer_h;
        let bx = info_x;
        let bw = info_w;
        if game.can_clue {
            let single_row = compact && !sidecar;
            let field_r = Rect::new(bx, fy, bw - if single_row { 124. } else { 92. }, 44.);
            bordered(
                field_r,
                13.,
                if field.1 {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                ui.theme.bg,
            );
            fit(
                ui,
                if field.0.is_empty() {
                    "One-word clue"
                } else {
                    field.0
                },
                Rect::new(field_r.x + 13., field_r.y, field_r.w - 26., field_r.h),
                17.,
                if field.0.is_empty() {
                    ui.theme.muted
                } else {
                    ui.theme.text
                },
                false,
            );
            if field.1 {
                crate::platform::editor_position(6, field_r);
            }
            if press.is_some_and(|p| field_r.contains(p)) || ui.keyboard_hit(field_r) {
                out.editor = Some(field_r);
                ui.activated = true;
            }
            // A compact stepper cycles 0…9 and infinity, without a long option menu.
            let number_rect = if single_row {
                Rect::new(bx + bw - 118., fy, 44., 44.)
            } else {
                Rect::new(bx + bw - 82., fy, 82., 44.)
            };
            if ui.button(&number_label(self.number), number_rect, false) {
                self.number = match self.number {
                    Some(9) => None,
                    Some(n) => Some(n + 1),
                    None => Some(0),
                };
            }
            let send_rect = if single_row {
                Rect::new(bx + bw - 64., fy, 64., 44.)
            } else {
                Rect::new(bx, fy + 54., bw, 44.)
            };
            if primary(
                ui,
                if single_row { "Send" } else { "Send clue" },
                send_rect,
                !field.0.trim().is_empty(),
            ) {
                out.command = Some(Command::Codenames(Move::Clue {
                    word: field.0.into(),
                    number: self.number,
                }));
            }
        } else if game.finished() {
            if room.host == room.you {
                if ui.button("New mission", Rect::new(bx, fy, bw, 44.), true) {
                    out.command = Some(Command::Rematch);
                }
            } else {
                fit(
                    ui,
                    "The host can open another mission",
                    Rect::new(bx, fy, bw, 44.),
                    14.,
                    ui.theme.muted,
                    false,
                );
            }
        } else if game.can_guess {
            if let Some(selected) = self.selected {
                let cw = (bw - 10.) / 2.;
                if ui.button("Cancel", Rect::new(bx, fy, cw, 44.), false) {
                    self.selected = None;
                }
                if ui.button("Reveal", Rect::new(bx + cw + 10., fy, cw, 44.), true) {
                    out.command = Some(Command::Codenames(Move::Guess { card: selected }));
                    self.selected = None;
                }
            } else if primary(
                ui,
                if game.can_pass {
                    "End turn"
                } else {
                    "Choose a word"
                },
                Rect::new(bx, fy, bw, 44.),
                game.can_pass,
            ) {
                out.command = Some(Command::Codenames(Move::Pass));
            }
        } else {
            fit(
                ui,
                if me.role == Role::Spymaster && game.phase == Phase::Guess && me.team == game.team
                {
                    "Your operatives are choosing. Keep the key secret."
                } else {
                    "Listen to the clue and discuss with your team."
                },
                Rect::new(bx, fy, bw, 44.),
                13.,
                ui.theme.muted,
                false,
            );
        }
        out
    }
}
trait Finished {
    fn finished(&self) -> bool;
}
impl Finished for View {
    fn finished(&self) -> bool {
        self.phase == Phase::Finished
    }
}
fn number_label(number: Option<u8>) -> String {
    number.map_or_else(|| "∞".into(), |n| n.to_string())
}
pub fn team_color(team: Team, saver: bool) -> Color {
    match (team, saver) {
        (Team::Red, false) => color_u8!(206, 79, 71, 255),
        (Team::Blue, false) => color_u8!(47, 116, 187, 255),
        (Team::Red, true) => color_u8!(255, 133, 120, 255),
        (Team::Blue, true) => color_u8!(117, 179, 244, 255),
    }
}
fn card_palette(ui: &Ui, identity: Option<Identity>, revealed: bool) -> (Color, Color, Color) {
    let saver = ui.theme.saver;
    let accent = match identity {
        Some(Identity::Red) => team_color(Team::Red, saver),
        Some(Identity::Blue) => team_color(Team::Blue, saver),
        Some(Identity::Assassin) => {
            if saver {
                color_u8!(210, 191, 231, 255)
            } else {
                color_u8!(58, 43, 77, 255)
            }
        }
        Some(Identity::Neutral) => {
            if saver {
                color_u8!(160, 154, 133, 255)
            } else {
                color_u8!(135, 123, 93, 255)
            }
        }
        None => ui.theme.text,
    };
    if saver {
        return (
            BLACK,
            if identity.is_some() {
                accent
            } else {
                ui.theme.text
            },
            if identity.is_some() {
                accent
            } else {
                ui.theme.line
            },
        );
    }
    if revealed {
        (accent, WHITE, accent)
    } else {
        let fill = match identity {
            Some(Identity::Red) => color_u8!(254, 231, 226, 255),
            Some(Identity::Blue) => color_u8!(225, 240, 254, 255),
            Some(Identity::Assassin) => color_u8!(237, 231, 245, 255),
            Some(Identity::Neutral) => color_u8!(246, 239, 220, 255),
            None => color_u8!(252, 248, 237, 255),
        };
        (
            fill,
            accent,
            if identity.is_some() {
                Color::new(accent.r, accent.g, accent.b, 0.38)
            } else {
                color_u8!(235, 226, 208, 255)
            },
        )
    }
}
pub fn agent(center: Vec2, radius: f32, ink: Color, paper: Color) {
    draw_circle(center.x, center.y - radius * 0.13, radius * 0.48, ink);
    rounded(
        Rect::new(
            center.x - radius * 0.65,
            center.y + radius * 0.31,
            radius * 1.3,
            radius * 0.44,
        ),
        radius * 0.17,
        ink,
    );
    draw_line(
        center.x - radius * 0.39,
        center.y - radius * 0.16,
        center.x + radius * 0.39,
        center.y - radius * 0.16,
        radius * 0.17,
        paper,
    );
    draw_triangle(
        vec2(center.x, center.y + radius * 0.23),
        vec2(center.x - radius * 0.11, center.y + radius * 0.41),
        vec2(center.x + radius * 0.11, center.y + radius * 0.41),
        paper,
    );
}
pub fn preview(ui: &Ui, rect: Rect) {
    let gap = rect.w * 0.035;
    let cw = (rect.w - gap * 4.) / 5.;
    let ch = (rect.h - gap * 4.) / 5.;
    let words = [
        "MOON", "ROSE", "SHIP", "WOLF", "STAR", "GOLD", "SNOW", "FISH", "TREE", "FIRE", "BELL",
        "RAIN", "BOOK", "KITE", "CAKE", "RING", "FROG", "WIND", "LION", "LEAF", "WAVE", "MILK",
        "SAND", "OWL", "ICE",
    ];
    for (i, word) in words.iter().enumerate() {
        let r = Rect::new(
            rect.x + (i % 5) as f32 * (cw + gap),
            rect.y + (i / 5) as f32 * (ch + gap),
            cw,
            ch,
        );
        let identity = if [0, 5, 6].contains(&i) {
            Some(Identity::Blue)
        } else if [3, 4, 9].contains(&i) {
            Some(Identity::Red)
        } else {
            None
        };
        let (fill, ink, edge) = card_palette(ui, identity, identity.is_some());
        bordered(r, 4., edge, fill);
        fit(
            ui,
            word,
            Rect::new(r.x + 1., r.y, r.w - 2., r.h),
            (cw * 0.24).min(12.),
            ink,
            true,
        );
    }
}
