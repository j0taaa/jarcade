//! Shared multiplayer interface; only the server can decide legal moves.
use crate::{
    card_art::{self, DeckArt},
    online_net::Network,
    online_style::{self as style, primary},
    platform,
    ui::{CORAL, Icon, Ui, bordered, rounded},
};
use jarcade::{
    board_pan::BoardPan,
    layout::touch_point,
    multiplayer::{
        ClientMessage, Command, GameKind, RoomView, ServerMessage, Session, clean_text, coup,
        reverie, wolves,
    },
};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
#[derive(Default, Serialize, Deserialize)]
struct Saved {
    name: String,
    sessions: Vec<Session>,
}
pub struct OnlinePage {
    pub game: GameKind,
    pub revision: u64,
    pub art: DeckArt,
    network: Network,
    room: Option<RoomView>,
    saved: Saved,
    connected: bool,
    connecting: bool,
    pending: Option<ClientMessage>,
    error: String,
    fields: [String; 3],
    editing: Option<usize>,
    help: bool,
    clue_open: bool,
    confirm_leave: bool,
    pan: BoardPan,
    blocked_touch: bool,
    #[cfg(any(target_os = "android", target_os = "ios"))]
    keyboard_pan: BoardPan,
    preview: Option<u8>,
    selected: Vec<u8>,
    exchange: Vec<usize>,
    target: Option<coup::Action>,
    sent: Option<u64>,
    wolves_ui: crate::wolves_view::WolvesUi,
}
impl OnlinePage {
    pub fn new() -> Self {
        let saved: Saved = serde_json::from_str(&platform::load_online()).unwrap_or_default();
        let name = saved.name.clone();
        Self {
            game: GameKind::Court,
            revision: 0,
            art: DeckArt::new(),
            network: Network::new(),
            room: None,
            saved,
            connected: false,
            connecting: false,
            pending: None,
            error: String::new(),
            fields: [name, String::new(), String::new()],
            editing: None,
            help: false,
            clue_open: false,
            confirm_leave: false,
            pan: BoardPan::default(),
            blocked_touch: false,
            #[cfg(any(target_os = "android", target_os = "ios"))]
            keyboard_pan: BoardPan::default(),
            preview: None,
            selected: vec![],
            exchange: vec![],
            target: None,
            sent: None,
            wolves_ui: crate::wolves_view::WolvesUi::default(),
        }
    }
    pub fn enter(&mut self, game: GameKind, code: Option<String>) {
        self.suspend();
        self.game = game;
        self.wolves_ui = crate::wolves_view::WolvesUi::default();
        self.room = None;
        self.help = false;
        self.clue_open = false;
        self.confirm_leave = false;
        self.error.clear();
        self.fields[1] = code.unwrap_or_default();
        self.fields[2].clear();
        self.pan = BoardPan::default();
        self.selected.clear();
        self.exchange.clear();
        self.target = None;
        self.preview = None;
        self.revision += 1;
    }
    pub fn suspend(&mut self) {
        self.network.close();
        self.connected = false;
        self.connecting = false;
        self.pending = None;
        self.sent = None;
        self.close_editor();
        self.pan.cancel();
    }
    fn close_editor(&mut self) {
        self.editing = None;
        platform::editor_close();
    }
    fn persist(&mut self) {
        self.saved.name = self.fields[0].clone();
        if !platform::save_online(&serde_json::to_string(&self.saved).unwrap()) {
            self.error = "This device could not save your reconnect seat.".into();
        }
    }
    fn session(&self) -> Option<&Session> {
        self.saved.sessions.iter().find(|s| s.game == self.game)
    }
    fn connect(&mut self, msg: ClientMessage) {
        self.error.clear();
        self.connecting = true;
        self.connected = false;
        self.pending = Some(msg);
        self.network.connect();
        self.close_editor();
        self.revision += 1;
    }
    fn reconnect(&mut self) {
        if let Some(s) = self.session().cloned() {
            self.connect(ClientMessage::Resume {
                room: s.room,
                token: s.token,
            });
        }
    }
    fn play(&mut self, command: Command) {
        if let Some(room) = &self.room
            && self.connected
            && self.sent != Some(room.revision)
        {
            self.network.send(&ClientMessage::Play {
                revision: room.epoch,
                command,
            });
            self.sent = Some(room.revision);
            self.close_editor();
            self.error.clear();
            self.revision += 1;
        }
    }
    pub fn poll(&mut self) {
        while let Some(edit) = platform::editor_poll() {
            let id = if edit.id == 5 { 2 } else { edit.id };
            if id < 3 {
                self.fields[id] = if id == 1 {
                    edit.text
                        .to_ascii_uppercase()
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric())
                        .take(6)
                        .collect()
                } else {
                    clean_text(&edit.text, if id == 0 { 24 } else { 160 })
                };
                if edit.done {
                    self.editing = None;
                }
                self.revision += 1;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(id) = self.editing {
            while let Some(c) = get_char_pressed() {
                if !c.is_control()
                    && self.fields[id].chars().count()
                        < if id == 2 {
                            160
                        } else if id == 1 {
                            6
                        } else {
                            24
                        }
                {
                    self.fields[id].push(c);
                    self.revision += 1;
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.fields[id].pop();
                self.revision += 1;
            }
            if is_key_pressed(KeyCode::Enter) {
                self.close_editor();
                // Consume the text submission so the same Enter cannot re-open
                // the field or activate the next game control in this frame.
                clear_input_queue();
                self.revision += 1;
            }
        }
        while let Some(message) = self.network.poll() {
            self.revision += 1;
            match message {
                ServerMessage::Connected => {
                    if let Some(msg) = self.pending.take() {
                        self.network.send(&msg);
                    }
                }
                ServerMessage::Disconnected { reason } => {
                    self.connected = false;
                    self.connecting = false;
                    self.sent = None;
                    self.error = reason;
                }
                ServerMessage::Welcome { session } => {
                    self.connected = true;
                    self.connecting = false;
                    self.game = session.game;
                    self.saved.sessions.retain(|s| s.game != session.game);
                    self.saved.sessions.push(session);
                    self.persist();
                }
                ServerMessage::State { room } => {
                    let phase = phase_key(&room);
                    if self.room.as_ref().map(phase_key) != Some(phase) {
                        self.wolves_ui.phase_changed();
                        self.selected.clear();
                        self.exchange.clear();
                        self.target = None;
                        self.preview = None;
                        self.clue_open = false;
                        if self.game != GameKind::Wolves {
                            self.fields[2].clear();
                        }
                        self.pan = BoardPan::default();
                        self.close_editor();
                    }
                    self.sent = None;
                    self.room = Some(*room);
                }
                ServerMessage::Error { message } => {
                    self.error = message;
                    self.sent = None;
                    self.connecting = false;
                }
                ServerMessage::Left => {
                    self.saved.sessions.retain(|s| s.game != self.game);
                    self.persist();
                    self.room = None;
                    self.confirm_leave = false;
                    self.suspend();
                }
            }
        }
    }
    pub fn needs_frame(&self) -> bool {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        if self.keyboard_pan.active() {
            return true;
        }
        self.art.loading() || self.pan.active() || self.wolves_ui.active()
    }
    pub fn delay(&self) -> Option<f64> {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        if self.editing.is_some() {
            return None;
        }
        self.room
            .as_ref()
            .filter(|_| self.connected && !self.help && !self.confirm_leave)
            .and_then(|r| r.wolves.as_ref())
            .filter(|g| g.phase != wolves::Phase::Finished)
            .map(|_| 1.0)
    }
    pub fn announcement(&self) -> String {
        let mut text = format!("Jarcade. {}. ", self.game.title());
        if let Some(room) = &self.room {
            text.push_str(&format!(
                "Room {}. You are {}. ",
                room.code, room.members[room.you].name
            ));
            if let Some(g) = &room.court {
                text.push_str(&g.prompt);
                text.push_str(". Actions: ");
                text.push_str(
                    &g.actions
                        .iter()
                        .map(|a| a.title())
                        .chain(g.choices.iter().map(|c| c.label.as_str()))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            } else if let Some(g) = &room.reverie {
                text.push_str(&format!(
                    "Round {}. {}. Clue: {}. ",
                    g.round, g.phase, g.clue
                ));
                text.push_str(
                    &g.players
                        .iter()
                        .map(|p| format!("{} {} points", p.name, p.score))
                        .collect::<Vec<_>>()
                        .join(". "),
                );
            } else if let Some(g) = &room.wolves {
                text.push_str(&format!(
                    "{} {}. Your role: {}. {} alive. {} ",
                    g.phase.title(),
                    g.day,
                    g.role.title(),
                    g.alive,
                    g.events.last().map_or("", String::as_str)
                ));
                text.push_str(&self.wolves_ui.announcement());
            } else {
                if room.game == GameKind::Wolves {
                    text.push_str(&self.wolves_ui.announcement());
                }
                text.push_str("Lobby. Ready, then the host starts.");
            }
        } else {
            text.push_str("Create a room or join using a room code.");
        }
        if self.confirm_leave {
            text.push_str(" Leave this table? Stay or Leave.");
        } else if self.help {
            text.push_str(" Rules are open.");
        } else if self.clue_open {
            text.push_str(" Full clue is open.");
        } else if self.preview.is_some() {
            text.push_str(" Card preview is open.");
        }
        if !self.selected.is_empty() {
            text.push_str(&format!(" {} cards selected.", self.selected.len()));
        }
        if !self.error.is_empty() {
            text.push_str(&self.error);
        }
        text
    }
    pub fn back(&mut self) -> bool {
        self.revision += 1;
        if self.wolves_ui.close_setup() {
            return false;
        }
        if self.clue_open {
            self.clue_open = false;
            self.pan = BoardPan::default();
            return false;
        }
        if self.preview.take().is_some() {
            return false;
        }
        if self.help {
            self.help = false;
            self.pan = BoardPan::default();
            return false;
        }
        if self.confirm_leave {
            self.confirm_leave = false;
            return false;
        }
        if self.editing.is_some() {
            self.close_editor();
            return false;
        }
        true
    }
    fn field(&mut self, ui: &mut Ui, id: usize, rect: Rect, label: &str, tap: Option<Vec2>) {
        bordered(
            rect,
            14.,
            if self.editing == Some(id) {
                ui.theme.accent
            } else {
                ui.theme.line
            },
            ui.theme.bg,
        );
        let value = if self.fields[id].is_empty() {
            label
        } else {
            &self.fields[id]
        };
        fit(
            ui,
            value,
            Rect::new(rect.x + 14., rect.y, rect.w - 28., rect.h),
            16.,
            if self.fields[id].is_empty() {
                ui.theme.muted
            } else {
                ui.theme.text
            },
            false,
        );
        if tap.is_some_and(|p| rect.contains(p)) || ui.keyboard_hit(rect) {
            self.editing = Some(id);
            ui.activated = true;
            #[cfg(any(target_os = "android", target_os = "ios"))]
            {
                self.keyboard_pan = BoardPan::default();
            }
            platform::editor_open(
                &self.fields[id],
                if id == 2 && self.game == GameKind::Wolves {
                    5
                } else {
                    id
                },
                rect,
                if id == 0 {
                    24
                } else if id == 1 {
                    6
                } else {
                    160
                },
            );
        }
    }
    pub fn draw(&mut self, ui: &mut Ui, press: Option<Vec2>) -> bool {
        let back = self.draw_page(ui, press);
        if ui.activated {
            self.revision += 1;
        }
        back
    }
    fn draw_page(&mut self, ui: &mut Ui, press: Option<Vec2>) -> bool {
        let (accent, panel, line) = style::palette(self.game, ui.theme.saver);
        ui.theme.accent = accent;
        ui.theme.panel = panel;
        ui.theme.line = line;
        let width = (screen_width() - 32.).min(1100.);
        let x = (screen_width() - width) / 2.;
        let header = Rect::new(x, 8., width, 44.);
        if ui.icon_button(Icon::Back, Rect::new(x, 8., 44., 44.), false) && self.back() {
            return true;
        }
        style::emblem(self.game, vec2(x + 64., 29.), 10., ui.theme.accent);
        fit(
            ui,
            self.game.title(),
            Rect::new(x + 80., 7., width - 180., 28.),
            21.,
            ui.theme.text,
            true,
        );
        if let Some(room) = &self.room {
            fit(
                ui,
                &room.code,
                Rect::new(x + 80., 32., width - 180., 16.),
                11.,
                ui.theme.muted,
                true,
            );
        }
        if ui.button("?", Rect::new(header.right() - 44., 8., 44., 44.), false) {
            self.help = !self.help;
            self.pan = BoardPan::default();
            self.close_editor();
        }
        if let Some(room) = &self.room
            && ui.button("↗", Rect::new(header.right() - 94., 8., 44., 44.), false)
        {
            platform::copy_invite(self.game, &room.code);
            self.error =
                "Invite link copied when clipboard access is available. Share the room code too."
                    .into();
        }
        #[cfg(any(target_os = "android", target_os = "ios"))]
        if self.editing.is_some() {
            self.draw_keyboard(ui, press, x, width);
            return false;
        }
        if self.help {
            self.draw_help(ui, press, x, width);
            return false;
        }
        if self.confirm_leave {
            let panel = Rect::new(x, screen_height() * 0.35, width, 180.);
            bordered(panel, 20., ui.theme.line, ui.theme.bg);
            ui.centered(
                "Leave this table?",
                Rect::new(panel.x, panel.y + 16., panel.w, 36.),
                21.,
                ui.theme.text,
                true,
            );
            ui.centered(
                if self
                    .room
                    .as_ref()
                    .is_some_and(|r| r.court.is_some() || r.reverie.is_some() || r.wolves.is_some())
                {
                    "Leaving ends your participation."
                } else {
                    "Your seat will be removed."
                },
                Rect::new(panel.x, panel.y + 56., panel.w, 30.),
                14.,
                ui.theme.muted,
                false,
            );
            if ui.button(
                "Stay",
                Rect::new(panel.x + 12., panel.y + 112., (panel.w - 36.) / 2., 48.),
                false,
            ) {
                self.confirm_leave = false;
            }
            if ui.button(
                "Leave",
                Rect::new(
                    panel.center().x + 6.,
                    panel.y + 112.,
                    (panel.w - 36.) / 2.,
                    48.,
                ),
                true,
            ) {
                self.network.send(&ClientMessage::Leave);
                self.confirm_leave = false;
            }
            return false;
        }
        let room = self.room.clone();
        if let Some(room) = room {
            if !self.connected {
                let r = Rect::new(x, 64., width, 48.);
                if ui.button(
                    if self.connecting {
                        "Connecting…"
                    } else {
                        "Reconnect"
                    },
                    r,
                    true,
                ) && !self.connecting
                {
                    self.reconnect();
                }
                fit(
                    ui,
                    &self.error,
                    Rect::new(x, 122., width, 52.),
                    14.,
                    CORAL,
                    false,
                );
                return false;
            }
            if let Some(game) = &room.court {
                self.draw_court(ui, press, &room, game, x, width);
            } else if let Some(game) = &room.reverie {
                self.draw_reverie(ui, press, &room, game, x, width);
            } else if let Some(game) = &room.wolves {
                let out = self.wolves_ui.draw(
                    ui,
                    press,
                    &room,
                    game,
                    (&self.fields[2], self.editing == Some(2)),
                    Rect::new(x, 66., width, screen_height() - 82.),
                );
                self.wolves_outcome(out);
                self.error_banner(ui, x, width);
            } else if self.wolves_ui.setup.is_some() {
                let out = self.wolves_ui.draw_setup(ui, press, &room, x, width);
                self.wolves_outcome(out);
            } else {
                self.draw_lobby(ui, press, &room, x, width);
            }
        } else {
            self.draw_join(ui, press, x, width);
        }
        false
    }
    fn wolves_outcome(&mut self, out: crate::wolves_view::Outcome) {
        if let Some(rect) = out.editor {
            self.editing = Some(2);
            platform::editor_open(&self.fields[2], 5, rect, 160);
        }
        if let Some(command) = out.command {
            self.play(command);
        }
        if out.clear_chat {
            self.fields[2].clear();
        }
    }
    fn draw_join(&mut self, ui: &mut Ui, press: Option<Vec2>, x: f32, width: f32) {
        let wide = width >= 800.;
        let compact = screen_height() < 650.;
        let w = if wide { 400. } else { width.min(440.) };
        let form_x = if wide {
            x + width * 0.54
        } else {
            x + (width - w) / 2.
        };
        let hero_h = if wide {
            360.
        } else if compact {
            146.
        } else {
            248.
        };
        let total = if wide { 510. } else { hero_h + 390. };
        let viewport = Rect::new(x, 66., width, screen_height() - 82.);
        let (origin, tap) = page_input(
            &mut self.pan,
            &mut self.blocked_touch,
            press,
            viewport,
            total,
        );
        let original_pointer = ui.override_pointer(tap);
        clip(Some(viewport));
        let hero = if wide {
            Rect::new(x, origin.y + 40., width * 0.47, hero_h)
        } else {
            Rect::new(form_x, origin.y, w, hero_h)
        };
        let art_h = if compact && !wide {
            76.
        } else if wide {
            200.
        } else {
            138.
        };
        let art_w = art_h * 1.7;
        let art = Rect::new(hero.center().x - art_w / 2., hero.y + 10., art_w, art_h);
        if !ui.theme.saver {
            rounded(
                Rect::new(art.x - 16., art.y + 8., art.w + 32., art.h - 8.),
                art_h * 0.45,
                ui.theme.panel,
            );
            style::spark(vec2(art.x + 12., art.y + 18.), 8., ui.theme.accent);
            style::spark(
                vec2(art.right() - 4., art.bottom() - 20.),
                5.,
                ui.theme.accent,
            );
        }
        if self.game == GameKind::Court {
            card_art::preview(ui, art);
        } else if self.game == GameKind::Wolves {
            crate::wolves_art::preview(ui, art);
        } else {
            self.art.preview(ui, art);
        }
        let title_y = art.bottom() + 12.;
        ui.centered(
            self.game.title(),
            Rect::new(hero.x, title_y, hero.w, if compact { 32. } else { 48. }),
            if compact { 28. } else { 40. },
            ui.theme.text,
            true,
        );
        let tagline = if self.game == GameKind::Court {
            "A little charm. A lot of bluff."
        } else if self.game == GameKind::Wolves {
            "Friendly faces. Hidden fangs."
        } else {
            "One picture. A thousand stories."
        };
        if !compact || wide {
            ui.centered(
                tagline,
                Rect::new(hero.x, title_y + 51., hero.w, 24.),
                14.,
                ui.theme.muted,
                false,
            );
        }
        let y = origin.y + if wide { 64. } else { hero_h };
        bordered(
            Rect::new(
                form_x,
                y,
                w,
                if self.session().is_some() { 384. } else { 320. },
            ),
            24.,
            ui.theme.line,
            ui.theme.bg,
        );
        let fx = form_x + 20.;
        let fw = w - 40.;
        ui.label("YOUR NAME", fx, y + 28., 11., ui.theme.muted);
        self.field(
            ui,
            0,
            Rect::new(fx, y + 40., fw, 50.),
            "What should we call you?",
            tap,
        );
        let enabled = !self.fields[0].is_empty() && !self.connecting;
        if primary(
            ui,
            if self.connecting {
                "Connecting…"
            } else {
                if self.game == GameKind::Wolves {
                    "Create a village"
                } else {
                    "Create a table"
                }
            },
            Rect::new(fx, y + 104., fw, 52.),
            enabled,
        ) {
            self.persist();
            self.connect(ClientMessage::Create {
                game: self.game,
                name: self.fields[0].clone(),
            });
        }
        let (min, max) = self.game.limits();
        ui.centered(
            &format!("{min}–{max} friends · online"),
            Rect::new(fx, y + 163., fw, 22.),
            12.,
            ui.theme.muted,
            false,
        );
        let mid = fx + fw / 2.;
        draw_line(fx, y + 207., mid - 58., y + 207., 1., ui.theme.line);
        draw_line(mid + 58., y + 207., fx + fw, y + 207., 1., ui.theme.line);
        ui.centered(
            "or join friends",
            Rect::new(mid - 56., y + 194., 112., 24.),
            11.,
            ui.theme.muted,
            false,
        );
        self.field(
            ui,
            1,
            Rect::new(fx, y + 231., fw - 88., 50.),
            "Room code",
            tap,
        );
        if primary(
            ui,
            "Join",
            Rect::new(fx + fw - 78., y + 231., 78., 50.),
            enabled && self.fields[1].len() == 6,
        ) {
            self.persist();
            self.connect(ClientMessage::Join {
                room: self.fields[1].clone(),
                name: self.fields[0].clone(),
            });
        }
        if let Some(session) = self.session().cloned()
            && ui.button(
                &format!("Back to {}", session.room),
                Rect::new(fx, y + 307., fw, 48.),
                false,
            )
            && !self.connecting
        {
            self.reconnect();
        }
        clip(None);
        ui.override_pointer(original_pointer);
        self.error_banner(ui, x, width);
    }
    fn draw_lobby(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        room: &RoomView,
        x: f32,
        width: f32,
    ) {
        let original_width = width;
        let width = width.min(720.);
        let x = x + (original_width - width) / 2.;
        let cols = if width >= 340. { 2 } else { 1 };
        let count = room.members.len().max(self.game.limits().0);
        let row_h = if cols == 2 { 94. } else { 68. };
        let roster_h = count.div_ceil(cols) as f32 * row_h;
        let role_h = if self.game == GameKind::Wolves {
            58.
        } else {
            0.
        };
        let total = 158. + role_h + roster_h + 196.;
        let viewport = Rect::new(x, 66., width, screen_height() - 82.);
        let (origin, tap) = page_input(
            &mut self.pan,
            &mut self.blocked_touch,
            press,
            viewport,
            total,
        );
        clip(Some(viewport));
        let ticket = Rect::new(x, origin.y + 4., width, 120.);
        bordered(ticket, 24., ui.theme.line, ui.theme.panel);
        ui.centered(
            if self.game == GameKind::Wolves {
                "YOUR VILLAGE"
            } else {
                "YOUR TABLE"
            },
            Rect::new(x, ticket.y + 12., width, 20.),
            10.,
            ui.theme.muted,
            true,
        );
        let code_w = (width - 48.).min(264.);
        let cell = code_w / 6.;
        let code_x = ticket.center().x - code_w / 2.;
        for (i, c) in room.code.chars().enumerate() {
            let r = Rect::new(code_x + i as f32 * cell, ticket.y + 38., cell - 5., 38.);
            rounded(r, 9., ui.theme.bg);
            ui.centered(&c.to_string(), r, 23., ui.theme.accent, true);
        }
        ui.centered(
            if self.game == GameKind::Wolves {
                "Share the code. Keep your role secret."
            } else {
                "Share the code. Pull up a seat."
            },
            Rect::new(x, ticket.y + 84., width, 24.),
            13.,
            ui.theme.muted,
            false,
        );
        for side in [x, x + width] {
            draw_circle(side, ticket.center().y, 7., ui.theme.bg);
        }
        if self.game == GameKind::Wolves {
            let preset = room
                .wolves_setup
                .as_ref()
                .map_or(wolves::Preset::Advanced, |s| s.preset);
            if button(
                ui,
                &format!("Roles · {:?}  ↗", preset),
                Rect::new(x, origin.y + 134., width, 44.),
                false,
                tap,
                viewport,
            ) {
                self.wolves_ui.open_setup(room);
            }
        }
        let gap = 10.;
        let sw = (width - gap * (cols - 1) as f32) / cols as f32;
        for i in 0..count {
            let r = Rect::new(
                x + (i % cols) as f32 * (sw + gap),
                origin.y + 146. + role_h + (i / cols) as f32 * row_h,
                sw,
                row_h - 10.,
            );
            bordered(r, 18., ui.theme.line, ui.theme.bg);
            if let Some(m) = room.members.get(i) {
                let avatar = vec2(r.x + 27., r.y + 25.);
                if self.game == GameKind::Wolves {
                    crate::wolves_art::portrait(avatar, 18., i, None, ui.theme.saver);
                } else {
                    style::avatar(ui, &m.name, i, avatar, 17.);
                }
                fit(
                    ui,
                    &m.name,
                    Rect::new(r.x + 51., r.y + 9., r.w - 59., 24.),
                    14.,
                    ui.theme.text,
                    true,
                );
                if cols == 2 {
                    ui.label(
                        if i == room.you {
                            "YOU"
                        } else if i == room.host {
                            "HOST"
                        } else {
                            "PLAYER"
                        },
                        r.x + 16.,
                        r.y + 65.,
                        9.,
                        ui.theme.muted,
                    );
                }
                let status = if !m.connected {
                    "Offline"
                } else if m.ready {
                    "Ready"
                } else {
                    "Getting ready"
                };
                let sy = if cols == 2 { r.y + 51. } else { r.y + 31. };
                fit(
                    ui,
                    status,
                    Rect::new(
                        r.x + if cols == 2 { r.w * 0.34 } else { 51. },
                        sy,
                        if cols == 2 {
                            r.w * 0.64 - 10.
                        } else {
                            r.w - 60.
                        },
                        20.,
                    ),
                    11.,
                    if m.ready {
                        ui.theme.accent
                    } else {
                        ui.theme.muted
                    },
                    m.ready,
                );
                if m.ready {
                    style::check(vec2(r.right() - 15., r.y + 16.), 6., ui.theme.accent);
                }
            } else {
                draw_circle_lines(r.x + 27., r.y + 25., 16., 1., ui.theme.line);
                fit(
                    ui,
                    "Open seat",
                    Rect::new(r.x + 49., r.y + 10., r.w - 57., 24.),
                    13.,
                    ui.theme.muted,
                    false,
                );
            }
        }
        let y = origin.y + 146. + role_h + roster_h + 8.;
        let me = &room.members[room.you];
        if button(
            ui,
            if me.ready {
                "✓  Ready"
            } else {
                "I’m ready"
            },
            Rect::new(x, y, width, 48.),
            !me.ready,
            tap,
            viewport,
        ) {
            self.play(Command::Ready(!me.ready));
        }
        if room.host == room.you {
            let can_start = room.members.len() >= self.game.limits().0
                && room.members.iter().all(|m| m.ready && m.connected)
                && room
                    .wolves_setup
                    .as_ref()
                    .is_none_or(|s| s.roles_for(room.members.len()).is_ok());
            let pointer = ui.override_pointer(tap);
            if primary(
                ui,
                "Let’s play",
                Rect::new(x, y + 60., width, 48.),
                can_start,
            ) {
                self.play(Command::Start);
            }
            ui.override_pointer(pointer);
        } else {
            ui.centered(
                "The host starts when everyone is ready",
                Rect::new(x, y + 60., width, 36.),
                12.,
                ui.theme.muted,
                false,
            );
        }
        if button(
            ui,
            if self.game == GameKind::Wolves {
                "Leave village"
            } else {
                "Leave table"
            },
            Rect::new(x, y + 120., width, 44.),
            false,
            tap,
            viewport,
        ) {
            self.confirm_leave = true;
        }
        clip(None);
        self.error_banner(ui, x, width);
    }
    fn error_banner(&self, ui: &Ui, x: f32, width: f32) {
        if !self.error.is_empty() {
            let r = Rect::new(x, screen_height() - 54., width, 44.);
            rounded(r, 12., ui.theme.bg);
            fit(
                ui,
                &self.error,
                Rect::new(x + 8., r.y, width - 16., 44.),
                13.,
                CORAL,
                false,
            );
        }
    }
    fn draw_court(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        room: &RoomView,
        g: &coup::View,
        x: f32,
        width: f32,
    ) {
        if let Some(player) = self.preview.and_then(|i| g.players.get(usize::from(i))) {
            ui.centered(
                &format!("{} · {} coins", player.name, player.coins),
                Rect::new(x, 66., width, 36.),
                21.,
                ui.theme.text,
                true,
            );
            let h = (screen_height() - 220.).clamp(120., 360.);
            let w = (h / 1.5).min((width - 12.) / 2.);
            for (i, c) in player.cards.iter().enumerate() {
                let r = Rect::new(
                    screen_width() / 2. - w - 6. + i as f32 * (w + 12.),
                    114.,
                    w,
                    w * 1.5,
                );
                card_art::court_card(ui, r, c.role, c.revealed, false);
                if let Some(role) = c.role {
                    wrap(
                        ui,
                        role.power(),
                        Rect::new(r.x, r.bottom() + 12., r.w, 56.),
                        13.,
                        ui.theme.text,
                        false,
                    );
                }
            }
            if ui.button(
                "Close",
                Rect::new(x, screen_height() - 68., width, 48.),
                false,
            ) {
                self.preview = None;
            }
            return;
        }
        let desktop = width >= 800.;
        let cols = if desktop { 3 } else { 2 };
        let rowh = 70.;
        let scores = g.players.len().div_ceil(cols) as f32 * rowh;
        let handh: f32 = if desktop {
            240.
        } else if screen_height() < 600. {
            128.
        } else {
            156.
        };
        let actions = if self.target.is_some() {
            g.players.len()
        } else if !g.exchange.is_empty() {
            1
        } else {
            g.actions.len().max(g.choices.len()).max(1)
        };
        let gridcols = 2;
        let actionheight = actions.div_ceil(gridcols) as f32 * 66.;
        let hand_width = if desktop { width * 0.45 } else { width };
        let action_x = if desktop { x + hand_width + 28. } else { x };
        let action_width = if desktop {
            width - hand_width - 28.
        } else {
            width
        };
        let total = scores
            + 76.
            + if desktop {
                handh.max(actionheight) + 216.
            } else {
                handh + actionheight + 226.
            };
        let viewport = Rect::new(x, 66., width, screen_height() - 82.);
        let (origin, tap) = page_input(
            &mut self.pan,
            &mut self.blocked_touch,
            press,
            viewport,
            total,
        );
        clip(Some(viewport));
        for (i, p) in g.players.iter().enumerate() {
            let rect = Rect::new(
                x + (i % cols) as f32 * (width / cols as f32),
                origin.y + (i / cols) as f32 * rowh,
                width / cols as f32 - 8.,
                60.,
            );
            let active = i == g.turn && g.winner.is_none();
            bordered(
                rect,
                18.,
                if active {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                if active { ui.theme.panel } else { ui.theme.bg },
            );
            style::avatar(ui, &p.name, i, vec2(rect.x + 24., rect.y + 23.), 15.);
            fit(
                ui,
                &p.name,
                Rect::new(rect.x + 45., rect.y + 7., rect.w - 58., 24.),
                14.,
                ui.theme.text,
                true,
            );
            style::coin(vec2(rect.x + 50., rect.y + 44.), 6.);
            ui.heading(
                &p.coins.to_string(),
                rect.x + 61.,
                rect.y + 48.,
                12.,
                ui.theme.text,
            );
            if i == room.you {
                ui.label("YOU", rect.x + 12., rect.y + 52., 8., ui.theme.muted);
            }
            for (j, c) in p.cards.iter().enumerate() {
                let r = Rect::new(rect.right() - 42. + j as f32 * 17., rect.y + 36., 12., 17.);
                bordered(
                    r,
                    3.,
                    if c.revealed {
                        ui.theme.line
                    } else {
                        ui.theme.accent
                    },
                    if c.revealed {
                        ui.theme.bg
                    } else {
                        ui.theme.accent
                    },
                );
                if !c.revealed {
                    style::spark(r.center(), 3., if ui.theme.saver { BLACK } else { WHITE });
                }
            }
            if !room.members[i].connected {
                draw_circle(rect.right() - 10., rect.y + 10., 3., CORAL);
            }
            if rect.overlaps(&viewport)
                && (tap.is_some_and(|point| rect.contains(point)) || ui.keyboard_hit(rect))
            {
                self.preview = Some(i as u8);
                ui.activated = true;
            }
        }
        let y = origin.y + scores + 4.;
        let banner = Rect::new(x, y, width, 56.);
        rounded(banner, 18., ui.theme.panel);
        let my_move = !g.actions.is_empty() || !g.choices.is_empty() || !g.exchange.is_empty();
        let phase = if g.winner.is_some() {
            "THE CROWN IS CLAIMED"
        } else if my_move {
            "YOUR MOVE"
        } else {
            "AT THE TABLE"
        };
        ui.label(phase, x + 16., y + 17., 9., ui.theme.accent);
        let prompt = self
            .target
            .map(|action| format!("Choose a target · {}", action.title()));
        fit(
            ui,
            prompt.as_deref().unwrap_or(&g.prompt),
            Rect::new(x + 12., y + 23., width - 24., 27.),
            16.,
            ui.theme.text,
            true,
        );
        let pool: Vec<_> = if !g.exchange.is_empty() {
            g.exchange
                .iter()
                .map(|r| coup::CardView {
                    role: Some(*r),
                    revealed: false,
                })
                .collect()
        } else {
            g.players[room.you].cards.clone()
        };
        let hand_y = y + 68.;
        let table = Rect::new(x, hand_y, hand_width, handh + 16.);
        bordered(table, 28., ui.theme.line, ui.theme.panel);
        let cw = ((hand_width - 32.) / pool.len() as f32 - 10.).min(handh / 1.5);
        let ch = cw * 1.5;
        let start = x + (hand_width - (cw + 10.) * pool.len() as f32 + 10.) / 2.;
        for (i, c) in pool.iter().enumerate() {
            let r = Rect::new(
                start + i as f32 * (cw + 10.),
                hand_y + 8. + (handh - ch) / 2.,
                cw,
                ch,
            );
            if !ui.theme.saver {
                rounded(Rect::new(r.x, r.y + 3., r.w, r.h), 12., ui.theme.line);
            }
            card_art::court_card(ui, r, c.role, c.revealed, false);
            if self.exchange.contains(&i) {
                bordered(
                    Rect::new(r.right() - 23., r.y + 7., 18., 18.),
                    9.,
                    ui.theme.accent,
                    ui.theme.accent,
                );
                style::check(
                    vec2(r.right() - 14., r.y + 16.),
                    7.,
                    if ui.theme.saver { BLACK } else { WHITE },
                );
            }
            if !g.exchange.is_empty()
                && r.overlaps(&viewport)
                && (tap.is_some_and(|p| r.contains(p)) || ui.keyboard_hit(r))
            {
                if let Some(at) = self.exchange.iter().position(|p| *p == i) {
                    self.exchange.remove(at);
                } else if self.exchange.len() < g.keep {
                    self.exchange.push(i);
                }
                ui.activated = true;
            }
        }
        ui.centered(
            if g.exchange.is_empty() {
                "Your secret influences"
            } else {
                "Choose the influences to keep"
            },
            Rect::new(x, table.bottom() + 4., hand_width, 24.),
            11.,
            ui.theme.muted,
            false,
        );
        let ay = if desktop {
            hand_y
        } else {
            table.bottom() + 38.
        };
        let mut moves: Vec<(String, Option<Command>)> = vec![];
        if let Some(action) = self.target {
            for (i, p) in g.players.iter().enumerate() {
                if i != room.you && p.cards.iter().any(|c| !c.revealed) {
                    moves.push((
                        p.name.clone(),
                        Some(Command::Court(coup::Move::Act {
                            action,
                            target: Some(i),
                        })),
                    ));
                }
            }
            moves.push(("Cancel".into(), None));
        } else if !g.exchange.is_empty() {
            moves.push((
                format!("Keep {} / {}", self.exchange.len(), g.keep),
                Some(Command::Court(coup::Move::Keep {
                    cards: self.exchange.clone(),
                })),
            ));
        } else if !g.actions.is_empty() {
            for &action in &g.actions {
                moves.push((
                    format!(
                        "{}{}",
                        action.title(),
                        match action {
                            coup::Action::Income => " +1",
                            coup::Action::Aid => " +2",
                            coup::Action::Tax => " +3",
                            coup::Action::Steal => " 2",
                            coup::Action::Assassinate => " · 3",
                            coup::Action::Coup => " · 7",
                            _ => "",
                        }
                    ),
                    Some(Command::Court(coup::Move::Act {
                        action,
                        target: None,
                    })),
                ));
            }
        } else {
            moves.extend(
                g.choices
                    .iter()
                    .map(|c| (c.label.clone(), Some(Command::Court(c.command.clone())))),
            );
        }
        if g.winner.is_some() && room.you == room.host {
            moves.push(("Play again".into(), Some(Command::Rematch)));
        }
        for (i, (label, command)) in moves.iter().enumerate() {
            let w = (action_width - 10. * (gridcols - 1) as f32) / gridcols as f32;
            let r = Rect::new(
                action_x + (i % gridcols) as f32 * (w + 10.),
                ay + (i / gridcols) as f32 * 66.,
                w,
                56.,
            );
            let hit = if let Some(Command::Court(coup::Move::Act {
                action,
                target: None,
            })) = command
            {
                style::action_tile(ui, *action, r, tap, viewport)
            } else if let Some(Command::Court(coup::Move::Act {
                target: Some(index),
                ..
            })) = command
            {
                bordered(r, 16., ui.theme.line, ui.theme.panel);
                style::avatar(ui, label, *index, vec2(r.x + 23., r.center().y), 14.);
                fit(
                    ui,
                    label,
                    Rect::new(r.x + 44., r.y, r.w - 52., r.h),
                    14.,
                    ui.theme.text,
                    true,
                );
                let hit = r.overlaps(&viewport)
                    && (tap.is_some_and(|p| r.contains(p)) || ui.keyboard_hit(r));
                ui.activated |= hit;
                hit
            } else {
                button(
                    ui,
                    label,
                    r,
                    !matches!(command, None | Some(Command::Court(coup::Move::Pass))),
                    tap,
                    viewport,
                )
            };
            if hit {
                match command {
                    Some(Command::Court(coup::Move::Act {
                        action,
                        target: None,
                    })) if action.targeted() => self.target = Some(*action),
                    Some(Command::Court(coup::Move::Keep { cards })) if cards.len() != g.keep => {}
                    Some(c) => self.play(c.clone()),
                    None => self.target = None,
                };
            }
        }
        let ly = (ay + moves.len().max(1).div_ceil(gridcols) as f32 * 66.)
            .max(table.bottom() + 32.)
            + 16.;
        ui.label("RECENT MOVES", x + 8., ly + 10., 9., ui.theme.muted);
        for (i, event) in g.log.iter().rev().take(3).enumerate() {
            fit(
                ui,
                event,
                Rect::new(x, ly + 20. + i as f32 * 26., width, 24.),
                12.,
                ui.theme.muted,
                false,
            );
        }
        if button(
            ui,
            "Leave table",
            Rect::new(x + (width - 140.) / 2., ly + 108., 140., 44.),
            false,
            tap,
            viewport,
        ) {
            self.confirm_leave = true;
        }
        clip(None);
        self.error_banner(ui, x, width);
    }
    fn draw_reverie(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        room: &RoomView,
        g: &reverie::View,
        x: f32,
        width: f32,
    ) {
        if self.clue_open {
            self.draw_clue(ui, press, g, x, width);
            return;
        }
        if let Some(card) = self.preview {
            self.card_modal(ui, g, card, x, width);
            return;
        }
        let short = screen_height() < 600.;
        let landscape = short && screen_width() > screen_height() * 1.3;
        let side = if landscape {
            (width * 0.35).clamp(170., 240.)
        } else {
            width
        };
        let cols = if landscape {
            if g.players.len() > 4 { 2 } else { 1 }
        } else if short || width > 760. {
            4
        } else {
            2
        };
        let sw = (side - 6. * (cols - 1) as f32) / cols as f32;
        let rh = if short { 24. } else { 40. };
        for (i, p) in g.players.iter().enumerate() {
            let r = Rect::new(
                x + (i % cols) as f32 * (sw + 6.),
                64. + (i / cols) as f32 * rh,
                sw,
                rh - 6.,
            );
            let teller = i == g.storyteller;
            bordered(
                r,
                12.,
                if teller {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                if teller { ui.theme.panel } else { ui.theme.bg },
            );
            let left = if short { 6. } else { 30. };
            if !short {
                if teller {
                    style::spark(vec2(r.x + 15., r.center().y), 7., ui.theme.accent);
                } else {
                    style::avatar(ui, &p.name, i, vec2(r.x + 15., r.center().y), 10.);
                }
            }
            let compact_name = if short && sw < 90. {
                format!("{}{}", p.name.chars().next().unwrap_or('?'), i + 1)
            } else {
                p.name.clone()
            };
            fit(
                ui,
                &compact_name,
                Rect::new(r.x + left, r.y, r.w - left - 34., r.h),
                if short { 10. } else { 12. },
                ui.theme.text,
                teller,
            );
            let score = if p.gained > 0 {
                format!("{} +{}", p.score, p.gained)
            } else {
                p.score.to_string()
            };
            fit(
                ui,
                &score,
                Rect::new(r.right() - 38., r.y, 33., r.h),
                if short { 11. } else { 14. },
                ui.theme.accent,
                true,
            );
            if p.done && !teller {
                style::check(vec2(r.right() - 4., r.y + 4.), 4., ui.theme.accent);
            }
        }
        let progress_y = 64. + g.players.len().div_ceil(cols) as f32 * rh + 6.;
        let step = match g.phase.as_str() {
            "story" => 0,
            "submit" => 1,
            "vote" => 2,
            _ => 3,
        };
        let round_w = if short { 40. } else { 58. };
        ui.centered(
            &format!("R{}", g.round),
            Rect::new(x, progress_y, round_w, 24.),
            12.,
            ui.theme.accent,
            true,
        );
        let pw = (side - round_w - 6.) / 3.;
        for (i, label) in ["Clue", "Cards", if step == 3 { "Reveal" } else { "Vote" }]
            .iter()
            .enumerate()
        {
            let r = Rect::new(
                x + round_w + i as f32 * pw,
                progress_y,
                (pw - 4.).max(16.),
                24.,
            );
            rounded(
                r,
                12.,
                if i == step.min(2) {
                    ui.theme.panel
                } else {
                    ui.theme.bg
                },
            );
            if i == step.min(2) && r.w > 60. {
                draw_circle(r.x + 8., r.center().y, 2., ui.theme.accent);
            }
            fit(
                ui,
                label,
                Rect::new(r.x + 6., r.y, r.w - 12., r.h),
                10.,
                if i == step.min(2) {
                    ui.theme.accent
                } else {
                    ui.theme.muted
                },
                i == step.min(2),
            );
        }
        let top = progress_y + if short { 30. } else { 38. };
        let status = match g.phase.as_str() {
            "story" => {
                if g.can_play {
                    "Choose a card. Give a clue.".into()
                } else {
                    format!("{} is finding a story", g.players[g.storyteller].name)
                }
            }
            "submit" => {
                if g.can_play {
                    format!(
                        "Choose {} matching card{}",
                        g.pick,
                        if g.pick > 1 { "s" } else { "" }
                    )
                } else {
                    "Waiting for cards".into()
                }
            }
            "vote" => {
                if g.can_play {
                    "Which card inspired the clue?".into()
                } else {
                    "Waiting for votes".into()
                }
            }
            "results" => "The story, revealed".into(),
            _ => g.ended.clone().unwrap_or_else(|| {
                format!(
                    "{} wins",
                    g.winners
                        .iter()
                        .map(|&i| g.players[i].name.clone())
                        .collect::<Vec<_>>()
                        .join(" & ")
                )
            }),
        };
        fit(
            ui,
            &status,
            Rect::new(x, top, side, if short { 22. } else { 30. }),
            if short { 14. } else { 18. },
            ui.theme.text,
            true,
        );
        let clue_offset = if short { 28. } else { 38. };
        let below = if g.phase == "story" && g.can_play {
            self.field(
                ui,
                2,
                Rect::new(x, top + clue_offset, side, 48.),
                "Your clue",
                press,
            );
            top + clue_offset + 60.
        } else if !g.clue.is_empty() {
            let clue = Rect::new(x, top + clue_offset, side, 54.);
            bordered(clue, 16., ui.theme.line, ui.theme.panel);
            let text = format!("“{}”", g.clue);
            let size = if short { 13. } else { 16. };
            if ui.text_width(&text, size, true) <= clue.w - 36. {
                ui.centered(
                    &text,
                    Rect::new(clue.x + 14., clue.y, clue.w - 28., clue.h),
                    size,
                    ui.theme.accent,
                    true,
                );
            } else {
                wrap(
                    ui,
                    &text,
                    Rect::new(clue.x + 14., clue.y + 2., clue.w - 38., clue.h - 4.),
                    size,
                    ui.theme.accent,
                    true,
                );
                ui.label(
                    "↗",
                    clue.right() - 18.,
                    clue.bottom() - 8.,
                    10.,
                    ui.theme.accent,
                );
            }
            if ui.hit(clue) {
                self.clue_open = true;
                self.pan = BoardPan::default();
            }
            top + clue_offset + 66.
        } else {
            top + if short { 30. } else { 42. }
        };
        let gy = if landscape { 66. } else { below };
        let gx = if landscape { x + side + 16. } else { x };
        let gw = if landscape { width - side - 16. } else { width };
        let cards: Vec<u8> = if matches!(g.phase.as_str(), "vote" | "results" | "finished") {
            g.table.iter().map(|c| c.card).collect()
        } else {
            g.hand.clone()
        };
        self.art.update(&cards);
        let viewport = Rect::new(gx, gy, gw, (screen_height() - gy - 82.).max(44.));
        let gc = if landscape {
            (gw / 80.).floor().max(2.) as usize
        } else if width >= 1000. {
            5
        } else if width >= 650. {
            4
        } else {
            2
        };
        let gap = 12.;
        let cw = (gw - gap * (gc - 1) as f32) / gc as f32;
        let ch = cw * 1.5;
        let caption = if g.phase == "results" || g.phase == "finished" {
            54.
        } else {
            32.
        };
        let total = cards.len().div_ceil(gc) as f32 * (ch + caption + gap);
        let (origin, tap) = page_input(
            &mut self.pan,
            &mut self.blocked_touch,
            press,
            viewport,
            total,
        );
        clip(Some(viewport));
        for (i, &card) in cards.iter().enumerate() {
            let r = Rect::new(
                gx + (i % gc) as f32 * (cw + gap),
                origin.y + (i / gc) as f32 * (ch + caption + gap),
                cw,
                ch,
            );
            if !r.overlaps(&viewport) {
                continue;
            }
            let table = g.table.iter().find(|c| c.card == card);
            let marked = self.selected.contains(&card) || g.vote == Some(card);
            let story = table.is_some_and(|c| c.story);
            if !ui.theme.saver {
                rounded(Rect::new(r.x, r.y + 3., r.w, r.h), 16., ui.theme.line);
            }
            rounded(
                r,
                16.,
                if marked || story {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
            );
            let frame = Rect::new(
                r.x + if marked || story { 3. } else { 1. },
                r.y + if marked || story { 3. } else { 1. },
                r.w - if marked || story { 6. } else { 2. },
                r.h - if marked || story { 6. } else { 2. },
            );
            rounded(frame, 14., ui.theme.bg);
            self.art.draw(
                ui,
                card,
                Rect::new(r.x + 7., r.y + 7., r.w - 14., r.h - 14.),
            );
            let token = vec2(r.x + 20., r.y + 20.);
            draw_circle(
                token.x,
                token.y,
                12.,
                if ui.theme.saver { BLACK } else { WHITE },
            );
            ui.centered(
                &(i + 1).to_string(),
                Rect::new(token.x - 12., token.y - 12., 24., 24.),
                11.,
                ui.theme.accent,
                true,
            );
            if marked || story {
                let center = vec2(r.right() - 20., r.y + 20.);
                draw_circle(center.x, center.y, 12., ui.theme.accent);
                if story {
                    style::spark(center, 7., if ui.theme.saver { BLACK } else { WHITE });
                } else {
                    style::check(center, 8., if ui.theme.saver { BLACK } else { WHITE });
                }
            }
            let label = if let Some(owner) = table.and_then(|c| c.owner) {
                format!(
                    "{}{}",
                    g.players[owner].name,
                    if table.unwrap().story {
                        " · story"
                    } else {
                        ""
                    }
                )
            } else if table.is_some_and(|c| c.own) {
                "Your card".into()
            } else if marked {
                "Selected".into()
            } else {
                String::new()
            };
            fit(
                ui,
                &label,
                Rect::new(r.x, r.bottom() + 3., r.w, 24.),
                12.,
                ui.theme.muted,
                false,
            );
            if let Some(t) = table
                && t.owner.is_some()
            {
                let votes = t
                    .votes
                    .iter()
                    .map(|&i| g.players[i].name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                fit(
                    ui,
                    if votes.is_empty() { "No votes" } else { &votes },
                    Rect::new(r.x, r.bottom() + 28., r.w, 24.),
                    11.,
                    ui.theme.muted,
                    false,
                );
            }
            if tap.is_some_and(|p| r.contains(p)) || ui.keyboard_hit(r) {
                self.preview = Some(card);
                ui.activated = true;
            }
        }
        clip(None);
        let bottom = Rect::new(gx, screen_height() - 68., gw, 48.);
        let busy = self.sent.is_some();
        let label = if busy {
            "Sending…"
        } else {
            match g.phase.as_str() {
                "story" => {
                    if g.can_play {
                        "Tell story"
                    } else {
                        "A story is on its way…"
                    }
                }
                "submit" => {
                    if g.can_play {
                        "Submit cards"
                    } else {
                        "Cards are on their way…"
                    }
                }
                "vote" => {
                    if g.can_play {
                        "Confirm vote"
                    } else {
                        "Votes are on their way…"
                    }
                }
                "results" => {
                    if g.can_play {
                        "Next round"
                    } else {
                        "Waiting for others"
                    }
                }
                "finished" => {
                    if room.you == room.host {
                        "Play again"
                    } else {
                        "Waiting for host"
                    }
                }
                _ => "Waiting",
            }
        };
        let can = match g.phase.as_str() {
            "story" => g.can_play && self.selected.len() == 1 && !self.fields[2].is_empty(),
            "submit" => g.can_play && self.selected.len() == g.pick,
            "vote" => g.can_play && self.selected.len() == 1,
            "results" => g.can_play,
            "finished" => room.you == room.host,
            _ => false,
        };
        rounded(
            Rect::new(gx - 4., bottom.y - 10., gw + 8., 78.),
            20.,
            ui.theme.bg,
        );
        if primary(
            ui,
            label,
            Rect::new(bottom.x, bottom.y, bottom.w - 60., 48.),
            can && !busy,
        ) && can
            && !busy
        {
            let command = match g.phase.as_str() {
                "story" => Command::Reverie(reverie::Move::Story {
                    card: self.selected[0],
                    clue: self.fields[2].clone(),
                }),
                "submit" => Command::Reverie(reverie::Move::Submit {
                    cards: self.selected.clone(),
                }),
                "vote" => Command::Reverie(reverie::Move::Vote {
                    card: self.selected[0],
                }),
                "results" => Command::Reverie(reverie::Move::Next),
                _ => Command::Rematch,
            };
            self.play(command);
        }
        if ui.button(
            "×",
            Rect::new(bottom.right() - 48., bottom.y, 48., 48.),
            false,
        ) {
            self.confirm_leave = true;
        }
        if self.art.failed() && ui.button("Retry images", Rect::new(gx, gy, 144., 44.), false) {
            self.art.retry();
        }
        if let Some(card) = self.preview {
            self.card_modal(ui, g, card, x, width);
        }
        self.error_banner(ui, x, width);
    }
    fn draw_clue(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        g: &reverie::View,
        x: f32,
        width: f32,
    ) {
        let viewport = Rect::new(x, 66., width, screen_height() - 146.);
        let text = format!("“{}”", g.clue);
        let height = wrapped_lines(ui, &text, width - 24., 20.).len() as f32 * 29. + 32.;
        let (origin, _) = page_input(
            &mut self.pan,
            &mut self.blocked_touch,
            press,
            viewport,
            height,
        );
        clip(Some(viewport));
        wrap(
            ui,
            &text,
            Rect::new(x + 12., origin.y + 12., width - 24., height),
            20.,
            ui.theme.accent,
            true,
        );
        clip(None);
        if ui.button(
            "Close clue",
            Rect::new(x, screen_height() - 68., width, 48.),
            false,
        ) {
            self.clue_open = false;
            self.pan = BoardPan::default();
        }
    }
    fn card_modal(&mut self, ui: &mut Ui, g: &reverie::View, card: u8, x: f32, width: f32) {
        draw_rectangle(0., 56., screen_width(), screen_height() - 56., ui.theme.bg);
        ui.centered(
            "A closer look",
            Rect::new(x, 64., width, 26.),
            14.,
            ui.theme.muted,
            false,
        );
        let h = (screen_height() - 208.).clamp(90., 660.);
        let w = (h / 1.5).min(width - 16.);
        let r = Rect::new((screen_width() - w) / 2., 100., w, w * 1.5);
        bordered(
            Rect::new(r.x - 7., r.y - 7., r.w + 14., r.h + 14.),
            18.,
            ui.theme.line,
            ui.theme.bg,
        );
        self.art.draw(ui, card, r);
        let own = g.table.iter().any(|c| c.card == card && c.own);
        let can = g.can_play
            && matches!(g.phase.as_str(), "story" | "submit" | "vote")
            && !(g.phase == "vote" && own);
        let label = if self.selected.contains(&card) {
            "Remove"
        } else if own && g.phase == "vote" {
            "Your card"
        } else if g.phase == "vote" {
            "Select vote"
        } else {
            "Select card"
        };
        let by = screen_height() - 68.;
        let bw = (width - 12.) / 2.;
        if ui.button("Close", Rect::new(x, by, bw, 48.), false) {
            self.preview = None;
        }
        if primary(ui, label, Rect::new(x + bw + 12., by, bw, 48.), can) && can {
            if let Some(i) = self.selected.iter().position(|c| *c == card) {
                self.selected.remove(i);
            } else if g.phase == "submit" && g.pick == 2 {
                if self.selected.len() < 2 {
                    self.selected.push(card);
                }
            } else {
                self.selected = vec![card];
            }
            self.preview = None;
        }
    }
    /// Provide shared touch keys on native mobile where no browser IME is available.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn draw_keyboard(&mut self, ui: &mut Ui, press: Option<Vec2>, x: f32, width: f32) {
        let Some(id) = self.editing else {
            return;
        };
        let viewport = Rect::new(x, 66., width, screen_height() - 82.);
        let cols = (width / 48.).floor().max(4.) as usize;
        let keys = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
            .chars()
            .collect::<Vec<_>>();
        let rows = keys.len().div_ceil(cols);
        let height = 72. + rows as f32 * 54. + 120.;
        let (origin, tap) = page_input(
            &mut self.keyboard_pan,
            &mut self.blocked_touch,
            press,
            viewport,
            height,
        );
        clip(Some(viewport));
        fit(
            ui,
            if self.fields[id].is_empty() {
                [
                    "Your name",
                    "Room code",
                    if self.game == GameKind::Wolves {
                        "Your message"
                    } else {
                        "Your clue"
                    },
                ][id]
            } else {
                &self.fields[id]
            },
            Rect::new(x, origin.y, width, 52.),
            18.,
            ui.theme.text,
            true,
        );
        let w = (width - 6. * (cols - 1) as f32) / cols as f32;
        for (i, c) in keys.iter().enumerate() {
            let r = Rect::new(
                x + (i % cols) as f32 * (w + 6.),
                origin.y + 72. + (i / cols) as f32 * 54.,
                w,
                48.,
            );
            if button(ui, &c.to_string(), r, false, tap, viewport)
                && self.fields[id].chars().count()
                    < if id == 2 {
                        160
                    } else if id == 1 {
                        6
                    } else {
                        24
                    }
            {
                self.fields[id].push(*c);
                self.revision += 1;
            }
        }
        let y = origin.y + 72. + rows as f32 * 54.;
        let half = (width - 10.) / 2.;
        if button(
            ui,
            "Space",
            Rect::new(x, y, half, 48.),
            false,
            tap,
            viewport,
        ) && id != 1
        {
            self.fields[id].push(' ');
            self.revision += 1;
        }
        if button(
            ui,
            "⌫",
            Rect::new(x + half + 10., y, half, 48.),
            false,
            tap,
            viewport,
        ) {
            self.fields[id].pop();
            self.revision += 1;
        }
        if button(
            ui,
            "Done",
            Rect::new(x, y + 60., width, 48.),
            true,
            tap,
            viewport,
        ) {
            self.close_editor();
        }
        clip(None);
    }
    fn draw_help(&mut self, ui: &mut Ui, press: Option<Vec2>, x: f32, width: f32) {
        let viewport = Rect::new(x, 66., width, screen_height() - 82.);
        let lines: Vec<String> = if self.game == GameKind::Court {
            vec!["Two influences. Be the last player with one hidden.".into(),"Claim any role, even when you are bluffing. Everyone may challenge a claim before allowing it. A truthful claim replaces the shown card; the challenger loses an influence. A bluff loses an influence and the action fails.".into(),"Income: +1, cannot be challenged or blocked. Foreign aid: +2, anyone may block as Regent. Coup: pay 7 to remove an influence, cannot be blocked. At 10 coins a Coup is mandatory.".into(),"Regent (Duke): Tax +3; blocks Foreign aid.".into(),"Shade (Assassin): pay 3 to remove an influence. Sentinel can block. The cost is paid even if blocked or challenged. A failed challenge followed by assassination can cost two influences.".into(),"Corsair (Captain): steal up to 2 coins. The target can block as Corsair or Envoy.".into(),"Envoy (Ambassador): draw 2 cards, choose your remaining influences, return the rest.".into(),"Sentinel (Contessa): blocks assassination.".into(),"Scroll to see the table, actions and recent turns. Tab/Enter select; arrows/Page Up/Down scroll. Other devices join with the room code. Back to the arcade saves your seat; Leave forfeits it. Reconnect resumes your hidden hand.".into()]
        } else if self.game == GameKind::Wolves {
            let mut rules = vec!["Invite 6–16 friends. The host chooses Classic, Advanced, or a custom role list in the lobby. Every setup needs at least one wolf and a larger non-wolf team. Each special role can appear once.".into(), "Night: choose a player for your ability and, for wolves, a separate hunt vote. Choices lock when confirmed. Alpha votes count twice; tied hunts spare everyone. Even if an actor dies, a locked night action resolves.".into(), "Dawn reveals losses. Discuss in village chat, then vote. A strict majority of living players is needed to eliminate someone. Votes stay secret until the phase ends. A timer prevents disconnected seats from stalling the game.".into(), "Wolves win at parity unless the serial killer remains. Village wins after all wolves and the serial killer are gone. The Fool wins alone if voted out; the serial killer wins as the last survivor.".into(), "Pack chat stays private; wolves speak there at night. Ghosts talk in their own channel. The Medium can speak anonymously to ghosts at night. You can keep reading while your ability is locked.".into(), "Tap a portrait, then confirm your choice. Use My role for findings, remaining powers, the public role list, and the last revealed vote. Back saves your reconnect seat; Leave forfeits it. Disconnected players remain in the game.".into()];
            rules.extend(
                wolves::ROLES
                    .into_iter()
                    .map(|r| format!("{}: {}", r.title(), r.description())),
            );
            rules
        } else {
            vec!["The storyteller chooses a secret picture and gives a clue: a phrase, sound, title, or anything that sparks an association.".into(),"Each other player submits one picture matching the clue. At three players, hands have seven pictures and each other player submits two.".into(),"The pictures are shuffled. Everyone except the storyteller votes for the storyteller’s picture. You cannot vote for your own picture.".into(),"Some, but not everyone, guess correctly: the storyteller and correct voters get 3. Everyone or nobody guesses correctly: the storyteller gets 0 and every other player gets 2.".into(),"Each vote on your decoy earns you 1 extra point. Finish the round when someone reaches 30. Highest score wins; ties share the win.".into(),"Refill hands, rotate the storyteller, and reuse discarded pictures when the deck is low. There are 84 original generated illustrations.".into(),"Tap a card for a full-size preview, choose it, then confirm using the bottom button. Swipe or scroll to browse. Tab/Enter select; arrows/Page Up/Down scroll. Back to the arcade saves your seat. Leaving ends the match; Reconnect resumes it.".into()]
        };
        let heights: Vec<_> = lines
            .iter()
            .map(|l| wrapped_lines(ui, l, width - 24., 16.).len() as f32 * 25. + 28.)
            .collect();
        let total = heights.iter().sum::<f32>() + 30.;
        let (origin, _) = page_input(
            &mut self.pan,
            &mut self.blocked_touch,
            press,
            viewport,
            total,
        );
        clip(Some(viewport));
        let mut y = origin.y;
        for (line, h) in lines.iter().zip(heights) {
            wrap(
                ui,
                line,
                Rect::new(x + 12., y, width - 24., h),
                16.,
                ui.theme.text,
                false,
            );
            y += h;
        }
        clip(None);
    }
}
fn phase_key(r: &RoomView) -> String {
    if let Some(g) = &r.court {
        format!("{}:{}:{}", r.epoch, g.phase, g.turn)
    } else if let Some(g) = &r.reverie {
        format!("{}:{}:{}", g.phase, g.round, g.storyteller)
    } else if let Some(g) = &r.wolves {
        format!("{:?}:{}", g.phase, g.day)
    } else {
        "lobby".into()
    }
}
pub(crate) fn button(
    ui: &mut Ui,
    text: &str,
    r: Rect,
    primary: bool,
    tap: Option<Vec2>,
    viewport: Rect,
) -> bool {
    let fill = if primary && !ui.theme.saver {
        ui.theme.accent
    } else {
        ui.theme.bg
    };
    bordered(
        r,
        14.,
        if primary {
            ui.theme.accent
        } else {
            ui.theme.line
        },
        fill,
    );
    fit(
        ui,
        text,
        Rect::new(r.x + 8., r.y, r.w - 16., r.h),
        15.,
        if primary && !ui.theme.saver {
            WHITE
        } else {
            ui.theme.text
        },
        true,
    );
    if !r.overlaps(&viewport) {
        return false;
    }
    let hit = tap.is_some_and(|p| r.contains(p)) || ui.keyboard_hit(r);
    ui.activated |= hit;
    hit
}
pub(crate) fn page_input(
    pan: &mut BoardPan,
    blocked: &mut bool,
    press: Option<Vec2>,
    viewport: Rect,
    height: f32,
) -> (Vec2, Option<Vec2>) {
    let content = vec2(viewport.w, height.max(viewport.h));
    pan.clamp(viewport, content);
    let touches = touches();
    let live: Vec<_> = touches
        .iter()
        .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
        .collect();
    if live.len() > 1 || touches.iter().any(|t| t.phase == TouchPhase::Cancelled) {
        *blocked = true;
        pan.cancel();
    }
    let mut tap = None;
    if !*blocked {
        if let Some(p) = press
            && viewport.contains(p)
        {
            pan.begin(p);
        }
        let point = live
            .first()
            .map(|t| touch_point(t.position, screen_dpi_scale()))
            .unwrap_or_else(|| {
                let (x, y) = mouse_position();
                vec2(x, y)
            });
        if pan.active() {
            if is_mouse_button_released(MouseButton::Left)
                || touches.iter().any(|t| t.phase == TouchPhase::Ended)
            {
                let end = touches
                    .iter()
                    .find(|t| t.phase == TouchPhase::Ended)
                    .map_or(point, |t| touch_point(t.position, screen_dpi_scale()));
                tap = pan.end(end, viewport, content);
            } else {
                pan.update(point, viewport, content);
            }
        }
    } else if live.is_empty() {
        *blocked = false;
    }
    let (_, wheel) = mouse_wheel();
    if wheel != 0. {
        pan.scroll(vec2(0., -wheel * 60.), viewport, content);
    }
    let step = if is_key_pressed(KeyCode::PageDown) {
        viewport.h * 0.8
    } else if is_key_pressed(KeyCode::PageUp) {
        -viewport.h * 0.8
    } else if is_key_pressed(KeyCode::Down) {
        80.
    } else if is_key_pressed(KeyCode::Up) {
        -80.
    } else {
        0.
    };
    if step != 0. {
        pan.scroll(vec2(0., step), viewport, content);
    }
    (pan.board(viewport, content).point(), tap)
}
pub(crate) fn clip(viewport: Option<Rect>) {
    let dpi = screen_dpi_scale();
    // SAFETY: clipping is changed synchronously on the Macroquad render thread.
    unsafe {
        get_internal_gl().quad_gl.scissor(viewport.map(|r| {
            (
                (r.x * dpi) as i32,
                (r.y * dpi) as i32,
                (r.w * dpi) as i32,
                (r.h * dpi) as i32,
            )
        }));
    }
}
pub(crate) fn fit(ui: &Ui, text: &str, r: Rect, size: f32, color: Color, bold: bool) {
    if r.w < 4. {
        return;
    }
    let size = (size * r.w / ui.text_width(text, size, bold).max(1.))
        .min(size)
        .max(8.);
    if ui.text_width(text, size, bold) <= r.w {
        ui.centered(text, r, size, color, bold);
    } else {
        let mut shortened = text.to_owned();
        while !shortened.is_empty() && ui.text_width(&format!("{shortened}…"), size, bold) > r.w {
            shortened.pop();
        }
        ui.centered(&format!("{shortened}…"), r, size, color, bold);
    }
}
pub(crate) fn wrapped_lines(ui: &Ui, text: &str, width: f32, size: f32) -> Vec<String> {
    let mut lines = vec![];
    let mut line = String::new();
    for word in text.split_whitespace() {
        let test = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if ui.text_width(&test, size, false) <= width {
            line = test;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        for c in word.chars() {
            let test = format!("{line}{c}");
            if !line.is_empty() && ui.text_width(&test, size, false) > width {
                lines.push(std::mem::take(&mut line));
            }
            line.push(c);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
pub(crate) fn wrap(ui: &Ui, text: &str, r: Rect, size: f32, color: Color, bold: bool) {
    for (i, line) in wrapped_lines(ui, text, r.w, size).iter().enumerate() {
        let y = r.y + size + i as f32 * (size + 9.);
        if y > r.bottom() {
            break;
        }
        if bold {
            ui.heading(line, r.x, y, size, color);
        } else {
            ui.label(line, r.x, y, size, color);
        }
    }
}
