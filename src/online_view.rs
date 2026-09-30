//! Shared multiplayer interface; only the server can decide legal moves.
use crate::{
    card_art::{self, DeckArt},
    online_net::Network,
    platform,
    ui::{CORAL, Icon, Ui, bordered, rounded},
};
use jarcade::{
    board_pan::BoardPan,
    layout::touch_point,
    multiplayer::{
        ClientMessage, Command, GameKind, RoomView, ServerMessage, Session, clean_text, coup,
        reverie,
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
    #[cfg(target_os = "android")]
    keyboard_pan: BoardPan,
    preview: Option<u8>,
    selected: Vec<u8>,
    exchange: Vec<usize>,
    target: Option<coup::Action>,
    sent: Option<u64>,
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
            #[cfg(target_os = "android")]
            keyboard_pan: BoardPan::default(),
            preview: None,
            selected: vec![],
            exchange: vec![],
            target: None,
            sent: None,
        }
    }
    pub fn enter(&mut self, game: GameKind, code: Option<String>) {
        self.suspend();
        self.game = game;
        self.room = None;
        self.help = false;
        self.clue_open = false;
        self.confirm_leave = false;
        self.error.clear();
        self.fields[1] = code.unwrap_or_default();
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
            if edit.id < 3 {
                self.fields[edit.id] = if edit.id == 1 {
                    edit.text
                        .to_ascii_uppercase()
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric())
                        .take(6)
                        .collect()
                } else {
                    clean_text(&edit.text, if edit.id == 0 { 24 } else { 160 })
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
                        self.selected.clear();
                        self.exchange.clear();
                        self.target = None;
                        self.preview = None;
                        self.clue_open = false;
                        self.fields[2].clear();
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
        #[cfg(target_os = "android")]
        if self.keyboard_pan.active() {
            return true;
        }
        self.art.loading() || self.pan.active()
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
            } else {
                text.push_str("Lobby. Ready, then the host starts.");
            }
        } else {
            text.push_str("Create a room or join using a room code.");
        }
        if !self.error.is_empty() {
            text.push_str(&self.error);
        }
        text
    }
    pub fn back(&mut self) -> bool {
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
            #[cfg(target_os = "android")]
            {
                self.keyboard_pan = BoardPan::default();
            }
            platform::editor_open(
                &self.fields[id],
                id,
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
        let width = (screen_width() - 24.).min(1100.);
        let x = (screen_width() - width) / 2.;
        let header = Rect::new(x, 8., width, 44.);
        if ui.icon_button(Icon::Back, Rect::new(x, 8., 44., 44.), false) && self.back() {
            return true;
        }
        let heading = if let Some(room) = &self.room {
            format!("{}  ·  {}", self.game.title(), room.code)
        } else {
            self.game.title().into()
        };
        fit(
            ui,
            &heading,
            Rect::new(x + 56., 8., width - 156., 44.),
            20.,
            ui.theme.text,
            true,
        );
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
        #[cfg(target_os = "android")]
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
                    .is_some_and(|r| r.court.is_some() || r.reverie.is_some())
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
            } else {
                self.draw_lobby(ui, press, &room, x, width);
            }
        } else {
            self.draw_join(ui, press, x, width);
        }
        false
    }
    fn draw_join(&mut self, ui: &mut Ui, press: Option<Vec2>, x: f32, width: f32) {
        let w = width.min(440.);
        let x = x + (width - w) / 2.;
        let top = if screen_height() < 600. { 70. } else { 112. };
        let illustration = Rect::new(x, top, w, 80.);
        if screen_height() > 650. {
            if self.game == GameKind::Court {
                card_art::preview(
                    ui,
                    Rect::new(illustration.center().x - 72., top, 144., 110.),
                );
            } else {
                self.art.preview(
                    ui,
                    Rect::new(illustration.center().x - 72., top, 144., 110.),
                );
            }
        }
        let short = screen_height() <= 650.;
        let viewport = Rect::new(x, 66., w, screen_height() - 82.);
        let (y, press) = if short {
            let (origin, tap) = page_input(
                &mut self.pan,
                &mut self.blocked_touch,
                press,
                viewport,
                380.,
            );
            clip(Some(viewport));
            (origin.y + 10., tap)
        } else {
            (top + 138., press)
        };
        let original_pointer = if short {
            ui.override_pointer(press)
        } else {
            None
        };
        let (min, max) = self.game.limits();
        ui.centered(
            &format!("Online · {min}–{max} players"),
            Rect::new(x, y, w, 30.),
            15.,
            ui.theme.muted,
            false,
        );
        self.field(ui, 0, Rect::new(x, y + 42., w, 50.), "Your name", press);
        let enabled = !self.fields[0].is_empty() && !self.connecting;
        if ui.button(
            if self.connecting {
                "Connecting…"
            } else {
                "Create room"
            },
            Rect::new(x, y + 104., w, 50.),
            enabled,
        ) && enabled
        {
            self.persist();
            self.connect(ClientMessage::Create {
                game: self.game,
                name: self.fields[0].clone(),
            });
        }
        self.field(
            ui,
            1,
            Rect::new(x, y + 178., w - 114., 50.),
            "Room code",
            press,
        );
        if ui.button(
            "Join",
            Rect::new(x + w - 104., y + 178., 104., 50.),
            enabled,
        ) && enabled
            && self.fields[1].len() == 6
        {
            self.persist();
            self.connect(ClientMessage::Join {
                room: self.fields[1].clone(),
                name: self.fields[0].clone(),
            });
        }
        if let Some(session) = self.session().cloned()
            && ui.button(
                &format!("Resume {}", session.room),
                Rect::new(x, y + 244., w, 48.),
                false,
            )
            && !self.connecting
        {
            self.reconnect();
        }
        if short {
            clip(None);
            ui.override_pointer(original_pointer);
        }
        wrap(
            ui,
            &self.error,
            Rect::new(x, y + 306., w, (screen_height() - y - 322.).max(48.)),
            14.,
            CORAL,
            false,
        );
    }
    fn draw_lobby(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        room: &RoomView,
        x: f32,
        width: f32,
    ) {
        let cols = if width > 650. { 2 } else { 1 };
        let height = 80. + room.members.len().div_ceil(cols) as f32 * 58. + 180.;
        let viewport = Rect::new(x, 66., width, screen_height() - 82.);
        let (origin, tap) = page_input(
            &mut self.pan,
            &mut self.blocked_touch,
            press,
            viewport,
            height,
        );
        clip(Some(viewport));
        ui.centered(
            "Invite your friends",
            Rect::new(x, origin.y + 2., width, 30.),
            22.,
            ui.theme.text,
            true,
        );
        ui.centered(
            &format!(
                "Room {} · {}–{} players",
                room.code,
                self.game.limits().0,
                self.game.limits().1
            ),
            Rect::new(x, origin.y + 36., width, 24.),
            15.,
            ui.theme.muted,
            false,
        );
        for (i, m) in room.members.iter().enumerate() {
            let rect = Rect::new(
                x + (i % cols) as f32 * (width / cols as f32),
                origin.y + 80. + (i / cols) as f32 * 58.,
                width / cols as f32 - 8.,
                50.,
            );
            bordered(rect, 14., ui.theme.line, ui.theme.panel);
            fit(
                ui,
                &format!("{}{}", m.name, if i == room.you { " · you" } else { "" }),
                Rect::new(rect.x + 14., rect.y, rect.w - 112., 50.),
                16.,
                ui.theme.text,
                true,
            );
            fit(
                ui,
                if !m.connected {
                    "Offline"
                } else if m.ready {
                    "Ready"
                } else {
                    "Waiting"
                },
                Rect::new(rect.right() - 88., rect.y, 76., 50.),
                13.,
                ui.theme.muted,
                false,
            );
        }
        let y = origin.y + 80. + room.members.len().div_ceil(cols) as f32 * 58. + 12.;
        let me = &room.members[room.you];
        if button(
            ui,
            if me.ready { "Not ready" } else { "Ready" },
            Rect::new(x, y, width, 48.),
            !me.ready,
            tap,
            viewport,
        ) {
            self.play(Command::Ready(!me.ready));
        }
        if room.host == room.you
            && button(
                ui,
                "Start game",
                Rect::new(x, y + 60., width, 48.),
                true,
                tap,
                viewport,
            )
        {
            self.play(Command::Start);
        }
        if button(
            ui,
            "Leave room",
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
        let cols = if width >= 700. { 3 } else { 2 };
        let rowh = 70.;
        let scores = g.players.len().div_ceil(cols) as f32 * rowh;
        let handh = if screen_height() < 600. { 128. } else { 182. };
        let actions = if self.target.is_some() {
            g.players.len()
        } else if !g.exchange.is_empty() {
            1
        } else {
            g.actions.len().max(g.choices.len()).max(1)
        };
        let gridcols = if width >= 700. { 3 } else { 2 };
        let actionheight = actions.div_ceil(gridcols) as f32 * 58.;
        let logs = 96.;
        let total = scores + 50. + handh + 38. + actionheight + logs + 68.;
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
            bordered(
                rect,
                14.,
                if i == g.turn {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                ui.theme.panel,
            );
            fit(
                ui,
                &format!("{}{}", p.name, if i == room.you { " · you" } else { "" }),
                Rect::new(rect.x + 10., rect.y + 4., rect.w - 48., 26.),
                14.,
                ui.theme.text,
                true,
            );
            ui.label(
                &format!("{} coins", p.coins),
                rect.x + 10.,
                rect.y + 48.,
                12.,
                ui.theme.muted,
            );
            for (j, c) in p.cards.iter().enumerate() {
                let r = Rect::new(rect.right() - 42. + j as f32 * 17., rect.y + 34., 12., 17.);
                rounded(
                    r,
                    3.,
                    if c.revealed {
                        ui.theme.line
                    } else {
                        ui.theme.accent
                    },
                );
            }
            if !room.members[i].connected {
                draw_circle(rect.right() - 12., rect.y + 14., 3., CORAL);
            }
            if rect.overlaps(&viewport)
                && (tap.is_some_and(|point| rect.contains(point)) || ui.keyboard_hit(rect))
            {
                self.preview = Some(i as u8);
                ui.activated = true;
            }
        }
        let y = origin.y + scores + 8.;
        fit(
            ui,
            &g.prompt,
            Rect::new(x, y, width, 38.),
            20.,
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
        let cw = (width / pool.len() as f32 - 10.).min(handh / 1.5);
        let start = x + (width - (cw + 10.) * pool.len() as f32 + 10.) / 2.;
        for (i, c) in pool.iter().enumerate() {
            let r = Rect::new(start + i as f32 * (cw + 10.), y + 48., cw, handh);
            card_art::court_card(ui, r, c.role, c.revealed, false);
            if self.exchange.contains(&i) {
                draw_rectangle_lines(r.x - 2., r.y - 2., r.w + 4., r.h + 4., 3., ui.theme.accent);
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
        let ay = y + 48. + handh + 16.;
        let mut moves: Vec<(String, Option<Command>)> = vec![];
        if let Some(action) = self.target {
            for (i, p) in g.players.iter().enumerate() {
                if i != room.you && p.cards.iter().any(|c| !c.revealed) {
                    moves.push((
                        format!("{} → {}", action.title(), p.name),
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
            let w = (width - 10. * (gridcols - 1) as f32) / gridcols as f32;
            let r = Rect::new(
                x + (i % gridcols) as f32 * (w + 10.),
                ay + (i / gridcols) as f32 * 58.,
                w,
                48.,
            );
            if button(ui, label, r, true, tap, viewport) {
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
        let ly = ay + moves.len().max(1).div_ceil(gridcols) as f32 * 58. + 12.;
        for (i, event) in g.log.iter().rev().take(3).enumerate() {
            fit(
                ui,
                event,
                Rect::new(x, ly + i as f32 * 26., width, 24.),
                12.,
                ui.theme.muted,
                false,
            );
        }
        if button(
            ui,
            "Leave table",
            Rect::new(x, ly + 88., width, 44.),
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
        let rh = if short { 24. } else { 34. };
        for (i, p) in g.players.iter().enumerate() {
            let r = Rect::new(
                x + (i % cols) as f32 * (sw + 6.),
                64. + (i / cols) as f32 * rh,
                sw,
                rh - 6.,
            );
            rounded(r, 9., ui.theme.panel);
            let label = format!(
                "{}{}  {}{}",
                p.name,
                if i == room.you { " · you" } else { "" },
                p.score,
                if p.gained > 0 {
                    format!(" +{}", p.gained)
                } else {
                    String::new()
                }
            );
            fit(
                ui,
                &label,
                Rect::new(r.x + 8., r.y, r.w - 16., r.h),
                12.,
                if i == g.storyteller {
                    ui.theme.accent
                } else {
                    ui.theme.text
                },
                i == g.storyteller,
            );
        }
        let top = 64. + g.players.len().div_ceil(cols) as f32 * rh + 8.;
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
            wrap(
                ui,
                &format!("“{}”", g.clue),
                Rect::new(x, top + clue_offset, side, 54.),
                if short { 14. } else { 17. },
                ui.theme.accent,
                true,
            );
            if ui.hit(Rect::new(x, top + clue_offset, side, 54.)) {
                self.clue_open = true;
                self.pan = BoardPan::default();
            }
            top + clue_offset + 60.
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
            self.art.draw(ui, card, r);
            let table = g.table.iter().find(|c| c.card == card);
            let marked = self.selected.contains(&card) || g.vote == Some(card);
            if marked {
                draw_rectangle_lines(r.x - 2., r.y - 2., r.w + 4., r.h + 4., 3., ui.theme.accent);
            }
            if table.is_some_and(|c| c.story) {
                draw_rectangle_lines(r.x - 2., r.y - 2., r.w + 4., r.h + 4., 4., ui.theme.accent);
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
                format!("{}", i + 1)
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
                "story" => "Tell story",
                "submit" => "Submit cards",
                "vote" => "Confirm vote",
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
        if ui.button(
            label,
            Rect::new(bottom.x, bottom.y, bottom.w - 60., 48.),
            can,
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
        draw_rectangle(
            0.,
            56.,
            screen_width(),
            screen_height() - 56.,
            Color::new(0., 0., 0., 0.75),
        );
        let h = (screen_height() - 170.).clamp(90., 720.);
        let w = (h / 1.5).min(width - 16.);
        let r = Rect::new((screen_width() - w) / 2., 70., w, w * 1.5);
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
        if ui.button(label, Rect::new(x + bw + 12., by, bw, 48.), can) && can {
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
    /// Miniquad's Android backend has no text IME; provide shared-code touch keys.
    #[cfg(target_os = "android")]
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
                ["Your name", "Room code", "Your clue"][id]
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
    } else {
        "lobby".into()
    }
}
fn button(
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
fn page_input(
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
fn clip(viewport: Option<Rect>) {
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
fn fit(ui: &Ui, text: &str, r: Rect, size: f32, color: Color, bold: bool) {
    let size = (size * r.w / ui.text_width(text, size, bold).max(1.)).min(size);
    ui.centered(text, r, size.max(8.), color, bold);
}
fn wrapped_lines(ui: &Ui, text: &str, width: f32, size: f32) -> Vec<String> {
    let mut lines = vec![];
    let mut line = String::new();
    for word in text.split_whitespace() {
        let test = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if ui.text_width(&test, size, false) > width && !line.is_empty() {
            lines.push(line);
            line = word.into();
        } else {
            line = test;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
fn wrap(ui: &Ui, text: &str, r: Rect, size: f32, color: Color, bold: bool) {
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
